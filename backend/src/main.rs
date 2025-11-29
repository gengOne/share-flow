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
    let device_name = "RustService";
    let device_id = "unique-id-123";

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

    // Discovered devices
    let discovered_devices = Arc::new(Mutex::new(HashMap::<String, DeviceInfo>::new()));

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
    
    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "Unknown Device".to_string());

    println!("Local IP: {}", local_ip);
    println!("Hostname: {}", hostname);

    // Input capture receiver (will be initialized when capture starts)
    let mut input_rx: Option<mpsc::UnboundedReceiver<CaptureControl>> = None;

    // Main event loop
    loop {
        tokio::select! {
            // Handle UDP Discovery Events
            Some((msg, addr)) = rx.recv() => {
                println!("\n>>> 主循环收到 UDP 消息来自 {}", addr);
                match msg {
                    Message::Discovery { id, name, port: peer_port } => {
                        println!("  消息类型: Discovery");
                        println!("  设备 ID: {}", id);
                        println!("  设备名称: {}", name);
                        println!("  端口: {}", peer_port);
                        
                        // Skip our own broadcasts
                        if id == device_id {
                            println!("  -> 跳过 (这是本机的广播)");
                            continue;
                        }
                        
                        println!("  ✓ 发现新设备: {} ({}) at {}:{}", name, id, addr.ip(), peer_port);
                        
                        let device = DeviceInfo {
                            id: id.clone(),
                            name: name.clone(),
                            ip: addr.ip().to_string(),
                            device_type: "DESKTOP".to_string(),
                        };
                        
                        // Store device
                        discovered_devices.lock().await.insert(id.clone(), device.clone());
                        println!("  -> 已保存到设备列表");
                        
                        // Notify frontend
                        ws_server.broadcast(WsMessage::DeviceFound { device });
                        println!("  -> 已通知前端");
                    }
                    _ => println!("  其他消息类型: {:?}", msg),
                }
            }
            
            // Handle WebSocket messages from frontend
            Ok(ws_msg) = ws_broadcast_rx.recv() => {
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
                        println!("Frontend requested discovery start");
                        // Discovery is already running, just acknowledge
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
                        println!("Frontend requested connection to: {}", target_device_id);
                        // TODO: Implement actual TCP connection logic
                        // For now, simulate success after 2 seconds
                        let ws_server_clone = Arc::clone(&ws_server);
                        tokio::spawn(async move {
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                            ws_server_clone.broadcast(WsMessage::ConnectionEstablished { 
                                device_id: target_device_id 
                            });
                        });
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
