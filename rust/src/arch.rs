use crate::models::*;
use regex::Regex;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Component as PathComponent, Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct ArchitectureCheckReport {
    pub spec_id: String,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub components: Vec<ComponentCoverage>,
    pub dependencies: Vec<SourceDependency>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComponentCoverage {
    pub component: String,
    pub domain: String,
    pub paths: Vec<String>,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceDependency {
    pub from_component: String,
    pub from_domain: String,
    pub from_file: String,
    pub to_component: String,
    pub to_domain: String,
    pub to_file: String,
    pub reference: String,
}

pub fn architecture_check(
    spec: &Spec,
    project_root: &Path,
) -> anyhow::Result<ArchitectureCheckReport> {
    let mut report = ArchitectureCheckReport {
        spec_id: spec.id.clone(),
        errors: Vec::new(),
        warnings: Vec::new(),
        components: Vec::new(),
        dependencies: Vec::new(),
    };

    for error in crate::validate_spec(spec) {
        report.errors.push(format!("schema: {}", error));
    }
    for error in crate::graph_check(spec) {
        report.errors.push(format!("graph: {}", error));
    }

    let Some(artifacts) = spec.artifacts.as_ref() else {
        return Ok(report);
    };

    let mut file_owners: HashMap<PathBuf, &Component> = HashMap::new();
    let mut owned_files: Vec<PathBuf> = Vec::new();

    for component in &artifacts.components {
        let mut matched_files = Vec::new();
        for pattern in &component.paths {
            if component_path_escapes_project(pattern) {
                report.errors.push(format!(
                    "source: component '{}' path '{}' escapes the project root. Fix: use a relative source path under the repository.",
                    component.id, pattern
                ));
                continue;
            }
            let files = resolve_component_path(project_root, pattern)?;
            if files.is_empty() {
                report.errors.push(format!(
                    "source: component '{}' path '{}' matched no source files. Fix: update artifacts.components[].paths or create the file.",
                    component.id, pattern
                ));
            }
            matched_files.extend(files);
        }

        matched_files.sort();
        matched_files.dedup();

        for file in &matched_files {
            if let Some(previous) = file_owners.insert(file.clone(), component) {
                report.errors.push(format!(
                    "source: file '{}' is owned by both '{}' and '{}'. Fix: make component paths non-overlapping.",
                    display_rel(project_root, file),
                    previous.id,
                    component.id
                ));
            }
        }

        owned_files.extend(matched_files.iter().cloned());
        report.components.push(ComponentCoverage {
            component: component.id.clone(),
            domain: component.domain.clone(),
            paths: component.paths.clone(),
            files: matched_files
                .iter()
                .map(|file| display_rel(project_root, file))
                .collect(),
        });
    }

    owned_files.sort();
    owned_files.dedup();

    let module_index = build_rust_module_index(project_root, &owned_files);
    let allowed_edges = allowed_domain_edges(spec);
    let crate_ref = Regex::new(r"\bcrate::([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)")?;

    for from_file in &owned_files {
        let Some(from_component) = file_owners.get(from_file).copied() else {
            continue;
        };
        if from_file.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(from_file) else {
            report.warnings.push(format!(
                "source: could not read '{}'",
                display_rel(project_root, from_file)
            ));
            continue;
        };

        let mut seen_refs = HashSet::new();
        for captures in crate_ref.captures_iter(&content) {
            let Some(reference_match) = captures.get(1) else {
                continue;
            };
            let reference = reference_match.as_str();
            if !seen_refs.insert(reference.to_string()) {
                continue;
            }
            let Some(to_file) = resolve_rust_reference(reference, &module_index) else {
                continue;
            };
            if to_file == *from_file {
                continue;
            }
            let Some(to_component) = file_owners.get(&to_file).copied() else {
                continue;
            };
            if from_component.id == to_component.id {
                continue;
            }

            let dependency = SourceDependency {
                from_component: from_component.id.clone(),
                from_domain: from_component.domain.clone(),
                from_file: display_rel(project_root, from_file),
                to_component: to_component.id.clone(),
                to_domain: to_component.domain.clone(),
                to_file: display_rel(project_root, &to_file),
                reference: format!("crate::{}", reference),
            };

            if dependency.from_domain != dependency.to_domain
                && !is_edge_allowed(
                    &allowed_edges,
                    &dependency.from_domain,
                    &dependency.to_domain,
                )
            {
                report.errors.push(format!(
                    "architecture: component '{}' ({}) imports '{}' ({}) via {} -> {}. Fix: add links.edges {} -> {} if this dependency is intentional, or route it through an allowed boundary.",
                    dependency.from_component,
                    dependency.from_domain,
                    dependency.to_component,
                    dependency.to_domain,
                    dependency.from_file,
                    dependency.reference,
                    dependency.from_domain,
                    dependency.to_domain
                ));
            }

            report.dependencies.push(dependency);
        }
    }

    Ok(report)
}

fn resolve_component_path(project_root: &Path, pattern: &str) -> anyhow::Result<Vec<PathBuf>> {
    if component_path_escapes_project(pattern) {
        anyhow::bail!(
            "component path '{}' escapes the project root; use a relative source path",
            pattern
        );
    }

    let mut files = Vec::new();
    let full_pattern = project_root.join(pattern);

    if contains_glob(pattern) {
        for entry in glob::glob(&full_pattern.to_string_lossy())? {
            let path = entry?;
            collect_source_files(&path, &mut files)?;
        }
    } else {
        collect_source_files(&full_pattern, &mut files)?;
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn component_path_escapes_project(pattern: &str) -> bool {
    pattern.is_empty()
        || pattern.contains('\0')
        || Path::new(pattern).is_absolute()
        || Path::new(pattern).components().any(|component| {
            matches!(
                component,
                PathComponent::ParentDir | PathComponent::RootDir | PathComponent::Prefix(_)
            )
        })
}

fn collect_source_files(path: &Path, files: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    if path.is_file() {
        if is_source_file(path) {
            files.push(path.canonicalize()?);
        }
        return Ok(());
    }
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let child = entry.path();
            if child
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with('.'))
            {
                continue;
            }
            collect_source_files(&child, files)?;
        }
    }
    Ok(())
}

fn is_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some(
            "rs" | "py"
                | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "go"
                | "md"
                | "sh"
                | "json"
                | "yaml"
                | "yml"
                | "toml"
                | "tpl"
                | "java"
                | "kt"
                | "swift"
                | "c"
                | "cc"
                | "cpp"
                | "h"
                | "hpp"
        )
    )
}

