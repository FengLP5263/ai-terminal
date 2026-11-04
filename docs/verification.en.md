# 0.1.0 local preview verification

[简体中文](verification.md) | English

2026-10-30. Windows x64, Rust 1.99.1 MSVC, egui/eframe 0.35.0, Alacritty 0.26.0, default glow/OpenGL. The repository had not yet received its first commit; current Git state must be checked separately.

## Completed checks

`cargo fmt --all -- --check`, all-target Clippy with warnings denied, 26 workspace tests, `cargo build --locked --release`, and optimized native smoke passed. The screenshot was reviewed for mixed Chinese line spacing. After smoke, its three managed PowerShell PIDs 52400 / 59520 / 43348 had exited.

The 26 tests comprised 15 core, one background app logic, one native hook process, one installed Codex configuration parser, one real ConPTY, and seven terminal widget tests. The background logic test used real CMD PTY and authenticated status messages, verifying completed/unread state while minimized/unfocused without UI painting; a long-running dev service did not keep the AI spinner active.

## Real Agent integration scope

Claude Code 2.1.314 completed one real model turn using existing user access with tools disabled, producing SessionStart → UserPromptSubmit → Stop → SessionEnd in 8.281 s. User settings, credentials and trust were unchanged. The script's per-run budget limit was $0.15; it is not default CI.

Codex 0.164.0 accepted generated session-scoped hook TOML in isolated `CODEX_HOME` without login or writing `auth.json`. First use still requires user review in `/hooks`; no trust was bypassed and parsing was not called a real model turn. Native smoke screenshots used six real PowerShell terminals and **synthetic lifecycle events** (covering all six recognized Agent avatars), so they cannot prove full real-Agent interaction compatibility.

## Measurements on this device

| Configuration | Empty-window manager resident memory | Three real terminals, manager resident memory |
| --- | ---: | ---: |
| Final OpenGL release | 89.6 MiB | 115.0 MiB |
| OpenGL debug comparison | 151.2 MiB | 117.4 MiB |
| Initial DX12 debug comparison | 612.1 MiB | 721.2 MiB |

Only the manager process was counted, excluding child terminals and Agents; these were not long-term peaks. OpenGL used Quadro RTX 4000 / OpenGL 3.3 / NVIDIA 573.91 and DX12 used Intel UHD 630. This was a local configuration choice, not a same-GPU backend comparison or universal performance claim.

Final release empty window: 8 frames in about 8.03 s, UI callback CPU 14.5 ms; first UI callback at about 2.678 s (100 ms external sampling, not strict first displayed frame). Five seconds after first UI, manager CPU totaled 0.234375 s on a 16-logical-processor machine. The short window may include startup/input and is not a zero-CPU steady-state claim. The two optimized program files totaled 7,702,528 bytes (7.35 MiB). Preview `output/preview/ai-terminal.exe` required the helper in the same directory; `build.json` recorded checksums.

## Evidence and reproduction

- UI: `output/smoke-20261030-125909/smoke.json`, `smoke.png`.
- Cost: `output/idle-final-opengl-20261030-125923-473/idle.json`, `process.json`.
- Real Claude: `output/live-claude-20261030-122056/result.json`.
- Repeat with `scripts/smoke.ps1 -Release` and `scripts/measure-idle.ps1 -Release`.

`output` is ignored and can be regenerated. Real Claude uses the existing model service; default tests do not. All physical Chinese IME combinations, image pasting, and long-running many-session use remained unverified.

## Normal startup check

The interactive preview opened with title `ai-terminal`, main window handle 31852918, and a responsive process. An additional isolated start obtained a valid window and first-UI marker. A restricted inspection process had reported a zero handle; checking from the same desktop confirmed the window without source changes. The extra preview was closed normally, leaving one. The native window screenshot channel was unavailable; visual validation used the app's own native smoke screenshot. Local evidence: `output/preview/window-check.json`.

## Unreleased: Codex takeover verification (2026-11-04)

- Independent CI passed: 181 Rust and 14 Python tests, including governance, fmt and Clippy. Seven native-text regressions include two new retirement checks: same-frame shapes keep valid textures, and the next frame emits a texture free even on a cache hit.
- The optimized build and six-session native smoke passed; owned PTYs were reaped. Expanded-sidebar captures at 100% and 146% show all three new icons, Chinese and emoji. Collapsed-sidebar smoke also passed at 200%, but its viewport shows only the start of the list; that capture does not establish visibility of all three lower icons.
- The three assets were verified against the official Kimi Code documentation repository, ZCode README and TRAE CLI documentation. Production sidebar icons use the same bounded cache, with no runtime downloads; SOURCE.json records provenance, license scope and original/delivered identities per file.
- Logs and records are ignored under output/codex-takeover-20261104; the standard native capture is output/smoke-20261104-223847-661.
- No model requests were sent. Real CLI interaction, actual first-run Desktop creation, the user's previous dynamic flicker and sustained load require separate acceptance. A generic DSR roundtrip does not prove the cause of real Qoder input residue.

## 0.4.7 release verification supplement

Public version queries passed for Kimi Code 2.5.0, TRAE CLI 0.213.1 public, and community zcode-app-cli 3.17.2-44 / runtime 0.17.1. Native startup captures using production App components show the correct avatars and "Awaiting task events": Kimi stops at directory trust, community ZCode shows its input interface, and TRAE shows login. Focusing the empty terminal and cancelling with Esc/Ctrl+C returned all three to PowerShell and restored its avatar. No trust approval, login or model prompt was submitted. Early Kimi captures without a focus click did not establish exit; final captures after explicit focus were reviewed.

Evidence is ignored under output/release-0.4.7-20261104-232823: installed-clis contains startup captures and ZCode/TRAE return captures; kimi-focus-cancel contains the final Kimi return capture. Six README badges were natively rendered by the production painter, inspected on light/dark surfaces and copied unchanged, with provenance and hashes in assets/readme/SOURCE.json. Actual first-time Desktop creation, real model tasks, Windows 10 and sustained load are outside these new checks; offline regressions and startup captures do not replace those acceptance steps.

Final 0.4.7 gates passed: 181 Rust / 14 Python, governance/fmt/Clippy, optimized build and six-owned-PTY native smoke (output/smoke-20261104-235553-750), with the screenshot reviewed. A pre-existing native lifecycle test incorrectly assumed no producer progress between two drains of the same invocation wake; the replay assertion now runs after the final Done state, while the complete ordered sequence is still required. Its native target passed three consecutive runs before full CI passed.
