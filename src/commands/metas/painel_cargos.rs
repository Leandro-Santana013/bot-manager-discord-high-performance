use serenity::builder::{CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, CreateEmbed, CreateActionRow, CreateSelectMenu, CreateSelectMenuKind};
use serenity::model::prelude::*;
use serenity::prelude::*;

pub fn register() -> CreateCommand {
    CreateCommand::new("painelcargos")
        .description("Envia o painel de gerenciamento de cargos (Apenas Staff).")
        .default_member_permissions(Permissions::ADMINISTRATOR)
}

pub async fn run(ctx: &Context, interaction: &serenity::model::application::CommandInteraction) {
    let staff_roles: Vec<RoleId> = vec![
        RoleId::new(1528880766979936399),
        RoleId::new(1496150278108479629),
        RoleId::new(1528910395656507392),
        RoleId::new(1528884120439095537),
    ];

    let mut has_staff_role = false;
    if let Some(member) = &interaction.member {
        if member.permissions.unwrap_or(Permissions::empty()).administrator() {
            has_staff_role = true;
        } else {
            for role in &member.roles {
                if staff_roles.contains(role) {
                    has_staff_role = true;
                    break;
                }
            }
        }
    }

    if !has_staff_role {
        let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content("❌ Apenas membros da equipe podem usar o painel de cargos.")
                .ephemeral(true)
        )).await;
        return;
    }

    let embed = CreateEmbed::new()
        .title("Gerenciamento de Cargos 🛡️")
        .description("Selecione um membro no menu abaixo para gerenciar os cargos dele.\n\n⚠️ **Atenção:** Só adicione cargos autorizados.")
        .color(0x2F3136);

    let select_menu = CreateSelectMenu::new(
        "menu_selecionar_usuario_cargo",
        CreateSelectMenuKind::User { default_users: None }
    )
    .placeholder("Selecione o membro...");

    let action_row = CreateActionRow::SelectMenu(select_menu);

    if let Err(e) = interaction.channel_id.send_message(&ctx.http, serenity::builder::CreateMessage::new()
        .embed(embed)
        .components(vec![action_row])
    ).await {
        let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content(format!("Erro ao enviar: {}", e)).ephemeral(true)
        )).await;
        return;
    }

    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new().content("✅ Painel de gerenciamento de cargos enviado com sucesso!").ephemeral(true)
    )).await;
}
