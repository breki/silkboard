//! Run the status service on loopback by default.

use std::{io::IsTerminal, net::SocketAddr, path::PathBuf};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let database = std::env::var_os("SILKBOARD_DATABASE").map(PathBuf::from);
    let origin = std::env::var_os("SILKBOARD_OWNER_ORIGIN")
        .map(|value| {
            value
                .into_string()
                .map_err(|_| "owner origin must be UTF-8")
        })
        .transpose()?;
    if args.first().map(String::as_str) == Some("owner") {
        if args.len() != 2 {
            return Err(
                "usage: owner set-password | owner prepare-setup (private pipes)"
                    .into(),
            );
        }
        let path =
            database.ok_or("owner commands require SILKBOARD_DATABASE")?;
        let store = silkboard_web::progress::ProgressStore::open(&path)?;
        match args[1].as_str() {
            "set-password" => {
                if std::io::stdin().is_terminal() {
                    return Err(
                        "password must arrive through a private pipe".into()
                    );
                }
                silkboard_web::owner::set_password_from_reader(
                    &store,
                    std::io::stdin().lock(),
                )?;
                eprintln!("Owner password set; all browser sessions revoked");
            }
            "prepare-setup" => {
                if std::io::stdout().is_terminal() {
                    return Err(
                        "setup link requires a private output pipe".into()
                    );
                }
                let origin =
                    origin.ok_or("setup requires SILKBOARD_OWNER_ORIGIN")?;
                println!(
                    "{}",
                    silkboard_web::owner::prepare_setup(&store, &origin)?
                );
            }
            _ => return Err("unknown owner command".into()),
        }
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("publisher") {
        let path =
            database.ok_or("publisher commands require SILKBOARD_DATABASE")?;
        silkboard_web::progress::publisher_command(&path, &args[1..])?;
        eprintln!("Publisher credential operation completed");
        return Ok(());
    }
    if !args.is_empty() {
        return Err("unknown arguments".into());
    }
    let address: SocketAddr = std::env::var("SILKBOARD_LISTEN")
        .unwrap_or_else(|_| "127.0.0.1:3000".into())
        .parse()?;
    let frontend = PathBuf::from(
        std::env::var_os("SILKBOARD_FRONTEND")
            .unwrap_or_else(|| "frontend/dist".into()),
    );
    // Refuse to advertise a running service with no status page installed.
    tokio::fs::metadata(frontend.join("index.html")).await?;
    let router = if let Some(path) = database {
        // Never make the database downloadable through the static asset service.
        let parent = path
            .parent()
            .ok_or("database path needs a parent")?
            .canonicalize()?;
        if parent.starts_with(frontend.canonicalize()?) {
            return Err(
                "database must be outside the static asset directory".into()
            );
        }
        let store = silkboard_web::progress::ProgressStore::open(&path)?;
        if let Some(origin) = origin {
            silkboard_web::create_owner_router(&frontend, store, &origin)?
        } else {
            silkboard_web::create_progress_router(&frontend, store)
        }
    } else {
        if origin.is_some() {
            return Err("owner origin requires SILKBOARD_DATABASE".into());
        }
        silkboard_web::create_router(&frontend)
    };
    let listener = tokio::net::TcpListener::bind(address).await?;
    eprintln!("Silkboard listening on http://{}", listener.local_addr()?);
    axum::serve(listener, router)
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
