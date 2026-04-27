//! S5D-Spec: feat.s5d.pretool-enforcement
//!
//! L2 PreToolUse enforcement gate. Decides whether an Edit/Write/MultiEdit
//! tool call should be approved or blocked based on:
//!   1. Whether `.s5d/` exists in the project (opt-in).
//!   2. Whether the file matches a trivial allowlist (docs/config/tests).
//!   3. Whether the file is covered by an approved spec (paths declared in
//!      `artifacts.components[].paths` or `workflow.structure_outline.files`).
//!   4. Whether accumulated session edits crossed the threshold without spec.
//!
//! See spec: `.s5d/packages/feat.s5d.pretool-enforcement__20260427.s5d.yaml`

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Default thresholds — overridable via `.s5d/config.toml` (h1).
pub const DEFAULT_MAX_LOC: usize = 30;
pub const DEFAULT_MAX_FILES: usize = 4;

/// Decision returned by the gate. Serialized as JSON for hook consumption.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum GateDecision {
    Approve,
    Block { reason: String },
}

/// Per-Claude-session counter persisted at `.s5d/.session-counter-<id>.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionState {
    pub session_id: String,
    pub files: BTreeSet<PathBuf>,
    pub loc_delta: usize,
    pub started: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct ThresholdConfig {
    pub max_loc: usize,
    pub max_files: usize,
}

impl Default for ThresholdConfig {
    fn default() -> Self {
        Self { max_loc: DEFAULT_MAX_LOC, max_files: DEFAULT_MAX_FILES }
    }
}

/// Suffix-based trivial allowlist (h1 — refine via measurement).
/// Matches by filename or path-suffix; case-insensitive on extension.
pub fn matches_trivial_allowlist(path: &Path) -> bool {
    let s = path.to_string_lossy().to_lowercase();

    // Lockfiles
    for lock in ["cargo.lock", "package-lock.json", "yarn.lock", "uv.lock", "poetry.lock"] {
        if s.ends_with(lock) {
            return true;
        }
    }

    // Special files (full filename match)
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        let n = name.to_lowercase();
        if matches!(n.as_str(), ".gitignore" | ".gitattributes" | "readme" | "license" | "license.md")
            || n.starts_with(".env")
            || n.starts_with("readme.")
            || n.starts_with("license.")
        {
            return true;
        }
    }

    // Extensions: docs + config
    let trivial_exts = [
        ".md", ".rst", ".txt",                          // docs
        ".toml", ".yaml", ".yml", ".json", ".ini",      // config
        ".cfg", ".conf",
    ];
    for ext in trivial_exts {
        if s.ends_with(ext) {
            return true;
        }
    }

    // Test paths (handle both leading-slash and start-of-path)
    let path_str = s.as_str();
    if path_str.contains("/tests/")
        || path_str.contains("/test/")
        || path_str.contains("/spec/")
        || path_str.starts_with("tests/")
        || path_str.starts_with("test/")
        || path_str.starts_with("spec/")
        || path_str.ends_with("_test.rs")
        || path_str.ends_with("_test.py")
        || path_str.ends_with(".test.ts")
        || path_str.ends_with(".test.js")
        || path_str.ends_with(".spec.ts")
        || path_str.ends_with(".spec.js")
    {
        return true;
    }

    // .s5d/ directory itself (specs are docs)
    if path_str.contains("/.s5d/") || path_str.starts_with(".s5d/") {
        return true;
    }

    false
}

/// Returns spec ID if any spec in `.s5d/packages/` declares this file path
/// in its components or structure_outline. Best-effort substring match.
pub fn covered_by_spec(s5d_dir: &Path, file: &Path) -> Option<String> {
    let packages_dir = s5d_dir.join("packages");
    let entries = std::fs::read_dir(&packages_dir).ok()?;
    let needle = file.to_string_lossy().to_string();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else { continue };

        // Best-effort: spec id is line `id: feat.foo.bar`
        let spec_id = content
            .lines()
            .find_map(|l| l.strip_prefix("id: ").map(|s| s.trim().to_string()));

        // Search for the file path mention in the spec body. Conservative:
        // only count it as covered if the spec literally names the file.
        if content.contains(&needle) {
            if let Some(id) = spec_id {
                return Some(id);
            }
        }
    }
    None
}

