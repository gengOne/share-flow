# WebSocket 连接说明

## 架构

```
Frontend (React) <--WebSocket--> Rust Service
     ↓                              ↓
  Port 3000                    Port 4000 (WS)
                                Port 8080 (UDP/TCP)
```

## 启动步骤

### 1. 编译并启动 Rust 服务

```bash
cd rust-service
cargo build
cargo run
```

你应该看到：
```
Starting ShareFlow Service
  UDP Discovery: port 8080
  WebSocket API: ws://127.0.0.1:4000
Service is running. Waiting for events...
```

### 2. 启动前端

在另一个终端：

```bash
npm run dev
```

或使用 Electron：

```bash
npm run electron:dev
```

## 验证连接

1. 打开浏览器控制台 (F12)
2. 查看是否有连接成功的日志：
   ```
   [RealBackend] Connecting to Rust service at ws://127.0.0.1:4000
   [RealBackend] Connected to Rust service
   ```

3. 如果看到连接错误，检查：
   - Rust 服务是否正在运行
   - 端口 4000 是否被占用
   - 防火墙设置

## 切换后端模式

编辑 `services/backend.ts`：

```typescript
// 使用模拟数据（无需 Rust 服务）
export { backend } from './mockBackend';

// 使用真实 Rust 服务
export { backend } from './realBackend';
```

## WebSocket 消息协议

### 前端 → Rust

- `startDiscovery` - 开始设备发现
- `requestConnection` - 请求连接设备
- `cancelConnection` - 取消连接请求
- `acceptConnection` - 接受连接请求
- `disconnect` - 断开连接
- `sendInput` - 发送输入事件

### Rust → 前端

- `deviceFound` - 发现新设备
- `connectionRequest` - 收到连接请求
- `connectionEstablished` - 连接建立
- `connectionFailed` - 连接失败
- `disconnected` - 已断开
- `remoteInput` - 远程输入事件

## 故障排查

### WebSocket 连接失败

1. 确认 Rust 服务正在运行
2. 检查端口占用：`netstat -ano | findstr :4000`
3. 查看 Rust 服务日志

### 设备发现不工作

1. 确认 UDP 端口 8080 未被占用
2. 检查防火墙是否允许 UDP 广播
3. 确认设备在同一局域网

### 输入事件不响应

1. 检查连接状态是否为 CONNECTED
2. 查看浏览器控制台是否有错误
3. 检查 Rust 服务是否收到消息
