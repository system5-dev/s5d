use crate::project::atomic_write_string;
use crate::{Component, S5dProject, Spec};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodebaseSnapshot {
    pub modules: Vec<CodebaseModule>,
    pub coverage: CodebaseCoverage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodebaseModule {
    pub name: String,
    pub path: String,
    pub language: String,
    pub file_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodebaseCoverage {
    pub product: String,
    pub total_modules: usize,
    pub governed: usize,
    pub partial: usize,
    pub blind: usize,
    pub modules: Vec<ModuleCoverage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleCoverage {
    pub module: CodebaseModule,
    pub status: CoverageStatus,
    pub specs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoverageStatus {
    Governed,
    Partial,
    Blind,
}

pub fn build_codebase_snapshot(project: &S5dProject) -> anyhow::Result<CodebaseSnapshot> {
    let modules = discover_modules(&project.root)?;
    let specs = project.discover_specs()?;
    let product = product_name(&project.root, specs.iter().map(|(_, spec)| spec));
    let module_sources = module_source_files(&project.root, &modules)?;

    let mut covered_files: BTreeMap<String, BTreeSet<PathBuf>> = BTreeMap::new();
    let mut specs_by_module: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for (_, spec) in &specs {
        let Some(artifacts) = spec.artifacts.as_ref() else {
            continue;
        };
        for component in &artifacts.components {
            for file in component_source_files(&project.root, component)? {
                for module in &modules {
                    let Some(source_files) = module_sources.get(&module.path) else {
                        continue;
                    };
                    if source_files.contains(&file) {
                        covered_files
                            .entry(module.path.clone())
                            .or_default()
                            .insert(file.clone());
                        specs_by_module
                            .entry(module.path.clone())
                            .or_default()
                            .insert(spec.id.clone());
                    }
                }
            }
        }
    }

    let mut coverage_modules = Vec::new();
    let mut governed = 0;
    let mut partial = 0;
    let mut blind = 0;

    for module in &modules {
        let source_count = module_sources
            .get(&module.path)
            .map(BTreeSet::len)
            .unwrap_or_default();
        let covered_count = covered_files
            .get(&module.path)
            .map(BTreeSet::len)
            .unwrap_or_default();
        let status = if covered_count == 0 {
            blind += 1;
            CoverageStatus::Blind
        } else if covered_count >= source_count {
            governed += 1;
            CoverageStatus::Governed
        } else {
            partial += 1;
            CoverageStatus::Partial
        };

        coverage_modules.push(ModuleCoverage {
            module: module.clone(),
            status,
            specs: specs_by_module
                .get(&module.path)
                .map(|ids| ids.iter().cloned().collect())
                .unwrap_or_default(),
        });
    }

    Ok(CodebaseSnapshot {
        modules: modules.clone(),
        coverage: CodebaseCoverage {
            product,
            total_modules: modules.len(),
            governed,
            partial,
            blind,
            modules: coverage_modules,
        },
    })
}

pub fn write_codebase_snapshot(
    project: &S5dProject,
    snapshot: &CodebaseSnapshot,
) -> anyhow::Result<()> {
    let dir = project.s5d_dir().join("codebase");
    std::fs::create_dir_all(&dir)?;
    atomic_write_string(
        &dir.join("modules.yaml"),
        &serde_yaml::to_string(&snapshot.modules)?,
    )?;
    atomic_write_string(
        &dir.join("coverage.yaml"),
        &serde_yaml::to_string(&snapshot.coverage)?,
    )?;
    Ok(())
}

pub fn load_codebase_snapshot(project: &S5dProject) -> anyhow::Result<Option<CodebaseSnapshot>> {
    let dir = project.s5d_dir().join("codebase");
    let modules_path = dir.join("modules.yaml");
    let coverage_path = dir.join("coverage.yaml");
    if !modules_path.exists() || !coverage_path.exists() {
        return Ok(None);
    }

    let modules = serde_yaml::from_str(&std::fs::read_to_string(modules_path)?)?;
    let coverage = serde_yaml::from_str(&std::fs::read_to_string(coverage_path)?)?;
    Ok(Some(CodebaseSnapshot { modules, coverage }))
}

fn discover_modules(project_root: &Path) -> anyhow::Result<Vec<CodebaseModule>> {
    let mut modules = Vec::new();
    for entry in std::fs::read_dir(project_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() || should_skip_dir(&path) {
            continue;
        }

        let mut files = BTreeSet::new();
        collect_source_files(&path, &mut files)?;
        if files.is_empty() {
            continue;
        }

        let rel = display_rel(project_root, &path);
        modules.push(CodebaseModule {
            name: rel
                .rsplit('/')
                .next()
                .map(str::to_string)
                .unwrap_or_else(|| rel.clone()),
            path: rel,
            language: language_for_files(&files),
            file_count: files.len(),
        });
    }
    modules.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(modules)
}

fn module_source_files(
    project_root: &Path,
    modules: &[CodebaseModule],
) -> anyhow::Result<BTreeMap<String, BTreeSet<PathBuf>>> {
    let mut files_by_module = BTreeMap::new();
    for module in modules {
        let mut files = BTreeSet::new();
        collect_source_files(&project_root.join(&module.path), &mut files)?;
        files_by_module.insert(module.path.clone(), files);
    }
    Ok(files_by_module)
}

fn component_source_files(
    project_root: &Path,
    component: &Component,
) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = BTreeSet::new();
    for pattern in &component.paths {
        let full_pattern = project_root.join(pattern);
        if contains_glob(pattern) {
            for entry in glob::glob(&full_pattern.to_string_lossy())? {
                let path = entry?;
                collect_source_files(&path, &mut files)?;
            }
        } else {
            collect_source_files(&full_pattern, &mut files)?;
        }
    }
    Ok(files.into_iter().collect())
}

fn collect_source_files(path: &Path, files: &mut BTreeSet<PathBuf>) -> anyhow::Result<()> {
    if path.is_file() {
        if is_source_file(path) {
            files.insert(path.canonicalize()?);
        }
        return Ok(());
    }
    if path.is_dir() {
        if should_skip_dir(path) {
            return Ok(());
        }
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            collect_source_files(&entry.path(), files)?;
        }
    }
    Ok(())
}

fn should_skip_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.starts_with('.')
        || matches!(
            name,
            "target" | "node_modules" | "dist" | "build" | "vendor" | "__pycache__"
        )
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

fn language_for_files(files: &BTreeSet<PathBuf>) -> String {
    let mut languages = BTreeSet::new();
    for file in files {
        languages.insert(match file.extension().and_then(|ext| ext.to_str()) {
            Some("rs") => "Rust",
            Some("py") => "Python",
            Some("ts" | "tsx" | "js" | "jsx") => "TypeScript",
            Some("go") => "Go",
            Some("java") => "Java",
            Some("kt") => "Kotlin",
            Some("swift") => "Swift",
            Some("c" | "cc" | "cpp" | "h" | "hpp") => "C/C++",
            _ => "Unknown",
        });
    }

    if languages.len() == 1 {
        languages
            .into_iter()
            .next()
            .unwrap_or("Unknown")
            .to_string()
    } else {
        "Mixed".to_string()
    }
}

fn product_name<'a>(project_root: &Path, specs: impl Iterator<Item = &'a Spec>) -> String {
    let products = specs
        .map(|spec| spec.product.clone())
        .collect::<BTreeSet<_>>();
    if products.len() == 1 {
        products.into_iter().next().unwrap()
    } else {
        project_root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string()
    }
}

fn contains_glob(pattern: &str) -> bool {
    pattern.contains('*') || pattern.contains('?') || pattern.contains('[')
}

fn display_rel(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}
