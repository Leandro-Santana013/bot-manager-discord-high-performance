use serenity::builder::{CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::application::{CommandOptionType, ResolvedValue};
use serenity::model::prelude::*;
use serenity::prelude::*;

pub fn register() -> CreateCommand {
    CreateCommand::new("unban")
        .description("Desbane um usuário do servidor.")
        .default_member_permissions(Permissions::BAN_MEMBERS)
        .add_option(
            CreateCommandOption::new(CommandOptionType::String, "id", "O ID do usuário para desbanir")
                .required(true),
        )
}

pub async fn run(ctx: &Context, interaction: &serenity::model::application::CommandInteraction) {
    let mut target_id = String::new();

    let options = interaction.data.options();
    for option in &options {
        if option.name == "id" {
            if let ResolvedValue::String(s) = &option.value {
                target_id = s.to_string();
            }
        }
    }

    if let Ok(user_id) = target_id.parse::<u64>() {
        let user_id = UserId::new(user_id);
        if let Some(guild_id) = interaction.guild_id {
            match guild_id.unban(&ctx.http, user_id).await {
                Ok(_) => {
                    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(format!("✅ O usuário com ID `{}` foi desbanido.", user_id.get())).ephemeral(true)
                    )).await;
                }
                Err(e) => {
                    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(format!("❌ Erro ao desbanir: {}", e)).ephemeral(true)
                    )).await;
                }
            }
        }
    } else {
        let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content("❌ ID inválido fornecido.").ephemeral(true)
        )).await;
    }
}
