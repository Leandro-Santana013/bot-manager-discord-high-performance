use serenity::builder::{CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, CreateAttachment};
use serenity::model::prelude::*;
use serenity::prelude::*;
use crate::database::voice::VoiceDb;
use crate::database::tickets::TicketDb;

pub fn register() -> CreateCommand {
    CreateCommand::new("fechar_metas")
        .description("Verifica quem bateu a meta semanal, rebaixa quem falhou e zera os inativos.")
        .default_member_permissions(Permissions::ADMINISTRATOR)
}

pub async fn run(ctx: &Context, interaction: &serenity::model::application::CommandInteraction) {
    if !interaction.member.as_ref().map(|m| m.permissions.unwrap_or(Permissions::empty()).administrator()).unwrap_or(false) {
        let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content("❌ Apenas administradores podem fechar as metas.").ephemeral(true)
        )).await;
        return;
    }

    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Defer(
        CreateInteractionResponseMessage::new().ephemeral(false)
    )).await;

    let data = ctx.data.read().await;
    let pool = data.get::<crate::DatabasePool>().expect("DB pool not initialized").clone();

    let relatorio = VoiceDb::get_all_users_closing_stats(&pool).await;

    struct MetaConfig {
        nome: &'static str,
        horas_para_manter: f64,
        id: String,
    }

    let mut metas = vec![
        MetaConfig { nome: "god", horas_para_manter: 50.0, id: TicketDb::get_config(&pool, "meta_role_god", "").await },
        MetaConfig { nome: "ace", horas_para_manter: 45.0, id: TicketDb::get_config(&pool, "meta_role_ace", "").await },
        MetaConfig { nome: "cry", horas_para_manter: 40.0, id: TicketDb::get_config(&pool, "meta_role_cry", "").await },
        MetaConfig { nome: "high", horas_para_manter: 35.0, id: TicketDb::get_config(&pool, "meta_role_high", "").await },
        MetaConfig { nome: "1st", horas_para_manter: 30.0, id: TicketDb::get_config(&pool, "meta_role_1st", "").await },
        MetaConfig { nome: "2nd", horas_para_manter: 25.0, id: TicketDb::get_config(&pool, "meta_role_2nd", "").await },
        MetaConfig { nome: "sub", horas_para_manter: 20.0, id: TicketDb::get_config(&pool, "meta_role_sub", "").await },
        MetaConfig { nome: "base", horas_para_manter: 15.0, id: TicketDb::get_config(&pool, "meta_role_base", "").await },
    ];
    metas.retain(|m| !m.id.is_empty());

    let mut texto_relatorio = String::from("=== RELATÓRIO DE FECHAMENTO DE METAS ===\n\n");
    let mut zerados = 0;
    let mut rebaixados = 0;
    let mut mantidos = 0;

    let guild_id = interaction.guild_id.unwrap();

    for stats in relatorio {
        if let Ok(user_id) = stats.id_usuario.parse::<u64>() {
            if let Ok(member) = guild_id.member(&ctx.http, user_id).await {
                let horas_na_semana = (stats.this_week_ms as f64) / (1000.0 * 60.0 * 60.0);

                if stats.days_inactive >= 14 {
                    VoiceDb::reset_user_total(&pool, &stats.id_usuario).await;
                    for meta in &metas {
                        if let Ok(role_id) = meta.id.parse::<u64>() {
                            if member.roles.contains(&RoleId::new(role_id)) {
                                let _ = member.remove_role(&ctx.http, RoleId::new(role_id)).await;
                            }
                        }
                    }
                    texto_relatorio.push_str(&format!("[INATIVO ZERADO] <@{}> estava inativo há {} dias. Perdeu todos os cargos e tempo total foi a 0.\n", user_id, stats.days_inactive));
                    zerados += 1;
                    continue;
                }

                for meta in &metas {
                    if let Ok(role_id) = meta.id.parse::<u64>() {
                        if member.roles.contains(&RoleId::new(role_id)) {
                            if horas_na_semana < meta.horas_para_manter {
                                let _ = member.remove_role(&ctx.http, RoleId::new(role_id)).await;
                                texto_relatorio.push_str(&format!("[REBAIXADO] <@{}> perdeu o cargo {} (Fez {:.1}h / Precisava de {}h).\n", user_id, meta.nome.to_uppercase(), horas_na_semana, meta.horas_para_manter));
                                rebaixados += 1;
                            } else {
                                texto_relatorio.push_str(&format!("[MANTEVE] <@{}> manteve o cargo {} com sucesso ({:.1}h).\n", user_id, meta.nome.to_uppercase(), horas_na_semana));
                                mantidos += 1;
                            }
                        }
                    }
                }

                if horas_na_semana >= 20.0 {
                    texto_relatorio.push_str(&format!("[ELEGÍVEL PARA UP] <@{}> fez impressionantes {:.1}h essa semana e tem MÉRITO para subir!\n", user_id, horas_na_semana));
                }
            }
        }
    }

    texto_relatorio.push_str(&format!("\n\n=== RESUMO ===\nInativos Zerados: {}\nRebaixados: {}\nMantiveram a patente: {}\n", zerados, rebaixados, mantidos));

    let attachment = CreateAttachment::bytes(texto_relatorio.into_bytes(), "relatorio_metas.txt");

    let _ = interaction.edit_response(&ctx.http, serenity::builder::EditInteractionResponse::new()
        .content("✅ **Fechamento concluído!** Baixe o relatório em texto abaixo para ver o que aconteceu detalhadamente com cada membro (quem foi zerado por inatividade e quem perdeu cargo por não bater as horas).")
        .new_attachment(attachment)
    ).await;
}
