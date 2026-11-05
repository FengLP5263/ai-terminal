# egui_glow 本地补丁

简体中文 | [English](UPSTREAM.md)

上游：https://github.com/emilk/egui/tree/6f15dc0e16b26edce1fc2a05212eaf7e749c1d05/crates/egui_glow
版本：0.35.0；registry 压缩包 SHA-256：52274b9bfb8d8e252cd0f00c9f6214f360d75a0962baa77a2d62ddb002fe99d4。
许可证：MIT OR Apache-2.0；已附两份上游许可证。

Windows 上复用顶点和索引缓冲区，并在需要时扩容，避免每个网格都重新分配。实测环境中的 OpenGL 驱动在反复绘制原生字形时会持续增加进程私有内存。绘制顺序、纹理和顶点内容保持不变；其他平台继续使用上游上传方式，超大缓冲区也回退到上游方式。

规范化清单改为包含本地许可证以便再分发；Cargo.toml.orig 保留上游原始清单。

0.4.9-preview.5：Windows 将本次可见绘制的网格整理到可复用 CPU 暂存，跨网格重定位索引，在任何 draw 之前各上传一次 GPU 缓冲区；绘制范围保持纹理切换、裁剪与回调顺序。避免 draw 之间重复用 glBufferSubData 覆盖相同区域时观测到的瞬时字形拉扯，同时保留按需扩容，不恢复逐网格替换存储。超大帧保留原逐网格回退，其他平台保持上游实现。固定表格原生像素探针在旧路径捕获一帧错绘，对应新路径 48 次采样无像素差异。
