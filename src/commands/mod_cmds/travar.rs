use serenity::builder::{CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::channel::{PermissionOverwrite, PermissionOverwriteType};
use serenity::model::Permissions;
use serenity::model::prelude::*;
use serenity::prelude::*;

pub fn register() -> CreateCommand {
    CreateCommand::new("travar")
        .description("Tranca o canal de voz que você está atualmente para impedir a entrada de novos membros.")
}

pub async fn run(ctx: &Context, interaction: &serenity::model::application::CommandInteraction) {
    let guild_id = match interaction.guild_id {
        Some(id) => id,
        None => return,
    };

    let channel_id = ctx.cache.guild(guild_id)
        .and_then(|g| g.voice_states.get(&interaction.user.id).cloned())
        .and_then(|vs| vs.channel_id);

    let channel_id = match channel_id {
        Some(id) => id,
        None => {
            let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content("❌ Você precisa estar em um canal de voz para usar este comando!").ephemeral(true)
            )).await;
            return;
        }
    };

    let overwrite = PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::CONNECT,
        kind: PermissionOverwriteType::Role(RoleId::new(guild_id.get())), // everyone role id == guild id
    };

    if let Err(e) = channel_id.create_permission(&ctx.http, overwrite).await {
        let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content(format!("❌ Falha ao travar o canal: {}", e)).ephemeral(true)
        )).await;
        return;
    }

    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new().content(format!("🔒 O canal <#{}> foi travado com sucesso! Membros normais não podem mais entrar.", channel_id)).ephemeral(false)
    )).await;
}
