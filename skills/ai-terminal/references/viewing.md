# 文件、Git 与媒体查看

简体中文 | [English](viewing.en.md)

要求 native_controls 能力。这些命令对应界面当前选中的会话／项目；先 `workspace show` 核对 active，必要时显式 `session activate <UUID>`。`current` 的调用者含义保持不变，不会暗中决定文件命令的项目。

```powershell
ai-terminal-cli project show
ai-terminal-cli file list --path src --offset 0 --limit 100
ai-terminal-cli file expand --path src
ai-terminal-cli file collapse --path src
ai-terminal-cli file collapse-all
ai-terminal-cli file refresh
ai-terminal-cli file open --path README.md
ai-terminal-cli file show
ai-terminal-cli file read --offset 0 --limit 100
ai-terminal-cli file source
ai-terminal-cli file preview
ai-terminal-cli file next
ai-terminal-cli file previous
ai-terminal-cli file first
ai-terminal-cli file reload
ai-terminal-cli file close
```

文件路径相对 project show 返回的 root；也接受该根内的绝对路径。拒绝 `..`、项目外路径和链接／junction；不会执行查看的脚本。open／refresh／翻页只是排队，show／list 返回 loading=true 时稍后查询；不要因尚未加载完成重复创建对象。读取失败查看 error；加载完成不等于文件成功。

list 每级最多缓存 1,000 项、最多 64 个目录，响应携带 partial 和 next_offset；未展示完整时不能声称已枚举整个项目。file read 只返回当前有界文本页的 lines，--offset/--limit 为页内行下标，next_offset 用于响应分页；单次最多 200 项、序列化项约 256 KiB，按返回值继续。更大文件用 next/previous；show 给出字节偏移、首行、总字节和前后页标记。不要整文件连续读取来轮询状态。source/preview 仅切换 Markdown 源码与渲染。

```powershell
ai-terminal-cli image fit
ai-terminal-cli image original
ai-terminal-cli image zoom --percent 200
ai-terminal-cli media status
ai-terminal-cli media volume --percent 0
ai-terminal-cli media play
ai-terminal-cli media pause
ai-terminal-cli media seek --seconds 1.5
```

先打开相应图片或音视频文件。图片缩放 5–800%；媒体默认暂停，duration_seconds 尚为 null 时等待就绪。volume 为 0–100，seek 不能越过时长。媒体来自已验证的本地文件，复用 Windows 原生播放器；不接入远程 URL、播放列表或浏览器，解码器缺失明确报错。关闭文件或切换会话会释放播放资源。支持会话预览恢复的版本在切回时从保存的进度暂停，恢复期间 `media status` 可返回 `loading=true`。

```powershell
ai-terminal-cli git status
ai-terminal-cli git files --offset 0 --limit 100
ai-terminal-cli git history --offset 0 --limit 50
ai-terminal-cli git more
ai-terminal-cli git inspect --commit <完整SHA>
ai-terminal-cli git commit
ai-terminal-cli git open-diff --path src/main.rs
ai-terminal-cli git open-diff --path src/main.rs --commit <完整SHA>
ai-terminal-cli git diff --offset 0 --limit 100
ai-terminal-cli git close-diff
ai-terminal-cli git refresh
```

只读共享现有 Git 后台快照；不会提交、切换分支、写索引或访问远端。SHA 必须完整 40／64 位十六进制。inspect/open-diff 异步返回 queued，查询 commit/diff 的 loading 和 error；没有待查看对象时不会虚构数据。history 的 next_offset 是当前快照分页；more_history=true 时可显式 more 请求下一批历史。partial、omitted_files 等保留现有界面限额。提交正文超过 32,768 字符时 message_truncated=true；查看原始 Git 仍用 Git 自身命令。非 Git 目录用 not_repository 区分，不当作空仓库。
