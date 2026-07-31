use serenity::builder::{CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::application::{CommandOptionType, ResolvedValue};
use serenity::prelude::*;
use tracing::error;

pub fn register() -> CreateCommand {
    CreateCommand::new("call")
        .description("Altera o limite de vagas da sua sala de voz.")
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "limite", "O novo limite de usuários para a sala (0 a 99)")
                .required(true)
                .min_int_value(0)
                .max_int_value(99),
        )
}

pub async fn run(ctx: &Context, interaction: &serenity::model::application::CommandInteraction) {
    let mut limit = 0;

    let options = interaction.data.options();
    for option in &options {
        if option.name == "limite" {
            if let ResolvedValue::Integer(v) = &option.value {
                limit = *v as u32;
            }
        }
    }

    if let Some(guild_id) = interaction.guild_id {
        let channel_id = ctx.cache.guild(guild_id)
            .and_then(|g| g.voice_states.get(&interaction.user.id).cloned())
            .and_then(|vs| vs.channel_id);
            
        if let Some(channel_id) = channel_id {
            let edit = serenity::builder::EditChannel::new().user_limit(limit);
            match channel_id.edit(&ctx.http, edit).await {
                Ok(_) => {
                    let msg = if limit == 0 {
                        "✅ Limite de vagas removido da sua sala.".to_string()
                    } else {
                        format!("✅ O limite da sua sala foi alterado para **{} vagas**.", limit)
                    };
                    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(msg).ephemeral(true)
                    )).await;
                    return;
                }
                Err(e) => {
                    error!("Erro ao alterar limite da call: {}", e);
                    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content("❌ Erro ao alterar o limite da sala. O bot tem permissão?").ephemeral(true)
                    )).await;
                    return;
                }
            }
        }
    }

    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new().content("❌ Você precisa estar em uma sala de voz para usar este comando!").ephemeral(true)
    )).await;
}
