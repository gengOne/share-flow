import React, { useState, useEffect, useCallback, useRef } from 'react';
import { MousePointer2, Settings, Shield, Cast, AlertCircle, Check, X, Loader2 } from 'lucide-react';
import { backend } from './services/backend';
import { Device, ConnectionState, InputEvent } from './types';
import { MOCK_LOCAL_DEVICE, EXIT_SHORTCUT } from './constants';
import DeviceCard from './components/DeviceCard';
import StatusBadge from './components/StatusBadge';
import Button from './components/Button';
import InputVisualizer from './components/InputVisualizer';
import LocalInputDisplay from './components/LocalInputDisplay';

enum AppMode {
  IDLE = 'IDLE',
  HOSTING = 'HOSTING', // Sender (We control them)
  CLIENT = 'CLIENT',   // Receiver (They control us)
}

const App: React.FC = () => {
  const [connectionStatus, setConnectionStatus] = useState<ConnectionState>(ConnectionState.DISCONNECTED);
  const [appMode, setAppMode] = useState<AppMode>(AppMode.IDLE);
  const [devices, setDevices] = useState<Device[]>([]);
  const [connectedDevice, setConnectedDevice] = useState<Device | null>(null);
  const [pendingDeviceId, setPendingDeviceId] = useState<string | null>(null);
  const [incomingRequest, setIncomingRequest] = useState<Device | null>(null);
  const [localDevice, setLocalDevice] = useState<Device>(MOCK_LOCAL_DEVICE);
  
  const captureRef = useRef<HTMLDivElement>(null);

  // --- Discovery & Connection Events ---
  useEffect(() => {
    backend.getLocalInfo();
    backend.startDiscovery();
    
    const handleLocalInfo = (device: Device) => {
      console.log('Received local device info:', device);
      setLocalDevice(device);
    };

    const handleDeviceFound = (device: Device) => {
      setDevices(prev => {
        if (prev.find(d => d.id === device.id)) return prev;
        return [...prev, device];
      });
    };

    const handleDisconnect = () => {
      setConnectionStatus(ConnectionState.DISCONNECTED);
      setAppMode(AppMode.IDLE);
      setConnectedDevice(null);
      setPendingDeviceId(null);
      if (document.pointerLockElement) {
        document.exitPointerLock();
      }
    };

    const handleIncomingRequest = (device: Device) => {
      // Only accept requests if we are idle
      if (connectionStatus === ConnectionState.DISCONNECTED) {
        setIncomingRequest(device);
      }
    };

    backend.on('local-info', handleLocalInfo);
    backend.on('device-found', handleDeviceFound);
    backend.on('disconnected', handleDisconnect);
    backend.on('connection-request', handleIncomingRequest);

    return () => {
      backend.off('local-info', handleLocalInfo);
      backend.off('device-found', handleDeviceFound);
      backend.off('disconnected', handleDisconnect);
      backend.off('connection-request', handleIncomingRequest);
    };
  }, [connectionStatus]);

  // --- Shortcut Listener ---
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Check for Ctrl + Alt + Q to Force Exit
      if (e.ctrlKey && e.altKey && e.key.toLowerCase() === 'q') {
        console.log("Exit shortcut triggered");
        disconnect();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  // --- Actions ---

  const connectToDevice = async (device: Device) => {
    console.log('[App] 开始连接到设备:', device);
    setPendingDeviceId(device.id);
    setConnectionStatus(ConnectionState.CONNECTING);
    
    // Simulate Backend Handshake
    // If we initiate connection, we typically want to CONTROL the other device (Host Mode)
    // But this can be negotiated. For this demo, assuming "Connect" -> "I Control You".
    try {
        console.log('[App] 调用 backend.requestConnection...');
        const success = await backend.requestConnection(device.id);
        console.log('[App] requestConnection 返回:', success);
        
        // Check if user cancelled while waiting
        if (connectionStatus === ConnectionState.DISCONNECTED) {
            console.log('[App] 用户已取消连接');
            return;
        }

        if (success) {
            console.log('[App] 连接成功，更新状态');
            setConnectionStatus(ConnectionState.CONNECTED);
            setConnectedDevice(device);
            setAppMode(AppMode.HOSTING); // We become the Host (Sender)
        } else {
            console.log('[App] 连接失败');
            handleConnectFail("对方拒绝了连接请求");
        }
    } catch (e) {
        console.error('[App] 连接异常:', e);
        handleConnectFail("连接超时");
    }
  };

  const cancelConnectRequest = () => {
      backend.cancelConnectionRequest();
      setConnectionStatus(ConnectionState.DISCONNECTED);
      setPendingDeviceId(null);
  };

  const handleConnectFail = (msg: string) => {
      setConnectionStatus(ConnectionState.DISCONNECTED);
      setPendingDeviceId(null);
      alert(msg);
  };

  const acceptIncomingConnection = () => {
    if (!incomingRequest) return;
    backend.acceptConnection(incomingRequest.id);
    setConnectedDevice(incomingRequest);
    setConnectionStatus(ConnectionState.CONNECTED);
    setIncomingRequest(null);
    // If they connected to us, they control us. We are Client (Receiver).
    setAppMode(AppMode.CLIENT); 
  };

  const denyIncomingConnection = () => {
    if (!incomingRequest) return;
    console.log('[App] 拒绝连接请求:', incomingRequest);
    backend.rejectConnection(incomingRequest.id);
    setIncomingRequest(null);
  };

  const startHosting = async () => {
    if (!connectedDevice) return;
    setAppMode(AppMode.HOSTING);
    backend.startCapture();
    setTimeout(() => {
        captureRef.current?.requestPointerLock();
    }, 100);
  };

  const disconnect = () => {
    backend.stopCapture();
    backend.disconnect();
    setConnectionStatus(ConnectionState.DISCONNECTED);
    setAppMode(AppMode.IDLE);
    setConnectedDevice(null);
    setPendingDeviceId(null);
  };

  // --- Input Capture Logic (Host Mode) ---
  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (appMode !== AppMode.HOSTING) return;
    
    const inputEvent: InputEvent = {
        type: 'mousemove',
        dx: e.movementX,
        dy: e.movementY,
        timestamp: Date.now()
    };
    backend.sendInputEvent(inputEvent);
  }, [appMode]);

  const handleKeyDownCapture = useCallback((e: React.KeyboardEvent) => {
     if (appMode !== AppMode.HOSTING) return;
     if (!(e.ctrlKey && e.altKey && e.key.toLowerCase() === 'q')) {
         e.preventDefault();
         const inputEvent: InputEvent = {
             type: 'keydown',
             key: e.key,
             timestamp: Date.now()
         };
         backend.sendInputEvent(inputEvent);
     }
  }, [appMode]);


  // --- Render Helpers ---

  const renderSidebar = () => (
    <div className="w-80 bg-gray-950 border-r border-gray-800 flex flex-col p-6 gap-6 z-20 shadow-2xl">
      {/* Brand */}
      <div className="flex items-center gap-3 mb-2">
        <div className="p-2.5 bg-indigo-600 rounded-xl shadow-lg shadow-indigo-500/20">
          <Cast className="text-white" size={24} />
        </div>
        <div>
          <h1 className="text-xl font-bold text-white tracking-tight">ShareFlow</h1>
          <div className="flex items-center gap-2">
            <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse"></span>
            <p className="text-xs text-gray-500 font-mono">LAN Active</p>
          </div>
        </div>
      </div>

      {/* Local Device Info */}
      <div className="p-4 bg-gray-900/50 rounded-xl border border-gray-800 hover:border-gray-700 transition-colors">
        <h3 className="text-[10px] font-bold text-gray-500 uppercase tracking-widest mb-3">本机信息</h3>
        <div className="flex items-center gap-3">
            <div className="bg-gray-800 p-2 rounded-lg text-gray-400">
                <Settings size={20} />
            </div>
            <div className="overflow-hidden">
                <p className="text-sm font-medium text-gray-200 truncate">{localDevice.name}</p>
                <p className="text-xs text-gray-500 font-mono">{localDevice.ip}</p>
            </div>
        </div>
      </div>

      {/* Connection Status */}
      <div className="space-y-3">
        <h3 className="text-[10px] font-bold text-gray-500 uppercase tracking-widest">当前状态</h3>
        <StatusBadge status={connectionStatus} />
        
        {connectionStatus === ConnectionState.CONNECTED && (
          <div className="mt-4 p-5 rounded-xl bg-gradient-to-br from-indigo-900/30 to-purple-900/30 border border-indigo-500/30 relative overflow-hidden group">
             <div className="absolute top-0 right-0 p-2 opacity-10 group-hover:opacity-20 transition-opacity">
                <MousePointer2 size={64} />
             </div>
             <p className="text-xs text-indigo-300 mb-1">已连接到设备</p>
             <p className="font-bold text-white text-lg leading-tight mb-4">{connectedDevice?.name}</p>
             
             <div className="bg-gray-900/60 rounded px-3 py-2 mb-4 border border-gray-700/50 backdrop-blur-sm">
                <p className="text-[10px] text-gray-400 mb-1">紧急断开快捷键</p>
                <div className="flex items-center gap-1.5">
                    {EXIT_SHORTCUT.split('+').map((k, i) => (
                        <React.Fragment key={i}>
                            <span className="bg-gray-700 text-gray-200 px-1.5 py-0.5 rounded text-xs font-mono border border-gray-600 shadow-sm">{k.trim()}</span>
                            {i < 2 && <span className="text-gray-600 text-[10px]">+</span>}
                        </React.Fragment>
                    ))}
                </div>
             </div>
             
             <Button variant="danger" className="w-full text-sm shadow-lg shadow-red-900/20" onClick={disconnect}>
                断开连接
             </Button>
          </div>
        )}
      </div>

      <div className="mt-auto pt-6 border-t border-gray-900 space-y-4">
        {/* Debug: Test Input Display */}
        {connectionStatus === ConnectionState.DISCONNECTED && (
          <button 
            onClick={() => {
              // Simulate connection for testing
              const testDevice: Device = {
                id: 'test-1',
                name: '测试设备',
                ip: '192.168.1.100',
                type: 0
              };
              setConnectedDevice(testDevice);
              setConnectionStatus(ConnectionState.CONNECTED);
              setAppMode(AppMode.HOSTING);
              backend.startCapture();
            }}
            className="w-full text-xs text-gray-600 hover:text-indigo-400 transition-colors text-center py-2 border border-dashed border-gray-800 hover:border-gray-700 rounded-lg"
          >
            [测试] 模拟主控模式
          </button>
        )}
        <div className="flex items-center justify-center gap-2 text-[10px] text-gray-600">
            <Shield size={10} />
            <span>TLS 1.3 端到端加密</span>
        </div>
      </div>
    </div>
  );

  const refreshDevices = () => {
    setDevices([]);
    backend.startDiscovery();
  };

  const renderIdleView = () => (
    <div className="flex-1 flex flex-col p-8 relative overflow-y-auto bg-gray-950">
      <div className="max-w-5xl w-full mx-auto z-0">
        <div className="mb-10 flex justify-between items-end">
            <div>
                <h2 className="text-3xl font-bold text-white mb-2 tracking-tight">附近的设备</h2>
                <p className="text-gray-400">选择局域网内的设备进行连接，支持跨平台共享。</p>
            </div>
            <div className="flex items-center gap-3">
                {devices.length > 0 && (
                    <span className="px-3 py-1 bg-gray-800 rounded-full text-xs text-gray-400 border border-gray-700">
                        发现 {devices.length} 个设备
                    </span>
                )}
                <Button 
                    variant="secondary" 
                    size="sm"
                    onClick={refreshDevices}
                    className="flex items-center gap-2"
                >
                    <Cast size={16} />
                    搜索设备
                </Button>
            </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            {devices.map(device => (
                <DeviceCard 
                    key={device.id} 
                    device={device} 
                    onConnect={connectToDevice}
                    onCancel={cancelConnectRequest}
                    connectionStatus={connectionStatus}
                    isConnectingToThis={pendingDeviceId === device.id || connectedDevice?.id === device.id}
                />
            ))}
            
            {/* Show message when no devices found */}
            {devices.length === 0 && (
                <div className="col-span-full flex flex-col items-center justify-center py-16 text-center">
                    <div className="w-20 h-20 bg-gray-800/30 rounded-full flex items-center justify-center mb-4">
                        <Cast className="text-gray-600" size={40} />
                    </div>
                    <p className="text-gray-400 text-lg mb-2">未发现设备</p>
                    <p className="text-gray-600 text-sm">确保其他设备在同一局域网并运行 ShareFlow</p>
                </div>
            )}
        </div>

        {connectionStatus === ConnectionState.CONNECTING && (
            <div className="fixed bottom-8 right-8 bg-gray-900 border border-gray-700 p-4 rounded-xl shadow-2xl flex items-center gap-3 animate-in slide-in-from-bottom-5">
                <Loader2 className="animate-spin text-indigo-500" />
                <div>
                    <p className="text-sm font-medium text-white">正在请求连接...</p>
                    <p className="text-xs text-gray-500">等待对方确认</p>
                </div>
                <Button variant="ghost" size="sm" onClick={cancelConnectRequest} className="ml-2 text-red-400 hover:text-red-300">取消</Button>
            </div>
        )}
      </div>

      {/* Incoming Connection Modal */}
      {incomingRequest && (
        <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm p-4">
            <div className="bg-gray-900 border border-gray-700 p-1 rounded-2xl shadow-2xl max-w-sm w-full transform transition-all scale-100 animate-in fade-in zoom-in duration-300">
                <div className="bg-gray-800/50 rounded-xl p-6 border border-gray-700/50">
                    <div className="flex flex-col items-center mb-6 text-center">
                        <div className="w-16 h-16 bg-indigo-500/20 rounded-full flex items-center justify-center mb-4 relative">
                            <AlertCircle className="text-indigo-400" size={32} />
                            <span className="absolute w-full h-full rounded-full border border-indigo-500/50 animate-ping opacity-20"></span>
                        </div>
                        <h3 className="text-xl font-bold text-white mb-1">收到连接请求</h3>
                        <p className="text-sm text-gray-400">对方请求与此设备共享键鼠</p>
                    </div>
                    
                    <div className="bg-black/40 rounded-lg p-4 mb-6 border border-gray-700/50 space-y-3">
                        <div className="flex justify-between items-center">
                            <span className="text-gray-500 text-xs uppercase font-bold tracking-wider">设备名称</span>
                            <span className="text-white font-medium">{incomingRequest.name}</span>
                        </div>
                        <div className="h-px bg-gray-700/50"></div>
                        <div className="flex justify-between items-center">
                            <span className="text-gray-500 text-xs uppercase font-bold tracking-wider">IP 地址</span>
                            <span className="text-gray-300 font-mono text-sm">{incomingRequest.ip}</span>
                        </div>
                    </div>

                    <div className="grid grid-cols-2 gap-3">
                        <Button variant="secondary" onClick={denyIncomingConnection} className="w-full justify-center">
                            <X size={18} /> 拒绝
                        </Button>
                        <Button variant="primary" onClick={acceptIncomingConnection} className="w-full justify-center">
                            <Check size={18} /> 接受
                        </Button>
                    </div>
                </div>
            </div>
        </div>
      )}
    </div>
  );

  const renderActiveView = () => (
    <div className="flex-1 flex flex-col h-screen overflow-hidden bg-gray-950 relative">
        {/* Connection Overlay Info */}
        <div className="absolute top-6 left-1/2 -translate-x-1/2 z-30 flex items-center gap-2 bg-gray-900/80 backdrop-blur border border-gray-700 px-4 py-2 rounded-full shadow-xl">
             <span className={`w-2 h-2 rounded-full ${appMode === AppMode.HOSTING ? 'bg-indigo-500' : 'bg-emerald-500'} animate-pulse`}></span>
             <span className="text-sm font-medium text-gray-200">
                {appMode === AppMode.HOSTING ? '主控模式 (HOST)' : '被控模式 (CLIENT)'}
             </span>
        </div>

        {appMode === AppMode.CLIENT ? (
             <InputVisualizer connectedDeviceName={connectedDevice?.name || '未知设备'} />
        ) : (
            <div 
                ref={captureRef}
                className="flex-1 m-4 rounded-2xl border-2 border-dashed border-indigo-500/20 bg-gray-900/20 flex flex-col items-center justify-center cursor-none outline-none focus:border-indigo-500/60 focus:bg-gray-900/40 transition-all duration-300 relative group overflow-hidden"
                onMouseMove={handleMouseMove}
                onKeyDown={handleKeyDownCapture}
                onClick={() => captureRef.current?.requestPointerLock()}
                tabIndex={0}
            >
                {/* Background Grid */}
                <div className="absolute inset-0 opacity-5" 
                    style={{ backgroundImage: 'linear-gradient(#4f46e5 1px, transparent 1px), linear-gradient(90deg, #4f46e5 1px, transparent 1px)', backgroundSize: '40px 40px' }}>
                </div>

                <div className="text-center pointer-events-none z-10 p-8 rounded-2xl bg-gray-950/50 backdrop-blur border border-gray-800 group-hover:border-indigo-500/30 transition-all">
                    <div className="w-24 h-24 bg-gradient-to-br from-indigo-500/20 to-purple-500/20 rounded-full flex items-center justify-center mx-auto mb-6 relative">
                        <MousePointer2 size={40} className="text-indigo-400 relative z-10" />
                        <span className="absolute inset-0 rounded-full border border-indigo-500/30 animate-ping opacity-20"></span>
                    </div>
                    <h3 className="text-3xl font-bold text-white mb-3">输入已捕获</h3>
                    <p className="text-gray-400 max-w-md mx-auto mb-8 leading-relaxed">
                        您的键盘和鼠标操作正在实时发送给<br/>
                        <span className="text-indigo-400 font-semibold text-lg">{connectedDevice?.name}</span>
                    </p>
                    
                    <div className="flex justify-center gap-4 text-xs font-mono text-gray-500">
                        <div className="flex items-center gap-2 bg-gray-900 px-3 py-1.5 rounded border border-gray-800">
                            <span className="text-gray-300">ESC</span> 
                            <span>释放鼠标</span>
                        </div>
                        <div className="flex items-center gap-2 bg-gray-900 px-3 py-1.5 rounded border border-gray-800">
                            <span className="text-gray-300">{EXIT_SHORTCUT}</span> 
                            <span>断开连接</span>
                        </div>
                    </div>
                </div>
                
                {/* Click hint overlay if not focused */}
                <div className="absolute inset-0 flex items-center justify-center bg-black/60 opacity-0 group-focus:opacity-0 hover:opacity-100 transition-opacity backdrop-blur-[2px]">
                    <div className="text-white font-medium flex items-center gap-2 bg-gray-900 px-6 py-3 rounded-full shadow-xl border border-gray-700 transform scale-95 group-hover:scale-100 transition-transform">
                        <MousePointer2 size={16} /> 点击屏幕开始控制
                    </div>
                </div>

                {/* Local Input Display */}
                <LocalInputDisplay connectedDeviceName={connectedDevice?.name || '未知设备'} />
            </div>
        )}
    </div>
  );

  return (
    <div className="flex h-screen bg-gray-950 text-gray-100 font-sans selection:bg-indigo-500/30 overflow-hidden">
      {renderSidebar()}
      {connectionStatus === ConnectionState.CONNECTED ? renderActiveView() : renderIdleView()}
    </div>
  );
};

export default App;