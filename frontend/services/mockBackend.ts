import { Device, DeviceType, InputEvent } from '../types';
import { MOCK_REMOTE_DEVICES } from '../constants';

// ----------------------------------------------------------------------------
// Low-Level Implementation Logic (Rust Backend Architecture):
// 
// The actual system-level logic is implemented in the `rust-service` directory.
// 
// Communication Bridge:
// Frontend (Electron/React) <--> WebSocket (ws://127.0.0.1:4000) <--> Rust Service
//
// Rust Service Responsibilities:
// 1. **Discovery (UDP)**:
//    - Uses `tokio::net::UdpSocket` on port 12345.
//    - Broadcasts "SHAREFLOW_HELLO" packets containing device info.
// 
// 2. **Input Hook & Injection (Crate: rdev)**:
//    - `rdev::listen`: Captures global keyboard/mouse events when in HOST mode.
//    - `rdev::simulate`: Injects events received from network when in CLIENT mode.
//
// 3. **Data Transport (TCP)**:
//    - Uses `tokio::net::TcpStream` for low-latency input event streaming.
//    - Protobuf or Bincode for serialization (high performance).
// ----------------------------------------------------------------------------

type Listener = (data: any) => void;

class MockBackendService {
  private listeners: Record<string, Listener[]> = {};
  private connectedDeviceId: string | null = null;
  private isScanning: boolean = false;
  private pendingTimeout: any = null;

  constructor() {
    this.listeners = {};
  }

  on(event: string, callback: Listener) {
    if (!this.listeners[event]) {
      this.listeners[event] = [];
    }
    this.listeners[event].push(callback);
  }

  off(event: string, callback: Listener) {
    if (!this.listeners[event]) return;
    this.listeners[event] = this.listeners[event].filter(cb => cb !== callback);
  }

  emit(event: string, data: any) {
    if (this.listeners[event]) {
      this.listeners[event].forEach(cb => cb(data));
    }
  }

  // --- Mock Implementation ---

  startDiscovery() {
    if (this.isScanning) return;
    this.isScanning = true;
    console.log("[Backend] UDP Broadcaster Started. Listening for devices...");
    
    // Simulate finding devices with a slight random delay
    MOCK_REMOTE_DEVICES.forEach((device, index) => {
      setTimeout(() => {
        this.emit('device-found', device);
      }, (index + 1) * 800 + Math.random() * 500);
    });
  }

  requestConnection(targetDeviceId: string) {
    console.log(`[Backend] Sending TCP handshake to ${targetDeviceId}...`);
    
    // Clear any existing pending request
    if (this.pendingTimeout) clearTimeout(this.pendingTimeout);

    return new Promise<boolean>((resolve) => {
      this.pendingTimeout = setTimeout(() => {
        // Simulate a 90% chance the user accepts, purely for demo happiness
        const success = true; 
        if (success) {
          console.log(`[Backend] Connection accepted by ${targetDeviceId}`);
          this.connectedDeviceId = targetDeviceId;
          this.emit('connection-established', targetDeviceId);
          resolve(true);
        } else {
          console.log(`[Backend] Connection rejected by ${targetDeviceId}`);
          resolve(false);
        }
        this.pendingTimeout = null;
      }, 2500); // 2.5s wait time to simulate user decision
    });
  }

  cancelConnectionRequest() {
    if (this.pendingTimeout) {
        clearTimeout(this.pendingTimeout);
        this.pendingTimeout = null;
        console.log("[Backend] Connection request cancelled by local user.");
    }
  }

  // Called when WE accept an incoming request from the UI
  acceptConnection(targetDeviceId: string) {
      console.log(`[Backend] Accepted incoming connection from ${targetDeviceId}`);
      this.connectedDeviceId = targetDeviceId;
      this.emit('connection-established', targetDeviceId);
  }

  disconnect() {
    console.log("[Backend] Closing socket/hooks...");
    this.connectedDeviceId = null;
    this.emit('disconnected', null);
  }

  sendInputEvent(event: InputEvent) {
    if (!this.connectedDeviceId) return;
    // In real app: Socket.write(protobuf.encode(event))
    // console.log(`[Backend] > Sending ${event.type}`);
  }

  // Demo helper: Simulate another computer trying to connect to US
  simulateIncomingRequest() {
    if (this.connectedDeviceId) return; // Busy
    
    console.log("[Backend] Simulating incoming TCP connection request...");
    const mockDevice: Device = {
        id: 'incoming-demo-1',
        name: 'Guest Laptop (MacBook)',
        ip: '192.168.1.188',
        type: DeviceType.LAPTOP
    };
    this.emit('connection-request', mockDevice);
  }

  simulateIncomingTraffic() {
    if (!this.connectedDeviceId) return;
    
    // Random mouse movements (Simulating received packets)
    const moveInterval = setInterval(() => {
        if (!this.connectedDeviceId) {
            clearInterval(moveInterval);
            return;
        }
        this.emit('remote-input', {
            type: 'mousemove',
            dx: (Math.random() - 0.5) * 40,
            dy: (Math.random() - 0.5) * 40,
            timestamp: Date.now()
        } as InputEvent);
    }, 16); // 60fps

    // Random key presses
    const keyInterval = setInterval(() => {
        if (!this.connectedDeviceId) {
            clearInterval(keyInterval);
            return;
        }
        const keys = ['H', 'E', 'L', 'L', 'O', 'Enter', 'Cmd'];
        const key = keys[Math.floor(Math.random() * keys.length)];
        this.emit('remote-input', {
            type: 'keydown',
            key: key,
            timestamp: Date.now()
        } as InputEvent);
    }, 2000);
  }
}

export const backend = new MockBackendService();