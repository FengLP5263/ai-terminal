<#
.SYNOPSIS
ai-terminal 本地开发、验证与打包的 Windows 权威入口。

.EXAMPLE
# 常用开发与检查
    .\scripts\windows\project.ps1 governance                                       # 静态治理与提交候选检查
    .\scripts\windows\project.ps1 check                                            # Rust 格式与 Clippy
    .\scripts\windows\project.ps1 test                                             # Rust 与 Python 离线回归
    .\scripts\windows\project.ps1 ci                                               # 组合治理、格式和测试
    .\scripts\windows\project.ps1 run                                              # 构建并打开交互窗口

.EXAMPLE
# 本地制品与校验
    .\scripts\windows\project.ps1 smoke -Release                                   # 检查已构建的原生优化版
    .\scripts\windows\project.ps1 package                                          # 构建并校验本地 ZIP
    .\scripts\windows\project.ps1 verify-package -Path output/packages/demo.zip    # 校验指定 ZIP，不执行包内程序

.EXAMPLE
# 明确授权的模型核查
    .\scripts\windows\project.ps1 verify-claude -AllowModelRequest                 # 明确授权一次可能计费的真实模型核查

.PARAMETER Action
governance（默认）、format-check、check、test、ci、build、run、release、smoke、measure-idle、package、verify-package、verify-claude。

.PARAMETER Release
smoke 和 measure-idle 使用 release 目录；否则使用 debug。

.PARAMETER Path
verify-package 的 ZIP；相对路径以仓库根目录为准。

.PARAMETER AllowDirty
仅本地 package 验收允许脏工作区，manifest 明确记录。

.PARAMETER AllowModelRequest
显式允许 verify-claude 调用现有模型服务，禁用工具、预算上限 0.15 美元。

.PARAMETER Profile
真实模型核查的 helper 构建目录，debug/release，默认 release。

.PARAMETER Label
空闲测量目录标签，默认 native。

.NOTES
从脚本位置定位仓库；只管理本项目。Python 3.11+，原生动作要求 Windows/Rust/C++ SDK。
命令失败返回非零；真实模型调用、远端发布和清理用户数据不包含在默认检查内。
#>
param(
    [ValidateSet('governance','format-check','check','test','ci','build','run','release','smoke','measure-idle','package','verify-package','verify-claude')][string]$Action='governance',
    [switch]$Release,[string]$Path,[switch]$AllowDirty,[switch]$AllowModelRequest,
    [ValidateSet('debug','release')][string]$Profile='release',[ValidatePattern('^[A-Za-z0-9-]+$')][string]$Label='native'
)
$ErrorActionPreference='Stop'
[Console]::InputEncoding=[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
$OutputEncoding=[Console]::OutputEncoding
$taskRoot=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$taskLocalEnv=Join-Path $taskRoot '.tools\env.ps1'
if(Test-Path -LiteralPath $taskLocalEnv){. $taskLocalEnv}
$taskPython=if($env:AI_TERMINAL_PYTHON){$env:AI_TERMINAL_PYTHON}else{'python.exe'}
function Invoke-ProjectPython([string[]]$Parameters){
    if(-not(Get-Command $taskPython -ErrorAction SilentlyContinue)){throw 'Python 3.11+ required; configure AI_TERMINAL_PYTHON.'}
    & $taskPython -X utf8 @Parameters
    if($LASTEXITCODE -ne 0){throw "Python command failed ($LASTEXITCODE)."}
}
function Invoke-ProjectDev([string]$Mode){
    & (Join-Path $taskRoot 'scripts\dev.ps1') -Action $Mode
    if($LASTEXITCODE -ne 0){throw "Cargo action failed: $Mode"}
}
Push-Location $taskRoot
try {
    switch($Action){
        'governance' {Invoke-ProjectPython -Parameters @('scripts/check-repository.py')}
        'format-check' {Invoke-ProjectDev 'format-check'}
        'check' {Invoke-ProjectDev 'check'}
        'test' {Invoke-ProjectDev 'test'; & cargo test --offline --locked -p egui-winit --lib paste_tests; if($LASTEXITCODE -ne 0){throw 'egui-winit paste regression failed.'};Invoke-ProjectPython -Parameters @('-m','unittest','discover','-s','tests','-p','test_*.py')}
        'ci' {Invoke-ProjectPython -Parameters @('scripts/check-repository.py');Invoke-ProjectDev 'check';Invoke-ProjectDev 'test'; & cargo test --offline --locked -p egui-winit --lib paste_tests; if($LASTEXITCODE -ne 0){throw 'egui-winit paste regression failed.'};Invoke-ProjectPython -Parameters @('-m','unittest','discover','-s','tests','-p','test_*.py')}
        'build' {Invoke-ProjectDev 'build'}
        'run' {Invoke-ProjectDev 'run'}
        'release' {Invoke-ProjectDev 'release'}
        'smoke' {& (Join-Path $taskRoot 'scripts\smoke.ps1') -Release:$Release}
        'measure-idle' {& (Join-Path $taskRoot 'scripts\measure-idle.ps1') -Release:$Release -Label $Label}
        'package' {$taskArguments=@('scripts/package.py','build');if($AllowDirty){$taskArguments+='--allow-dirty'};Invoke-ProjectPython -Parameters $taskArguments}
        'verify-package' {if(-not $Path){throw '-Path is required.'};Invoke-ProjectPython -Parameters @('scripts/package.py','verify',$Path)}
        'verify-claude' {if(-not $AllowModelRequest){throw 'Use -AllowModelRequest only after explicit user authorization.'};Invoke-ProjectPython -Parameters @('scripts/verify-claude.py','--profile',$Profile,'--allow-model-request')}
    }
} finally {Pop-Location}
