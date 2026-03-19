use crate::codebase::storage::CodebaseIndex;
use crate::models::Spec;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub enum WeakestLinkKind {
    UnmatchedArtifact,
    LowConfidenceMatch(f64),
    CrossDomainWithoutTrace,
}

#[derive(Debug, Clone)]
pub struct WeakestLink {
    pub artifact_kind: String,
    pub artifact_id: String,
    pub kind: WeakestLinkKind,
}

#[derive(Debug, Clone)]
pub struct BlastRadius {
    pub domains_touched: Vec<String>,
    pub capabilities_affected: Vec<String>,
    pub components_affected: Vec<String>,
    pub cross_domain_edges: Vec<(String, String, String)>,
    pub files: HashSet<String>,
    pub symbols: usize,
    pub weakest_link: Option<WeakestLink>,
}

pub fn compute_blast_radius(index: &CodebaseIndex, spec: &Spec) -> anyhow::Result<BlastRadius> {
    let links = index.get_trace_links_for_spec(&spec.id)?;

    // Collect unique files and count symbols
    let mut files: HashSet<String> = HashSet::new();
    let mut symbols = 0usize;
    let mut traced_artifacts: HashSet<(String, String)> = HashSet::new();

    for link in &links {
        files.insert(link.file_path.clone());
        if link.symbol_name.is_some() {
            symbols += 1;
        }
        traced_artifacts.insert((link.artifact_kind.clone(), link.artifact_id.clone()));
    }

    let artifacts = match &spec.artifacts {
        Some(a) => a,
        None => {
            return Ok(BlastRadius {
                domains_touched: vec![],
                capabilities_affected: vec![],
                components_affected: vec![],
                cross_domain_edges: vec![],
                files,
                symbols,
                weakest_link: None,
            });
        }
    };

    // Domains touched = domains that have ANY capability/entity/component with a trace link
    let mut domain_has_trace: HashSet<String> = HashSet::new();
    for cap in &artifacts.capabilities {
        if traced_artifacts.contains(&("capability".to_string(), cap.id.clone())) {
            domain_has_trace.insert(cap.domain.clone());
        }
    }
    for entity in &artifacts.entities {
        if traced_artifacts.contains(&("entity".to_string(), entity.id.clone())) {
            domain_has_trace.insert(entity.domain.clone());
        }
    }
    for comp in &artifacts.components {
        if traced_artifacts.contains(&("component".to_string(), comp.id.clone())) {
            domain_has_trace.insert(comp.domain.clone());
        }
    }

    let domains_touched: Vec<String> = artifacts
        .domains
        .iter()
        .filter(|d| domain_has_trace.contains(&d.id))
        .map(|d| {
            let classification = d.classification.as_deref().unwrap_or("unknown");
            format!("{} [{}]", d.name, classification)
        })
        .collect();

    let capabilities_affected: Vec<String> = artifacts
        .capabilities
        .iter()
        .filter(|c| traced_artifacts.contains(&("capability".to_string(), c.id.clone())))
        .map(|c| c.name.clone())
        .collect();

    let components_affected: Vec<String> = artifacts
        .components
        .iter()
        .filter(|c| traced_artifacts.contains(&("component".to_string(), c.id.clone())))
        .map(|c| c.name.clone())
        .collect();

    // Cross-domain edges from spec.links.edges
    let cross_domain_edges: Vec<(String, String, String)> = spec
        .links
        .as_ref()
        .map(|l| {
            l.edges
                .iter()
                .map(|e| (e.from.clone(), e.to.clone(), e.archetype.clone()))
                .collect()
        })
        .unwrap_or_default();

    // Weakest link: priority UnmatchedArtifact > CrossDomainWithoutTrace > LowConfidenceMatch
    let mut weakest_link: Option<WeakestLink> = None;

    // Check for unmatched artifacts (worst)
    let all_artifacts: Vec<(String, String)> = {
        let mut v = Vec::new();
        for d in &artifacts.domains {
            v.push(("domain".into(), d.id.clone()));
        }
        for c in &artifacts.capabilities {
            v.push(("capability".into(), c.id.clone()));
        }
        for e in &artifacts.entities {
            v.push(("entity".into(), e.id.clone()));
        }
        for comp in &artifacts.components {
            v.push(("component".into(), comp.id.clone()));
        }
        v
    };

    for (kind, id) in &all_artifacts {
        if !traced_artifacts.contains(&(kind.clone(), id.clone())) {
            weakest_link = Some(WeakestLink {
                artifact_kind: kind.clone(),
                artifact_id: id.clone(),
                kind: WeakestLinkKind::UnmatchedArtifact,
            });
            break; // First unmatched is enough
        }
    }

    // If no unmatched, check cross-domain edges without trace
    if weakest_link.is_none() {
        if let Some(ref links_struct) = spec.links {
            for edge in &links_struct.edges {
                let from_traced = domain_has_trace.contains(&edge.from);
                let to_traced = domain_has_trace.contains(&edge.to);
                if !from_traced || !to_traced {
                    weakest_link = Some(WeakestLink {
                        artifact_kind: "edge".into(),
                        artifact_id: format!("{}→{}", edge.from, edge.to),
                        kind: WeakestLinkKind::CrossDomainWithoutTrace,
                    });
                    break;
                }
            }
        }
    }

    // If no unmatched or untrace edges, find lowest confidence match
    if weakest_link.is_none() && !links.is_empty() {
        let min_link = links
            .iter()
            .min_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap());
        if let Some(link) = min_link {
            if link.confidence < 0.6 {
                weakest_link = Some(WeakestLink {
                    artifact_kind: link.artifact_kind.clone(),
                    artifact_id: link.artifact_id.clone(),
                    kind: WeakestLinkKind::LowConfidenceMatch(link.confidence),
                });
            }
        }
    }

    Ok(BlastRadius {
        domains_touched,
        capabilities_affected,
        components_affected,
        cross_domain_edges,
        files,
        symbols,
        weakest_link,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebase::storage::CodebaseIndex;
    use crate::models::*;
    use tempfile::TempDir;

    fn make_spec(id: &str) -> Spec {
        Spec {
            s5d: "1.0".into(),
            id: id.into(),
            version: "1.0.0".into(),
            product: "TestProduct".into(),
            tier: Tier::Standard,
            allow_update: false,
            meta: None,
            context: None,
            artifacts: None,
            links: None,
            contracts: vec![],
            gates: vec![],
            roc: None,
            problem: None,
            hypotheses: vec![],
            decision: None,
            note_rationale: None,
            expires_at: None,
            auto_noted: false,
        }
    }

    fn make_index() -> (TempDir, CodebaseIndex) {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let index = CodebaseIndex::open(&db_path).unwrap();
        (dir, index)
    }

    fn make_trace_link(
        spec_id: &str,
        kind: &str,
        artifact_id: &str,
        file_path: &str,
        confidence: f64,
    ) -> TraceLink {
        TraceLink {
            spec_id: spec_id.into(),
            artifact_kind: kind.into(),
            artifact_id: artifact_id.into(),
            file_path: file_path.into(),
            symbol_name: Some("some_fn".into()),
            line_start: 10,
            line_end: Some(20),
            source: "annotated".into(),
            confidence,
            updated_at: "2026-03-29".into(),
        }
    }

    #[test]
    fn test_blast_radius_empty_spec() {
        let (_dir, index) = make_index();
        let spec = make_spec("empty-spec");

        let br = compute_blast_radius(&index, &spec).unwrap();

        assert!(br.domains_touched.is_empty());
        assert!(br.capabilities_affected.is_empty());
        assert!(br.components_affected.is_empty());
        assert!(br.files.is_empty());
        assert_eq!(br.symbols, 0);
        assert!(br.weakest_link.is_none());
    }

    #[test]
    fn test_blast_radius_with_traced_artifacts() {
        let (_dir, index) = make_index();

        // 2 capabilities in d1 (traced), 1 in d2 (untraced)
        // No domain-level artifacts — only capabilities — so WLNK is the untraced capability
        index
            .insert_trace_link(&make_trace_link(
                "spec-traced",
                "capability",
                "cap-1",
                "src/auth.rs",
                0.95,
            ))
            .unwrap();
        index
            .insert_trace_link(&make_trace_link(
                "spec-traced",
                "capability",
                "cap-2",
                "src/billing.rs",
                0.90,
            ))
            .unwrap();

        let mut spec = make_spec("spec-traced");
        spec.artifacts = Some(Artifacts {
            domains: vec![
                Domain {
                    id: "d1".into(),
                    product: "P".into(),
                    name: "Auth".into(),
                    classification: Some("core".into()),
                    description: None,
                    team: None,
                    maturity_level: None,
                },
                Domain {
                    id: "d2".into(),
                    product: "P".into(),
                    name: "Payments".into(),
                    classification: Some("supporting".into()),
                    description: None,
                    team: None,
                    maturity_level: None,
                },
            ],
            capabilities: vec![
                Capability {
                    id: "cap-1".into(),
                    domain: "d1".into(),
                    name: "Authenticate".into(),
                    description: None,
                    since: None,
                },
                Capability {
                    id: "cap-2".into(),
                    domain: "d1".into(),
                    name: "Authorize".into(),
                    description: None,
                    since: None,
                },
                Capability {
                    id: "cap-3".into(),
                    domain: "d2".into(),
                    name: "ChargeCard".into(),
                    description: None,
                    since: None,
                },
            ],
            ..Default::default()
        });
        // Also trace the domains so that caps are the first untraced artifacts
        index
            .insert_trace_link(&make_trace_link(
                "spec-traced",
                "domain",
                "d1",
                "src/auth.rs",
                0.99,
            ))
            .unwrap();
        index
            .insert_trace_link(&make_trace_link(
                "spec-traced",
                "domain",
                "d2",
                "src/payments.rs",
                0.99,
            ))
            .unwrap();

        let br = compute_blast_radius(&index, &spec).unwrap();

        // d1 has 2 traced capabilities, d2 has none
        assert_eq!(br.domains_touched.len(), 1);
        assert!(br.domains_touched[0].contains("Auth"));

        assert_eq!(br.capabilities_affected.len(), 2);
        assert!(br
            .capabilities_affected
            .contains(&"Authenticate".to_string()));
        assert!(br.capabilities_affected.contains(&"Authorize".to_string()));

        assert!(br.files.len() >= 2);

        // cap-3 untraced → WLNK = UnmatchedArtifact
        let wlnk = br.weakest_link.expect("expected weakest_link");
        assert!(matches!(wlnk.kind, WeakestLinkKind::UnmatchedArtifact));
        assert_eq!(wlnk.artifact_id, "cap-3");
    }

    #[test]
    fn test_blast_radius_cross_domain_edges() {
        let (_dir, index) = make_index();

        // Both domains traced at domain level; cap-1 in d1 traced; d2 has no capability
        // Edge d1→d2: d2 has no capability trace → domain_has_trace won't include d2
        // → CrossDomainWithoutTrace
        index
            .insert_trace_link(&make_trace_link(
                "spec-cross",
                "domain",
                "d1",
                "src/a.rs",
                0.99,
            ))
            .unwrap();
        index
            .insert_trace_link(&make_trace_link(
                "spec-cross",
                "domain",
                "d2",
                "src/b.rs",
                0.99,
            ))
            .unwrap();
        index
            .insert_trace_link(&make_trace_link(
                "spec-cross",
                "capability",
                "cap-1",
                "src/a.rs",
                0.9,
            ))
            .unwrap();

        let mut spec = make_spec("spec-cross");
        spec.artifacts = Some(Artifacts {
            domains: vec![
                Domain {
                    id: "d1".into(),
                    product: "P".into(),
                    name: "Orders".into(),
                    classification: Some("core".into()),
                    description: None,
                    team: None,
                    maturity_level: None,
                },
                Domain {
                    id: "d2".into(),
                    product: "P".into(),
                    name: "Inventory".into(),
                    classification: Some("generic".into()),
                    description: None,
                    team: None,
                    maturity_level: None,
                },
            ],
            capabilities: vec![Capability {
                id: "cap-1".into(),
                domain: "d1".into(),
                name: "PlaceOrder".into(),
                description: None,
                since: None,
            }],
            ..Default::default()
        });
        spec.links = Some(Links {
            edges: vec![Edge {
                from: "d1".into(),
                to: "d2".into(),
                archetype: "OHS".into(),
                description: None,
                downstream_capability: None,
                waiver: None,
                transport_ref: None,
            }],
            ..Default::default()
        });

        let br = compute_blast_radius(&index, &spec).unwrap();

        // d2 has no capability-level trace → domain_has_trace misses d2 → CrossDomainWithoutTrace
        let wlnk = br.weakest_link.expect("expected weakest_link");
        assert!(matches!(
            wlnk.kind,
            WeakestLinkKind::CrossDomainWithoutTrace
        ));
    }

    #[test]
    fn test_blast_radius_low_confidence_wlnk() {
        let (_dir, index) = make_index();

        // Both capabilities traced (no unmatched), but cap-2 has low confidence
        index
            .insert_trace_link(&make_trace_link(
                "spec-low",
                "capability",
                "cap-1",
                "src/a.rs",
                0.9,
            ))
            .unwrap();
        index
            .insert_trace_link(&TraceLink {
                spec_id: "spec-low".into(),
                artifact_kind: "capability".into(),
                artifact_id: "cap-2".into(),
                file_path: "src/b.rs".into(),
                symbol_name: None,
                line_start: 5,
                line_end: None,
                source: "inferred".into(),
                confidence: 0.4,
                updated_at: "2026-03-29".into(),
            })
            .unwrap();

        // No domain artifacts — only capabilities — so no domain-level unmatched
        let mut spec = make_spec("spec-low");
        spec.artifacts = Some(Artifacts {
            capabilities: vec![
                Capability {
                    id: "cap-1".into(),
                    domain: "d1".into(),
                    name: "Cap1".into(),
                    description: None,
                    since: None,
                },
                Capability {
                    id: "cap-2".into(),
                    domain: "d1".into(),
                    name: "Cap2".into(),
                    description: None,
                    since: None,
                },
            ],
            ..Default::default()
        });

        let br = compute_blast_radius(&index, &spec).unwrap();

        let wlnk = br.weakest_link.expect("expected weakest_link");
        match wlnk.kind {
            WeakestLinkKind::LowConfidenceMatch(c) => assert!((c - 0.4).abs() < 1e-6),
            other => panic!("expected LowConfidenceMatch, got {:?}", other),
        }
    }

    #[test]
    fn test_blast_radius_wlnk_priority() {
        let (_dir, index) = make_index();

        // cap-1 has low confidence trace; cap-2 has no trace at all
        // No domain artifacts — so first unmatched is cap-2 (capabilities come before domains in
        // all_artifacts, but the code pushes domains first). Use only capabilities to avoid domain noise.
        index
            .insert_trace_link(&TraceLink {
                spec_id: "spec-priority".into(),
                artifact_kind: "capability".into(),
                artifact_id: "cap-1".into(),
                file_path: "src/a.rs".into(),
                symbol_name: None,
                line_start: 1,
                line_end: None,
                source: "inferred".into(),
                confidence: 0.3,
                updated_at: "2026-03-29".into(),
            })
            .unwrap();
        // cap-2 intentionally has no trace link

        let mut spec = make_spec("spec-priority");
        // Only capabilities — no domain artifacts so UnmatchedArtifact points to cap-2
        spec.artifacts = Some(Artifacts {
            capabilities: vec![
                Capability {
                    id: "cap-1".into(),
                    domain: "d1".into(),
                    name: "Cap1".into(),
                    description: None,
                    since: None,
                },
                Capability {
                    id: "cap-2".into(),
                    domain: "d1".into(),
                    name: "Cap2".into(),
                    description: None,
                    since: None,
                },
            ],
            ..Default::default()
        });

        let br = compute_blast_radius(&index, &spec).unwrap();

        // UnmatchedArtifact (cap-2) has higher priority than LowConfidenceMatch
        let wlnk = br.weakest_link.expect("expected weakest_link");
        assert!(matches!(wlnk.kind, WeakestLinkKind::UnmatchedArtifact));
        assert_eq!(wlnk.artifact_id, "cap-2");
    }
}
