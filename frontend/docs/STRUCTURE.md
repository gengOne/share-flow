# 📁 文档结构说明

## 目录组织

所有项目文档现已集中在 `docs/` 目录下，便于管理和查找。

```
shareflow/
├── docs/                           # 📚 文档中心
│   ├── README.md                   # 文档索引（从这里开始）
│   ├── QUICKSTART.md              # 快速启动指南
│   ├── WEBSOCKET.md               # WebSocket 架构文档
│   ├── VERIFICATION.md            # 验证清单
│   ├── INTEGRATION_COMPLETE.md    # 集成完成报告
│   ├── INTEGRATION_SUMMARY.md     # 集成总结
│   └── STRUCTURE.md               # 本文件 - 文档结构说明
│
├── README.md                       # 项目主页
├── test-connection.html           # WebSocket 测试工具
├── start-test.bat                 # 一键测试脚本
│
├── rust-service/                  # Rust 后端服务
│   ├── src/
│   │   ├── main.rs
│   │   ├── websocket.rs
│   │   ├── discovery.rs
│   │   ├── protocol.rs
│   │   └── transport.rs
│   └── Cargo.toml
│
├── services/                      # 前端服务层
│   ├── backend.ts
│   ├── realBackend.ts
│   └── mockBackend.ts
│
├── components/                    # React 组件
├── electron/                      # Electron 主进程
└── ...
```

## 文档分类

### 📖 用户文档

面向所有用户的文档：

| 文档 | 说明 | 目标读者 |
|------|------|----------|
| [README.md](../README.md) | 项目概览和快速开始 | 所有人 |
| [QUICKSTART.md](./QUICKSTART.md) | 详细的安装和启动指南 | 新用户 |
| [VERIFICATION.md](./VERIFICATION.md) | 测试和故障排查 | 用户、测试人员 |

### 🔧 开发文档

面向开发者的技术文档：

| 文档 | 说明 | 目标读者 |
|------|------|----------|
| [WEBSOCKET.md](./WEBSOCKET.md) | WebSocket 架构和协议 | 开发者 |
| [INTEGRATION_COMPLETE.md](./INTEGRATION_COMPLETE.md) | 集成实现细节 | 开发者 |
| [INTEGRATION_SUMMARY.md](./INTEGRATION_SUMMARY.md) | 完整技术总结 | 开发者、架构师 |

### 🛠️ 工具文档

辅助工具和脚本：

| 文件 | 说明 | 用途 |
|------|------|------|
| [test-connection.html](../test-connection.html) | WebSocket 测试页面 | 测试连接 |
| [start-test.bat](../start-test.bat) | 一键测试脚本 | 快速测试 |

## 文档导航

### 按使用场景

**场景 1: 第一次使用**
```
README.md → QUICKSTART.md → test-connection.html
```

**场景 2: 遇到问题**
```
VERIFICATION.md → QUICKSTART.md (常见问题) → WEBSOCKET.md (架构)
```

**场景 3: 参与开发**
```
INTEGRATION_SUMMARY.md → WEBSOCKET.md → INTEGRATION_COMPLETE.md
```

**场景 4: 了解架构**
```
README.md → WEBSOCKET.md → INTEGRATION_SUMMARY.md
```

### 按阅读顺序

**新用户推荐阅读顺序:**
1. [README.md](../README.md) - 了解项目
2. [QUICKSTART.md](./QUICKSTART.md) - 快速开始
3. [VERIFICATION.md](./VERIFICATION.md) - 验证安装

**开发者推荐阅读顺序:**
1. [README.md](../README.md) - 项目概览
2. [INTEGRATION_SUMMARY.md](./INTEGRATION_SUMMARY.md) - 技术总结
3. [WEBSOCKET.md](./WEBSOCKET.md) - 架构设计
4. [INTEGRATION_COMPLETE.md](./INTEGRATION_COMPLETE.md) - 实现细节

