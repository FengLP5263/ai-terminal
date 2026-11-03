use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=assets/branding/ai-terminal.ico");
    println!("cargo:rerun-if-changed=build.rs");
    for name in [
        "RC",
        "WindowsSdkDir",
        "WindowsSDKVersion",
        "ProgramFiles(x86)",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    stage_native_console(&root, &out);
    let icon = root
        .join("assets/branding/ai-terminal.ico")
        .to_string_lossy()
        .replace('\\', "/");
    let resource = out.join("ai-terminal-icon.rc");
    fs::write(
        &resource,
        format!("#pragma code_page(65001)\n1 ICON \"{icon}\"\n"),
    )
    .expect("write icon resource");
    let compiled = out.join("ai-terminal-icon.res");
    let compiler = resource_compiler();
    let output = Command::new(&compiler)
        .arg("/nologo")
        .arg("/fo")
        .arg(&compiled)
        .arg(&resource)
        .output()
        .unwrap_or_else(|error| {
            panic!(
                "Windows SDK resource compiler {}: {error}",
                compiler.display()
            )
        });
    assert!(
        output.status.success(),
        "Windows icon resource compilation failed: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // Main app and developer examples share the approved icon; the helper stays independent.
    println!(
        "cargo:rustc-link-arg-bin=ai-terminal={}",
        compiled.display()
    );
    println!("cargo:rustc-link-arg-examples={}", compiled.display());
}

fn stage_native_console(root: &std::path::Path, out: &std::path::Path) {
    assert_eq!(
        env::var("CARGO_CFG_TARGET_ARCH").as_deref(),
        Ok("x86_64"),
        "The bundled native console currently supports Windows x64 only"
    );
    let profile = out.ancestors().nth(3).expect("Cargo profile directory");
    for name in ["conpty.dll", "OpenConsole.exe"] {
        let source = root.join("assets/windows/conpty/x64").join(name);
        println!("cargo:rerun-if-changed={}", source.display());
        let bytes = fs::read(&source).expect("pinned native console asset");
        for folder in [
            profile.to_path_buf(),
            profile.join("examples"),
            profile.join("deps"),
        ] {
            fs::create_dir_all(&folder).expect("native output directory");
            let target = folder.join(name);
            println!("cargo:rerun-if-changed={}", target.display());
            if fs::read(&target).ok().as_deref() != Some(bytes.as_slice()) {
                fs::copy(&source, &target).unwrap_or_else(|error| panic!("Cannot stage {}: {error}. Use a separate CARGO_TARGET_DIR if an existing preview owns this file.", target.display()));
            }
        }
    }
}

fn resource_compiler() -> PathBuf {
    if let Some(path) = env::var_os("RC") {
        return PathBuf::from(path);
    }
    if Command::new("rc.exe").arg("/?").output().is_ok() {
        return PathBuf::from("rc.exe");
    }
    let mut roots = Vec::new();
    if let Some(path) = env::var_os("WindowsSdkDir") {
        roots.push(PathBuf::from(path).join("bin"));
    }
    if let Some(path) = env::var_os("ProgramFiles(x86)") {
        roots.push(PathBuf::from(path).join("Windows Kits/10/bin"));
    }
    for root in roots {
        let mut versions: Vec<_> = fs::read_dir(&root)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        versions.sort_by_key(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .split('.')
                .map(|number| number.parse::<u32>().unwrap_or(0))
                .collect::<Vec<_>>()
        });
        for version in versions.into_iter().rev() {
            let path = version.join("x64/rc.exe");
            if path.is_file() {
                return path;
            }
        }
        let path = root.join("x64/rc.exe");
        if path.is_file() {
            return path;
        }
    }
    panic!(
        "Windows SDK rc.exe is required; install the Windows SDK or set RC to its executable path"
    );
}
