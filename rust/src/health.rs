use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use chrono::Utc;

use crate::graph::{check_domain_layering, tarjan_scc};
use crate::models::*;

// ── Coupling metrics ─────────────────────────────────────────────────────────

pub fn compute_domain_metrics(spec: &Spec, sccs: &[Vec<String>]) -> Vec<DomainMetrics> {
    let domains = match &spec.artifacts {
        Some(a) => &a.domains,
        None => return Vec::new(),
    };

    let edges = match &spec.links {
        Some(l) => &l.edges,
        None => {
            return domains
                .iter()
                .map(|d| DomainMetrics {
                    domain_id: d.id.clone(),
                    ca: 0,
                    ce: 0,
                    instability: 0.0,
                    cycle_member: false,
                    health_score: 100,
                    violations: Vec::new(),
                })
                .collect();
        }
    };

    let scc_members: HashSet<&str> = sccs.iter().flat_map(|scc| scc.iter().map(|s| s.as_str())).collect();

    let domain_ids: HashSet<&str> = domains.iter().map(|d| d.id.as_str()).collect();

    let mut ca_map: HashMap<&str, usize> = HashMap::new();
    let mut ce_map: HashMap<&str, usize> = HashMap::new();

    for edge in edges {
        let from = edge.from.as_str();
        let to = edge.to.as_str();
        // separate_ways = intentional decoupling, not a real dependency
        if edge.archetype == "separate_ways" {
            continue;
        }
        if domain_ids.contains(from) && domain_ids.contains(to) && from != to {
            *ce_map.entry(from).or_default() += 1;
            *ca_map.entry(to).or_default() += 1;
        }
    }

    domains
        .iter()
        .map(|d| {
            let id = d.id.as_str();
            let ca = *ca_map.get(id).unwrap_or(&0);
            let ce = *ce_map.get(id).unwrap_or(&0);
            let instability = if ca + ce == 0 {
                0.0
            } else {
                ce as f64 / (ca + ce) as f64
            };
            let cycle_member = scc_members.contains(id);

            let mut violations = Vec::new();
            let mut penalty: u8 = 0;

            if cycle_member {
                violations.push("cycle_member".into());
                penalty = penalty.saturating_add(15);
            }
            if instability > 0.8 && ce > 3 {
                violations.push(format!("fragile (I={instability:.2}, Ce={ce})"));
                penalty = penalty.saturating_add(5);
            }
            if ca > 5 {
                violations.push(format!("hub (Ca={ca})"));
                penalty = penalty.saturating_add(3);
            }
            if ce > 5 {
                violations.push(format!("scattered (Ce={ce})"));
                penalty = penalty.saturating_add(3);
            }

            DomainMetrics {
                domain_id: d.id.clone(),
                ca,
                ce,
                instability,
                cycle_member,
                health_score: 100u8.saturating_sub(penalty),
                violations,
            }
        })
        .collect()
}

// ── Health report ────────────────────────────────────────────────────────────

pub fn compute_health_report(spec: &Spec) -> HealthReport {
    let sccs = tarjan_scc(spec);
    let layering_errors = check_domain_layering(spec);
    let mut domain_metrics = compute_domain_metrics(spec, &sccs);

    let mut violations: Vec<HealthViolation> = Vec::new();

    // Cycle violations
    for scc in &sccs {
        let msg = format!("cycle: {}", scc.join(" → "));
        for member in scc {
            violations.push(HealthViolation {
                kind: "cycle".into(),
                target: member.clone(),
                message: msg.clone(),
                penalty: 15,
            });
        }
    }

    // Layering violations
    for err in &layering_errors {
        // Extract target domain from error message
        let target = err.split(':').nth(1).unwrap_or("unknown").trim().split(' ').next().unwrap_or("unknown");
        violations.push(HealthViolation {
            kind: "layering".into(),
            target: target.into(),
            message: err.clone(),
            penalty: 10,
        });
        // Apply layering penalty to the offending domain
        if let Some(dm) = domain_metrics.iter_mut().find(|m| m.domain_id == target) {
            dm.health_score = dm.health_score.saturating_sub(10);
            dm.violations.push("layering_violation".into());
        }
    }

    // Coupling violations
    for dm in &domain_metrics {
        if dm.instability > 0.8 && dm.ce > 3 {
            violations.push(HealthViolation {
                kind: "high_instability".into(),
                target: dm.domain_id.clone(),
                message: format!("{}: I={:.2}, Ce={}", dm.domain_id, dm.instability, dm.ce),
                penalty: 5,
            });
        }
        if dm.ca > 5 {
            violations.push(HealthViolation {
                kind: "hub_domain".into(),
                target: dm.domain_id.clone(),
                message: format!("{}: Ca={} (hub)", dm.domain_id, dm.ca),
                penalty: 3,
            });
        }
    }

    // Aggregate = min(domain scores) — WLNK
    let aggregate_score = domain_metrics
        .iter()
        .map(|m| m.health_score)
        .min()
        .unwrap_or(100);

    HealthReport {
        timestamp: Utc::now().to_rfc3339(),
        domain_metrics,
        aggregate_score,
        cycles: sccs,
        violations,
    }
}

