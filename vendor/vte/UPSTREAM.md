# vte upstream and local patches

[简体中文](UPSTREAM.zh.md) | English

Source: crates.io published archive `vte-0.15.0.crate`; repository: https://github.com/alacritty/vte; VCS revision `3b3da71c34cc1256c7e20981cf03f8eb95e08ffc`.
Archive SHA-256: `a5924018406ce0063cd67f8e008104968b74b563ee1b85dde3ed1f7cb87d3dbd`. Preserve the original `Apache-2.0 OR MIT` license and published contents; do not modify the machine Cargo cache.

0.4.9 preview: Add a default private device-status Handler method and route CSI ? n to it. Preserve the private prefix defined by [Xterm control sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html), without scanning or rewriting terminal text. Original upstream tests and documentation remain unchanged.
Native Windows ConPTY regressions separately check repeated ordinary/private responses, coordinates and format; this is not long-running real Agent acceptance.

Git attributes preserve published vendor bytes without EOL conversion; terminal recording fixtures are binary diffs. Original legal/fixture whitespace is not rewritten to satisfy project-source whitespace checks.
