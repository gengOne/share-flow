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
        println!("\n=== Discovery 初始化 ===");
        
        // Bind to any available port for sending
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        let local_addr = socket.local_addr()?;
        println!("UDP 发送 socket 绑定到: {}", local_addr);
        
        socket.set_broadcast(true)?;
        println!("已启用广播模式");
        
        // Get all broadcast addresses for local networks
        let mut broadcast_addrs = Vec::new();
        
        println!("\n检测网络接口:");
        // Try to get network interfaces and calculate broadcast addresses
        if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
            for (name, ip) in interfaces.iter() {
                println!("  接口: {} -> {}", name, ip);
                
                if let IpAddr::V4(ipv4) = ip {
                    let octets = ipv4.octets();
                    
                    // Skip loopback and APIPA
                    if ipv4.is_loopback() {
                        println!("    -> 跳过 (回环地址)");
                        continue;
                    }
                    
                    if octets[0] == 169 && octets[1] == 254 {
                        println!("    -> 跳过 (APIPA 地址)");
                        continue;
                    }
                    
                    // For private networks, calculate broadcast address
                    // Assuming /24 subnet (255.255.255.0) for simplicity
                    if octets[0] == 192 && octets[1] == 168 {
                        let broadcast = Ipv4Addr::new(octets[0], octets[1], octets[2], 255);
                        broadcast_addrs.push(SocketAddr::new(IpAddr::V4(broadcast), port));
                        println!("    -> ✓ 添加广播地址: {}:{}", broadcast, port);
                    } else if octets[0] == 10 {
                        // For 10.x.x.x networks, also use /24
                        let broadcast = Ipv4Addr::new(octets[0], octets[1], octets[2], 255);
                        broadcast_addrs.push(SocketAddr::new(IpAddr::V4(broadcast), port));
                        println!("    -> ✓ 添加广播地址: {}:{}", broadcast, port);
                    } else if octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31 {
                        // For 172.16-31.x.x networks
                        let broadcast = Ipv4Addr::new(octets[0], octets[1], octets[2], 255);
                        broadcast_addrs.push(SocketAddr::new(IpAddr::V4(broadcast), port));
                        println!("    -> ✓ 添加广播地址: {}:{}", broadcast, port);
                    } else {
                        println!("    -> 跳过 (非私有网络地址)");
                    }
                }
            }
        } else {
            println!("  ⚠ 无法获取网络接口列表");
        }
        
        // Fallback to global broadcast if no specific networks found
        if broadcast_addrs.is_empty() {
            println!("\n⚠ 未找到有效的私有网络，使用全局广播");
            broadcast_addrs.push(SocketAddr::new(IpAddr::from([255, 255, 255, 255]), port));
        }
        
        println!("\n最终广播地址列表:");
        for addr in &broadcast_addrs {
            println!("  - {}", addr);
        }
        println!("===================\n");
        
        Ok(Self {
            socket: Arc::new(socket),
            broadcast_addrs,
        })
    }

    pub fn start_broadcast(&self, message: Message) {
        let data = match bincode::serialize(&message) {
            Ok(d) => {
                println!("广播消息序列化成功，大小: {} 字节", d.len());
                d
            },
            Err(e) => {
                eprintln!("❌ 序列化广播消息失败: {}", e);
                return;
            }
        };
        let socket = self.socket.clone();
        let addrs = self.broadcast_addrs.clone();

        println!("启动广播任务，每秒发送一次");
        
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(1));
            
            loop {
                interval.tick().await;
                
                // Broadcast to all network addresses
                for addr in &addrs {
                    if let Err(e) = socket.send_to(&data, addr).await {
                        eprintln!("❌ 广播到 {} 失败: {}", addr, e);
                    }
                }
            }
        });
    }

    pub async fn listen(port: u16, tx: mpsc::Sender<(Message, SocketAddr)>) -> Result<()> {
        println!("\n=== Discovery 监听器 ===");
        let bind_addr = format!("0.0.0.0:{}", port);
        println!("尝试绑定 UDP 监听: {}", bind_addr);
        
        let socket = UdpSocket::bind(&bind_addr).await?;
        let local_addr = socket.local_addr()?;
        println!("✓ UDP 监听器成功绑定到: {}", local_addr);
        println!("等待接收广播消息...");
        println!("===================\n");
        
        let mut buf = [0u8; 1024];

        tokio::spawn(async move {
            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((len, addr)) => {
                        match bincode::deserialize::<Message>(&buf[..len]) {
                            Ok(msg) => {
                                if let Err(e) = tx.send((msg, addr)).await {
                                    eprintln!("❌ 发送到主循环失败: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                eprintln!("❌ 消息反序列化失败: {} (来自 {})", e, addr);
                            }
                        }
                    }
                    Err(e) => eprintln!("❌ UDP 接收错误: {}", e),
                }
            }
        });
        Ok(())
    }
}
