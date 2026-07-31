use axum::{routing::get, Router};
use tracing::info;

pub async fn start_server() {
    let app = Router::new()
        .route("/", get(|| async { "O bot está online! 🦀" }));

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    
    info!("Servidor Anti-Sleep iniciado em http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