/// Pure decision function. Caller is responsible for loading/saving session state
/// and for computing the post-edit `loc_delta`.
pub fn evaluate_edit(
    s5d_dir: Option<&Path>,
    file: &Path,
    loc_delta: usize,
    session: &SessionState,
    config: &ThresholdConfig,
) -> GateDecision {
    // Branch 1: project hasn't opted in to s5d
    let Some(s5d_dir) = s5d_dir else {
        return GateDecision::Approve;
    };

    // Branch 2: trivial allowlist
    if matches_trivial_allowlist(file) {
        return GateDecision::Approve;
    }

    // Branch 3: covered by an existing spec
    if covered_by_spec(s5d_dir, file).is_some() {
        return GateDecision::Approve;
    }

    // Branch 4: would crossing the threshold land here?
    let projected_loc = session.loc_delta + loc_delta;
    let projected_files = {
        let mut s = session.files.clone();
        s.insert(file.to_path_buf());
        s.len()
    };

    if projected_loc > config.max_loc || projected_files > config.max_files {
        return GateDecision::Block {
            reason: format!(
                "S5D enforcement: non-trivial scope without spec ({} files, +{} LOC across session, > {}f/{}LOC threshold). \
                 Run `s5d new <feature-id> --product <name>` to declare a spec, or `s5d note <rationale>` for a one-off, \
                 or set S5D_BYPASS=1 to bypass this single tool call.",
                projected_files, projected_loc, config.max_files, config.max_loc
            ),
        };
    }

    // Branch 5: under threshold, approve and let the caller increment counter
    GateDecision::Approve
}

