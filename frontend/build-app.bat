@echo off
echo ========================================
echo ShareFlow 应用打包脚本
echo ========================================
echo.

echo [1/4] 检查 Rust 环境...
where cargo >nul 2>&1
if %errorlevel% neq 0 (
    echo [错误] 未找到 Cargo，请先安装 Rust
    echo 访问: https://rustup.rs/
    pause
    exit /b 1
)
echo [成功] Rust 环境就绪
echo.

echo [2/4] 编译 Rust 服务 (Release 模式)...
cd rust-service
cargo build --release
if %errorlevel% neq 0 (
    echo [错误] Rust 编译失败
    cd ..
    pause
    exit /b 1
)
cd ..
echo [成功] Rust 服务编译完成
echo.

echo [3/4] 构建前端...
call npm run build
if %errorlevel% neq 0 (
    echo [错误] 前端构建失败
    pause
    exit /b 1
)
echo [成功] 前端构建完成
echo.

echo [4/4] 打包 Electron 应用...
call npx electron-builder
if %errorlevel% neq 0 (
    echo [错误] Electron 打包失败
    pause
    exit /b 1
)
echo [成功] 应用打包完成
echo.

echo ========================================
echo 打包完成！
echo 输出目录: release/
echo ========================================
echo.

explorer release

pause
