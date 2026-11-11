//! Shared outline file icons: original Tabler SVGs, tint only, rasterized at final physical size.
use egui::{Color32, Rect};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileIcon {
    pub name: &'static str,
    pub asset: &'static str,
    pub color: Color32,
    svg: &'static [u8],
}
const CACHE_SIZES: usize = 8;
struct Raster {
    tree: Option<resvg::usvg::Tree>,
    sizes: VecDeque<(usize, egui::TextureHandle)>,
}
impl FileIcon {
    fn tree(self) -> Option<resvg::usvg::Tree> {
        let [r, g, b, _] = self.color.to_array();
        let tint = format!("#{r:02x}{g:02x}{b:02x}");
        let svg = std::str::from_utf8(self.svg)
            .ok()?
            .replace("currentColor", &tint);
        resvg::usvg::Tree::from_data(svg.as_bytes(), &resvg::usvg::Options::default()).ok()
    }
    fn cache_id(self) -> egui::Id {
        egui::Id::new(("file-icon-raster", self.asset, self.color))
    }
    fn texture(self, ctx: &egui::Context, pixels: usize) -> Option<egui::TextureId> {
        let cache = ctx.data_mut(|d| {
            d.get_temp_mut_or_insert_with(self.cache_id(), || {
                Arc::new(Mutex::new(Raster {
                    tree: self.tree(),
                    sizes: VecDeque::new(),
                }))
            })
            .clone()
        });
        let mut cache = cache.lock().unwrap();
        if let Some((_, texture)) = cache.sizes.iter().find(|(size, _)| *size == pixels) {
            return Some(texture.id());
        }
        let tree = cache.tree.as_ref()?;
        let mut bitmap = resvg::tiny_skia::Pixmap::new(pixels as u32, pixels as u32)?;
        let scale = resvg::tiny_skia::Transform::from_scale(
            pixels as f32 / tree.size().width(),
            pixels as f32 / tree.size().height(),
        );
        resvg::render(tree, scale, &mut bitmap.as_mut());
        let image = egui::ColorImage::from_rgba_premultiplied([pixels, pixels], bitmap.data());
        let texture = ctx.load_texture(
            format!("file-{}-{pixels}", self.asset),
            image,
            egui::TextureOptions::LINEAR,
        );
        let id = texture.id();
        if cache.sizes.len() == CACHE_SIZES {
            cache.sizes.pop_front();
        }
        cache.sizes.push_back((pixels, texture));
        Some(id)
    }
    pub fn paint(self, ui: &egui::Ui, rect: Rect) {
        if !ui.is_rect_visible(rect) {
            return;
        }
        let ppp = ui.ctx().pixels_per_point();
        let pixels = (rect.width() * ppp).round().clamp(4.0, 256.0) as usize;
        let min = egui::pos2(
            (rect.left() * ppp).round() / ppp,
            (rect.top() * ppp).round() / ppp,
        );
        let target = Rect::from_min_size(min, egui::Vec2::splat(pixels as f32 / ppp));
        if let Some(texture) = self.texture(ui.ctx(), pixels) {
            ui.painter().image(
                texture,
                target,
                Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    }
    pub fn show(self, ui: &mut egui::Ui, size: f32) -> egui::Response {
        let (rect, response) =
            ui.allocate_exact_size(egui::Vec2::splat(size), egui::Sense::hover());
        self.paint(ui, rect);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Image, ui.is_enabled(), self.name)
        });
        response
    }
}
const BLUE: Color32 = Color32::from_rgb(96, 165, 250);
const GREEN: Color32 = Color32::from_rgb(134, 239, 172);
const ORANGE: Color32 = Color32::from_rgb(251, 146, 60);
const PINK: Color32 = Color32::from_rgb(249, 168, 212);
const PURPLE: Color32 = Color32::from_rgb(196, 181, 253);
const RED: Color32 = Color32::from_rgb(252, 165, 165);
const STEEL: Color32 = Color32::from_rgb(148, 163, 184);
const YELLOW: Color32 = Color32::from_rgb(253, 224, 71);
const WHITE: Color32 = Color32::from_rgb(203, 213, 225);
const CYAN: Color32 = Color32::from_rgb(103, 232, 249);
pub fn icon(path: &str, binary: bool) -> FileIcon {
    let path = path.replace('\\', "/");
    let name = path
        .rsplit('/')
        .next()
        .unwrap_or(&path)
        .to_ascii_lowercase();
    let (kind, color) = match name.as_str() {
        ".gitignore" => ("git_ignore", Color32::from_gray(128)),
        ".gitattributes" => ("git", RED),
        ".gitmodules" => ("git", RED),
        ".gitconfig" => ("git", RED),
        ".editorconfig" => ("editorconfig", ORANGE),
        "package.json" => ("npm", RED),
        "package-lock.json" => ("npm", RED),
        ".npmrc" => ("npm", RED),
        ".yarnrc" => ("yarn", BLUE),
        "yarn.lock" => ("yarn", BLUE),
        "cargo.toml" => ("rust", ORANGE),
        "cargo.lock" => ("rust", ORANGE),
        "go.mod" => ("go", BLUE),
        "go.sum" => ("go", BLUE),
        "pyproject.toml" => ("python", BLUE),
        "pipfile" => ("python", BLUE),
        "pipfile.lock" => ("python", BLUE),
        "requirements.txt" => ("python", BLUE),
        "setup.py" => ("python", BLUE),
        "dockerfile" => ("docker", BLUE),
        "containerfile" => ("docker", BLUE),
        "compose.yml" => ("docker", BLUE),
        "compose.yaml" => ("docker", BLUE),
        "docker-compose.yml" => ("docker", BLUE),
        "docker-compose.yaml" => ("docker", BLUE),
        "makefile" => ("makefile", ORANGE),
        "gnumakefile" => ("makefile", ORANGE),
        "cmakelists.txt" => ("makefile", BLUE),
        "jenkinsfile" => ("jenkins", RED),
        "license" => ("license", YELLOW),
        "license.md" => ("license", YELLOW),
        "license.txt" => ("license", YELLOW),
        "licence" => ("license", YELLOW),
        "copying" => ("license", YELLOW),
        ".prettierrc" => ("config", PURPLE),
        ".eslintrc" => ("eslint", PURPLE),
        "tsconfig.json" => ("tsconfig", BLUE),
        "jsconfig.json" => ("javascript", YELLOW),
        ".babelrc" => ("babel", YELLOW),
        "gemfile" => ("ruby", RED),
        "gemfile.lock" => ("ruby", RED),
        "rakefile" => ("ruby", RED),
        "pom.xml" => ("maven", RED),
        "build.gradle" => ("gradle", BLUE),
        "settings.gradle" => ("gradle", BLUE),
        _ if name.starts_with(".env.") || name == ".env" => ("config", STEEL),
        _ if name.starts_with("dockerfile.") || name.starts_with("containerfile.") => {
            ("docker", BLUE)
        }
        _ if name.starts_with("webpack.config.") => ("webpack", BLUE),
        _ if name.starts_with("eslint.config.") || name.starts_with(".eslintrc.") => {
            ("eslint", PURPLE)
        }
        _ if name.starts_with("babel.config.") || name.starts_with(".babelrc.") => {
            ("babel", YELLOW)
        }
        _ if name.starts_with("requirements") && name.ends_with(".txt") => ("python", BLUE),
        _ => match name.rsplit_once('.').map_or("", |(_, ext)| ext) {
            "md" | "mdx" | "markdown" => ("markdown", BLUE),
            "json" | "jsonc" | "json5" | "cson" => ("json", YELLOW),
            "py" | "pyw" | "pyi" | "pyx" | "pxd" => ("python", BLUE),
            "ipynb" => ("notebook", ORANGE),
            "html" | "htm" | "xhtml" | "shtml" => ("html", ORANGE),
            "xml" | "xsd" | "xsl" | "xslt" | "plist" => ("xml", ORANGE),
            "css" => ("css", BLUE),
            "scss" | "sass" => ("sass", PINK),
            "less" => ("less", BLUE),
            "styl" | "stylus" => ("stylus", GREEN),
            "js" | "mjs" | "cjs" => ("javascript", YELLOW),
            "ts" | "mts" | "cts" => ("typescript", BLUE),
            "jsx" => ("react", YELLOW),
            "tsx" => ("react", BLUE),
            "vue" => ("vue", GREEN),
            "svelte" => ("svelte", ORANGE),
            "rs" => ("rust", ORANGE),
            "go" => ("go", BLUE),
            "java" | "class" | "jar" => ("java", RED),
            "kt" | "kts" => ("kotlin", PURPLE),
            "c" | "h" => ("c", BLUE),
            "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => ("cpp", BLUE),
            "cs" | "csx" => ("c-sharp", BLUE),
            "fs" | "fsx" | "fsi" => ("f-sharp", BLUE),
            "swift" => ("swift", ORANGE),
            "m" | "mm" => ("apple", STEEL),
            "dart" => ("dart", BLUE),
            "rb" | "rake" | "gemspec" => ("ruby", RED),
            "php" | "phtml" => ("php", PURPLE),
            "lua" => ("lua", BLUE),
            "pl" | "pm" => ("perl", BLUE),
            "r" | "rmd" => ("R", BLUE),
            "scala" | "sc" | "sbt" => ("scala", RED),
            "jl" => ("julia", PURPLE),
            "ex" | "exs" => ("elixir", PURPLE),
            "erl" | "hrl" => ("elixir", RED),
            "hs" | "lhs" => ("haskell", PURPLE),
            "clj" | "cljs" | "cljc" | "edn" => ("clojure", GREEN),
            "ml" | "mli" => ("ocaml", ORANGE),
            "nim" => ("nim", YELLOW),
            "zig" => ("zig", ORANGE),
            "asm" | "s" => ("asm", RED),
            "coffee" | "litcoffee" => ("coffee", YELLOW),
            "pug" | "jade" => ("pug", RED),
            "ejs" => ("ejs", YELLOW),
            "hbs" | "handlebars" | "mustache" => ("mustache", ORANGE),
            "jinja" | "jinja2" | "j2" => ("jinja", RED),
            "twig" => ("twig", GREEN),
            "graphql" | "gql" => ("graphql", PINK),
            "prisma" => ("prisma", BLUE),
            "sql" | "db" | "sqlite" | "sqlite3" | "mdb" => ("db", ORANGE),
            "ps1" | "psm1" | "psd1" => ("powershell", BLUE),
            "sh" | "bash" | "zsh" | "fish" | "ksh" => ("shell", GREEN),
            "cmd" | "bat" => ("windows", BLUE),
            "yml" | "yaml" => ("yml", PURPLE),
            "toml" | "ini" | "cfg" | "config" | "conf" | "env" | "properties" | "rc" => {
                ("config", STEEL)
            }
            "lock" => ("lock", YELLOW),
            "log" => ("info", STEEL),
            "csv" | "tsv" => ("csv", GREEN),
            "tf" | "tfvars" => ("terraform", PURPLE),
            "tex" | "bib" => ("tex", BLUE),
            "png" | "jpg" | "jpeg" | "gif" | "ico" | "bmp" | "webp" | "avif" | "heic" | "tif"
            | "tiff" => ("image", PURPLE),
            "svg" | "svgz" => ("svg", ORANGE),
            "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "wma" | "mid" => ("audio", PURPLE),
            "mp4" | "mkv" | "avi" | "mov" | "webm" | "wmv" | "mpeg" => ("video", PURPLE),
            "pdf" => ("pdf", RED),
            "zip" | "7z" | "gz" | "tgz" | "tar" | "rar" | "xz" | "bz2" | "zst" => ("zip", YELLOW),
            "ttf" | "otf" | "woff" | "woff2" | "eot" => ("font", RED),
            "doc" | "docx" | "odt" | "rtf" => ("word", BLUE),
            "xls" | "xlsx" | "ods" => ("xls", GREEN),
            "exe" | "dll" | "msi" | "com" => ("windows", BLUE),
            "wasm" | "wat" => ("wasm", PURPLE),
            "pem" | "key" | "cer" | "crt" | "cert" | "pfx" => ("lock", GREEN),
            _ if binary => ("hex", STEEL),
            _ => ("default", WHITE),
        },
    };
    let (asset, svg): (&str, &[u8]) = match kind {
        "R" => (
            "letter-r.svg",
            include_bytes!("../assets/file-icons/letter-r.svg"),
        ),
        "apple" => (
            "brand-apple.svg",
            include_bytes!("../assets/file-icons/brand-apple.svg"),
        ),
        "asm" => ("cpu.svg", include_bytes!("../assets/file-icons/cpu.svg")),
        "audio" => (
            "volume.svg",
            include_bytes!("../assets/file-icons/volume.svg"),
        ),
        "babel" => (
            "brand-javascript.svg",
            include_bytes!("../assets/file-icons/brand-javascript.svg"),
        ),
        "c" => (
            "letter-c.svg",
            include_bytes!("../assets/file-icons/letter-c.svg"),
        ),
        "c-sharp" => (
            "brand-c-sharp.svg",
            include_bytes!("../assets/file-icons/brand-c-sharp.svg"),
        ),
        "clojure" => (
            "code-circle.svg",
            include_bytes!("../assets/file-icons/code-circle.svg"),
        ),
        "coffee" => (
            "coffee.svg",
            include_bytes!("../assets/file-icons/coffee.svg"),
        ),
        "config" => (
            "settings.svg",
            include_bytes!("../assets/file-icons/settings.svg"),
        ),
        "cpp" => (
            "brand-cpp.svg",
            include_bytes!("../assets/file-icons/brand-cpp.svg"),
        ),
        "css" => (
            "brand-css3.svg",
            include_bytes!("../assets/file-icons/brand-css3.svg"),
        ),
        "csv" => (
            "file-type-csv.svg",
            include_bytes!("../assets/file-icons/file-type-csv.svg"),
        ),
        "dart" => (
            "code-dots.svg",
            include_bytes!("../assets/file-icons/code-dots.svg"),
        ),
        "db" => (
            "database.svg",
            include_bytes!("../assets/file-icons/database.svg"),
        ),
        "default" => (
            "file-description.svg",
            include_bytes!("../assets/file-icons/file-description.svg"),
        ),
        "docker" => (
            "brand-docker.svg",
            include_bytes!("../assets/file-icons/brand-docker.svg"),
        ),
        "editorconfig" => (
            "file-settings.svg",
            include_bytes!("../assets/file-icons/file-settings.svg"),
        ),
        "ejs" => (
            "file-code.svg",
            include_bytes!("../assets/file-icons/file-code.svg"),
        ),
        "elixir" => (
            "droplet.svg",
            include_bytes!("../assets/file-icons/droplet.svg"),
        ),
        "eslint" => (
            "brand-javascript.svg",
            include_bytes!("../assets/file-icons/brand-javascript.svg"),
        ),
        "f-sharp" => (
            "math-function.svg",
            include_bytes!("../assets/file-icons/math-function.svg"),
        ),
        "font" => (
            "typography.svg",
            include_bytes!("../assets/file-icons/typography.svg"),
        ),
        "git" => (
            "brand-git.svg",
            include_bytes!("../assets/file-icons/brand-git.svg"),
        ),
        "git_ignore" => (
            "file-off.svg",
            include_bytes!("../assets/file-icons/file-off.svg"),
        ),
        "go" => (
            "brand-golang.svg",
            include_bytes!("../assets/file-icons/brand-golang.svg"),
        ),
        "gradle" => (
            "brand-android.svg",
            include_bytes!("../assets/file-icons/brand-android.svg"),
        ),
        "graphql" => (
            "brand-graphql.svg",
            include_bytes!("../assets/file-icons/brand-graphql.svg"),
        ),
        "haskell" => (
            "math-function.svg",
            include_bytes!("../assets/file-icons/math-function.svg"),
        ),
        "hex" => (
            "binary.svg",
            include_bytes!("../assets/file-icons/binary.svg"),
        ),
        "html" => ("code.svg", include_bytes!("../assets/file-icons/code.svg")),
        "image" => (
            "photo.svg",
            include_bytes!("../assets/file-icons/photo.svg"),
        ),
        "info" => (
            "file-text.svg",
            include_bytes!("../assets/file-icons/file-text.svg"),
        ),
        "java" => (
            "coffee.svg",
            include_bytes!("../assets/file-icons/coffee.svg"),
        ),
        "javascript" => (
            "brand-javascript.svg",
            include_bytes!("../assets/file-icons/brand-javascript.svg"),
        ),
        "jenkins" => (
            "automation.svg",
            include_bytes!("../assets/file-icons/automation.svg"),
        ),
        "jinja" => (
            "template.svg",
            include_bytes!("../assets/file-icons/template.svg"),
        ),
        "json" => (
            "braces.svg",
            include_bytes!("../assets/file-icons/braces.svg"),
        ),
        "julia" => (
            "circles.svg",
            include_bytes!("../assets/file-icons/circles.svg"),
        ),
        "kotlin" => (
            "brand-kotlin.svg",
            include_bytes!("../assets/file-icons/brand-kotlin.svg"),
        ),
        "less" => (
            "brand-css3.svg",
            include_bytes!("../assets/file-icons/brand-css3.svg"),
        ),
        "license" => (
            "license.svg",
            include_bytes!("../assets/file-icons/license.svg"),
        ),
        "lock" => ("lock.svg", include_bytes!("../assets/file-icons/lock.svg")),
        "lua" => (
            "moon-stars.svg",
            include_bytes!("../assets/file-icons/moon-stars.svg"),
        ),
        "makefile" => (
            "hammer.svg",
            include_bytes!("../assets/file-icons/hammer.svg"),
        ),
        "markdown" => (
            "markdown.svg",
            include_bytes!("../assets/file-icons/markdown.svg"),
        ),
        "maven" => (
            "package.svg",
            include_bytes!("../assets/file-icons/package.svg"),
        ),
        "mustache" => (
            "template.svg",
            include_bytes!("../assets/file-icons/template.svg"),
        ),
        "nim" => (
            "crown.svg",
            include_bytes!("../assets/file-icons/crown.svg"),
        ),
        "notebook" => (
            "notebook.svg",
            include_bytes!("../assets/file-icons/notebook.svg"),
        ),
        "npm" => (
            "brand-npm.svg",
            include_bytes!("../assets/file-icons/brand-npm.svg"),
        ),
        "ocaml" => (
            "lambda.svg",
            include_bytes!("../assets/file-icons/lambda.svg"),
        ),
        "pdf" => (
            "file-type-pdf.svg",
            include_bytes!("../assets/file-icons/file-type-pdf.svg"),
        ),
        "perl" => ("code.svg", include_bytes!("../assets/file-icons/code.svg")),
        "php" => (
            "brand-php.svg",
            include_bytes!("../assets/file-icons/brand-php.svg"),
        ),
        "powershell" => (
            "terminal.svg",
            include_bytes!("../assets/file-icons/terminal.svg"),
        ),
        "prisma" => (
            "triangle.svg",
            include_bytes!("../assets/file-icons/triangle.svg"),
        ),
        "pug" => (
            "template.svg",
            include_bytes!("../assets/file-icons/template.svg"),
        ),
        "python" => (
            "brand-python.svg",
            include_bytes!("../assets/file-icons/brand-python.svg"),
        ),
        "react" => (
            "brand-react.svg",
            include_bytes!("../assets/file-icons/brand-react.svg"),
        ),
        "ruby" => (
            "diamond.svg",
            include_bytes!("../assets/file-icons/diamond.svg"),
        ),
        "rust" => (
            "brand-rust.svg",
            include_bytes!("../assets/file-icons/brand-rust.svg"),
        ),
        "sass" => (
            "brand-sass.svg",
            include_bytes!("../assets/file-icons/brand-sass.svg"),
        ),
        "scala" => (
            "stairs.svg",
            include_bytes!("../assets/file-icons/stairs.svg"),
        ),
        "shell" => (
            "terminal-2.svg",
            include_bytes!("../assets/file-icons/terminal-2.svg"),
        ),
        "stylus" => (
            "brand-css3.svg",
            include_bytes!("../assets/file-icons/brand-css3.svg"),
        ),
        "svelte" => (
            "brand-svelte.svg",
            include_bytes!("../assets/file-icons/brand-svelte.svg"),
        ),
        "svg" => (
            "file-type-svg.svg",
            include_bytes!("../assets/file-icons/file-type-svg.svg"),
        ),
        "swift" => (
            "brand-swift.svg",
            include_bytes!("../assets/file-icons/brand-swift.svg"),
        ),
        "terraform" => (
            "brand-terraform.svg",
            include_bytes!("../assets/file-icons/brand-terraform.svg"),
        ),
        "tex" => (
            "math-function.svg",
            include_bytes!("../assets/file-icons/math-function.svg"),
        ),
        "tsconfig" => (
            "file-settings.svg",
            include_bytes!("../assets/file-icons/file-settings.svg"),
        ),
        "twig" => (
            "template.svg",
            include_bytes!("../assets/file-icons/template.svg"),
        ),
        "typescript" => (
            "brand-typescript.svg",
            include_bytes!("../assets/file-icons/brand-typescript.svg"),
        ),
        "video" => (
            "movie.svg",
            include_bytes!("../assets/file-icons/movie.svg"),
        ),
        "vue" => (
            "brand-vue.svg",
            include_bytes!("../assets/file-icons/brand-vue.svg"),
        ),
        "wasm" => (
            "binary.svg",
            include_bytes!("../assets/file-icons/binary.svg"),
        ),
        "webpack" => ("box.svg", include_bytes!("../assets/file-icons/box.svg")),
        "windows" => (
            "brand-windows.svg",
            include_bytes!("../assets/file-icons/brand-windows.svg"),
        ),
        "word" => (
            "file-type-doc.svg",
            include_bytes!("../assets/file-icons/file-type-doc.svg"),
        ),
        "xls" => (
            "file-spreadsheet.svg",
            include_bytes!("../assets/file-icons/file-spreadsheet.svg"),
        ),
        "xml" => (
            "file-type-xml.svg",
            include_bytes!("../assets/file-icons/file-type-xml.svg"),
        ),
        "yarn" => (
            "brand-yarn.svg",
            include_bytes!("../assets/file-icons/brand-yarn.svg"),
        ),
        "yml" => (
            "file-settings.svg",
            include_bytes!("../assets/file-icons/file-settings.svg"),
        ),
        "zig" => (
            "letter-z.svg",
            include_bytes!("../assets/file-icons/letter-z.svg"),
        ),
        "zip" => (
            "file-zip.svg",
            include_bytes!("../assets/file-icons/file-zip.svg"),
        ),
        _ => unreachable!("icon kind comes from the static association table"),
    };
    FileIcon {
        name: kind,
        asset,
        color: if kind == "python" { CYAN } else { color },
        svg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn svg_cache_is_bounded_and_tints_do_not_share_a_texture() {
        let ctx = egui::Context::default();
        let icon = icon("sample.py", false);
        for pixels in 8..30 {
            assert!(icon.texture(&ctx, pixels).is_some());
        }
        let cache = ctx.data_mut(|d| d.get_temp::<Arc<Mutex<Raster>>>(icon.cache_id()).unwrap());
        assert_eq!(cache.lock().unwrap().sizes.len(), CACHE_SIZES);
        assert_ne!(
            super::icon("a.jsx", false).texture(&ctx, 24),
            super::icon("a.tsx", false).texture(&ctx, 24)
        );
    }
}