## 文档维护

### 更新原则

1. **保持同步** - 代码更新时同步更新文档
2. **清晰简洁** - 使用简单明了的语言
3. **示例丰富** - 提供充足的代码示例
4. **及时更新** - 标注最后更新时间

### 文档模板

每个文档应包含：

```markdown
# 标题

简短描述（1-2 句话）

## 目录（可选）

## 主要内容

### 小节 1
内容...

### 小节 2
内容...

## 示例（如适用）

## 常见问题（如适用）

## 相关链接

---

**最后更新:** YYYY-MM-DD
**版本:** vX.X.X
```

### 交叉引用规范

**相对路径规则:**

从 `docs/` 内部引用：
```markdown
[其他文档](./OTHER.md)
[项目根文件](../README.md)
[代码文件](../rust-service/src/main.rs)
```

从项目根引用：
```markdown
[文档](./docs/QUICKSTART.md)
[代码](./services/backend.ts)
```

## 文档贡献

### 添加新文档

1. 在 `docs/` 目录创建新文件
2. 使用清晰的文件名（大写，下划线分隔）
3. 在 `docs/README.md` 添加索引
4. 更新本文件的文档列表

### 修改现有文档

1. 保持文档结构一致
2. 更新"最后更新"时间
3. 检查所有交叉引用
4. 测试所有代码示例

### 文档审查清单

- [ ] 标题清晰明确
- [ ] 内容准确无误
- [ ] 代码示例可运行
- [ ] 链接全部有效
- [ ] 格式统一规范
- [ ] 更新时间正确

## 文档工具

### Markdown 编辑器推荐

- **VS Code** - 内置 Markdown 预览
- **Typora** - 所见即所得编辑器
- **MarkText** - 开源 Markdown 编辑器

### 在线预览

GitHub 会自动渲染 Markdown 文件，可以直接在仓库中预览。

### 本地预览

```bash
# 使用 VS Code
code docs/README.md

# 使用浏览器（需要插件）
start docs/README.md
```

## 文档统计

| 文档 | 行数 | 字数 | 阅读时间 |
|------|------|------|----------|
| README.md (主) | ~200 | ~1500 | 5 分钟 |
| QUICKSTART.md | ~150 | ~1000 | 5 分钟 |
| WEBSOCKET.md | ~120 | ~800 | 10 分钟 |
| VERIFICATION.md | ~250 | ~1800 | 15 分钟 |
| INTEGRATION_COMPLETE.md | ~300 | ~2200 | 20 分钟 |
| INTEGRATION_SUMMARY.md | ~400 | ~3000 | 25 分钟 |
| **总计** | **~1420** | **~10300** | **80 分钟** |

## 快速查找

### 关键词索引

- **安装** → QUICKSTART.md
- **启动** → QUICKSTART.md, README.md
- **测试** → VERIFICATION.md, test-connection.html
- **架构** → WEBSOCKET.md, INTEGRATION_SUMMARY.md
- **协议** → WEBSOCKET.md
- **故障排查** → VERIFICATION.md, QUICKSTART.md
- **开发** → INTEGRATION_COMPLETE.md, INTEGRATION_SUMMARY.md
- **API** → WEBSOCKET.md, README.md

### 常见问题快速链接

- [如何启动？](./QUICKSTART.md#启动步骤)
- [连接失败？](./VERIFICATION.md#websocket-连接失败)
- [找不到设备？](./VERIFICATION.md#找不到设备)
- [如何切换后端？](./QUICKSTART.md#切换后端模式)
- [如何调试？](./INTEGRATION_SUMMARY.md#调试技巧)

## 反馈和改进

如果你对文档有任何建议：

1. 📝 提交 Issue 描述问题
2. 💡 在讨论区分享想法
3. 🔧 提交 PR 改进文档

---

**文档结构版本:** v1.0  
**最后更新:** 2024-11-29  
**维护者:** ShareFlow Team
