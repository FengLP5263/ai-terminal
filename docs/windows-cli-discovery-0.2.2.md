# 0.2.2 Windows CLI 发现修复

简体中文 | [English](windows-cli-discovery-0.2.2.en.md)

0.2.1 未嵌入开发者个人路径，但只搜索 codex.exe／claude.exe，并依赖一个固定的 npm 内部二进制布局。
这不能覆盖通常暴露为 codex.cmd 的安装、自定义 npm prefix、其他包管理器启动器，以及桌面进程持有旧 PATH 的情况。
用户报告的具体设备未直接检查；这里记录已确认的代码缺陷与隔离复现，不将某一种安装方式认定为其唯一原因。

## 修复

- 读取当前 PATH 和 Windows 注册表中保存的系统／用户 PATH，保留已有目录优先级。
- 查找公开 exe/cmd/bat 入口和用户安装目录，移除对 npm 私有 vendor 目录的依赖。
- 启动器及 Node 使用同一份子进程 PATH，不更改用户或管理器环境，不执行 profile，不扫描磁盘。
- cmd/bat 通过轻量 helper 的 --launch-cli 模式启动，交给 Rust Command 处理批处理转义。保留真实终端输入输出、退出码和 Job Object 所有权。
- helper 使用 console subsystem 继承已有终端，hook 仍走原来的元数据协议，不初始化字体或 GPU；两个 EXE 的分发结构保持不变。
- 找不到 CLI 时说明查找范围、自定义目录的处理方法及 WSL 边界。

## 隔离验证

- 不同用户根目录、自定义 prefix、原生 exe 与 cmd 的目录优先级、持久 PATH 补充和空／相对目录过滤。
- 自定义中文、空格、单引号、&、%、! 目录中的真实 cmd，复杂 hook 参数、环境变量字符和引号保持原样，子进程退出码 37 原样透传。
- 真 ConPTY 启动 cmd → 原生测试程序，接收中文输入，并回收测试自己拥有的进程树。
- 本机真实 Codex 的公开 npm 启动入口接受会话级 hook 参数，features list 正常；使用隔离 CODEX_HOME，不调用模型，不操作信任或认证。

## 最终验证与本地制品（2026-10-30）

- 治理检查、fmt、Clippy -D warnings、51 项 Rust 与 3 项 Python 测试通过，共 54 项。
- 优化版真实 Codex：output/codex-v022-trusted-20261030-182148；从公开 npm 入口经 helper 启动，中文输入可见，PNG 已查看，正常退出且自身 PTY 已回收；没有 Enter／模型请求。
- 较早探针对未信任的测试目录捕获到空白或未就绪状态，不计入通过。只读确认已有信任后，在已信任目录核查；未自动确认或修改 CLI 的信任。探针只有输入内容实际可见时才采集通过截图。
- 优化版原生 smoke：output/smoke-20261030-182211-115；真实 PowerShell、合成状态、截图、正常退出和自身 PTY 回收通过。
- 本地 ZIP：output/packages/ai-terminal-v0.2.2-windows-x64-20261030T102221Z-1a7291.zip，317 个成员，回读校验通过。
  SHA-256：3e13c96e7dc1007740cfea1f5d66e223514724eff245e3476692aefe121e566d。
- 主程序 8,908,288 字节，helper 292,352 字节；helper 仍不初始化字体或 GPU。
- 测试包构建时，修复位于 fix/windows-cli-discovery，工作区尚未提交；包中 dirty=true 记录构建时状态，后续 Git 提交不改变该包的身份记录。本记录不代表已创建 0.2.2 标签或远端发行；app-v0.2.1 标签及发行附件保持原样。


## 参考

- [npm 文件布局与 prefix](https://docs.npmjs.com/cli/v11/configuring-npm/folders/)
- [Windows 环境继承](https://learn.microsoft.com/en-us/windows/win32/procthread/environment-variables)
- [Rust Command 的 Windows 参数说明](https://doc.rust-lang.org/std/process/struct.Command.html)

生产源码身份：`95a7f86505fec9dcfb1b045b6e54e3627dfa271730025e45a590347e04d60965`。
