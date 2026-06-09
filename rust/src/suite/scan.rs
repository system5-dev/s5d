//! Shared repo scanning core used by `ddd` and `scaling` analyzers.
//!
//! Walk is deterministic and NOT gitignore-driven: `.git_ignore(false)`,
//! `.git_global(false)`, `.git_exclude(false)`, `.hidden(false)`. Pruned
//! directory names are fixed at build time; the cap prevents OOM on huge repos.

use anyhow::Result;
use regex::Regex;
use std::path::{Path, PathBuf};

/// Maximum number of files indexed by one scan.
const FILE_CAP: usize = 20_000;

/// Maximum file size read for grep (1 MiB).
const MAX_FILE_BYTES: u64 = 1024 * 1024;

/// Directories pruned at any depth.
const PRUNED_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "vendor",
    "target",
    "build",
    "dist",
    ".next",
    ".venv",
    "__pycache__",
    ".claude",
    ".codex",
    ".agents",
    "test-reports",
    ".playwright-browsers",
];

/// Detected tech stacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stack {
    TypeScript,
    JavaScript,
    Rust,
    Python,
    Go,
}

impl Stack {
    pub fn as_str(self) -> &'static str {
        match self {
            Stack::TypeScript => "typescript",
            Stack::JavaScript => "javascript",
            Stack::Rust => "rust",
            Stack::Python => "python",
            Stack::Go => "go",
        }
    }
}

/// One line from a grep result.
#[derive(Debug, Clone)]
pub struct GrepHit {
    pub path: PathBuf,
    pub line_no: usize,
    pub line: String,
}

/// The result of scanning a repo root.
pub struct RepoScan {
    pub root: PathBuf,
    /// All non-pruned files found, relative to root.
    pub files: Vec<PathBuf>,
    pub stacks: Vec<Stack>,
    /// True when the walk stopped at FILE_CAP — coverage is partial.
    pub truncated: bool,
}

impl RepoScan {
    /// Walk `root` and detect stacks.
    pub fn build(root: &Path) -> Result<Self> {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

        let mut files: Vec<PathBuf> = Vec::new();
        let mut truncated = false;
        let mut ts_count = 0usize;
        let mut js_count = 0usize;
        let mut rs_count = 0usize;
        let mut py_count = 0usize;
        let mut go_count = 0usize;

        let walker = ignore::WalkBuilder::new(&root)
            .git_ignore(false)
            .git_global(false)
            .git_exclude(false)
            .hidden(false)
            .filter_entry(|e| {
                if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let name = e.file_name().to_string_lossy();
                    !PRUNED_DIRS.iter().any(|p| *p == name.as_ref())
                } else {
                    true
                }
            })
            .build();

        for entry in walker {
            let entry = entry?;
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            let path = entry
                .path()
                .strip_prefix(&root)
                .unwrap_or(entry.path())
                .to_path_buf();

            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                match ext {
                    "ts" | "tsx" => ts_count += 1,
                    "js" | "jsx" => js_count += 1,
                    "rs" => rs_count += 1,
                    "py" => py_count += 1,
                    "go" => go_count += 1,
                    _ => {}
                }
            }

            files.push(path);
            if files.len() >= FILE_CAP {
                truncated = true;
                break;
            }
        }

        // Manifest-based detection thresholds (complement file counts).
        let has_manifest = |name: &str| root.join(name).exists();

        let mut stacks = Vec::new();
        // TypeScript: ≥3 .ts/.tsx files, or ≥1 with a TS/JS manifest present.
        // A manifest alone (package.json for tooling in a Rust/Python repo) is
        // NOT coverage — claiming TypeScript with zero .ts files would let
        // analyze report "scanned" on a repo it cannot actually inspect.
        if ts_count >= 3
            || (ts_count >= 1 && (has_manifest("tsconfig.json") || has_manifest("package.json")))
        {
            stacks.push(Stack::TypeScript);
        }
        // JavaScript: ≥3 .js/.jsx files
        if js_count >= 3 {
            stacks.push(Stack::JavaScript);
        }
        // Rust: ≥3 .rs files OR Cargo.toml
        if rs_count >= 3 || has_manifest("Cargo.toml") {
            stacks.push(Stack::Rust);
        }
        // Python: ≥3 .py files OR pyproject.toml / requirements.txt
        if py_count >= 3
            || has_manifest("pyproject.toml")
            || has_manifest("requirements.txt")
        {
            stacks.push(Stack::Python);
        }
        // Go: ≥3 .go files OR go.mod
        if go_count >= 3 || has_manifest("go.mod") {
            stacks.push(Stack::Go);
        }

        Ok(Self { root, files, stacks, truncated })
    }

    /// All files whose extension matches any of `exts`.
    pub fn files_with_ext<'a>(&'a self, exts: &[&str]) -> Vec<&'a Path> {
        self.files
            .iter()
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| exts.contains(&e))
                    .unwrap_or(false)
            })
            .map(|p| p.as_path())
            .collect()
    }

    /// Grep all provided files for `pattern`, returning matching lines.
    /// Skips files > 1 MiB; reads with lossy UTF-8.
    pub fn grep_files(&self, files: &[&Path], pattern: &Regex) -> Vec<GrepHit> {
        let mut hits = Vec::new();
        for rel in files {
            let full = self.root.join(rel);
            // Skip large files
            if full
                .metadata()
                .map(|m| m.len() > MAX_FILE_BYTES)
                .unwrap_or(false)
            {
                continue;
            }
            let content = match std::fs::read(&full) {
                Ok(b) => String::from_utf8_lossy(&b).into_owned(),
                Err(_) => continue,
            };
            for (i, line) in content.lines().enumerate() {
                if pattern.is_match(line) {
                    hits.push(GrepHit {
                        path: rel.to_path_buf(),
                        line_no: i + 1,
                        line: line.to_string(),
                    });
                }
            }
        }
        hits
    }

    /// Returns true if `pattern` matches any line in any of the dependency
    /// manifests at root: package.json, go.mod, pyproject.toml, requirements.txt, Gemfile.
    pub fn dep_seen(&self, pattern: &Regex) -> bool {
        let manifests = [
            "package.json",
            "go.mod",
            "pyproject.toml",
            "requirements.txt",
            "Gemfile",
        ];
        for name in manifests {
            let path = self.root.join(name);
            if let Ok(content) = std::fs::read_to_string(&path) {
                if content.lines().any(|l| pattern.is_match(l)) {
                    return true;
                }
            }
        }
        false
    }
}
