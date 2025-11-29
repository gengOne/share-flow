import React from 'react';
import { ConnectionState } from '../types';
import { Wifi, WifiOff, RefreshCw } from 'lucide-react';

interface StatusBadgeProps {
  status: ConnectionState;
}

const StatusBadge: React.FC<StatusBadgeProps> = ({ status }) => {
  if (status === ConnectionState.CONNECTED) {
    return (
      <div className="flex items-center gap-2 px-3 py-1 bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 rounded-full text-sm font-medium">
        <Wifi size={14} />
        <span>已安全连接</span>
      </div>
    );
  }

  if (status === ConnectionState.CONNECTING) {
    return (
      <div className="flex items-center gap-2 px-3 py-1 bg-yellow-500/10 border border-yellow-500/20 text-yellow-400 rounded-full text-sm font-medium">
        <RefreshCw size={14} className="animate-spin" />
        <span>正在握手协商...</span>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2 px-3 py-1 bg-gray-700/50 border border-gray-600 text-gray-400 rounded-full text-sm font-medium">
      <WifiOff size={14} />
      <span>未连接</span>
    </div>
  );
};

export default StatusBadge;