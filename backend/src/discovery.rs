use crate::protocol::Message;
use anyhow::Result;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::time::{self, Duration};

pub struct Discovery {
    socket: Arc<UdpSocket>,
    broadcast_addrs: Vec<SocketAddr>,
}

impl Discovery {
    pub async fn new(port: u16) -> Result<Self> {
        // Bind to any available port for sending
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.set_broadcast(true)?;
        
        // Get all broadcast addresses for local networks
        let mut broadcast_addrs = Vec::new();
        
        // Try to get network interfaces and calculate broadcast addresses
        if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
            for (_name, ip) in interfaces.iter() {
                if let IpAddr::V4(ipv4) = ip {
                    let octets = ipv4.octets();
                    
                    // Skip loopback and APIPA
                    if ipv4.is_loopback() || (octets[0] == 169 && octets[1] == 254) {
                        continue;
                    }
                    
                    // For private networks, calculate broadcast address
                    // Assuming /24 subnet (255.255.255.0) for simplicity
                    if octets[0] == 192 && octets[1] == 168 {
                        let broadcast = Ipv4Addr::new(octets[0], octets[1], octets[2], 255);
                        broadcast_addrs.push(SocketAddr::new(IpAddr::V4(broadcast), port));
                        println!("Will broadcast to: {}", broadcast);
                    } else if octets[0] == 10 {
                        // For 10.x.x.x networks, also use /24
                        let broadcast = Ipv4Addr::new(octets[0], octets[1], octets[2], 255);
                        broadcast_addrs.push(SocketAddr::new(IpAddr::V4(broadcast), port));
                        println!("Will broadcast to: {}", broadcast);
                    } else if octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31 {
                        // For 172.16-31.x.x networks
                        let broadcast = Ipv4Addr::new(octets[0], octets[1], octets[2], 255);
                        broadcast_addrs.push(SocketAddr::new(IpAddr::V4(broadcast), port));
                        println!("Will broadcast to: {}", broadcast);
                    }
                }
            }
        }
        
        // Fallback to global broadcast if no specific networks found
        if broadcast_addrs.is_empty() {
            println!("No specific networks found, using global broadcast");
            broadcast_addrs.push(SocketAddr::new(IpAddr::from([255, 255, 255, 255]), port));
        }
        
        Ok(Self {
            socket: Arc::new(socket),
            broadcast_addrs,
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
        let addrs = self.broadcast_addrs.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                // Broadcast to all network addresses
                for addr in &addrs {
                    if let Err(e) = socket.send_to(&data, addr).await {
                        eprintln!("Broadcast error to {}: {}", addr, e);
                    }
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
