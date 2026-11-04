# 数据、字体与制品

简体中文 | [English](data-and-assets.en.md)

## 数据与恢复

%LOCALAPPDATA%/ai-terminal/sessions.json 保存会话名称、目录、Agent 类型、实际 resume ID、列表顺序和 UI 偏好。
0.3.0 新增可选 last_directory；Cmd/Bash 类型与旧 Codex/Claude/Shell 并存，旧记录保持兼容。0.3.2 起将旧 Codex/Claude 类型转换为 PowerShell，保留其余字段；只自动打开选中的 Shell，其余按需打开。当前 shell 的目录由提示符元数据更新；运行中的 Agent 图标与调用 ID 只存在内存。
0.3.3 每个会话新增可选 color（7 个固定颜色的枚举值）；旧记录缺失时默认无色，恢复默认时省略字段。改色不改变 UUID、目录、Agent 身份或运行进程。
ui 对象包含 sidebar_collapsed、sidebar_width、ui_scale；旧记录通过 serde 默认值兼容加载。
terminal_font_size 作为旧版兼容字段保留；界面不再单独缩放终端字号。全界面比例 0.5–2.5，侧栏逻辑宽度 190–420；缩放／拖动宽度延迟 400ms 保存，退出时立即保存。
使用同目录临时文件、flush/sync 与原子替换；坏 JSON 不能被静默覆盖。
runtime/ 保存会话级 hook 配置；桥 token 只在进程环境和短消息内传递。
终端历史保留在进程内，最多 5000 行，不保存提示词／转录，不承诺重启恢复原始屏幕或进程。

尚无数据库和迁移框架。未来改 session schema 时使用兼容默认值与 fixture 回归；
出现不兼容变化必须先定义备份／迁移／回退，不允许清空用户记录解决问题。

## 资产与来源

- assets/fonts/Hack-*.ttf：官方 v3.003，来源、大小和 SHA-256 在 SOURCE.json，原许可证 LICENSE.md。
- assets/branding：项目原创 B「会话叠窗」图标，SVG 与用户选定源字节相同，PNG／ICO 为固定导出；SOURCE.json 记录大小和 SHA-256，治理检查逐项核对。
- assets/icons：OpenAI／Claude／Qoder 来自 Lobe Icons（MIT）；Kimi Code／ZCode 使用各自官方仓库的产品图标，TRAE 使用官方 CLI 文档站的图标。SOURCE.json 逐文件记录来源、许可声明、原始尺寸、变换、大小与 SHA-256，不跨来源套用许可证。
- Windows 中文／UI／emoji 字体：读取系统字体（含 Segoe UI Emoji），不复制到 Git 或交付包。
- vendor/egui_term：MIT，上游固定 revision 和本地补丁记录在 UPSTREAM.md。
- 其他 Cargo 依赖：见 Cargo.lock 和 THIRD_PARTY_NOTICES.md；打包生成依赖清单并收集本机可获得的许可证。

字体升级必须同步四种字形、来源、哈希、许可证、DPI/基线测试及实际截图。
包内程序、helper、项目／vendor／字体许可证必须完整；包校验失败不能作为可交付包。

## 生成与排除

target/、output/、.runtime/、runtime/、.tools/、参考交接文件、数据库／dump、真实 .env、凭据和压缩制品不提交。
测试在 tempfile 或 output/ 的唯一目录运行，不复用用户真实数据目录。字体属于源码资产，不被二进制忽略规则隐藏。

发布包通过 scripts/package.py 构建、记录实际版本／Git／编译器／源文件与程序 SHA-256、归档并回读校验。
默认要求干净 Git；治理验收可显式 AllowDirty，manifest 必须如实记录。没有部署服务、云盘或远端上传副作用。

界面名称与重命名框使用完整 Unicode grapheme 布局，emoji 以原生透明图块覆盖；文字编辑、选择与剪贴板仍由标准 egui TextEdit 管理。与终端共用有界图集和资源生命周期，不安装／打包系统字体。

`design/icon-concepts/` 保存 4 份原创 SVG 与离线 HTML 选择页；0.3.4 起已选定 B；正式资产位于 assets/branding，构建时嵌入，不在运行时读取候选页。

制品源码身份包含 build.rs 和 assets/branding；两个 EXE 已含所需图标，不需要另放 ICO／PNG 文件。


0.3.7 的 egui-winit 本地补丁固定于 0.35.0，保留原始归档 SHA-256、上游 commit 及 MIT/Apache-2.0 许可证。制品源码身份纳入该补丁的 manifest 和 Rust 源码；许可证由 Cargo metadata 归集。图片粘贴仅转发原生快捷键，图片数据仍归 CLI 管理，不写入 sessions.json。


