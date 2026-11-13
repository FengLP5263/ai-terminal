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

- assets/fonts/CascadiaMono-*.ttf：可选英文／数字字体，微软官方 v2407.24 的四种原始静态字形，SIL OFL 1.1；归档、固定提交、逐文件哈希与原始许可见 SOURCE.json、LICENSE-Cascadia.txt。
- assets/fonts/Hack-*.ttf：官方 v3.003，可选 0.4.8 字体；来源、大小和 SHA-256 在 SOURCE.json，原许可证 LICENSE.md。
- assets/fonts/JetBrainsMono-*.ttf：官方 v2.304 的四种原始静态字形，默认英文字体；SOURCE.json 固定提交、归档和逐文件 SHA-256，原始 SIL OFL 1.1 在 LICENSE-JetBrainsMono.txt。
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

0.4.8 预览的 README 主图分别保存中文 workspace.png／floating.png 和英文 workspace.en.png／floating.en.png。四张图均由 Release 原生组件生成，使用合成内容与示例目录，原样复制并逐文件记录版本、语言、尺寸与 SHA-256；英文名称及终端样例也使用英文。ZCode 基础设置快照仅在受管调用的忽略 runtime 目录生成，不含 provider／凭据、自定义命令或会话正文，不改会话持久化格式。

0.4.9 预览：新增本地 alacritty_terminal 0.26.0（Apache-2.0）与 vte 0.15.0（Apache-2.0 OR MIT）发布归档的最小协议补丁。保留原许可与双语来源，Cargo 清单和 src 下的 Rust 文件纳入制品摘要及依赖许可清单。字体文件和来源不变；preview.1 的 GDI classic 笔画拟合在用户目视验收后撤回；preview.2 恢复 0.4.8 默认渲染参数，emoji 同样保留原参数。背景先于块光标和反色字形绘制，宽字符占两格。

0.4.9-preview.3 按用户目视选择采用 Cascadia Mono＋系统微软雅黑 UI，保持原生默认参数和字号。Hack 保留为对比样例；ZIP 附带 Cascadia-LICENSE.txt 与 FONT-SOURCE.json。新包声明字体但缺少来源／许可时拒绝验证，旧包继续可校验。不复制或安装系统中文／UI／emoji 字体。

0.4.9-preview.5：egui_glow 的 CPU 暂存仅保留当前可见绘制的顶点、索引和范围；容量按最大帧复用，不持久化终端正文。新 mesh_batch.rs 与其 vendor 来源记录继续计入制品源码身份。

发布源码中的上游终端录制保留为二进制差异，两个新增 vendor 快照禁止 Git 换行转换；Cascadia 原始许可保留全部字节及原始空白。项目自有源码继续执行正常格式与空白检查。


0.4.10 预览：`ui.terminal_font` 使用 `jetbrains_mono`（默认）、`cascadia_mono`、`hack`、`consolas`、`courier_new` 稳定 ID。三个内置家族复用已固定的四种字形；新增系统字体不分发字节，SOURCE.json 记录选择目录与来源边界。NSIS 编译工具及原始许可固定在 scripts/installer 的来源记录；程序安装文件带独立身份标记，卸载只删除生成清单中的文件和空目录，不递归删除程序目录，不接触数据目录。EXE／摘要／JSON 输出与隔离安装证据均在忽略的 output/。

0.4.12 预览：颜色菜单为六色加无颜色，沿用 `blue`、`green`、`amber`、`pink`、`purple` 并新增 `red`。旧 `teal`、`orange` 原 ID 可往返保存，显示及菜单选择分别映射到蓝／红，不改写会话 UUID、目录或顺序。颜色和无颜色图标均为代码绘制，无新增图片或字体资产。

0.4.12 Git 面板仅新增 `ui.git_panel_open` 布尔偏好，旧记录默认收起；分支、路径、行数和提交元数据只在当前后台快照内存中存在。未跟踪文本只临时读取以计行，不保存正文、差异、历史缓存或 Git 凭据；不更改会话数据根目录或用户仓库。

