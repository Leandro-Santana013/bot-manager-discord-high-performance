use serenity::model::application::Interaction;
use serenity::prelude::*;
use tracing::{info, error};

pub async fn handle(ctx: Context, interaction: Interaction) {
    if let Interaction::Command(command) = interaction {
        info!("Recebeu comando: {}", command.data.name);
        crate::commands::handle_command(&ctx, &command).await;
    } else if let Interaction::Component(component) = interaction {

        info!("Recebeu interação de componente: {}", component.data.custom_id);

        let staff_roles: Vec<serenity::model::id::RoleId> = vec![
            serenity::model::id::RoleId::new(1528880766979936399),
            serenity::model::id::RoleId::new(1496150278108479629),
            serenity::model::id::RoleId::new(1528910395656507392),
            serenity::model::id::RoleId::new(1528884120439095537),
        ];

        let mut has_staff_role = false;
        if let Some(member) = &component.member {
            if member.permissions.unwrap_or(serenity::model::Permissions::empty()).administrator() {
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

        if component.data.custom_id == "menu_selecionar_usuario_cargo" {
            use serenity::model::application::ComponentInteractionDataKind;
            use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage, CreateActionRow, CreateSelectMenu, CreateSelectMenuKind};

            if !has_staff_role {
                let _ = component.create_response(&ctx.http, CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content("❌ Apenas membros da equipe podem usar este painel.").ephemeral(true)
                )).await;
                return;
            }

            let target_user_id = match &component.data.kind {
                ComponentInteractionDataKind::UserSelect { values } => {
                    if let Some(user_id) = values.first() {
                        user_id.get()
                    } else {
                        return;
                    }
                }
                _ => return,
            };

            let select_menu = CreateSelectMenu::new(
                format!("menu_selecionar_cargo_{}", target_user_id),
                CreateSelectMenuKind::Role { default_roles: None }
            )
            .placeholder("Selecione os cargos para adicionar/remover...")
            .min_values(1)
            .max_values(5);

            let action_row = CreateActionRow::SelectMenu(select_menu);

            let msg_content = format!("👤 **Usuário Selecionado:** <@{}>\nSelecione no menu abaixo os cargos que deseja aplicar ou remover. O bot fará a troca automaticamente!", target_user_id);

            let _ = component.create_response(&ctx.http, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(msg_content)
                    .components(vec![action_row])
                    .ephemeral(true)
            )).await;
        } else if component.data.custom_id.starts_with("menu_selecionar_cargo_") {
            use serenity::model::application::ComponentInteractionDataKind;
            use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage, EditInteractionResponse};

            if !has_staff_role {
                let _ = component.create_response(&ctx.http, CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content("❌ Apenas membros da equipe podem alterar cargos.").ephemeral(true)
                )).await;
                return;
            }

            let Some(target_user_id_str) = component.data.custom_id.strip_prefix("menu_selecionar_cargo_") else { return; };
            let target_user_id: u64 = target_user_id_str.parse().unwrap_or(0);

            let selected_roles = match &component.data.kind {
                ComponentInteractionDataKind::RoleSelect { values } => values.clone(),
                _ => return,
            };

            let _ = component.create_response(&ctx.http, CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true)
            )).await;

            if let Some(guild_id) = component.guild_id {
                if let Ok(member) = guild_id.member(&ctx.http, target_user_id).await {
                    let mut added = vec![];
                    let mut removed = vec![];

                    for role_id in selected_roles {
                        if member.roles.contains(&role_id) {
                            if member.remove_role(&ctx.http, role_id).await.is_ok() {
                                removed.push(format!("<@&{}>", role_id.get()));
                            }
                        } else {
                            if member.add_role(&ctx.http, role_id).await.is_ok() {
                                added.push(format!("<@&{}>", role_id.get()));
                            }
                        }
                    }

                    let mut msg = format!("⚙️ **Alterações no membro <@{}>:**\n", target_user_id);
                    if !added.is_empty() {
                        msg.push_str(&format!("✅ **Adicionados:** {}\n", added.join(", ")));
                    }
                    if !removed.is_empty() {
                        msg.push_str(&format!("❌ **Removidos:** {}\n", removed.join(", ")));
                    }
                    if added.is_empty() && removed.is_empty() {
                        msg.push_str("Nenhuma alteração permitida foi feita (Cargos muito altos?).");
                    }

                    let _ = component.edit_response(&ctx.http, EditInteractionResponse::new().content(msg)).await;
                }
            }
        } else if component.data.custom_id == "menu_ajuda" {
            use serenity::model::application::ComponentInteractionDataKind;
            use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage, CreateEmbed, CreateActionRow, CreateButton};
            use serenity::model::application::ButtonStyle;
            use crate::database::tickets::TicketDb;

            let selected_option_id = match &component.data.kind {
                ComponentInteractionDataKind::StringSelect { values } => {
                    if let Some(val) = values.first() {
                        val.clone()
                    } else {
                        return;
                    }
                }
                _ => return,
            };

            if selected_option_id == "none" {
                let _ = component.create_response(&ctx.http, CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content("❌ Nenhuma opção configurada no momento.").ephemeral(true)
                )).await;
                return;
            }

            let data = ctx.data.read().await;
            let db_pool = data.get::<crate::DatabasePool>().expect("DB pool not initialized");
            let options = TicketDb::get_ticket_options(db_pool).await;

            let option = options.into_iter().find(|o| o.id == selected_option_id);
            let descricao = option.map(|o| o.reply).unwrap_or_else(|| "Clique no botão abaixo para abrir o seu ticket.".to_string());
            let id_botao = format!("abrir_ticket_{}", selected_option_id);

            let embed = CreateEmbed::new()
                .description(descricao)
                .color(0x5865F2);

            let row_button = CreateActionRow::Buttons(vec![
                CreateButton::new(id_botao)
                    .label("Abrir Ticket")
                    .style(ButtonStyle::Primary)
                    .emoji('🎫')
            ]);

            let _ = component.create_response(&ctx.http, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(embed).components(vec![row_button]).ephemeral(true)
            )).await;

        } else if component.data.custom_id.starts_with("abrir_ticket_") {
            use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage, EditInteractionResponse, CreateChannel, CreateMessage, CreateEmbed, CreateActionRow, CreateButton};
            use serenity::model::application::ButtonStyle;
            use serenity::model::channel::{ChannelType, PermissionOverwrite, PermissionOverwriteType};
            use serenity::model::Permissions;

            let Some(tipo_ticket) = component.data.custom_id.strip_prefix("abrir_ticket_").map(|s| s.to_uppercase()) else { return; };

            let _ = component.create_response(&ctx.http, CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true)
            )).await;

            let Some(guild_id) = component.guild_id else { return; };
            let user_id = component.user.id;
            let Some(db_pool) = ({
                let data = ctx.data.read().await;
                data.get::<crate::DatabasePool>().cloned()
            }) else { return; };
            let conf_cat = crate::database::tickets::TicketDb::get_config(&db_pool, "ticket_category_id", "1528913685739733053").await;
            let category_id = serenity::model::id::ChannelId::new(conf_cat.parse::<u64>().unwrap_or(1528913685739733053));

            let conf_staff = crate::database::tickets::TicketDb::get_config(&db_pool, "ticket_staff_role", "").await;

            let everyone_role = serenity::model::id::RoleId::new(guild_id.get());

            let current_user_id = match ctx.http.get_current_user().await {
                Ok(u) => u.id,
                Err(e) => {
                    tracing::error!("Erro ao obter current user: {}", e);
                    return;
                }
            };

            let mut permissions = vec![
                PermissionOverwrite {
                    allow: Permissions::empty(),
                    deny: Permissions::VIEW_CHANNEL,
                    kind: PermissionOverwriteType::Role(everyone_role),
                },
                PermissionOverwrite {
                    allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY,
                    deny: Permissions::empty(),
                    kind: PermissionOverwriteType::Member(user_id),
                },
                PermissionOverwrite {
                    allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY | Permissions::MANAGE_CHANNELS,
                    deny: Permissions::empty(),
                    kind: PermissionOverwriteType::Member(current_user_id),
                }
            ];

            for role_id in &staff_roles {
                permissions.push(PermissionOverwrite {
                    allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY,
                    deny: Permissions::empty(),
                    kind: PermissionOverwriteType::Role(*role_id),
                });
            }

            if let Ok(id) = conf_staff.parse::<u64>() {
                permissions.push(PermissionOverwrite {
                    allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY,
                    deny: Permissions::empty(),
                    kind: PermissionOverwriteType::Role(serenity::model::id::RoleId::new(id)),
                });
            }

            match guild_id.create_channel(&ctx.http, CreateChannel::new(format!("ticket-{}", component.user.name))
                .kind(ChannelType::Text)
                .category(category_id)
                .permissions(permissions)
            ).await {
                Ok(channel) => {
                    let embed = CreateEmbed::new()
                        .title(format!("🎫 Ticket Aberto - {}", tipo_ticket))
                        .description(format!("Olá <@{}>, sua solicitação de **{}** foi recebida.\n\nAguarde o atendimento da equipe. Enquanto isso, sinta-se à vontade para enviar mais detalhes, fotos ou prints.", user_id, tipo_ticket))
                        .color(0x2b2d31);

                    let row = CreateActionRow::Buttons(vec![
                        CreateButton::new("assumir_ticket")
                            .label("Assumir Ticket")
                            .style(ButtonStyle::Success)
                            .emoji('🙋'),
                        CreateButton::new("fechar_ticket")
                            .label("Fechar Ticket")
                            .style(ButtonStyle::Danger)
                            .emoji('🔒'),
                    ]);

                    let _ = channel.send_message(&ctx.http, CreateMessage::new().content(format!("<@{}>", user_id)).embed(embed).components(vec![row])).await;

                    let _ = component.edit_response(&ctx.http, EditInteractionResponse::new().content(format!("✅ Seu ticket foi criado com sucesso! Acesse aqui: <#{}>", channel.id))).await;
                }
                Err(e) => {
                    error!("Erro ao criar ticket: {}", e);
                    let _ = component.edit_response(&ctx.http, EditInteractionResponse::new().content("❌ Ocorreu um erro ao criar o seu ticket.")).await;
                }
            }
        } else if component.data.custom_id == "assumir_ticket" {
            use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage, EditChannel, CreateActionRow, CreateButton};
            use serenity::model::application::ButtonStyle;
            use serenity::model::Permissions;

            let staff_user_id = component.user.id;
            let channel_id = component.channel_id;

            let edit_channel = EditChannel::new().topic(format!("ASSUMIDO_POR:{}", staff_user_id));
            let _ = channel_id.edit(&ctx.http, edit_channel).await;

            for role_id in &staff_roles {
                let _ = channel_id.create_permission(&ctx.http, serenity::model::channel::PermissionOverwrite {
                    allow: Permissions::empty(),
                    deny: Permissions::VIEW_CHANNEL,
                    kind: serenity::model::channel::PermissionOverwriteType::Role(*role_id)
                }).await;
            }

            let _ = channel_id.create_permission(&ctx.http, serenity::model::channel::PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES | Permissions::READ_MESSAGE_HISTORY,
                deny: Permissions::empty(),
                kind: serenity::model::channel::PermissionOverwriteType::Member(staff_user_id)
            }).await;

            let message = component.message.clone();
            if let Some(old_embed) = message.embeds.first() {
                use serenity::builder::CreateEmbed;
                let mut new_embed = CreateEmbed::from(old_embed.clone())
                    .color(0xFEE75C);

                new_embed = new_embed.field("Atendente Atual:", format!("<@{}>", staff_user_id), false);

                let row = CreateActionRow::Buttons(vec![
                    CreateButton::new("fechar_ticket")
                        .label("Fechar Ticket")
                        .style(ButtonStyle::Danger)
                        .emoji('🔒'),
                ]);

                let _ = component.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new().embed(new_embed).components(vec![row])
                )).await;
            }

            let _ = channel_id.send_message(&ctx.http, serenity::builder::CreateMessage::new().content(format!("✅ Ticket assumido por <@{}>. Apenas ele e o dono do ticket têm acesso a essa sala agora.", staff_user_id))).await;

        } else if component.data.custom_id == "fechar_ticket" {
            use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage};
            use crate::database::tickets::TicketDb;

            let channel_id = component.channel_id;
            let mut staff_id_assumed = None;

            if let Ok(channel) = channel_id.to_channel(&ctx.http).await {
                if let Some(guild_channel) = channel.guild() {
                    if let Some(topic) = guild_channel.topic {
                        if topic.starts_with("ASSUMIDO_POR:") {
                            let parts: Vec<&str> = topic.split(':').collect();
                            if parts.len() > 1 {
                                staff_id_assumed = Some(parts[1].to_string());
                            }

                            let mut owner_id = String::new();
                            if let Some(embed) = component.message.embeds.first() {
                                if let Some(desc) = &embed.description {
                                    if let Some(start) = desc.find("<@") {
                                        if let Some(end) = desc[start..].find('>') {
                                            owner_id = desc[start+2..start+end].to_string();
                                        }
                                    }
                                }
                            }

                            let user_id_str = component.user.id.to_string();
                            if Some(user_id_str.clone()) != staff_id_assumed && user_id_str != owner_id {
                                let _ = component.create_response(&ctx.http, CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new().content("❌ Você não pode fechar este ticket! Ele já foi assumido por outro atendente.").ephemeral(true)
                                )).await;
                                return;
                            }
                        }
                    }
                }
            }

            let _ = component.create_response(&ctx.http, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content("Fechando o ticket em 5 segundos...")
            )).await;

            let Some(db_pool) = ({
                let data = ctx.data.read().await;
                data.get::<crate::DatabasePool>().cloned()
            }) else { return; };

            if let Some(staff_id_str) = &staff_id_assumed {
                if let Err(e) = TicketDb::add_ticket(&db_pool, staff_id_str).await {
                    error!("Erro ao salvar ponto do ticket: {}", e);
                } else {
                    info!("Ponto computado para o staff {}", staff_id_str);
                }
            }

            let logs_channel = TicketDb::get_config(&db_pool, "ticket_logs_channel", "").await;

            let ctx_clone = ctx.clone();
            let staff_assumed_clone = staff_id_assumed.clone();

            let channel_name = match channel_id.to_channel(&ctx.http).await {
                Ok(serenity::model::channel::Channel::Guild(c)) => c.name,
                _ => "desconhecido".to_string(),
            };

            tokio::spawn(async move {
                if let Ok(log_id) = logs_channel.parse::<u64>() {
                    let log_chan = serenity::model::id::ChannelId::new(log_id);
                    let msg = format!("🔒 **Ticket Fechado:** `{}`\nAtendente: {}", channel_name, staff_assumed_clone.map(|s| format!("<@{}>", s)).unwrap_or_else(|| "Nenhum".to_string()));
                    let _ = log_chan.send_message(&ctx_clone.http, serenity::builder::CreateMessage::new().content(msg)).await;
                }

                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                let _ = channel_id.delete(&ctx_clone.http).await;
            });
        } else if component.data.custom_id.starts_with("config_vip_") {
            use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage, CreateModal, CreateActionRow, CreateInputText};
            use serenity::model::application::InputTextStyle;
            use crate::database::vip::VipDb;

            if !has_staff_role {
                let _ = component.create_response(&ctx.http, CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content("❌ Apenas administradores podem configurar o VIP.").ephemeral(true)
                )).await;
                return;
            }

            let Some(pool) = ({
                let data = ctx.data.read().await;
                data.get::<crate::DatabasePool>().cloned()
            }) else { return; };

            if component.data.custom_id == "config_vip_main" {
                let main_desc = VipDb::get_main_text(&pool).await;
                let main_img = VipDb::get_main_image(&pool).await;

                let modal = CreateModal::new("modal_config_vip_main", "Configurar Painel VIP")
                    .components(vec![
                        CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Paragraph, "Descrição do Painel", "main_desc").value(main_desc).required(true)),
                        CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Link da Imagem (Opcional)", "main_img").value(main_img).required(false)),
                    ]);
                let _ = component.create_response(&ctx.http, CreateInteractionResponse::Modal(modal)).await;
            } else if component.data.custom_id == "config_vip_add_block" {
                let modal = CreateModal::new("modal_config_vip_add_block", "Novo Bloco de Mensagem")
                    .components(vec![
                        CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "ID (sem espaços)", "block_id").required(true)),
                        CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Título", "block_title").required(true)),
                        CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Paragraph, "Descrição", "block_desc").required(true)),
                        CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Cor (Hex, ex: #ff0000)", "block_color").value("#2F3136").required(false)),
                    ]);
                let _ = component.create_response(&ctx.http, CreateInteractionResponse::Modal(modal)).await;
            } else if component.data.custom_id == "config_vip_add_prod" {
                let modal = CreateModal::new("modal_config_vip_add_prod", "Novo Produto VIP")
                    .components(vec![
                        CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "ID (ex: vip_gold)", "prod_id").required(true)),
                        CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Nome no Menu", "prod_label").required(true)),
                        CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Preço (Apenas número)", "prod_price").required(true)),
                        CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "ID do Cargo a ser dado", "prod_role_id").required(true)),
                    ]);
                let _ = component.create_response(&ctx.http, CreateInteractionResponse::Modal(modal)).await;
            } else if component.data.custom_id == "config_vip_edit_block" {
                use serenity::model::application::ComponentInteractionDataKind;
                let opt_id = match &component.data.kind {
                    ComponentInteractionDataKind::StringSelect { values } => values.first().cloned().unwrap_or_default(),
                    _ => return,
                };

                let blocks = VipDb::get_extra_blocks(&pool).await;
                if let Some(block) = blocks.iter().find(|b| b.id == opt_id) {
                    let modal = CreateModal::new(format!("modal_config_vip_edit_block_{}", opt_id), "Editar Bloco")
                        .components(vec![
                            CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Título", "block_title").value(&block.title).required(true)),
                            CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Paragraph, "Descrição", "block_desc").value(&block.desc).required(true)),
                            CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Cor (Hex) - Opcional", "block_color").value(&block.color).required(false)),
                            CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Digite EXCLUIR para apagar", "block_action").required(false)),
                        ]);
                    let _ = component.create_response(&ctx.http, CreateInteractionResponse::Modal(modal)).await;
                }
            } else if component.data.custom_id == "config_vip_edit_prod" {
                use serenity::model::application::ComponentInteractionDataKind;
                let opt_id = match &component.data.kind {
                    ComponentInteractionDataKind::StringSelect { values } => values.first().cloned().unwrap_or_default(),
                    _ => return,
                };

                let prods = VipDb::get_products(&pool).await;
                if let Some(prod) = prods.iter().find(|p| p.id == opt_id) {
                    let modal = CreateModal::new(format!("modal_config_vip_edit_prod_{}", opt_id), "Editar Produto VIP")
                        .components(vec![
                            CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Nome no Menu", "prod_label").value(&prod.label).required(true)),
                            CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Preço (Apenas número)", "prod_price").value(&prod.price).required(true)),
                            CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "ID do Cargo", "prod_role_id").value(&prod.role_id).required(true)),
                            CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Digite EXCLUIR para apagar", "prod_action").required(false)),
                        ]);
                    let _ = component.create_response(&ctx.http, CreateInteractionResponse::Modal(modal)).await;
                }
            }
                    } else if component.data.custom_id.starts_with("config_suporte_") {
                use serenity::model::application::ComponentInteractionDataKind;
                use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage, CreateModal, CreateActionRow, CreateInputText};
                use serenity::model::application::InputTextStyle;
                use crate::database::tickets::TicketDb;

                if !has_staff_role {
                    let _ = component.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content("❌ Apenas administradores podem configurar o suporte.").ephemeral(true)
                    )).await;
                    return;
                }

                let Some(pool) = ({
                    let data = ctx.data.read().await;
                    data.get::<crate::DatabasePool>().cloned()
                }) else { return; };

                if component.data.custom_id == "config_suporte_main" {
                    let main_title = TicketDb::get_config(&pool, "panel_title", "Central de Ajuda").await;
                    let main_desc = TicketDb::get_config(&pool, "panel_description", "").await;
                    let main_img = TicketDb::get_config(&pool, "panel_image", "").await;

                    let modal = CreateModal::new("modal_config_suporte_main", "Textos Principais")
                        .components(vec![
                            CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Título do Painel", "main_title").value(main_title).required(true)),
                            CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Paragraph, "Descrição do Painel", "main_desc").value(main_desc).required(true)),
                            CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Link da Imagem (Opcional)", "main_img").value(main_img).required(false)),
                        ]);
                    let _ = component.create_response(&ctx.http, CreateInteractionResponse::Modal(modal)).await;
                } else if component.data.custom_id == "config_suporte_channels" {
                    let cat_id = TicketDb::get_config(&pool, "ticket_category_id", "").await;
                    let role_id = TicketDb::get_config(&pool, "ticket_staff_role", "").await;
                    let log_id = TicketDb::get_config(&pool, "ticket_logs_channel", "").await;

                    let modal = CreateModal::new("modal_config_suporte_channels", "Canais e Cargos")
                        .components(vec![
                            CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "ID da Categoria", "cat_id").value(cat_id).required(true)),
                            CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "ID do Cargo da Staff", "role_id").value(role_id).required(true)),
                            CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "ID do Canal de Logs", "log_id").value(log_id).required(true)),
                        ]);
                    let _ = component.create_response(&ctx.http, CreateInteractionResponse::Modal(modal)).await;
                } else if component.data.custom_id == "config_suporte_add_opt" {
                    let modal = CreateModal::new("modal_config_suporte_add_opt", "Nova Opção de Ticket")
                        .components(vec![
                            CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "ID da Opção (ex: suporte_vip)", "opt_id").required(true)),
                            CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Nome no Menu", "opt_label").required(true)),
                            CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Descrição no Menu", "opt_desc").required(true)),
                            CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Emoji", "opt_emoji").required(true)),
                            CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Paragraph, "Mensagem do Bot (ao criar o ticket)", "opt_reply").value("Olá, aguarde o atendimento.").required(true)),
                        ]);
                    let _ = component.create_response(&ctx.http, CreateInteractionResponse::Modal(modal)).await;
                } else if component.data.custom_id == "config_suporte_edit_opt" {
                    let opt_id = match &component.data.kind {
                        ComponentInteractionDataKind::StringSelect { values } => values.first().cloned().unwrap_or_default(),
                        _ => return,
                    };
                    let options = TicketDb::get_ticket_options(&pool).await;
                    if let Some(opt) = options.iter().find(|o| o.id == opt_id) {
                        let modal = CreateModal::new(format!("modal_config_suporte_edit_opt_{}", opt_id), "Editar Opção")
                            .components(vec![
                                CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Nome no Menu", "opt_label").value(&opt.label).required(true)),
                                CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Descrição no Menu", "opt_desc").value(&opt.description).required(true)),
                                CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Emoji", "opt_emoji").value(&opt.emoji).required(true)),
                                CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Paragraph, "Mensagem do Bot", "opt_reply").value(&opt.reply).required(true)),
                            ]);
                        let _ = component.create_response(&ctx.http, CreateInteractionResponse::Modal(modal)).await;
                    }
                } else if component.data.custom_id == "config_suporte_delete_opt" {
                    let opt_id = match &component.data.kind {
                        ComponentInteractionDataKind::StringSelect { values } => values.first().cloned().unwrap_or_default(),
                        _ => return,
                    };
                    let mut options = TicketDb::get_ticket_options(&pool).await;
                    options.retain(|o| o.id != opt_id);
                    let new_json = serde_json::to_string(&options).unwrap_or_default();
                    let _ = TicketDb::set_config(&pool, "ticket_options", &new_json).await;

                    let (embed, comps) = crate::commands::tickets::config_suporte::build_config_suporte_panel(&pool).await;
                    let _ = component.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::new().embed(embed).components(comps)
                    )).await;
                }
            } else if component.data.custom_id == "menu_vip" {
            use serenity::model::application::ComponentInteractionDataKind;
            use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage, CreateModal, CreateActionRow, CreateInputText};
            use serenity::model::application::InputTextStyle;

            let selected_vip = match &component.data.kind {
                ComponentInteractionDataKind::StringSelect { values } => values.first().cloned().unwrap_or_default(),
                _ => return,
            };

            if selected_vip == "vip_add_saldo" {
                let _ = component.create_response(&ctx.http, CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content("💳 Função de adicionar saldo em breve!").ephemeral(true)
                )).await;
                return;
            }

            let modal = CreateModal::new(format!("modal_vip_{}", selected_vip), "Vipão")
                .components(vec![
                    CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "CPF", "vip_cpf").placeholder("000.000.000-00").required(true)),
                    CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Email *", "vip_email").placeholder("usuario@gmail.com").required(true)),
                    CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Telefone *", "vip_telefone").placeholder("11 999994124").required(true)),
                    CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Nome completo *", "vip_nome").required(true)),
                    CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Cupom", "vip_cupom").required(false)),
                ]);

            let _ = component.create_response(&ctx.http, CreateInteractionResponse::Modal(modal)).await;
        } else if component.data.custom_id == "blacklist_add_user" {
            use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage, CreateModal, CreateActionRow, CreateInputText};
            use serenity::model::application::InputTextStyle;
            use crate::database::blacklist::BlacklistDb;

            let Some(pool) = ({
                let data = ctx.data.read().await;
                data.get::<crate::DatabasePool>().cloned()
            }) else { return; };

            let target_user_id = if let serenity::model::application::ComponentInteractionDataKind::UserSelect { values } = &component.data.kind {
                values.first().cloned()
            } else {
                None
            };

            if let Some(user_id_str) = target_user_id {
                let msg_id = component.message.id.to_string();
                if let Some(panel) = BlacklistDb::get_panel(&pool, &msg_id).await {
                    let uid = user_id_str;
                    if let Some(guild_id) = component.guild_id {
                        let member = guild_id.member(&ctx.http, uid).await;
                        if let Ok(m) = member {
                            if !m.roles.contains(&serenity::model::id::RoleId::new(panel.role_id.parse::<u64>().unwrap_or(0))) {
                                let _ = component.create_response(&ctx.http, CreateInteractionResponse::Message(
                                    CreateInteractionResponseMessage::new().content(format!("❌ O usuário <@{}> não possui o cargo gerenciado por este painel.", uid)).ephemeral(true)
                                )).await;
                                return;
                            }
                        }
                    }

                    let modal = CreateModal::new(format!("modal_blacklist_add_{}_{}", user_id_str.get(), msg_id), "Tempo de Blacklist")
                        .components(vec![
                            CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Duração (Ex: 10m, 2h, 1d) *", "blacklist_time").required(true)),
                        ]);

                    let _ = component.create_response(&ctx.http, CreateInteractionResponse::Modal(modal)).await;
                } else {
                    let _ = component.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content("❌ Painel não encontrado no banco.").ephemeral(true)
                    )).await;
                }
            }
        } else if component.data.custom_id == "blacklist_remove_user" {
            use serenity::builder::{CreateInteractionResponse, CreateModal, CreateActionRow, CreateInputText};
            use serenity::model::application::InputTextStyle;

            let msg_id = component.message.id.to_string();
            let modal = CreateModal::new(format!("modal_blacklist_remove_{}", msg_id), "Remover da Blacklist")
                .components(vec![
                    CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "ID do Usuário", "blacklist_userid").placeholder("Ex: 123456789012345678").required(true)),
                ]);

            let _ = component.create_response(&ctx.http, CreateInteractionResponse::Modal(modal)).await;
        }
    } else if let Interaction::Modal(modal) = interaction {

        info!("Recebeu modal submetido: {}", modal.data.custom_id);

        let Some(pool) = ({
            let data = ctx.data.read().await;
            data.get::<crate::DatabasePool>().cloned()
        }) else { return; };

        if modal.data.custom_id.starts_with("modal_config_suporte_") {
            use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage, EditInteractionResponse};
            use crate::database::tickets::{TicketDb, TicketOption};

            let _ = modal.create_response(&ctx.http, CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true)
            )).await;

            let pool = {
                let data = ctx.data.read().await;
                data.get::<crate::DatabasePool>().unwrap().clone()
            };

            if modal.data.custom_id == "modal_config_suporte_main" {
                let main_title = match &modal.data.components[0].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let main_desc = match &modal.data.components[1].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let main_img = match &modal.data.components[2].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };

                let _ = TicketDb::set_config(&pool, "panel_title", &main_title).await;
                let _ = TicketDb::set_config(&pool, "panel_description", &main_desc).await;
                let _ = TicketDb::set_config(&pool, "panel_image", &main_img).await;

                let (embed, comps) = crate::commands::tickets::config_suporte::build_config_suporte_panel(&pool).await;
                if let Some(mut msg) = modal.message.clone() {
                    let _ = msg.edit(&ctx.http, serenity::builder::EditMessage::new().embed(embed).components(comps)).await;
                }
                let _ = modal.edit_response(&ctx.http, EditInteractionResponse::new().content("✅ Textos principais atualizados com sucesso!")).await;

            } else if modal.data.custom_id == "modal_config_suporte_channels" {
                let cat_id = match &modal.data.components[0].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let role_id = match &modal.data.components[1].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let log_id = match &modal.data.components[2].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };

                let _ = TicketDb::set_config(&pool, "ticket_category_id", &cat_id).await;
                let _ = TicketDb::set_config(&pool, "ticket_staff_role", &role_id).await;
                let _ = TicketDb::set_config(&pool, "ticket_logs_channel", &log_id).await;

                let _ = modal.edit_response(&ctx.http, EditInteractionResponse::new().content("✅ Canais e cargos configurados!")).await;

            } else if modal.data.custom_id == "modal_config_suporte_add_opt" {
                let opt_id = match &modal.data.components[0].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() }.to_lowercase().replace(|c: char| !c.is_ascii_alphanumeric(), "_");
                let opt_label = match &modal.data.components[1].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let opt_desc = match &modal.data.components[2].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let opt_emoji = match &modal.data.components[3].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let opt_reply = match &modal.data.components[4].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };

                let mut options = TicketDb::get_ticket_options(&pool).await;
                options.retain(|o| o.id != opt_id);
                options.push(TicketOption { id: opt_id, label: opt_label, description: opt_desc, emoji: opt_emoji, reply: opt_reply });
                let new_json = serde_json::to_string(&options).unwrap_or_default();
                let _ = TicketDb::set_config(&pool, "ticket_options", &new_json).await;

                let (embed, comps) = crate::commands::tickets::config_suporte::build_config_suporte_panel(&pool).await;
                if let Some(mut msg) = modal.message.clone() {
                    let _ = msg.edit(&ctx.http, serenity::builder::EditMessage::new().embed(embed).components(comps)).await;
                }
                let _ = modal.edit_response(&ctx.http, EditInteractionResponse::new().content("✅ Opção de suporte adicionada com sucesso!")).await;

            } else if modal.data.custom_id.starts_with("modal_config_suporte_edit_opt_") {
                let Some(opt_id) = modal.data.custom_id.strip_prefix("modal_config_suporte_edit_opt_").map(|s| s.to_string()) else { return; };
                let opt_label = match &modal.data.components[0].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let opt_desc = match &modal.data.components[1].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let opt_emoji = match &modal.data.components[2].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let opt_reply = match &modal.data.components[3].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };

                let mut options = TicketDb::get_ticket_options(&pool).await;
                if let Some(opt) = options.iter_mut().find(|o| o.id == opt_id) {
                    opt.label = opt_label;
                    opt.description = opt_desc;
                    opt.emoji = opt_emoji;
                    opt.reply = opt_reply;
                }
                let new_json = serde_json::to_string(&options).unwrap_or_default();
                let _ = TicketDb::set_config(&pool, "ticket_options", &new_json).await;

                let (embed, comps) = crate::commands::tickets::config_suporte::build_config_suporte_panel(&pool).await;
                if let Some(mut msg) = modal.message.clone() {
                    let _ = msg.edit(&ctx.http, serenity::builder::EditMessage::new().embed(embed).components(comps)).await;
                }
                let _ = modal.edit_response(&ctx.http, EditInteractionResponse::new().content("✅ Opção de suporte atualizada com sucesso!")).await;
            }
        } else if modal.data.custom_id.starts_with("modal_config_vip_") {
            use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage, EditInteractionResponse};
            use crate::database::vip::VipDb;

            let _ = modal.create_response(&ctx.http, CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true)
            )).await;

            if modal.data.custom_id == "modal_config_vip_main" {
                let main_desc = match &modal.data.components[0].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let main_img = match &modal.data.components[1].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };

                VipDb::set_main_text(&pool, &main_desc, &main_img).await;
                crate::commands::vip::painelvip::update_panel(&ctx).await;

                let _ = modal.edit_response(&ctx.http, EditInteractionResponse::new().content("✅ Textos do Painel VIP atualizados com sucesso!")).await;
            } else if modal.data.custom_id == "modal_config_vip_add_block" {
                let block_id = match &modal.data.components[0].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() }.to_lowercase().replace(|c: char| !c.is_ascii_alphanumeric(), "_");
                let title = match &modal.data.components[1].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let desc = match &modal.data.components[2].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let color = match &modal.data.components[3].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_else(|| "#2F3136".to_string()), _ => "#2F3136".to_string() };

                VipDb::save_extra_block(&pool, block_id, title, desc, color).await;
                crate::commands::vip::painelvip::update_panel(&ctx).await;

                let _ = modal.edit_response(&ctx.http, EditInteractionResponse::new().content("✅ Bloco Extra adicionado com sucesso!")).await;
            } else if modal.data.custom_id == "modal_config_vip_add_prod" {
                let prod_id = match &modal.data.components[0].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() }.to_lowercase().replace(|c: char| !c.is_ascii_alphanumeric(), "_");
                let label = match &modal.data.components[1].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let price = match &modal.data.components[2].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let role_id = match &modal.data.components[3].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };

                VipDb::save_product(&pool, prod_id, label, price, role_id).await;
                crate::commands::vip::painelvip::update_panel(&ctx).await;

                let _ = modal.edit_response(&ctx.http, EditInteractionResponse::new().content("✅ Produto VIP adicionado com sucesso!")).await;
            } else if modal.data.custom_id.starts_with("modal_config_vip_edit_block_") {
                let Some(opt_id) = modal.data.custom_id.strip_prefix("modal_config_vip_edit_block_") else { return; };
                let title = match &modal.data.components[0].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let desc = match &modal.data.components[1].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let color = match &modal.data.components[2].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let action = match &modal.data.components[3].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };

                if action.eq_ignore_ascii_case("EXCLUIR") {
                    VipDb::delete_extra_block(&pool, opt_id).await;
                    crate::commands::vip::painelvip::update_panel(&ctx).await;
                    let _ = modal.edit_response(&ctx.http, EditInteractionResponse::new().content("✅ Bloco Extra excluído com sucesso!")).await;
                } else {
                    VipDb::save_extra_block(&pool, opt_id.to_string(), title, desc, color).await;
                    crate::commands::vip::painelvip::update_panel(&ctx).await;
                    let _ = modal.edit_response(&ctx.http, EditInteractionResponse::new().content("✅ Bloco Extra atualizado com sucesso!")).await;
                }
            } else if modal.data.custom_id.starts_with("modal_config_vip_edit_prod_") {
                let Some(opt_id) = modal.data.custom_id.strip_prefix("modal_config_vip_edit_prod_") else { return; };
                let label = match &modal.data.components[0].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let price = match &modal.data.components[1].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let role_id = match &modal.data.components[2].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
                let action = match &modal.data.components[3].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };

                if action.eq_ignore_ascii_case("EXCLUIR") {
                    VipDb::delete_product(&pool, opt_id).await;
                    crate::commands::vip::painelvip::update_panel(&ctx).await;
                    let _ = modal.edit_response(&ctx.http, EditInteractionResponse::new().content("✅ Produto VIP excluído com sucesso!")).await;
                } else {
                    VipDb::save_product(&pool, opt_id.to_string(), label, price, role_id).await;
                    crate::commands::vip::painelvip::update_panel(&ctx).await;
                    let _ = modal.edit_response(&ctx.http, EditInteractionResponse::new().content("✅ Produto VIP atualizado com sucesso!")).await;
                }
            }
        } else if modal.data.custom_id.starts_with("modal_vip_") {
            use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage, EditInteractionResponse, CreateEmbed, CreateMessage, CreateAttachment};
            use crate::database::vip::VipDb;
            use crate::database::payments::PaymentDb;
            use crate::cron::mercado_pago::MercadoPagoClient;
            use base64::{Engine as _, engine::general_purpose};

            let Some(pacote) = modal.data.custom_id.strip_prefix("modal_vip_") else { return; };
            let cpf = match &modal.data.components[0].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
            let email = match &modal.data.components[1].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
            let _telefone = match &modal.data.components[2].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
            let nome = match &modal.data.components[3].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };
            let _cupom = match modal.data.components.get(4).and_then(|r| r.components.get(0)) { Some(serenity::model::application::ActionRowComponent::InputText(i)) => i.value.clone().unwrap_or_default(), _ => String::new() };

            let _ = modal.create_response(&ctx.http, CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true)
            )).await;

            let prods = VipDb::get_products(&pool).await;
            let product = prods.iter().find(|p| p.id == pacote);

            if let Some(prod) = product {
                if let Ok(valor) = prod.price.parse::<f64>() {
                    let mp_client = MercadoPagoClient::new();

                    match mp_client.create_pix_payment(valor, email, nome, cpf, format!("Pacote VIP {}", pacote)).await {
                        Ok(payment_info) => {
                            let _ = PaymentDb::add_payment(&pool, &payment_info.id.to_string(), &modal.user.id.to_string(), pacote).await;

                            if let Some(poi) = payment_info.point_of_interaction {
                                let mut embed = CreateEmbed::new()
                                    .color(0x9b59b6)
                                    .title("🪙 Fatura PIX Gerada")
                                    .description(format!("Sua fatura para o pacote **{}** foi gerada no valor de **R$ {:.2}**.\n\nAbra o app do seu banco e escaneie o QR Code abaixo ou copie e cole o código PIX para efetuar o pagamento.\n\nSeu cargo será ativado automaticamente após a confirmação. (Pode levar até 1 minuto)", pacote.replace("vip_", "").to_uppercase(), valor))
                                    .field("PIX Copia e Cola", format!("```\n{}\n```", poi.transaction_data.qr_code), false);

                                let mut attachments = vec![];
                                if let Ok(image_bytes) = general_purpose::STANDARD.decode(&poi.transaction_data.qr_code_base64) {
                                    attachments.push(CreateAttachment::bytes(image_bytes, "qrcode.png"));
                                    embed = embed.image("attachment://qrcode.png");
                                }

                                let msg_create = CreateMessage::new().embed(embed);
                                let msg_create = if !attachments.is_empty() {
                                    msg_create.files(attachments)
                                } else {
                                    msg_create
                                };

                                if let Ok(dm_channel) = modal.user.create_dm_channel(&ctx.http).await {
                                    if let Err(e) = dm_channel.send_message(&ctx.http, msg_create).await {
                                        error!("Erro ao enviar DM PIX: {}", e);
                                        let _ = modal.edit_response(&ctx.http, EditInteractionResponse::new().content("❌ Sua fatura foi gerada, mas **suas DMs estão fechadas!** Libere o envio de mensagens diretas e tente novamente.")).await;
                                        return;
                                    } else {
                                        let _ = modal.edit_response(&ctx.http, EditInteractionResponse::new().content("✅ Sua fatura foi gerada e enviada para as suas **Mensagens Privadas** (DM). Verifique sua caixa de entrada!")).await;
                                        return;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Erro ao criar PIX: {:?}", e);
                            let _ = modal.edit_response(&ctx.http, EditInteractionResponse::new().content("❌ Houve um erro ao gerar a cobrança PIX no Mercado Pago. Verifique os dados ou tente novamente.")).await;
                            return;
                        }
                    }
                }
            }

            let _ = modal.edit_response(&ctx.http, EditInteractionResponse::new().content("❌ Pacote VIP inválido ou sem preço configurado.")).await;
        } else if modal.data.custom_id.starts_with("modal_blacklist_add_") {
            use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage};
            use crate::database::blacklist::BlacklistDb;

            let parts: Vec<&str> = modal.data.custom_id.split('_').collect();
            if parts.len() >= 5 {
                let msg_id = parts[parts.len() - 1].to_string();
                let target_user_id = parts[parts.len() - 2].to_string();
                let time_str = match &modal.data.components[0].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };

                let multiplier = if time_str.ends_with('m') {
                    1000 * 60
                } else if time_str.ends_with('h') {
                    1000 * 60 * 60
                } else if time_str.ends_with('d') {
                    1000 * 60 * 60 * 24
                } else {
                    0
                };

                let num_str: String = time_str.chars().filter(|c| c.is_ascii_digit()).collect();
                if let Ok(num) = num_str.parse::<i64>() {
                    if multiplier > 0 {
                        let time_ms = num * multiplier;
                        let expires_at = chrono::Utc::now().timestamp_millis() + time_ms;

                        let Some(guild_id_obj) = modal.guild_id else { return; };
                        let guild_id = guild_id_obj.get().to_string();
                        let channel_id = modal.channel_id.get().to_string();

                        let _ = modal.create_response(&ctx.http, CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new().ephemeral(true))).await;

                        if let Some(panel) = BlacklistDb::get_panel(&pool, &msg_id).await {
                            if let Ok(uid) = target_user_id.parse::<u64>() {
                                if let Ok(member) = guild_id_obj.member(&ctx.http, uid).await {
                                    let role_id = serenity::model::id::RoleId::new(panel.role_id.parse::<u64>().unwrap_or(0));
                                    let _ = member.remove_role(&ctx.http, role_id).await;
                                }
                            }

                            let _ = BlacklistDb::add_user(&pool, &guild_id, &target_user_id, &panel.role_id, expires_at, &msg_id, &channel_id).await;
                            crate::commands::mod_cmds::blacklist::update_panel(&ctx, &pool, &msg_id).await;

                            {
                                let data = ctx.data.read().await;
                                if let Some(notify) = data.get::<crate::BlacklistNotify>() {
                                    notify.notify_one();
                                }
                            }

                            let _ = modal.edit_response(&ctx.http, serenity::builder::EditInteractionResponse::new().content(format!("✅ O usuário <@{}> foi colocado na blacklist por **{}**.", target_user_id, time_str))).await;
                        }
                    } else {
                        let _ = modal.create_response(&ctx.http, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content("❌ Formato de tempo inválido. Use 10m, 2h, 1d.").ephemeral(true))).await;
                    }
                } else {
                    let _ = modal.create_response(&ctx.http, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content("❌ Formato de tempo inválido. Use 10m, 2h, 1d.").ephemeral(true))).await;
                }
            }
        } else if modal.data.custom_id.starts_with("modal_blacklist_remove_") {
            use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage};
            use crate::database::blacklist::BlacklistDb;

            let msg_id = modal.data.custom_id.replace("modal_blacklist_remove_", "");
            let target_user_id = match &modal.data.components[0].components[0] { serenity::model::application::ActionRowComponent::InputText(i) => i.value.clone().unwrap_or_default(), _ => String::new() };

            let Some(guild_id_obj) = modal.guild_id else { return; };
            let guild_id = guild_id_obj.get().to_string();

            let _ = modal.create_response(&ctx.http, CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new().ephemeral(true))).await;

            if let Some(panel) = BlacklistDb::get_panel(&pool, &msg_id).await {
                let users = BlacklistDb::get_users_for_panel(&pool, &msg_id).await;
                if users.iter().any(|u| u.user_id == target_user_id) {
                    if let Ok(uid) = target_user_id.parse::<u64>() {
                        if let Ok(member) = guild_id_obj.member(&ctx.http, uid).await {
                            let role_id = serenity::model::id::RoleId::new(panel.role_id.parse::<u64>().unwrap_or(0));
                            let _ = member.add_role(&ctx.http, role_id).await;
                        }
                    }

                    let _ = BlacklistDb::remove_user(&pool, &guild_id, &target_user_id).await;
                    crate::commands::mod_cmds::blacklist::update_panel(&ctx, &pool, &msg_id).await;

                    let _ = modal.edit_response(&ctx.http, serenity::builder::EditInteractionResponse::new().content(format!("✅ O usuário <@{}> foi removido da blacklist com sucesso!", target_user_id))).await;
                } else {
                    let _ = modal.edit_response(&ctx.http, serenity::builder::EditInteractionResponse::new().content("❌ Esse usuário não está na blacklist deste painel.")).await;
                }
            }
        }
    }
}
