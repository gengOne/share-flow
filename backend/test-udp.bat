@echo off
echo 测试 UDP 连通性
echo.
echo 当前电脑 IP: 192.168.3.53
echo 目标电脑 IP: 192.168.3.43
echo.

echo 1. 测试网络连通性 (ping)
ping -n 2 192.168.3.43

echo.
echo 2. 检查防火墙规则
netsh advfirewall firewall show rule name="ShareFlow UDP" 2>nul
if errorlevel 1 (
    echo [警告] 未找到防火墙规则
    echo.
    echo 是否添加防火墙规则? 需要管理员权限
    echo 请以管理员身份运行以下命令:
    echo.
    echo netsh advfirewall firewall add rule name="ShareFlow UDP" dir=in action=allow protocol=UDP localport=8080
    echo netsh advfirewall firewall add rule name="ShareFlow TCP" dir=in action=allow protocol=TCP localport=8080
)

echo.
echo 3. 检查端口占用
netstat -an | findstr ":8080"

pause
