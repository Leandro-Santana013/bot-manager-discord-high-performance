pub mod payments;
pub mod blacklist;
pub mod fechamento;
pub mod mercado_pago;

use std::sync::Arc;
use serenity::prelude::Context;
use tracing::info;

pub async fn start_crons(ctx: Arc<Context>) {
    info!("Iniciando tarefas agendadas (Cron Jobs)...");

    let ctx_clone = ctx.clone();
    tokio::spawn(async move {
        blacklist::start(ctx_clone).await;
    });

    let ctx_clone2 = ctx.clone();
    tokio::spawn(async move {
        payments::start(ctx_clone2).await;
    });

    let ctx_clone3 = ctx.clone();
    tokio::spawn(async move {
        fechamento::start(ctx_clone3).await;
    });
}
