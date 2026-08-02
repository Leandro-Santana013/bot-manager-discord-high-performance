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
    let (tracker, pool) = {
        let data = ctx.data.read().await;
        let Some(tracker) = data.get::<VoiceTracker>().cloned() else {
            error!("VoiceTracker ausente no TypeMap!");
            return;
        };
        let Some(pool) = data.get::<crate::DatabasePool>().cloned() else {
            error!("DatabasePool ausente no TypeMap!");
            return;
        };
        (tracker, pool)
    };

    let user_id = new.user_id.to_string();

    if new.channel_id.is_some() && new.member.is_some_and(|m| !m.user.bot) {
        let is_muted = new.self_mute || new.mute || new.self_deaf || new.deaf;
        let now = chrono::Utc::now().timestamp_millis();

        if old.as_ref().is_none_or(|o| o.channel_id.is_none()) {
            tracker.insert(user_id.clone(), VoiceJoin {
                joined_at: now,
                last_mute_at: if is_muted { Some(now) } else { None },
                total_muted: 0,
            });
            info!("Usuário {} começou a rastrear tempo de call", user_id);
        } else if let Some(mut join) = tracker.get_mut(&user_id) {
            if is_muted && join.last_mute_at.is_none() {
                join.last_mute_at = Some(now);
            } else if !is_muted {
                if let Some(last_mute) = join.last_mute_at {
                    join.total_muted += now.saturating_sub(last_mute);
                    join.last_mute_at = None;
                }
            }
        }
    } else if new.channel_id.is_none() {
        if let Some((_, mut join)) = tracker.remove(&user_id) {
            let now = chrono::Utc::now().timestamp_millis();
            if let Some(last_mute) = join.last_mute_at {
                join.total_muted += now.saturating_sub(last_mute);
            }

            let total_time_ms = now.saturating_sub(join.joined_at);
            let valid_time_ms = total_time_ms.saturating_sub(join.total_muted);

            info!("Usuário {} saiu. Tempo total: {} ms, Mutado: {} ms, Válido: {} ms", user_id, total_time_ms, join.total_muted, valid_time_ms);

            if let Err(e) = VoiceDb::update_user_time(&pool, &user_id, valid_time_ms, join.total_muted).await {
                error!("Falha ao salvar tempo no DB: {}", e);
            }
        }
    }
}

pub async fn start_voice_flush_cron(ctx: Arc<Context>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(300)); // Cada 5 minutos
    loop {
        interval.tick().await;

        let (tracker, pool) = {
            let data = ctx.data.read().await;
            let Some(tracker) = data.get::<VoiceTracker>().cloned() else { continue; };
            let Some(pool) = data.get::<crate::DatabasePool>().cloned() else { continue; };
            (tracker, pool)
        };

        let now = chrono::Utc::now().timestamp_millis();
        let mut updates = Vec::new();

        for mut item in tracker.iter_mut() {
            let user_id = item.key().clone();
            let join = item.value_mut();

            let mut muted_delta = 0;
            if let Some(last_mute) = join.last_mute_at {
                muted_delta = now.saturating_sub(last_mute);
                join.last_mute_at = Some(now); // reseta para o marco atual
            }

            let total_muted_period = join.total_muted + muted_delta;
            let total_time_period = now.saturating_sub(join.joined_at);
            let valid_time_period = total_time_period.saturating_sub(total_muted_period);

            // Reseta marco de início para o próximo período incremental
            join.joined_at = now;
            join.total_muted = 0;

            if valid_time_period > 0 || total_muted_period > 0 {
                updates.push((user_id, valid_time_period, total_muted_period));
            }
        }

        for (uid, valid_ms, muted_ms) in updates {
            if let Err(e) = VoiceDb::update_user_time(&pool, &uid, valid_ms, muted_ms).await {
                error!("Erro no flush incremental de voz para {}: {}", uid, e);
            }
        }
    }
}

