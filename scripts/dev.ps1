<#
.SYNOPSIS
兼容开发入口；新工作优先使用 scripts/windows/project.ps1。

.EXAMPLE
    .\scripts\dev.ps1 check      # 格式与静态检查
    .\scripts\dev.ps1 test       # Rust 离线回归
    .\scripts\dev.ps1 build      # 构建 debug
    .\scripts\dev.ps1 run        # 构建并打开窗口
    .\scripts\dev.ps1 release    # 构建优化版

.PARAMETER Action
build/test/check/run/release/format-check，默认 run。

.NOTES
从脚本位置定位仓库；只管理本项目。Python 3.11+，原生动作要求 Windows/Rust/C++ SDK。
命令失败返回非零；真实模型调用、远端发布和清理用户数据不包含在默认检查内。
#>
param([ValidateSet('build','test','check','run','release','format-check')][string]$Action='run')
$ErrorActionPreference='Stop'
[Console]::InputEncoding=[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false)
$OutputEncoding=[Console]::OutputEncoding
$taskRoot=Split-Path -Parent $PSScriptRoot
$taskLocalEnv=Join-Path $taskRoot '.tools\env.ps1'
if(Test-Path -LiteralPath $taskLocalEnv){. $taskLocalEnv}
if(-not(Get-Command cargo -ErrorAction SilentlyContinue)){throw 'Install Rust stable (MSVC) or configure .tools/env.ps1.'}
Push-Location $taskRoot
try {
    $taskNetwork=Join-Path $taskRoot '.tools\cargo-network.toml'
    $taskCargoArgs=if(Test-Path -LiteralPath $taskNetwork){@('--config',$taskNetwork)}else{@()}
    switch($Action){
        'format-check' {cargo @taskCargoArgs fmt --all -- --check}
        'build' {cargo @taskCargoArgs build --locked -j 4}
        'test' {cargo @taskCargoArgs test --locked --workspace -j 4}
        'check' {cargo @taskCargoArgs fmt --all -- --check; if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}; cargo @taskCargoArgs clippy --locked --workspace --all-targets -j 4 -- -D warnings}
        'run' {cargo @taskCargoArgs build --locked -j 4; if($LASTEXITCODE -eq 0){& (Join-Path $taskRoot 'target\debug\ai-terminal.exe')}}
        'release' {cargo @taskCargoArgs build --locked --release -j 4}
    }
    if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
} finally {Pop-Location}
