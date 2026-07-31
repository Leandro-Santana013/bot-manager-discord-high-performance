pub mod mod_cmds;
pub mod vip;
pub mod tickets;
pub mod metas;
pub mod voice;

use serenity::model::application::CommandInteraction;
use serenity::prelude::Context;
use tracing::error;

pub async fn handle_command(ctx: &Context, interaction: &CommandInteraction) {
    match interaction.data.name.as_str() {
        "ban" => mod_cmds::ban::run(ctx, interaction).await,
        "limpar" => mod_cmds::limpar::run(ctx, interaction).await,
        "restringir" => mod_cmds::restringir::run(ctx, interaction).await,
        "travar" => mod_cmds::travar::run(ctx, interaction).await,
        "destravar" => mod_cmds::destravar::run(ctx, interaction).await,
        "automod" => mod_cmds::automod::run(ctx, interaction).await,
        "blacklist" => mod_cmds::blacklist::run(ctx, interaction).await,
        "unban" => mod_cmds::unban::run(ctx, interaction).await,
        "desrestringir" => mod_cmds::desrestringir::run(ctx, interaction).await,
        "painelvip" => vip::painelvip::run(ctx, interaction).await,
        "config_vip" => vip::config_vip::run(ctx, interaction).await,
        "painel_suporte" => tickets::painel_suporte::run(ctx, interaction).await,
        "config_suporte" => tickets::config_suporte::run(ctx, interaction).await,
        "ranking" => tickets::ranking::run(ctx, interaction).await,
        "top" => voice::top::run(ctx, interaction).await,
        "tempo" => voice::tempo::run(ctx, interaction).await,
        "call" => voice::call::run(ctx, interaction).await,
        "painel_cargos" => metas::painel_cargos::run(ctx, interaction).await,
        "config_metas" => metas::config_metas::run(ctx, interaction).await,
        "fechar_metas" => metas::fechar_metas::run(ctx, interaction).await,
        _ => {
            error!("Comando não implementado: {}", interaction.data.name);
        }
    }
}