// ── Snapshot persistence ─────────────────────────────────────────────────────

pub fn save_snapshot(s5d_dir: &Path, spec_id: &str, report: &HealthReport) -> anyhow::Result<()> {
    let snapshot = MetricsSnapshot {
        schema: "1.0".into(),
        timestamp: report.timestamp.clone(),
        spec_id: spec_id.into(),
        domain_metrics: report.domain_metrics.clone(),
        aggregate_score: report.aggregate_score,
    };
    let path = s5d_dir.join("records").join(format!("{spec_id}.health.yaml"));
    fs::write(&path, serde_yaml::to_string(&snapshot)?)?;
    Ok(())
}

pub fn load_snapshot(s5d_dir: &Path, spec_id: &str) -> anyhow::Result<Option<MetricsSnapshot>> {
    let path = s5d_dir.join("records").join(format!("{spec_id}.health.yaml"));
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)?;
    Ok(Some(serde_yaml::from_str(&content)?))
}

// ── Degradation detection ────────────────────────────────────────────────────

pub fn detect_degradation(
    current: &HealthReport,
    baseline: &MetricsSnapshot,
) -> DegradationReport {
    let baseline_map: HashMap<&str, &DomainMetrics> = baseline
        .domain_metrics
        .iter()
        .map(|m| (m.domain_id.as_str(), m))
        .collect();

    let current_map: HashMap<&str, &DomainMetrics> = current
        .domain_metrics
        .iter()
        .map(|m| (m.domain_id.as_str(), m))
        .collect();

    let mut domain_deltas = Vec::new();

    for cm in &current.domain_metrics {
        let score_before = baseline_map
            .get(cm.domain_id.as_str())
            .map(|b| b.health_score)
            .unwrap_or(100); // new domain assumed healthy baseline
        let delta = cm.health_score as i16 - score_before as i16;

        let baseline_violations: HashSet<&str> = baseline_map
            .get(cm.domain_id.as_str())
            .map(|b| b.violations.iter().map(|v| v.as_str()).collect())
            .unwrap_or_default();
        let current_violations: HashSet<&str> =
            cm.violations.iter().map(|v| v.as_str()).collect();

        let new_v: Vec<String> = current_violations
            .difference(&baseline_violations)
            .map(|s| s.to_string())
            .collect();
        let fixed_v: Vec<String> = baseline_violations
            .difference(&current_violations)
            .map(|s| s.to_string())
            .collect();

        let status = classify_delta(delta);

        domain_deltas.push(DomainDelta {
            domain_id: cm.domain_id.clone(),
            score_before,
            score_after: cm.health_score,
            delta,
            status,
            new_violations: new_v,
            fixed_violations: fixed_v,
        });
    }

    // Domains removed since baseline
    for bm in &baseline.domain_metrics {
        if !current_map.contains_key(bm.domain_id.as_str()) {
            domain_deltas.push(DomainDelta {
                domain_id: bm.domain_id.clone(),
                score_before: bm.health_score,
                score_after: 0,
                delta: -(bm.health_score as i16),
                status: "removed".into(),
                new_violations: Vec::new(),
                fixed_violations: Vec::new(),
            });
        }
    }

    let overall_delta = current.aggregate_score as i16 - baseline.aggregate_score as i16;
    let status = classify_delta(overall_delta);

    DegradationReport {
        current_score: current.aggregate_score,
        baseline_score: baseline.aggregate_score,
        delta: overall_delta,
        status,
        domain_deltas,
    }
}

