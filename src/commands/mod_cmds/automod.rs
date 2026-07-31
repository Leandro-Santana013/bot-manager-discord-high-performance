use serenity::builder::{CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::application::CommandOptionType;
use serenity::model::prelude::*;
use serenity::prelude::*;
use crate::database::tickets::TicketDb;

pub fn register() -> CreateCommand {
    CreateCommand::new("automod")
        .description("Filtro automático de mensagens (Apenas Admins).")
        .default_member_permissions(Permissions::ADMINISTRATOR)
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "add", "Adiciona uma palavra ao filtro.")
                .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "palavra", "A palavra para censurar").required(true))
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "list", "Lista todas as palavras censuradas.")
        )
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "remove", "Remove uma palavra do filtro.")
                .add_sub_option(CreateCommandOption::new(CommandOptionType::String, "palavra", "A palavra para remover da censura").required(true))
        )
}

pub async fn run(ctx: &Context, interaction: &serenity::model::application::CommandInteraction) {
    if !interaction.member.as_ref().map(|m| m.permissions.unwrap_or(Permissions::empty()).administrator()).unwrap_or(false) {
        let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content("❌ Apenas administradores podem usar isso.").ephemeral(true)
        )).await;
        return;
    }

    let data = ctx.data.read().await;
    let pool = data.get::<crate::DatabasePool>().expect("DB pool not initialized").clone();

    let options = interaction.data.options();
    if let Some(subcommand) = options.first() {
        if let serenity::model::application::ResolvedValue::SubCommand(sub_opts) = &subcommand.value {
            match subcommand.name {
                "add" => {
                    let mut palavra = String::new();
                    if let Some(opt) = sub_opts.first() {
                        if let serenity::model::application::ResolvedValue::String(s) = &opt.value {
                            palavra = s.to_lowercase();
                        }
                    }
                    
                    let raw_list = TicketDb::get_config(&pool, "automod_words", "[]").await;
                    let mut words: Vec<String> = serde_json::from_str(&raw_list).unwrap_or_else(|_| vec![]);
                    
                    if words.contains(&palavra) {
                        let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().content(format!("⚠️ A palavra `{}` já está no filtro.", palavra)).ephemeral(true)
                        )).await;
                        return;
                    }

                    words.push(palavra.clone());
                    let new_raw = serde_json::to_string(&words).unwrap();
                    let _ = TicketDb::set_config(&pool, "automod_words", &new_raw).await;

                    // Atualizar cache em memória
                    if let Some(cache) = data.get::<crate::AutomodCache>() {
                        *cache.write().await = words;
                    }

                    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content(format!("✅ A palavra `{}` foi adicionada ao Automod. Mensagens contendo ela serão deletadas.", palavra)).ephemeral(true)
                    )).await;
                }
                "remove" => {
                    let mut palavra = String::new();
                    if let Some(opt) = sub_opts.first() {
                        if let serenity::model::application::ResolvedValue::String(s) = &opt.value {
                            palavra = s.to_lowercase();
                        }
                    }
                    
                    let raw_list = TicketDb::get_config(&pool, "automod_words", "[]").await;
                    let mut words: Vec<String> = serde_json::from_str(&raw_list).unwrap_or_else(|_| vec![]);
                    
                    if let Some(pos) = words.iter().position(|x| *x == palavra) {
                        words.remove(pos);
                        let new_raw = serde_json::to_string(&words).unwrap();
                        let _ = TicketDb::set_config(&pool, "automod_words", &new_raw).await;

                        // Atualizar cache em memória
                        if let Some(cache) = data.get::<crate::AutomodCache>() {
                            *cache.write().await = words.clone();
                        }

                        let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().content(format!("✅ A palavra `{}` foi removida do filtro.", palavra)).ephemeral(true)
                        )).await;
                    } else {
                        let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().content(format!("⚠️ A palavra `{}` não foi encontrada no filtro.", palavra)).ephemeral(true)
                        )).await;
                    }
                }
                "list" => {
                    let raw_list = TicketDb::get_config(&pool, "automod_words", "[]").await;
                    let words: Vec<String> = serde_json::from_str(&raw_list).unwrap_or_else(|_| vec![]);

                    if words.is_empty() {
                        let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().content("O filtro do Automod está vazio.").ephemeral(true)
                        )).await;
                    } else {
                        let words_list = words.iter().map(|w| format!("- `{}`", w)).collect::<Vec<String>>().join("\n");
                        let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new().content(format!("🛡️ **Palavras censuradas pelo Automod:**\n{}", words_list)).ephemeral(true)
                        )).await;
                    }
                }
                _ => {}
            }
        }
    }
}
