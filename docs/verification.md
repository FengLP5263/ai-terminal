# 0.1.0 本地预览验证记录

简体中文 | [English](verification.en.md)

2026-10-30。Windows x64，Rust 1.99.1 MSVC，egui/eframe 0.35.0，Alacritty 0.26.0。
默认 GPU 后端为 glow/OpenGL。记录时尚未首次提交；这是当时的历史状态，当前 Git 状态以仓库为准。

## 已完成的检查

- `cargo fmt --all -- --check`：通过。
- `cargo clippy --locked --workspace --all-targets -- -D warnings`：通过。
- `cargo test --locked --workspace`：26 项通过。
- `cargo build --locked --release`：通过。
- 优化构建原生冒烟测试：通过；截图已检查，中文混排行距正确。
- 冒烟进程结束后，三个受管 PowerShell PID 52400 / 59520 / 43348 均已退出。

26 项测试：15 core、1 主程序后台逻辑、1 native hook 进程、1 已安装 Codex 配置解析、
1 真实 ConPTY、7 terminal widget。后台逻辑测试使用真实 CMD PTY 和认证状态消息，
在最小化/失焦输入、零 UI 绘制下检查完成状态与未读标记。常驻开发服务不保持 AI 转圈。

## 真实 Agent 联调范围

Claude Code 2.1.314：通过既有用户接入完成一次禁用工具的真实模型回合，收到
SessionStart → UserPromptSubmit → Stop → SessionEnd，8.281 秒；未修改用户设置、
凭据或信任记录。脚本单次预算上限 $0.15，不属于默认单元测试。

Codex 0.164.0：实际 CLI 接受生成的会话级 hook TOML，在隔离 CODEX_HOME 下解析配置，
没有登录或写入 auth.json。首次使用仍需用户在 `/hooks` 中审查状态钩子；
没有绕过审查，也没有把配置解析冒充真实 Codex 模型回合测试。

原生冒烟截图使用六个真实 PowerShell 终端和**合成生命周期事件**（覆盖六个已识别的 Agent 头像）。
该截图验证渲染、后台状态、进程管理，不能单独证明真实 Agent 的全部交互兼容性。

## 当前设备测量

| 配置 | 空窗口主进程驻留内存 | 三个真实终端时主进程驻留内存 |
| --- | ---: | ---: |
| 最终 OpenGL release | 89.6 MiB | 115.0 MiB |
| OpenGL debug 对照 | 151.2 MiB | 117.4 MiB |
| 初始 DX12 debug 对照 | 612.1 MiB | 721.2 MiB |

以上只统计管理器进程，**不包含子终端或 Agent 进程**，也不代表长时间运行峰值。
OpenGL 使用 Quadro RTX 4000 / OpenGL 3.3 / NVIDIA 573.91；DX12 使用 Intel UHD 630。
这是本机配置选择，不是同显卡纯后端对比，不宣称对所有机器最优。

最终 release 空窗口在约 8.03 秒窗口内绘制 8 帧；
UI 回调累计 CPU 14.5 ms。首次 UI 回调标记约
2.678 秒（100ms 外部采样，并非严格屏幕首帧显示）。
首次 UI 后 5 秒内进程 CPU 累计 0.234375 秒，机器有
16 个逻辑处理器；该短窗口可能含启动/输入成本，不当作稳态零 CPU 指标。

两个优化程序文件合计 7,702,528 字节（7.35 MiB）。
预览入口 `output/preview/ai-terminal.exe`，状态 helper 必须同目录；校验见 `build.json`。

## 本机证据与复现

- 最终界面：`output/smoke-20261030-125909/smoke.json`、`smoke.png`。
- 最终开销：`output/idle-final-opengl-20261030-125923-473/idle.json`、`process.json`。
- 真实 Claude：`output/live-claude-20261030-122056/result.json`。
- 重跑：`scripts/smoke.ps1 -Release`、`scripts/measure-idle.ps1 -Release`。

output 不入 Git，可通过脚本重新生成。真实 Claude 脚本会调用已有模型服务；
默认测试不调用模型 API。实际中文输入法、图片粘贴、长期大量会话仍待用户体验验证。

## 普通启动核查

普通交互预览已启动，窗口标题 ai-terminal，主窗口句柄 31852918，进程响应正常。
额外隔离启动也产生有效主窗口和首次 UI 标记。受限检查进程曾返回零句柄，
改为同桌面环境读取后确认窗口存在，无需修改源码。额外预览已正常关闭，仅保留一个。
原生窗口截图工具当前不可用；渲染截图验证使用上文应用自身的原生冒烟测试。
本机记录：`output/preview/window-check.json`。

## 未发布：Codex 接管复核（2026-11-04）

- 独立 CI 通过：181 项 Rust、14 项 Python，含治理、fmt、Clippy。原生字形模块 7 项回归包含两项新增的退休纹理验证：同帧形状仍有效，下一帧即使命中缓存也输出释放记录。
- 优化版构建与六会话原生 smoke 通过，自建 PTY 均已回收。100% 与 146% 展开侧栏截图中三枚新图标、中文和 emoji 可辨识；200% 收起侧栏也完成 smoke，但视口只容纳列表前部，未据该图证明下方三枚图标全部可见。
- 三枚资产来源分别核实到 Kimi Code 官方文档仓库、ZCode 官方 README 和 TRAE 官方 CLI 文档。使用正式侧栏同一图标缓存，无运行时下载；SOURCE.json 逐文件记录固定来源、许可范围和原始／交付身份。
- 记录与日志在忽略的 output/codex-takeover-20261104；标准原生截图位于 output/smoke-20261104-223847-661。
- 未发起模型请求；真实 CLI 交互、真实桌面首次创建、用户原先动态闪烁症状与长期负载仍需分别验收。通用 DSR 往返测试不构成真实 Qoder 输入残留的根因证明。

## 0.4.7 发行验证补充

本机公开版本查询通过：Kimi Code 2.5.0，TRAE CLI 0.213.1 public，社区 zcode-app-cli 3.17.2-44／运行时 0.17.1。正式 App 组件的原生启动截图显示正确头像和“等待任务通知”：Kimi 停在目录信任页，ZCode 社区版显示输入界面，TRAE 显示登录页。仅在终端空白区聚焦并用 Esc／Ctrl+C 取消后，三者均返回 PowerShell，头像恢复；未确认信任、未登录、未提交模型提示。早期 Kimi 未聚焦的取消截图不能算退出验证；最终已用显式聚焦后的截图复核。

证据位于忽略的 output/release-0.4.7-20261104-232823：installed-clis 中保存三款启动画面及 ZCode／TRAE 返回画面，kimi-focus-cancel 保存 Kimi 最终返回画面。README 六枚头像由正式绘制器原生生成，已检查浅色／深色底图并原样复制，来源与哈希同步到 assets/readme/SOURCE.json。首次真实桌面创建、真实模型任务、Windows 10 及长期负载未在本次新增验证中覆盖；离线回归与启动截图不替代这些验收。

0.4.7 最终门槛通过：181 项 Rust／14 项 Python，治理／fmt／Clippy、优化构建与六个自建 PTY 的原生 smoke（output/smoke-20261104-235553-750），截图已检查。原有原生生命周期测试曾错误假定同一调用唤醒的两次 drain 之间生产者不会推进；现把不重放断言放到最终 Done 之后，完整有序状态序列断言仍保留。该原生测试目标连续三次通过后，完整 CI 通过。
