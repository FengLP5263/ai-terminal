# 数据、字体与制品

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
- assets/icons：Lobe Icons 的 OpenAI／Claude PNG，MIT；SOURCE.json 固定下载 URL、日期、大小和 SHA-256，许可证随包携带。
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
