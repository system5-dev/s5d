use std::collections::HashSet;
use std::path::Path;

use chrono::Utc;

use crate::codebase::annotations;
use crate::codebase::storage::CodebaseIndex;
use crate::models::{Spec, TraceLink, TraceReport};

/// Build trace links by combining annotations + symbol inference.
/// Clears existing links for this spec, then writes new ones.
pub fn build_trace(
    index: &CodebaseIndex,
    spec: &Spec,
    root: &Path,
) -> anyhow::Result<Vec<TraceLink>> {
    let now = Utc::now().to_rfc3339();
    let mut links: Vec<TraceLink> = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();

    // 1. Scan for @s5d: annotations
    let annotations = annotations::scan_directory(root);
    for hit in &annotations {
        let key = (hit.artifact_kind.clone(), hit.artifact_id.clone());
        seen.insert(key);
        links.push(TraceLink {
            spec_id: spec.id.clone(),
            artifact_kind: hit.artifact_kind.clone(),
            artifact_id: hit.artifact_id.clone(),
            file_path: hit.file_path.clone(),
            symbol_name: None,
            line_start: hit.line,
            line_end: None,
            source: "annotated".into(),
            confidence: 0.95,
            updated_at: now.clone(),
        });
    }

    let artifacts = match &spec.artifacts {
        Some(a) => a,
        None => {
            index.delete_trace_links_for_spec(&spec.id)?;
            for link in &links {
                index.insert_trace_link(link)?;
            }
            return Ok(links);
        }
    };

    // 2. Infer from symbol names — match capabilities and entities
    for cap in &artifacts.capabilities {
        let key = ("capability".to_string(), cap.id.clone());
        if seen.contains(&key) {
            continue;
        }
        if let Ok(syms) = index.symbol_lookup(&cap.name) {
            for sym in &syms {
                seen.insert(key.clone());
                links.push(TraceLink {
                    spec_id: spec.id.clone(),
                    artifact_kind: "capability".into(),
                    artifact_id: cap.id.clone(),
                    file_path: sym.file_path.clone(),
                    symbol_name: Some(sym.name.clone()),
                    line_start: sym.line,
                    line_end: sym.line_end,
                    source: "inferred".into(),
                    confidence: 0.7,
                    updated_at: now.clone(),
                });
            }
        }
    }

    for entity in &artifacts.entities {
        let key = ("entity".to_string(), entity.id.clone());
        if seen.contains(&key) {
            continue;
        }
        if let Ok(syms) = index.symbol_lookup(&entity.name) {
            for sym in &syms {
                seen.insert(key.clone());
                links.push(TraceLink {
                    spec_id: spec.id.clone(),
                    artifact_kind: "entity".into(),
                    artifact_id: entity.id.clone(),
                    file_path: sym.file_path.clone(),
                    symbol_name: Some(sym.name.clone()),
                    line_start: sym.line,
                    line_end: sym.line_end,
                    source: "inferred".into(),
                    confidence: 0.7,
                    updated_at: now.clone(),
                });
            }
        }
    }

    // 3. Components — match by paths field
    for comp in &artifacts.components {
        let key = ("component".to_string(), comp.id.clone());
        if seen.contains(&key) {
            continue;
        }
        for path in &comp.paths {
            seen.insert(key.clone());
            links.push(TraceLink {
                spec_id: spec.id.clone(),
                artifact_kind: "component".into(),
                artifact_id: comp.id.clone(),
                file_path: path.clone(),
                symbol_name: None,
                line_start: 0,
                line_end: None,
                source: "inferred".into(),
                confidence: 0.5,
                updated_at: now.clone(),
            });
        }
    }

    // Write to DB
    index.delete_trace_links_for_spec(&spec.id)?;
    for link in &links {
        index.insert_trace_link(link)?;
    }

    Ok(links)
}

/// Check trace links for a spec — report matched, unmatched, orphaned.
pub fn check_trace(index: &CodebaseIndex, spec: &Spec) -> anyhow::Result<TraceReport> {
    let links = index.get_trace_links_for_spec(&spec.id)?;

    let mut matched_kinds: HashSet<(String, String)> = HashSet::new();
    for link in &links {
        matched_kinds.insert((link.artifact_kind.clone(), link.artifact_id.clone()));
    }

    let mut unmatched_spec = Vec::new();

    if let Some(ref artifacts) = spec.artifacts {
        for d in &artifacts.domains {
            let key = ("domain".to_string(), d.id.clone());
            if !matched_kinds.contains(&key) {
                unmatched_spec.push(key);
            }
        }
        for c in &artifacts.capabilities {
            let key = ("capability".to_string(), c.id.clone());
            if !matched_kinds.contains(&key) {
                unmatched_spec.push(key);
            }
        }
        for e in &artifacts.entities {
            let key = ("entity".to_string(), e.id.clone());
            if !matched_kinds.contains(&key) {
                unmatched_spec.push(key);
            }
        }
        for comp in &artifacts.components {
            let key = ("component".to_string(), comp.id.clone());
            if !matched_kinds.contains(&key) {
                unmatched_spec.push(key);
            }
        }
    }

    Ok(TraceReport {
        matched: links,
        unmatched_spec,
        orphaned_annotations: vec![],
    })
}
