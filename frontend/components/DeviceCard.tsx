import React from 'react';
import { Device, DeviceType, ConnectionState } from '../types';
import { Monitor, Laptop, Smartphone, ArrowRight, Activity, X } from 'lucide-react';
import Button from './Button';

interface DeviceCardProps {
  device: Device;
  onConnect: (device: Device) => void;
  onCancel: () => void;
  connectionStatus: ConnectionState;
  isConnectingToThis: boolean;
}

const DeviceCard: React.FC<DeviceCardProps> = ({ 
  device, 
  onConnect, 
  onCancel,
  connectionStatus,
  isConnectingToThis
}) => {
  const getIcon = () => {
    switch (device.type) {
      case DeviceType.DESKTOP: return <Monitor size={24} className="text-indigo-400" />;
      case DeviceType.LAPTOP: return <Laptop size={24} className="text-purple-400" />;
      case DeviceType.TABLET: return <Smartphone size={24} className="text-pink-400" />;
      default: return <Monitor size={24} className="text-gray-400" />;
    }
  };

  const isConnected = connectionStatus === ConnectionState.CONNECTED && isConnectingToThis;
  const isGlobalConnecting = connectionStatus === ConnectionState.CONNECTING;

  // If we are connecting to THIS device, show Cancel.
  // If we are connecting to ANOTHER device, disable Connect button.
  // If we are idle, show Connect button.

  return (
    <div className={`
      relative group flex items-center justify-between p-4 rounded-xl border transition-all duration-300
      ${isConnected 
        ? 'bg-emerald-900/10 border-emerald-500/30 shadow-[0_0_15px_rgba(16,185,129,0.1)]' 
        : isConnectingToThis
            ? 'bg-indigo-900/20 border-indigo-500/50'
            : 'bg-gray-800/40 border-gray-700/50 hover:bg-gray-800/80 hover:border-gray-600'
      }
    `}>
      <div className="flex items-center gap-4">
        <div className={`p-3 rounded-lg transition-colors ${isConnected ? 'bg-emerald-500/10' : 'bg-gray-800'}`}>
          {getIcon()}
        </div>
        <div>
          <h3 className="font-semibold text-gray-200">{device.name}</h3>
          <p className="text-xs text-gray-500 font-mono flex items-center gap-2">
            {device.ip}
            {isConnected && <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse"></span>}
          </p>
        </div>
      </div>

      <div>
        {isConnected ? (
           <div className="flex items-center gap-1 px-3 py-1 bg-emerald-500/20 text-emerald-400 text-xs font-bold uppercase tracking-wider rounded">
             <Activity size={12} />
             <span>已配对</span>
           </div>
        ) : isConnectingToThis ? (
            <Button 
                variant="danger" 
                size="sm"
                onClick={onCancel}
                className="animate-in fade-in zoom-in duration-200"
            >
                <X size={16} /> 取消
            </Button>
        ) : (
          <Button 
            variant="secondary" 
            size="sm"
            onClick={() => onConnect(device)}
            disabled={isGlobalConnecting} // Disable all connect buttons if any connection is in progress
            className={`transition-all ${isGlobalConnecting ? 'opacity-30 cursor-not-allowed' : 'opacity-0 group-hover:opacity-100 translate-x-2 group-hover:translate-x-0'}`}
          >
            连接 <ArrowRight size={16} />
          </Button>
        )}
      </div>
    </div>
  );
};

export default DeviceCard;