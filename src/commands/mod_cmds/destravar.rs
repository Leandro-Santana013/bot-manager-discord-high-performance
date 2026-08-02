use serenity::builder::{CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::channel::{PermissionOverwrite, PermissionOverwriteType};
use serenity::model::Permissions;
use serenity::model::prelude::*;
use serenity::prelude::*;

pub fn register() -> CreateCommand {
    CreateCommand::new("destravar")
        .description("Destranca o canal de voz atual, permitindo que as pessoas voltem a entrar.")
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

    if let Err(_e) = channel_id.delete_permission(&ctx.http, PermissionOverwriteType::Role(RoleId::new(guild_id.get()))).await {

        let overwrite = PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Role(RoleId::new(guild_id.get())),
        };
        let _ = channel_id.create_permission(&ctx.http, overwrite).await;
    }

    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new().content(format!("🔓 O canal <#{}> foi destravado! Membros normais podem entrar novamente.", channel_id)).ephemeral(false)
    )).await;
}
