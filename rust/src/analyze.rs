use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::Utc;
use regex::Regex;

use crate::models::*;

fn escape_path_for_glob(path: &Path) -> String {
    glob::Pattern::escape(&path.to_string_lossy())
}

pub struct AnalysisConfig {
    pub path: PathBuf,
    pub product: String,
    pub spec_id: String,
}

pub struct AnalysisResult {
    pub spec: Spec,
    pub language: String,
    pub framework: Option<String>,
    pub stats: AnalysisStats,
}

pub struct AnalysisStats {
    pub items_discovered: usize,
    pub domains_found: usize,
    pub entities_found: usize,
    pub capabilities_found: usize,
    pub components_found: usize,
    pub edges_found: usize,
}

// ── Language / framework detection ────────────────────────────────────────────

fn detect_language_and_framework(root: &Path) -> (String, Option<String>) {
    if root.join("Cargo.toml").exists() {
        return ("Rust".into(), None);
    }
    if root.join("manage.py").exists() {
        return ("Python".into(), Some("Django".into()));
    }
    if root.join("pyproject.toml").exists() || root.join("setup.py").exists() {
        return ("Python".into(), None);
    }
    if root.join("go.mod").exists() {
        return ("Go".into(), None);
    }
    if root.join("package.json").exists() {
        let framework = detect_js_framework(root);
        return ("TypeScript".into(), framework);
    }

    // Fallback: count extensions
    let ext_counts = count_extensions(root);
    if let Some(lang) = dominant_language(&ext_counts) {
        return (lang, None);
    }
    ("Unknown".into(), None)
}

fn detect_js_framework(root: &Path) -> Option<String> {
    let pkg_path = root.join("package.json");
    let content = std::fs::read_to_string(&pkg_path).ok()?;
    if content.contains("\"next\"") || content.contains("\"next\":") {
        return Some("Next.js".into());
    }
    if content.contains("\"express\"") || content.contains("\"express\":") {
        return Some("Express".into());
    }
    if content.contains("\"fastify\"") || content.contains("\"fastify\":") {
        return Some("Fastify".into());
    }
    None
}

fn count_extensions(root: &Path) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let pattern = format!("{}/**/*", escape_path_for_glob(root));
    if let Ok(paths) = glob::glob(&pattern) {
        for entry in paths.flatten() {
            if let Some(ext) = entry.extension().and_then(|e| e.to_str()) {
                *counts.entry(ext.to_string()).or_insert(0) += 1;
            }
        }
    }
    counts
}

fn dominant_language(counts: &HashMap<String, usize>) -> Option<String> {
    let rs = counts.get("rs").copied().unwrap_or(0);
    let py = counts.get("py").copied().unwrap_or(0);
    let ts = counts.get("ts").copied().unwrap_or(0) + counts.get("tsx").copied().unwrap_or(0);
    let go = counts.get("go").copied().unwrap_or(0);

    let max = rs.max(py).max(ts).max(go);
    if max == 0 {
        return None;
    }
    if rs == max {
        Some("Rust".into())
    } else if py == max {
        Some("Python".into())
    } else if ts == max {
        Some("TypeScript".into())
    } else {
        Some("Go".into())
    }
}

// ── Monorepo detection ────────────────────────────────────────────────────────

/// Detect monorepo service directories (services/, packages/, apps/)
fn detect_monorepo_services(root: &Path) -> Vec<(String, PathBuf)> {
    let mut result = vec![];
    let monorepo_dirs = ["services", "packages", "apps", "modules"];
    for dir_name in &monorepo_dirs {
        let dir = root.join(dir_name);
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let name = match path.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n.to_string(),
                    None => continue,
                };
                if name.starts_with('.') || name == "node_modules" || name == "__pycache__" {
                    continue;
                }
                result.push((name, path));
            }
        }
    }
    result
}

/// Detect polyglot project — returns all languages present
fn detect_all_languages(root: &Path) -> Vec<(String, Option<String>)> {
    let mut langs = vec![];
    let (primary, framework) = detect_language_and_framework(root);
    langs.push((primary, framework));

    // Check for secondary languages
    if root.join("ui").is_dir() || root.join("frontend").is_dir() || root.join("web").is_dir() {
        let ui_dir = if root.join("ui").is_dir() {
            root.join("ui")
        } else if root.join("frontend").is_dir() {
            root.join("frontend")
        } else {
            root.join("web")
        };
        if ui_dir.join("package.json").exists() || ui_dir.join("tsconfig.json").exists() {
            let fw = detect_js_framework(&ui_dir);
            if !langs.iter().any(|(l, _)| l == "TypeScript") {
                langs.push(("TypeScript".into(), fw));
            }
        }
    }
    if root.join("infrastructure").is_dir() {
        // Terraform / CDK presence
        let infra = root.join("infrastructure");
        let has_tf = glob::glob(&format!("{}/**/*.tf", escape_path_for_glob(&infra)))
            .ok()
            .map(|mut p| p.next().is_some())
            .unwrap_or(false);
        if has_tf && !langs.iter().any(|(l, _)| l == "Terraform") {
            langs.push(("Terraform".into(), None));
        }
    }
    langs
}

// ── Domain inference ───────────────────────────────────────────────────────────

