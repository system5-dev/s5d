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

pub const USER_PROMPT_HOOK_COMMAND: &str = "s5d hook user-prompt-submit";
pub const USER_PROMPT_TIMEOUT_MS: u64 = 5_000;

pub const REQUIRE_SPEC_HOOK_COMMAND: &str = "s5d hook require-spec";
pub const REQUIRE_SPEC_HOOK_MATCHER: &str = "Bash";
pub const REQUIRE_SPEC_TIMEOUT_MS: u64 = 10_000;

#[derive(Debug, PartialEq, Eq)]
pub enum HooksJsonUpdate {
    Created,
    Inserted,
    Unchanged,
}

/// Ensure the PreToolUse hook entry exists in the given hooks.json file.
/// Idempotent: re-runs return Unchanged. Preserves any unrelated existing entries.
pub fn ensure_pretool_hook(path: &Path) -> Result<HooksJsonUpdate> {
    ensure_hook_entries(path, &[
        HookSpec::Matched {
            event: "PreToolUse",
            matcher: PRETOOL_HOOK_MATCHER,
            command: PRETOOL_HOOK_COMMAND,
            timeout_ms: PRETOOL_TIMEOUT_MS,
        },
    ])
}

/// Ensure all three S5D enforcement hooks are registered:
///   * L1: UserPromptSubmit advisory (Phase 3)
///   * L2: PreToolUse(Edit|Write|MultiEdit) gate (Phase 2)
///   * L3: PreToolUse(Bash) require-spec — pure-Rust replacement for
///         hooks/require-spec.sh (Phase 4 / l3-migration)
/// Single write per file. Used by `s5d init`.
pub fn ensure_all_s5d_hooks(path: &Path) -> Result<HooksJsonUpdate> {
    ensure_hook_entries(path, &[
        HookSpec::Matched {
            event: "PreToolUse",
            matcher: PRETOOL_HOOK_MATCHER,
            command: PRETOOL_HOOK_COMMAND,
            timeout_ms: PRETOOL_TIMEOUT_MS,
        },
        HookSpec::Matched {
            event: "PreToolUse",
            matcher: REQUIRE_SPEC_HOOK_MATCHER,
            command: REQUIRE_SPEC_HOOK_COMMAND,
            timeout_ms: REQUIRE_SPEC_TIMEOUT_MS,
        },
        HookSpec::Unmatched {
            event: "UserPromptSubmit",
            command: USER_PROMPT_HOOK_COMMAND,
            timeout_ms: USER_PROMPT_TIMEOUT_MS,
        },
    ])
}

/// Internal hook descriptor — either a matcher-keyed event (PreToolUse)
/// or a flat event (UserPromptSubmit which has no tool_name to match on).
enum HookSpec {
    Matched {
        event: &'static str,
        matcher: &'static str,
        command: &'static str,
        timeout_ms: u64,
    },
    Unmatched {
        event: &'static str,
        command: &'static str,
        timeout_ms: u64,
    },
}

