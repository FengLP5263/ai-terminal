# 第三方来源

简体中文 | [English](THIRD_PARTY_NOTICES.en.md)

| 组件／资产 | 声明 | 来源与原许可证 |
| --- | --- | --- |
| egui_term 本地快照 | MIT | vendor/egui_term/LICENSE、UPSTREAM.md，固定上游 revision |
| Alacritty terminal | Apache-2.0 | Cargo.lock 锁定；上游 https://github.com/alacritty/alacritty |
| egui / eframe | MIT OR Apache-2.0 | Cargo.lock 锁定；上游 https://github.com/emilk/egui |
| Hack v3.003 四种字形 | 以原文件为准（MIT／Bitstream Vera 条款） | assets/fonts/LICENSE.md、SOURCE.json |
| 其他 Rust 依赖 | 以各 Cargo 元数据和原文件为准 | package 清单记录版本、声明与可找到的许可证 |

Windows 系统字体只在运行时读取，不随包复制。项目 LICENSE 适用于本项目代码，不替代第三方许可。
本地 package 流程生成 dependencies.json，收集可获得的 LICENSE/COPYING/NOTICE 并保留未找到项。
这份清单记录来源和收集范围，不声称已经完成所有依赖、运行时或发行方式的法律审查。

## Agent icons

- winit 0.30.13：本地 Windows 拖动 DPI 兼容补丁，Apache-2.0。见 [补丁来源](vendor/winit/UPSTREAM.zh.md) 和原始 LICENSE；制品依赖清单收集该本地依赖及许可证。

OpenAI, Claude and Qoder icons are from [Lobe Icons](https://github.com/lobehub/lobe-icons), MIT.
See assets/icons/SOURCE.json and assets/icons/LICENSE-lobe-icons. Brand names and marks belong to their respective owners.

- egui-winit 0.35.0: local paste-event compatibility patch, MIT OR Apache-2.0. See vendor/egui-winit/UPSTREAM.md and its included license files. The packaged dependency inventory includes this patched crate.

- egui_glow 0.35.0：本地 OpenGL 缓冲区复用补丁，MIT OR Apache-2.0。见 vendor/egui_glow/UPSTREAM.zh.md 及随附许可证；制品依赖清单记录该补丁。

- Microsoft.Windows.Console.ConPTY 1.25.260930003：MIT，微软官方 x64 原生控制台组件。见 [来源与校验记录](assets/windows/conpty/SOURCE.json) 和[原始许可证](assets/windows/conpty/LICENSE.txt)。发行包包含 conpty.dll、OpenConsole.exe 及许可／来源副本；该依赖单独记录在 manifest.json 的 native_console 中。
