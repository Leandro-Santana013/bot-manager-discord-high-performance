use serenity::model::gateway::Ready;
use serenity::prelude::*;
use tracing::info;

pub async fn handle(ctx: Context, ready: Ready) {
    info!("🤖 Logado como {}! Sistema de banco de dados e rastreio de call operantes.", ready.user.tag());
    
    let commands = vec![
        crate::commands::mod_cmds::ban::register(),
        crate::commands::mod_cmds::limpar::register(),
        crate::commands::mod_cmds::restringir::register(),
        crate::commands::mod_cmds::travar::register(),
        crate::commands::mod_cmds::destravar::register(),
        crate::commands::mod_cmds::automod::register(),
        crate::commands::mod_cmds::blacklist::register(),
        crate::commands::mod_cmds::unban::register(),
        crate::commands::mod_cmds::desrestringir::register(),
        crate::commands::vip::painelvip::register(),
        crate::commands::vip::config_vip::register(),
        crate::commands::tickets::painel_suporte::register(),
        crate::commands::tickets::config_suporte::register(),
        crate::commands::tickets::ranking::register(),
        crate::commands::metas::painel_cargos::register(),
        crate::commands::metas::config_metas::register(),
        crate::commands::metas::fechar_metas::register(),
        crate::commands::voice::tempo::register(),
        crate::commands::voice::top::register(),
        crate::commands::voice::call::register(),
    ];

    if let Err(why) = serenity::model::application::Command::set_global_commands(&ctx.http, commands).await {
        tracing::error!("Erro ao registrar comandos: {:?}", why);
    }
    
    // Iniciar Cron Jobs
    crate::cron::start_crons(std::sync::Arc::new(ctx.clone())).await;

    // Recupera pessoas que já estavam em call antes do bot reiniciar
    let ctx_clone = ctx.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(10)).await; // Aguarda cache carregar
        let data = ctx_clone.data.read().await;
        if let Some(tracker) = data.get::<crate::events::voice::VoiceTracker>() {
            let mut users_in_voice = Vec::new();
            {
                let guilds = ctx_clone.cache.guilds();
                for guild_id in guilds {
                    if let Some(guild) = ctx_clone.cache.guild(guild_id) {
                        for (user_id, voice_state) in guild.voice_states.iter() {
                            let is_bot = voice_state.member.as_ref().map(|m| m.user.bot).unwrap_or(false);
                            if !is_bot {
                                let is_muted = voice_state.self_mute || voice_state.mute || voice_state.self_deaf || voice_state.deaf;
                                users_in_voice.push((*user_id, is_muted));
                            }
                        }
                    }
                }
            }
            
            let mut recovered = 0;
            for (user_id, is_muted) in users_in_voice {
                let uid_str = user_id.to_string();
                if !tracker.contains_key(&uid_str) {
                    tracker.insert(uid_str.clone(), crate::events::voice::VoiceJoin {
                        joined_at: chrono::Utc::now().timestamp_millis(),
                        last_mute_at: if is_muted { Some(chrono::Utc::now().timestamp_millis()) } else { None },
                        total_muted: 0,
                    });
                    recovered += 1;
                }
            }
            if recovered > 0 {
                tracing::info!("Rastreador recuperou o tempo de call de {} usuário(s) ativos!", recovered);
            }
        }
    });
}
