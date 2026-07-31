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
    drop(data); // Drop o TypeMap lock imediatamente! Max performance.

    let user_id = new.user_id.to_string();

    // Se o usuário entrou em um canal e não é bot
    if new.channel_id.is_some() && new.member.map_or(false, |m| !m.user.bot) {
        let is_muted = new.self_mute || new.mute || new.self_deaf || new.deaf;

        // Entrou em um canal novo e não estava sendo rastreado
        if old.as_ref().map_or(true, |o| o.channel_id.is_none()) {
            tracker.insert(user_id.clone(), VoiceJoin {
                joined_at: chrono::Utc::now().timestamp_millis(),
                last_mute_at: if is_muted { Some(chrono::Utc::now().timestamp_millis()) } else { None },
                total_muted: 0,
            });
            info!("Usuário {} começou a rastrear tempo de call", user_id);
        } else {
            // Mudou de estado no canal (mutou, desmutou, moveu de sala)
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
        // Saiu de todos os canais de voz, deve salvar o tempo
        if let Some((_, mut join)) = tracker.remove(&user_id) {
            // Se saiu mutado, contabilizar o tempo mutado final
            if let Some(last_mute) = join.last_mute_at {
                join.total_muted += chrono::Utc::now().timestamp_millis() - last_mute;
            }
            
            let total_time_ms = chrono::Utc::now().timestamp_millis() - join.joined_at;
            let valid_time_ms = total_time_ms - join.total_muted;
            
            info!("Usuário {} saiu. Tempo total: {} ms, Mutado: {} ms, Válido: {} ms", user_id, total_time_ms, join.total_muted, valid_time_ms);
            
            // O Banco de Dados deve somar o "Tempo Válido" no "Tempo Total" para não dar XP a quem fica mutado o dia todo.
            // Argumento 3: tempo válido (vai para sessoes.tempo e usuarios.tempo_total)
            // Argumento 4: tempo mutado (vai para sessoes.tempo_mutado)
            if let Err(e) = VoiceDb::update_user_time(&pool, &user_id, valid_time_ms, join.total_muted).await {
                error!("Falha ao salvar tempo no DB: {}", e);
            }
        }
    }
}
