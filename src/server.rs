use anyhow::Context;
use axum::{
    Router,
    extract::{
        ConnectInfo, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    routing::get,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::select;

use crate::{
    controller::{Controller, ControllerHandle, ControllerOutputEvent, create_controller},
    shared::{LowestIdManager, PlayerId},
};

#[derive(Debug, Clone, Copy)]
pub enum ControllerInputEventRaw {
    Ping,
    Button(u32),          // 0x01
    Triggers(u8, u8),     // 0x02
    StickLeft(i16, i16),  // 0x03
    StickRight(i16, i16), // 0x04
}

#[derive(Debug, Clone, Copy)]
pub enum ControllerInputEvent {
    Button(u32),          // 0x01
    Triggers(u8, u8),     // 0x02
    StickLeft(i16, i16),  // 0x03
    StickRight(i16, i16), // 0x04
}

impl ControllerInputEventRaw {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes.len() {
            4 => Some(Self::Ping),
            5 => {
                // I just need to hope that this will not cause problems
                let kind = bytes[4];

                match kind {
                    0x01 => Some(Self::Button(u32::from_le_bytes(
                        bytes[0..4].try_into().ok()?,
                    ))),
                    0x02 => Some(Self::Triggers(bytes[0], bytes[1])),
                    0x03 => Some(Self::StickLeft(
                        i16::from_le_bytes(bytes[0..2].try_into().ok()?),
                        i16::from_le_bytes(bytes[2..4].try_into().ok()?),
                    )),
                    0x04 => Some(Self::StickRight(
                        i16::from_le_bytes(bytes[0..2].try_into().ok()?),
                        i16::from_le_bytes(bytes[2..4].try_into().ok()?),
                    )),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn into_event(self) -> Option<ControllerInputEvent> {
        match self {
            Self::Button(buttons) => Some(ControllerInputEvent::Button(buttons)),
            Self::StickLeft(x, y) => Some(ControllerInputEvent::StickLeft(x, y)),
            Self::Triggers(left, right) => Some(ControllerInputEvent::Triggers(left, right)),
            Self::StickRight(x, y) => Some(ControllerInputEvent::StickRight(x, y)),
            _ => None,
        }
    }
}

pub struct Server {
    id: LowestIdManager,
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
}

impl Server {
    pub async fn start(
        path: &str,
        shutdown_tx: tokio::sync::broadcast::Sender<()>,
    ) -> anyhow::Result<()> {
        let server = Arc::new(Self {
            shutdown_tx: shutdown_tx.clone(),
            id: LowestIdManager::new(),
        });

        let router: Router<()> = Router::new()
            .route("/ws", get(Server::ws_handler))
            .with_state(server)
            .fallback(|req: axum::http::Request<axum::body::Body>| async move {
                tracing::warn!("Unmatched request: {} {}", req.method(), req.uri());
                (axum::http::StatusCode::NOT_ACCEPTABLE, "websockets only")
            });

        let mut shutdown_serve_rx = shutdown_tx.subscribe();

        // it is very fine to use unwrap since we needed it anyways
        let listener = tokio::net::TcpListener::bind(path).await.unwrap();
        tracing::info!("Listening on {}", listener.local_addr().unwrap());
        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move {
            if let Err(err) = shutdown_serve_rx.recv().await {
                tracing::error!(error = %err, "Could not handle shutdown signal");
            }
        })
        .await
        .unwrap();

        Ok(())
    }

    async fn ws_handler(
        ws: WebSocketUpgrade,
        State(state): State<Arc<Server>>,
        ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ) -> axum::response::Response {
        let state_clone = state.clone();
        ws.on_upgrade(move |socket| async move {
            tracing::debug!("Attempting to upgrade websocket from {}", addr);
            Server::handle_websocket(state_clone, socket, addr).await
        })
    }

    async fn handle_websocket_message(
        handle: &Arc<ControllerHandle>,
        msg: Message,
        socket: &mut WebSocket,
    ) -> anyhow::Result<bool> {
        tracing::debug!("Received message from {} with message {:?}", handle, msg);

        match msg {
            Message::Binary(bytes) => {
                let result = match ControllerInputEventRaw::from_bytes(&bytes) {
                    Some(event) => match event {
                        // Some(event) => handle.send_input_update(event).await,
                        ControllerInputEventRaw::Ping => {
                            socket.send(Message::binary(bytes)).await?;
                            Ok(())
                        }

                        _ => {
                            // I promise if theres more responses than ping and not compatible with
                            // into_event(), then its the dev's fault
                            handle.send_input_update(event.into_event().unwrap()).await
                        }
                    },
                    None => Err(anyhow::format_err!("{} sent an unknown event", handle)),
                };

                return result
                    .with_context(|| format!("Failed to decode response on {}", handle))
                    .map(|_| false);
            }
            Message::Text(_) => {
                // Just message the sender with a custom message

                // We do NOT care if it failed
                let _ = socket
                    .send(Message::Text(
                        "Text is unsupported. Please switch to arraybuffer.".into(),
                    ))
                    .await;

                // we want to close the connection since we know 99% its some
                // random bullshit
                return Err(anyhow::format_err!(
                    "{} sent an text-based request which is incompatible with this server",
                    handle,
                ));
            }
            Message::Close(frame) => {
                let _ = socket.send(Message::Close(frame)).await;
                Ok(true)
            }
            Message::Ping(payload) => {
                return socket
                    .send(Message::Pong(payload))
                    .await
                    .with_context(|| format!("Failed to send a pong to {}", handle))
                    .map(|_| false);
            }
            Message::Pong(_) => {
                // just do nothing
                return Ok(false);
            }
        }
    }

    async fn run_listeners(
        self: Arc<Self>,
        id: PlayerId,
        mut socket: WebSocket,
        who: SocketAddr,
    ) -> anyhow::Result<()> {
        let mut shutdown_signal = self.shutdown_tx.subscribe();
        let (server_tx, mut server_rx) = tokio::sync::mpsc::channel::<ControllerOutputEvent>(1024);

        let (controller, handle) = create_controller(id, server_tx.clone())
            .with_context(|| format!("Failed to create controller for {}", who))?;

        tracing::info!("client({}) connected and associated to {}", who, handle);

        let mut joinset: tokio::task::JoinSet<anyhow::Result<()>> = tokio::task::JoinSet::new();

        joinset.spawn(async move {
            let name = controller.to_string();
            controller
                .run_event()
                .await
                .with_context(|| format!("Could not start controller {}", name))
        });

        let external_handle = handle.clone();

        joinset.spawn(async move {
            'ws_loop: loop {
                select! {
                    _ = shutdown_signal.recv() => {break 'ws_loop; }
                    msg_raw = socket.recv() => {
                        // since msg_raw returns a Option<Result<msg>>, we just run it two times
                        // the Option is for the stream just ended
                        // the Result is where it couldn't even decode the received message
                        let msg = msg_raw
                            .ok_or_else(|| anyhow::anyhow!("{} stream ended", handle))? // first unwrap
                            .with_context(|| format!("Failed to read websocket of {}", handle))?; // second unwrap

                        let result = Self::handle_websocket_message(&handle, msg, &mut socket)
                            .await
                            .with_context(|| format!("Failed to handle message for {}", handle))?;

                        if result == true {
                            break 'ws_loop;
                        }
                    }
                    msg_raw = server_rx.recv() => {
                        if let Some(msg) = msg_raw {
                            let message = msg.into_bytes();
                            tracing::debug!("[Network]: Sending message {:?} to {}", message, handle);
                            socket.send(Message::Binary(message.into()))
                                .await
                                .with_context(|| format!("Failed to send event({:?}) to {}", msg, handle))?;
                        } else {
                            tracing::warn!("[Bug] Send output event is None for {}", handle);
                        }
                    }
                }
            }

            // Final post quit
            handle.terminate().with_context(|| format!("Failed to terminate {}", handle))

            // There is no need for Ok(()) since handle just returns Result<()>
        });

        // although its sad that the leds in linux does not even exist since theres no
        // xinput manager, we will just artificially make one

        let external_handle_2 = external_handle.clone();
        joinset.spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            // Fake assign player_id
            let player_event = ControllerOutputEvent::PlayerChange(u32::from(id));

            tracing::debug!("Attempting to send player number to {}", external_handle_2);

            server_tx.send(player_event).await.with_context(|| {
                format!("Failed to send player number to {}", external_handle_2)
            })?;

            tracing::info!("Player number {} sent to {}", id, external_handle_2);

            Ok(())
        });

        while let Some(result) = joinset.join_next().await {
            result??;
        }

        tracing::info!("{} disconnected gracefully", external_handle);

        Ok(())
    }

    async fn handle_websocket(self: Arc<Self>, socket: WebSocket, who: SocketAddr) {
        match self.id.acquire_id().await {
            Ok(mut id) => {
                if let Err(err) = Self::run_listeners(self, id.inner(), socket, who).await {
                    tracing::error!(error = %err, "An error occurred while running listeners for {}", who);
                }
                id.release().await;
            }
            Err(err) => {
                tracing::error!(error = %err, "Could not assign player id to {}", who);
            }
        }

        tracing::info!("{} disconnected", who);
    }
}
