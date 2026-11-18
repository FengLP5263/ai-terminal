# 设置、窗口和浮窗

简体中文 | [English](settings.en.md)

这些命令要求 0.14.0-preview.2 或更新的管理器。先运行 `instance capabilities`，确认 `capabilities.native_controls=true`。新 CLI 对较旧管理器返回 unsupported_command；不要因为设置失败重建会话。

```powershell
ai-terminal-cli settings show
ai-terminal-cli settings fonts
ai-terminal-cli floating set --enabled true --transparency 50 --border true
ai-terminal-cli floating set --scale 142
ai-terminal-cli floating center
ai-terminal-cli sidebar hide
ai-terminal-cli sidebar show
ai-terminal-cli sidebar set --width 220
ai-terminal-cli settings set --language zh_cn --font jetbrains_mono --scale 114
```

百分数均为人类百分数：透明度 50 表示背景半透明；0 完全不透明、100 背景全透明，头像仍不透明。主窗口与浮窗的缩放独立，均为 50–250%。侧栏宽度为 190–420 egui 点。`floating show/hide/status` 分别开启、关闭、查询；`floating set --x X --y Y` 用屏幕物理像素，必须成对指定，程序会约束到可用屏幕。

`settings set` 还支持 `--sidebar-collapsed true|false`、`--sidebar-width`、`--floating-enabled`、`--floating-border`、`--floating-scale`、`--floating-transparency`。布尔值明确填 true/false；不使用 `--force` 代替布尔值。语言 ID 为 system、zh_cn、en；字体 ID 见 fonts，缺失的系统字体明确失败。

可使用 `settings set --file patch.json`，命令行字段覆盖同名文件字段。文件最多 16 KiB，UTF-8／UTF-8 BOM；只修改提供的字段，未知字段拒绝。字段示例：

```json
{"floating_enabled":true,"floating_transparency_percent":50,"floating_border":true,"ui_scale_percent":114,"terminal_font":"jetbrains_mono","language":"zh_cn"}
```

其他字段为 sidebar_collapsed、sidebar_width、floating_scale_percent、floating_position_pixels（双元素数组）、floating_center；后两者互斥。整份补丁校验、保存成功后立即应用，保留会话与进程。`settings reset` 重置全部界面偏好，应只在用户要求恢复默认时使用；不会删除会话或重做桌面快捷方式。

```powershell
ai-terminal-cli window show
ai-terminal-cli window focus
ai-terminal-cli window minimize
ai-terminal-cli window maximize
ai-terminal-cli window restore
ai-terminal-cli window set --width 1100 --height 720
ai-terminal-cli window set --x 100 --y 100
ai-terminal-cli panel open files
ai-terminal-cli panel open history
ai-terminal-cli panel close git
ai-terminal-cli panel open settings
```

窗口尺寸／位置为 egui 点，宽高和坐标各自必须成对指定；窗口命令返回 requested，需回读 window show 核实系统实际结果。支持的面板为 files、git、history、changes、settings；文件和 Git 侧栏互斥，关闭侧栏不关闭中央文档。设置在最小化时仍可执行，无需恢复窗口或发送鼠标。

`window capture --output <绝对路径.png>` 只截取目标管理器自己的主窗口，异步返回 pending；用 `window capture-status` 等待 complete／failed。父目录必须存在，已有文件不会覆盖。主窗口最小化可能超时，恢复后重试。截图是用户显式请求的制品，可能含当前终端文字；不要默认截图用户工作区。`app quit --force` 关闭目标管理器及其拥有的进程；不要关闭用户未要求关闭的实例。
