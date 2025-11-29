@echo off
echo UDP 广播测试工具
echo.
echo 本机会向 192.168.3.255:8080 发送测试消息
echo 请确保另一台电脑正在运行 ShareFlow 程序
echo.
pause

powershell -Command "$client = New-Object System.Net.Sockets.UdpClient; $client.EnableBroadcast = $true; $bytes = [System.Text.Encoding]::ASCII.GetBytes('TEST'); for($i=1; $i -le 10; $i++) { $client.Send($bytes, $bytes.Length, '192.168.3.255', 8080); Write-Host \"发送测试消息 #$i\"; Start-Sleep -Seconds 1 }; $client.Close()"

echo.
echo 测试完成
pause
