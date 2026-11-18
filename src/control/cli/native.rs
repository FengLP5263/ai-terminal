use super::{Args, invalid, number};
use crate::control::{ApiResult, actions::*};
use std::{io::Read, path::PathBuf};

pub const HELP: &str = r#"
Native controls (require a manager with native_controls capability):
  instance capabilities                    Discover supported native domains
  settings show | fonts | reset
  settings set [--language system|zh_cn|en] [--font FONT_ID] [--scale 50..250]
      [--sidebar-collapsed true|false] [--sidebar-width 190..420]
      [--floating-enabled true|false] [--floating-border true|false]
      [--floating-scale 50..250] [--floating-transparency 0..100]
      [--file SETTINGS_PATCH.json]
  floating show | hide | status | center
  floating set [--enabled true|false] [--transparency 0..100] [--border true|false]
      [--scale 50..250] [--x PIXELS --y PIXELS]
  sidebar show | hide | set --width POINTS
  window show | focus | minimize | maximize | restore
  window set [--width POINTS --height POINTS] [--x POINTS --y POINTS]
  window capture --output IMAGE.png         Capture this manager's main viewport
  window capture-status                    Result of the last requested capture
  app quit [--force]                        Stop only this manager and its sessions
  panel open files|git|history|changes|settings
  panel close files|git|settings
  project show
  file list [--path RELATIVE] [--offset N --limit N]
  file expand --path RELATIVE | collapse --path RELATIVE | collapse-all | refresh
  file open --path RELATIVE | close | show | read [--offset N --limit N]
  file next | previous | first | reload | source | preview
  image fit | original | zoom --percent 5..800
  media status | play | pause | seek --seconds N | volume --percent 0..100
  git status | files | history | commit | diff [--offset N --limit N]
  git refresh | more | inspect --commit FULL_SHA
  git open-diff --path RELATIVE [--commit FULL_SHA] | close-diff
  terminal read ID [--offset N --limit N]    Explicit current-viewport text only
  terminal write ID --text TEXT|--file PATH|- [--submit]
  terminal key ID --key enter|escape|tab|backspace|up|down|left|right|home|end|
      page_up|page_down|ctrl_c|ctrl_d|ctrl_l|ctrl_z
  terminal scroll ID --lines N|--offset N    Local history; never sends mouse/key input
  session stop ID [--force] | restart ID [--force] | acknowledge ID

Percentages are human percentages: --transparency 50 means 50% transparent.
Main/floating scale and position remain independent. Settings validate before save.
File/Git I/O is asynchronous: loading=true means queued; query again after it settles.
Terminal input is sent only when explicitly requested. --submit adds Enter; it can
execute Shell commands or submit a model prompt. No input/output is logged by the manager.
"#;