fn infer_domains_rust(root: &Path) -> Vec<String> {
    let cargo_path = root.join("Cargo.toml");
    let content = match std::fs::read_to_string(&cargo_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    // Workspace with members
    let re = Regex::new(r#""([^"]+)""#).unwrap();
    let mut in_members = false;
    let mut domains = vec![];
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("members") {
            in_members = true;
        }
        if in_members {
            for cap in re.captures_iter(trimmed) {
                let member = cap[1].to_string();
                // member may be "crates/foo" — take last component
                let name = member.split('/').next_back().unwrap_or(&member).to_string();
                domains.push(name);
            }
            if trimmed.contains(']') {
                break;
            }
        }
    }
    domains
}

fn infer_domains_django(root: &Path) -> Vec<String> {
    // Django apps: dirs containing models.py
    let mut domains = vec![];
    let pattern = format!("{}/**/models.py", escape_path_for_glob(root));
    if let Ok(paths) = glob::glob(&pattern) {
        for entry in paths.flatten() {
            if let Some(parent) = entry.parent() {
                if let Some(name) = parent.file_name().and_then(|n| n.to_str()) {
                    domains.push(name.to_string());
                }
            }
        }
    }
    domains
}

fn infer_domains_python(root: &Path) -> Vec<String> {
    infer_top_level_code_dirs(root, &["py"])
}

fn infer_domains_ts(root: &Path) -> Vec<String> {
    infer_top_level_code_dirs(root, &["ts", "tsx", "js", "jsx"])
}

fn infer_domains_go(root: &Path) -> Vec<String> {
    infer_top_level_code_dirs(root, &["go"])
}

fn infer_top_level_code_dirs(root: &Path, exts: &[&str]) -> Vec<String> {
    let src_candidates = ["src", "app", "lib", "cmd", "pkg", "."];
    let mut domains = vec![];
    for candidate in &src_candidates {
        let base = if *candidate == "." {
            root.to_path_buf()
        } else {
            root.join(candidate)
        };
        if !base.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let name = match path.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n.to_string(),
                    None => continue,
                };
                if name.starts_with('.')
                    || name == "node_modules"
                    || name == "target"
                    || name == "__pycache__"
                {
                    continue;
                }
                // Check if this dir contains code files
                if dir_has_code_files(&path, exts) {
                    domains.push(name);
                }
            }
        }
        if !domains.is_empty() {
            break;
        }
    }
    domains
}

fn dir_has_code_files(dir: &Path, exts: &[&str]) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if exts.contains(&ext) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

// ── Entity/capability/component/edge extraction (unused — agent does this now) ─
// Kept for reference and potential future use (e.g., hints mode).
#[allow(dead_code)]
fn extract_entities_python(root: &Path, domain: &str) -> Vec<String> {
    let mut entities = HashSet::new();
    // Django Model, Pydantic BaseModel, SQLAlchemy Base, dataclass, TypedDict, NamedTuple
    let re_model =
        Regex::new(r"class\s+(\w+)\s*\([^)]*(?:Model|BaseModel|Base|TypedDict|NamedTuple)[^)]*\)")
            .unwrap();
    let re_dataclass = Regex::new(r"@dataclass\s*\nclass\s+(\w+)").unwrap();

    // Search within domain dir first, then broadly
    let domain_dir = find_domain_dir_python(root, domain);
    let search_root = domain_dir.as_deref().unwrap_or(root);

    let escaped_search_root = escape_path_for_glob(search_root);
    let patterns = [
        format!("{}/**/models.py", escaped_search_root),
        format!("{}/**/*model*.py", escaped_search_root),
        format!("{}/**/*schema*.py", escaped_search_root),
        format!("{}/**/*types*.py", escaped_search_root),
        format!("{}/**/*entities*.py", escaped_search_root),
        format!("{}/**/schemas/*.py", escaped_search_root),
    ];
    for pattern in &patterns {
        if let Ok(paths) = glob::glob(pattern) {
            for entry in paths.flatten() {
                if entry
                    .to_str()
                    .map(|s| {
                        s.contains("__pycache__")
                            || s.contains(".venv")
                            || s.contains("node_modules")
                    })
                    .unwrap_or(false)
                {
                    continue;
                }
                if let Ok(content) = std::fs::read_to_string(&entry) {
                    for cap in re_model.captures_iter(&content) {
                        entities.insert(cap[1].to_string());
                    }
                    for cap in re_dataclass.captures_iter(&content) {
                        entities.insert(cap[1].to_string());
                    }
                }
            }
        }
    }
    entities.into_iter().collect()
}

#[allow(dead_code)]
fn extract_entities_rust(root: &Path, domain: &str) -> Vec<String> {
    let mut entities = HashSet::new();
    let re = Regex::new(r"pub\s+struct\s+(\w+)").unwrap();

    // Prioritize model/type files but scan all .rs in this domain dir
    let domain_dir = find_domain_dir_rust(root, domain);
    let base = domain_dir.unwrap_or_else(|| root.to_path_buf());

    let escaped_base = escape_path_for_glob(&base);
    let patterns = [
        format!("{}/src/model*.rs", escaped_base),
        format!("{}/src/types*.rs", escaped_base),
        format!("{}/src/*.rs", escaped_base),
    ];
    for pattern in &patterns {
        if let Ok(paths) = glob::glob(pattern) {
            for entry in paths.flatten() {
                if entry.to_str().map(|s| s.contains("test")).unwrap_or(false) {
                    continue;
                }
                if let Ok(content) = std::fs::read_to_string(&entry) {
                    for cap in re.captures_iter(&content) {
                        entities.insert(cap[1].to_string());
                    }
                }
            }
        }
    }
    entities.into_iter().collect()
}

