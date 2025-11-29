# ShareFlow - 局域网键鼠共享

一个基于 Electron + React + Rust 的跨平台局域网键鼠共享工具。

## ✨ 特性

- 🖱️ 无缝键鼠共享 - 在多台设备间自由切换
- 🔍 自动设备发现 - UDP 广播自动发现局域网设备
- 🚀 低延迟传输 - 基于 TCP 的高性能数据传输
- 🔒 安全可靠 - 端到端加密通信
- 🎨 现代化 UI - 基于 React 的精美界面
- ⚡ 高性能后端 - Rust 实现的系统级服务

## 🏗️ 架构

```
┌─────────────────┐         WebSocket          ┌──────────────────┐
│                 │ ←─────────────────────────→ │                  │
│  React Frontend │    ws://127.0.0.1:4000     │   Rust Service   │
│   (Electron)    │                             │                  │
└─────────────────┘                             └──────────────────┘
                                                         ↓
                                                    UDP:8080 (发现)
                                                    TCP:8080 (数据)
```

## 🚀 快速开始

### 前置要求

- Node.js (v16+)
- Rust (最新稳定版)
- Windows 系统

### 安装

```bash
# 1. 克隆项目
git clone <repository-url>
cd shareflow

# 2. 安装前端依赖
npm install

# 3. 编译 Rust 服务
npm run rust:build
```

### 运行

**方式 1: 快速测试（推荐）**
```bash
start-test.bat
```

**方式 2: 手动启动**

终端 1 - Rust 服务:
```bash
npm run rust:run
```

终端 2 - 前端:
```bash
npm run dev
```

浏览器访问: http://localhost:3000

**方式 3: Electron 应用**
```bash
# 终端 1
npm run rust:run

# 终端 2
npm run electron:dev
```

## 📚 文档

**→ [文档中心](./docs/README.md)** - 查看所有文档

快速链接：
- [快速启动指南](./docs/QUICKSTART.md) - 详细的启动步骤
- [WebSocket 架构](./docs/WEBSOCKET.md) - 通信协议说明
- [验证清单](./docs/VERIFICATION.md) - 测试和故障排查
- [集成总结](./docs/INTEGRATION_SUMMARY.md) - 完整的技术总结

## 🧪 测试

### WebSocket 连接测试

```bash
npm run test:connection
```

或直接打开 `test-connection.html`

### 验证清单

- [ ] Rust 服务正常启动
- [ ] WebSocket 连接成功
- [ ] 前端应用无错误
- [ ] 设备发现功能正常

详见 [VERIFICATION.md](./docs/VERIFICATION.md)

## 🛠️ 开发

### 项目结构

```
shareflow/
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
│   ├── backend.ts        # 后端配置
│   ├── realBackend.ts    # WebSocket 客户端
│   └── mockBackend.ts    # 模拟数据
│
├── components/            # React 组件
├── electron/              # Electron 主进程
├── App.tsx               # 主应用
└── test-connection.html  # 测试工具
```

### 切换后端模式

编辑 `services/backend.ts`:

```typescript
// 使用真实 Rust 服务
export { backend } from './realBackend';

// 使用模拟数据（UI 开发）
export { backend } from './mockBackend';
```

### 可用脚本

```bash
npm run dev                 # 启动前端开发服务器
npm run build              # 构建前端
npm run rust:build         # 编译 Rust 服务 (Debug)
npm run rust:run           # 运行 Rust 服务
npm run rust:build-release # 编译 Rust 服务 (Release)
npm run electron:dev       # 启动 Electron 应用
npm run electron:build     # 打包 Electron 应用
npm run test:connection    # 打开 WebSocket 测试页面
```

## 🔌 WebSocket API

### 前端 → Rust

- `startDiscovery` - 开始设备发现
- `requestConnection` - 请求连接设备
- `cancelConnection` - 取消连接
- `acceptConnection` - 接受连接
- `disconnect` - 断开连接
- `sendInput` - 发送输入事件

### Rust → 前端

- `deviceFound` - 发现新设备
- `connectionRequest` - 收到连接请求
- `connectionEstablished` - 连接成功
- `connectionFailed` - 连接失败
- `disconnected` - 已断开
- `remoteInput` - 远程输入事件

详见 [WEBSOCKET.md](./docs/WEBSOCKET.md)

## 🐛 故障排查

### WebSocket 连接失败

1. 确认 Rust 服务正在运行
2. 检查端口占用: `netstat -ano | findstr :4000`
3. 检查防火墙设置

### 找不到设备

1. 确认设备在同一局域网
2. 检查 UDP 端口 8080
3. 允许防火墙规则

详见 [VERIFICATION.md](./docs/VERIFICATION.md)

## 📦 打包应用

### 快速打包

```bash
# 方式 1: 使用脚本（推荐）
build-app.bat

# 方式 2: 使用 npm 命令
npm run electron:build
```

### 打包输出

打包完成后，文件位于 `release/` 目录：
- `ShareFlow Setup.exe` - 安装程序（推荐分发）
- `ShareFlow.exe` - 便携版（无需安装）

详细打包指南请查看 [BUILD.md](./docs/BUILD.md)

## 📝 开发状态

### ✅ 已完成

- [x] 基础 UI 框架
- [x] Rust 服务架构
- [x] WebSocket 通信
- [x] UDP 设备发现
- [x] 连接管理流程

### 🚧 进行中

- [ ] TCP 连接实现
- [ ] 输入捕获 (rdev)
- [ ] 输入注入
- [ ] 加密通信

### 📋 计划中

- [ ] 跨平台支持 (macOS, Linux)
- [ ] 剪贴板共享
- [ ] 文件传输
- [ ] 多显示器支持

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 许可

MIT License

---

**最后更新:** 2024-11-29  
**状态:** ✅ WebSocket 集成完成
