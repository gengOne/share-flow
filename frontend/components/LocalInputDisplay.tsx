import React, { useState, useEffect } from 'react';
import { MousePointer2, Keyboard } from 'lucide-react';
import { backend } from '../services/backend';

interface LocalInputDisplayProps {
  connectedDeviceName: string;
}

const LocalInputDisplay: React.FC<LocalInputDisplayProps> = ({ connectedDeviceName }) => {
  const [recentKeys, setRecentKeys] = useState<string[]>([]);
  const [mouseActivity, setMouseActivity] = useState(0);
  const [lastMouseMove, setLastMouseMove] = useState<{ dx: number; dy: number } | null>(null);

  useEffect(() => {
    let mouseTimer: any = null;

    const handleLocalInput = (event: any) => {
      if (event.type === 'mousemove') {
        if (event.x !== undefined && event.y !== undefined) {
          setLastMouseMove({ dx: event.x, dy: event.y });
          setMouseActivity(prev => prev + 1);
          
          if (mouseTimer) clearTimeout(mouseTimer);
          mouseTimer = setTimeout(() => {
            setLastMouseMove(null);
          }, 100);
        }
      } else if (event.type === 'keydown' && event.key) {
        const key = event.key.replace('Key', '').replace('Digit', '');
        setRecentKeys(prev => {
          const newKeys = [key, ...prev].slice(0, 10);
          return newKeys;
        });

        // Remove key after 2 seconds
        setTimeout(() => {
          setRecentKeys(prev => prev.filter(k => k !== key));
        }, 2000);
      }
    };

    backend.on('local-input', handleLocalInput);

    return () => {
      backend.off('local-input', handleLocalInput);
      if (mouseTimer) clearTimeout(mouseTimer);
    };
  }, []);

  return (
    <div className="absolute bottom-6 right-6 z-30 space-y-3">
      {/* Mouse Activity Indicator */}
      <div className="bg-gray-900/90 backdrop-blur border border-gray-700 rounded-xl p-4 shadow-2xl min-w-[200px]">
        <div className="flex items-center gap-3 mb-3">
          <div className={`p-2 rounded-lg transition-colors ${lastMouseMove ? 'bg-indigo-500/20 text-indigo-400' : 'bg-gray-800 text-gray-500'}`}>
            <MousePointer2 size={20} />
          </div>
          <div>
            <p className="text-xs text-gray-500">鼠标活动</p>
            <p className="text-sm font-mono text-gray-300">{mouseActivity} 次移动</p>
          </div>
        </div>
        {lastMouseMove && (
          <div className="text-[10px] font-mono text-gray-600 bg-gray-800/50 rounded px-2 py-1">
            Δx: {lastMouseMove.dx.toFixed(0)} | Δy: {lastMouseMove.dy.toFixed(0)}
          </div>
        )}
      </div>

      {/* Keyboard Activity */}
      {recentKeys.length > 0 && (
        <div className="bg-gray-900/90 backdrop-blur border border-gray-700 rounded-xl p-4 shadow-2xl min-w-[200px]">
          <div className="flex items-center gap-3 mb-3">
            <div className="p-2 rounded-lg bg-emerald-500/20 text-emerald-400">
              <Keyboard size={20} />
            </div>
            <div>
              <p className="text-xs text-gray-500">键盘输入</p>
              <p className="text-sm font-mono text-gray-300">{recentKeys.length} 个按键</p>
            </div>
          </div>
          <div className="flex flex-wrap gap-1.5">
            {recentKeys.slice(0, 8).map((key, index) => (
              <div
                key={`${key}-${index}`}
                className="px-2 py-1 bg-gray-800 border border-gray-700 rounded text-xs font-mono text-gray-300 animate-in fade-in zoom-in duration-200"
                style={{ animationDelay: `${index * 50}ms` }}
              >
                {key}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Target Device Info */}
      <div className="bg-indigo-900/30 backdrop-blur border border-indigo-500/30 rounded-xl p-3 shadow-2xl">
        <p className="text-[10px] text-indigo-400 mb-1">正在控制</p>
        <p className="text-sm font-medium text-white">{connectedDeviceName}</p>
      </div>
    </div>
  );
};

export default LocalInputDisplay;