#[allow(dead_code)]
fn extract_entities_ts(root: &Path, _domain: &str) -> Vec<String> {
    let mut entities = HashSet::new();
    let re_interface = Regex::new(r"export\s+interface\s+(\w+)").unwrap();
    let re_class = Regex::new(r"export\s+class\s+(\w+)").unwrap();
    let re_type = Regex::new(r"export\s+type\s+(\w+)\s*=").unwrap();

    let escaped_root = escape_path_for_glob(root);
    let patterns = [
        format!("{}/**/*model*.ts", escaped_root),
        format!("{}/**/*entity*.ts", escaped_root),
        format!("{}/**/*types*.ts", escaped_root),
        format!("{}/**/*schema*.ts", escaped_root),
    ];
    for pattern in &patterns {
        if let Ok(paths) = glob::glob(pattern) {
            for entry in paths.flatten() {
                if let Ok(content) = std::fs::read_to_string(&entry) {
                    for cap in re_interface.captures_iter(&content) {
                        entities.insert(cap[1].to_string());
                    }
                    for cap in re_class.captures_iter(&content) {
                        entities.insert(cap[1].to_string());
                    }
                    for cap in re_type.captures_iter(&content) {
                        entities.insert(cap[1].to_string());
                    }
                }
            }
        }
    }
    entities.into_iter().collect()
}

#[allow(dead_code)]
fn extract_entities_go(root: &Path, _domain: &str) -> Vec<String> {
    let mut entities = HashSet::new();
    let re = Regex::new(r"type\s+(\w+)\s+struct").unwrap();
    let pattern = format!("{}/**/*.go", escape_path_for_glob(root));
    if let Ok(paths) = glob::glob(&pattern) {
        for entry in paths.flatten() {
            if let Ok(content) = std::fs::read_to_string(&entry) {
                for cap in re.captures_iter(&content) {
                    entities.insert(cap[1].to_string());
                }
            }
        }
    }
    entities.into_iter().collect()
}

// ── Capability extraction ──────────────────────────────────────────────────────

