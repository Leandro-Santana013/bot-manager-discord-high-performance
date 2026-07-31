use serenity::builder::{CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, CreateEmbed, CreateActionRow, CreateButton, EditInteractionResponse};
use serenity::futures::StreamExt;
use std::time::Duration;
use serenity::collector::ComponentInteractionCollector;
use serenity::prelude::*;

pub fn register() -> CreateCommand {
    CreateCommand::new("top")
        .description("Mostra o top usuários com mais tempo em canais de voz.")
}

fn ms_to_time(ms: i64) -> String {
    if ms <= 0 { return "0m".to_string(); }
    let minutes = (ms / (1000 * 60)) % 60;
    let hours = ms / (1000 * 60 * 60);
    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

pub async fn run(ctx: &Context, interaction: &serenity::model::application::CommandInteraction) {
    let _ = interaction.create_response(&ctx.http, CreateInteractionResponse::Defer(
        CreateInteractionResponseMessage::new().ephemeral(true)
    )).await;

    // Fetch static DB
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>().unwrap().clone()
    };
    
    let db_users = crate::database::voice::VoiceDb::get_all_users_time(&pool).await;
    let mut user_map: std::collections::HashMap<String, i64> = db_users.into_iter().collect();

    // Sum active session time
    {
        let data = ctx.data.read().await;
        if let Some(tracker) = data.get::<crate::events::voice::VoiceTracker>() {
            let now = chrono::Utc::now().timestamp_millis();
            for entry in tracker.iter() {
                let user_id = entry.key().clone();
                let join = entry.value();
                let mut active_ms = now - join.joined_at;
                let mut active_muted = join.total_muted;
                if let Some(last_mute) = join.last_mute_at {
                    active_muted += now - last_mute;
                }
                active_ms -= active_muted;
                if active_ms < 0 { active_ms = 0; }

                let existing = user_map.get(&user_id).copied().unwrap_or(0);
                user_map.insert(user_id, existing + active_ms);
            }
        }
    }

    let mut ranking: Vec<(String, i64)> = user_map.into_iter().collect();
    ranking.sort_by(|a, b| b.1.cmp(&a.1));

    if ranking.is_empty() {
        let _ = interaction.edit_response(&ctx.http, EditInteractionResponse::new()
            .content("Nenhum tempo registrado ainda.")
        ).await;
        return;
    }

    let total_ms: i64 = ranking.iter().map(|x| x.1).sum();
    let total_time_str = ms_to_time(total_ms);

    let max_pages = (ranking.len() as f64 / 10.0).ceil() as usize;
    let mut current_page = 0;

    let mut guild_icon_url = None;
    if let Some(guild_id) = interaction.guild_id {
        if let Ok(guild) = guild_id.to_partial_guild(&ctx.http).await {
            guild_icon_url = guild.icon_url();
        }
    }

    let generate_embed = |page: usize| -> CreateEmbed {
        let start = page * 10;
        let end = (start + 10).min(ranking.len());
        let current_users = &ranking[start..end];

        let mut desc = format!("🕒 Total de Tempo: `{}`\n\n", total_time_str);

        for (i, (user_id, ms)) in current_users.iter().enumerate() {
            let global_index = start + i;
            let time_str = ms_to_time(*ms);
            
            let prefix = match global_index {
                0 => "🥇",
                1 => "🥈",
                2 => "🥉",
                _ => &format!("{}.", global_index + 1),
            };

            desc.push_str(&format!("{} <@{}>: `{}`\n", prefix, user_id, time_str));
        }

        let mut embed = CreateEmbed::new()
            .title("🏆 Ranking de Tempo de Call")
            .description(desc)
            .color(0x3498db);

        if let Some(ref url) = guild_icon_url {
            embed = embed.thumbnail(url);
        }
        embed
    };

    let generate_buttons = |page: usize| -> CreateActionRow {
        let btn_prev = CreateButton::new("top_prev")
            .emoji('⬅')
            .style(serenity::model::application::ButtonStyle::Danger)
            .disabled(page == 0);
            
        let btn_ind = CreateButton::new("top_ind")
            .label(format!("{} / {}", page + 1, max_pages))
            .style(serenity::model::application::ButtonStyle::Secondary)
            .disabled(true);

        let btn_next = CreateButton::new("top_next")
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
                "top_prev" => if current_page > 0 { current_page -= 1; },
                "top_next" => if current_page < max_pages - 1 { current_page += 1; },
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

pub async fn run_message(ctx: &Context, msg: &serenity::model::channel::Message) {
    let pool = {
        let data = ctx.data.read().await;
        data.get::<crate::DatabasePool>().unwrap().clone()
    };
    
    let db_users = crate::database::voice::VoiceDb::get_all_users_time(&pool).await;
    let mut user_map: std::collections::HashMap<String, i64> = db_users.into_iter().collect();

    {
        let data = ctx.data.read().await;
        if let Some(tracker) = data.get::<crate::events::voice::VoiceTracker>() {
            let now = chrono::Utc::now().timestamp_millis();
            for entry in tracker.iter() {
                let user_id = entry.key().clone();
                let join = entry.value();
                let mut active_ms = now - join.joined_at;
                let mut active_muted = join.total_muted;
                if let Some(last_mute) = join.last_mute_at {
                    active_muted += now - last_mute;
                }
                active_ms -= active_muted;
                if active_ms < 0 { active_ms = 0; }

                let existing = user_map.get(&user_id).copied().unwrap_or(0);
                user_map.insert(user_id, existing + active_ms);
            }
        }
    }

    let mut ranking: Vec<(String, i64)> = user_map.into_iter().collect();
    ranking.sort_by(|a, b| b.1.cmp(&a.1));

    if ranking.is_empty() {
        let _ = msg.channel_id.send_message(&ctx.http, serenity::builder::CreateMessage::new().content("Nenhum tempo registrado ainda.")).await;
        return;
    }

    let total_ms: i64 = ranking.iter().map(|x| x.1).sum();
    let total_time_str = ms_to_time(total_ms);

    let max_pages = (ranking.len() as f64 / 10.0).ceil() as usize;
    let mut current_page = 0;

    let mut guild_icon_url = None;
    if let Some(guild_id) = msg.guild_id {
        if let Ok(guild) = guild_id.to_partial_guild(&ctx.http).await {
            guild_icon_url = guild.icon_url();
        }
    }

    let generate_embed = |page: usize| -> CreateEmbed {
        let start = page * 10;
        let end = (start + 10).min(ranking.len());
        let current_users = &ranking[start..end];

        let mut desc = format!("🕒 Total de Tempo: `{}`\n\n", total_time_str);

        for (i, (user_id, ms)) in current_users.iter().enumerate() {
            let global_index = start + i;
            let time_str = ms_to_time(*ms);
            
            let prefix = match global_index {
                0 => "🥇",
                1 => "🥈",
                2 => "🥉",
                _ => &format!("{}.", global_index + 1),
            };

            desc.push_str(&format!("{} <@{}>: `{}`\n", prefix, user_id, time_str));
        }

        let mut embed = CreateEmbed::new()
            .title("🏆 Ranking de Tempo de Call")
            .description(desc)
            .color(0x3498db);

        if let Some(ref url) = guild_icon_url {
            embed = embed.thumbnail(url);
        }
        embed
    };

    let generate_buttons = |page: usize| -> CreateActionRow {
        let btn_prev = CreateButton::new("top_prev")
            .emoji('⬅')
            .style(serenity::model::application::ButtonStyle::Danger)
            .disabled(page == 0);
            
        let btn_ind = CreateButton::new("top_ind")
            .label(format!("{} / {}", page + 1, max_pages))
            .style(serenity::model::application::ButtonStyle::Secondary)
            .disabled(true);

        let btn_next = CreateButton::new("top_next")
            .emoji('➡')
            .style(serenity::model::application::ButtonStyle::Danger)
            .disabled(page >= max_pages.saturating_sub(1));

        CreateActionRow::Buttons(vec![btn_prev, btn_ind, btn_next])
    };

    let embed = generate_embed(current_page);
    let mut response = serenity::builder::CreateMessage::new().embed(embed);
    if max_pages > 1 {
        response = response.components(vec![generate_buttons(current_page)]);
    }

    if let Ok(reply) = msg.channel_id.send_message(&ctx.http, response).await {
        let http_clone = ctx.http.clone();
        
        let msg_id = reply.id;
        let _shard = ctx.shard.clone();
        
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(20)).await;
            let _ = reply.delete(&http_clone).await;
        });

        if max_pages > 1 {
            let mut collector = ComponentInteractionCollector::new(&ctx.shard)
                .message_id(msg_id)
                .timeout(Duration::from_secs(20))
                .stream();

            while let Some(mci) = collector.next().await {
                if mci.user.id != msg.author.id {
                    let _ = mci.create_response(&ctx.http, CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new().content("Você não pode usar estes botões.").ephemeral(true)
                    )).await;
                    continue;
                }

                match mci.data.custom_id.as_str() {
                    "top_prev" => if current_page > 0 { current_page -= 1; },
                    "top_next" => if current_page < max_pages - 1 { current_page += 1; },
                    _ => {}
                }

                let _ = mci.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(generate_embed(current_page))
                        .components(vec![generate_buttons(current_page)])
                )).await;
            }
        }
    }
}
