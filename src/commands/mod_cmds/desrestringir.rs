use serenity::builder::{CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::application::{CommandOptionType, ResolvedValue};
use serenity::model::prelude::*;
use serenity::prelude::*;

pub fn register() -> CreateCommand {
    CreateCommand::new("desrestringir")
        .description("Remove o castigo (Timeout) de um usuário.")
        .default_member_permissions(Permissions::MODERATE_MEMBERS)
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "usuario", "O usuário que terá o castigo removido")
                .required(true),
        )
}

pub async fn run(ctx: &Context, interaction: &serenity::model::application::CommandInteraction) {
    let mut target_user = None;

    let options = interaction.data.options();
    for option in &options {
        if option.name == "usuario" {
            if let ResolvedValue::User(user, _) = &option.value {
                target_user = Some(user);
            }
        }
    }

    if let Some(user) = target_user {
        if let Some(guild_id) = interaction.guild_id {
            match guild_id.edit_member(&ctx.http, user.id, serenity::builder::EditMember::new().enable_communication()).await {
                Ok(_) => {
                    let msg = format!("✅ O usuário <@{}> foi desrestringido e pode falar novamente.", user.id.get());
                    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(msg).ephemeral(true)
                    )).await;
                }
                Err(e) => {
                    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(format!("❌ Falha ao desrestringir: {}", e)).ephemeral(true)
                    )).await;
                }
            }
        }
    } else {
        let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content("❌ Usuário inválido.").ephemeral(true)
        )).await;
    }
}
