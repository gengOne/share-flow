# 🚀 快速启动指南

## 前置要求

- Node.js (v16+)
- Rust (最新稳定版)
- Windows 系统

## 第一次运行

### 1️⃣ 安装依赖

```bash
# 安装前端依赖
npm install

# 编译 Rust 服务
npm run rust:build
```

### 2️⃣ 启动服务

**方式 A: 分别启动（推荐用于开发调试）**

终端 1 - 启动 Rust 服务:
```bash
npm run rust:run
```

你应该看到:
```
Starting ShareFlow Service
  UDP Discovery: port 8080
  WebSocket API: ws://127.0.0.1:4000
Service is running. Waiting for events...
```

终端 2 - 启动前端:
```bash
npm run dev
```

浏览器访问: http://localhost:3000

**方式 B: 使用 Electron**

终端 1 - 启动 Rust 服务:
```bash
npm run rust:run
```

终端 2 - 启动 Electron:
```bash
npm run electron:dev
```

### 3️⃣ 测试连接

打开测试页面验证 WebSocket 连接:
```bash
npm run test:connection
```

或直接在浏览器打开 `test-connection.html`

## 验证清单

✅ Rust 服务显示 "WebSocket API: ws://127.0.0.1:4000"
✅ 前端控制台显示 "[RealBackend] Connected to Rust service"
✅ 测试页面显示 "状态: 已连接"

## 常见问题

### Q: WebSocket 连接失败
**A:** 确保 Rust 服务正在运行，检查端口 4000 是否被占用

### Q: 找不到设备
**A:** 
- 确认 UDP 端口 8080 未被占用
- 检查防火墙设置
- 确保设备在同一局域网

### Q: Rust 编译失败
**A:** 
```bash
# 更新 Rust
rustup update

# 清理并重新编译
cd rust-service
cargo clean
cargo build
```

### Q: 想使用模拟数据测试 UI
**A:** 编辑 `services/backend.ts`:
```typescript
// 改为使用 mockBackend
export { backend } from './mockBackend';
```

## 开发模式

### 切换后端

编辑 `services/backend.ts`:

```typescript
// 模拟后端（无需 Rust 服务）
export { backend } from './mockBackend';

// 真实后端（需要 Rust 服务）
export { backend } from './realBackend';
```

### 查看日志

**Rust 服务日志:**
- 直接在运行 `npm run rust:run` 的终端查看

**前端日志:**
- 浏览器控制台 (F12)
- 搜索 `[RealBackend]` 或 `[Backend]`

### 调试 WebSocket

1. 打开 `test-connection.html`
2. 点击"连接"按钮
3. 点击"开始发现设备"
4. 查看日志输出

## 生产构建

```bash
# 编译 Rust Release 版本
npm run rust:build-release

# 构建前端
npm run build

# 打包 Electron 应用
npm run electron:build
```

## 项目结构

```
├── rust-service/          # Rust 后端服务
│   ├── src/
│   │   ├── main.rs       # 主入口
│   │   ├── websocket.rs  # WebSocket 服务器
│   │   ├── discovery.rs  # UDP 设备发现
│   │   ├── protocol.rs   # 协议定义
│   │   └── transport.rs  # 网络传输
│   └── Cargo.toml
│
├── services/              # 前端服务层
│   ├── backend.ts        # 后端配置（切换这里）
│   ├── realBackend.ts    # 真实 WebSocket 客户端
│   └── mockBackend.ts    # 模拟数据
│
├── App.tsx               # 主应用
└── test-connection.html  # WebSocket 测试工具
```

## 下一步

- 📖 阅读 [WEBSOCKET.md](./WEBSOCKET.md) 了解详细架构
- 🔧 查看 [rust-service/src/websocket.rs](../rust-service/src/websocket.rs) 了解消息协议
- 🎨 修改 [App.tsx](../App.tsx) 自定义 UI

## 需要帮助？

- 检查 Rust 服务是否正常运行
- 查看浏览器控制台错误信息
- 使用 `test-connection.html` 测试基础连接
- 确认防火墙和网络设置
