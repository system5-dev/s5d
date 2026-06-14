use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn canonical_skill_benchmark_covers_required_scenario_families() {
    let suite_path = repo_root().join("examples/skill-benchmark.json");
    let suite = s5d::load_skill_benchmark(&suite_path).unwrap();
    let report = s5d::score_skill_benchmark(&suite).unwrap();

    assert_eq!(
        report.required_tags,
        vec![
            "happy-path",
            "edge-case",
            "failure-handling",
            "scope-drift",
            "stale-intent"
        ]
    );
    for tag in &report.required_tags {
        assert!(
            report.covered_tags.contains(tag),
            "canonical benchmark must cover required tag {tag}"
        );
    }
    assert_eq!(report.cases.len(), 5);
    assert_eq!(report.means.len(), 2);
}
