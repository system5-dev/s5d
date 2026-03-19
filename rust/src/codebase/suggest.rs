use std::collections::{HashMap, HashSet};

use crate::codebase::parser::ImportEdge;
use crate::codebase::storage::CodebaseIndex;

/// A suggested domain boundary derived from coupling analysis.
#[derive(Debug, Clone)]
pub struct SuggestedDomain {
    pub name: String,
    pub classification: String, // core, supporting, generic
    pub files: Vec<String>,
    pub symbols: usize,
    pub ca: usize,        // afferent coupling (who depends on me)
    pub ce: usize,        // efferent coupling (who I depend on)
    pub instability: f64, // Ce / (Ca + Ce)
    pub cohesion: f64,    // internal_edges / total_edges
    pub pagerank: f64,    // aggregate PageRank centrality
    pub confidence: f64,  // 0.0-1.0 how confident we are this is a good boundary
}

/// Suggestion report with all domains and merge candidates.
#[derive(Debug, Clone)]
pub struct SuggestionReport {
    pub domains: Vec<SuggestedDomain>,
    pub merge_candidates: Vec<MergeCandidate>,
    pub total_files: usize,
    pub total_edges: usize,
}

/// Two modules with high mutual coupling — candidates for merging into one domain.
#[derive(Debug, Clone)]
pub struct MergeCandidate {
    pub module_a: String,
    pub module_b: String,
    pub mutual_edges: usize,
    pub coupling_ratio: f64, // mutual_edges / min(total_edges_a, total_edges_b)
    pub reason: String,
}

/// Run coupling-based domain suggestion on the codebase index.
///
/// Algorithm:
/// 1. Load import edges from codebase.db
/// 2. Group files into modules by directory path at given depth
/// 3. Build module-level dependency graph
/// 4. Compute coupling metrics (Ca, Ce, Instability) per module
/// 5. Compute cohesion (internal_edges / total_edges) per module
/// 6. Classify by PageRank centrality: high=core, mid=supporting, low=generic
/// 7. Identify merge candidates (high mutual coupling between modules)
pub fn suggest_domains(
    index: &CodebaseIndex,
    depth: usize,
    min_files: usize,
) -> anyhow::Result<SuggestionReport> {
    let mut edges = index.get_import_edges()?;

    // Relativize absolute paths: find common prefix and strip it
    let common_prefix = find_common_prefix(&edges);
    if !common_prefix.is_empty() {
        for e in &mut edges {
            if let Some(rel) = e.source_file.strip_prefix(&common_prefix) {
                e.source_file = rel.trim_start_matches('/').to_string();
            }
        }
    }
    if edges.is_empty() {
        return Ok(SuggestionReport {
            domains: vec![],
            merge_candidates: vec![],
            total_files: 0,
            total_edges: 0,
        });
    }

    // Step 1: Collect all files from edges
    let mut all_files = HashSet::new();
    for e in &edges {
        all_files.insert(e.source_file.clone());
        // imported might be a module path, not a file — resolve later
    }

    // Step 2: Group files into modules by directory depth
    let modules = group_by_module(&edges, depth);

    // Step 3: Build module-level edge matrix
    let (module_edges, internal_edges) = build_module_edges(&edges, &modules, depth);

    // Step 4: Compute metrics per module
    let mut module_files: HashMap<String, Vec<String>> = HashMap::new();
    for (file, module) in &modules {
        module_files
            .entry(module.clone())
            .or_default()
            .push(file.clone());
    }

    // Count symbols per module from the index
    let symbol_counts = count_symbols_per_module(index, &module_files, &common_prefix);

    // PageRank at file level, then aggregate to module level
    let graph = crate::codebase::graph::ImportGraph::from_index(index)?;
    let file_pr = graph.pagerank(0.85, 20);
    let module_pr = aggregate_pagerank(&file_pr, &modules);

    // Compute Ca/Ce per module
    let mut ca: HashMap<String, usize> = HashMap::new();
    let mut ce: HashMap<String, usize> = HashMap::new();
    for ((from, to), count) in &module_edges {
        *ce.entry(from.clone()).or_default() += count;
        *ca.entry(to.clone()).or_default() += count;
    }

    // Step 5: Build suggested domains
    let total_edges = edges.len();
    let mut domains: Vec<SuggestedDomain> = Vec::new();

    for (module, files) in &module_files {
        if files.len() < min_files {
            continue;
        }

        let ca_val = ca.get(module).copied().unwrap_or(0);
        let ce_val = ce.get(module).copied().unwrap_or(0);
        let instability = if ca_val + ce_val == 0 {
            0.0
        } else {
            ce_val as f64 / (ca_val + ce_val) as f64
        };

        let int_edges = internal_edges.get(module).copied().unwrap_or(0);
        let total_module_edges = ca_val + ce_val + int_edges;
        let cohesion = if total_module_edges == 0 {
            1.0
        } else {
            int_edges as f64 / total_module_edges as f64
        };

        let pr = module_pr.get(module).copied().unwrap_or(0.0);
        let symbols = symbol_counts.get(module).copied().unwrap_or(0);

        // Confidence: higher cohesion + more files + more symbols = more confident boundary
        let size_factor = (files.len() as f64).min(20.0) / 20.0;
        let confidence = (cohesion * 0.5 + size_factor * 0.3 + (1.0 - instability) * 0.2)
            .clamp(0.0, 1.0);

        domains.push(SuggestedDomain {
            name: module.clone(),
            classification: String::new(), // classified below
            files: files.clone(),
            symbols,
            ca: ca_val,
            ce: ce_val,
            instability,
            cohesion,
            pagerank: pr,
            confidence,
        });
    }

    // Step 6: Classify by PageRank centrality
    classify_domains(&mut domains);

    // Sort: core first, then by PageRank descending
    domains.sort_by(|a, b| {
        let class_ord = |c: &str| match c {
            "core" => 0,
            "supporting" => 1,
            _ => 2,
        };
        class_ord(&a.classification)
            .cmp(&class_ord(&b.classification))
            .then(b.pagerank.partial_cmp(&a.pagerank).unwrap_or(std::cmp::Ordering::Equal))
    });

    // Step 7: Identify merge candidates
    let merge_candidates = find_merge_candidates(&module_edges, &module_files);

    Ok(SuggestionReport {
        total_files: all_files.len(),
        total_edges,
        domains,
        merge_candidates,
    })
}

