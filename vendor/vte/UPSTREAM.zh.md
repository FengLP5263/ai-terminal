# vte 来源与本地补丁

简体中文 | [English](UPSTREAM.md)

来自 crates.io 发布归档 `vte-0.15.0.crate`，仓库：https://github.com/alacritty/vte，VCS 提交 `3b3da71c34cc1256c7e20981cf03f8eb95e08ffc`。
归档 SHA-256：`a5924018406ce0063cd67f8e008104968b74b563ee1b85dde3ed1f7cb87d3dbd`。原始许可证 `Apache-2.0 OR MIT` 及发布包内容保留；不修改本机 Cargo 缓存。

0.4.9 预览：为 ANSI Handler 新增默认私有状态查询入口，将 CSI ? n 路由到该入口。按 [Xterm 控制序列](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html) 保留私有前缀，不扫描或重写终端正文。上游单元测试和文档保持原样。
Windows 原生 ConPTY 回归分别验证普通／私有查询多次应答的数量、坐标和格式；它不等于真实 Agent 长时间交互验收。

Git 属性保留已发布 vendor 字节，不转换换行；终端录制样例按二进制比较。原始许可／样例的空白不为项目源码空白检查而改写。
