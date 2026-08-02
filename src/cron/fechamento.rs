use std::sync::Arc;
use serenity::prelude::Context;
use tokio::time::{sleep, Duration};
use tracing::{info, error};
use chrono::{Utc, Timelike, Datelike, Weekday, FixedOffset};
use serenity::builder::{CreateMessage, CreateAttachment};
use crate::database::tickets::TicketDb;

pub async fn start(ctx: Arc<Context>) {
    let br_tz = match FixedOffset::east_opt(-3 * 3600) {
        Some(tz) => tz,
        None => return,
    };

    loop {
        let now = Utc::now().with_timezone(&br_tz);

        let days_until_reset = match now.weekday() {
            Weekday::Mon => {
                if now.hour() == 0 && now.minute() == 0 && now.second() < 10 {
                    0
                } else {
                    7
                }
            }
            Weekday::Tue => 6,
            Weekday::Wed => 5,
            Weekday::Thu => 4,
            Weekday::Fri => 3,
            Weekday::Sat => 2,
            Weekday::Sun => 1,
        };

        if days_until_reset == 0 {
            info!("Fechamento Semanal Automático Iniciado (Horário de Brasília)!");

            let guilds = ctx.cache.guilds();
            let pool = {
                let data = ctx.data.read().await;
                data.get::<crate::DatabasePool>().cloned()
            };

            for guild_id in guilds {
                info!("Executando fechamento de metas automático para a guilda: {}", guild_id);
                let relatorio = crate::commands::metas::fechar_metas::execute_closing_for_guild(&ctx, guild_id).await;

                if let Some(ref pool) = pool {
                    let logs_channel = TicketDb::get_config(pool, "ticket_logs_channel", "").await;
                    if let Ok(chan_id) = logs_channel.parse::<u64>() {
                        let channel = serenity::model::id::ChannelId::new(chan_id);
                        let attachment = CreateAttachment::bytes(relatorio.into_bytes(), "fechamento_semanal_metas.txt");
                        let msg = CreateMessage::new()
                            .content("⏰ **Fechamento Semanal Automático de Metas Concluído!** Segue o relatório detalhado em anexo:")
                            .add_file(attachment);
                        if let Err(e) = channel.send_message(&ctx.http, msg).await {
                            error!("Erro ao enviar log de fechamento no canal {}: {}", chan_id, e);
                        }
                    }
                }
            }

            sleep(Duration::from_secs(65)).await;
        } else {
            let seconds_remaining_today =
                (23 - now.hour() as u64) * 3600
                + (59 - now.minute() as u64) * 60
                + (60 - now.second() as u64);
            let total_sleep_secs = seconds_remaining_today + (days_until_reset - 1) * 24 * 3600;

            info!("[Cron Fechamento] Dormindo {} horas até a virada de Domingo para Segunda 00:00 (Horário de Brasília).", total_sleep_secs / 3600);
            sleep(Duration::from_secs(total_sleep_secs)).await;
        }
    }
}
