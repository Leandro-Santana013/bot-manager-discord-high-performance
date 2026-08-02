use serenity::model::prelude::*;
use serenity::prelude::*;
use tracing::{info, error};
use crate::database::voice::VoiceDb;
use dashmap::DashMap;
use std::sync::Arc;

pub struct VoiceJoin {
    pub joined_at: i64,
    pub last_mute_at: Option<i64>,
    pub total_muted: i64,
}

pub struct VoiceTracker;

impl TypeMapKey for VoiceTracker {
    type Value = Arc<DashMap<String, VoiceJoin>>;
}

pub async fn handle(ctx: Context, old: Option<VoiceState>, new: VoiceState) {
    let data = ctx.data.read().await;
    let tracker = data.get::<VoiceTracker>().expect("VoiceTracker ausente!").clone();
    let pool = data.get::<crate::DatabasePool>().expect("DatabasePool ausente!").clone();
    drop(data);

    let user_id = new.user_id.to_string();

    if new.channel_id.is_some() && new.member.map_or(false, |m| !m.user.bot) {
        let is_muted = new.self_mute || new.mute || new.self_deaf || new.deaf;

        if old.as_ref().map_or(true, |o| o.channel_id.is_none()) {
            tracker.insert(user_id.clone(), VoiceJoin {
                joined_at: chrono::Utc::now().timestamp_millis(),
                last_mute_at: if is_muted { Some(chrono::Utc::now().timestamp_millis()) } else { None },
                total_muted: 0,
            });
            info!("Usuário {} começou a rastrear tempo de call", user_id);
        } else {

            if let Some(mut join) = tracker.get_mut(&user_id) {
                if is_muted && join.last_mute_at.is_none() {
                    join.last_mute_at = Some(chrono::Utc::now().timestamp_millis());
                } else if !is_muted {
                    if let Some(last_mute) = join.last_mute_at {
                        join.total_muted += chrono::Utc::now().timestamp_millis() - last_mute;
                        join.last_mute_at = None;
                    }
                }
            }
        }
    } else if new.channel_id.is_none() {

        if let Some((_, mut join)) = tracker.remove(&user_id) {

            if let Some(last_mute) = join.last_mute_at {
                join.total_muted += chrono::Utc::now().timestamp_millis() - last_mute;
            }

            let total_time_ms = chrono::Utc::now().timestamp_millis() - join.joined_at;
            let valid_time_ms = total_time_ms - join.total_muted;

            info!("Usuário {} saiu. Tempo total: {} ms, Mutado: {} ms, Válido: {} ms", user_id, total_time_ms, join.total_muted, valid_time_ms);

            if let Err(e) = VoiceDb::update_user_time(&pool, &user_id, valid_time_ms, join.total_muted).await {
                error!("Falha ao salvar tempo no DB: {}", e);
            }
        }
    }
}
