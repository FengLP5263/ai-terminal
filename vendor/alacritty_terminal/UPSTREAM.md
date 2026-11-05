# alacritty_terminal upstream and local patches

[简体中文](UPSTREAM.zh.md) | English

Source: crates.io published archive `alacritty_terminal-0.26.0.crate`; repository: https://github.com/alacritty/alacritty; VCS revision `94e7c8874e526b1e67b349d9ba30ddf81669119e`.
Archive SHA-256: `bda177466b9524d59f1b12f0dd30b68696788e9992a7e959021c4a0ed96fcf59`. Preserve the original `Apache-2.0` license and published contents; do not modify the machine Cargo cache.

0.4.9 preview: Reply to CSI ? 6 n with CSI ? row;column R; ordinary CSI 6 n still returns CSI row;column R. Preserve the private prefix defined by [Xterm control sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html), without scanning or rewriting terminal text. Original upstream tests and documentation remain unchanged.
Native Windows ConPTY regressions separately check repeated ordinary/private responses, coordinates and format; this is not long-running real Agent acceptance.

Git attributes preserve published vendor bytes without EOL conversion; terminal recording fixtures are binary diffs. Original legal/fixture whitespace is not rewritten to satisfy project-source whitespace checks.
