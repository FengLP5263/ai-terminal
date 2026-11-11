use ai_terminal::file_icons;
use std::collections::HashSet;

#[test]
fn common_languages_have_distinct_outline_assets_and_every_svg_renders() {
    let examples = [
        ("tool.py", "python", "brand-python.svg"),
        ("index.html", "html", "code.svg"),
        ("style.css", "css", "brand-css3.svg"),
        ("app.js", "javascript", "brand-javascript.svg"),
        ("app.ts", "typescript", "brand-typescript.svg"),
        ("app.rs", "rust", "brand-rust.svg"),
        ("app.go", "go", "brand-golang.svg"),
        ("main.c", "c", "letter-c.svg"),
        ("main.cpp", "cpp", "brand-cpp.svg"),
        ("Main.cs", "c-sharp", "brand-c-sharp.svg"),
        ("build.ps1", "powershell", "terminal.svg"),
        ("build.sh", "shell", "terminal-2.svg"),
        ("query.sql", "db", "database.svg"),
    ];
    let mut assets = HashSet::new();
    for (path, kind, asset) in examples {
        let icon = file_icons::icon(path, false);
        assert_eq!(icon.name, kind);
        assert_eq!(icon.asset, asset);
        assert!(assets.insert(icon.asset));
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/file-icons");
    let source: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("SOURCE.json")).unwrap()).unwrap();
    for name in source["files"]
        .as_object()
        .unwrap()
        .keys()
        .filter(|n| n.ends_with(".svg"))
    {
        let data = std::fs::read(root.join(name)).unwrap();
        let svg = String::from_utf8(data)
            .unwrap()
            .replace("currentColor", "#60a5fa");
        let tree = resvg::usvg::Tree::from_data(svg.as_bytes(), &Default::default()).unwrap();
        assert_eq!((tree.size().width(), tree.size().height()), (24.0, 24.0));
        let mut bitmap = resvg::tiny_skia::Pixmap::new(32, 32).unwrap();
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(32.0 / 24.0, 32.0 / 24.0),
            &mut bitmap.as_mut(),
        );
        assert!(
            bitmap.data().as_chunks::<4>().0.iter().any(|p| p[3] > 0),
            "Empty icon {name}"
        );
    }
}

#[test]
fn filename_rules_are_case_insensitive_and_take_precedence_over_generic_extensions() {
    for (path, kind) in [
        (r"C:\项目\.tmp_prepare.PY", "python"),
        (r"C:\项目\INDEX.HTML", "html"),
        ("package.json", "npm"),
        ("settings.json", "json"),
        ("Cargo.toml", "rust"),
        ("pyproject.toml", "python"),
        ("config.toml", "config"),
        ("Dockerfile.prod", "docker"),
        ("compose.yaml", "docker"),
        ("Makefile", "makefile"),
        ("CMakeLists.txt", "makefile"),
        ("requirements-dev.txt", "python"),
        (".gitignore", "git_ignore"),
        (".env.production.local", "config"),
        ("webpack.config.ts", "webpack"),
        ("eslint.config.js", "eslint"),
        ("tsconfig.json", "tsconfig"),
    ] {
        assert_eq!(file_icons::icon(path, false).name, kind, "{path}");
    }
    assert_eq!(file_icons::icon("component.jsx", false).name, "react");
    assert_ne!(
        file_icons::icon("component.jsx", false).color,
        file_icons::icon("component.tsx", false).color
    );
}
#[test]
fn document_media_and_unknown_types_keep_meaningful_safe_fallbacks() {
    for (path, kind) in [
        ("README.md", "markdown"),
        ("config.yaml", "yml"),
        ("photo.png", "image"),
        ("vector.svg", "svg"),
        ("sound.mp3", "audio"),
        ("clip.mp4", "video"),
        ("manual.pdf", "pdf"),
        ("archive.tar.gz", "zip"),
        ("table.csv", "csv"),
        ("table.xlsx", "xls"),
        ("document.docx", "word"),
        ("font.ttf", "font"),
        ("unknown.custom", "default"),
        ("no_extension", "default"),
    ] {
        assert_eq!(file_icons::icon(path, false).name, kind, "{path}");
    }
    assert_eq!(file_icons::icon("unknown.custom", true).name, "hex");
    assert_eq!(file_icons::icon("photo.png", true).name, "image");
}
