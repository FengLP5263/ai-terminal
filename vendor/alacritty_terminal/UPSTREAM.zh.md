# alacritty_terminal 来源与本地补丁

简体中文 | [English](UPSTREAM.md)

来自 crates.io 发布归档 `alacritty_terminal-0.26.0.crate`，仓库：https://github.com/alacritty/alacritty，VCS 提交 `94e7c8874e526b1e67b349d9ba30ddf81669119e`。
归档 SHA-256：`bda177466b9524d59f1b12f0dd30b68696788e9992a7e959021c4a0ed96fcf59`。原始许可证 `Apache-2.0` 及发布包内容保留；不修改本机 Cargo 缓存。

0.4.9 预览：为 CSI ? 6 n 返回 CSI ? 行;列 R；普通 CSI 6 n 的返回保持 CSI 行;列 R。按 [Xterm 控制序列](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html) 保留私有前缀，不扫描或重写终端正文。上游单元测试和文档保持原样。
Windows 原生 ConPTY 回归分别验证普通／私有查询多次应答的数量、坐标和格式；它不等于真实 Agent 长时间交互验收。

Git 属性保留已发布 vendor 字节，不转换换行；终端录制样例按二进制比较。原始许可／样例的空白不为项目源码空白检查而改写。
