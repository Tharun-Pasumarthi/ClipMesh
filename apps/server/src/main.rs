use clipmesh_server::{build_app, state::AppState};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,clipmesh_server=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let host = std::env::var("CLIPMESH_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("CLIPMESH_PORT")
        .unwrap_or_else(|_| "8085".to_string())
        .parse()
        .unwrap_or(8085);

    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "clipmesh-dev-jwt-secret-minimum-32-chars-key".to_string());

    let state = AppState::new(jwt_secret);
    let app = build_app(state);

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    tracing::info!("Starting ClipMesh Relay Server on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
