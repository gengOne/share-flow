@echo off
echo 添加 ShareFlow 防火墙规则
echo 需要管理员权限
echo.

net session >nul 2>&1
if %errorLevel% neq 0 (
    echo [错误] 请以管理员身份运行此脚本！
    echo 右键点击 -^> 以管理员身份运行
    pause
    exit /b 1
)

echo 添加入站规则...
netsh advfirewall firewall add rule name="ShareFlow UDP In" dir=in action=allow protocol=UDP localport=8080
netsh advfirewall firewall add rule name="ShareFlow TCP In" dir=in action=allow protocol=TCP localport=8080

echo.
echo 添加出站规则...
netsh advfirewall firewall add rule name="ShareFlow UDP Out" dir=out action=allow protocol=UDP localport=8080
netsh advfirewall firewall add rule name="ShareFlow TCP Out" dir=out action=allow protocol=TCP localport=8080

echo.
echo ✓ 防火墙规则添加成功！
echo.
echo 当前规则:
netsh advfirewall firewall show rule name=all | findstr "ShareFlow"

pause
