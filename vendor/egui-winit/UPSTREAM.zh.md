# egui-winit 本地补丁

[English](UPSTREAM.md) | 简体中文

上游：https://github.com/emilk/egui/tree/6f15dc0e16b26edce1fc2a05212eaf7e749c1d05/crates/egui-winit

版本：0.35.0；registry 归档 SHA-256：`9ea6bf3608db949588b95b8b341ee358d0c3f95cf4dc3f53d8d76717edee87db`。

许可：MIT OR Apache-2.0；包含两种上游许可证原文。

唯一行为变化：剪贴板没有文本时仍保留 `Event::Paste` 请求，使原生终端控件可让前台 CLI 处理图片粘贴。文本粘贴仍将 CRLF 标准化。此补丁不写入或持久化剪贴板内容。

规范化的 manifest 包含本地许可证文件以便再分发。`Cargo.toml.orig` 是未更改的上游 manifest。
