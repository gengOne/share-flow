export enum DeviceType {
  DESKTOP = 'DESKTOP',
  LAPTOP = 'LAPTOP',
  TABLET = 'TABLET',
}

export enum ConnectionState {
  DISCONNECTED = 'DISCONNECTED',
  CONNECTING = 'CONNECTING', // Waiting for handshake
  CONNECTED = 'CONNECTED',
}

export interface Device {
  id: string;
  name: string;
  ip: string;
  type: DeviceType;
  isSelf?: boolean;
}

export interface InputEvent {
  type: 'mousemove' | 'mousedown' | 'mouseup' | 'keydown' | 'keyup';
  x?: number;
  y?: number;
  dx?: number;
  dy?: number;
  key?: string;
  timestamp: number;
}

export interface ConnectionRequest {
  fromDevice: Device;
  timestamp: number;
}