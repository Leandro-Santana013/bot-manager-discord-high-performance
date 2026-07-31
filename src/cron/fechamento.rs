use std::sync::Arc;
use serenity::prelude::Context;
use tokio::time::{sleep, Duration};
use tracing::info;
use chrono::{Utc, Timelike, Datelike, Weekday};

pub async fn start(_ctx: Arc<Context>) {
    // Loop para verificar o fechamento semanal de metas todo domingo à meia noite
    loop {
        let now = Utc::now();
        
        // Calcula quantos segundos faltam até o próximo domingo 00:00:00 UTC
        let days_until_sunday = match now.weekday() {
            Weekday::Sun => {
                // Se já é domingo, verifica se já passou da meia-noite
                if now.hour() == 0 && now.minute() == 0 && now.second() < 2 {
                    0 // É agora!
                } else {
                    7 // Próximo domingo
                }
            }
            Weekday::Mon => 6,
            Weekday::Tue => 5,
            Weekday::Wed => 4,
            Weekday::Thu => 3,
            Weekday::Fri => 2,
            Weekday::Sat => 1,
        };

        if days_until_sunday == 0 {
            info!("Fechamento Automático Iniciado!");
            
            // TODO: Processar o ranking de tempo de call, resetar metas, e anunciar os vencedores
            
            // Dorme 7 dias até o próximo domingo
            sleep(Duration::from_secs(7 * 24 * 60 * 60)).await;
        } else {
            // Calcula segundos restantes até meia-noite do próximo domingo
            let seconds_remaining_today = 
                (23 - now.hour() as u64) * 3600 
                + (59 - now.minute() as u64) * 60 
                + (60 - now.second() as u64);
            let total_sleep_secs = seconds_remaining_today + (days_until_sunday - 1) * 24 * 3600;
            
            info!("[Cron Fechamento] Dormindo {} horas até o próximo domingo 00:00 UTC.", total_sleep_secs / 3600);
            sleep(Duration::from_secs(total_sleep_secs)).await;
        }
    }
}
