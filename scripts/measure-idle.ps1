<#
.SYNOPSIS
隔离空闲测量，只统计管理器进程。

.EXAMPLE
    .\scripts\measure-idle.ps1 -Release                  # 优化版短窗口测量
    .\scripts\measure-idle.ps1 -Release -Label opengl    # 带标签对照

.PARAMETER Release
选择 release；默认 debug。

.PARAMETER Label
目录标签，仅字母、数字、连字符，默认 native。

.NOTES
从脚本位置定位仓库；只管理本项目。Python 3.11+，原生动作要求 Windows/Rust/C++ SDK。
命令失败返回非零；真实模型调用、远端发布和清理用户数据不包含在默认检查内。
#>
param([switch]$Release,[ValidatePattern('^[A-Za-z0-9-]+$')][string]$Label='native')
$ErrorActionPreference='Stop'
[Console]::InputEncoding=[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false)
$OutputEncoding=[Console]::OutputEncoding
$taskRoot=Split-Path -Parent $PSScriptRoot
$taskProfile=if($Release){'release'}else{'debug'}
$taskExe=Join-Path $taskRoot "target\$taskProfile\ai-terminal.exe"
$taskOut=Join-Path $taskRoot ('output\idle-'+$Label+'-'+(Get-Date -Format 'yyyyMMdd-HHmmss-fff'))
New-Item -ItemType Directory -Path $taskOut -Force | Out-Null
$taskWatch=[Diagnostics.Stopwatch]::StartNew()
$taskProcess=Start-Process -FilePath $taskExe -ArgumentList @('--idle-benchmark','--data-dir',('"'+$taskOut+'"')) -WorkingDirectory $taskRoot -WindowStyle Hidden -PassThru
while(-not(Test-Path -LiteralPath (Join-Path $taskOut 'ready.json')) -and $taskWatch.Elapsed.TotalSeconds -lt 25 -and -not $taskProcess.HasExited){Start-Sleep -Milliseconds 100}
$taskReadyMs=$taskWatch.ElapsedMilliseconds
if($taskProcess.HasExited){throw 'Process exited before idle measurements.'}
if(-not(Test-Path -LiteralPath (Join-Path $taskOut 'ready.json'))){$taskProcess.Kill();throw 'First UI marker timed out.'}
$taskProcess.Refresh()
$taskCpuBefore=$taskProcess.TotalProcessorTime.TotalSeconds
Start-Sleep -Seconds 5
$taskProcess.Refresh()
$taskCpuDelta=$taskProcess.TotalProcessorTime.TotalSeconds-$taskCpuBefore
$taskMeasure=[ordered]@{ready_ms=$taskReadyMs;cpu_seconds_in_5s=$taskCpuDelta;working_set_bytes=$taskProcess.WorkingSet64;private_bytes=$taskProcess.PrivateMemorySize64;logical_processors=[Environment]::ProcessorCount}
$taskMeasure | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'process.json') -Encoding UTF8
if(-not $taskProcess.WaitForExit(15000)){$taskProcess.Kill();throw 'Idle benchmark timed out.'}
Get-Content -LiteralPath (Join-Path $taskOut 'idle.json') -Encoding UTF8 -Raw
$taskMeasure | ConvertTo-Json
Write-Output "Evidence: $taskOut"
