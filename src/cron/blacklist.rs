use std::sync::Arc;
use tokio::time::{sleep, Duration};
use serenity::prelude::Context;
use std::collections::HashSet;
use tracing::{info, error};

use crate::database::blacklist::BlacklistDb;

pub async fn start(ctx: Arc<Context>) {
    info!("Cron Blacklist pronto (event-driven, aguardando notificações).");

    // Wait for pool to be inserted
    sleep(Duration::from_secs(5)).await;

    let (pool, notify) = {
        let data = ctx.data.read().await;
        let pool = data.get::<crate::DatabasePool>().expect("DB pool not initialized").clone();
        let notify = data.get::<crate::BlacklistNotify>().expect("BlacklistNotify not initialized").clone();
        (pool, notify)
    };

    let http = ctx.http.clone();

    // Ao iniciar, verifica se já existem usuários pendentes no banco (de antes do restart)
    let has_pending = !BlacklistDb::get_next_expiration(&pool).await.is_none();
    if has_pending {
        info!("[Cron Blacklist] Usuários pendentes encontrados no banco, iniciando verificação.");
    }

    loop {
        // Calcula quanto tempo dormir até a próxima expiração
        let sleep_duration = if let Some(next_expiry) = BlacklistDb::get_next_expiration(&pool).await {
            let now = chrono::Utc::now().timestamp_millis();
            let diff = next_expiry - now;
            if diff <= 0 {
                // Já expirou, processar imediatamente
                Duration::from_millis(0)
            } else {
                // Espera até a expiração + 1s de margem
                Duration::from_millis(diff as u64 + 1000)
            }
        } else {
            // Sem ninguém na blacklist — dorme até ser notificado
            info!("[Cron Blacklist] Nenhum usuário na blacklist, aguardando notificação...");
            notify.notified().await;
            info!("[Cron Blacklist] Notificação recebida! Verificando blacklist...");
            // Após notificação, recalcula o tempo de espera
            continue;
        };

        // Espera pelo tempo calculado OU por uma nova notificação (o que vier primeiro)
        if sleep_duration > Duration::from_millis(0) {
            tokio::select! {
                _ = sleep(sleep_duration) => {},
                _ = notify.notified() => {
                    info!("[Cron Blacklist] Nova notificação recebida durante espera.");
                },
            }
        }

        // Processa usuários expirados
        let now = chrono::Utc::now().timestamp_millis();
        let expired_users = BlacklistDb::get_expired_users(&pool, now).await;

        if expired_users.is_empty() {
            continue;
        }

        let mut panels_to_update = HashSet::new();

        for record in expired_users {
            if let Ok(guild_id) = record.guild_id.parse::<u64>() {
                if let Ok(user_id) = record.user_id.parse::<u64>() {
                    let guild_id = serenity::model::id::GuildId::new(guild_id);
                    if let Ok(member) = guild_id.member(&http, serenity::model::id::UserId::new(user_id)).await {
                        if let Ok(role_id) = record.role_id.parse::<u64>() {
                            let role_id = serenity::model::id::RoleId::new(role_id);
                            if let Err(e) = member.add_role(&http, role_id).await {
                                error!("[Cron Blacklist] Erro ao devolver cargo: {}", e);
                            }
                        }
                    }
                }
            }

            let _ = BlacklistDb::remove_user(&pool, &record.guild_id, &record.user_id).await;
            panels_to_update.insert(record.panel_msg_id);
        }

        for msg_id in panels_to_update {
            if let Some(panel) = BlacklistDb::get_panel(&pool, &msg_id).await {
                let users = BlacklistDb::get_users_for_panel(&pool, &msg_id).await;
                
                let mut desc = String::new();
                if users.is_empty() {
                    desc = "Nenhum membro restrito.".to_string();
                } else {
                    for u in users {
                        desc.push_str(&format!("- <@{}> até <t:{}:R>\n", u.user_id, u.expires_at / 1000));
                    }
                }

                let embed = serenity::builder::CreateEmbed::new()
                    .title("🛡️ Painel de Blacklist")
                    .description(format!("Este painel gerencia a blacklist para o cargo <@&{}>.\n\n**Membros na Blacklist atualmente:**\n{}", panel.role_id, desc))
                    .footer(serenity::builder::CreateEmbedFooter::new("Selecione um usuário no menu abaixo para adicionar à blacklist."));

                if let Ok(channel_id) = panel.channel_id.parse::<u64>() {
                    let channel_id = serenity::model::id::ChannelId::new(channel_id);
                    if let Ok(msg_id_u64) = msg_id.parse::<u64>() {
                        let _ = channel_id.edit_message(&http, serenity::model::id::MessageId::new(msg_id_u64), serenity::builder::EditMessage::new().embed(embed)).await;
                    }
                }
            }
        }
    }
}