/// Find common path prefix across all source files in edges.
fn find_common_prefix(edges: &[ImportEdge]) -> String {
    let files: Vec<&str> = edges.iter().map(|e| e.source_file.as_str()).collect();
    if files.is_empty() {
        return String::new();
    }
    let first = files[0];
    let mut prefix_len = 0;
    for (i, ch) in first.char_indices() {
        if files.iter().all(|f| f.get(..=i).map(|s| s.ends_with(ch)).unwrap_or(false))
            && files.iter().all(|f| f.len() > i && f.as_bytes()[i] == ch as u8)
        {
            if ch == '/' {
                prefix_len = i + 1;
            }
        } else {
            break;
        }
    }
    first[..prefix_len].to_string()
}

/// Group files into modules by directory path at given depth.
/// depth=1: src/auth/login.rs → "auth"
/// depth=2: src/auth/oauth/provider.rs → "auth/oauth"
fn group_by_module(edges: &[ImportEdge], depth: usize) -> HashMap<String, String> {
    let mut modules = HashMap::new();

    let extract_module = |path: &str| -> String {
        let parts: Vec<&str> = path.split('/').collect();
        // Skip common prefixes: src/, lib/, pkg/
        let skip = parts
            .iter()
            .position(|&p| p != "src" && p != "lib" && p != "pkg" && p != ".")
            .unwrap_or(0);
        let meaningful: Vec<&str> = parts[skip..].to_vec();
        if meaningful.len() <= 1 {
            return "root".to_string();
        }
        // Take `depth` directory levels (excluding filename)
        let dirs: Vec<&str> = meaningful[..meaningful.len() - 1]
            .iter()
            .take(depth)
            .copied()
            .collect();
        if dirs.is_empty() {
            "root".to_string()
        } else {
            dirs.join("/")
        }
    };

    for e in edges {
        let module = extract_module(&e.source_file);
        modules.insert(e.source_file.clone(), module);
    }

    modules
}

/// Build module-level edges: (from_module, to_module) → count.
/// Also returns internal edge count per module.
fn build_module_edges(
    edges: &[ImportEdge],
    modules: &HashMap<String, String>,
    depth: usize,
) -> (HashMap<(String, String), usize>, HashMap<String, usize>) {
    let mut module_edges: HashMap<(String, String), usize> = HashMap::new();
    let mut internal: HashMap<String, usize> = HashMap::new();

    for e in edges {
        let from_module = modules.get(&e.source_file).cloned().unwrap_or_default();

        // Resolve imported module: try to match against known file modules
        let imported_module = resolve_import_module(&e.imported, modules, depth);

        if from_module == imported_module || imported_module.is_empty() {
            *internal.entry(from_module).or_default() += 1;
        } else {
            *module_edges
                .entry((from_module, imported_module))
                .or_default() += 1;
        }
    }

    (module_edges, internal)
}

