use serenity::builder::{CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage, CreateChannel, CreateMessage};
use serenity::model::application::{CommandOptionType, ResolvedValue};
use serenity::model::prelude::*;
use serenity::prelude::*;

pub fn register() -> CreateCommand {
    CreateCommand::new("limpar")
        .description("Apaga mensagens do chat atual.")
        .default_member_permissions(Permissions::MANAGE_MESSAGES)
        .add_option(
            CreateCommandOption::new(CommandOptionType::Integer, "quantidade", "Quantas mensagens apagar (se não informar nada, apagará o canal inteiro!)")
                .required(false),
        )
}

pub async fn run(ctx: &Context, interaction: &serenity::model::application::CommandInteraction) {
    let mut amount = None;

    for option in &interaction.data.options() {
        if option.name == "quantidade" {
            if let ResolvedValue::Integer(v) = option.value {
                amount = Some(v);
            }
        }
    }

    let channel_id = interaction.channel_id;

    if let Some(qty) = amount {
        if qty < 1 || qty > 100 {
            let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content("Por favor, forneça um número entre 1 e 100.").ephemeral(true)
            )).await;
            return;
        }

        let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Defer(
            CreateInteractionResponseMessage::new().ephemeral(true)
        )).await;

        match channel_id.messages(&ctx.http, serenity::builder::GetMessages::new().limit(qty as u8)).await {
            Ok(messages) => {
                let message_ids: Vec<MessageId> = messages.into_iter().map(|m| m.id).collect();
                if let Err(e) = channel_id.delete_messages(&ctx.http, message_ids).await {
                    let _ = interaction.edit_response(&ctx.http, serenity::builder::EditInteractionResponse::new()
                        .content(format!("Falha ao deletar mensagens: {}", e))
                    ).await;
                } else {
                    let _ = interaction.edit_response(&ctx.http, serenity::builder::EditInteractionResponse::new()
                        .content(format!("🧹 {} mensagens deletadas com sucesso!", qty))
                    ).await;
                }
            }
            Err(e) => {
                let _ = interaction.edit_response(&ctx.http, serenity::builder::EditInteractionResponse::new()
                    .content(format!("Falha ao buscar mensagens: {}", e))
                ).await;
            }
        }
    } else {
        // Modo Nuke: Clonar e apagar o canal
        let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content("⚠️ Iniciando o protocolo de limpeza total (Nuke)...").ephemeral(true)
        )).await;

        if let Ok(channel) = channel_id.to_channel(&ctx.http).await {
            if let Some(guild_channel) = channel.guild() {
                if let Some(guild_id) = interaction.guild_id {
                    let mut builder = CreateChannel::new(guild_channel.name.clone())
                        .kind(guild_channel.kind);
                    
                    if let Some(parent_id) = guild_channel.parent_id {
                        builder = builder.category(parent_id);
                    }
                    
                    match guild_id.create_channel(&ctx.http, builder).await {
                        Ok(new_channel) => {
                            let _ = guild_channel.delete(&ctx.http).await;
                            let _ = new_channel.send_message(&ctx.http, CreateMessage::new().content("🧹 **Chat limpo com sucesso!** Todas as mensagens anteriores foram apagadas.")).await;
                            let _ = interaction.edit_response(&ctx.http, serenity::builder::EditInteractionResponse::new()
                                .content("🧹 Chat recriado com sucesso.")
                            ).await;
                        }
                        Err(e) => {
                            let _ = interaction.edit_response(&ctx.http, serenity::builder::EditInteractionResponse::new()
                                .content(format!("Houve um erro ao recriar o canal: {}", e))
                            ).await;
                        }
                    }
                }
            }
        }
    }
}
