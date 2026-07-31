use serenity::builder::{CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, CreateEmbed, CreateActionRow, CreateButton, CreateSelectMenu, CreateSelectMenuOption, CreateSelectMenuKind};
use serenity::model::prelude::*;
use serenity::prelude::*;
use crate::database::vip::VipDb;

pub fn register() -> CreateCommand {
    CreateCommand::new("config_vip")
        .description("Edita os textos e opções do painel VIP (Apenas Administração).")
        .default_member_permissions(Permissions::ADMINISTRATOR)
}

pub async fn build_config_vip_panel(pool: &sqlx::PgPool) -> (CreateEmbed, Vec<CreateActionRow>) {
    let embed = CreateEmbed::new()
        .title("⚙️ Configuração do Painel VIP")
        .description("Gerencie os textos, blocos de mensagens e os pacotes VIP do servidor.\n\n\
            • **Editar Texto Principal**: Altera a descrição e a imagem do embed principal do Painel VIP.\n\
            • **Adicionar Bloco Extra**: Cria um novo bloco de texto (embed extra) que aparecerá acima das opções.\n\
            • **Adicionar Pacote VIP**: Cria um novo pacote no menu de seleção VIP.\n\
            • **Gerenciar (Editar/Excluir)**: Use os menus suspensos abaixo para editar ou excluir Blocos Extras ou Pacotes VIP existentes.")
        .color(0x2b2d31);

    let btn_main = CreateButton::new("config_vip_main").label("Editar Texto Principal").style(serenity::model::application::ButtonStyle::Primary).emoji('📝');
    let btn_add_block = CreateButton::new("config_vip_add_block").label("Adicionar Bloco Extra").style(serenity::model::application::ButtonStyle::Success).emoji('➕');
    let btn_add_prod = CreateButton::new("config_vip_add_prod").label("Adicionar Pacote VIP").style(serenity::model::application::ButtonStyle::Secondary).emoji('🛒');
    let mut components = vec![CreateActionRow::Buttons(vec![btn_main, btn_add_block, btn_add_prod])];

    let blocks = VipDb::get_extra_blocks(pool).await;
    if !blocks.is_empty() {
        let options: Vec<CreateSelectMenuOption> = blocks.into_iter().map(|b| {
            CreateSelectMenuOption::new(b.title.chars().take(100).collect::<String>(), b.id).description(b.desc.chars().take(100).collect::<String>()).emoji('📄')
        }).collect();
        let menu = CreateSelectMenu::new("config_vip_edit_block", CreateSelectMenuKind::String { options }).placeholder("📝 Editar ou Excluir Bloco Extra...");
        components.push(CreateActionRow::SelectMenu(menu));
    }

    let prods = VipDb::get_products(pool).await;
    if !prods.is_empty() {
        let options: Vec<CreateSelectMenuOption> = prods.into_iter().map(|p| {
            CreateSelectMenuOption::new(p.label.chars().take(100).collect::<String>(), p.id).description(format!("Preço: R$ {}", p.price)).emoji('💎')
        }).collect();
        let menu = CreateSelectMenu::new("config_vip_edit_prod", CreateSelectMenuKind::String { options }).placeholder("💎 Editar ou Excluir Pacote VIP...");
        components.push(CreateActionRow::SelectMenu(menu));
    }

    (embed, components)
}

pub async fn run(ctx: &Context, interaction: &serenity::model::application::CommandInteraction) {
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>().unwrap().clone()
    };

    let (embed, components) = build_config_vip_panel(&pool).await;

    let response = CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(components)
        .ephemeral(true);

    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(response)).await;
}

pub async fn run_message(ctx: &Context, msg: &Message) {
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>().unwrap().clone()
    };

    let (embed, components) = build_config_vip_panel(&pool).await;
    let _ = msg.channel_id.send_message(&ctx.http, serenity::builder::CreateMessage::new().embed(embed).components(components)).await;
}