fn boolean(value: String) -> ApiResult<bool> {
    match value.as_str() {
        "true" | "on" | "1" => Ok(true),
        "false" | "off" | "0" => Ok(false),
        _ => Err(invalid("Use true or false")),
    }
}
fn float(value: String) -> ApiResult<f32> {
    value
        .trim_end_matches('%')
        .parse::<f32>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| invalid("Expected a finite number"))
}
fn scalar(value: String) -> ApiResult<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| invalid("Expected a finite number"))
}
fn flag_bool(args: &mut Args, name: &str) -> ApiResult<Option<bool>> {
    args.take(name).map(boolean).transpose()
}
fn flag_float(args: &mut Args, name: &str) -> ApiResult<Option<f32>> {
    args.take(name).map(float).transpose()
}
fn settings(args: &mut Args) -> ApiResult<SettingsPatch> {
    let mut p = if let Some(file) = args.take("file") {
        let mut bytes = Vec::new();
        std::fs::File::open(file)
            .map_err(|e| invalid(e.to_string()))?
            .take(16_385)
            .read_to_end(&mut bytes)
            .map_err(|e| invalid(e.to_string()))?;
        if bytes.len() > 16_384 {
            return Err(invalid("Settings patch exceeds 16 KiB"));
        }
        serde_json::from_slice(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes))
            .map_err(|e| invalid(format!("Invalid settings patch: {e}")))?
    } else {
        SettingsPatch::default()
    };
    if let Some(v) = args.take("language") {
        p.language = Some(v);
    }
    if let Some(v) = args.take("font") {
        p.terminal_font = Some(v);
    }
    if let Some(v) = flag_float(args, "scale")? {
        p.ui_scale_percent = Some(v);
    }
    if let Some(v) = flag_bool(args, "sidebar-collapsed")? {
        p.sidebar_collapsed = Some(v);
    }
    if let Some(v) = flag_float(args, "sidebar-width")? {
        p.sidebar_width = Some(v);
    }
    if let Some(v) = flag_bool(args, "floating-enabled")? {
        p.floating_enabled = Some(v);
    }
    if let Some(v) = flag_bool(args, "floating-border")? {
        p.floating_border = Some(v);
    }
    if let Some(v) = flag_float(args, "floating-scale")? {
        p.floating_scale_percent = Some(v);
    }
    if let Some(v) = flag_float(args, "floating-transparency")? {
        p.floating_transparency_percent = Some(v);
    }
    Ok(p)
}
fn input(args: &mut Args) -> ApiResult<String> {
    match (args.take("text"), args.take("file")) {
        (Some(text), None) => Ok(text),
        (None, Some(path)) => {
            let mut bytes = Vec::new();
            let reader: Box<dyn Read> = if path == "-" {
                Box::new(std::io::stdin())
            } else {
                Box::new(std::fs::File::open(path).map_err(|e| invalid(e.to_string()))?)
            };
            reader
                .take(65_537)
                .read_to_end(&mut bytes)
                .map_err(|e| invalid(e.to_string()))?;
            if bytes.len() > 65_536 {
                return Err(invalid("Terminal input exceeds 64 KiB"));
            }
            String::from_utf8(bytes).map_err(|_| invalid("Terminal input must be UTF-8"))
        }
        _ => Err(invalid("Supply exactly one of --text or --file")),
    }
}

