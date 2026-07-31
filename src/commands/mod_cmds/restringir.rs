use serenity::builder::{CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::application::{CommandOptionType, ResolvedValue};
use serenity::model::prelude::*;
use serenity::prelude::*;
use chrono::{Utc, Duration};

pub fn register() -> CreateCommand {
    CreateCommand::new("restringir")
        .description("Restringe (castiga) um usuário por um tempo")
        .default_member_permissions(Permissions::MODERATE_MEMBERS)
        .add_option(
            CreateCommandOption::new(CommandOptionType::User, "usuario", "O usuário que vai para o castigo")
                .required(true),
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "tempo", "Tempo do castigo em minutos")
                .required(true),
        )
}

pub async fn run(ctx: &Context, interaction: &serenity::model::application::CommandInteraction) {
    let mut user_to_restrict = None;
    let mut minutes = 0;

    let options = interaction.data.options();
    for option in &options {
        match option.name {
            "usuario" => {
                if let ResolvedValue::User(user, _) = &option.value {
                    user_to_restrict = Some(user);
                }
            }
            "tempo" => {
                if let ResolvedValue::Integer(v) = &option.value {
                    minutes = *v;
                }
            }
            _ => {}
        }
    }

    if let Some(user) = user_to_restrict {
        if minutes <= 0 {
            let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content("O tempo deve ser maior que 0 minutos.").ephemeral(true)
            )).await;
            return;
        }

        if let Some(guild_id) = interaction.guild_id {
            let end_time = Utc::now() + Duration::minutes(minutes as i64);
            let timestamp = serenity::model::Timestamp::from_unix_timestamp(end_time.timestamp()).unwrap();

            // Usa o timeout (communication_disabled_until) 
            match guild_id.edit_member(&ctx.http, user.id, serenity::builder::EditMember::new().disable_communication_until(timestamp.to_string())).await {
                Ok(_) => {
                    let msg = format!("🔇 O usuário {} foi restringido por {} minutos.", user.tag(), minutes);
                    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(msg).ephemeral(true)
                    )).await;
                }
                Err(e) => {
                    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(format!("Falha ao restringir: {}", e)).ephemeral(true)
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
