use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, CreateEmbed,
    CreateActionRow, CreateButton, CreateSelectMenu, CreateSelectMenuOption, CreateSelectMenuKind,
};
use serenity::model::prelude::*;
use serenity::prelude::*;
use crate::database::tickets::TicketDb;

pub fn register() -> CreateCommand {
    CreateCommand::new("config_suporte")
        .description("Configura o painel interativo de suporte/tickets (Apenas Administração).")
        .default_member_permissions(Permissions::ADMINISTRATOR)
}

pub async fn build_config_suporte_panel(pool: &sqlx::PgPool) -> (CreateEmbed, Vec<CreateActionRow>) {
    let embed = CreateEmbed::new()
        .title("⚙️ Configuração do Painel de Tickets")
        .description("Gerencie os textos principais e as opções de suporte do servidor.\n\n\
            • **Editar Textos Principais**: Altera título, descrição e imagem da mensagem principal.\n\
            • **Adicionar Opção**: Cria uma nova categoria/embed de atendimento.\n\
            • **Configurações Internas**: Define o ID da Categoria, Canal de Logs e Cargo da Staff.\n\
            • **Editar / Excluir Opção**: Selecione no menu abaixo para gerenciar opções existentes.")
        .color(0x2b2d31);

    let btn_main = CreateButton::new("config_suporte_main").label("Editar Textos Principais").style(serenity::model::application::ButtonStyle::Primary).emoji('📝');
    let btn_add_opt = CreateButton::new("config_suporte_add_opt").label("Adicionar Nova Opção").style(serenity::model::application::ButtonStyle::Success).emoji('➕');
    let btn_cfg_internal = CreateButton::new("config_suporte_channels").label("Configurar Canais/Cargos").style(serenity::model::application::ButtonStyle::Secondary).emoji('⚙');

    let mut components = vec![CreateActionRow::Buttons(vec![btn_main, btn_add_opt, btn_cfg_internal])];

    let options = TicketDb::get_ticket_options(pool).await;

    if !options.is_empty() {
        let edit_options: Vec<CreateSelectMenuOption> = options.iter().map(|o| {
            let mut opt = CreateSelectMenuOption::new(o.label.chars().take(100).collect::<String>(), o.id.clone())
                .description(o.description.chars().take(100).collect::<String>());
            if let Ok(emoji) = o.emoji.parse::<ReactionType>() {
                opt = opt.emoji(emoji);
            }
            opt
        }).collect();
        let menu_edit = CreateSelectMenu::new("config_suporte_edit_opt", CreateSelectMenuKind::String { options: edit_options }).placeholder("📝 Selecione uma opção para EDITAR...");
        components.push(CreateActionRow::SelectMenu(menu_edit));

        let delete_options: Vec<CreateSelectMenuOption> = options.iter().map(|o| {
            let mut opt = CreateSelectMenuOption::new(o.label.chars().take(100).collect::<String>(), o.id.clone())
                .description(o.description.chars().take(100).collect::<String>());
            if let Ok(emoji) = o.emoji.parse::<ReactionType>() {
                opt = opt.emoji(emoji);
            }
            opt
        }).collect();
        let menu_delete = CreateSelectMenu::new("config_suporte_delete_opt", CreateSelectMenuKind::String { options: delete_options }).placeholder("❌ Selecione uma opção para EXCLUIR...");
        components.push(CreateActionRow::SelectMenu(menu_delete));
    }

    (embed, components)
}

pub async fn run(ctx: &Context, interaction: &serenity::model::application::CommandInteraction) {
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>().unwrap().clone()
    };

    let (embed, components) = build_config_suporte_panel(&pool).await;

    let response = CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(components)
        .ephemeral(true);

    if let Err(e) = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(response)).await {
        tracing::error!("Erro ao enviar menu config_suporte: {}", e);
    }
}
