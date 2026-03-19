use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct GitState {
    pub spec_tracked: bool,
    pub on_main: bool,
    pub has_pr: bool,
    pub pr_approved: bool,
    pub merged_to_main: bool,
    pub last_commit_msg: Option<String>,
}

/// Check git state for a spec file
pub fn check_git_state(spec_path: &Path) -> Result<GitState> {
    let dir = spec_path.parent().unwrap_or(Path::new("."));

    // Is file tracked by git?
    let tracked = Command::new("git")
        .args(["ls-files", "--error-unmatch"])
        .arg(spec_path)
        .current_dir(dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    // Current branch
    let branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(dir)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let on_main = branch == "main" || branch == "master";

    // Check if spec file exists on main
    let merged = Command::new("git")
        .args(["log", "main", "--oneline", "-1", "--"])
        .arg(spec_path)
        .current_dir(dir)
        .output()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false);

    // Last commit message touching this file
    let last_msg = Command::new("git")
        .args(["log", "-1", "--format=%s", "--"])
        .arg(spec_path)
        .current_dir(dir)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    Ok(GitState {
        spec_tracked: tracked,
        on_main,
        has_pr: false, // Would need gh CLI integration
        pr_approved: false,
        merged_to_main: merged,
        last_commit_msg: last_msg,
    })
}

/// For each domain directory, infer the top committer from git history.
/// Returns a map of domain_id → author name.
pub fn infer_domain_owners(
    root: &Path,
    domain_paths: &[(String, PathBuf)],
) -> HashMap<String, String> {
    let mut owners = HashMap::new();
    for (domain_id, path) in domain_paths {
        // Use relative path from root for the git query
        let rel = path.strip_prefix(root).unwrap_or(path);
        let output = Command::new("git")
            .args(["shortlog", "-sn", "--no-merges", "HEAD", "--"])
            .arg(rel)
            .current_dir(root)
            .output();
        if let Ok(out) = output {
            if let Ok(stdout) = String::from_utf8(out.stdout) {
                // First line = top committer: "    42\tJohn Doe"
                if let Some(line) = stdout.lines().next() {
                    if let Some(name) = line.split('\t').nth(1) {
                        let name = name.trim();
                        if !name.is_empty() {
                            owners.insert(domain_id.clone(), name.to_string());
                        }
                    }
                }
            }
        }
    }
    owners
}

/// Derive suggested S5D phase from git state
pub fn suggested_phase(git: &GitState) -> &'static str {
    if !git.spec_tracked {
        "Not in git — commit the spec first"
    } else if git.merged_to_main {
        "Merged to main — ready for EXECUTE (import)"
    } else if git.on_main {
        "On main branch — spec needs a feature branch for review"
    } else {
        "On feature branch — create PR for APPROVE"
    }
}
