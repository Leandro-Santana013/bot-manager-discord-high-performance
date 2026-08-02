use std::sync::Arc;
use serenity::prelude::Context;
use tokio::time::{sleep, Duration};
use tracing::info;
use chrono::{Utc, Timelike, Datelike, Weekday, FixedOffset};

pub async fn start(_ctx: Arc<Context>) {
    let br_tz = FixedOffset::east_opt(-3 * 3600).unwrap();

    loop {
        let now = Utc::now().with_timezone(&br_tz);

        let days_until_reset = match now.weekday() {
            Weekday::Mon => {

                if now.hour() == 0 && now.minute() == 0 && now.second() < 5 {
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

            sleep(Duration::from_secs(60)).await;
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
