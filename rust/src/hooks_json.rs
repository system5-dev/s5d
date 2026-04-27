//! S5D-Spec: feat.s5d.pretool-enforcement
//!
//! Pure-Rust idempotent installer for the PreToolUse hook entry in a
//! project's hooks.json files. Mirrors the `ensure_agents_md` pattern.
//! NO shell scripts — the registered command string is `s5d hook pretool-edit`
//! invoked directly.
//!
//! Targets (project-local only — user-global install is out of scope here):
//!   - .claude-plugin/hooks.json (Claude Code marketplace plugin layout)
//!   - .codex-plugin/hooks.json  (Codex marketplace plugin layout)
//!   - hooks/hooks.json          (existing s5d project layout, if present)

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const PRETOOL_HOOK_COMMAND: &str = "s5d hook pretool-edit";
pub const PRETOOL_HOOK_MATCHER: &str = "Edit|Write|MultiEdit";
pub const PRETOOL_TIMEOUT_MS: u64 = 5_000;

#[derive(Debug, PartialEq, Eq)]
pub enum HooksJsonUpdate {
    Created,
    Inserted,
    Unchanged,
}

/// Ensure the PreToolUse hook entry exists in the given hooks.json file.
/// Idempotent: re-runs return Unchanged. Preserves any unrelated existing entries.
pub fn ensure_pretool_hook(path: &Path) -> Result<HooksJsonUpdate> {
    let mut existing: Value = if path.exists() {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("read {}", path.display()))?;
        if raw.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str(&raw)
                .with_context(|| format!("parse {}", path.display()))?
        }
    } else {
        json!({})
    };

    let was_created = !path.exists();
    let already = inject_pretool_entry(&mut existing);

    if already && !was_created {
        return Ok(HooksJsonUpdate::Unchanged);
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let pretty = serde_json::to_string_pretty(&existing)?;
    std::fs::write(path, format!("{}\n", pretty))
        .with_context(|| format!("write {}", path.display()))?;

    if was_created {
        Ok(HooksJsonUpdate::Created)
    } else {
        Ok(HooksJsonUpdate::Inserted)
    }
}

/// Mutates `root` to ensure a PreToolUse(Edit|Write|MultiEdit) hook entry
/// pointing at `s5d hook pretool-edit` exists. Returns true if the entry
/// was already present (no mutation needed).
fn inject_pretool_entry(root: &mut Value) -> bool {
    let root_obj = match root.as_object_mut() {
        Some(o) => o,
        None => {
            *root = json!({});
            root.as_object_mut().unwrap()
        }
    };
    let hooks = root_obj
        .entry("hooks".to_string())
        .or_insert_with(|| json!({}));
    let hooks_obj = match hooks.as_object_mut() {
        Some(o) => o,
        None => {
            *hooks = json!({});
            hooks.as_object_mut().unwrap()
        }
    };
    let pretool = hooks_obj
        .entry("PreToolUse".to_string())
        .or_insert_with(|| json!([]));
    let pretool_arr = match pretool.as_array_mut() {
        Some(a) => a,
        None => {
            *pretool = json!([]);
            pretool.as_array_mut().unwrap()
        }
    };

    // Search for an existing matcher group with our matcher
    for entry in pretool_arr.iter_mut() {
        if entry.get("matcher").and_then(|v| v.as_str()) == Some(PRETOOL_HOOK_MATCHER) {
            // Group exists — check if our command is already registered
            let group_hooks = entry
                .get_mut("hooks")
                .and_then(|v| v.as_array_mut());
            if let Some(group_hooks) = group_hooks {
                let already = group_hooks.iter().any(|h| {
                    h.get("command").and_then(|v| v.as_str()) == Some(PRETOOL_HOOK_COMMAND)
                });
                if already {
                    return true;
                }
                group_hooks.push(make_hook_entry());
                return false;
            } else {
                entry["hooks"] = json!([make_hook_entry()]);
                return false;
            }
        }
    }

    // No matching group — append a new one
    pretool_arr.push(json!({
        "matcher": PRETOOL_HOOK_MATCHER,
        "hooks": [make_hook_entry()]
    }));
    false
}

fn make_hook_entry() -> Value {
    json!({
        "type": "command",
        "command": PRETOOL_HOOK_COMMAND,
        "timeout": PRETOOL_TIMEOUT_MS,
    })
}