pub(super) fn parse(args: &mut Args) -> ApiResult<Option<(Action, usize)>> {
    let domain = args
        .words
        .first()
        .map(String::as_str)
        .unwrap_or("")
        .to_owned();
    let verb = args
        .words
        .get(1)
        .map(String::as_str)
        .unwrap_or("")
        .to_owned();
    let mut count = 2;
    let action = match (domain.as_str(), verb.as_str()) {
        ("settings", "show") | ("floating", "status") => Action::SettingsShow,
        ("settings", "fonts") => Action::Fonts,
        ("settings", "reset") => Action::SettingsReset,
        ("settings", "set") => Action::SettingsSet {
            patch: settings(args)?,
        },
        ("floating", "show" | "hide" | "center") => Action::SettingsSet {
            patch: SettingsPatch {
                floating_enabled: (verb != "center").then_some(verb == "show"),
                floating_center: (verb == "center").then_some(true),
                ..Default::default()
            },
        },
        ("floating", "set") => {
            let x = flag_float(args, "x")?;
            let y = flag_float(args, "y")?;
            let position = match (x, y) {
                (None, None) => None,
                (Some(x), Some(y)) => Some([x, y]),
                _ => return Err(invalid("Specify both --x and --y")),
            };
            Action::SettingsSet {
                patch: SettingsPatch {
                    floating_enabled: flag_bool(args, "enabled")?,
                    floating_border: flag_bool(args, "border")?,
                    floating_transparency_percent: flag_float(args, "transparency")?,
                    floating_scale_percent: flag_float(args, "scale")?,
                    floating_position_pixels: position,
                    ..Default::default()
                },
            }
        }
        ("sidebar", "show" | "hide" | "set") => Action::SettingsSet {
            patch: SettingsPatch {
                sidebar_collapsed: (verb != "set").then_some(verb == "hide"),
                sidebar_width: flag_float(args, "width")?,
                ..Default::default()
            },
        },
        ("window", "show") => Action::WindowShow,
        ("window", "set") => Action::WindowSet {
            width: flag_float(args, "width")?,
            height: flag_float(args, "height")?,
            x: flag_float(args, "x")?,
            y: flag_float(args, "y")?,
        },
        ("window", "focus" | "minimize" | "maximize" | "restore") => Action::WindowState {
            state: serde_json::from_value(serde_json::json!(verb)).unwrap(),
        },
        ("window", "capture") => Action::Capture {
            output: std::path::absolute(args.required("output")?)
                .map_err(|e| invalid(e.to_string()))?,
        },
        ("window", "capture-status") => Action::CaptureStatus,
        ("app", "quit") => Action::Quit {
            force: args.yes("force"),
        },
        ("panel", "open" | "close") => {
            count = 3;
            let panel = args.id()?;
            Action::Panel {
                panel: serde_json::from_value(serde_json::json!(panel))
                    .map_err(|_| invalid("Unknown panel"))?,
                open: verb == "open",
            }
        }
        ("project", "show") => Action::ProjectShow,
        ("file", "list") => Action::DirectoryList {
            path: PathBuf::from(args.take("path").unwrap_or_default()),
            offset: number(args.take("offset"), 0)?,
            limit: number(args.take("limit"), 100)?,
        },
        ("file", "expand" | "collapse") => Action::DirectoryExpand {
            path: args.required("path")?.into(),
            expand: verb == "expand",
        },
        ("file", "collapse-all") => Action::DirectoryCollapseAll,
        ("file", "refresh") => Action::FilesRefresh,
        ("file", "open") => Action::FileOpen {
            path: args.required("path")?.into(),
        },
        ("file", "close") => Action::FileClose,
        ("file", "show" | "read") => Action::FileShow {
            content: verb == "read",
            offset: number(args.take("offset"), 0)?,
            limit: number(args.take("limit"), 200)?,
        },
        ("file", "next" | "previous" | "first" | "reload") => Action::FilePage {
            direction: serde_json::from_value(serde_json::json!(verb)).unwrap(),
        },
        ("file", "source" | "preview") => Action::FileMode {
            source: verb == "source",
        },
        ("image", "fit" | "original" | "zoom") => Action::ImageZoom {
            percent: match verb.as_str() {
                "fit" => None,
                "original" => Some(100.),
                _ => Some(float(args.required("percent")?)?),
            },
        },
        ("media", "status" | "play" | "pause" | "seek" | "volume") => Action::Media {
            operation: serde_json::from_value(serde_json::json!(verb)).unwrap(),
            value: match verb.as_str() {
                "seek" => Some(scalar(args.required("seconds")?)?),
                "volume" => Some(scalar(args.required("percent")?)?),
                _ => None,
            },
        },
        ("git", "status" | "files" | "history" | "commit" | "diff") => Action::GitShow {
            section: serde_json::from_value(serde_json::json!(verb)).unwrap(),
            offset: number(args.take("offset"), 0)?,
            limit: number(args.take("limit"), 100)?,
        },
        ("git", "refresh") => Action::GitRefresh,
        ("git", "more") => Action::GitMore,
        ("git", "inspect") => Action::GitInspect {
            id: args.required("commit")?,
        },
        ("git", "open-diff") => Action::GitOpenDiff {
            path: args.required("path")?,
            commit: args.take("commit"),
        },
        ("git", "close-diff") => Action::GitCloseDiff,
        ("terminal", "read") => {
            count = 3;
            Action::TerminalRead {
                id: args.id()?,
                offset: number(args.take("offset"), 0)?,
                limit: number(args.take("limit"), 200)?,
            }
        }
        ("terminal", "write") => {
            count = 3;
            Action::TerminalWrite {
                id: args.id()?,
                text: input(args)?,
                submit: args.yes("submit"),
            }
        }
        ("terminal", "key") => {
            count = 3;
            let key = args
                .required("key")?
                .to_ascii_lowercase()
                .replace(['+', '-'], "_");
            Action::TerminalKey {
                id: args.id()?,
                key: serde_json::from_value(serde_json::json!(key))
                    .map_err(|_| invalid("Unknown terminal key"))?,
            }
        }
        ("terminal", "scroll") => {
            count = 3;
            Action::TerminalScroll {
                id: args.id()?,
                lines: args
                    .take("lines")
                    .map(|v| v.parse::<i32>().map_err(|_| invalid("Invalid line count")))
                    .transpose()?,
                offset: args
                    .take("offset")
                    .map(|v| v.parse::<usize>().map_err(|_| invalid("Invalid offset")))
                    .transpose()?,
            }
        }
        ("session", "stop" | "restart") => {
            count = 3;
            Action::SessionStop {
                id: args.id()?,
                force: args.yes("force"),
                restart: verb == "restart",
            }
        }
        ("session", "acknowledge") => {
            count = 3;
            Action::SessionAcknowledge { id: args.id()? }
        }
        _ => return Ok(None),
    };
    Ok(Some((action, count)))
}
