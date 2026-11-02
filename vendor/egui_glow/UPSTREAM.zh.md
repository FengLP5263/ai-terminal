# egui_glow 本地补丁

简体中文 | [English](UPSTREAM.md)

上游：https://github.com/emilk/egui/tree/6f15dc0e16b26edce1fc2a05212eaf7e749c1d05/crates/egui_glow
版本：0.35.0；registry 压缩包 SHA-256：52274b9bfb8d8e252cd0f00c9f6214f360d75a0962baa77a2d62ddb002fe99d4。
许可证：MIT OR Apache-2.0；已附两份上游许可证。

Windows 上复用顶点和索引缓冲区，并在需要时扩容，避免每个网格都重新分配。实测环境中的 OpenGL 驱动在反复绘制原生字形时会持续增加进程私有内存。绘制顺序、纹理和顶点内容保持不变；其他平台继续使用上游上传方式，超大缓冲区也回退到上游方式。

规范化清单改为包含本地许可证以便再分发；Cargo.toml.orig 保留上游原始清单。