/// Discover hooks.json files to update in the given project root.
/// Always includes the canonical Claude/Codex plugin paths; includes the
/// existing s5d hooks/hooks.json only if it's already present (we don't
/// create that legacy layout).
pub fn target_hooks_paths(project_root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    out.push(project_root.join(".claude-plugin").join("hooks.json"));
    out.push(project_root.join(".codex-plugin").join("hooks.json"));
    let legacy = project_root.join("hooks").join("hooks.json");
    if legacy.exists() {
        out.push(legacy);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_file_when_absent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".claude-plugin/hooks.json");
        assert_eq!(ensure_pretool_hook(&path).unwrap(), HooksJsonUpdate::Created);
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains(PRETOOL_HOOK_COMMAND));
        assert!(body.contains(PRETOOL_HOOK_MATCHER));
        // No bash/shell indirection
        assert!(!body.contains("bash "));
        assert!(!body.contains(".sh"));
    }

    #[test]
    fn idempotent_on_second_run() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("hooks.json");
        ensure_pretool_hook(&path).unwrap();
        assert_eq!(
            ensure_pretool_hook(&path).unwrap(),
            HooksJsonUpdate::Unchanged
        );
    }

    #[test]
    fn preserves_existing_unrelated_entries() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("hooks.json");
        let preexisting = json!({
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Bash",
                        "hooks": [{"type":"command","command":"my-other-check"}]
                    }
                ],
                "SessionStart": [
                    {"matcher":"*","hooks":[{"type":"command","command":"foo"}]}
                ]
            }
        });
        std::fs::write(&path, serde_json::to_string_pretty(&preexisting).unwrap()).unwrap();

        assert_eq!(ensure_pretool_hook(&path).unwrap(), HooksJsonUpdate::Inserted);
        let body = std::fs::read_to_string(&path).unwrap();
        // Old Bash entry preserved
        assert!(body.contains("my-other-check"));
        // SessionStart preserved
        assert!(body.contains("foo"));
        // Our new entry added
        assert!(body.contains(PRETOOL_HOOK_COMMAND));
    }

    #[test]
    fn merges_into_existing_matcher_group() {
        // If an Edit|Write|MultiEdit group already exists with a different
        // command, we should append into it, not duplicate the matcher.
        let dir = tempdir().unwrap();
        let path = dir.path().join("hooks.json");
        let preexisting = json!({
            "hooks": {
                "PreToolUse": [{
                    "matcher": PRETOOL_HOOK_MATCHER,
                    "hooks": [{"type":"command","command":"some-formatter"}]
                }]
            }
        });
        std::fs::write(&path, serde_json::to_string_pretty(&preexisting).unwrap()).unwrap();

        ensure_pretool_hook(&path).unwrap();
        let updated: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let group = &updated["hooks"]["PreToolUse"][0]["hooks"];
        assert_eq!(group.as_array().unwrap().len(), 2, "should append, not replace");
        let cmds: Vec<&str> = group.as_array().unwrap()
            .iter()
            .filter_map(|h| h.get("command").and_then(|v| v.as_str()))
            .collect();
        assert!(cmds.contains(&"some-formatter"));
        assert!(cmds.contains(&PRETOOL_HOOK_COMMAND));
    }

    #[test]
    fn target_paths_include_plugin_dirs() {
        let dir = tempdir().unwrap();
        let paths = target_hooks_paths(dir.path());
        assert!(paths.iter().any(|p| p.ends_with(".claude-plugin/hooks.json")));
        assert!(paths.iter().any(|p| p.ends_with(".codex-plugin/hooks.json")));
    }

    #[test]
    fn target_paths_includes_legacy_only_if_present() {
        let dir = tempdir().unwrap();
        // Without legacy file
        let paths = target_hooks_paths(dir.path());
        assert!(!paths.iter().any(|p| p.ends_with("hooks/hooks.json")));

        // With legacy file
        std::fs::create_dir_all(dir.path().join("hooks")).unwrap();
        std::fs::write(dir.path().join("hooks/hooks.json"), "{}").unwrap();
        let paths = target_hooks_paths(dir.path());
        assert!(paths.iter().any(|p| p.ends_with("hooks/hooks.json")));
    }
}
