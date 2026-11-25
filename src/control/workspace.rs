use super::{ApiError, ApiResult, MAX_OPERATIONS, Operation};
use crate::model::{self, SessionGroup, SessionSpec, Workspace};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};

pub struct Prepared {
    pub workspace: Workspace,
    pub results: Vec<Value>,
}

/// An optimistic concurrency stamp for navigation metadata, including GUI edits.
pub fn revision(workspace: &Workspace) -> String {
    let bytes = serde_json::to_vec(&json!({
        "sessions": workspace.sessions, "groups": workspace.groups, "active": workspace.active
    }))
    .expect("workspace metadata is serializable");
    // FNV-1a is a change stamp, not an authentication or content-integrity hash.
    let hash = bytes.iter().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    });
    format!("{hash:016x}")
}

pub fn resolve(
    id: &str,
    caller: Option<&str>,
    keys: &BTreeMap<String, String>,
) -> ApiResult<String> {
    if id == "current" || id == "$current" {
        caller.map(str::to_owned).ok_or_else(|| {
            ApiError::new(
                "no_current_session",
                "Use an explicit session UUID outside a managed terminal",
            )
        })
    } else if let Some(key) = id.strip_prefix('$') {
        keys.get(key).cloned().ok_or_else(|| {
            ApiError::new("not_found", format!("Unknown earlier operation key: {key}"))
        })
    } else {
        Ok(id.into())
    }
}
fn label(name: &str) -> ApiResult<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 128 || name.chars().any(char::is_control) {
        return Err(ApiError::new(
            "invalid_argument",
            "Names must contain 1–128 characters without control characters",
        ));
    }
    Ok(name.into())
}
fn remember(keys: &mut BTreeMap<String, String>, key: &Option<String>, id: &str) -> ApiResult<()> {
    if let Some(key) = key {
        if key.is_empty()
            || key.len() > 64
            || key == "current"
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            || keys.contains_key(key)
        {
            return Err(ApiError::new(
                "invalid_argument",
                "Operation keys must be unique ASCII identifiers (1–64 characters); current is reserved",
            ));
        }
        keys.insert(key.clone(), id.into());
    }
    Ok(())
}
fn session_index(w: &Workspace, id: &str) -> ApiResult<usize> {
    w.sessions
        .iter()
        .position(|s| s.id == id)
        .ok_or_else(|| ApiError::new("not_found", format!("Session not found: {id}")))
}

/// Prepare a complete replacement before touching any live terminals or persistent data.
pub fn prepare(
    workspace: &Workspace,
    operations: &[Operation],
    caller: Option<&str>,
    expected: Option<&str>,
) -> ApiResult<Prepared> {
    if operations.is_empty() || operations.len() > MAX_OPERATIONS {
        return Err(ApiError::new("invalid_argument", "Supply 1–128 operations"));
    }
    if expected.is_some_and(|r| r != revision(workspace)) {
        return Err(ApiError::new(
            "conflict",
            "Workspace changed; read it again before applying this plan",
        ));
    }
    let mut w = workspace.clone();
    let mut keys = BTreeMap::new();
    let mut results = Vec::new();
    for (index, op) in operations.iter().enumerate() {
        let result: ApiResult<Value> = (|| {
            let resolve = |id: &str| resolve(id, caller, &keys);
            match op {
                Operation::Create {
                    key,
                    name,
                    directory,
                    shell,
                    color,
                } => {
                    let name = label(name)?;
                    if !directory.is_absolute() || !directory.is_dir() {
                        return Err(ApiError::new(
                            "invalid_directory",
                            "Directory must be an existing absolute local or UNC directory",
                        ));
                    }
                    let id = uuid::Uuid::new_v4().to_string();
                    w.sessions.push(SessionSpec {
                        id: id.clone(),
                        name,
                        directory: directory.clone(),
                        agent: shell.agent(),
                        agent_session_id: None,
                        color: *color,
                    });
                    remember(&mut keys, key, &id)?;
                    Ok(json!({"id": id, "key": key}))
                }
                Operation::Rename { id, name } => {
                    let id = resolve(id)?;
                    let i = session_index(&w, &id)?;
                    w.sessions[i].name = label(name)?;
                    w.unnamed_sessions.remove(&id);
                    Ok(json!({"id": id}))
                }
                Operation::Color { id, color } => {
                    let id = resolve(id)?;
                    let i = session_index(&w, &id)?;
                    w.sessions[i].color = *color;
                    Ok(json!({"id": id}))
                }
                Operation::Assign { id, group } => {
                    let id = resolve(id)?;
                    session_index(&w, &id)?;
                    let group = group.as_deref().map(resolve).transpose()?;
                    if group
                        .as_ref()
                        .is_some_and(|id| !w.groups.iter().any(|g| g.id == *id))
                    {
                        return Err(ApiError::new("not_found", "Group not found"));
                    }
                    model::assign_group(&mut w.groups, &id, group.as_deref());
                    Ok(json!({"id": id, "group_id": group}))
                }
                Operation::Move { id, before, after } => {
                    let id = resolve(id)?;
                    let from = session_index(&w, &id)?;
                    let (target, after) = match (before, after) {
                        (Some(t), None) => (resolve(t)?, false),
                        (None, Some(t)) => (resolve(t)?, true),
                        _ => {
                            return Err(ApiError::new(
                                "invalid_argument",
                                "Specify exactly one of before or after",
                            ));
                        }
                    };
                    let to = session_index(&w, &target)?;
                    model::reorder(&mut w.sessions, from, to, after);
                    Ok(json!({"id": id}))
                }
                Operation::GroupCreate { key, name, members } => {
                    let name = label(name)?;
                    if members.is_empty() || members.len() > MAX_OPERATIONS {
                        return Err(ApiError::new(
                            "invalid_argument",
                            "A group needs 1–128 distinct members",
                        ));
                    }
                    let members = members
                        .iter()
                        .map(|id| resolve(id))
                        .collect::<ApiResult<Vec<_>>>()?;
                    let mut unique = HashSet::new();
                    for id in &members {
                        session_index(&w, id)?;
                        if !unique.insert(id) {
                            return Err(ApiError::new(
                                "invalid_argument",
                                "Duplicate group member",
                            ));
                        }
                    }
                    let id = uuid::Uuid::new_v4().to_string();
                    for member in &members {
                        model::assign_group(&mut w.groups, member, None);
                    }
                    w.groups.push(SessionGroup {
                        id: id.clone(),
                        name,
                        members,
                    });
                    remember(&mut keys, key, &id)?;
                    Ok(json!({"id": id, "key": key}))
                }
                Operation::GroupRename { id, name } => {
                    let id = resolve(id)?;
                    let group = w
                        .groups
                        .iter_mut()
                        .find(|g| g.id == id)
                        .ok_or_else(|| ApiError::new("not_found", "Group not found"))?;
                    group.name = label(name)?;
                    Ok(json!({"id": id}))
                }
                Operation::GroupDissolve { id } => {
                    let id = resolve(id)?;
                    let i = w
                        .groups
                        .iter()
                        .position(|g| g.id == id)
                        .ok_or_else(|| ApiError::new("not_found", "Group not found"))?;
                    w.groups.remove(i);
                    Ok(json!({"id": id}))
                }
            }
        })();
        results
            .push(result.map_err(|e| {
                ApiError::new(&e.code, format!("Operation {index}: {}", e.message))
            })?);
    }
    Ok(Prepared {
        workspace: w,
        results,
    })
}