0.4.12-preview.3：`assets/fonts/Codicons.ttf`、原始 CSS 字形映射及许可来自微软官方 Codicons 0.0.45 npm 包；SOURCE.json 记录原始字节、归档摘要和 U+EC6F 分支符号。该字体仅用于 UI 图标，不进入终端字体选项、不安装到系统。正式 appearance 与探针共用同一字体定义；helper 不加载。ZIP 复制 Codicons-LICENSE.txt 并计入字体许可校验。

Git 提交详情不持久化：仅保留当前查询与已展开提交的内存数据，至多展示 500 个文件；每条命令仍为 6 秒／2 MiB 上限。复制仅由用户点击写入完整提交 ID；不自动读取剪贴板。复制与文件图标复用既有 Codicons U+EBCC／U+EA7B 原始字形，无新增资产。

0.6.0-preview.2：更改与提交文件共用 git_ui/files.rs，分类图标全部来自既有 Codicons 原始字体；SOURCE.json 列出实际使用的 Markdown、代码、JSON、配置、脚本、媒体、PDF、归档与二进制字形。没有新增字体或图片资产。

0.7.0 预览：历史大小仅为 O(1) viewport 元数据，不复制完整历史。差异只保留一个有界内存结果（20,000 行／每行 4,000 字符，命令输出上限 2 MiB），不保存至工作区或磁盘；无新增字体／图像资产，颜色复用 chrome::COMPLETED／FAILED。

Syntect 5.3.0 的语法资源随锁定 crate 内嵌；不启用默认主题资源，主题颜色为项目代码定义。Cargo.lock 记录归档校验值；MIT 原始 LICENSE.txt 随 dependencies.json 和 licenses/syntect-5.3.0/ 自动归档。展示快照最多保留当前文件 100,000 token（每行至多 256 段），只含字节边界、RGB、源行索引和缩进，不写入工作区。新增 fancy-regex 等依赖的声明与原始许可由制品依赖清单记录。

preview.6 的工作区文件正文同样只存在于当前有界内存结果，不保存文件副本、索引或差异缓存。PatchSource 为展示内存类型，不改变 sessions.json；没有新增字体、图像或第三方依赖。

0.7.0 的 README 图由 readme_capture（默认／history／diff）和 island_probe 的优化构建生成，八张中英文主界面、Git 历史、只读差异和浮窗 PNG 保持原生字节；SOURCE.json 逐文件记录版本、语言、尺寸与 SHA-256。示例代码、元数据和历史均为固定合成内容，不读取真实用户配置、仓库或会话。

0.8.0 预览为 UiSettings 增加 floating_scale，兼容缺失字段且不改数据根、会话 ID／目录／顺序、字体或进程。配置保存后两种比例独立；不新增字体或图标资产。

0.8.0-preview.2 语法数据来自 Cargo.lock 固定的 two-face 0.5.2+bat-0.26.1，使用完整 Oniguruma 变体，不加载第三方主题。assets/syntaxes/LICENSES.json 从该 crate 的 acknowledgement::listing().for_syntaxes() 导出，保留 66 项完整许可文本；SOURCE.json 记录版本、Cargo 归档校验值、导出命令与 SHA-256。治理／打包核对来源，ZIP 携带 Syntax-LICENSES.json／SYNTAX-SOURCE.json，两者计入源码身份。two-face、onig/onig_sys 自身许可另随依赖清单归档。字体字节未变，来源记录新增 terminal-powershell（U+EBC7）、terminal-bash（U+EBCA）、terminal-cmd（U+EBC4）。

0.8.0-preview.3 不新增字体或图标资产，不修改字体字节或笔画参数；头像缓存按最终像素尺寸命中，保持每个图标至多 12 个尺寸。新表单样式与响应式布局不改变会话配置字段、目录或数据根。

