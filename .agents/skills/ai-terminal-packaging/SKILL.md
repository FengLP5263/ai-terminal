---
name: ai-terminal-packaging
description: 构建、归档和回读校验 ai-terminal 的本地 Windows ZIP，记录版本、源码身份、依赖与许可证，不自动发布。
---

# 本地制品

简体中文 | [English](SKILL.en.md)

先读根 AGENTS.md、数据与资产记录。Windows x64，Python 3.11+，Rust 与 C++/SDK 已就绪。
先运行 ci 与对应版本原生 smoke；视觉／输入修改还要真实截图验收。

```powershell
.\scripts\windows\project.ps1 package
.\scripts\windows\project.ps1 verify-package -Path .\output\packages\<package>.zip
```

POSIX 配对入口可跨平台 verify-package；原生 build/package 需要 Windows。
package 会调用 Cargo release 构建，生成独立版本目录和 ZIP，记录实际 Git/dirty、编译器、源输入及文件哈希，
归档 manager/helper 与许可证，再回读核验；源码身份包括 build.rs 与 assets/branding，正式图标已内嵌于主程序。默认拒绝未提交改动；治理／本地验收显式 AllowDirty 且 manifest 标记。
不覆盖正在运行的 preview、不改用户数据、不创建标签、远端或上传。
依赖许可证收集缺口须在 manifest/第三方清单明确，不把自动收集当作完整法律审查。
签名、公开发行与自动更新尚未建立；用户授权发布后才对具体目标处理。
变更打包规则时同步 Python 回归、Windows/POSIX 入口、scripts/README.md、README 和数据/资产记录。


打包从 Cargo metadata 的 target_directory 读取本次 release 二进制，遵循标准 CARGO_TARGET_DIR／build.target-dir。默认构建目录的 EXE 被正在运行的预览占用时，可在当前命令环境设置 CARGO_TARGET_DIR 为另一个忽略的构建目录再执行相同 package 入口；不会覆盖已打开的程序，也不会从旧默认目录误取 EXE。

制品源码身份还须覆盖本地 egui_glow 的 Cargo 清单、Rust 源码与 GLSL 着色器；相关许可证通过 Cargo metadata 收集。

原生控制台升级后，新包使用 format_version 2；必须包含 conpty.dll、OpenConsole.exe、微软原始许可证及来源记录，并覆盖到源码身份。打包前核对 assets/windows/conpty 的固定哈希；缺失组件不能交付，也不从其他预览目录补拷。保留对旧格式 ZIP 的只读校验。

打包前校验 CHANGELOG.md／CHANGELOG.en.md 含 Cargo 当前版本唯一、同日期、分类且非空的条目；缺失时拒绝构建。ZIP 附带完整双语更新日志，其字节也计入源码身份。更新日志校验可通过既有 governance、ci、package 入口执行，Windows／POSIX 的入口参数不变。
