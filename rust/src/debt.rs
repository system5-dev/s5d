//! Simplification debt harvest.
//!
//! Decision: decision.s5d.simplification-debt-marker (H3). Constitution #3
//! mandates that temporary measures be explicitly marked with a removal plan.
//! The marker is an inline comment written at the cut-site, carrying a `ceiling`
//! argument (what was cut) and a `trigger` argument (when to revisit). See
//! SKILL.md for the literal syntax. A marker with an empty or absent trigger is
//! reported as `no-trigger` — it rots silently.
//!
//! Harvest is read-only: it walks the tree, binds each marker's file to its
//! owning component/spec via the same component-path resolver the architecture
//! check uses, and reports grouped by owner. A marker in a file not covered by
//! any component is reported as `unowned`. The marker must sit behind a comment
//! leader (`//`, `#`, `/*`, `<!--`), so prose mentions of the token do not match.

use crate::project::S5dProject;
use ignore::WalkBuilder;
use regex::Regex;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// One harvested simplification marker.
#[derive(Debug, Clone)]
pub struct DebtMarker {
    /// Repo-relative path of the file holding the marker.
    pub file: String,
    /// 1-based line number.
    pub line: usize,
    /// What was cut (the `ceiling=` argument).
    pub ceiling: String,
    /// When to revisit (the `trigger=` argument); `None` or empty => no-trigger.
    pub trigger: Option<String>,
    /// Owning `feature / component`, or `None` when no component covers the file.
    pub owner: Option<String>,
}

impl DebtMarker {
    /// A marker is trigger-less when the trigger argument is missing or blank.
    pub fn is_no_trigger(&self) -> bool {
        self.trigger.as_deref().map(str::trim).unwrap_or("").is_empty()
    }

    pub fn is_unowned(&self) -> bool {
        self.owner.is_none()
    }
}

/// Result of a harvest run.
#[derive(Debug, Clone, Default)]
pub struct DebtReport {
    pub markers: Vec<DebtMarker>,
}

impl DebtReport {
    pub fn no_trigger_count(&self) -> usize {
        self.markers.iter().filter(|m| m.is_no_trigger()).count()
    }

    pub fn unowned_count(&self) -> usize {
        self.markers.iter().filter(|m| m.is_unowned()).count()
    }
}

/// Harvest all `s5d:debt(...)` markers in the project (read-only).
pub fn harvest(project: &S5dProject) -> anyhow::Result<DebtReport> {
    let file_owner = build_file_owner_index(project)?;

    // Marker must sit behind a comment leader (//, #, /*, <!--) so prose
    // mentions of the token do not match. Comment-syntax-agnostic across langs.
    let marker_re = Regex::new(r"(?://|#|/\*|<!--)\s*s5d:debt\(([^)]*)\)")?;
    let ceiling_re = Regex::new(r#"ceiling\s*=\s*"([^"]*)""#)?;
    let trigger_re = Regex::new(r#"trigger\s*=\s*"([^"]*)""#)?;

    let mut markers = Vec::new();
    // WalkBuilder respects .gitignore (skips target/, node_modules/) and hidden
    // entries (skips .git/, .s5d/) by default.
    for result in WalkBuilder::new(&project.root).build() {
        let entry = match result {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let path = entry.path();
        // Don't harvest this module's own doc examples.
        let Ok(content) = std::fs::read_to_string(path) else {
            continue; // binary or unreadable — markers live in text
        };
        if !content.contains("s5d:debt(") {
            continue;
        }
        let owner = lookup_owner(&file_owner, path);
        let rel = display_rel(&project.root, path);
        for (idx, raw) in content.lines().enumerate() {
            let Some(args) = marker_re.captures(raw).and_then(|c| c.get(1)) else {
                continue;
            };
            let args = args.as_str();
            let ceiling = ceiling_re
                .captures(args)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let trigger = trigger_re
                .captures(args)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string());
            markers.push(DebtMarker {
                file: rel.clone(),
                line: idx + 1,
                ceiling,
                trigger,
                owner: owner.clone(),
            });
        }
    }

    markers.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    Ok(DebtReport { markers })
}

/// Build a `canonical file path -> "feature / component"` index from every
/// spec's component paths, reusing the architecture resolver.
fn build_file_owner_index(project: &S5dProject) -> anyhow::Result<HashMap<PathBuf, String>> {
    let mut index = HashMap::new();
    for (_path, spec) in project.discover_specs()? {
        let Some(artifacts) = spec.artifacts.as_ref() else {
            continue;
        };
        for component in &artifacts.components {
            let owner = format!("{} / {}", component.feature, component.id);
            for pattern in &component.paths {
                let Ok(files) = crate::arch::resolve_component_path(&project.root, pattern) else {
                    continue;
                };
                for file in files {
                    let key = file.canonicalize().unwrap_or(file);
                    index.entry(key).or_insert_with(|| owner.clone());
                }
            }
        }
    }
    Ok(index)
}

fn lookup_owner(index: &HashMap<PathBuf, String>, path: &Path) -> Option<String> {
    let key = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    index.get(&key).cloned()
}

fn display_rel(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

/// Render a text report grouped by owner. Trigger-less and unowned markers are
/// flagged.
pub fn format_text(report: &DebtReport) -> String {
    use std::fmt::Write;

    if report.markers.is_empty() {
        return "No s5d:debt markers. Clean ledger.".to_string();
    }

    // Group by owner, preserving a stable order; unowned last.
    let mut by_owner: BTreeMap<String, Vec<&DebtMarker>> = BTreeMap::new();
    for marker in &report.markers {
        let key = marker.owner.clone().unwrap_or_else(|| "unowned".to_string());
        by_owner.entry(key).or_default().push(marker);
    }

    let mut out = String::new();
    for (owner, markers) in &by_owner {
        let _ = writeln!(out, "{} ({} marker(s))", owner, markers.len());
        for marker in markers {
            let trigger = if marker.is_no_trigger() {
                "no-trigger".to_string()
            } else {
                marker.trigger.clone().unwrap_or_default()
            };
            let _ = writeln!(
                out,
                "  {}:{}  ceiling: {}  trigger: {}",
                marker.file, marker.line, marker.ceiling, trigger
            );
        }
    }

    let _ = writeln!(
        out,
        "\n{} marker(s), {} no-trigger, {} unowned",
        report.markers.len(),
        report.no_trigger_count(),
        report.unowned_count()
    );
    out
}
