use crate::protocol::Message;
use anyhow::Result;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::time::{self, Duration};

pub struct Discovery {
    socket: Arc<UdpSocket>,
    broadcast_addr: SocketAddr,
}

impl Discovery {
    pub async fn new(port: u16) -> Result<Self> {
        // Bind to any available port for sending
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.set_broadcast(true)?;
        
        let broadcast_addr = SocketAddr::new(IpAddr::from([255, 255, 255, 255]), port);
        
        Ok(Self {
            socket: Arc::new(socket),
            broadcast_addr,
        })
    }

    pub fn start_broadcast(&self, message: Message) {
        let data = match bincode::serialize(&message) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Failed to serialize broadcast message: {}", e);
                return;
            }
        };
        let socket = self.socket.clone();
        let addr = self.broadcast_addr;

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                if let Err(e) = socket.send_to(&data, addr).await {
                    eprintln!("Broadcast error: {}", e);
                }
            }
        });
    }

    pub async fn listen(port: u16, tx: mpsc::Sender<(Message, SocketAddr)>) -> Result<()> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", port)).await?;
        let mut buf = [0u8; 1024];

        tokio::spawn(async move {
            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((len, addr)) => {
                        if let Ok(msg) = bincode::deserialize::<Message>(&buf[..len]) {
                            if let Err(_) = tx.send((msg, addr)).await {
                                break;
                            }
                        }
                    }
                    Err(e) => eprintln!("Discovery receive error: {}", e),
                }
            }
        });
        Ok(())
    }
}
