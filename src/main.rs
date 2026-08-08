// Vkontroller - Turns your browser into a virtual game controller
// Copyright (C) 2026  flamfrosticboio
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

mod controller;
mod server;
mod shared;

async fn shutdown_signal(shutdown_tx: tokio::sync::broadcast::Sender<()>) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    // This will block until ctrl+c or terminate signal has received
    tokio::select! {
        _ = ctrl_c => { tracing::warn!("Received Ctrl+C, shutting down gracefully..."); },
        _ = terminate => { tracing::warn!("Received SIGTERM, shutting down gracefully..."); },
    }

    // since the last was blocking, this should not execute directly so its fine if we just
    // let it expose here
    let _ = shutdown_tx.send(());
}

#[tokio::main]
async fn main() {
    // initialize logging
    tracing_subscriber::fmt::init();
    // tracing_subscriber::registry()
    //     .with(console_subscriber::spawn())
    //     .with(tracing_subscriber::fmt::layer())
    //     .try_init()
    //     .unwrap();

    let (shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(1);
    tokio::spawn(shutdown_signal(shutdown_tx.clone()));

    let host = std::env::var("HOST").unwrap_or("localhost:8000".into());
    tracing::info!("Hosting server on '{}'", host);

    if let Err(err) = server::Server::start(host.as_str(), shutdown_tx.clone()).await {
        tracing::error!(error = %err, "Failed to start server");
    };

    return;
}
