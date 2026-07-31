use serenity::builder::{CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, CreateEmbed, CreateActionRow, CreateButton, EditInteractionResponse};
use serenity::futures::StreamExt;
use std::time::Duration;
use serenity::collector::ComponentInteractionCollector;
use serenity::model::prelude::*;
use serenity::prelude::*;
use crate::database::tickets::TicketDb;

pub fn register() -> CreateCommand {
    CreateCommand::new("ranking")
        .description("Mostra o ranking de atendimentos (Staff).")
        .default_member_permissions(Permissions::MANAGE_MESSAGES)
}

pub async fn run(ctx: &Context, interaction: &serenity::model::application::CommandInteraction) {
    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Defer(
        CreateInteractionResponseMessage::new()
    )).await;

    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>().unwrap().clone()
    };
    
    let db_users_res = TicketDb::get_top_tickets(&pool, 1000).await;
    let ranking = match db_users_res {
        Ok(rows) => rows,
        Err(_) => vec![],
    };

    if ranking.is_empty() {
        let _ = interaction.edit_response(&ctx.http, EditInteractionResponse::new()
            .content("Nenhum ticket foi computado ainda.")
        ).await;
        return;
    }

    let total_tickets: i32 = ranking.iter().map(|x| x.1).sum();

    let max_pages = (ranking.len() as f64 / 10.0).ceil() as usize;
    let mut current_page = 0;

    let generate_embed = |page: usize| -> CreateEmbed {
        let start = page * 10;
        let end = (start + 10).min(ranking.len());
        let current_users = &ranking[start..end];

        let mut desc = format!("🎟️ Total de Tickets: `{}`\n\n", total_tickets);

        for (i, (user_id, quantidade)) in current_users.iter().enumerate() {
            let global_index = start + i;
            
            let prefix = match global_index {
                0 => "🥇",
                1 => "🥈",
                2 => "🥉",
                _ => &format!("{}.", global_index + 1),
            };

            desc.push_str(&format!("{} <@{}> — **{}** atendimentos\n", prefix, user_id, quantidade));
        }

        CreateEmbed::new()
            .title("🏆 Ranking de Atendimento (Staff)")
            .description(desc)
            .color(0x3498db)
    };

    let generate_buttons = |page: usize| -> CreateActionRow {
        let btn_prev = CreateButton::new("rank_prev")
            .emoji('⬅')
            .style(serenity::model::application::ButtonStyle::Danger)
            .disabled(page == 0);
            
        let btn_ind = CreateButton::new("rank_ind")
            .label(format!("{} / {}", page + 1, max_pages))
            .style(serenity::model::application::ButtonStyle::Secondary)
            .disabled(true);

        let btn_next = CreateButton::new("rank_next")
            .emoji('➡')
            .style(serenity::model::application::ButtonStyle::Danger)
            .disabled(page >= max_pages.saturating_sub(1));

        CreateActionRow::Buttons(vec![btn_prev, btn_ind, btn_next])
    };

    let embed = generate_embed(current_page);
    let mut response = EditInteractionResponse::new().embed(embed);
    if max_pages > 1 {
        response = response.components(vec![generate_buttons(current_page)]);
    }

    let mut msg = interaction.edit_response(&ctx.http, response).await.unwrap();

    if max_pages > 1 {
        let mut collector = ComponentInteractionCollector::new(&ctx.shard)
            .message_id(msg.id)
            .timeout(Duration::from_secs(60))
            .stream();

        while let Some(mci) = collector.next().await {
            if mci.user.id != interaction.user.id {
                let _ = mci.create_response(&ctx.http, CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content("Você não pode usar estes botões.").ephemeral(true)
                )).await;
                continue;
            }

            match mci.data.custom_id.as_str() {
                "rank_prev" => if current_page > 0 { current_page -= 1; },
                "rank_next" => if current_page < max_pages - 1 { current_page += 1; },
                _ => {}
            }

            let _ = mci.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(generate_embed(current_page))
                    .components(vec![generate_buttons(current_page)])
            )).await;
        }

        let _ = msg.edit(&ctx.http, serenity::builder::EditMessage::new().components(vec![])).await;
    }
}
