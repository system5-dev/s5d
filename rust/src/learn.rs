use crate::models::*;
use crate::project::S5dProject;

#[derive(Debug, Clone)]
pub struct LearnReport {
    pub operated_count: usize,
    pub heuristics: Vec<HeuristicEntry>,
    pub recurring_issues: Vec<(String, usize)>,
    pub open_follow_ups: Vec<FollowUpEntry>,
}

#[derive(Debug, Clone)]
pub struct HeuristicEntry {
    pub text: String,
    pub source_spec: String,
}

#[derive(Debug, Clone)]
pub struct FollowUpEntry {
    pub id: String,
    pub priority: Option<String>,
    pub description: Option<String>,
    pub source_spec: String,
}

/// Aggregate reflections from all operated specs.
pub fn aggregate_reflections(project: &S5dProject) -> anyhow::Result<LearnReport> {
    let specs = project.discover_specs()?;
    let mut operated_count = 0usize;
    let mut heuristics = Vec::new();
    let mut issue_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut follow_ups = Vec::new();

    for (path, spec) in &specs {
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        let record = match project.load_record(&filename)? {
            Some(r) => r,
            None => continue,
        };
        if record.status != SpecStatus::Operated {
            continue;
        }
        operated_count += 1;

        if let Some(ref reflection) = record.reflection {
            for h in &reflection.heuristics {
                heuristics.push(HeuristicEntry {
                    text: h.clone(),
                    source_spec: spec.id.clone(),
                });
            }
            for issue in &reflection.issues {
                *issue_counts.entry(issue.clone()).or_default() += 1;
            }
            for fu in &reflection.follow_ups {
                follow_ups.push(FollowUpEntry {
                    id: fu.id.clone(),
                    priority: fu.priority.clone(),
                    description: fu.description.clone(),
                    source_spec: spec.id.clone(),
                });
            }
        }
    }

    // Sort recurring issues by count descending
    let mut recurring_issues: Vec<(String, usize)> = issue_counts.into_iter().collect();
    recurring_issues.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(LearnReport {
        operated_count,
        heuristics,
        recurring_issues,
        open_follow_ups: follow_ups,
    })
}

