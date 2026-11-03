# winit 本地补丁

简体中文 | [English](UPSTREAM.md)

上游：https://github.com/rust-windowing/winit/tree/e9809ef54b18499bb4f2cac945719ecc2a61061b

版本：0.30.13。注册表归档 SHA-256：a6755fa58a9f8350bd1e472d4c3fcc25f824ec358933bba33306d0b63df5978d。

许可证：Apache-2.0，保留原始 LICENSE。本地快照省略注册表缓存标记及依赖独立的 Cargo.lock，以项目根锁文件为准。

注册表归档不含 CONTRIBUTING.md，因此 README 的贡献说明链接改为固定上游 revision；其他上游文档保持原样。

Windows 原生拖动期间直接采用 WM_DPICHANGED 提供的建议矩形。重新计算鼠标锚点并将窗口推向旧显示器，可能递归切换 DPI、拉伸窗口和闪烁。保留 ScaleFactorChanged 事件以及现有非拖动／最大化／全屏路径。依据：[上游问题 4600](https://github.com/rust-windowing/winit/issues/4600) 与 [Microsoft 说明](https://learn.microsoft.com/en-us/windows/win32/hidpi/wm-dpichanged)。这是本地兼容补丁，不声称已经合并至上游。

通过项目的双窗口原生探针，保持移动状态跨越真实 DPI 边界，比较建议／实际矩形、递归次数与松手后的尺寸。程序化移动不能代替真实鼠标拖动验收。上游测试／示例保留为源码参考，不声称已运行完整上游测试套件。
