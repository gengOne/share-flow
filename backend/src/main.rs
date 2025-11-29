mod protocol;
mod discovery;
mod transport;
mod websocket;
mod input_capture;

use anyhow::Result;
use discovery::Discovery;
use protocol::Message;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use transport::Transport;
use websocket::{DeviceInfo, InputEvent, WebSocketServer, WsMessage};
use input_capture::{CaptureControl, InputCapture};

fn get_local_ip() -> String {
    // Try to get all network interfaces
    if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
        let mut candidates = Vec::new();
        
        for (name, ip) in interfaces.iter() {
            if let IpAddr::V4(ipv4) = ip {
                let octets = ipv4.octets();
                let name_lower = name.to_lowercase();
                
                // Skip loopback
                if ipv4.is_loopback() {
                    continue;
                }
                
                // Skip common virtual adapters
                if name_lower.contains("virtualbox") 
                    || name_lower.contains("vmware")
                    || name_lower.contains("hyper-v")
                    || name_lower.contains("vethernet")
                    || name_lower.contains("docker")
                    || name_lower.contains("wsl")
                    || octets[0] == 198 && octets[1] == 18  // Skip 198.18.x.x (Windows ICS)
                    || octets[0] == 169 && octets[1] == 254 // Skip 169.254.x.x (APIPA)
                {
                    println!("Skipping virtual adapter {}: {}", name, ip);
                    continue;
                }
                
                // Prioritize 192.168.x.x (most common home/office networks)
                if octets[0] == 192 && octets[1] == 168 {
                    println!("Found preferred local IP on interface {}: {}", name, ip);
                    return ip.to_string();
                }
                
                // Store other private IPs as candidates
                if octets[0] == 10 || (octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31) {
                    candidates.push((name.clone(), ip.to_string()));
                }
            }
        }
        
        // Use first candidate if no 192.168.x.x found
        if let Some((name, ip)) = candidates.first() {
            println!("Using local IP on interface {}: {}", name, ip);
            return ip.clone();
        }
    }
    
    // Final fallback
    local_ip_address::local_ip()
        .unwrap_or_else(|_| "127.0.0.1".parse().unwrap())
        .to_string()
}

