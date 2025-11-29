@echo off
echo UDP 8080 监听测试
echo.
echo 此工具会监听 UDP 8080 端口
echo 如果能收到数据，说明防火墙配置正确
echo.
pause

powershell -Command "$endpoint = New-Object System.Net.IPEndPoint([System.Net.IPAddress]::Any, 8080); $udpClient = New-Object System.Net.Sockets.UdpClient 8080; Write-Host '正在监听 UDP 8080...'; Write-Host '按 Ctrl+C 停止'; Write-Host ''; $count = 0; while($true) { try { $content = $udpClient.Receive([ref]$endpoint); $count++; $message = [System.Text.Encoding]::ASCII.GetString($content); Write-Host \"[收到 #$count] 来自 $($endpoint.Address):$($endpoint.Port) - $($content.Length) 字节\"; } catch { Write-Host \"错误: $_\"; break; } }"
