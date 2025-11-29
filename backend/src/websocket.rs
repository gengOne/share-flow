use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::{accept_async, tungstenite::Message};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WsMessage {
    // From Frontend
    StartDiscovery,
    StartCapture,
    StopCapture,
    RequestConnection { target_device_id: String },
    CancelConnection,
    AcceptConnection { target_device_id: String },
    RejectConnection { target_device_id: String },
    Disconnect,
    SendInput { event: InputEvent },
    GetLocalInfo,
    
    // To Frontend
    LocalInfo { device: DeviceInfo },
    LocalInput { event: InputEvent },
    DeviceFound { device: DeviceInfo },
    ConnectionRequest { device: DeviceInfo },
    ConnectionRequestCancelled { device_id: String },
    ConnectionEstablished { device_id: String },
    ConnectionFailed { device_id: String, reason: String },
    Disconnected,
    RemoteInput { event: InputEvent },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub ip: String,
    #[serde(rename = "type")]
    pub device_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub dx: Option<f64>,
    pub dy: Option<f64>,
    pub key: Option<String>,
    pub timestamp: u64,
}

pub struct WebSocketServer {
    port: u16,
    broadcast_tx: broadcast::Sender<WsMessage>,
}

impl WebSocketServer {
    pub fn new(port: u16) -> (Self, broadcast::Receiver<WsMessage>) {
        let (broadcast_tx, broadcast_rx) = broadcast::channel(100);
        (Self { port, broadcast_tx }, broadcast_rx)
    }

    pub async fn start(self: Arc<Self>) -> Result<()> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        println!("WebSocket server listening on ws://{}", addr);

        while let Ok((stream, addr)) = listener.accept().await {
            println!("New WebSocket connection from: {}", addr);
            let server = Arc::clone(&self);
            tokio::spawn(async move {
                if let Err(e) = server.handle_connection(stream).await {
                    eprintln!("WebSocket connection error: {}", e);
                }
            });
        }

        Ok(())
    }

    async fn handle_connection(&self, stream: TcpStream) -> Result<()> {
        let ws_stream = accept_async(stream).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        let mut broadcast_rx = self.broadcast_tx.subscribe();
        let broadcast_tx = self.broadcast_tx.clone();

        // Spawn task to forward broadcast messages to this client
        let sender_task = tokio::spawn(async move {
            while let Ok(msg) = broadcast_rx.recv().await {
                if let Ok(json) = serde_json::to_string(&msg) {
                    if ws_sender.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }
        });

        // Handle incoming messages from client
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                        // Echo back to main loop via broadcast
                        let _ = broadcast_tx.send(ws_msg);
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(_) => break,
                _ => {}
            }
        }

        sender_task.abort();
        Ok(())
    }

    pub fn broadcast(&self, msg: WsMessage) {
        // Debug: print the JSON that will be sent
        if let Ok(json) = serde_json::to_string(&msg) {
            println!("[WS] 广播消息: {}", json);
        }
        let _ = self.broadcast_tx.send(msg);
    }

    pub fn get_sender(&self) -> broadcast::Sender<WsMessage> {
        self.broadcast_tx.clone()
    }
}
