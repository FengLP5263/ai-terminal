# Windows 10 compatibility verification

[简体中文](windows10-compatibility.md) | English

2026-11-03, development and acceptance record for 0.4.3, based on 0.4.2. Development-stage status is preserved below; see the [changelog](../CHANGELOG.en.md) for release information. Test records do not imply every GitHub issue is closed.

## Findings and status

| Issue | Change and verification | Status |
| --- | --- | --- |
| [#11 Stale Claude npm entry](https://github.com/FengLP5263/ai-terminal/issues/11) | Inspect only literal targets in standard npm wrappers and continue discovery when missing. Custom scripts retain precedence. The original default entry failed on Windows 10; the new resolver found a working entry. Claude Code 2.1.316 and Codex 0.166.0 both exited successfully for `--version`. | Fix verified; no global PATH or CLI configuration changes. |
| [#12 Composer disappears at high zoom](https://github.com/FengLP5263/ai-terminal/issues/12) | A fullscreen response probe restores 151.13608% zoom and switches through 125%, 150%, and 151.13608%, checking final rows, viewport bounds, and captured pixels. Both the old and new console runtimes passed this case. | Original symptom not reliably reproduced; actual CLI regression testing remains necessary. This evidence does not establish resolution. |
| [#13 White caption under a light system theme](https://github.com/FengLP5263/ai-terminal/issues/13) | Each main window explicitly requests a dark non-client area and reasserts it after theme notifications. Initial and post-notification caption/button checks passed on Windows 10 with AppsUseLightTheme=1 and 125% display scaling. | Fix verified; system personalization is unchanged. |

## Native console comparison

On the same Windows 10 build 19045 with the same test EXE, the inbox ConPTY dropped ALT_SCREEN/mouse modes and the application-internal wheel test failed. Adding the pinned Microsoft-signed runtime made scrolling and left-drag selection pass. This confirms a console protocol compatibility problem; it is not a direct reproduction of the original #12 symptom.

The runtime is Microsoft.Windows.Console.ConPTY 1.25.260930003. See [assets/windows/conpty](../assets/windows/conpty/README.en.md) for provenance, hashes, minimum OS version, and the original MIT license. Builds and new-format ZIPs include `conpty.dll` and `OpenConsole.exe`, without installing services or replacing Windows files.

## Verification scope

- Governance, fmt, Clippy, 144 Rust tests, and 10 Python tests passed.
- Locally and on Windows 10: three-session native smoke, public CLI discovery, four-stage composer regression, native caption, application scrolling, and drag selection passed. Test PTYs exited.
- On Windows 10: sidebar checks with 30 synthetic sessions and floating-window minimize/restore/close/reopen checks passed.
- Complete local ZIP read-back verification passed, with manifest format_version=2, dirty=true, and published=false. Executable version remains 0.4.2; this is not a new official release.
- No real model requests were sent. CLI version queries, synthetic fullscreen responses, synthetic status events, and real model tasks must be described separately. These checks do not establish full compatibility for long, multi-turn Agent sessions.
- Local screenshots were reviewed. Windows 10 native PNGs passed pixel checks; caption and 151.13608%-zoom composer regions were retrieved, SHA-256 verified, and visually reviewed. Full originals remain in the isolated remote directory. All test binary processes exited, and both application/system light-theme settings remain 1.

UU previously returned `environment_check_failed (code 6)`. Remote commands recovered after the user cleared historical terminal sessions and created a new one. The additional test session exited after use, leaving the user's new session intact. This involves UU terminal session state; the generic error alone does not identify the controlling or controlled endpoint as responsible. The screenshot retrieval harness also corrected long-command input and echo parsing; these are not product code changes.

Native evidence remains in ignored `output/` and an isolated remote temporary directory. User paths, conversations, credentials, and screenshots are not committed. See the [script index](../scripts/README.en.md) for rerun commands. Follow-up for #12 should cover consecutive turns and maximization using the real CLI with loopback fake responses, recording composer and terminal geometry without automatically accepting project or hook trust.

## Local 0.4.3 preview

After the user confirmed basic functionality in the Windows 10 preview above, the package manifest and lock file were updated to 0.4.3 and the application was rebuilt and launched locally. Governance, fmt, Clippy, 144 Rust tests, 10 Python tests, and three-session native smoke passed. The local preview archive passed read-back verification; its window title is `ai-terminal 0.4.3` and it remains responsive after the launch command exits.

The preview uses isolated session data and a launcher under `output/previews/0.4.3-<timestamp>/`. The running 0.4.2 instance and its configuration are preserved. That preview's manifest recorded dirty=true and published=false; launching it did not create a tag or remote release. Windows 10 verification used the same fixes before the version-number change. Basic acceptance does not replace targeted reproduction of the original #12 conditions. The user subsequently reported no significant issues in the local 0.4.3 trial and authorized release; formal artifacts must be rebuilt from the clean PR-merged commit.
