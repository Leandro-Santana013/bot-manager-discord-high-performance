use serenity::builder::{CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::application::CommandOptionType;
use serenity::model::prelude::*;
use serenity::prelude::*;
use crate::database::tickets::TicketDb;

pub fn register() -> CreateCommand {
    CreateCommand::new("config_metas")
        .description("Configura os cargos da tabela de metas semanais (Apenas Admins).")
        .default_member_permissions(Permissions::ADMINISTRATOR)
        .add_option(CreateCommandOption::new(CommandOptionType::Role, "role_god", "Cargo para 50h+ (god)").required(false))
        .add_option(CreateCommandOption::new(CommandOptionType::Role, "role_ace", "Cargo para 45h+ (ace)").required(false))
        .add_option(CreateCommandOption::new(CommandOptionType::Role, "role_cry", "Cargo para 40h+ (cry)").required(false))
        .add_option(CreateCommandOption::new(CommandOptionType::Role, "role_high", "Cargo para 35h+ (high)").required(false))
        .add_option(CreateCommandOption::new(CommandOptionType::Role, "role_1st", "Cargo para 30h+ (1st)").required(false))
        .add_option(CreateCommandOption::new(CommandOptionType::Role, "role_2nd", "Cargo para 25h+ (2nd)").required(false))
        .add_option(CreateCommandOption::new(CommandOptionType::Role, "role_sub", "Cargo para 20h+ (sub)").required(false))
        .add_option(CreateCommandOption::new(CommandOptionType::Role, "role_base", "Cargo para 15h+ (Demais Cargos)").required(false))
        .add_option(CreateCommandOption::new(CommandOptionType::Channel, "canal_relatorio", "Canal para receber o relatório automático no Domingo").required(false))
}

pub async fn run(ctx: &Context, interaction: &serenity::model::application::CommandInteraction) {
    if !interaction.member.as_ref().map(|m| m.permissions.unwrap_or(Permissions::empty()).administrator()).unwrap_or(false) {
        let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content("❌ Apenas administradores podem usar isso.").ephemeral(true)
        )).await;
        return;
    }

    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Defer(
        CreateInteractionResponseMessage::new().ephemeral(true)
    )).await;

    let data = ctx.data.read().await;
    let pool = data.get::<crate::DatabasePool>().expect("DB pool not initialized").clone();
    drop(data);

    let roles = vec!["god", "ace", "cry", "high", "1st", "2nd", "sub", "base"];
    let mut mensagem = String::from("✅ **Cargos de Metas Atualizados:**\n\n");

    for r in &roles {
        let option_name = format!("role_{}", r);
        let role_input = interaction.data.options.iter().find(|o| o.name == option_name);

        if let Some(opt) = role_input {
            if let serenity::model::application::CommandDataOptionValue::Role(role_id) = &opt.value {
                let _ = TicketDb::set_config(&pool, &format!("meta_role_{}", r), &role_id.to_string()).await;
                mensagem.push_str(&format!("- **{}:** <@&{}>\n", r.to_uppercase(), role_id));
            }
        } else {
            let salvo = TicketDb::get_config(&pool, &format!("meta_role_{}", r), "").await;
            if !salvo.is_empty() {
                mensagem.push_str(&format!("- **{}:** <@&{}> (Mantido)\n", r.to_uppercase(), salvo));
            } else {
                mensagem.push_str(&format!("- **{}:** Não configurado\n", r.to_uppercase()));
            }
        }
    }

    let canal_input = interaction.data.options.iter().find(|o| o.name == "canal_relatorio");
    if let Some(opt) = canal_input {
        if let serenity::model::application::CommandDataOptionValue::Channel(channel_id) = &opt.value {
            let _ = TicketDb::set_config(&pool, "meta_report_channel", &channel_id.to_string()).await;
            mensagem.push_str(&format!("\n📁 **Canal de Relatórios:** <#{}>\n", channel_id));
        }
    } else {
        let salvo = TicketDb::get_config(&pool, "meta_report_channel", "").await;
        if !salvo.is_empty() {
            mensagem.push_str(&format!("\n📁 **Canal de Relatórios:** <#{}> (Mantido)\n", salvo));
        } else {
            mensagem.push_str("\n📁 **Canal de Relatórios:** Nenhum configurado (O bot não mandará o fechamento automático!).\n");
        }
    }

    mensagem.push_str("\n*(Obs: Se você não preencheu algum campo agora, mas já tinha preenchido antes, ele continua salvo!)*");

    let _ = interaction.edit_response(&ctx.http, serenity::builder::EditInteractionResponse::new().content(mensagem)).await;
}
