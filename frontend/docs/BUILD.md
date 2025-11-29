# 📦 ShareFlow 打包指南

## 前置要求

### 必需工具

1. **Node.js** (v16+)
   - 下载: https://nodejs.org/

2. **Rust** (最新稳定版)
   - 下载: https://rustup.rs/
   - 安装后运行: `rustup update`

3. **构建工具**
   - Windows: Visual Studio Build Tools 或 Visual Studio
   - 需要 C++ 构建工具

### 验证环境

```bash
# 检查 Node.js
node --version

# 检查 npm
npm --version

# 检查 Rust
cargo --version

# 检查 rustc
rustc --version
```

## 快速打包

### 方式 1: 使用脚本（推荐）

```bash
build-app.bat
```

这会自动完成所有步骤并打开输出目录。

### 方式 2: 手动打包

```bash
# 1. 安装依赖
npm install

# 2. 编译 Rust 服务 (Release)
npm run rust:build-release

# 3. 构建前端
npm run build

# 4. 打包 Electron
npx electron-builder
```

### 方式 3: 仅打包不安装（测试用）

```bash
npm run electron:pack
```

这会创建未打包的应用目录，可以直接运行测试。

## 打包输出

打包完成后，文件位于 `release/` 目录：

```
release/
├── ShareFlow Setup 0.0.0.exe    # NSIS 安装程序
├── ShareFlow 0.0.0.exe          # 便携版（无需安装）
└── win-unpacked/                # 未打包的应用目录
    ├── ShareFlow.exe
    ├── resources/
    │   └── rust-service.exe     # Rust 服务
    └── ...
```

### 文件说明

| 文件 | 说明 | 大小 |
|------|------|------|
| `ShareFlow Setup.exe` | 安装程序，推荐分发 | ~100MB |
| `ShareFlow.exe` | 便携版，解压即用 | ~100MB |
| `win-unpacked/` | 开发测试用 | ~150MB |

## 打包配置

### 修改应用信息

编辑 `package.json`:

```json
{
  "name": "shareflow",
  "version": "1.0.0",
  "build": {
    "appId": "com.shareflow.app",
    "productName": "ShareFlow",
    ...
  }
}
```

### 添加应用图标

1. 准备图标文件:
   - Windows: `icon.ico` (256x256)
   - macOS: `icon.icns`
   - Linux: `icon.png` (512x512)

2. 放置在项目根目录

3. 更新 `package.json`:
   ```json
   "win": {
     "icon": "icon.ico"
   }
   ```

### 自定义安装程序

编辑 `package.json` 中的 `nsis` 配置:

```json
"nsis": {
  "oneClick": false,              // 允许自定义安装路径
  "allowToChangeInstallationDirectory": true,
  "createDesktopShortcut": true,  // 创建桌面快捷方式
  "createStartMenuShortcut": true,
  "installerIcon": "icon.ico",
  "uninstallerIcon": "icon.ico",
  "license": "LICENSE.txt"        // 许可协议
}
```

## 高级配置

### 多平台打包

```bash
# Windows
npm run electron:build -- --win

# macOS (需要在 macOS 上运行)
npm run electron:build -- --mac

# Linux
npm run electron:build -- --linux
```

### 指定架构

```bash
# 64位
npm run electron:build -- --x64

# 32位
npm run electron:build -- --ia32

# ARM64
npm run electron:build -- --arm64
```

### 代码签名（可选）

Windows 代码签名需要证书：

```json
"win": {
  "certificateFile": "cert.pfx",
  "certificatePassword": "password",
  "signingHashAlgorithms": ["sha256"]
}
```

## 优化打包大小

### 1. 排除不必要的文件

编辑 `package.json`:

```json
"files": [
  "dist/**/*",
  "electron/**/*",
  "!**/*.map",
  "!**/*.md"
]
```

### 2. 压缩 Rust 二进制

```bash
# 使用 UPX 压缩
upx --best rust-service/target/release/rust-service.exe
```

### 3. 启用 ASAR 打包

```json
"asar": true
```

## 故障排查

### 问题 1: Rust 编译失败

**症状:**
```
error: linker `link.exe` not found
```

**解决方案:**
安装 Visual Studio Build Tools:
```bash
# 下载并安装
https://visualstudio.microsoft.com/downloads/
# 选择 "C++ 构建工具"
```

### 问题 2: Electron 打包失败

**症状:**
```
Error: Cannot find module 'electron-builder'
```

**解决方案:**
```bash
npm install --save-dev electron-builder
```

### 问题 3: Rust 服务未打包

**症状:**
打包后的应用无法启动 Rust 服务

**解决方案:**
1. 确认 Rust 服务已编译:
   ```bash
   dir rust-service\target\release\rust-service.exe
   ```

2. 检查 `package.json` 中的 `extraResources` 配置

3. 手动复制测试:
   ```bash
   copy rust-service\target\release\rust-service.exe release\win-unpacked\resources\
   ```

### 问题 4: 打包后无法运行

**症状:**
双击 exe 无反应或闪退

**解决方案:**
1. 以管理员身份运行
2. 检查 Windows Defender 是否拦截
3. 查看日志:
   ```
   %APPDATA%\ShareFlow\logs\
   ```

## 分发应用

### 1. 创建安装包

推荐分发 `ShareFlow Setup.exe`：
- 包含自动更新功能
- 用户体验更好
- 可以创建卸载程序

### 2. 便携版

分发 `ShareFlow.exe`：
- 无需安装
- 适合企业部署
- 可以放在 U 盘运行

### 3. 自动更新

配置自动更新服务器:

```json
"publish": {
  "provider": "github",
  "owner": "your-username",
  "repo": "shareflow"
}
```

## 测试清单

打包后测试：

- [ ] 应用可以正常启动
- [ ] Rust 服务自动启动
- [ ] WebSocket 连接正常
- [ ] 设备发现功能正常
- [ ] 输入捕获功能正常
- [ ] 快捷键退出正常
- [ ] 应用可以正常关闭
- [ ] 卸载程序正常工作

## 持续集成

### GitHub Actions 示例

创建 `.github/workflows/build.yml`:

```yaml
name: Build

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - run: npm install
      - run: npm run electron:build
      
      - uses: actions/upload-artifact@v3
        with:
          name: ShareFlow-Windows
          path: release/*.exe
```

## 相关资源

- [Electron Builder 文档](https://www.electron.build/)
- [Rust 交叉编译](https://rust-lang.github.io/rustup/cross-compilation.html)
- [代码签名指南](https://www.electron.build/code-signing)

---

**最后更新:** 2024-11-29  
**版本:** v1.0
