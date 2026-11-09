# 第三方来源

简体中文 | [English](THIRD_PARTY_NOTICES.en.md)

| 组件／资产 | 声明 | 来源与原许可证 |
| --- | --- | --- |
| egui_term 本地快照 | MIT | vendor/egui_term/LICENSE、UPSTREAM.md，固定上游 revision |
| Alacritty terminal | Apache-2.0 | Cargo.lock 锁定；上游 https://github.com/alacritty/alacritty |
| egui / eframe | MIT OR Apache-2.0 | Cargo.lock 锁定；上游 https://github.com/emilk/egui |
| Cascadia Mono v2407.24 四种字形（可选） | SIL Open Font License 1.1 | 微软官方发布包；assets/fonts/LICENSE-Cascadia.txt、SOURCE.json |
| Hack v3.003 四种字形（可选） | 以原文件为准（MIT／Bitstream Vera 条款） | assets/fonts/LICENSE.md、SOURCE.json |
| 其他 Rust 依赖 | 以各 Cargo 元数据和原文件为准 | package 清单记录版本、声明与可找到的许可证 |

Windows 系统字体只在运行时读取，不随包复制。项目 LICENSE 适用于本项目代码，不替代第三方许可。
本地 package 流程生成 dependencies.json，收集可获得的 LICENSE/COPYING/NOTICE 并保留未找到项。
这份清单记录来源和收集范围，不声称已经完成所有依赖、运行时或发行方式的法律审查。

## Agent icons

- winit 0.30.13：本地 Windows 拖动 DPI 兼容补丁，Apache-2.0。见 [补丁来源](vendor/winit/UPSTREAM.zh.md) 和原始 LICENSE；制品依赖清单收集该本地依赖及许可证。

- OpenAI／Claude／Qoder 图标来自 [Lobe Icons](https://github.com/lobehub/lobe-icons)，MIT；原文为 assets/icons/LICENSE-lobe-icons。
- Kimi Code 图标为 [MoonshotAI/kimi-code](https://github.com/MoonshotAI/kimi-code) 官方文档使用的 PNG，固定提交并保留原始字节；仓库声明 MIT，原文为 assets/icons/LICENSE-kimi-code。
- ZCode 图标为 [zai-org/ZCode](https://github.com/zai-org/ZCode) 官方 README 使用的应用 PNG，固定提交并保留原始字节；仓库声明 Apache-2.0，原文为 assets/icons/LICENSE-zcode。
- TRAE 图标来自 [官方 CLI 文档站](https://docs.trae.cn/cli_about-trae-code-cli-2) 所引用的 PNG，仅离线裁去外围留白并缩小；该官方来源未单独声明品牌资产的开源许可，不适用 Lobe Icons 的 MIT 声明。

各品牌图标仅用于识别相应集成，名称与商标归各自所有者；仓库开源许可不授予商标权。逐文件 URL、固定提交、尺寸、处理方式和 SHA-256 见 assets/icons/SOURCE.json；发行包的 agent-icons/ 包含同一来源记录及上述可获得的原始许可证。

- egui-winit 0.35.0: local paste-event compatibility patch, MIT OR Apache-2.0. See vendor/egui-winit/UPSTREAM.md and its included license files. The packaged dependency inventory includes this patched crate.

- egui_glow 0.35.0：本地 OpenGL 缓冲区复用补丁，MIT OR Apache-2.0。见 vendor/egui_glow/UPSTREAM.zh.md 及随附许可证；制品依赖清单记录该补丁。

- Microsoft.Windows.Console.ConPTY 1.25.260930003：MIT，微软官方 x64 原生控制台组件。见 [来源与校验记录](assets/windows/conpty/SOURCE.json) 和[原始许可证](assets/windows/conpty/LICENSE.txt)。发行包包含 conpty.dll、OpenConsole.exe 及许可／来源副本；该依赖单独记录在 manifest.json 的 native_console 中。

## JetBrains Mono

可选终端字体来自 [JetBrains 官方 v2.304](https://github.com/JetBrains/JetBrainsMono/releases/tag/v2.304)，采用 SIL OFL 1.1。四种静态字形与许可保留原始字节，来源和 SHA-256 见 `assets/fonts/SOURCE.json`。源码许可原文为 [LICENSE-JetBrainsMono.txt](assets/fonts/LICENSE-JetBrainsMono.txt)，发行 ZIP 随附 `JetBrainsMono-LICENSE.txt`；系统字体不复制或分发。

## Codicons

Git 分支、提交、复制和文件图标采用微软 [VS Code Codicons](https://github.com/microsoft/vscode-codicons) 官方 `@vscode/codicons` 0.0.45 包的原始字体和 `git-branch`（U+EC6F）、`copy`（U+EBCC）、`git-commit`（U+EAFC）与 `file`（U+EA7B）等分类文件字形，CC BY 4.0。字体、CSS 映射和许可未改动；来源及哈希见 `assets/fonts/SOURCE.json`，原许可为 `assets/fonts/LICENSE-Codicons.txt`，ZIP 随附 `Codicons-LICENSE.txt`。只作为程序内符号使用，不安装系统字体或作为终端字体选项。