0.9.0 预览只新增项目文件面板布尔偏好，不保存文件正文、目录快照或最近文件。目录每级最多 1,000 项，最多 64 个展开目录；文本最多 2 MiB、20,000 行、每行 4,000 字符。支持 UTF-8／BOM 和带 BOM 的 UTF-16，错误明确；拒绝跟随符号链接／junction。Codicons 字节不变，仅扩展 SOURCE.json 中已使用字形的登记。详见 docs/files.md。

0.9.0 的公开 README 图使用 readme_capture 的默认／history／diff／files 模式和 island_probe，共十张中英文原生 PNG。文件树样例仅写入自身输出目录，顶部固定示例路径；不读取用户项目。原样复制并逐文件记录来源、版本、尺寸与摘要，不覆盖历史 Agent 头像的来源版本。

0.9.1-preview.3：文件类型图标改用 Seti UI 1.11.0 的原始 Seti.ttf（MIT），固定提交、原始 LESS 映射／配色／关联参考与 SHA-256 见 assets/fonts/SOURCE.json。file-icons 族仅供文件树、Git 文件行及只读标题使用，保留 Codicons 目录／操作符号。appearance 统一注册，随字号和最终 DPI 直接栅格化，不改终端字体或用户数据；helper 不加载。ZIP 携带 Seti-LICENSE.txt 并核验其声明。

Seti 按官方 VS Code 文件图标的 150% 尺寸规范校准图标族的显示比例；布局槽和继承字号不变，不启用新的笔画拟合或改变终端字体。

0.9.1-preview.4 以 Tabler 原始描边 SVG 替代 Seti 字体，来源、许可和 SHA-256 在 assets/file-icons/SOURCE.json。原始几何不变，只替换 currentColor；SVG 作为编译期资源，resvg 在最终物理像素尺寸栅格化，并按类型／颜色缓存解析结果与至多 8 个纹理尺寸（4–256 像素）。移除 Seti 字体注册和资产，不改变终端字体；ZIP 新增 FILE-ICONS-SOURCE.json／Tabler-Icons-LICENSE.txt，旧包仍可校验。

0.10.0 预览仅新增 ui.floating_opacity 浮窗底板偏好，配置原子保存与旧格式加载路径不变；不新增字体、图片或第三方依赖。原生透明度探针只使用合成元数据，PNG、alpha／几何证据保存在忽略的 output 中，不读取用户会话。

0.10.0-preview.3 的 ignored 标记仅随有界目录缓存存在，不保存到用户设置或读取文件正文；不新增资产或依赖。边缘探针保留 136% 浮窗比例、浅色／灰色底板及窗外 16 像素的原生桌面截图，核对逐像素合成与窗外残影。

0.10.0 正式发行：assets/file-icons 的 SVG 与 MIT 许可通过 .gitattributes 的 -text 保留上游字节，禁止 Windows core.autocrlf 改写行尾；来源摘要必须在提交与分支切换后继续一致。

0.11.0 预览在工作区根对象新增可选 groups：每组包含 UUID id、name 和会话 UUID members。旧数据缺失时为空；加载时去除无效／重复成员及空组，一位成员最多属于一组。成员显示顺序和代表头像依据 sessions 顺序，members 不持有运行进程、历史正文或凭据。关闭最后成员会删除空组；移出／解散分组仅修改导航元数据。普通保存和异步退出保存均保留 groups。

0.11.0-preview.3 新增 ui.floating_border，默认 true，旧配置缺失时同样开启。开关与浮窗背景透明度、主窗／浮窗缩放分别保存。0.11.0-preview.4 工牌只显示组名，统一白底黑字，不新增配色或角色字段；首位由 sessions 顺序自然体现，不显示角色标签。floating_opacity 支持 0.0–1.0，默认及非有限值回退仍为 1.0；旧数值保留。工牌及边框不改变生命周期或任务状态。

0.11.0 README 原生截图新增中英文 session-groups.png／session-groups.en.png，来自 session_groups_probe 的固定三成员面板；其余十二张当前工作区、历史、差异、文件、浮窗和设置图同步刷新。全部使用正式组件及隔离样例，原样复制并逐文件登记版本、语言、尺寸与 SHA-256；历史六枚 Agent 头像的来源记录保留原版本。
