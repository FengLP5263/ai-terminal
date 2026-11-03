# 参与贡献

简体中文 | [English](CONTRIBUTING.en.md)

感谢你愿意一起完善 ai-terminal。欢迎修复问题、改善输入和显示、提高状态可靠性与性能，也欢迎补充文档和复现案例。

## 反馈问题与建议

请先搜索[现有 Issue](https://github.com/FengLP5263/ai-terminal/issues)。
报告问题时说明 Windows 版本、ai-terminal 版本、Agent CLI 版本、复现步骤、预期结果和实际结果。
渲染与缩放问题最好附上截图、显示器缩放比例和多屏环境；输入问题注明输入法。
不要上传 API Key、认证文件、私人对话或真实会话数据。默认检查不需要模型账号。

较大的功能或架构调整建议先开 Issue 讨论。项目主要关注多 Agent 会话管理、真实终端和可靠状态，尽量保持界面简单、资源开销可控。

## 本地开发

先按 [README](README.md) 配好 Windows、Rust MSVC、C++ Build Tools、Windows SDK 与 Python。
阅读 [AGENTS.md](AGENTS.md) 和[架构总览](.agents/architecture/overview.md)，然后执行：

```powershell
.\scripts\windows\project.ps1 ci
.\scripts\windows\project.ps1 run
```

`main` 是对外主线，`develop` 用于日常集成。Fork 后从 `develop` 建立 `feature/<name>` 或 `fix/<name>` 分支；PR 通常提交到 `develop`。
保持提交范围清晰，不改写其他人的历史。合入 `main` 通过维护者审查。分支同步、PR 合并与发布的具体流程见 [Git 协作规范](.agents/skills/ai-terminal-git-workflow/SKILL.md)。

## 修改时请保留的边界

- 不引入浏览器运行时，不在管理器中实现模型代理或保存 API Key。
- 状态依据真实生命周期事件；不要用输出静默或进程存在推测任务完成。
- 不自动同意 CLI 的项目信任、授权或模型请求，不修改用户级 CLI 配置。
- 不按进程名清理终端；测试只使用隔离数据和自己创建的进程。
- 保留有界历史与队列、后台输出排空和按需重绘，注意空闲资源占用。
- 修改 `vendor/egui_term` 时同步[补丁记录](vendor/egui_term/UPSTREAM.md)；资产需保留来源、哈希和许可证。

## 提交 Pull Request

发版变更须同步 [CHANGELOG.md](CHANGELOG.md) 和 [CHANGELOG.en.md](CHANGELOG.en.md)，为 Cargo 当前版本填写同一日期，按实际列出新增、改进、修复、升级注意事项和验证边界。GitHub Release 沿用对应条目；打包会检查版本条目并附带两份日志。普通开发先记入“未发布”，发版前整理为明确版本，历史已发行记录不改写成当前行为。

PR 说明解决了什么问题、用户可见的变化，以及验证结果。小改动保持简短即可。
提交前执行：

```powershell
.\scripts\windows\project.ps1 ci
```

修改终端显示、输入、界面或生命周期时，还需构建优化版并运行原生 smoke：

```powershell
.\scripts\windows\project.ps1 release
.\scripts\windows\project.ps1 smoke -Release
```

视觉修改附上实际截图，输入修改说明测试过的输入法和操作；性能修改说明构建模式、测量场景与方法。
不要把历史结果当作本次检查结果。默认测试不调用模型 API，不要求贡献者消耗模型额度。
文档或其他低影响修改运行相关检查即可；完整命令与限制见[脚本目录](scripts/README.md)。

不提交 `target/`、`output/`、`.tools/`、会话数据、真实配置、凭据或打包文件。
变更协议、数据或操作命令时同步相关文档。 文档按[双语规范](docs/i18n.md)同步维护中英文。贡献的项目代码沿用 [MIT License](LICENSE)，第三方内容须保留原许可和来源。