fn classify_delta(delta: i16) -> String {
    if delta > 5 {
        "improved".into()
    } else if delta >= -5 {
        "stable".into()
    } else if delta >= -15 {
        "degraded".into()
    } else {
        "critical".into()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_spec(domains: Vec<(&str, &str)>, edges: Vec<(&str, &str)>) -> Spec {
        let domain_list: Vec<Domain> = domains
            .iter()
            .map(|(id, class)| Domain {
                id: id.to_string(),
                product: "test".into(),
                name: id.to_string(),
                classification: Some(class.to_string()),
                description: None,
                team: None,
                maturity_level: None,
            })
            .collect();

        let edge_list: Vec<Edge> = edges
            .iter()
            .map(|(from, to)| Edge {
                from: from.to_string(),
                to: to.to_string(),
                archetype: "customer_supplier".into(),
                ..Default::default()
            })
            .collect();

        Spec {
            s5d: "1.0".into(),
            id: "test".into(),
            version: "1.0.0".into(),
            product: "test".into(),
            tier: Tier::Standard,
            allow_update: false,
            meta: None,
            context: None,
            artifacts: Some(Artifacts {
                products: Vec::new(),
                domains: domain_list,
                capabilities: Vec::new(),
                entities: Vec::new(),
                features: Vec::new(),
                use_cases: Vec::new(),
                systems: Vec::new(),
                containers: Vec::new(),
                components: Vec::new(),
                roles: Vec::new(),
                concerns: Vec::new(),
                metrics: Vec::new(),
                supersystems: Vec::new(),
                transports: Vec::new(),
            }),
            links: Some(Links {
                edges: edge_list,
                ..Default::default()
            }),
            contracts: Vec::new(),
            gates: Vec::new(),
            roc: None,
            problem: None,
            hypotheses: Vec::new(),
            decision: None,
            note_rationale: None,
            expires_at: None,
            auto_noted: false,
        }
    }

    #[test]
    fn test_empty_spec_score_100() {
        let spec = make_spec(vec![("a", "core")], vec![]);
        let report = compute_health_report(&spec);
        assert_eq!(report.aggregate_score, 100);
        assert!(report.violations.is_empty());
    }

    #[test]
    fn test_coupling_metrics_simple_graph() {
        // A→B, A→C, B→C
        let spec = make_spec(
            vec![("a", "core"), ("b", "supporting"), ("c", "generic")],
            vec![("a", "b"), ("a", "c"), ("b", "c")],
        );
        let sccs = tarjan_scc(&spec);
        let metrics = compute_domain_metrics(&spec, &sccs);

        let a = metrics.iter().find(|m| m.domain_id == "a").unwrap();
        assert_eq!(a.ca, 0);
        assert_eq!(a.ce, 2);
        assert!((a.instability - 1.0).abs() < 0.001);

        let b = metrics.iter().find(|m| m.domain_id == "b").unwrap();
        assert_eq!(b.ca, 1);
        assert_eq!(b.ce, 1);
        assert!((b.instability - 0.5).abs() < 0.001);

        let c = metrics.iter().find(|m| m.domain_id == "c").unwrap();
        assert_eq!(c.ca, 2);
        assert_eq!(c.ce, 0);
        assert!((c.instability - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_cycle_detected_and_penalized() {
        let spec = make_spec(
            vec![("a", "core"), ("b", "core")],
            vec![("a", "b"), ("b", "a")],
        );
        let report = compute_health_report(&spec);
        assert!(report.aggregate_score < 100);
        assert!(!report.cycles.is_empty());
        assert!(report.violations.iter().any(|v| v.kind == "cycle"));
    }

    #[test]
    fn test_tarjan_finds_multiple_sccs() {
        // Two cycles: {a,b,c} and {d,e}
        let spec = make_spec(
            vec![
                ("a", "core"),
                ("b", "core"),
                ("c", "core"),
                ("d", "core"),
                ("e", "core"),
            ],
            vec![
                ("a", "b"),
                ("b", "c"),
                ("c", "a"),
                ("d", "e"),
                ("e", "d"),
            ],
        );
        let sccs = tarjan_scc(&spec);
        assert_eq!(sccs.len(), 2);
    }

    #[test]
    fn test_degradation_detection() {
        let baseline = MetricsSnapshot {
            schema: "1.0".into(),
            timestamp: "2026-03-01T00:00:00Z".into(),
            spec_id: "test".into(),
            domain_metrics: vec![DomainMetrics {
                domain_id: "a".into(),
                ca: 0,
                ce: 0,
                instability: 0.0,
                cycle_member: false,
                health_score: 90,
                violations: Vec::new(),
            }],
            aggregate_score: 90,
        };

        let current = HealthReport {
            timestamp: "2026-04-01T00:00:00Z".into(),
            domain_metrics: vec![DomainMetrics {
                domain_id: "a".into(),
                ca: 0,
                ce: 0,
                instability: 0.0,
                cycle_member: true,
                health_score: 72,
                violations: vec!["cycle_member".into()],
            }],
            aggregate_score: 72,
            cycles: vec![vec!["a".into(), "b".into()]],
            violations: Vec::new(),
        };

        let deg = detect_degradation(&current, &baseline);
        assert_eq!(deg.status, "critical"); // delta = -18
        assert_eq!(deg.domain_deltas[0].status, "critical");
        assert!(deg.domain_deltas[0].new_violations.contains(&"cycle_member".to_string()));
    }

    #[test]
    fn test_instability_bounds() {
        // Domain with Ce=0 → I=0.0
        let spec = make_spec(
            vec![("a", "core"), ("b", "core")],
            vec![("b", "a")],
        );
        let sccs = tarjan_scc(&spec);
        let metrics = compute_domain_metrics(&spec, &sccs);
        let a = metrics.iter().find(|m| m.domain_id == "a").unwrap();
        assert_eq!(a.ce, 0);
        assert!((a.instability - 0.0).abs() < 0.001);

        // Domain with Ca=0 → I=1.0
        let b = metrics.iter().find(|m| m.domain_id == "b").unwrap();
        assert_eq!(b.ca, 0);
        assert!((b.instability - 1.0).abs() < 0.001);
    }
}
