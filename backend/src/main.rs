mod protocol;
mod discovery;
mod transport;
mod websocket;
mod input_capture;
mod input_simulator;

use anyhow::Result;
use discovery::Discovery;
use protocol::Message;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
// use tokio::time::Duration;
use transport::Transport;
use websocket::{DeviceInfo, InputEvent, WebSocketServer, WsMessage};
use input_capture::{CaptureControl, InputCapture};
use input_simulator::InputSimulator;

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

    // Active TCP connections storage - use channel for lock-free sending
    type MessageSender = mpsc::UnboundedSender<Message>;
    let active_connections = Arc::new(Mutex::new(HashMap::<String, MessageSender>::new()));
    
    // Pending connection requests (addr -> (stream, device_info, timestamp))
    type PendingConnection = (TcpStream, Option<DeviceInfo>, std::time::Instant);
    let pending_connections = Arc::new(Mutex::new(HashMap::<String, PendingConnection>::new()));
    
    // Latest connection request to show to frontend (only one at a time)
    let latest_connection_request = Arc::new(Mutex::new(Option::<DeviceInfo>::None));
    
    // Outgoing connection request (when we are the initiator)
    // Stores the target device ID and a cancel sender
    type CancelSender = tokio::sync::oneshot::Sender<()>;
    let outgoing_request = Arc::new(Mutex::new(Option::<(String, CancelSender)>::None));
    
    // Start TCP Listener for peer connections
    let listener = TcpListener::bind(format!("0.0.0.0:{}", udp_port)).await?;
    let pending_connections_clone = Arc::clone(&pending_connections);
    let latest_request_clone = Arc::clone(&latest_connection_request);
    let ws_server_for_tcp = Arc::clone(&ws_server);
    let discovered_devices_for_tcp = Arc::clone(&discovered_devices);
    
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((mut stream, addr)) => {
                    println!("\n>>> 收到 TCP 连接来自: {}", addr);
                    if let Err(e) = stream.set_nodelay(true) {
                        eprintln!("Failed to set TCP_NODELAY: {}", e);
                    }
                    
                    let ws_server_clone = Arc::clone(&ws_server_for_tcp);
                    let pending_conns = Arc::clone(&pending_connections_clone);
                    let latest_req = Arc::clone(&latest_request_clone);
                    let devices = Arc::clone(&discovered_devices_for_tcp);
                    
                    tokio::spawn(async move {
                        // Read handshake message
                        match Transport::recv_tcp(&mut stream).await {
                            Ok(Message::ConnectRequest) => {
                                println!("  收到连接请求握手");
                                
                                // Find device info by IP
                                let device_info = {
                                    let devs = devices.lock().await;
                                    devs.values()
                                        .find(|(dev, _)| dev.ip == addr.ip().to_string())
                                        .map(|(dev, _)| dev.clone())
                                };
                                
                                if let Some(device) = device_info {
                                    println!("  来自设备: {} ({})", device.name, device.id);
                                    
                                    // Check if there's already a pending request
                                    let mut pending = pending_conns.lock().await;
                                    let now = std::time::Instant::now();
                                    
                                    // Clean up expired pending connections (older than 30 seconds)
                                    let expired: Vec<String> = pending.iter()
                                        .filter(|(_, (_, _, timestamp))| now.duration_since(*timestamp).as_secs() > 30)
                                        .map(|(addr, _)| addr.clone())
                                        .collect();
                                    
                                    for old_addr in expired {
                                        if let Some((mut old_stream, _, _)) = pending.remove(&old_addr) {
                                            println!("  清理过期的待处理连接: {}", old_addr);
                                            let _ = Transport::send_tcp(&mut old_stream, &Message::ConnectResponse { success: false }).await;
                                        }
                                    }
                                    
                                    // Reject other pending connections (only keep the latest)
                                    if !pending.is_empty() {
                                        println!("  ⚠ 已有待处理的连接请求，拒绝旧请求");
                                        for (old_addr, (mut old_stream, _, _)) in pending.drain() {
                                            println!("    拒绝来自 {} 的旧请求", old_addr);
                                            let _ = Transport::send_tcp(&mut old_stream, &Message::ConnectResponse { success: false }).await;
                                        }
                                    }
                                    
                                    // Store new pending connection with timestamp
                                    pending.insert(addr.to_string(), (stream, Some(device.clone()), now));
                                    drop(pending);
                                    
                                    // Save as latest request
                                    *latest_req.lock().await = Some(device.clone());
                                    
                                    // Notify frontend
                                    println!("  通知前端显示连接请求弹窗");
                                    ws_server_clone.broadcast(WsMessage::ConnectionRequest { device });
                                } else {
                                    println!("  ⚠ 未找到设备信息，自动拒绝");
                                    let _ = Transport::send_tcp(&mut stream, &Message::ConnectResponse { success: false }).await;
                                }
                            }
                            Ok(msg) => {
                                println!("  收到意外消息: {:?}", msg);
                            }
                            Err(e) => {
                                println!("  读取握手消息失败: {}", e);
                                
                                // Check if this was a pending connection that got cancelled
                                let mut pending = pending_conns.lock().await;
                                if let Some((_, dev_opt, _)) = pending.remove(&addr.to_string()) {
                                    if let Some(device) = dev_opt {
                                        println!("  连接被取消，通知前端");
                                        let device_id = device.id.clone();
                                        ws_server_clone.broadcast(WsMessage::ConnectionRequestCancelled { 
                                            device_id: device_id.clone()
                                        });
                                        
                                        // Clear latest request if it matches
                                        let mut latest = latest_req.lock().await;
                                        if latest.as_ref().map(|d| &d.id) == Some(&device_id) {
                                            *latest = None;
                                        }
                                    }
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

    // Start periodic cleanup task for expired pending connections
    let pending_conns_cleanup = Arc::clone(&pending_connections);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            
            let mut pending = pending_conns_cleanup.lock().await;
            let now = std::time::Instant::now();
            
            let expired: Vec<String> = pending.iter()
                .filter(|(_, (_, _, timestamp))| now.duration_since(*timestamp).as_secs() > 30)
                .map(|(addr, _)| addr.clone())
                .collect();
            
            for addr in expired {
                if let Some((mut stream, dev, _)) = pending.remove(&addr) {
                    if let Some(device) = dev {
                        println!("\n⏰ 清理超时的待处理连接: {} (来自 {})", addr, device.name);
                    } else {
                        println!("\n⏰ 清理超时的待处理连接: {}", addr);
                    }
                    let _ = Transport::send_tcp(&mut stream, &Message::ConnectResponse { success: false }).await;
                }
            }
        }
    });

    // Subscribe to WebSocket messages
    let mut ws_broadcast_rx = ws_server.get_sender().subscribe();

    // Get local IP address - prefer 192.168.x.x or 10.x.x.x
    let local_ip = get_local_ip();

    println!("Local IP: {}", local_ip);
    println!("Hostname: {}", hostname);
    println!("Device ID: {}", device_id);

    // Input capture receiver (will be initialized when capture starts)
    let mut input_rx: Option<mpsc::UnboundedReceiver<CaptureControl>> = None;

    // Mouse accumulation state removed for immediate transmission
    // let mut accumulated_mouse_delta = (0.0f64, 0.0f64);
    // let mut mouse_flush_interval = tokio::time::interval(Duration::from_millis(1));
    // mouse_flush_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // Main event loop
    loop {
        tokio::select! {
            // Periodic flush of accumulated mouse events
            // Periodic flush removed - sending immediately
            // _ = mouse_flush_interval.tick() => { ... }
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
                        
                        // Check if there's a pending connection request
                        let latest_req = latest_connection_request.lock().await;
                        if let Some(ref device) = *latest_req {
                            println!("  检测到待处理的连接请求，重新发送给前端");
                            ws_server.broadcast(WsMessage::ConnectionRequest { device: device.clone() });
                        }
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
                        
                        // Create cancel channel
                        let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel::<()>();
                        
                        // Save outgoing request with cancel sender
                        *outgoing_request.lock().await = Some((target_device_id.clone(), cancel_tx));
                        
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
                            let active_conns = Arc::clone(&active_connections);
                            let outgoing_req = Arc::clone(&outgoing_request);
                            
                            tokio::spawn(async move {
                                use tokio::net::TcpStream;
                                use tokio::time::Duration;
                                
                                match tokio::time::timeout(
                                    Duration::from_secs(5),
                                    TcpStream::connect(format!("{}:8080", target_ip))
                                ).await {
                                    Ok(Ok(mut stream)) => {
                                        let peer_addr = stream.peer_addr().unwrap();
                                        println!("  ✓ TCP 连接成功: {}", peer_addr);
                                        if let Err(e) = stream.set_nodelay(true) {
                                            eprintln!("Failed to set TCP_NODELAY: {}", e);
                                        }
                                        
                                        // Send handshake
                                        println!("  发送连接请求握手...");
                                        if let Err(e) = Transport::send_tcp(&mut stream, &Message::ConnectRequest).await {
                                            eprintln!("  发送握手失败: {}", e);
                                            ws_server_clone.broadcast(WsMessage::ConnectionFailed { 
                                                device_id: device_id_clone,
                                                reason: format!("握手失败: {}", e)
                                            });
                                            return;
                                        }
                                        
                                        // Wait for response (30 seconds to give user time to accept)
                                        println!("  等待握手响应（等待对方用户确认）...");
                                        
                                        let response_future = Transport::recv_tcp(&mut stream);
                                        
                                        tokio::select! {
                                            _ = &mut cancel_rx => {
                                                println!("  收到取消信号，关闭连接");
                                                *outgoing_req.lock().await = None;
                                                // Connection will be closed when stream is dropped
                                                return;
                                            }
                                            result = tokio::time::timeout(Duration::from_secs(30), response_future) => {
                                                match result {
                                            Ok(Ok(Message::ConnectResponse { success: true })) => {
                                                println!("  ✓ 握手成功，连接已建立");
                                                
                                                // Clear outgoing request
                                                *outgoing_req.lock().await = None;
                                                
                                                // Create channel for lock-free sending
                                                let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<Message>();
                                                let conn_key = format!("{}:{}", target_ip, 8080);
                                                active_conns.lock().await.insert(conn_key.clone(), msg_tx);
                                                println!("  连接已存储: {}", conn_key);
                                                
                                                ws_server_clone.broadcast(WsMessage::ConnectionEstablished { 
                                                    device_id: device_id_clone.clone()
                                                });
                                                
                                                // Split stream for concurrent read/write
                                                let (mut read_half, mut write_half) = tokio::io::split(stream);
                                                
                                                // Spawn dedicated sender task
                                                let active_conns_clone = Arc::clone(&active_conns);
                                                let conn_key_clone = conn_key.clone();
                                                let ws_clone = Arc::clone(&ws_server_clone);
                                                tokio::spawn(async move {
                                                    while let Some(msg) = msg_rx.recv().await {
                                                        if let Err(e) = Transport::send_tcp_split(&mut write_half, &msg).await {
                                                            eprintln!("发送失败: {}", e);
                                                            active_conns_clone.lock().await.remove(&conn_key_clone);
                                                            ws_clone.broadcast(WsMessage::Disconnected);
                                                            break;
                                                        }
                                                    }
                                                });
                                                
                                                // Keep connection alive and handle any incoming messages
                                                loop {
                                                    // Try to receive with timeout
                                                    match tokio::time::timeout(
                                                        Duration::from_secs(1),
                                                        Transport::recv_tcp_split(&mut read_half)
                                                    ).await {
                                                        Ok(Ok(msg)) => {
                                                            println!("收到对方消息: {:?}", msg);
                                                            // Handle any control messages if needed
                                                        }
                                                        Ok(Err(e)) => {
                                                            println!("连接断开: {}", e);
                                                            // Remove from active connections
                                                            active_conns.lock().await.remove(&conn_key);
                                                            ws_server_clone.broadcast(WsMessage::Disconnected);
                                                            break;
                                                        }
                                                        Err(_) => {
                                                            // Timeout, continue
                                                        }
                                                    }
                                                }
                                            }
                                            Ok(Ok(Message::ConnectResponse { success: false })) => {
                                                eprintln!("  ❌ 对方拒绝连接");
                                                *outgoing_req.lock().await = None;
                                                ws_server_clone.broadcast(WsMessage::ConnectionFailed { 
                                                    device_id: device_id_clone,
                                                    reason: "对方拒绝连接".to_string()
                                                });
                                            }
                                            Ok(Ok(msg)) => {
                                                eprintln!("  ❌ 收到意外响应: {:?}", msg);
                                                *outgoing_req.lock().await = None;
                                                ws_server_clone.broadcast(WsMessage::ConnectionFailed { 
                                                    device_id: device_id_clone,
                                                    reason: "握手协议错误".to_string()
                                                });
                                            }
                                            Ok(Err(e)) => {
                                                eprintln!("  ❌ 读取响应失败: {}", e);
                                                *outgoing_req.lock().await = None;
                                                ws_server_clone.broadcast(WsMessage::ConnectionFailed { 
                                                    device_id: device_id_clone,
                                                    reason: format!("读取响应失败: {}", e)
                                                });
                                            }
                                            Err(_) => {
                                                eprintln!("  ❌ 握手超时");
                                                *outgoing_req.lock().await = None;
                                                ws_server_clone.broadcast(WsMessage::ConnectionFailed { 
                                                    device_id: device_id_clone,
                                                    reason: "握手超时".to_string()
                                                });
                                            }
                                        }
                                    }
                                        }
                                    }
                                    Ok(Err(e)) => {
                                        eprintln!("  ❌ TCP 连接失败: {}", e);
                                        *outgoing_req.lock().await = None;
                                        ws_server_clone.broadcast(WsMessage::ConnectionFailed { 
                                            device_id: device_id_clone,
                                            reason: format!("连接失败: {}", e)
                                        });
                                    }
                                    Err(_) => {
                                        eprintln!("  ❌ 连接超时");
                                        *outgoing_req.lock().await = None;
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
                    WsMessage::RejectConnection { target_device_id } => {
                        println!("\n>>> 前端拒绝了来自 {} 的连接", target_device_id);
                        
                        // Clear latest request
                        *latest_connection_request.lock().await = None;
                        
                        // Find and reject pending connection
                        let mut pending = pending_connections.lock().await;
                        let pending_addr = pending.iter()
                            .find(|(_, (_, dev, _))| dev.as_ref().map(|d| &d.id) == Some(&target_device_id))
                            .map(|(addr, _)| addr.clone());
                        
                        if let Some(addr) = pending_addr {
                            if let Some((mut stream, _, _)) = pending.remove(&addr) {
                                println!("  找到待处理连接: {}", addr);
                                println!("  发送拒绝响应");
                                let _ = Transport::send_tcp(&mut stream, &Message::ConnectResponse { success: false }).await;
                            }
                        }
                    }
                    WsMessage::CancelConnection => {
                        println!("\n>>> 前端取消了连接请求");
                        
                        // Get the target device ID and cancel sender from outgoing request
                        let request = outgoing_request.lock().await.take();
                        
                        if let Some((device_id, cancel_tx)) = request {
                            println!("  取消对 {} 的连接请求", device_id);
                            
                            // Send cancel signal
                            let _ = cancel_tx.send(());
                            println!("  已发送取消信号");
                        } else {
                            println!("  没有正在进行的连接请求");
                        }
                    }
                    WsMessage::AcceptConnection { target_device_id } => {
                        println!("\n>>> 前端接受了来自 {} 的连接", target_device_id);
                        
                        // Clear latest request
                        *latest_connection_request.lock().await = None;
                        
                        // Find pending connection by device ID
                        let mut pending = pending_connections.lock().await;
                        let pending_addr = pending.iter()
                            .find(|(_, (_, dev, _))| dev.as_ref().map(|d| &d.id) == Some(&target_device_id))
                            .map(|(addr, _)| addr.clone());
                        
                        if let Some(addr) = pending_addr {
                            if let Some((mut stream, _device, _)) = pending.remove(&addr) {
                                println!("  找到待处理连接: {}", addr);
                                
                                // Send accept response
                                match Transport::send_tcp(&mut stream, &Message::ConnectResponse { success: true }).await {
                                    Ok(_) => {
                                        println!("  ✓ 已发送接受响应");
                                        
                                        // Create channel for lock-free sending
                                        let (msg_tx_send, mut msg_rx_send) = mpsc::unbounded_channel::<Message>();
                                        active_connections.lock().await.insert(addr.clone(), msg_tx_send);
                                        
                                        // Notify frontend
                                        ws_server.broadcast(WsMessage::ConnectionEstablished { 
                                            device_id: target_device_id.clone() 
                                        });
                                        
                                        println!("  ✓ 连接已建立，开始接收输入事件");
                                        
                                        // Create input simulator
                                        let simulator = Arc::new(InputSimulator::new());
                                        
                                        // Split stream for concurrent read/write
                                        let (mut read_half, mut write_half) = tokio::io::split(stream);
                                        
                                        // Spawn dedicated sender task
                                        let active_conns_clone = Arc::clone(&active_connections);
                                        let addr_clone = addr.clone();
                                        let ws_clone = Arc::clone(&ws_server);
                                        tokio::spawn(async move {
                                            while let Some(msg) = msg_rx_send.recv().await {
                                                if let Err(e) = Transport::send_tcp_split(&mut write_half, &msg).await {
                                                    eprintln!("发送失败: {}", e);
                                                    active_conns_clone.lock().await.remove(&addr_clone);
                                                    ws_clone.broadcast(WsMessage::Disconnected);
                                                    break;
                                                }
                                            }
                                        });
                                        
                                        // Start receiving input events
                                        let ws_server_for_input = Arc::clone(&ws_server);
                                        tokio::spawn(async move {
                                            // Channel for receiving messages
                                            let (msg_tx, mut msg_rx) = mpsc::channel::<Message>(1000);
                                            
                                            // Spawn task to receive TCP messages
                                            tokio::spawn(async move {
                                                loop {
                                                    match Transport::recv_tcp_split(&mut read_half).await {
                                                        Ok(msg) => {
                                                            if msg_tx.send(msg).await.is_err() {
                                                                break;
                                                            }
                                                        }
                                                        Err(e) => {
                                                            println!("接收输入事件失败: {}", e);
                                                            break;
                                                        }
                                                    }
                                                }
                                            });
                                            
                                            // Main processing loop - parallel execution for non-blocking
                                            loop {
                                                tokio::select! {
                                                    // Process incoming messages
                                                    Some(msg) = msg_rx.recv() => {
                                                        match msg {
                                                            Message::MouseMove { x, y } => {
                                                                // Mouse movement: execute synchronously to maintain order
                                                                // but it's fast enough (direct Windows API) that it won't block
                                                                if x != 0 || y != 0 {
                                                                    simulator.mouse_move(x, y);
                                                                }
                                                            }
                                                            Message::MouseClick { button, state } => {
                                                                // Execute click in parallel to avoid blocking mouse movement
                                                                let sim = Arc::clone(&simulator);
                                                                tokio::task::spawn_blocking(move || {
                                                                    sim.mouse_click(button, state);
                                                                });
                                                                
                                                                // Forward to frontend for visualization (optional, can be disabled for performance)
                                                                let event = InputEvent {
                                                                    event_type: if state { "mousedown" } else { "mouseup" }.to_string(),
                                                                    x: None,
                                                                    y: None,
                                                                    dx: None,
                                                                    dy: None,
                                                                    key: Some(format!("button{}", button)),
                                                                    timestamp: std::time::SystemTime::now()
                                                                        .duration_since(std::time::UNIX_EPOCH)
                                                                        .unwrap()
                                                                        .as_millis() as u64,
                                                                };
                                                                ws_server_for_input.broadcast(WsMessage::RemoteInput { event });
                                                            }
                                                            Message::KeyPress { key, state } => {
                                                                // Execute keypress in parallel to avoid blocking mouse movement
                                                                let sim = Arc::clone(&simulator);
                                                                tokio::task::spawn_blocking(move || {
                                                                    sim.key_press(key, state);
                                                                });
                                                                
                                                                // Forward to frontend for visualization (optional, can be disabled for performance)
                                                                let event = InputEvent {
                                                                    event_type: if state { "keydown" } else { "keyup" }.to_string(),
                                                                    x: None,
                                                                    y: None,
                                                                    dx: None,
                                                                    dy: None,
                                                                    key: Some(char::from_u32(key).unwrap_or('?').to_string()),
                                                                    timestamp: std::time::SystemTime::now()
                                                                        .duration_since(std::time::UNIX_EPOCH)
                                                                        .unwrap()
                                                                        .as_millis() as u64,
                                                                };
                                                                ws_server_for_input.broadcast(WsMessage::RemoteInput { event });
                                                            }
                                                            _ => {
                                                                println!("收到其他消息: {:?}", msg);
                                                            }
                                                        }
                                                    }
                                                    
                                                    else => break,
                                                }
                                            }
                                            
                                            println!("输入事件接收循环结束");
                                            
                                            // Notify frontend about disconnection
                                            ws_server_for_input.broadcast(WsMessage::Disconnected);
                                        });
                                    }
                                    Err(e) => {
                                        eprintln!("  ❌ 发送响应失败: {}", e);
                                    }
                                }
                            }
                        } else {
                            eprintln!("  ❌ 未找到待处理的连接");
                        }
                    }
                    WsMessage::Disconnect => {
                        println!("\n>>> 前端请求断开连接");
                        
                        // Stop input capture when disconnecting
                        let mut capturing = is_capturing.lock().await;
                        if *capturing {
                            *input_capture_handle.lock().await = None;
                            input_rx = None;
                            *capturing = false;
                            println!("  输入捕获已停止");
                        }
                        
                        // Close all active connections
                        let mut connections = active_connections.lock().await;
                        let conn_count = connections.len();
                        connections.clear();
                        println!("  已关闭 {} 个连接", conn_count);
                        
                        // Clear pending connections
                        pending_connections.lock().await.clear();
                        
                        ws_server.broadcast(WsMessage::Disconnected);
                        println!("  ✓ 断开完成");
                    }
                    WsMessage::SendInput { event } => {
                        // Forward input to connected peer via TCP (lock-free)
                        let connections = active_connections.lock().await;
                        
                        if connections.is_empty() {
                            // No active connection, ignore
                            continue;
                        }
                        
                        match event.event_type.as_str() {
                            "mousemove" => {
                                // Send mouse move immediately (no accumulation)
                                if let (Some(dx), Some(dy)) = (event.dx, event.dy) {
                                    let dx_int = dx as i32;
                                    let dy_int = dy as i32;
                                    
                                    if dx_int != 0 || dy_int != 0 {
                                        let msg = Message::MouseMove { x: dx_int, y: dy_int };
                                        for sender in connections.values() {
                                            let _ = sender.send(msg.clone());
                                        }
                                    }
                                }
                            }
                            _ => {
                                // For other events (clicks, keys), send immediately
                                let msg = match event.event_type.as_str() {
                                    "mousedown" => {
                                        let button = match event.key.as_deref() {
                                            Some("button1") => 1, // Right
                                            Some("button2") => 2, // Middle
                                            _ => 0, // Left
                                        };
                                        Some(Message::MouseClick { button, state: true })
                                    }
                                    "mouseup" => {
                                        let button = match event.key.as_deref() {
                                            Some("button1") => 1, // Right
                                            Some("button2") => 2, // Middle
                                            _ => 0, // Left
                                        };
                                        Some(Message::MouseClick { button, state: false })
                                    }
                                    "keydown" => {
                                        if let Some(key) = event.key {
                                            Some(Message::KeyPress {
                                                key: key.chars().next().unwrap_or('\0') as u32,
                                                state: true,
                                            })
                                        } else {
                                            None
                                        }
                                    }
                                    "keyup" => {
                                        if let Some(key) = event.key {
                                            Some(Message::KeyPress {
                                                key: key.chars().next().unwrap_or('\0') as u32,
                                                state: false,
                                            })
                                        } else {
                                            None
                                        }
                                    }
                                    "wheel" => None, // TODO: Implement wheel support
                                    _ => None,
                                };

                                if let Some(msg) = msg {
                                    for sender in connections.values() {
                                        let _ = sender.send(msg.clone());
                                    }
                                }
                            }
                        };
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
                        // Convert to WebSocket message and broadcast to frontend for visualization
                        // Optimization: Skip mousemove events to prevent frontend crash due to high frequency
                        if input_event.event_type != "mousemove" {
                            let ws_event = InputEvent {
                                event_type: input_event.event_type.clone(),
                                x: input_event.x,
                                y: input_event.y,
                                dx: input_event.dx,
                                dy: input_event.dy,
                                key: input_event.key.clone(),
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis() as u64,
                            };
                            ws_server.broadcast(WsMessage::LocalInput { event: ws_event });
                        }
                        
                        // Forward to connected peer via TCP
                        let connections = active_connections.lock().await;
                        if !connections.is_empty() {
                            match input_event.event_type.as_str() {
                                "mousemove" => {
                                    // Send mouse move immediately (no accumulation)
                                    if let (Some(dx), Some(dy)) = (input_event.dx, input_event.dy) {
                                        let dx_int = dx as i32;
                                        let dy_int = dy as i32;
                                        
                                        if dx_int != 0 || dy_int != 0 {
                                            let msg = Message::MouseMove { x: dx_int, y: dy_int };
                                            for sender in connections.values() {
                                                let _ = sender.send(msg.clone());
                                            }
                                        }
                                    }
                                }
                                "mousedown" | "mouseup" => {
                                    if let Some(key) = input_event.key {
                                        let button = match key.as_str() {
                                            "button0" => 0, // Left
                                            "button1" => 1, // Right
                                            "button2" => 2, // Middle
                                            _ => 0,
                                        };
                                        let state = input_event.event_type == "mousedown";
                                        println!("[主控端] 捕获到鼠标点击: button={}, state={}", button, state);
                                        let msg = Message::MouseClick { button, state };
                                        
                                        for sender in connections.values() {
                                            if sender.send(msg.clone()).is_ok() {
                                                println!("  ✓ 已发送到被控端");
                                            }
                                        }
                                    }
                                }
                                "keydown" | "keyup" => {
                                    if let Some(code) = input_event.key_code {
                                        let state = input_event.event_type == "keydown";
                                        // println!("[主控端] 捕获到按键: code={}, state={}", code, state);
                                        
                                        if code != 0 {
                                            let msg = Message::KeyPress { key: code, state };
                                            
                                            for sender in connections.values() {
                                                let _ = sender.send(msg.clone());
                                            }
                                        }
                                    } else if let Some(key_str) = input_event.key {
                                        // Fallback for legacy support or unmapped keys
                                        // Convert rdev key format (e.g., "KeyA") to character
                                        let key_code = if key_str.starts_with("Key") && key_str.len() == 4 {
                                            // Single letter key like "KeyA" -> 'A'
                                            key_str.chars().nth(3).unwrap_or('\0') as u32
                                        } else if key_str.starts_with("Num") && key_str.len() == 4 {
                                            // Number key like "Num0" -> '0'
                                            key_str.chars().nth(3).unwrap_or('\0') as u32
                                        } else {
                                            // Special keys
                                            match key_str.as_str() {
                                                "Return" => 13,
                                                "Space" => 32,
                                                "Backspace" => 8,
                                                "Tab" => 9,
                                                "Escape" => 27,
                                                _ => 0,
                                            }
                                        };
                                        
                                        if key_code != 0 {
                                            let state = input_event.event_type == "keydown";
                                            println!("[主控端] 捕获到按键(Fallback): key_str={}, key_code={}, state={}", key_str, key_code, state);
                                            let msg = Message::KeyPress { key: key_code, state };
                                            
                                            for sender in connections.values() {
                                                let _ = sender.send(msg.clone());
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
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
                        
                        // Close all active connections (this will notify remote peers)
                        let mut connections = active_connections.lock().await;
                        let conn_count = connections.len();
                        connections.clear(); // Dropping senders will close the channels
                        println!("  已关闭 {} 个连接", conn_count);
                        
                        // Clear pending connections
                        pending_connections.lock().await.clear();
                        
                        // Notify frontend to disconnect
                        ws_server.broadcast(WsMessage::Disconnected);
                        println!("  ✓ 断开完成");
                    }
                }
            }
        }
    }
}
