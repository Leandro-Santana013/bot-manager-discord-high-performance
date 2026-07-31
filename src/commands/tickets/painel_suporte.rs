use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, CreateEmbed,
    CreateActionRow, CreateSelectMenu, CreateSelectMenuOption, CreateSelectMenuKind
};
use serenity::model::prelude::*;
use serenity::prelude::*;
use crate::database::tickets::TicketDb;

pub fn register() -> CreateCommand {
    CreateCommand::new("painel_suporte")
        .description("Envia o painel de suporte interativo neste canal (Apenas Administração).")
        .default_member_permissions(Permissions::ADMINISTRATOR)
}

pub async fn run(ctx: &Context, interaction: &serenity::model::application::CommandInteraction) {
    let data = ctx.data.read().await;
    let db_pool = data.get::<crate::DatabasePool>().expect("DB pool not initialized");
    
    let title = TicketDb::get_config(db_pool, "panel_title", "Central de Ajuda").await;
    let desc = TicketDb::get_config(db_pool, "panel_description", "Nessa seção, você pode tirar suas dúvidas ou entrar em contato com a nossa equipe de Suporte.\n\nPara evitar problemas, leia as opções com atenção e selecione o motivo do seu contato no menu abaixo.").await;
    let img = TicketDb::get_config(db_pool, "panel_image", "").await;

    let mut embed = CreateEmbed::new()
        .title(title)
        .description(desc)
        .color(0x5865F2);

    if !img.is_empty() && img.starts_with("http") {
        embed = embed.image(img);
    }

    let options = TicketDb::get_ticket_options(db_pool).await;
    let mut select_options = Vec::new();

    for opt in options {
        let mut menu_opt = CreateSelectMenuOption::new(opt.label, opt.id)
            .description(opt.description);
        
        if let Ok(emoji) = opt.emoji.parse::<ReactionType>() {
            menu_opt = menu_opt.emoji(emoji);
        }
        
        select_options.push(menu_opt);
    }

    if select_options.is_empty() {
        select_options.push(
            CreateSelectMenuOption::new("Nenhuma opção disponível", "none")
                .description("Nenhuma opção configurada no momento.")
        );
    }

    let select_menu = CreateSelectMenu::new(
        "menu_ajuda",
        CreateSelectMenuKind::String { options: select_options }
    )
    .placeholder("Selecione uma opção...");

    let action_row = CreateActionRow::SelectMenu(select_menu);

    if let Err(e) = interaction.channel_id.send_message(&ctx.http, serenity::builder::CreateMessage::new()
        .embed(embed)
        .components(vec![action_row])
    ).await {
        let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content(format!("Erro: {}", e)).ephemeral(true)
        )).await;
        return;
    }

    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new().content("✅ Painel de tickets enviado com sucesso neste canal!").ephemeral(true)
    )).await;
}

pub async fn run_message(ctx: &Context, msg: &Message) {
    let data = ctx.data.read().await;
    let db_pool = data.get::<crate::DatabasePool>().expect("DB pool not initialized");
    
    let title = TicketDb::get_config(db_pool, "panel_title", "Central de Ajuda").await;
    let desc = TicketDb::get_config(db_pool, "panel_description", "Nessa seção, você pode tirar suas dúvidas ou entrar em contato com a nossa equipe de Suporte.\n\nPara evitar problemas, leia as opções com atenção e selecione o motivo do seu contato no menu abaixo.").await;
    let img = TicketDb::get_config(db_pool, "panel_image", "").await;

    let mut embed = CreateEmbed::new()
        .title(title)
        .description(desc)
        .color(0x5865F2);

    if !img.is_empty() && img.starts_with("http") {
        embed = embed.image(img);
    }

    let options = TicketDb::get_ticket_options(db_pool).await;
    let mut select_options = Vec::new();

    for opt in options {
        let mut menu_opt = CreateSelectMenuOption::new(opt.label, opt.id)
            .description(opt.description);
        
        if let Ok(emoji) = opt.emoji.parse::<ReactionType>() {
            menu_opt = menu_opt.emoji(emoji);
        }
        
        select_options.push(menu_opt);
    }

    if select_options.is_empty() {
        select_options.push(
            CreateSelectMenuOption::new("Nenhuma opção disponível", "none")
                .description("Nenhuma opção configurada no momento.")
        );
    }

    let select_menu = CreateSelectMenu::new(
        "menu_ajuda",
        CreateSelectMenuKind::String { options: select_options }
    )
    .placeholder("Selecione uma opção...");

    let action_row = CreateActionRow::SelectMenu(select_menu);

    let _ = msg.channel_id.send_message(&ctx.http, serenity::builder::CreateMessage::new()
        .embed(embed)
        .components(vec![action_row])
    ).await;
}
