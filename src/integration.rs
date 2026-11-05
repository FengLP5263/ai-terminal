//! Invocation-only integrations. Never edit upstream user/project configuration.
use crate::model::Agent;
use std::{collections::HashMap, io::Read, path::Path};

pub fn environment(agent: Agent, runtime: &Path, helper: &Path) -> HashMap<String, String> {
    let mut env = HashMap::new();
    if agent == Agent::Kimi && std::env::var_os("PI_PROGRAM_STATUS").is_none() {
        // pi-tui's documented force switch avoids a capability-query round trip.
        env.insert("PI_PROGRAM_STATUS".into(), "1".into());
    }
    if agent == Agent::ZCode && std::env::var_os("ZCODE_CLI_SETTINGS_FILE").is_none() {
        let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
        let Some(home) = home else {
            return env;
        };
        let path = std::path::PathBuf::from(home).join(".zcode/cli/setting.json");
        if let Some(target) = zcode_snapshot(&path, runtime, helper) {
            env.insert(
                "ZCODE_CLI_SETTINGS_FILE".into(),
                target.to_string_lossy().into_owned(),
            );
        } else {
            eprintln!(
                "ai-terminal: ZCode custom settings preserved; task hooks were not injected. See the status connection help."
            );
        }
    }
    env
}

