use serenity::builder::{
    CreateActionRow, CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateSelectMenu, CreateSelectMenuKind, CreateButton,
};
use serenity::model::application::{CommandOptionType, ResolvedValue};
use serenity::model::prelude::*;
use serenity::prelude::*;
use crate::database::blacklist::BlacklistDb;

pub fn register() -> CreateCommand {
    CreateCommand::new("blacklist")
        .description("Cria um painel interativo de blacklist (Apenas Admin).")
        .default_member_permissions(Permissions::ADMINISTRATOR)
        .add_option(
            CreateCommandOption::new(CommandOptionType::Role, "cargo", "O cargo que será gerenciado pelo painel")
                .required(true),
        )
}

pub async fn run(ctx: &Context, interaction: &serenity::model::application::CommandInteraction) {
    let mut role_id = None;

    let options = interaction.data.options();
    for option in &options {
        if option.name == "cargo" {
            if let ResolvedValue::Role(role) = &option.value {
                role_id = Some(role.id.get());
            }
        }
    }

    if let Some(r_id) = role_id {
        let embed = CreateEmbed::new()
            .title("🛡️ Painel de Blacklist")
            .description(format!("Este painel gerencia a blacklist para o cargo <@&{}>.\n\n**Membros na Blacklist atualmente:**\nNenhum membro restrito.", r_id))
            .footer(serenity::builder::CreateEmbedFooter::new("Selecione um usuário no menu abaixo para adicionar à blacklist."));

        let select = CreateSelectMenu::new("blacklist_add_user", CreateSelectMenuKind::User { default_users: None })
            .placeholder("Adicionar usuário à blacklist...");
        
        let button = CreateButton::new("blacklist_remove_user")
            .label("Remover Usuário da Blacklist")
            .emoji('🔓')
            .style(serenity::model::application::ButtonStyle::Secondary);

        let row1 = CreateActionRow::SelectMenu(select);
        let row2 = CreateActionRow::Buttons(vec![button]);

        // Envia mensagem
        let msg_result = interaction.channel_id.send_message(&ctx.http, serenity::builder::CreateMessage::new()
            .embed(embed)
            .components(vec![row1, row2])
        ).await;

        match msg_result {
            Ok(msg) => {
                let data = ctx.data.read().await;
                let pool = data.get::<crate::DatabasePool>().expect("DB pool not initialized").clone();
                let guild_id = interaction.guild_id.unwrap().get();
                
                let _ = BlacklistDb::add_panel(&pool, &msg.id.to_string(), &msg.channel_id.to_string(), &guild_id.to_string(), &r_id.to_string()).await;

                let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content("✅ Painel de blacklist criado com sucesso!").ephemeral(true)
                )).await;
            }
            Err(e) => {
                let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content(format!("❌ Erro ao criar o painel: {}", e)).ephemeral(true)
                )).await;
            }
        }
    } else {
        let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content("❌ Cargo inválido.").ephemeral(true)
        )).await;
    }
}

pub async fn update_panel(ctx: &Context, pool: &sqlx::PgPool, msg_id: &str) {
    if let Some(panel) = BlacklistDb::get_panel(pool, msg_id).await {
        let users = BlacklistDb::get_users_for_panel(pool, msg_id).await;
        
        let mut desc = String::new();
        if users.is_empty() {
            desc = "Nenhum membro restrito.".to_string();
        } else {
            for u in users {
                desc.push_str(&format!("- <@{}> até <t:{}:R>\n", u.user_id, u.expires_at / 1000));
            }
        }

        let embed = CreateEmbed::new()
            .title("🛡️ Painel de Blacklist")
            .description(format!("Este painel gerencia a blacklist para o cargo <@&{}>.\n\n**Membros na Blacklist atualmente:**\n{}", panel.role_id, desc))
            .footer(serenity::builder::CreateEmbedFooter::new("Selecione um usuário no menu abaixo para adicionar à blacklist."));

        if let Ok(channel_id) = panel.channel_id.parse::<u64>() {
            let channel_id = ChannelId::new(channel_id);
            if let Ok(msg_id_u64) = msg_id.parse::<u64>() {
                let _ = channel_id.edit_message(&ctx.http, MessageId::new(msg_id_u64), serenity::builder::EditMessage::new().embed(embed)).await;
            }
        }
    }
}
