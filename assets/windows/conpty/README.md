# 原生控制台组件

简体中文 | [English](README.en.md)

随应用分发微软官方 `Microsoft.Windows.Console.ConPTY` 的固定 x64 运行时。来源、版本、大小、SHA-256 与签名核查记录见 [SOURCE.json](SOURCE.json)，原始许可见 [LICENSE.txt](LICENSE.txt)。这些是运行依赖资产，不是本项目的构建输出。

Windows 10 build 19045 的内置 ConPTY 在本项目的原生样例中丢失全屏和鼠标模式；相同程序切换到此组件后，滚轮翻页和拖选检查通过。使用相同原生控制台版本减少 Windows 10／11 的行为差异，不增加浏览器运行时，不修改系统文件、PATH、用户配置或默认终端。

Alacritty 0.26 会从程序目录加载 `conpty.dll`，该 DLL 使用同目录的 `OpenConsole.exe`。`build.rs` 将两者复制到 Cargo 实际 profile、examples 和 deps 输出目录；内容相同时跳过复制，避免覆盖运行中的组件。正式包同时携带组件、许可证和来源记录，不能只取两个 ai-terminal EXE。包构建及治理检查核对固定资产哈希。

升级时从官方 NuGet 包选定具体版本，校验来源、签名和许可，更新 SOURCE.json，分别在 Windows 10／11 验证全屏输入区、鼠标、缩放、退出和原生截图。不得用临时测试结果宣称所有 CLI／输入法组合均已兼容。