// Optional diagnostics must not prevent the underlying CLI from starting.
fn zcode_snapshot(path: &Path, runtime: &Path, helper: &Path) -> Option<std::path::PathBuf> {
    // Leave legacy migration to the CLI without inspecting possible credentials.
    if !path.exists() && path.with_file_name("config.json").exists() {
        return None;
    }
    let settings = match std::fs::File::open(path) {
        Ok(file) => {
            let mut bytes = Vec::new();
            file.take(65537).read_to_end(&mut bytes).ok()?;
            if bytes.len() > 65536 {
                return None;
            }
            serde_json::from_slice(&bytes).ok()?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_json::json!({}),
        _ => return None,
    };
    let settings = zcode_settings(settings, helper)?;
    let bytes = serde_json::to_vec(&settings).ok()?;
    std::fs::create_dir_all(runtime).ok()?;
    let target = std::path::absolute(runtime.join("zcode-settings.json")).ok()?;
    std::fs::write(&target, bytes).ok()?;
    Some(target)
}

/// Only a credential-free settings subset can be snapshotted. Unknown/custom
/// settings keep the original CLI configuration and disable injection entirely.
/// Provider and credential files are never opened. Existing hooks are not cloned.
fn zcode_settings(mut value: serde_json::Value, helper: &Path) -> Option<serde_json::Value> {
    let root = value.as_object_mut()?;
    const SECTIONS: &[&str] = &[
        "modelStream",
        "permission",
        "storage",
        "network",
        "features",
        "subagents",
        "memory",
        "mcp",
        "plugins",
        "skills",
        "skill",
        "command",
        "logging",
        "ui",
        "toolConcurrency",
        "modelAnomalyGuard",
        "hooks",
    ];
    if root.keys().any(|k| !SECTIONS.contains(&k.as_str())) {
        return None;
    }
    fn safe(v: &serde_json::Value, path: &str) -> bool {
        match v {
            serde_json::Value::Bool(_) => matches!(
                path,
                "permission.autoApproveHighRisk"
                    | "permission.allowMediumRiskInAuto"
                    | "features.compact"
                    | "features.rewind"
                    | "features.subagent"
                    | "features.skill"
                    | "features.mcp"
                    | "memory.autoConsolidate"
                    | "plugins.enabled"
                    | "skills.enabled"
                    | "skills.includeInstructions"
                    | "ui.copyOnSelect"
                    | "hooks.enabled"
            ),
            serde_json::Value::Number(_) => matches!(
                path,
                "modelStream.idleTimeoutMs"
                    | "network.timeout"
                    | "subagents.autoBackgroundMs"
                    | "memory.summaryMaxBytes"
                    | "skills.metadataBudget"
                    | "toolConcurrency.maxConcurrency"
                    | "modelAnomalyGuard.repeatedToolCallWarningThreshold"
                    | "modelAnomalyGuard.maxBudgetWarningsPerTurn"
                    | "hooks.timeoutMs"
                    | "hooks.maxOutputBytes"
            ),
            serde_json::Value::Null => false,
            serde_json::Value::String(s) => match path {
                "storage.dir" | "storage.sessionDbPath" => {
                    !s.contains(['\n', '\r', '\0']) && s.len() < 2048
                }
                "permission.mode" => {
                    ["build", "edit", "plan", "yolo", "guarded"].contains(&s.as_str())
                }
                "ui.theme" => ["auto", "dark", "light"].contains(&s.as_str()),
                "ui.locale" => ["auto", "en-US", "zh-CN"].contains(&s.as_str()),
                "ui.tuiMode" => ["regular", "fullscreen"].contains(&s.as_str()),
                "ui.notifications.method" => {
                    ["auto", "osc9", "osc99", "osc777", "bell", "none"].contains(&s.as_str())
                }
                "ui.notifications.condition" => {
                    ["unfocused", "always", "never"].contains(&s.as_str())
                }
                "logging.level" => {
                    ["trace", "debug", "info", "warn", "error", "silent"].contains(&s.as_str())
                }
                "logging.format" => ["text", "json"].contains(&s.as_str()),
                _ => false,
            },
            serde_json::Value::Array(a) => {
                a.is_empty()
                    && matches!(
                        path,
                        "permission.allowedTools"
                            | "permission.disallowedTools"
                            | "plugins.dirs"
                            | "plugins.suppressedBuiltins"
                            | "skills.roots"
                            | "hooks.events.SessionStart"
                            | "hooks.events.UserPromptSubmit"
                            | "hooks.events.PreToolUse"
                            | "hooks.events.PermissionRequest"
                            | "hooks.events.PostToolUse"
                            | "hooks.events.PostToolUseFailure"
                            | "hooks.events.Stop"
                    )
            }
            serde_json::Value::Object(o) => {
                matches!(
                    path,
                    "" | "modelStream"
                        | "permission"
                        | "storage"
                        | "network"
                        | "features"
                        | "subagents"
                        | "memory"
                        | "mcp"
                        | "mcp.servers"
                        | "plugins"
                        | "plugins.enabledPlugins"
                        | "plugins.options"
                        | "skills"
                        | "skill"
                        | "command"
                        | "logging"
                        | "ui"
                        | "ui.notifications"
                        | "toolConcurrency"
                        | "modelAnomalyGuard"
                        | "hooks"
                        | "hooks.events"
                ) && o.iter().all(|(k, v)| {
                    let lower = k.to_ascii_lowercase();
                    ![
                        "token",
                        "secret",
                        "password",
                        "credential",
                        "apikey",
                        "api_key",
                        "authorization",
                    ]
                    .iter()
                    .any(|s| lower.contains(s))
                        && safe(
                            v,
                            &if path.is_empty() {
                                k.clone()
                            } else {
                                format!("{path}.{k}")
                            },
                        )
                })
            }
        }
    }
    if !safe(&value, "") {
        return None;
    }
    let mut events = serde_json::Map::new();
    for name in [
        "SessionStart",
        "UserPromptSubmit",
        "PreToolUse",
        "PostToolUse",
        "PostToolUseFailure",
        "Stop",
    ] {
        events.insert(name.into(), serde_json::json!([{"hooks":[{"type":"process","command":helper,"args":["--hook"],"timeoutMs":2000}]}]));
    }
    let hooks = value
        .as_object_mut()?
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()?;
    hooks.insert("enabled".into(), true.into());
    hooks.insert("events".into(), events.into());
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_io_keeps_original_untouched_and_skips_legacy_oversize_and_io_failure() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("setting.json");
        let runtime = root.path().join("invocation");
        let original = br#"{"ui":{"locale":"en-US"},"hooks":{"timeoutMs":60000}}"#;
        std::fs::write(&source, original).unwrap();
        let snapshot = zcode_snapshot(&source, &runtime, Path::new("helper.exe")).unwrap();
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(snapshot).unwrap()).unwrap();
        assert_eq!(value["ui"]["locale"], "en-US");
        assert_eq!(value["hooks"]["timeoutMs"], 60000);
        assert!(value["hooks"]["enabled"].as_bool().unwrap());
        std::fs::write(&source, vec![b' '; 65537]).unwrap();
        assert!(zcode_snapshot(&source, &runtime, Path::new("helper.exe")).is_none());
        std::fs::remove_file(&source).unwrap();
        std::fs::write(
            source.with_file_name("config.json"),
            b"legacy-private-fixture",
        )
        .unwrap();
        assert!(zcode_snapshot(&source, &runtime, Path::new("helper.exe")).is_none());
        std::fs::write(&source, original).unwrap();
        let occupied = root.path().join("occupied");
        std::fs::write(&occupied, b"file").unwrap();
        assert!(zcode_snapshot(&source, &occupied, Path::new("helper.exe")).is_none());
        assert_eq!(std::fs::read(&source).unwrap(), original);
    }
    #[test]
    fn zcode_snapshot_preserves_safe_preferences_and_refuses_custom_or_secret_data() {
        let base = serde_json::json!({"ui":{"locale":"zh-CN"},"hooks":{"enabled":false,"events":{"Stop":[]}}});
        let result = zcode_settings(base, Path::new("helper.exe")).unwrap();
        assert_eq!(result["ui"]["locale"], "zh-CN");
        assert_eq!(result["hooks"]["enabled"], true);
        for value in [
            serde_json::json!({"mcp":{"servers":{"private":{"env":{"API_KEY":"test"}}}}}),
            serde_json::json!({"hooks":{"events":{"Stop":[{"command":"private"}]}}}),
            serde_json::json!({"provider":{"token":"test"}}),
            serde_json::json!({"ui":{"custom":"private"}}),
            serde_json::json!({"network":{"pin":1234}}),
            serde_json::json!({"features":{"privateFlag":true}}),
        ] {
            assert!(zcode_settings(value, Path::new("helper.exe")).is_none());
        }
    }
}
