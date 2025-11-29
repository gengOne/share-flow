@echo off
echo ========================================
echo ShareFlow WebSocket 连接测试
echo ========================================
echo.

echo [1/3] 检查 Rust 服务...
cd rust-service
cargo check >nul 2>&1
if %errorlevel% neq 0 (
    echo [错误] Rust 编译失败，正在尝试编译...
    cargo build
    if %errorlevel% neq 0 (
        echo [失败] 无法编译 Rust 服务
        pause
        exit /b 1
    )
)
echo [成功] Rust 服务就绪
echo.

echo [2/3] 启动 Rust 服务...
start "ShareFlow Rust Service" cmd /k "cd rust-service && cargo run"
timeout /t 3 /nobreak >nul
echo [成功] Rust 服务已启动
echo.

echo [3/3] 打开测试页面...
cd ..
start test-connection.html
echo [成功] 测试页面已打开
echo.

echo ========================================
echo 测试步骤:
echo 1. 在测试页面点击 "连接" 按钮
echo 2. 查看状态是否显示 "已连接"
echo 3. 点击 "开始发现设备" 测试消息收发
echo ========================================
echo.
echo 按任意键关闭此窗口...
pause >nul