/// Load session state from `<s5d_dir>/.session-counter-<session_id>.json`.
/// Missing file → fresh state.
pub fn load_session_state(s5d_dir: &Path, session_id: &str) -> Result<SessionState> {
    let path = session_state_path(s5d_dir, session_id);
    if !path.exists() {
        return Ok(SessionState {
            session_id: session_id.to_string(),
            started: Some(Utc::now()),
            ..Default::default()
        });
    }
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("read session state {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("parse session state {}", path.display()))
}

pub fn save_session_state(s5d_dir: &Path, state: &SessionState) -> Result<()> {
    let path = session_state_path(s5d_dir, &state.session_id);
    std::fs::create_dir_all(s5d_dir).ok();
    let raw = serde_json::to_string_pretty(state)?;
    std::fs::write(&path, raw)
        .with_context(|| format!("write session state {}", path.display()))?;
    Ok(())
}

fn session_state_path(s5d_dir: &Path, session_id: &str) -> PathBuf {
    // Sanitize session_id — no path separators, no dots leading
    let safe: String = session_id
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    s5d_dir.join(format!(".session-counter-{}.json", safe))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn empty_session() -> SessionState {
        SessionState::default()
    }

    fn cfg() -> ThresholdConfig {
        ThresholdConfig::default()
    }

    #[test]
    fn branch1_no_s5d_dir_approves() {
        // Project hasn't opted in — gate is invisible
        let dec = evaluate_edit(
            None,
            Path::new("src/main.rs"),
            500,
            &empty_session(),
            &cfg(),
        );
        assert_eq!(dec, GateDecision::Approve);
    }

    #[test]
    fn branch2_trivial_allowlist_approves() {
        let dir = tempdir().unwrap();
        let s5d = dir.path();

        for trivial in [
            "README.md",
            "docs/setup.md",
            "config.toml",
            "Cargo.lock",
            ".gitignore",
            ".env.production",
            "tests/integration.rs",
            "src/foo_test.rs",
            ".s5d/packages/feat.bar.yaml",
        ] {
            let dec = evaluate_edit(Some(s5d), Path::new(trivial), 9999, &empty_session(), &cfg());
            assert_eq!(dec, GateDecision::Approve, "expected approve for {}", trivial);
        }
    }

    #[test]
    fn branch3_covered_by_spec_approves() {
        let dir = tempdir().unwrap();
        let s5d = dir.path();
        std::fs::create_dir_all(s5d.join("packages")).unwrap();
        std::fs::write(
            s5d.join("packages/feat.example.yaml"),
            "id: feat.example\nworkflow:\n  structure_outline:\n    files:\n    - src/critical.rs\n",
        )
        .unwrap();

        let dec = evaluate_edit(
            Some(s5d),
            Path::new("src/critical.rs"),
            500,
            &empty_session(),
            &cfg(),
        );
        assert_eq!(dec, GateDecision::Approve);
    }

    #[test]
    fn branch4_threshold_crossed_blocks() {
        let dir = tempdir().unwrap();
        let s5d = dir.path();
        std::fs::create_dir_all(s5d.join("packages")).unwrap();

        let session = SessionState {
            loc_delta: 25,
            ..Default::default()
        };
        // 25 + 10 = 35 > 30 → block
        let dec = evaluate_edit(Some(s5d), Path::new("src/hot.rs"), 10, &session, &cfg());
        match dec {
            GateDecision::Block { reason } => {
                assert!(reason.contains("non-trivial scope without spec"));
                assert!(reason.contains("S5D_BYPASS=1"));
            }
            _ => panic!("expected Block, got Approve"),
        }
    }

    #[test]
    fn branch4_files_threshold_crossed_blocks() {
        let dir = tempdir().unwrap();
        let s5d = dir.path();
        std::fs::create_dir_all(s5d.join("packages")).unwrap();

        let mut session = SessionState::default();
        for f in ["a.rs", "b.rs", "c.rs", "d.rs"] {
            session.files.insert(PathBuf::from(format!("src/{}", f)));
        }
        // session has 4 files, adding 5th unique → > 4 → block
        let dec = evaluate_edit(Some(s5d), Path::new("src/e.rs"), 1, &session, &cfg());
        assert!(matches!(dec, GateDecision::Block { .. }));
    }

    #[test]
    fn branch5_under_threshold_approves() {
        let dir = tempdir().unwrap();
        let s5d = dir.path();
        std::fs::create_dir_all(s5d.join("packages")).unwrap();

        let dec = evaluate_edit(
            Some(s5d),
            Path::new("src/small.rs"),
            5,
            &empty_session(),
            &cfg(),
        );
        assert_eq!(dec, GateDecision::Approve);
    }

    #[test]
    fn session_state_roundtrip() {
        let dir = tempdir().unwrap();
        let s5d = dir.path();
        let mut state = load_session_state(s5d, "sess-abc-123").unwrap();
        assert_eq!(state.loc_delta, 0);
        assert!(state.files.is_empty());

        state.loc_delta = 17;
        state.files.insert(PathBuf::from("src/foo.rs"));
        save_session_state(s5d, &state).unwrap();

        let loaded = load_session_state(s5d, "sess-abc-123").unwrap();
        assert_eq!(loaded.loc_delta, 17);
        assert_eq!(loaded.files.len(), 1);
    }

    #[test]
    fn session_id_path_sanitization() {
        let dir = tempdir().unwrap();
        let s5d = dir.path();
        // Path traversal attempt — must be sanitized
        let state = SessionState {
            session_id: "../../etc/passwd".to_string(),
            ..Default::default()
        };
        save_session_state(s5d, &state).unwrap();
        let written = session_state_path(s5d, &state.session_id);
        // Should not escape s5d dir
        assert!(written.starts_with(s5d));
        // Slashes and dots replaced with underscores
        assert!(!written.to_string_lossy().contains(".."));
    }
}
