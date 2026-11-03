# Native console runtime

[简体中文](README.md) | English

The application ships a pinned x64 runtime from Microsoft's official `Microsoft.Windows.Console.ConPTY` package. [SOURCE.json](SOURCE.json) records its origin, version, size, SHA-256 and signature check; [LICENSE.txt](LICENSE.txt) preserves the original license. These are runtime dependency assets, not this project's build output.

The inbox ConPTY on Windows 10 build 19045 drops fullscreen and mouse modes in this project's native fixture. Switching the same program to this runtime passes wheel paging and drag-selection checks. A consistent native console reduces Windows 10/11 differences without adding a browser runtime or modifying system files, PATH, user configuration or the default terminal.

Alacritty 0.26 loads `conpty.dll` from the application directory, and that DLL uses the adjacent `OpenConsole.exe`. `build.rs` copies both to Cargo's actual profile, examples and deps output directories, skipping identical content to avoid overwriting a running component. Packages carry the runtime, license and provenance; the two ai-terminal EXEs alone are incomplete. Packaging and governance verify pinned asset hashes.

For upgrades, select an exact official NuGet version, verify origin, signatures and license, update SOURCE.json, and test fullscreen composers, mouse, zoom, shutdown and native screenshots separately on Windows 10/11. Temporary tests do not establish compatibility with every CLI or IME combination.