fn ensure_hook_entries(path: &Path, specs: &[HookSpec]) -> Result<HooksJsonUpdate> {
    let was_created = !path.exists();
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

    let mut all_already = true;
    for spec in specs {
        let was_already = match spec {
            HookSpec::Matched { event, matcher, command, timeout_ms } => {
                inject_matched_entry(&mut existing, event, matcher, command, *timeout_ms)
            }
            HookSpec::Unmatched { event, command, timeout_ms } => {
                inject_unmatched_entry(&mut existing, event, command, *timeout_ms)
            }
        };
        if !was_already {
            all_already = false;
        }
    }

    if all_already && !was_created {
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

/// Mutates `root` to ensure a matched-event hook entry (e.g. PreToolUse)
/// exists. Returns true if already present.
fn inject_matched_entry(
    root: &mut Value,
    event: &str,
    matcher: &str,
    command: &str,
    timeout_ms: u64,
) -> bool {
    let event_arr = ensure_event_array(root, event);
    for entry in event_arr.iter_mut() {
        if entry.get("matcher").and_then(|v| v.as_str()) == Some(matcher) {
            let group_hooks = entry.get_mut("hooks").and_then(|v| v.as_array_mut());
            if let Some(group_hooks) = group_hooks {
                let already = group_hooks
                    .iter()
                    .any(|h| h.get("command").and_then(|v| v.as_str()) == Some(command));
                if already {
                    return true;
                }
                group_hooks.push(make_hook_entry(command, timeout_ms));
                return false;
            } else {
                entry["hooks"] = json!([make_hook_entry(command, timeout_ms)]);
                return false;
            }
        }
    }
    event_arr.push(json!({
        "matcher": matcher,
        "hooks": [make_hook_entry(command, timeout_ms)]
    }));
    false
}

/// Mutates `root` to ensure an unmatched-event hook entry (e.g. UserPromptSubmit)
/// exists. These events have no `tool_name` so no `matcher` field. We register
/// under a single group with no matcher key (Claude Code accepts that shape).
fn inject_unmatched_entry(
    root: &mut Value,
    event: &str,
    command: &str,
    timeout_ms: u64,
) -> bool {
    let event_arr = ensure_event_array(root, event);
    for entry in event_arr.iter_mut() {
        if entry.get("matcher").is_none() || entry.get("matcher").and_then(|v| v.as_str()) == Some("*") {
            let group_hooks = entry.get_mut("hooks").and_then(|v| v.as_array_mut());
            if let Some(group_hooks) = group_hooks {
                let already = group_hooks
                    .iter()
                    .any(|h| h.get("command").and_then(|v| v.as_str()) == Some(command));
                if already {
                    return true;
                }
                group_hooks.push(make_hook_entry(command, timeout_ms));
                return false;
            }
        }
    }
    event_arr.push(json!({
        "hooks": [make_hook_entry(command, timeout_ms)]
    }));
    false
}

fn ensure_event_array<'a>(root: &'a mut Value, event: &str) -> &'a mut Vec<Value> {
    if !root.is_object() {
        *root = json!({});
    }
    let root_obj = root.as_object_mut().unwrap();
    let hooks = root_obj.entry("hooks".to_string()).or_insert_with(|| json!({}));
    if !hooks.is_object() {
        *hooks = json!({});
    }
    let hooks_obj = hooks.as_object_mut().unwrap();
    let event_val = hooks_obj.entry(event.to_string()).or_insert_with(|| json!([]));
    if !event_val.is_array() {
        *event_val = json!([]);
    }
    event_val.as_array_mut().unwrap()
}

fn make_hook_entry(command: &str, timeout_ms: u64) -> Value {
    json!({
        "type": "command",
        "command": command,
        "timeout": timeout_ms,
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

    #[test]
    fn ensure_all_writes_all_three_layers() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("hooks.json");
        assert_eq!(ensure_all_s5d_hooks(&path).unwrap(), HooksJsonUpdate::Created);
        let body = std::fs::read_to_string(&path).unwrap();
        // L2 + L3 are both PreToolUse, different matchers
        assert!(body.contains(PRETOOL_HOOK_COMMAND), "L2 missing");
        assert!(body.contains(REQUIRE_SPEC_HOOK_COMMAND), "L3 missing");
        assert!(body.contains(USER_PROMPT_HOOK_COMMAND), "L1 missing");
        assert!(body.contains("UserPromptSubmit"));
        assert!(body.contains(PRETOOL_HOOK_MATCHER));
        assert!(body.contains(REQUIRE_SPEC_HOOK_MATCHER));
        assert!(!body.contains("bash "), "shell wrapper leaked");
        assert!(!body.contains(".sh"), "shell wrapper leaked");
    }

    #[test]
    fn l2_and_l3_share_pretooluse_event() {
        // Both PreToolUse hooks should land in the SAME PreToolUse array,
        // as separate matcher groups, not duplicate events.
        let dir = tempdir().unwrap();
        let path = dir.path().join("hooks.json");
        ensure_all_s5d_hooks(&path).unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let pretool_arr = v["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pretool_arr.len(), 2, "expected 2 matcher groups under PreToolUse");
        let matchers: Vec<&str> = pretool_arr
            .iter()
            .filter_map(|g| g.get("matcher").and_then(|m| m.as_str()))
            .collect();
        assert!(matchers.contains(&PRETOOL_HOOK_MATCHER));
        assert!(matchers.contains(&REQUIRE_SPEC_HOOK_MATCHER));
    }

    #[test]
    fn ensure_all_idempotent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("hooks.json");
        ensure_all_s5d_hooks(&path).unwrap();
        assert_eq!(
            ensure_all_s5d_hooks(&path).unwrap(),
            HooksJsonUpdate::Unchanged
        );
    }

    #[test]
    fn ensure_all_partial_existing() {
        // Phase 2 hook present, Phase 3 missing → should add only Phase 3.
        let dir = tempdir().unwrap();
        let path = dir.path().join("hooks.json");
        ensure_pretool_hook(&path).unwrap();
        // Now expand to all hooks — should write (Inserted), not Unchanged.
        assert_eq!(
            ensure_all_s5d_hooks(&path).unwrap(),
            HooksJsonUpdate::Inserted
        );
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains(PRETOOL_HOOK_COMMAND));
        assert!(body.contains(USER_PROMPT_HOOK_COMMAND));
    }
}
