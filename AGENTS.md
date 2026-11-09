# ai-terminal 协作规范

简体中文 | [English](AGENTS.en.md)

## 范围与边界

Windows 原生 Agent 终端管理器。只做项目／会话导航、真实终端、可靠状态提示和对话恢复；
不在管理器中实现模型代理、存储 API Key 或自动处理 CLI 的信任／授权。
技术栈 Rust、egui/eframe、Alacritty、ConPTY；产品不引入浏览器运行时。

## 改动前必读

- 所有工作：[架构](.agents/architecture/overview.md)、[运行与配置](.agents/architecture/runtime.md)。
- 数据／字体／制品：[数据与资产](.agents/architecture/data-and-assets.md)。
- Agent、协议或关联项目：[集成边界](.agents/architecture/integrations.md)。
- 执行工作流前读对应项目技能：[开发](.agents/skills/ai-terminal-development/SKILL.md)、
  [测试](.agents/skills/ai-terminal-testing/SKILL.md)、[制品](.agents/skills/ai-terminal-packaging/SKILL.md)、
  [Git 协作](.agents/skills/ai-terminal-git-workflow/SKILL.md)。

## 源码与兼容性

- 生产源码为 `src/` 及 `vendor/egui_term/src/`、`vendor/egui-winit/src/`、`vendor/egui_glow/src/`、`vendor/winit/src/`、`vendor/alacritty_terminal/src/`、`vendor/vte/src/` 中的本地补丁；字体为 `assets/fonts/`。不为治理重排源码。
- `src/bin/ai-terminal-hook.rs` 必须保持独立、轻量，不初始化字体或 GPU。
- 状态只来自 Agent 明确的生命周期／交互通知与进程退出。不得用输出静默、铃声或进程存在判断完成。
- 点开会话保留现有运行进程，未启动时直接打开其 Shell，并清除未读；绿点表示本轮完成且尚未查看，查看后消失，不等于项目验收通过。
- 状态消费必须在后台 logic 中进行；隐藏终端继续排空输出且不重绘。
- 保留有界队列、5000 行历史、合并唤醒和 viewport 快照；避免全历史复制、轮询扫描和空闲重绘。
- hook 仅会话级注入，不改用户级配置；不保存提示词、结果、账号凭据，不自动信任项目／hook。
- 保留 vendor 来源与许可证；修改同步对应 vendor 的 `UPSTREAM.md`。
- 字体变更维护来源和 SHA-256；正式界面与探针共用 `src/appearance.rs`。

## Git 与文件规则

- 日常在 `develop`；首次提交建立 `main` 同源基线。独立功能用 `feature/<name>`，修复用 `fix/<name>`。
- 发行按 [语义化版本规范](docs/versioning.md) 决定：兼容修复升修订号，兼容新增功能升次版本并清零修订号，稳定契约的不兼容改动升主版本；0.x 的开发期约定见规范。每次对照上个正式标签的完整范围定版，不覆盖已发布标签或制品。
- 后续进入 main 走审查流程；提交、远端、标签、PR 合并、发布分别以用户当前授权为准。
- 本地源码不含真实 `.env`、私钥、用户配置、数据库、输出、运行数据或交接压缩包。
- `.tools/env.ps1`、`.tools/cargo-network.toml` 是可选本机工具链／网络配置，必须保持忽略。
- 文档使用相对路径或环境变量，禁止写入开发者机器用户名和个人工作目录。
- 项目自有现行文档按[双语规范](docs/i18n.md)同步中英文、维护互链并通过治理检查；历史文档的翻译进度见台账。
- 不擅自停止用户已打开的终端；测试只管理自己创建的进程和隔离数据目录。
- 不创建额外 worktree，也不启动子代理，除非用户明确要求并行工作。

## 权威命令

统一入口见 [scripts/README.md](scripts/README.md)。常用 Windows 命令：

```powershell
.\scripts\windows\project.ps1 governance
.\scripts\windows\project.ps1 check
.\scripts\windows\project.ps1 test
.\scripts\windows\project.ps1 smoke -Release
.\scripts\windows\project.ps1 package
```

POSIX 对应 `sh scripts/posix/project.sh <action>`。当前原生构建仅支持 Windows；
macOS/Linux 对原生动作必须明确失败，不能以跳过测试冒充支持。静态治理和 ZIP 校验可跨平台执行。
PowerShell 使用 UTF-8 文本 I/O；带中文的 ps1 保存为 UTF-8 BOM。命令头先列常用示例再解释参数。

## 关联与完成标准

无其他源码仓库依赖。Codex、Claude Code、PowerShell 是外部 CLI，详见集成边界；
被打开的工作目录和桌宠等相邻项目不因此成为可修改的关联仓库。

完成必须满足：变更范围明确；治理检查、fmt、Clippy 与相应测试通过；
终端/UI/生命周期改动补原生 smoke 和实际截图；真实 Agent 核查与合成事件分开记录。
改命令同步 Windows/POSIX 入口、脚本目录和项目技能；改协议／数据／架构同步对应架构文档。
没有远端发布流程时只交付本地制品，不伪造发布完成。测试通过不能代替实际视觉与输入验收。