/// Try to resolve an import statement to a module name.
fn resolve_import_module(
    imported: &str,
    modules: &HashMap<String, String>,
    depth: usize,
) -> String {
    // Clean import: strip "use ", "from ", "import " prefixes and quotes
    let clean = imported
        .trim()
        .trim_start_matches("use ")
        .trim_start_matches("from ")
        .trim_start_matches("import ")
        .trim_matches('"')
        .trim_matches('\'')
        .trim_end_matches(';');

    // Try to match against known files
    for (file, module) in modules {
        // Check if the import references this file's module
        let file_stem = file
            .trim_end_matches(".rs")
            .trim_end_matches(".py")
            .trim_end_matches(".ts")
            .trim_end_matches(".tsx")
            .trim_end_matches(".js")
            .trim_end_matches(".jsx")
            .trim_end_matches(".go")
            .trim_end_matches(".java");

        if clean.contains(&module.replace('/', "::"))
            || clean.contains(&module.replace('/', "."))
            || clean.contains(module.as_str())
        {
            return module.clone();
        }
        // Check if import contains file stem
        let stem_parts: Vec<&str> = file_stem.split('/').collect();
        if let Some(last) = stem_parts.last() {
            if clean.contains(last) && !["mod", "index", "lib", "main", "__init__"].contains(last) {
                return module.clone();
            }
        }
    }

    // Fallback: extract module from import path structure
    let parts: Vec<&str> = clean
        .split([':',  '.', '/'])
        .filter(|s| !s.is_empty())
        .collect();

    // Skip common prefixes
    let skip = parts
        .iter()
        .position(|&p| {
            !["crate", "super", "self", "std", "core", "alloc", "src", "lib"].contains(&p)
        })
        .unwrap_or(0);

    let meaningful: Vec<&str> = parts[skip..].to_vec();
    if meaningful.is_empty() {
        return String::new();
    }

    meaningful[..meaningful.len().min(depth)].join("/")
}

/// Count symbols per module by querying the index.
/// `prefix` is the common prefix stripped from file paths — needed to reconstruct absolute paths for DB queries.
fn count_symbols_per_module(
    index: &CodebaseIndex,
    module_files: &HashMap<String, Vec<String>>,
    prefix: &str,
) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for (module, files) in module_files {
        let mut total = 0;
        for file in files {
            // Reconstruct full path for DB query
            let full_path = if prefix.is_empty() {
                file.clone()
            } else {
                format!("{}{}", prefix, file)
            };
            if let Ok(syms) = index.symbols_in_file(&full_path) {
                total += syms;
            }
        }
        counts.insert(module.clone(), total);
    }
    counts
}

/// Aggregate file-level PageRank to module level.
fn aggregate_pagerank(
    file_pr: &HashMap<String, f64>,
    modules: &HashMap<String, String>,
) -> HashMap<String, f64> {
    let mut module_pr: HashMap<String, f64> = HashMap::new();
    for (file, pr) in file_pr {
        if let Some(module) = modules.get(file) {
            *module_pr.entry(module.clone()).or_default() += pr;
        }
    }
    module_pr
}

/// Classify domains by PageRank centrality distribution.
/// Top 30% = core, middle 40% = supporting, bottom 30% = generic.
fn classify_domains(domains: &mut [SuggestedDomain]) {
    if domains.is_empty() {
        return;
    }

    let mut prs: Vec<f64> = domains.iter().map(|d| d.pagerank).collect();
    prs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let len = prs.len();
    let p70_idx = (len * 70 / 100).min(len - 1);
    let p30_idx = (len * 30 / 100).min(len - 1);
    let p70 = prs[p70_idx];
    let p30 = prs[p30_idx];

    for d in domains.iter_mut() {
        d.classification = if d.pagerank > p70 || (d.pagerank == p70 && p70 > 0.0) {
            "core".to_string()
        } else if d.pagerank > p30 {
            "supporting".to_string()
        } else {
            "generic".to_string()
        };
    }
}