fn contains_glob(pattern: &str) -> bool {
    pattern.contains('*') || pattern.contains('?') || pattern.contains('[')
}

fn build_rust_module_index(project_root: &Path, files: &[PathBuf]) -> HashMap<String, PathBuf> {
    let mut index = HashMap::new();
    for file in files {
        if file.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let Ok(rel) = file.strip_prefix(project_root) else {
            continue;
        };
        for key in rust_module_keys(rel) {
            index.entry(key).or_insert_with(|| file.clone());
        }
    }
    index
}

fn rust_module_keys(rel: &Path) -> Vec<String> {
    let parts: Vec<String> = rel
        .components()
        .filter_map(|part| match part {
            std::path::Component::Normal(value) => value.to_str().map(|s| s.to_string()),
            _ => None,
        })
        .collect();
    let Some(src_idx) = parts.iter().rposition(|part| part == "src") else {
        return Vec::new();
    };
    let mut tail = parts[src_idx + 1..].to_vec();
    if tail.is_empty() {
        return Vec::new();
    }

    let Some(last) = tail.last_mut() else {
        return Vec::new();
    };
    if let Some(stem) = last.strip_suffix(".rs") {
        *last = stem.to_string();
    } else {
        return Vec::new();
    }

    match tail.last().map(String::as_str) {
        Some("lib" | "main") => return Vec::new(),
        Some("mod") => {
            tail.pop();
        }
        _ => {}
    }
    if tail.is_empty() {
        return Vec::new();
    }

    let mut keys = vec![tail.join("::")];
    if src_idx > 0 {
        let crate_dir = parts[src_idx - 1].replace('-', "_");
        keys.push(format!("{}::{}", crate_dir, tail.join("::")));
    }
    keys
}

fn resolve_rust_reference(
    reference: &str,
    module_index: &HashMap<String, PathBuf>,
) -> Option<PathBuf> {
    let parts: Vec<&str> = reference.split("::").collect();
    for len in (1..=parts.len()).rev() {
        let key = parts[..len].join("::");
        if let Some(path) = module_index.get(&key) {
            return Some(path.clone());
        }
    }
    None
}

fn allowed_domain_edges(spec: &Spec) -> HashSet<(String, String)> {
    let mut edges = HashSet::new();
    let bidirectional = ["shared_kernel", "partnership", "separate_ways"];
    let Some(links) = spec.links.as_ref() else {
        return edges;
    };

    for edge in &links.edges {
        edges.insert((edge.from.clone(), edge.to.clone()));
        if bidirectional.contains(&edge.archetype.as_str()) {
            edges.insert((edge.to.clone(), edge.from.clone()));
        }
    }

    edges
}

fn is_edge_allowed(edges: &HashSet<(String, String)>, from_domain: &str, to_domain: &str) -> bool {
    edges.contains(&(from_domain.to_string(), to_domain.to_string()))
}

fn display_rel(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}
