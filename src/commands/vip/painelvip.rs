use serenity::builder::{CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, CreateEmbed, CreateActionRow, CreateSelectMenu, CreateSelectMenuOption, CreateSelectMenuKind, EditMessage};
use serenity::model::prelude::*;
use serenity::prelude::*;
use crate::database::vip::VipDb;

pub fn register() -> CreateCommand {
    CreateCommand::new("painelvip")
        .description("Envia o painel de compra de VIP (Apenas Staff).")
        .default_member_permissions(Permissions::ADMINISTRATOR)
}

pub async fn build_vip_message(pool: &sqlx::PgPool) -> (Vec<CreateEmbed>, Vec<CreateActionRow>) {
    let mut embeds = vec![];

    let extra_blocks = VipDb::get_extra_blocks(pool).await;
    for block in extra_blocks {
        let color_hex = block.color.trim_start_matches('#');
        let color_int = u32::from_str_radix(color_hex, 16).unwrap_or(0x2F3136);

        let embed = CreateEmbed::new()
            .title(block.title)
            .description(block.desc)
            .color(color_int);
        embeds.push(embed);
    }

    let main_text = VipDb::get_main_text(pool).await;
    let main_image = VipDb::get_main_image(pool).await;

    let mut main_embed = CreateEmbed::new()
        .description(main_text)
        .color(0x2F3136);

    if !main_image.is_empty() {
        main_embed = main_embed.image(main_image);
    }
    embeds.push(main_embed);

    let prods = VipDb::get_products(pool).await;
    let mut select_options = vec![];

    for prod in prods {
        select_options.push(CreateSelectMenuOption::new(
            prod.label.chars().take(100).collect::<String>(),
            prod.id
        ));
    }

    select_options.push(CreateSelectMenuOption::new("Adicionar saldo", "vip_add_saldo"));

    let select_menu = CreateSelectMenu::new("menu_vip", CreateSelectMenuKind::String { options: select_options })
        .placeholder("Selecione uma opção VIP...");

    let row = CreateActionRow::SelectMenu(select_menu);

    (embeds, vec![row])
}

pub async fn run(ctx: &Context, interaction: &serenity::model::application::CommandInteraction) {
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>().unwrap().clone()
    };

    let (embeds, components) = build_vip_message(&pool).await;

    let response = serenity::builder::CreateMessage::new()
        .embeds(embeds)
        .components(components);

    match interaction.channel_id.send_message(&ctx.http, response).await {
        Ok(msg) => {
            VipDb::set_panel_message(&pool, &msg.channel_id.to_string(), &msg.id.to_string()).await;

            let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content("✅ Painel VIP enviado com sucesso!").ephemeral(true)
            )).await;
        }
        Err(e) => {
            let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(format!("❌ Erro ao enviar painel: {}", e)).ephemeral(true)
            )).await;
        }
    }
}

pub async fn update_panel(ctx: &Context) {
    let pool = {
        let data = ctx.data.read().await;
        if let Some(p) = data.get::<crate::DatabasePool>() {
            p.clone()
        } else {
            return;
        }
    };

    let (channel_id_str, message_id_str) = VipDb::get_panel_message(&pool).await;
    if channel_id_str.is_empty() || message_id_str.is_empty() { return; }

    if let (Ok(channel_id), Ok(message_id)) = (channel_id_str.parse::<u64>(), message_id_str.parse::<u64>()) {
        let (embeds, components) = build_vip_message(&pool).await;
        let edit_msg = EditMessage::new().embeds(embeds).components(components);

        let _ = ctx.http.edit_message(serenity::model::id::ChannelId::new(channel_id), serenity::model::id::MessageId::new(message_id), &edit_msg, vec![]).await;
    }
}
