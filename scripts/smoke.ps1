<#
.SYNOPSIS
隔离原生 smoke：六个真实 PowerShell 与合成状态。

.EXAMPLE
    .\scripts\smoke.ps1 -Release    # 验证已构建优化版
    .\scripts\smoke.ps1             # 验证已构建 debug

.PARAMETER Release
选择 release；默认 debug。

.NOTES
从脚本位置定位仓库；只管理本项目。Python 3.11+，原生动作要求 Windows/Rust/C++ SDK。
命令失败返回非零；真实模型调用、远端发布和清理用户数据不包含在默认检查内。
#>
param([switch]$Release)
$ErrorActionPreference='Stop'
[Console]::InputEncoding=[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false)
$OutputEncoding=[Console]::OutputEncoding
$taskRoot=Split-Path -Parent $PSScriptRoot
$taskProfile=if($Release){'release'}else{'debug'}
$taskExe=Join-Path $taskRoot "target\$taskProfile\ai-terminal.exe"
$taskOutput=Join-Path $taskRoot ('output\smoke-'+(Get-Date -Format 'yyyyMMdd-HHmmss-fff'))
if(-not(Test-Path -LiteralPath $taskExe)){throw 'Build ai-terminal first.'}
New-Item -ItemType Directory -Path $taskOutput -Force | Out-Null
$taskProcess=Start-Process -FilePath $taskExe -ArgumentList @('--smoke-test','--data-dir',('"'+$taskOutput+'"')) -WorkingDirectory $taskRoot -WindowStyle Hidden -PassThru
if(-not $taskProcess.WaitForExit(30000)){Stop-Process -Id $taskProcess.Id;throw 'Native smoke test timed out.'}
if($taskProcess.ExitCode -ne 0){throw "Native process failed: $($taskProcess.ExitCode)"}
$taskReport=Join-Path $taskOutput 'smoke.json'
if(-not(Test-Path -LiteralPath $taskReport)){throw "No smoke report: $taskOutput"}
$taskResult=Get-Content -LiteralPath $taskReport -Encoding UTF8 -Raw | ConvertFrom-Json
$taskResult | Select-Object ok,fixture_events,manager_resident_bytes,frames,ui_cpu_micros,elapsed_ms | ConvertTo-Json
if(-not $taskResult.ok){throw "Smoke failed: $taskReport"}
if(-not(Test-Path -LiteralPath (Join-Path $taskOutput 'smoke.png'))){throw 'Native GPU screenshot missing.'}
foreach($taskSession in $taskResult.sessions){if($null -ne (Get-Process -Id $taskSession.pty_pid -ErrorAction SilentlyContinue)){throw "Owned smoke PTY still running: $($taskSession.pty_pid)"}}
Write-Output "Evidence: $taskOutput"
