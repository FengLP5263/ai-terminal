---
name: ai-terminal-packaging
description: 构建、归档和回读校验 ai-terminal 的本地 Windows ZIP，记录版本、源码身份、依赖与许可证，不自动发布。
---

# 本地制品

先读根 AGENTS.md、数据与资产记录。Windows x64，Python 3.11+，Rust 与 C++/SDK 已就绪。
先运行 ci 与对应版本原生 smoke；视觉／输入修改还要真实截图验收。

```powershell
.\scripts\windows\project.ps1 package
.\scripts\windows\project.ps1 verify-package -Path .\output\packages\<package>.zip
```

POSIX 配对入口可跨平台 verify-package；原生 build/package 需要 Windows。
package 会调用 Cargo release 构建，生成独立版本目录和 ZIP，记录实际 Git/dirty、编译器、源输入及文件哈希，
归档 manager/helper 与许可证，再回读核验。默认拒绝未提交改动；治理／本地验收显式 AllowDirty 且 manifest 标记。
不覆盖正在运行的 preview、不改用户数据、不创建标签、远端或上传。
依赖许可证收集缺口须在 manifest/第三方清单明确，不把自动收集当作完整法律审查。
签名、公开发行与自动更新尚未建立；用户授权发布后才对具体目标处理。
变更打包规则时同步 Python 回归、Windows/POSIX 入口、scripts/README.md、README 和数据/资产记录。