#[tokio::main]
async fn main() -> Result<()> {
    let udp_port = 8080;
    let ws_port = 4000;
    
    // Generate unique device ID based on hostname and MAC address
    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "Unknown".to_string());
    
    // Use hostname as device name
    let device_name = hostname.clone();
    
    // Create unique ID from hostname (you can also use MAC address or UUID)
    let device_id = format!("device-{}", hostname.replace(" ", "-").to_lowercase());

    println!("Starting ShareFlow Service");
    println!("  UDP Discovery: port {}", udp_port);
    println!("  WebSocket API: ws://127.0.0.1:{}", ws_port);

    // WebSocket Server
    let (ws_server, _ws_rx) = WebSocketServer::new(ws_port);
    let ws_server = Arc::new(ws_server);
    
    // Start WebSocket server
    let ws_server_clone = Arc::clone(&ws_server);
    tokio::spawn(async move {
        if let Err(e) = ws_server_clone.start().await {
            eprintln!("WebSocket server error: {}", e);
        }
    });

    // Discovered devices with last seen timestamp
    let discovered_devices = Arc::new(Mutex::new(HashMap::<String, (DeviceInfo, std::time::Instant)>::new()));

    // Input capture state
    let is_capturing = Arc::new(Mutex::new(false));
    let input_capture_handle: Arc<Mutex<Option<Arc<InputCapture>>>> = Arc::new(Mutex::new(None));

    // Channel for discovery events
    let (tx, mut rx) = mpsc::channel::<(Message, SocketAddr)>(32);

    // Start Discovery Listener
    println!("\n>>> 启动 Discovery 监听器...");
    Discovery::listen(udp_port, tx.clone()).await?;

    // Start Discovery Broadcaster
    println!("\n>>> 创建 Discovery 广播器...");
    let discovery = Discovery::new(udp_port).await?;
    
    let broadcast_msg = Message::Discovery {
        id: device_id.to_string(),
        name: device_name.to_string(),
        port: udp_port,
    };
    println!("\n>>> 启动广播，消息内容: {:?}", broadcast_msg);
    discovery.start_broadcast(broadcast_msg);

    // Start TCP Listener for peer connections
    let listener = TcpListener::bind(format!("0.0.0.0:{}", udp_port)).await?;
    
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((mut stream, addr)) => {
                    println!("New TCP connection from: {}", addr);
                    tokio::spawn(async move {
                        loop {
                            match Transport::recv_tcp(&mut stream).await {
                                Ok(msg) => println!("Received TCP from {}: {:?}", addr, msg),
                                Err(e) => {
                                    println!("TCP connection error from {}: {}", addr, e);
                                    break;
                                }
                            }
                        }
                    });
                }
                Err(e) => println!("TCP accept error: {}", e),
            }
        }
    });

    println!("Service is running. Waiting for events...");

    // Subscribe to WebSocket messages
    let mut ws_broadcast_rx = ws_server.get_sender().subscribe();

    // Get local IP address - prefer 192.168.x.x or 10.x.x.x
    let local_ip = get_local_ip();

    println!("Local IP: {}", local_ip);
    println!("Hostname: {}", hostname);
    println!("Device ID: {}", device_id);

    // Input capture receiver (will be initialized when capture starts)
    let mut input_rx: Option<mpsc::UnboundedReceiver<CaptureControl>> = None;

    // Main event loop
    loop {
        tokio::select! {
            // Handle UDP Discovery Events
            Some((msg, addr)) = rx.recv() => {
                match msg {
                    Message::Discovery { id, name, port: peer_port } => {
                        // Skip our own broadcasts
                        if id == device_id {
                            continue;
                        }
                        
                        let device = DeviceInfo {
                            id: id.clone(),
                            name: name.clone(),
                            ip: addr.ip().to_string(),
                            device_type: "DESKTOP".to_string(),
                        };
                        
                        let now = std::time::Instant::now();
                        
                        // Only log and notify if this is a new device
                        let mut devices = discovered_devices.lock().await;
                        if !devices.contains_key(&id) {
                            println!("\n✓ 发现新设备: {} ({}) at {}:{}", name, id, addr.ip(), peer_port);
                            devices.insert(id.clone(), (device.clone(), now));
                            
                            // Notify frontend
                            ws_server.broadcast(WsMessage::DeviceFound { device });
                        } else {
                            // Update timestamp silently
                            devices.insert(id.clone(), (device, now));
                        }
                    }
                    _ => println!("收到其他消息: {:?}", msg),
                }
            }
            
            // Handle WebSocket messages from frontend
            Ok(ws_msg) = ws_broadcast_rx.recv() => {
                println!("\n[WS] 收到前端消息: {:?}", ws_msg);
                match ws_msg {
                    WsMessage::GetLocalInfo => {
                        println!("Frontend requested local device info");
                        let local_device = DeviceInfo {
                            id: device_id.to_string(),
                            name: hostname.clone(),
                            ip: local_ip.clone(),
                            device_type: "DESKTOP".to_string(),
                        };
                        ws_server.broadcast(WsMessage::LocalInfo { device: local_device });
                    }
                    WsMessage::StartDiscovery => {
                        println!("\n>>> 前端请求开始发现设备");
                        
                        // Clean up stale devices (not seen in last 10 seconds)
                        let mut devices = discovered_devices.lock().await;
                        let now = std::time::Instant::now();
                        devices.retain(|id, (_, last_seen)| {
                            let age = now.duration_since(*last_seen).as_secs();
                            if age > 10 {
                                println!("  移除过期设备: {} ({}秒未见)", id, age);
                                false
                            } else {
                                true
                            }
                        });
                        
                        let device_count = devices.len();
                        
                        if device_count > 0 {
                            println!("  发送 {} 个已发现的设备到前端", device_count);
                            for (device, _) in devices.values() {
                                ws_server.broadcast(WsMessage::DeviceFound { device: device.clone() });
                            }
                        } else {
                            println!("  当前没有已发现的设备");
                        }
                        
                        println!("  发现服务持续运行中...");
                    }
                    WsMessage::StartCapture => {
                        println!("Frontend requested to start input capture");
                        let mut capturing = is_capturing.lock().await;
                        if !*capturing {
                            let (capture, rx) = InputCapture::new();
                            let capture = Arc::new(capture);
                            capture.clone().start_capture();
                            
                            *input_capture_handle.lock().await = Some(capture);
                            input_rx = Some(rx);
                            *capturing = true;
                            
                            println!("Input capture started");
                        }
                    }
                    WsMessage::StopCapture => {
                        println!("Frontend requested to stop input capture");
                        let mut capturing = is_capturing.lock().await;
                        if *capturing {
                            *input_capture_handle.lock().await = None;
                            input_rx = None;
                            *capturing = false;
                            println!("Input capture stopped");
                        }
                    }
                    WsMessage::RequestConnection { target_device_id } => {
                        println!("\n>>> 前端请求连接到设备: {}", target_device_id);
                        
                        // Get target device info
                        let devices = discovered_devices.lock().await;
                        if let Some((device, _)) = devices.get(&target_device_id) {
                            let target_ip = device.ip.clone();
                            let target_name = device.name.clone();
                            drop(devices);
                            
                            println!("  目标设备: {} ({})", target_name, target_ip);
                            println!("  尝试建立 TCP 连接到 {}:8080", target_ip);
                            
                            let ws_server_clone = Arc::clone(&ws_server);
                            let device_id_clone = target_device_id.clone();
                            
                            tokio::spawn(async move {
                                use tokio::net::TcpStream;
                                use tokio::time::{timeout, Duration};
                                
                                match timeout(
                                    Duration::from_secs(5),
                                    TcpStream::connect(format!("{}:8080", target_ip))
                                ).await {
                                    Ok(Ok(stream)) => {
                                        println!("  ✓ TCP 连接成功: {}", stream.peer_addr().unwrap());
                                        ws_server_clone.broadcast(WsMessage::ConnectionEstablished { 
                                            device_id: device_id_clone 
                                        });
                                    }
                                    Ok(Err(e)) => {
                                        eprintln!("  ❌ TCP 连接失败: {}", e);
                                        ws_server_clone.broadcast(WsMessage::ConnectionFailed { 
                                            device_id: device_id_clone,
                                            reason: format!("连接失败: {}", e)
                                        });
                                    }
                                    Err(_) => {
                                        eprintln!("  ❌ 连接超时");
                                        ws_server_clone.broadcast(WsMessage::ConnectionFailed { 
                                            device_id: device_id_clone,
                                            reason: "连接超时".to_string()
                                        });
                                    }
                                }
                            });
                        } else {
                            eprintln!("  ❌ 未找到设备: {}", target_device_id);
                            ws_server.broadcast(WsMessage::ConnectionFailed {
                                device_id: target_device_id,
                                reason: "设备未找到".to_string()
                            });
                        }
                    }
                    WsMessage::CancelConnection => {
                        println!("Frontend cancelled connection request");
                    }
                    WsMessage::AcceptConnection { target_device_id } => {
                        println!("Frontend accepted connection from: {}", target_device_id);
                        ws_server.broadcast(WsMessage::ConnectionEstablished { 
                            device_id: target_device_id 
                        });
                    }
                    WsMessage::Disconnect => {
                        println!("Frontend requested disconnect");
                        
                        // Stop input capture when disconnecting
                        let mut capturing = is_capturing.lock().await;
                        if *capturing {
                            *input_capture_handle.lock().await = None;
                            input_rx = None;
                            *capturing = false;
                            println!("Input capture stopped on disconnect");
                        }
                        
                        ws_server.broadcast(WsMessage::Disconnected);
                    }
                    WsMessage::SendInput { event: _ } => {
                        // TODO: Forward input to connected peer
                        // println!("Received input event: {:?}", event);
                    }
                    _ => {}
                }
            }
            
            // Handle captured input events
            Some(control_msg) = async {
                if let Some(ref mut rx) = input_rx {
                    rx.recv().await
                } else {
                    std::future::pending().await
                }
            } => {
                match control_msg {
                    CaptureControl::InputEvent(input_event) => {
                        // Convert to WebSocket message and broadcast
                        let ws_event = InputEvent {
                            event_type: input_event.event_type,
                            x: input_event.x,
                            y: input_event.y,
                            dx: input_event.dx,
                            dy: input_event.dy,
                            key: input_event.key,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64,
                        };
                        
                        ws_server.broadcast(WsMessage::LocalInput { event: ws_event });
                    }
                    CaptureControl::ExitRequested => {
                        println!("Exit requested from input capture - stopping capture and disconnecting");
                        
                        // Stop input capture
                        let mut capturing = is_capturing.lock().await;
                        if *capturing {
                            if let Some(capture) = input_capture_handle.lock().await.as_ref() {
                                capture.stop_capture();
                            }
                            *input_capture_handle.lock().await = None;
                            input_rx = None;
                            *capturing = false;
                        }
                        
                        // Notify frontend to disconnect
                        ws_server.broadcast(WsMessage::Disconnected);
                    }
                }
            }
        }
    }
}