#[allow(dead_code)]
fn extract_capabilities_python(root: &Path, domain: &str) -> Vec<String> {
    let mut caps = HashSet::new();
    let re_func = Regex::new(r"(?:async\s+)?def\s+([a-z][a-zA-Z0-9_]*)").unwrap();
    // FastAPI/Flask route decorators: @app.get, @router.post, etc.
    let re_route =
        Regex::new(r#"@(?:app|router)\.\w+\([^)]*\)\s*\n\s*(?:async\s+)?def\s+(\w+)"#).unwrap();

    let domain_dir = find_domain_dir_python(root, domain);
    let search_root = domain_dir.as_deref().unwrap_or(root);

    // Named service files (direct)
    let service_files = [
        "views.py",
        "handlers.py",
        "services.py",
        "tasks.py",
        "routes.py",
        "endpoints.py",
        "api.py",
    ];
    for fname in &service_files {
        let path = search_root.join(fname);
        if let Ok(content) = std::fs::read_to_string(&path) {
            for cap in re_func.captures_iter(&content) {
                let name = cap[1].to_string();
                if !name.starts_with('_') && name != "self" {
                    caps.insert(name);
                }
            }
        }
    }

    // Glob for service/handler/router/endpoint files recursively
    let escaped_search_root = escape_path_for_glob(search_root);
    let glob_patterns = [
        format!("{}/**/*service*.py", escaped_search_root),
        format!("{}/**/*handler*.py", escaped_search_root),
        format!("{}/**/*route*.py", escaped_search_root),
        format!("{}/**/*endpoint*.py", escaped_search_root),
        format!("{}/**/*api*.py", escaped_search_root),
        format!("{}/**/*task*.py", escaped_search_root),
    ];
    for pattern in &glob_patterns {
        if let Ok(paths) = glob::glob(pattern) {
            for entry in paths.flatten() {
                let path_str = entry.to_str().unwrap_or("");
                if path_str.contains("__pycache__")
                    || path_str.contains(".venv")
                    || path_str.contains("node_modules")
                    || path_str.contains("/test")
                {
                    continue;
                }
                if let Ok(content) = std::fs::read_to_string(&entry) {
                    // Extract route-decorated functions
                    for cap in re_route.captures_iter(&content) {
                        caps.insert(cap[1].to_string());
                    }
                    // Extract public functions from service files
                    for cap in re_func.captures_iter(&content) {
                        let name = cap[1].to_string();
                        if !name.starts_with('_') && name != "self" {
                            caps.insert(name);
                        }
                    }
                }
            }
        }
    }
    caps.into_iter().collect()
}

#[allow(dead_code)]
fn extract_capabilities_rust(root: &Path, domain: &str) -> Vec<String> {
    let mut caps = HashSet::new();
    let re = Regex::new(r"pub\s+(?:async\s+)?fn\s+(\w+)").unwrap();

    let domain_dir = find_domain_dir_rust(root, domain);
    let base = domain_dir.unwrap_or_else(|| root.to_path_buf());

    let pattern = format!("{}/src/*.rs", escape_path_for_glob(&base));
    if let Ok(paths) = glob::glob(&pattern) {
        for entry in paths.flatten() {
            let path_str = entry.to_str().unwrap_or("");
            if path_str.contains("test") || path_str.ends_with("main.rs") {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(&entry) {
                for cap in re.captures_iter(&content) {
                    caps.insert(cap[1].to_string());
                }
            }
        }
    }
    caps.into_iter().collect()
}

#[allow(dead_code)]
fn extract_capabilities_ts(root: &Path, domain: &str) -> Vec<String> {
    let mut caps = HashSet::new();
    let re = Regex::new(r"export\s+(?:async\s+)?function\s+(\w+)").unwrap();

    let escaped_root = escape_path_for_glob(root);
    let service_patterns = [
        format!("{}/**/*service*.ts", escaped_root),
        format!("{}/**/*handler*.ts", escaped_root),
        format!("{}/**/*controller*.ts", escaped_root),
    ];
    let _ = domain;
    for pattern in &service_patterns {
        if let Ok(paths) = glob::glob(pattern) {
            for entry in paths.flatten() {
                if let Ok(content) = std::fs::read_to_string(&entry) {
                    for cap in re.captures_iter(&content) {
                        caps.insert(cap[1].to_string());
                    }
                }
            }
        }
    }
    caps.into_iter().collect()
}

#[allow(dead_code)]
fn extract_capabilities_go(root: &Path, _domain: &str) -> Vec<String> {
    let mut caps = HashSet::new();
    let re = Regex::new(r"func\s+([A-Z][a-zA-Z0-9_]*)").unwrap();
    let pattern = format!("{}/**/*.go", escape_path_for_glob(root));
    if let Ok(paths) = glob::glob(&pattern) {
        for entry in paths.flatten() {
            if let Ok(content) = std::fs::read_to_string(&entry) {
                for cap in re.captures_iter(&content) {
                    caps.insert(cap[1].to_string());
                }
            }
        }
    }
    caps.into_iter().collect()
}

#[allow(dead_code)]
fn extract_entities_via_parser(domain_root: &Path) -> Vec<String> {
    use crate::codebase::parser::{detect_lang, parse_file};
    let mut names = std::collections::HashSet::new();
    let pattern = format!("{}/**/*", escape_path_for_glob(domain_root));
    if let Ok(entries) = glob::glob(&pattern) {
        for entry in entries.flatten() {
            if !entry.is_file() {
                continue;
            }
            if detect_lang(&entry).is_none() {
                continue;
            }
            if let Ok(result) = parse_file(&entry) {
                for sym in &result.symbols {
                    if matches!(
                        sym.kind.as_str(),
                        "struct" | "class" | "trait" | "enum" | "type"
                    ) {
                        names.insert(sym.name.clone());
                    }
                }
            }
        }
    }
    names.into_iter().collect()
}

#[allow(dead_code)]
fn extract_capabilities_via_parser(domain_root: &Path) -> Vec<String> {
    use crate::codebase::parser::{detect_lang, parse_file};
    let mut names = std::collections::HashSet::new();
    let pattern = format!("{}/**/*", escape_path_for_glob(domain_root));
    if let Ok(entries) = glob::glob(&pattern) {
        for entry in entries.flatten() {
            if !entry.is_file() {
                continue;
            }
            if detect_lang(&entry).is_none() {
                continue;
            }
            if let Ok(result) = parse_file(&entry) {
                for sym in &result.symbols {
                    if sym.kind == "fn" {
                        let mut c = sym.name.chars();
                        let capitalized = match c.next() {
                            None => sym.name.clone(),
                            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                        };
                        names.insert(capitalized);
                    }
                }
            }
        }
    }
    names.into_iter().take(20).collect()
}

// ── Component inference ────────────────────────────────────────────────────────

#[allow(dead_code)]
fn infer_components(root: &Path, domain: &str) -> Vec<String> {
    let mut components = vec![];
    let keywords = ["service", "handler", "controller", "worker", "middleware"];
    let escaped_root = escape_path_for_glob(root);
    let patterns = [
        format!("{}/**/*service*.rs", escaped_root),
        format!("{}/**/*handler*.rs", escaped_root),
        format!("{}/**/*controller*.rs", escaped_root),
        format!("{}/**/*worker*.rs", escaped_root),
        format!("{}/**/*middleware*.rs", escaped_root),
        format!("{}/**/*service*.py", escaped_root),
        format!("{}/**/*handler*.py", escaped_root),
        format!("{}/**/*controller*.py", escaped_root),
        format!("{}/**/*worker*.py", escaped_root),
        format!("{}/**/*service*.ts", escaped_root),
        format!("{}/**/*handler*.ts", escaped_root),
        format!("{}/**/*controller*.ts", escaped_root),
        format!("{}/**/*worker*.ts", escaped_root),
        format!("{}/**/*service*.go", escaped_root),
        format!("{}/**/*handler*.go", escaped_root),
    ];
    let mut seen = HashSet::new();
    for pattern in &patterns {
        if let Ok(paths) = glob::glob(pattern) {
            for entry in paths.flatten() {
                if let Some(stem) = entry.file_stem().and_then(|s| s.to_str()) {
                    let key = format!("{}.{}", domain, stem);
                    if seen.insert(key) {
                        components.push(stem.to_string());
                    }
                }
            }
        }
    }
    let _ = keywords;
    components
}

// ── Edge inference ─────────────────────────────────────────────────────────────

#[allow(dead_code)]
fn infer_edges_rust(root: &Path, domains: &[String]) -> Vec<(String, String)> {
    let mut edges = HashSet::new();

    for domain in domains {
        let domain_dir = find_domain_dir_rust(root, domain);
        let base = match domain_dir {
            Some(d) => d,
            None => continue,
        };
        let pattern = format!("{}/src/**/*.rs", escape_path_for_glob(&base));
        if let Ok(paths) = glob::glob(&pattern) {
            for entry in paths.flatten() {
                if let Ok(content) = std::fs::read_to_string(&entry) {
                    for line in content.lines() {
                        if let Some(pos) = line.find("use ") {
                            let rest = &line[pos + 4..];
                            for other in domains {
                                if other == domain {
                                    continue;
                                }
                                let crate_prefix = format!("{}::", other.replace('-', "_"));
                                if rest.starts_with(&crate_prefix) {
                                    edges.insert((domain.clone(), other.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    edges.into_iter().collect()
}

#[allow(dead_code)]
fn infer_edges_python(root: &Path, domains: &[String]) -> Vec<(String, String)> {
    let mut edges = HashSet::new();
    for domain in domains {
        let domain_dir = find_domain_dir_python(root, domain);
        let base = match domain_dir {
            Some(d) => d,
            None => continue,
        };
        let pattern = format!("{}/**/*.py", escape_path_for_glob(&base));
        if let Ok(paths) = glob::glob(&pattern) {
            for entry in paths.flatten() {
                if let Ok(content) = std::fs::read_to_string(&entry) {
                    for line in content.lines() {
                        for other in domains {
                            if other == domain {
                                continue;
                            }
                            let import_from = format!("from {} import", other);
                            if line.trim().starts_with(&import_from) {
                                edges.insert((domain.clone(), other.clone()));
                            }
                        }
                    }
                }
            }
        }
    }
    edges.into_iter().collect()
}

#[allow(dead_code)]
fn infer_edges_ts(root: &Path, domains: &[String]) -> Vec<(String, String)> {
    let mut edges = HashSet::new();
    let re = Regex::new(r#"from\s+['"][^'"]*['"]"#).unwrap();
    for domain in domains {
        let escaped_root = escape_path_for_glob(root);
        let patterns = [
            format!("{}/**/*.ts", escaped_root),
            format!("{}/**/*.tsx", escaped_root),
        ];
        for pattern in &patterns {
            if let Ok(paths) = glob::glob(pattern) {
                for entry in paths.flatten() {
                    let path_str = entry.to_str().unwrap_or("");
                    if !path_str.contains(&format!("/{}/", domain)) {
                        continue;
                    }
                    if let Ok(content) = std::fs::read_to_string(&entry) {
                        for cap in re.find_iter(&content) {
                            let import_str = cap.as_str();
                            for other in domains {
                                if other == domain {
                                    continue;
                                }
                                if import_str.contains(&format!("/{}/", other)) {
                                    edges.insert((domain.clone(), other.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    edges.into_iter().collect()
}

// ── Domain classification ─────────────────────────────────────────────────────

fn classify_domain(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if [
        "auth",
        "user",
        "identity",
        "account",
        "profile",
        "permission",
        "role",
    ]
    .iter()
    .any(|kw| lower.contains(kw))
    {
        return "Generic";
    }
    if [
        "notification",
        "email",
        "log",
        "monitor",
        "metric",
        "alert",
        "audit",
        "report",
    ]
    .iter()
    .any(|kw| lower.contains(kw))
    {
        return "Supporting";
    }
    "Core"
}

// ── Helper: find domain directory ─────────────────────────────────────────────

fn find_domain_dir_rust(root: &Path, domain: &str) -> Option<PathBuf> {
    // Try crates/<domain> pattern
    let candidates = [root.join("crates").join(domain), root.join(domain)];
    for c in &candidates {
        if c.is_dir() {
            return Some(c.clone());
        }
    }
    None
}

fn find_domain_dir_python(root: &Path, domain: &str) -> Option<PathBuf> {
    let candidates = [
        root.join("services").join(domain),
        root.join("services").join(domain).join("src"),
        root.join("packages").join(domain),
        root.join("apps").join(domain),
        root.join("src").join(domain),
        root.join("app").join(domain),
        root.join(domain),
        root.join(domain).join("src"),
    ];
    for c in &candidates {
        if c.is_dir() {
            return Some(c.clone());
        }
    }
    None
}

// ── Codebase-index-based extractors ───────────────────────────────────────────

#[allow(dead_code)]
fn extract_entities_from_index(
    index: &crate::codebase::storage::CodebaseIndex,
    domain_name: &str,
) -> Vec<String> {
    use rusqlite::params;
    let lower = domain_name.to_lowercase();
    let pattern = format!("%{}%", lower);
    let mut stmt = match index.conn.prepare(
        "SELECT DISTINCT name FROM symbols WHERE kind IN ('struct','class','type','trait') \
         AND (lower(name) LIKE ?1 OR lower(file_path) LIKE ?2)",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let rows = stmt.query_map(params![pattern, pattern], |row| row.get::<_, String>(0));
    match rows {
        Ok(r) => r.flatten().collect(),
        Err(_) => vec![],
    }
}

#[allow(dead_code)]
fn extract_capabilities_from_index(
    index: &crate::codebase::storage::CodebaseIndex,
    domain_name: &str,
) -> Vec<String> {
    use rusqlite::params;
    let lower = domain_name.to_lowercase();
    let pattern = format!("%{}%", lower);
    let mut stmt = match index.conn.prepare(
        "SELECT DISTINCT name FROM symbols WHERE kind = 'fn' \
         AND lower(file_path) LIKE ?1 LIMIT 15",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let rows = stmt.query_map(params![pattern], |row| row.get::<_, String>(0));
    let names: Vec<String> = match rows {
        Ok(r) => r.flatten().collect(),
        Err(_) => return vec![],
    };
    names
        .into_iter()
        .map(|n| {
            let mut c = n.chars();
            match c.next() {
                None => n,
                Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}

// ── Main analyze function ──────────────────────────────────────────────────────

pub fn analyze(config: &AnalysisConfig) -> anyhow::Result<AnalysisResult> {
    let root = &config.path;
    let product = &config.product;
    let spec_id = &config.spec_id;

    let (language, framework) = detect_language_and_framework(root);

    let all_languages = detect_all_languages(root);

    // Phase 1: Detect monorepo services (highest priority)
    let monorepo_services = detect_monorepo_services(root);

    // Phase 2: Infer domains — monorepo services + language-specific + special dirs
    let mut domain_entries: Vec<(String, Option<PathBuf>, Option<String>)> = vec![];
    let mut seen_names: HashSet<String> = HashSet::new();

    // Monorepo services first (services/, packages/, apps/)
    for (name, path) in &monorepo_services {
        if seen_names.insert(name.clone()) {
            domain_entries.push((name.clone(), Some(path.clone()), None));
        }
    }

    // Language-specific domains (workspace members, Django apps, etc)
    let lang_domains: Vec<String> = match (language.as_str(), framework.as_deref()) {
        ("Rust", _) => infer_domains_rust(root),
        ("Python", Some("Django")) => infer_domains_django(root),
        ("Python", _) => infer_domains_python(root),
        ("TypeScript", _) => infer_domains_ts(root),
        ("Go", _) => infer_domains_go(root),
        _ => vec![],
    };
    for name in lang_domains {
        if seen_names.insert(name.clone()) {
            domain_entries.push((name.clone(), None, None));
        }
    }

    // Phase 3: Domain grouping is done by the agent (skill), not here.
    // This function outputs raw modules. The agent reads the output and groups them into domains.

    // Special directories as domains
    let special_dirs = [
        ("ui", "Supporting"),
        ("frontend", "Supporting"),
        ("web", "Supporting"),
        ("shared", "Generic"),
        ("common", "Generic"),
        ("core", "Generic"),
        ("infrastructure", "Supporting"),
    ];
    for (dir_name, _) in &special_dirs {
        let dir_path = root.join(dir_name);
        if dir_path.is_dir() && seen_names.insert(dir_name.to_string()) {
            domain_entries.push((dir_name.to_string(), Some(dir_path), None));
        }
    }

    let product_id = slugify(product);
    let feature_id = format!("feat.{}.{}", product_id, spec_id);

    let product_artifact = Product {
        id: product_id.clone(),
        name: product.clone(),
        organization: None,
    };

    let feature_artifact = Feature {
        id: feature_id.clone(),
        product: product_id.clone(),
        name: "Domain Analysis".into(),
        description: Some(format!("Auto-generated domain map for {}", product)),
    };

    // Build Domain artifacts with smart classification
    let mut domains: Vec<Domain> = domain_entries
        .iter()
        .map(|(name, _, llm_classification)| {
            // Priority: LLM classification > special dirs > heuristic
            let classification = llm_classification.clone().unwrap_or_else(|| {
                special_dirs
                    .iter()
                    .find(|(d, _)| *d == name.as_str())
                    .map(|(_, c)| c.to_string())
                    .unwrap_or_else(|| classify_domain(name).to_string())
            });
            Domain {
                id: format!("dom.{}.{}", product_id, slugify(name)),
                product: product_id.clone(),
                name: name.clone(),
                classification: Some(classification),
                description: None,
                team: None,
                maturity_level: Some("custom".into()),
            }
        })
        .collect();

    // Infer domain owners from git history (top committer per domain directory)
    let domain_paths: Vec<(String, PathBuf)> = domains
        .iter()
        .enumerate()
        .filter_map(|(i, d)| {
            if i >= domain_entries.len() {
                return None;
            }
            let (name, path, _) = &domain_entries[i];
            let resolved = path
                .clone()
                .or_else(|| find_domain_dir_rust(root, name))
                .or_else(|| find_domain_dir_python(root, name))
                // Rust workspace may live under rust/ subdirectory
                .or_else(|| {
                    let rust_root = root.join("rust");
                    if rust_root.is_dir() {
                        find_domain_dir_rust(&rust_root, name)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| root.join(name));
            if resolved.is_dir() {
                Some((d.id.clone(), resolved))
            } else {
                None
            }
        })
        .collect();
    let owners = crate::git_state::infer_domain_owners(root, &domain_paths);
    for domain in &mut domains {
        if let Some(owner) = owners.get(&domain.id) {
            domain.team = Some(owner.clone());
        }
    }

    // Scaffold only — entities, capabilities, components, edges are filled by the AI skill.
    let entities: Vec<Entity> = vec![];
    let capabilities: Vec<Capability> = vec![];
    let components: Vec<Component> = vec![];
    let edges: Vec<Edge> = vec![];
    let items_discovered = 0usize;

    // Infer containers from infrastructure stacks
    let infra_stacks = root.join("infrastructure").join("stacks");
    let mut containers: Vec<Container> = vec![];
    if infra_stacks.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&infra_stacks) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        containers.push(Container {
                            id: format!("container.{}", slugify(name)),
                            system: format!("sys.{}", product_id),
                            name: name.to_string(),
                            technology: Some("AWS".into()),
                        });
                    }
                }
            }
        }
    }

    // feature_to_domain links
    let feature_to_domain: Vec<Binding> = domains
        .iter()
        .map(|d| Binding {
            fields: HashMap::from([
                ("feature".into(), feature_id.clone()),
                ("domain".into(), d.id.clone()),
            ]),
        })
        .collect();

    // component_to_capability links (first cap of each domain, if any)
    let component_to_capability: Vec<Binding> = components
        .iter()
        .filter_map(|comp| {
            let cap = capabilities.iter().find(|c| c.domain == comp.domain)?;
            Some(Binding {
                fields: HashMap::from([
                    ("component".into(), comp.id.clone()),
                    ("capability".into(), cap.id.clone()),
                ]),
            })
        })
        .collect();

    let today = Utc::now().format("%Y-%m-%d").to_string();

    // Language summary for context
    let lang_summary = if all_languages.len() > 1 {
        let parts: Vec<String> = all_languages
            .iter()
            .map(|(l, f)| match f {
                Some(fw) => format!("{} ({})", l, fw),
                None => l.clone(),
            })
            .collect();
        parts.join(", ")
    } else {
        format!(
            "{}{}",
            language,
            framework
                .as_ref()
                .map(|f| format!(" ({})", f))
                .unwrap_or_default()
        )
    };

    let stats = AnalysisStats {
        items_discovered,
        domains_found: domains.len(),
        entities_found: entities.len(),
        capabilities_found: capabilities.len(),
        components_found: components.len(),
        edges_found: edges.len(),
    };

    let systems = if !containers.is_empty() {
        vec![SoftwareSystem {
            id: format!("sys.{}", product_id),
            product: product_id.clone(),
            name: product.clone(),
        }]
    } else {
        vec![]
    };

    let spec = Spec {
        s5d: "1.0".into(),
        id: format!("feat.{}.{}", product_id, spec_id),
        version: "1.0.0".into(),
        product: product_id.clone(),
        tier: Tier::Standard,
        allow_update: true,
        meta: Some(Meta {
            title: format!("{} Domain Map (auto-generated)", product),
            authors: vec![],
            date: Some(today),
            tickets: vec![],
            adrs: vec![],
            renames: vec![],
        }),
        context: Some(format!(
            "Scaffold — fill artifacts via /s5d skill.\nProduct: {}\nLanguages: {}",
            product, lang_summary
        )),
        artifacts: Some(Artifacts {
            products: vec![product_artifact],
            features: vec![feature_artifact],
            domains,
            entities,
            capabilities,
            components,
            systems,
            containers,
            ..Default::default()
        }),
        links: Some(Links {
            feature_to_domain,
            component_to_capability,
            edges,
            ..Default::default()
        }),
        contracts: vec![],
        gates: vec![
            Gate { kind: "schema".into() },
            Gate { kind: "graph".into() },
        ],
        roc: None,
        problem: None,
        hypotheses: vec![],
        decision: None,
        note_rationale: None,
        expires_at: None,
        auto_noted: false,
    };

    Ok(AnalysisResult {
        spec,
        language: lang_summary,
        framework,
        stats,
    })
}

// Domain grouping is done by the agent (skill), not by Rust code.
// analyze outputs raw modules. The agent reads output and groups into architectural domains.

// ── Utility ───────────────────────────────────────────────────────────────────

fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_language_rust() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"foo\"\n").unwrap();
        let (lang, fw) = detect_language_and_framework(dir.path());
        assert_eq!(lang, "Rust");
        assert_eq!(fw, None);
    }

    #[test]
    fn test_detect_language_python() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("manage.py"), "# django").unwrap();
        let (lang, fw) = detect_language_and_framework(dir.path());
        assert_eq!(lang, "Python");
        assert_eq!(fw.as_deref(), Some("Django"));
    }

    #[test]
    fn test_extract_entities_python() {
        let dir = TempDir::new().unwrap();
        let content = "class Order(models.Model):\n    pass\nclass Customer(Base):\n    pass\n";
        fs::write(dir.path().join("models.py"), content).unwrap();
        let entities = extract_entities_python(dir.path(), "shop");
        assert!(
            entities.contains(&"Order".to_string()),
            "expected Order, got {:?}",
            entities
        );
        assert!(
            entities.contains(&"Customer".to_string()),
            "expected Customer, got {:?}",
            entities
        );
    }

    #[test]
    fn test_extract_entities_rust() {
        let dir = TempDir::new().unwrap();
        let src_dir = dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(
            src_dir.join("models.rs"),
            "pub struct Invoice {}\npub struct LineItem {}\n",
        )
        .unwrap();
        let entities = extract_entities_rust(dir.path(), "billing");
        assert!(
            entities.contains(&"Invoice".to_string()),
            "expected Invoice, got {:?}",
            entities
        );
        assert!(
            entities.contains(&"LineItem".to_string()),
            "expected LineItem, got {:?}",
            entities
        );
    }

    #[test]
    fn test_monorepo_services() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"myapp\"\n",
        )
        .unwrap();

        // services/agents with Pydantic models
        let agents_src = dir.path().join("services").join("agents").join("src");
        fs::create_dir_all(&agents_src).unwrap();
        fs::write(
            agents_src.join("models.py"),
            "from pydantic import BaseModel\n\nclass AgentConfig(BaseModel):\n    pass\nclass AgentResult(BaseModel):\n    pass\n",
        ).unwrap();

        // services/api
        let api_src = dir.path().join("services").join("api").join("src");
        fs::create_dir_all(&api_src).unwrap();
        fs::write(
            api_src.join("routes.py"),
            "from fastapi import APIRouter\nrouter = APIRouter()\n\n@router.get('/claims')\nasync def list_claims():\n    pass\n",
        ).unwrap();

        // ui/ with package.json
        let ui_dir = dir.path().join("ui");
        fs::create_dir_all(&ui_dir).unwrap();
        fs::write(
            ui_dir.join("package.json"),
            r#"{"name": "ui", "dependencies": {}}"#,
        )
        .unwrap();

        // shared/
        let shared = dir.path().join("shared");
        fs::create_dir_all(&shared).unwrap();
        fs::write(shared.join("core.py"), "# shared utilities").unwrap();

        let services = detect_monorepo_services(dir.path());
        assert!(
            services.len() >= 2,
            "expected >=2 services, got {:?}",
            services
        );

        let config = AnalysisConfig {
            path: dir.path().to_path_buf(),
            product: "MyApp".into(),
            spec_id: "analysis".into(),
        };
        let result = analyze(&config).unwrap();

        // Should find at minimum: agents, api, ui, shared
        assert!(
            result.stats.domains_found >= 4,
            "expected >=4 domains, got {} ({:?})",
            result.stats.domains_found,
            result
                .spec
                .artifacts
                .as_ref()
                .unwrap()
                .domains
                .iter()
                .map(|d| &d.name)
                .collect::<Vec<_>>()
        );

        // Scaffold: entities are filled by the AI skill, not extracted here
        assert_eq!(result.stats.entities_found, 0);

        // Check that ui is classified as Supporting and shared as Generic
        let artifacts = result.spec.artifacts.as_ref().unwrap();
        let ui_domain = artifacts.domains.iter().find(|d| d.name == "ui");
        assert!(ui_domain.is_some(), "ui domain not found");
        assert_eq!(
            ui_domain.unwrap().classification.as_deref(),
            Some("Supporting")
        );

        let shared_domain = artifacts.domains.iter().find(|d| d.name == "shared");
        assert!(shared_domain.is_some(), "shared domain not found");
        assert_eq!(
            shared_domain.unwrap().classification.as_deref(),
            Some("Generic")
        );
    }

    #[test]
    fn test_extract_pydantic_entities() {
        let dir = TempDir::new().unwrap();
        let content = r#"
from pydantic import BaseModel
from typing import TypedDict

class ClaimRequest(BaseModel):
    pass

class FieldResult(TypedDict):
    field: str

class LegacyModel(Base):
    pass
"#;
        fs::write(dir.path().join("schema.py"), content).unwrap();
        let entities = extract_entities_python(dir.path(), "claims");
        assert!(
            entities.contains(&"ClaimRequest".to_string()),
            "expected ClaimRequest, got {:?}",
            entities
        );
        assert!(
            entities.contains(&"FieldResult".to_string()),
            "expected FieldResult, got {:?}",
            entities
        );
        assert!(
            entities.contains(&"LegacyModel".to_string()),
            "expected LegacyModel, got {:?}",
            entities
        );
    }

    #[test]
    fn test_analyze_full_rust_project() {
        let dir = TempDir::new().unwrap();

        // Workspace Cargo.toml
        fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/orders\", \"crates/payments\"]\n",
        )
        .unwrap();

        // orders crate
        let orders_src = dir.path().join("crates").join("orders").join("src");
        fs::create_dir_all(&orders_src).unwrap();
        fs::write(
            orders_src.join("models.rs"),
            "pub struct Order {}\npub struct OrderLine {}\n",
        )
        .unwrap();
        fs::write(
            orders_src.join("service.rs"),
            "pub fn create_order() {}\npub fn cancel_order() {}\n",
        )
        .unwrap();

        // payments crate
        let pay_src = dir.path().join("crates").join("payments").join("src");
        fs::create_dir_all(&pay_src).unwrap();
        fs::write(pay_src.join("models.rs"), "pub struct Payment {}\n").unwrap();

        let config = AnalysisConfig {
            path: dir.path().to_path_buf(),
            product: "Shop".into(),
            spec_id: "analysis".into(),
        };

        let result = analyze(&config).unwrap();

        assert_eq!(result.language, "Rust");
        assert!(
            result.stats.domains_found >= 2,
            "expected >=2 domains, got {}",
            result.stats.domains_found
        );
        // Scaffold: entities/capabilities are filled by the AI skill
        assert_eq!(result.stats.entities_found, 0);
        assert_eq!(result.stats.capabilities_found, 0);

        let artifacts = result.spec.artifacts.unwrap();
        assert!(!artifacts.domains.is_empty());
        assert!(artifacts.entities.is_empty());
    }
}