打包从 Cargo metadata 的 target_directory 读取本次 release 二进制，遵循标准 CARGO_TARGET_DIR／build.target-dir。默认构建目录的 EXE 被正在运行的预览占用时，可在当前命令环境设置 CARGO_TARGET_DIR 为另一个忽略的构建目录再执行相同 package 入口；不会覆盖已打开的程序，也不会从旧默认目录误取 EXE。


0.4.0 在 ui 中新增 language（system／zh_cn／en，默认 system）、floating_enabled（默认 false）、floating_position（旧版可选逻辑坐标）以及 floating_position_pixels（按显示器物理像素保存，首次从旧坐标迁移）。旧文件通过 serde 默认值继续加载；切换语言与浮窗选项沿用 400ms 延迟原子保存，退出时立即保存。浮窗快照仅包含 ID、名称、Agent、颜色、状态和未读，不持久化任务正文。背景透明胶囊与齿轮都是代码绘制，没有新增图片、系统字体副本或浏览器依赖。

0.4.2 的 egui_glow 本地补丁记录上游来源和许可证；制品源码摘要包含其 Cargo 清单、Rust 源码及 GLSL 着色器。

## 原生控制台资产与制品格式

`assets/windows/conpty/SOURCE.json` 固定微软 NuGet 包版本、下载 URL、包哈希、x64 文件哈希、最低 Windows build、签名核查和许可证来源。治理仅对该目录中固定的 OpenConsole.exe 给予第三方资产例外，其他 EXE 仍被拒绝；许可证保持原文。

新包 manifest 使用 format_version 2，包含 native_console 身份、两个原生组件、Microsoft-ConPTY-LICENSE.txt 和 Microsoft-ConPTY-SOURCE.json；缺失时校验失败。验证器仍接受旧格式发行包。源码摘要包括整个原生资产目录；打包复制前再次核对固定哈希，不从另一版本的输出目录取组件。

0.4.3 起根中英文 CHANGELOG 纳入源码摘要并随 ZIP 分发；构建先核对当前版本、同一日期和非空的分类变更。GitHub Release 沿用相同版本条目。旧 ZIP 的只读完整性校验不因缺少新日志文件而失效。

Agent 头像保留上游 PNG 原始尺寸，TRAE 仅离线裁去外围留白并缩至 640×640；按实际物理尺寸做面积过滤，每种头像最多共享缓存 12 个尺寸，整数像素对齐，不在每帧缩图。原生表情图块完整适配单元格，VS15 保留文字呈现，不改变终端网格坐标。

新增本地 winit 0.30.13（Apache-2.0）补丁的来源及归档哈希见 vendor/winit/UPSTREAM.zh.md，原始 LICENSE 保留。制品源码身份包含它的 Cargo.toml、build.rs 及 src 下的源码／嵌入资产；依赖清单自动记录本地补丁并收集该 LICENSE。

Qoder CN 使用 Lobe Icons 的 640 像素 Qoder 图标，来源、MIT 许可和 SHA-256 记录在 assets/icons；与其他 Agent 共用按物理尺寸面积过滤的有界缓存。Agent::QoderCn 仅增加身份类型，会话持久化字段、认证边界与终端历史上限不变。

Kimi Code 使用官方仓库文档所用的 316×316 PNG，ZCode 使用官方仓库 README 所用的 1024×1024 PNG，均固定提交并保留原始字节；各仓库 MIT／Apache-2.0 原文分别保存在 LICENSE-kimi-code／LICENSE-zcode，仓库许可不授予商标权。TRAE 来自官方 CLI 文档站的 2400×2400 PNG，仅裁去外围留白并离线缩至 640×640，原始哈希和变换登记在 SOURCE.json；官方来源没有单独标明该品牌资产的开源许可，不能标为 Lobe/MIT。所有品牌图标仅用于识别集成，不新增运行时网络请求。ZIP 的 agent-icons/ 保存来源清单和已有原始许可，另附双语第三方说明。

0.4.7 的 README 支持表使用 assets/readme/agent-*.png：由 examples/agent_icons_capture.rs 原生调用正式头像绘制器，在深色底衬上生成六枚图标，保证浅色／深色文档页面的可读性。PNG 原样复制，逐文件版本、尺寸、SHA-256 与来源保存在 assets/readme/SOURCE.json；不含用户数据或真实模型会话。
