import { Device, DeviceType, InputEvent } from '../types';

type Listener = (data: any) => void;

interface WsDeviceInfo {
  id: string;
  name: string;
  ip: string;
  type: string;
}

interface WsMessage {
  type: string;
  device?: WsDeviceInfo;
  deviceId?: string;
  targetDeviceId?: string;
  reason?: string;
  event?: any;
}

class RealBackendService {
  private ws: WebSocket | null = null;
  private listeners: Record<string, Listener[]> = {};
  private reconnectTimer: any = null;
  private isConnecting: boolean = false;

  constructor() {
    this.connect();
  }

  private connect() {
    if (this.isConnecting || (this.ws && this.ws.readyState === WebSocket.OPEN)) {
      return;
    }

    this.isConnecting = true;
    console.log('[RealBackend] Connecting to Rust service at ws://127.0.0.1:4000');

    try {
      this.ws = new WebSocket('ws://127.0.0.1:4000');

      this.ws.onopen = () => {
        console.log('[RealBackend] Connected to Rust service');
        this.isConnecting = false;
        if (this.reconnectTimer) {
          clearTimeout(this.reconnectTimer);
          this.reconnectTimer = null;
        }
        // Request local device info immediately after connection
        setTimeout(() => {
          this.getLocalInfo();
        }, 100);
      };

      this.ws.onmessage = (event) => {
        try {
          const msg: WsMessage = JSON.parse(event.data);
          this.handleMessage(msg);
        } catch (e) {
          console.error('[RealBackend] Failed to parse message:', e);
        }
      };

      this.ws.onerror = (error) => {
        console.error('[RealBackend] WebSocket error:', error);
      };

      this.ws.onclose = () => {
        console.log('[RealBackend] Disconnected from Rust service');
        this.isConnecting = false;
        this.ws = null;
        
        // Auto-reconnect after 3 seconds
        this.reconnectTimer = setTimeout(() => {
          this.connect();
        }, 3000);
      };
    } catch (e) {
      console.error('[RealBackend] Failed to create WebSocket:', e);
      this.isConnecting = false;
      
      // Retry after 3 seconds
      this.reconnectTimer = setTimeout(() => {
        this.connect();
      }, 3000);
    }
  }

  private handleMessage(msg: WsMessage) {
    console.log('[RealBackend] Received:', msg);

    switch (msg.type) {
      case 'localInfo':
        if (msg.device) {
          const device: Device = {
            id: msg.device.id,
            name: msg.device.name,
            ip: msg.device.ip,
            type: this.mapDeviceType(msg.device.type),
            isSelf: true,
          };
          this.emit('local-info', device);
        }
        break;

      case 'localInput':
        if (msg.event) {
          this.emit('local-input', msg.event);
        }
        break;

      case 'deviceFound':
        if (msg.device) {
          const device: Device = {
            id: msg.device.id,
            name: msg.device.name,
            ip: msg.device.ip,
            type: this.mapDeviceType(msg.device.type),
          };
          this.emit('device-found', device);
        }
        break;

      case 'connectionRequest':
        if (msg.device) {
          const device: Device = {
            id: msg.device.id,
            name: msg.device.name,
            ip: msg.device.ip,
            type: this.mapDeviceType(msg.device.type),
          };
          this.emit('connection-request', device);
        }
        break;

      case 'connectionEstablished':
        if (msg.deviceId) {
          this.emit('connection-established', msg.deviceId);
        }
        break;

      case 'connectionFailed':
        this.emit('connection-failed', msg.reason || 'Unknown error');
        break;

      case 'disconnected':
        this.emit('disconnected', null);
        break;

      case 'remoteInput':
        if (msg.event) {
          this.emit('remote-input', msg.event);
        }
        break;
    }
  }

  private mapDeviceType(type: string): DeviceType {
    switch (type.toUpperCase()) {
      case 'DESKTOP':
        return DeviceType.DESKTOP;
      case 'LAPTOP':
        return DeviceType.LAPTOP;
      case 'TABLET':
        return DeviceType.TABLET;
      default:
        return DeviceType.DESKTOP;
    }
  }

  private send(msg: any) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
    } else {
      console.warn('[RealBackend] WebSocket not connected, cannot send message');
    }
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

  // --- Public API ---

  getLocalInfo() {
    console.log('[RealBackend] Requesting local device info...');
    this.send({ type: 'getLocalInfo' });
  }

  startDiscovery() {
    console.log('[RealBackend] Starting discovery...');
    this.send({ type: 'startDiscovery' });
  }

  startCapture() {
    console.log('[RealBackend] Starting input capture...');
    this.send({ type: 'startCapture' });
  }

  stopCapture() {
    console.log('[RealBackend] Stopping input capture...');
    this.send({ type: 'stopCapture' });
  }

  requestConnection(targetDeviceId: string): Promise<boolean> {
    console.log(`[RealBackend] Requesting connection to ${targetDeviceId}`);
    this.send({ 
      type: 'requestConnection', 
      targetDeviceId 
    });

    return new Promise((resolve) => {
      const onEstablished = (deviceId: string) => {
        if (deviceId === targetDeviceId) {
          this.off('connection-established', onEstablished);
          this.off('connection-failed', onFailed);
          resolve(true);
        }
      };

      const onFailed = () => {
        this.off('connection-established', onEstablished);
        this.off('connection-failed', onFailed);
        resolve(false);
      };

      this.on('connection-established', onEstablished);
      this.on('connection-failed', onFailed);

      // Timeout after 30 seconds
      setTimeout(() => {
        this.off('connection-established', onEstablished);
        this.off('connection-failed', onFailed);
        resolve(false);
      }, 30000);
    });
  }

  cancelConnectionRequest() {
    console.log('[RealBackend] Cancelling connection request');
    this.send({ type: 'cancelConnection' });
  }

  acceptConnection(targetDeviceId: string) {
    console.log(`[RealBackend] Accepting connection from ${targetDeviceId}`);
    this.send({ 
      type: 'acceptConnection', 
      targetDeviceId 
    });
  }

  disconnect() {
    console.log('[RealBackend] Disconnecting...');
    this.send({ type: 'disconnect' });
  }

  sendInputEvent(event: InputEvent) {
    this.send({ 
      type: 'sendInput', 
      event: {
        type: event.type,
        x: event.x,
        y: event.y,
        dx: event.dx,
        dy: event.dy,
        key: event.key,
        timestamp: event.timestamp,
      }
    });
  }
}

export const backend = new RealBackendService();
