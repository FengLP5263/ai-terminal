# alacritty_terminal upstream and local patches

[简体中文](UPSTREAM.zh.md) | English

Source: crates.io published archive `alacritty_terminal-0.26.0.crate`; repository: https://github.com/alacritty/alacritty; VCS revision `94e7c8874e526b1e67b349d9ba30ddf81669119e`.
Archive SHA-256: `bda177466b9524d59f1b12f0dd30b68696788e9992a7e959021c4a0ed96fcf59`. Preserve the original `Apache-2.0` license and published contents; do not modify the machine Cargo cache.

0.4.9 preview: Reply to CSI ? 6 n with CSI ? row;column R; ordinary CSI 6 n still returns CSI row;column R. Preserve the private prefix defined by [Xterm control sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html), without scanning or rewriting terminal text. Original upstream tests and documentation remain unchanged.
Native Windows ConPTY regressions separately check repeated ordinary/private responses, coordinates and format; this is not long-running real Agent acceptance.

Git attributes preserve published vendor bytes without EOL conversion; terminal recording fixtures are binary diffs. Original legal/fixture whitespace is not rewritten to satisfy project-source whitespace checks.

Unreleased terminal resize fix: on column changes, normal-buffer rows with a solid padded background receive a transient paint hint, including TUIs reserving the final column. One colored blank remains as an anchor; the remaining presentation padding becomes ordinary empty cells so narrowing does not wrap it into extra blank rows. egui_term extends only that background to the current terminal edge. Text, hard breaks, soft-wrap semantics, cursor coordinates and PTY identity remain intact. Inline highlights, mixed backgrounds, underlines and alternate screens do not expand; explicitly erasing or overwriting the leading cell clears the hint. No persistent settings, history copies, polling or input injection are added; the existing 5,000-line bound and resize events remain.

Windows ConPTY pipes re-arm readiness after each event batch. Buffered output posts another notification; an empty pipe atomically checks readiness and registers its waker, preventing a read-budget yield from losing the next output wake. Waker registration holds no lock acquired by its callback. Existing parse budgets, bounded pipes, 5,000-line history and hidden output draining remain; there is no idle polling or synthetic key input.
