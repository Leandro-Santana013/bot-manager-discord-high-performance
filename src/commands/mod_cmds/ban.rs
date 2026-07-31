use serenity::builder::{CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::application::{CommandOptionType, ResolvedValue};
use serenity::model::prelude::*;
use serenity::prelude::*;

pub fn register() -> CreateCommand {
    CreateCommand::new("ban")
        .description("Bane um usuário do servidor")
        .default_member_permissions(Permissions::BAN_MEMBERS)
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "usuario", "Usuário para banir")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "motivo", "Motivo do banimento")
                .required(false),
        )
}

pub async fn run(ctx: &Context, interaction: &serenity::model::application::CommandInteraction) {
    let mut user_to_ban = None;
    let mut reason = "Motivo não especificado";

    let options = interaction.data.options();
    for option in &options {
        match option.name {
            "usuario" => {
                if let ResolvedValue::User(user, _) = &option.value {
                    user_to_ban = Some(user);
                }
            }
            "motivo" => {
                if let ResolvedValue::String(r) = &option.value {
                    reason = r;
                }
            }
            _ => {}
        }
    }

    if let Some(user) = user_to_ban {
        if let Some(guild_id) = interaction.guild_id {
            match guild_id.ban_with_reason(&ctx.http, user.id, 0, reason).await {
                Ok(_) => {
                    let msg = format!("🔨 O usuário {} foi banido.\nMotivo: {}", user.tag(), reason);
                    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(msg).ephemeral(true)
                    )).await;
                }
                Err(e) => {
                    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(format!("Falha ao banir: {}", e)).ephemeral(true)
                    )).await;
                }
            }
        }
    } else {
        let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content("Usuário não encontrado.").ephemeral(true)
        )).await;
    }
}
