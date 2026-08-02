use serenity::prelude::*;
use serenity::model::prelude::*;
use sqlx::PgPool;
use tracing::error;
use std::sync::Arc;
use tokio::sync::RwLock;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;


mod database;
mod events;
mod cron;
mod commands;

struct Handler;

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        events::ready::handle(ctx, ready).await;
    }

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        events::voice::handle(ctx, old, new).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: serenity::model::application::Interaction) {
        events::interactions::handle(ctx, interaction).await;
    }

    async fn message(&self, ctx: Context, msg: Message) {
        events::message::handle(ctx, msg).await;
    }
}
pub struct DatabasePool;

impl serenity::prelude::TypeMapKey for DatabasePool {
    type Value = PgPool;
}

pub struct FontCache;

impl serenity::prelude::TypeMapKey for FontCache {
    type Value = Vec<u8>;
}

pub struct FontCacheBold;

impl serenity::prelude::TypeMapKey for FontCacheBold {
    type Value = Vec<u8>;
}

pub struct BlacklistNotify;

impl serenity::prelude::TypeMapKey for BlacklistNotify {
    type Value = Arc<tokio::sync::Notify>;
}

pub struct AutomodCache;

impl serenity::prelude::TypeMapKey for AutomodCache {
    type Value = Arc<RwLock<Vec<String>>>;
}

pub struct HttpClient;

impl serenity::prelude::TypeMapKey for HttpClient {
    type Value = reqwest::Client;
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();
    
    // Inicia o servidor web (Fly.io / Render / Port Scanner) IMEDIATAMENTE
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("Ligando servidor Anti-Sleep em {}...", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tokio::spawn(async move {
        let app = axum::Router::new().route("/", axum::routing::get(|| async { "O bot está online! 🦀" }));
        let _ = axum::serve(listener, app).await;
    });

    // Carrega .env
    dotenvy::dotenv().ok();
    
    let token = std::env::var("DISCORD_TOKEN").expect("A variável DISCORD_TOKEN não foi encontrada no .env");
    
    let intents = GatewayIntents::GUILDS 
        | GatewayIntents::GUILD_VOICE_STATES 
        | GatewayIntents::GUILD_MESSAGES 
        | GatewayIntents::MESSAGE_CONTENT;

    // Inicializa Banco de Dados
    let db_url = std::env::var("DATABASE_URL").expect("A variável DATABASE_URL é obrigatória para o PostgreSQL");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    tracing::info!("Banco de dados PostgreSQL conectado com sucesso.");
    
    // Inicia tabelas
    crate::database::voice::VoiceDb::init(&pool).await;
    crate::database::tickets::TicketDb::init(&pool).await;
    crate::database::vip::VipDb::init(&pool).await;
    crate::database::payments::PaymentDb::init(&pool).await;
    crate::database::blacklist::BlacklistDb::init(&pool).await;

    // Carregar cache do automod
    let raw_list = crate::database::tickets::TicketDb::get_config(&pool, "automod_words", "[]").await;
    let automod_words: Vec<String> = serde_json::from_str(&raw_list).unwrap_or_default();
    let automod_cache = Arc::new(RwLock::new(automod_words));

    let blacklist_notify = Arc::new(tokio::sync::Notify::new());
    let http_client = reqwest::Client::new();

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .type_map_insert::<events::voice::VoiceTracker>(std::sync::Arc::new(dashmap::DashMap::new()))
        .type_map_insert::<DatabasePool>(pool.clone())
        .type_map_insert::<FontCache>(std::fs::read("assets/fonts/Inter-Regular.ttf").unwrap_or_default())
        .type_map_insert::<FontCacheBold>(std::fs::read("assets/fonts/Inter-Bold.ttf").unwrap_or_default())
        .type_map_insert::<BlacklistNotify>(blacklist_notify)
        .type_map_insert::<AutomodCache>(automod_cache)
        .type_map_insert::<HttpClient>(http_client)
        .await?;

    if let Err(why) = client.start().await {
        error!("Erro ao iniciar o cliente discord: {:?}", why);
    }

    Ok(())
}
