import React, { useEffect, useState, useRef } from 'react';
import { backend } from '../services/backend';
import { InputEvent } from '../types';
import { MousePointer2 } from 'lucide-react';

interface InputVisualizerProps {
  connectedDeviceName: string;
}

const InputVisualizer: React.FC<InputVisualizerProps> = ({ connectedDeviceName }) => {
  const [cursorPos, setCursorPos] = useState({ x: 50, y: 50 }); // Percentage
  const [activeKey, setActiveKey] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleRemoteInput = (event: InputEvent) => {
      if (event.type === 'mousemove' && event.dx && event.dy) {
        setCursorPos(prev => {
          // Add sensitivity factor
          const speed = 0.2;
          let newX = prev.x + (event.dx! * speed);
          let newY = prev.y + (event.dy! * speed);
          
          // Clamp to screen 0-100
          newX = Math.max(0, Math.min(100, newX));
          newY = Math.max(0, Math.min(100, newY));
          
          return { x: newX, y: newY };
        });
      } else if (event.type === 'keydown') {
        setActiveKey(event.key || 'Unknown');
        setTimeout(() => setActiveKey(null), 300);
      }
    };

    backend.on('remote-input', handleRemoteInput);

    return () => {
        backend.off('remote-input', handleRemoteInput);
    };
  }, []);

  return (
    <div className="flex-1 flex flex-col bg-gray-900 rounded-xl overflow-hidden border border-gray-700 shadow-2xl relative" ref={containerRef}>
      {/* Mock Desktop Background */}
      <div className="absolute inset-0 bg-gradient-to-br from-indigo-900/20 to-purple-900/20" />
      <div className="absolute inset-0 opacity-10" 
           style={{ backgroundImage: 'radial-gradient(circle at 2px 2px, white 1px, transparent 0)', backgroundSize: '40px 40px' }}>
      </div>

      {/* Header Overlay */}
      <div className="relative z-10 p-4 bg-gray-900/80 backdrop-blur border-b border-gray-700 flex justify-between items-center">
        <div>
            <h4 className="text-gray-300 text-sm font-medium">远程桌面预览</h4>
            <p className="text-xs text-gray-500">正在接收来自 {connectedDeviceName} 的输入指令</p>
        </div>
        <div className="flex gap-2">
            <div className="w-3 h-3 rounded-full bg-red-500/20 border border-red-500/50"></div>
            <div className="w-3 h-3 rounded-full bg-yellow-500/20 border border-yellow-500/50"></div>
            <div className="w-3 h-3 rounded-full bg-green-500/20 border border-green-500/50"></div>
        </div>
      </div>

      {/* Virtual Cursor */}
      <div 
        className="absolute w-6 h-6 pointer-events-none transition-transform duration-75 z-20 text-emerald-400 drop-shadow-[0_0_10px_rgba(52,211,153,0.5)]"
        style={{ 
            left: `${cursorPos.x}%`, 
            top: `${cursorPos.y}%`,
            transform: `translate(-50%, -50%)`
        }}
      >
        <MousePointer2 fill="currentColor" size={24} className="rotate-[-15deg]" />
        <div className="absolute top-full left-full ml-2 mt-1 px-2 py-0.5 bg-emerald-500/90 text-gray-900 text-[10px] font-bold rounded whitespace-nowrap">
            {connectedDeviceName}
        </div>
      </div>

      {/* Key Press HUD */}
      <div className="absolute bottom-8 left-1/2 -translate-x-1/2 flex gap-2 z-20">
        {activeKey && (
            <div className="px-6 py-3 bg-white/10 backdrop-blur-md border border-white/20 rounded-xl text-white font-mono text-xl shadow-xl animate-bounce-short">
                {activeKey}
            </div>
        )}
      </div>
    </div>
  );
};

export default InputVisualizer;