# 0.2.1 全窗口缩放与 Windows 原生文字

简体中文 | [English](visual-quality-0.2.1.en.md)

用户验收指出 0.2.0 只缩放终端文字，侧栏没有同步，字体、颜色与 emoji 仍不足。
0.2.0 不再作为本轮视觉验收结论；本轮按同一组内容检查实际 GPU 画面。

## 行为与实现

Ctrl＋滚轮在 raw_input_hook 中消费，在窗口任何内容位置均调整 egui 全窗口比例；
Ctrl＋加减同样可用，Ctrl＋0／标题栏百分比恢复 100%。侧栏、控件、标题栏、正文共同缩放。
比例范围 50%–250%，保存为 ui.ui_scale；旧 terminal_font_size 字段仅保留兼容，不再单独改变正文。
终端继续占满侧栏右边的内容区，保留 12 逻辑像素边距，按剩余尺寸与真实字符度量更新 ConPTY 行列。

Windows 文字使用 DirectWrite 布局与 Direct2D/WIC 栅格化，启用系统彩色字体。
Hack 四种字体通过进程内 font set 加载，不安装到 Windows；中文指定 Microsoft YaHei UI，
emoji 使用系统 fallback。采用灰度抗锯齿避免跨显示器 RGB 彩边，字号、位置与单元格按物理像素对齐。
原始 ANSI／truecolor 字值保留；透明度按实际背景合成，不把彩色 emoji 统一染成文本前景色。

Unicode grapheme 合成处理肤色与 ZWJ 序列；保留 Alacritty 传入的原始格坐标，
不宣称更改了模拟器的 Unicode 宽度算法。VS16 爱心在紧随空格时使用完整两格画面。
系统未包含的 emoji 仍受已安装字体版本限制；不把 fallback 当成完整 Unicode 一致性保证。
原生图块和实际光标背景一致，插入／空心光标最后绘制，避免被不透明文字图块覆盖。

缓存每个上下文最多 8 张 1024² atlas、64 项格式／度量，线程最多保留两个上下文；
同一字形只在缓存未命中时栅格化，后台没有新增轮询。独立 hook helper 不初始化原生文字或 GPU。
TextRendererSession 在 on_exit 显式释放，再拆卸窗口和 COM，Drop 作为补充保护。

## 验证

- 治理、fmt、Clippy -D warnings、46 项 Rust 和 3 项 Python 检查通过。
- 75/100/150/200% 全窗口布局检查：侧栏物理尺寸同步变化，终端贴合侧栏和右／下边界，PID 不变。
- 真实 ConPTY 中肤色、VS16、ZWJ／区域序列合成，彩色像素、粗体、光标遮挡、IME 和保存偏好回归通过。
- 原生 debug 画面：output/ui-v021-clean-20261030-164113，三个 PNG 已查看；
  比例从 100% 变为 146.41%，全部控件同步，正常退出码 0，测试 PTY 已退出。
- 较早原生探针捕获完截图后曾发生 0xc000000d 退出异常；该结果不算通过。
  已增加 on_exit 原生资源释放，不依赖 Windows 线程 TLS 析构顺序。

Windows 彩色字体 API 依据 [Microsoft Color font support](https://learn.microsoft.com/en-us/windows/win32/directwrite/color-fonts)。
此实现不复用 Warp 私有代码，不宣称与 Warp 全部字体策略或终端协议一致。
最终优化版的 smoke、真实 TUI、截图和包身份记录在本文件后续条目。

## 最终优化版交付（2026-10-30）

- output/ui-v021-settled-20261030-170142：展开、146.41% 整体缩放、收起三种 GPU 画面均已查看；退出码 0，PTY 已回收。
  较早 output/ui-v021-final-20261030-165218/zoom.png 捕获了 ConPTY 重排过渡帧，不用作视觉通过证据。
  探针改为每次布局变化后至少经过三帧和一秒，并要求样例内容已就绪；正式程序未为截图改变绘制语义。
- output/claude-v021-final-20261030-165218：真实 Claude，中文可见，402 个非默认前景字符。
- output/codex-v021-final-20261030-165218：真实 Codex，中文可见，48 个非默认前景字符。
  两个 TUI PNG 已查看，未发送 Enter／模型请求，退出码 0，测试 PTY 已回收。
- output/smoke-20261030-165245-513：三个真实 PowerShell、合成生命周期、截图与正常退出检查通过。
- output/idle-v021-20261030-165305-784：8.031 秒 11 次 UI，UI CPU 6.326ms；
  五秒主进程 CPU 0.125 秒，工作集约 105.1 MiB。此窗口为空会话测量，不能冒充重负载结果。
- ZIP：output/packages/ai-terminal-v0.2.1-windows-x64-20261030T090002Z-bcf389.zip，4,969,021 字节，317 成员，回读校验通过。
  SHA-256：56307381ee9eef0bbd6fe485ce54f7e89c674a45455d5ff44d345aa43d52e1c4。
  source_sha256：b99244863f01849370cf04ebadadddf3f7b41f25fd969cb97ef29a4925abf8fb；dirty=true，未发布。
- 已打开 output/preview-v0.2.1/ai-terminal.exe，PID 51748，窗口标题 ai-terminal 0.2.1。
  本次使用 output/preview-state-v021-20261030-170417 的独立元数据，保留旧窗口和任务。
  两个 exe 与已校验包逐一核对哈希一致；helper 仍为 197,120 字节，不初始化字体或 GPU。
- 以上为提交前的本地预览验收记录，分支为 feature/terminal-workspace-ui。上述 ZIP 的 dirty=true 反映其构建时状态，不代表后续提交／标签状态。
