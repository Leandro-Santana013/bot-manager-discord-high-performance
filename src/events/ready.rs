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
    crate::cron::start_crons(std::sync::Arc::new(ctx)).await;
}
