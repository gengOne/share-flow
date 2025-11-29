import { Device, DeviceType } from './types';

export const APP_NAME = "ShareFlow";
export const EXIT_SHORTCUT = "Ctrl + Alt + Q";

export const MOCK_LOCAL_DEVICE: Device = {
  id: 'local-1',
  name: 'This MacBook Pro',
  ip: '192.168.1.105',
  type: DeviceType.LAPTOP,
  isSelf: true,
};

export const MOCK_REMOTE_DEVICES: Device[] = [
  {
    id: 'remote-1',
    name: 'Living Room PC',
    ip: '192.168.1.110',
    type: DeviceType.DESKTOP,
  },
  {
    id: 'remote-2',
    name: 'Work Station (Linux)',
    ip: '192.168.1.112',
    type: DeviceType.DESKTOP,
  },
  {
    id: 'remote-3',
    name: 'Surface Go',
    ip: '192.168.1.115',
    type: DeviceType.TABLET,
  },
];
