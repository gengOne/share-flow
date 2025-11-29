@echo off
echo ========================================
echo ShareFlow Backend - 调试模式
echo ========================================
echo.
echo 本机 IP 地址:
ipconfig | findstr "IPv4"
echo.
echo 按任意键启动服务...
pause >nul
echo.

cargo run --release

pause
