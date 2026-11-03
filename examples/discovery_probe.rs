//! Installed public launcher selection; --version only, without a model request.
use ai_terminal::{discovery::CliSearch, model::Agent};
use std::{path::PathBuf, process::Command};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("discovery_probe output-directory")?,
    );
    std::fs::create_dir_all(&output)?;
    let search = CliSearch::current();
    let mut checks = Vec::new();
    for agent in [Agent::Codex, Agent::Claude] {
        let binary = search.resolve(agent)?;
        let result = Command::new(&binary)
            .arg("--version")
            .env("PATH", search.child_path()?)
            .output()?;
        checks.push(serde_json::json!({"agent":agent.label(),"selected":binary,"exit":result.status.code(),"version":String::from_utf8_lossy(&result.stdout).trim(),"ok":result.status.success()}));
    }
    let ok = checks.iter().all(|check| check["ok"] == true);
    std::fs::write(
        output.join("result.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"ok":ok,"checks":checks,"model_request_sent":false}),
        )?,
    )?;
    if !ok {
        return Err("An installed CLI did not accept --version".into());
    }
    Ok(())
}
