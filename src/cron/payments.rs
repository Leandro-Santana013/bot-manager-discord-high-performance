use std::sync::Arc;
use serenity::prelude::Context;
use tokio::time::{sleep, Duration};
use tracing::{info, error};

use crate::database::payments::PaymentDb;
use crate::database::vip::VipDb;
use crate::cron::mercado_pago::MercadoPagoClient;

pub async fn start(ctx: Arc<Context>) {
    let mp_client = MercadoPagoClient::new();
    
    // Loop de monitoramento do Mercado Pago a cada 1 minuto (60 segundos)
    loop {
        sleep(Duration::from_secs(60)).await;

        let pool = {
            let data = ctx.data.read().await;
            if let Some(p) = data.get::<crate::DatabasePool>() {
                p.clone()
            } else {
                continue;
            }
        };

        let pending = match PaymentDb::get_pending_payments(&pool).await {
            Ok(p) => p,
            Err(e) => {
                error!("Erro ao buscar pagamentos pendentes: {}", e);
                continue;
            }
        };

        if pending.is_empty() {
            continue;
        }

        let prods = VipDb::get_products(&pool).await;

        for (payment_id, user_id_str, package_id) in pending {
            match mp_client.get_payment_status(&payment_id).await {
                Ok(status) => {
                    if status == "approved" {
                        info!("Pagamento aprovado: {} para usuário {}", payment_id, user_id_str);
                        
                        let _ = PaymentDb::remove_payment(&pool, &payment_id).await;

                        if let Ok(user_id) = user_id_str.parse::<u64>() {
                            let role_id_str = prods.iter().find(|p| p.id == package_id).map(|p| p.role_id.clone()).unwrap_or_default();
                            if let Ok(role_id) = role_id_str.parse::<u64>() {
                                // Temos que iterar nas guildas ou assumir a guild_id do .env
                                // Para simplificar, assumimos que há 1 guild_id ou pegamos do cache
                                let guilds = ctx.cache.guilds();
                                for guild_id in guilds {
                                    let http = ctx.http.clone();
                                    if let Ok(member) = guild_id.member(&http, user_id).await {
                                        let _ = member.add_role(&http, role_id).await;
                                        
                                        // Enviar DM de sucesso
                                        if let Ok(user) = serenity::model::id::UserId::new(user_id).to_user(&http).await {
                                            let embed = serenity::builder::CreateEmbed::new()
                                                .title("✅ Pagamento Aprovado!")
                                                .description(format!("O seu pacote VIP **{}** foi ativado e o cargo já foi entregue no servidor!", package_id.replace("vip_", "").to_uppercase()))
                                                .color(0x2ecc71);
                                            let _ = user.direct_message(&http, serenity::builder::CreateMessage::new().embed(embed)).await;
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    } else if status == "cancelled" || status == "rejected" {
                        let _ = PaymentDb::remove_payment(&pool, &payment_id).await;
                        info!("Pagamento {} cancelado/rejeitado", payment_id);
                    }
                }
                Err(e) => {
                    error!("Erro ao checar pagamento {}: {}", payment_id, e);
                }
            }
        }
    }
}