/// Feed relevant heuristics for a new spec based on shared domains.
pub fn feed_heuristics(project: &S5dProject, spec: &Spec) -> anyhow::Result<Vec<HeuristicEntry>> {
    // Extract domain IDs from the target spec
    let target_domains: std::collections::HashSet<String> = spec
        .artifacts
        .as_ref()
        .map(|a| a.domains.iter().map(|d| d.id.clone()).collect())
        .unwrap_or_default();

    if target_domains.is_empty() {
        // No domains → return ALL heuristics (spec might be early stage)
        let report = aggregate_reflections(project)?;
        return Ok(report.heuristics);
    }

    let specs = project.discover_specs()?;
    let mut result = Vec::new();

    for (path, past_spec) in &specs {
        // Skip self
        if past_spec.id == spec.id {
            continue;
        }

        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        let record = match project.load_record(&filename)? {
            Some(r) => r,
            None => continue,
        };
        if record.status != SpecStatus::Operated {
            continue;
        }

        // Check domain overlap
        let past_domains: std::collections::HashSet<String> = past_spec
            .artifacts
            .as_ref()
            .map(|a| a.domains.iter().map(|d| d.id.clone()).collect())
            .unwrap_or_default();

        let has_overlap = target_domains.iter().any(|d| past_domains.contains(d));
        if !has_overlap && !past_domains.is_empty() {
            continue;
        }

        // Collect heuristics from this past spec
        if let Some(ref reflection) = record.reflection {
            for h in &reflection.heuristics {
                result.push(HeuristicEntry {
                    text: h.clone(),
                    source_spec: past_spec.id.clone(),
                });
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_project(dir: &std::path::Path) -> S5dProject {
        let (project, _) = S5dProject::init(dir).unwrap();
        project
    }

    fn write_spec(project: &S5dProject, filename: &str, spec: &Spec) {
        let path = project.s5d_dir().join("packages").join(filename);
        let yaml = serde_yaml::to_string(spec).unwrap();
        std::fs::write(path, yaml).unwrap();
    }

    fn write_record(project: &S5dProject, spec_filename: &str, record: &Record) {
        project.save_record(spec_filename, record).unwrap();
    }

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

    fn make_operated_record(
        heuristics: Vec<String>,
        issues: Vec<String>,
        follow_ups: Vec<FollowUp>,
    ) -> Record {
        Record {
            spec_ref: "test".into(),
            spec_sha256: "sha256:abc".into(),
            status: SpecStatus::Operated,
            sync_status: SyncStatus::default(),
            status_history: vec![],
            approvals: vec![],
            preview: None,
            reflection: Some(Reflection {
                summary: Some("test summary".into()),
                worked: vec![],
                issues,
                structured_issues: vec![],
                follow_ups,
                evidence: vec![],
                heuristics,
            }),
            gate_results: vec![],
            decision: None,
            verified_by: None,
        }
    }

    #[test]
    fn test_aggregate_empty() {
        let dir = TempDir::new().unwrap();
        let project = setup_project(dir.path());

        let report = aggregate_reflections(&project).unwrap();
        assert_eq!(report.operated_count, 0);
        assert!(report.heuristics.is_empty());
        assert!(report.recurring_issues.is_empty());
        assert!(report.open_follow_ups.is_empty());
    }

    #[test]
    fn test_aggregate_with_reflections() {
        let dir = TempDir::new().unwrap();
        let project = setup_project(dir.path());

        // Spec 1: operated with heuristics and issues
        let spec1 = make_spec("spec-one");
        write_spec(&project, "spec-one.s5d.yaml", &spec1);
        write_record(
            &project,
            "spec-one.s5d.yaml",
            &make_operated_record(
                vec!["always test billing".into(), "check Redis failover".into()],
                vec!["flaky CI".into(), "slow deploys".into()],
                vec![FollowUp {
                    id: "migrate-redis".into(),
                    priority: Some("high".into()),
                    description: None,
                }],
            ),
        );

        // Spec 2: operated with overlapping issue
        let spec2 = make_spec("spec-two");
        write_spec(&project, "spec-two.s5d.yaml", &spec2);
        write_record(
            &project,
            "spec-two.s5d.yaml",
            &make_operated_record(
                vec!["monitor memory after deploy".into()],
                vec!["flaky CI".into()],
                vec![],
            ),
        );

        let report = aggregate_reflections(&project).unwrap();
        assert_eq!(report.operated_count, 2);
        assert_eq!(report.heuristics.len(), 3);
        assert_eq!(report.open_follow_ups.len(), 1);
        assert_eq!(report.open_follow_ups[0].id, "migrate-redis");

        // "flaky CI" should appear with count 2
        let flaky = report
            .recurring_issues
            .iter()
            .find(|(text, _)| text == "flaky CI");
        assert!(flaky.is_some());
        assert_eq!(flaky.unwrap().1, 2);
    }

    #[test]
    fn test_feed_matching_domains() {
        let dir = TempDir::new().unwrap();
        let project = setup_project(dir.path());

        // Past spec: operated, domain "billing", has heuristics
        let mut past_spec = make_spec("past-billing");
        past_spec.artifacts = Some(Artifacts {
            domains: vec![Domain {
                id: "billing".into(),
                product: "P".into(),
                name: "Billing".into(),
                classification: None,
                description: None,
                team: None,
                maturity_level: None,
            }],
            ..Default::default()
        });
        write_spec(&project, "past-billing.s5d.yaml", &past_spec);
        write_record(
            &project,
            "past-billing.s5d.yaml",
            &make_operated_record(
                vec!["always validate invoice totals".into()],
                vec![],
                vec![],
            ),
        );

        // Past spec: operated, domain "auth", has heuristics
        let mut past_auth = make_spec("past-auth");
        past_auth.artifacts = Some(Artifacts {
            domains: vec![Domain {
                id: "auth".into(),
                product: "P".into(),
                name: "Auth".into(),
                classification: None,
                description: None,
                team: None,
                maturity_level: None,
            }],
            ..Default::default()
        });
        write_spec(&project, "past-auth.s5d.yaml", &past_auth);
        write_record(
            &project,
            "past-auth.s5d.yaml",
            &make_operated_record(vec!["test token expiry edge cases".into()], vec![], vec![]),
        );

        // New spec: billing domain
        let mut new_spec = make_spec("new-billing-feature");
        new_spec.artifacts = Some(Artifacts {
            domains: vec![Domain {
                id: "billing".into(),
                product: "P".into(),
                name: "Billing".into(),
                classification: None,
                description: None,
                team: None,
                maturity_level: None,
            }],
            ..Default::default()
        });

        let feed = feed_heuristics(&project, &new_spec).unwrap();
        assert_eq!(feed.len(), 1);
        assert_eq!(feed[0].text, "always validate invoice totals");
        assert_eq!(feed[0].source_spec, "past-billing");
    }

    #[test]
    fn test_feed_no_match() {
        let dir = TempDir::new().unwrap();
        let project = setup_project(dir.path());

        // Past spec: billing domain
        let mut past_spec = make_spec("past-billing");
        past_spec.artifacts = Some(Artifacts {
            domains: vec![Domain {
                id: "billing".into(),
                product: "P".into(),
                name: "Billing".into(),
                classification: None,
                description: None,
                team: None,
                maturity_level: None,
            }],
            ..Default::default()
        });
        write_spec(&project, "past-billing.s5d.yaml", &past_spec);
        write_record(
            &project,
            "past-billing.s5d.yaml",
            &make_operated_record(vec!["billing heuristic".into()], vec![], vec![]),
        );

        // New spec: auth domain (no overlap)
        let mut new_spec = make_spec("new-auth");
        new_spec.artifacts = Some(Artifacts {
            domains: vec![Domain {
                id: "auth".into(),
                product: "P".into(),
                name: "Auth".into(),
                classification: None,
                description: None,
                team: None,
                maturity_level: None,
            }],
            ..Default::default()
        });

        let feed = feed_heuristics(&project, &new_spec).unwrap();
        assert!(feed.is_empty());
    }
}
