use serenity::model::channel::Message;
use serenity::prelude::*;
use tracing::info;

pub async fn handle(ctx: Context, msg: Message) {
    if msg.author.bot {
        return;
    }

    let cache = {
        let data = ctx.data.read().await;
        data.get::<crate::AutomodCache>().expect("AutomodCache not initialized").clone()
    };
    let words = cache.read().await.clone();
    let msg_lower = msg.content.to_lowercase();
    for word in words {
        if msg_lower.contains(&word) {
            let _ = msg.delete(&ctx.http).await;
            let _ = msg.channel_id.send_message(&ctx.http, serenity::builder::CreateMessage::new()
                .content(format!("⚠️ <@{}>, sua mensagem foi deletada pois continha uma palavra bloqueada pelo Automod.", msg.author.id))
            ).await;
            return;
        }
    }

    if !msg.content.starts_with("biz!") {
        return;
    }

    let command = msg.content.strip_prefix("biz!").unwrap_or("").trim().to_lowercase();
    let parts: Vec<&str> = command.split_whitespace().collect();

    if parts.is_empty() {
        return;
    }

    match parts[0] {
        "painel_suporte" => {
            info!("Comando de prefixo recebido: biz!painel_suporte");
            crate::commands::tickets::painel_suporte::run_message(&ctx, &msg).await;
        }
        "config_vip" => {
            info!("Comando de prefixo recebido: biz!config_vip");
            crate::commands::vip::config_vip::run_message(&ctx, &msg).await;
        }
        "painelvip" => {
            info!("Comando de prefixo recebido: biz!painelvip");

        }
        "tempo" => {
            info!("Comando de prefixo recebido: biz!tempo");
            crate::commands::voice::tempo::run_message(&ctx, &msg).await;
        }
        "top" => {
            info!("Comando de prefixo recebido: biz!top");
            crate::commands::voice::top::run_message(&ctx, &msg).await;
        }
        _ => {}
    }
}
