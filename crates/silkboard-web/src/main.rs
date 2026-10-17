//! Run the status service on loopback by default.

use std::{net::SocketAddr, path::PathBuf};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address: SocketAddr = std::env::var("SILKBOARD_LISTEN")
        .unwrap_or_else(|_| "127.0.0.1:3000".into())
        .parse()?;
    let frontend = PathBuf::from(
        std::env::var_os("SILKBOARD_FRONTEND")
            .unwrap_or_else(|| "frontend/dist".into()),
    );
    // Refuse to advertise a running service with no status page installed.
    tokio::fs::metadata(frontend.join("index.html")).await?;
    let listener = tokio::net::TcpListener::bind(address).await?;
    eprintln!("Silkboard listening on http://{}", listener.local_addr()?);
    axum::serve(listener, silkboard_web::create_router(&frontend))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut terminate = signal(SignalKind::terminate())
            .expect("could not install SIGTERM handler");
        tokio::select! {
            result = tokio::signal::ctrl_c() => result.expect("Ctrl+C handler failed"),
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c()
        .await
        .expect("Ctrl+C handler failed");
}