/// Find module pairs with high mutual coupling — merge candidates.
fn find_merge_candidates(
    module_edges: &HashMap<(String, String), usize>,
    module_files: &HashMap<String, Vec<String>>,
) -> Vec<MergeCandidate> {
    let mut mutual: HashMap<(String, String), usize> = HashMap::new();

    for ((from, to), count) in module_edges {
        let key = if from < to {
            (from.clone(), to.clone())
        } else {
            (to.clone(), from.clone())
        };
        *mutual.entry(key).or_default() += count;
    }

    let mut candidates: Vec<MergeCandidate> = Vec::new();

    for ((a, b), edges) in &mutual {
        let a_files = module_files.get(a).map(|f| f.len()).unwrap_or(0);
        let b_files = module_files.get(b).map(|f| f.len()).unwrap_or(0);
        let min_size = a_files.min(b_files);

        if min_size == 0 || *edges < 3 {
            continue;
        }

        // Total outgoing edges for each module
        let a_total: usize = module_edges
            .iter()
            .filter(|((from, _), _)| from == a)
            .map(|(_, c)| c)
            .sum();
        let b_total: usize = module_edges
            .iter()
            .filter(|((from, _), _)| from == b)
            .map(|(_, c)| c)
            .sum();

        let min_total = a_total.min(b_total).max(1);
        let ratio = *edges as f64 / min_total as f64;

        if ratio > 0.3 {
            candidates.push(MergeCandidate {
                module_a: a.clone(),
                module_b: b.clone(),
                mutual_edges: *edges,
                coupling_ratio: ratio,
                reason: format!(
                    "{} mutual imports ({:.0}% of smaller module's dependencies)",
                    edges,
                    ratio * 100.0
                ),
            });
        }
    }

    candidates.sort_by(|a, b| {
        b.coupling_ratio
            .partial_cmp(&a.coupling_ratio)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_edges(pairs: &[(&str, &str)]) -> Vec<ImportEdge> {
        pairs
            .iter()
            .map(|(s, i)| ImportEdge {
                source_file: s.to_string(),
                imported: i.to_string(),
            })
            .collect()
    }

    #[test]
    fn test_group_by_module_depth1() {
        let edges = make_edges(&[
            ("src/auth/login.rs", "use crate::db"),
            ("src/auth/register.rs", "use crate::db"),
            ("src/billing/invoice.rs", "use crate::auth"),
        ]);
        let modules = group_by_module(&edges, 1);
        assert_eq!(modules["src/auth/login.rs"], "auth");
        assert_eq!(modules["src/auth/register.rs"], "auth");
        assert_eq!(modules["src/billing/invoice.rs"], "billing");
    }

    #[test]
    fn test_group_by_module_depth2() {
        let edges = make_edges(&[
            ("src/auth/oauth/provider.rs", "use something"),
            ("src/auth/basic/handler.rs", "use something"),
        ]);
        let modules = group_by_module(&edges, 2);
        assert_eq!(modules["src/auth/oauth/provider.rs"], "auth/oauth");
        assert_eq!(modules["src/auth/basic/handler.rs"], "auth/basic");
    }

    #[test]
    fn test_module_edges_internal_vs_external() {
        let edges = make_edges(&[
            ("src/auth/login.rs", "use crate::auth::register"),
            ("src/auth/login.rs", "use crate::billing::invoice"),
            ("src/billing/invoice.rs", "use crate::auth::login"),
        ]);
        let modules = group_by_module(&edges, 1);
        let (ext, int) = build_module_edges(&edges, &modules, 1);

        // auth → auth = internal
        assert_eq!(int.get("auth").copied().unwrap_or(0), 1);
        // auth → billing = external
        assert!(ext.contains_key(&("auth".into(), "billing".into())));
        // billing → auth = external
        assert!(ext.contains_key(&("billing".into(), "auth".into())));
    }

    #[test]
    fn test_classify_domains() {
        let mut domains = vec![
            SuggestedDomain {
                name: "core-mod".into(),
                classification: String::new(),
                files: vec![],
                symbols: 0,
                ca: 10,
                ce: 2,
                instability: 0.17,
                cohesion: 0.8,
                pagerank: 0.5,
                confidence: 0.9,
            },
            SuggestedDomain {
                name: "mid-mod".into(),
                classification: String::new(),
                files: vec![],
                symbols: 0,
                ca: 5,
                ce: 5,
                instability: 0.5,
                cohesion: 0.5,
                pagerank: 0.2,
                confidence: 0.7,
            },
            SuggestedDomain {
                name: "leaf-mod".into(),
                classification: String::new(),
                files: vec![],
                symbols: 0,
                ca: 1,
                ce: 8,
                instability: 0.89,
                cohesion: 0.3,
                pagerank: 0.05,
                confidence: 0.5,
            },
        ];
        classify_domains(&mut domains);
        assert_eq!(domains[0].classification, "core");
        // At least one should be generic
        assert!(domains.iter().any(|d| d.classification == "generic"));
    }
}
