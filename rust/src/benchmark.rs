use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillBenchmarkSuite {
    pub benchmark: String,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub scale: Option<String>,
    #[serde(default)]
    pub required_tags: Vec<String>,
    #[serde(default)]
    pub artifacts: Vec<BenchmarkArtifact>,
    pub criteria: Vec<BenchmarkCriterion>,
    pub cases: Vec<BenchmarkCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkCriterion {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    pub weight: f64,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkArtifact {
    pub kind: String,
    pub path: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkCase {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub expected_best_shape: Option<String>,
    #[serde(default)]
    pub artifacts: Vec<BenchmarkArtifact>,
    pub runs: Vec<AssistantRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantRun {
    /// Stable variant id, for example "native" or "skill".
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    /// Criterion id -> score in the 0.0..1.0 range.
    #[serde(default)]
    pub scores: BTreeMap<String, f64>,
    /// Optional raw measurements such as turns, tokens, elapsed_s.
    #[serde(default)]
    pub metrics: BTreeMap<String, f64>,
    #[serde(default)]
    pub artifacts: Vec<BenchmarkArtifact>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillBenchmarkReport {
    pub benchmark: String,
    pub date: Option<String>,
    pub scale: Option<String>,
    pub required_tags: Vec<String>,
    pub covered_tags: Vec<String>,
    pub criteria: Vec<BenchmarkCriterion>,
    pub run_order: Vec<String>,
    pub cases: Vec<BenchmarkCaseResult>,
    pub means: Vec<BenchmarkMean>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkCaseResult {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    pub scores: Vec<BenchmarkRunScore>,
    pub winner: String,
    pub skill_minus_native: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkRunScore {
    pub id: String,
    pub label: Option<String>,
    /// Weighted percentage, 0.0..100.0.
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMean {
    pub id: String,
    pub label: Option<String>,
    pub cases: usize,
    /// Mean weighted percentage, 0.0..100.0.
    pub score: f64,
}

pub fn load_skill_benchmark(path: &Path) -> anyhow::Result<SkillBenchmarkSuite> {
    let content = std::fs::read_to_string(path)
        .map_err(|err| anyhow::anyhow!("failed to read benchmark {}: {}", path.display(), err))?;
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("yaml") | Some("yml") => serde_yaml::from_str(&content)
            .map_err(|err| anyhow::anyhow!("failed to parse benchmark YAML: {}", err)),
        _ => serde_json::from_str(&content)
            .map_err(|err| anyhow::anyhow!("failed to parse benchmark JSON: {}", err)),
    }
}

pub fn score_skill_benchmark(suite: &SkillBenchmarkSuite) -> anyhow::Result<SkillBenchmarkReport> {
    validate_suite(suite)?;
    let criteria_by_id: BTreeMap<&str, &BenchmarkCriterion> = suite
        .criteria
        .iter()
        .map(|criterion| (criterion.id.as_str(), criterion))
        .collect();

    let mut run_order = Vec::new();
    let mut totals: BTreeMap<String, (Option<String>, usize, f64)> = BTreeMap::new();
    let mut covered_tags = BTreeSet::new();
    let mut results = Vec::new();

    for case in &suite.cases {
        for tag in &case.tags {
            covered_tags.insert(tag.clone());
        }
        let mut scores = Vec::new();
        for run in &case.runs {
            if !run_order.contains(&run.id) {
                run_order.push(run.id.clone());
            }
            let score = score_run(run, &criteria_by_id)?;
            let entry = totals
                .entry(run.id.clone())
                .or_insert_with(|| (run.label.clone(), 0, 0.0));
            entry.1 += 1;
            entry.2 += score;
            scores.push(BenchmarkRunScore {
                id: run.id.clone(),
                label: run.label.clone(),
                score,
            });
        }

        let winner = winner_for_case(&scores);
        let skill_minus_native = score_by_id(&scores, "skill")
            .zip(score_by_id(&scores, "native"))
            .map(|(skill, native)| round1(skill - native));

        results.push(BenchmarkCaseResult {
            id: case.id.clone(),
            name: case.name.clone(),
            tags: case.tags.clone(),
            scores,
            winner,
            skill_minus_native,
        });
    }

    let means = run_order
        .iter()
        .filter_map(|id| {
            totals.get(id).map(|(label, count, total)| BenchmarkMean {
                id: id.clone(),
                label: label.clone(),
                cases: *count,
                score: round1(total / *count as f64),
            })
        })
        .collect();

    Ok(SkillBenchmarkReport {
        benchmark: suite.benchmark.clone(),
        date: suite.date.clone(),
        scale: suite.scale.clone(),
        required_tags: suite.required_tags.clone(),
        covered_tags: covered_tags.into_iter().collect(),
        criteria: suite.criteria.clone(),
        run_order,
        cases: results,
        means,
    })
}

pub fn format_benchmark_markdown(report: &SkillBenchmarkReport) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", report.benchmark));
    if let Some(date) = &report.date {
        out.push_str(&format!("Date: `{}`\n\n", date));
    }
    if let Some(scale) = &report.scale {
        out.push_str(&format!("Scale: {}\n\n", scale));
    }
    if !report.required_tags.is_empty() {
        out.push_str(&format!(
            "Required scenario tags: `{}`\n\n",
            report.required_tags.join("`, `")
        ));
    }

    out.push_str("## Criteria\n\n");
    out.push_str("| Criterion | Weight |\n|---|---:|\n");
    for criterion in &report.criteria {
        let label = criterion.label.as_deref().unwrap_or(&criterion.id);
        out.push_str(&format!("| {} | {:.1} |\n", label, criterion.weight));
    }

    out.push_str("\n## Case Results\n\n");
    let include_tags = report.cases.iter().any(|case| !case.tags.is_empty());
    out.push_str("| Case |");
    if include_tags {
        out.push_str(" Tags |");
    }
    for run_id in &report.run_order {
        out.push_str(&format!(" {} |", run_id));
    }
    out.push_str(" Skill - native | Winner |\n");
    out.push_str("|---|");
    if include_tags {
        out.push_str("---|");
    }
    for _ in &report.run_order {
        out.push_str("---:|");
    }
    out.push_str("---:|---|\n");

    for case in &report.cases {
        out.push_str(&format!("| {} |", case.id));
        if include_tags {
            out.push_str(&format!(" {} |", case.tags.join(", ")));
        }
        for run_id in &report.run_order {
            match score_by_id(&case.scores, run_id) {
                Some(score) => out.push_str(&format!(" {:.1} |", score)),
                None => out.push_str(" - |"),
            }
        }
        match case.skill_minus_native {
            Some(delta) => out.push_str(&format!(" {:+.1} |", delta)),
            None => out.push_str(" - |"),
        }
        out.push_str(&format!(" {} |\n", case.winner));
    }

    out.push_str("\n## Mean Scores\n\n");
    out.push_str("| Run | Cases | Mean |\n|---|---:|---:|\n");
    for mean in &report.means {
        out.push_str(&format!(
            "| {} | {} | {:.1} |\n",
            mean.id, mean.cases, mean.score
        ));
    }

    out
}

fn validate_suite(suite: &SkillBenchmarkSuite) -> anyhow::Result<()> {
    if suite.benchmark.trim().is_empty() {
        anyhow::bail!("benchmark name must not be empty");
    }
    if suite.criteria.is_empty() {
        anyhow::bail!("benchmark must define at least one criterion");
    }
    if suite.cases.is_empty() {
        anyhow::bail!("benchmark must define at least one case");
    }
    validate_artifacts("benchmark", &suite.artifacts)?;

    let mut seen_criteria = BTreeSet::new();
    for criterion in &suite.criteria {
        if criterion.id.trim().is_empty() {
            anyhow::bail!("criterion id must not be empty");
        }
        if !seen_criteria.insert(&criterion.id) {
            anyhow::bail!("duplicate criterion id: {}", criterion.id);
        }
        if !criterion.weight.is_finite() || criterion.weight <= 0.0 {
            anyhow::bail!(
                "criterion {} must have a positive finite weight",
                criterion.id
            );
        }
    }

    let mut required_tags = BTreeSet::new();
    for tag in &suite.required_tags {
        validate_tag("required scenario tag", tag)?;
        if !required_tags.insert(tag.clone()) {
            anyhow::bail!("duplicate required scenario tag: {}", tag);
        }
    }

    let mut covered_tags = BTreeSet::new();
    for case in &suite.cases {
        if case.id.trim().is_empty() {
            anyhow::bail!("case id must not be empty");
        }
        if case.runs.is_empty() {
            anyhow::bail!("case {} must define at least one run", case.id);
        }
        validate_artifacts(&format!("case {}", case.id), &case.artifacts)?;
        let mut seen_case_tags = BTreeSet::new();
        for tag in &case.tags {
            validate_tag(&format!("case {} scenario tag", case.id), tag)?;
            if !seen_case_tags.insert(tag.clone()) {
                anyhow::bail!("case {} has duplicate scenario tag {}", case.id, tag);
            }
            covered_tags.insert(tag.clone());
        }
        let mut seen_runs = BTreeSet::new();
        for run in &case.runs {
            if run.id.trim().is_empty() {
                anyhow::bail!("case {} has a run with an empty id", case.id);
            }
            if !seen_runs.insert(&run.id) {
                anyhow::bail!("case {} has duplicate run id {}", case.id, run.id);
            }
            validate_artifacts(&format!("case {} run {}", case.id, run.id), &run.artifacts)?;
            for criterion in &suite.criteria {
                let value = run.scores.get(&criterion.id).ok_or_else(|| {
                    anyhow::anyhow!(
                        "case {} run {} missing score for criterion {}",
                        case.id,
                        run.id,
                        criterion.id
                    )
                })?;
                if !value.is_finite() || !(0.0..=1.0).contains(value) {
                    anyhow::bail!(
                        "case {} run {} criterion {} score must be in 0.0..1.0",
                        case.id,
                        run.id,
                        criterion.id
                    );
                }
            }
        }
    }
    for tag in required_tags {
        if !covered_tags.contains(&tag) {
            anyhow::bail!("benchmark missing required scenario tag: {}", tag);
        }
    }

    Ok(())
}

fn validate_artifacts(scope: &str, artifacts: &[BenchmarkArtifact]) -> anyhow::Result<()> {
    for artifact in artifacts {
        if artifact.kind.trim().is_empty() {
            anyhow::bail!("{} artifact kind must not be empty", scope);
        }
        if artifact.path.trim().is_empty() {
            anyhow::bail!("{} artifact path must not be empty", scope);
        }
    }
    Ok(())
}

fn validate_tag(scope: &str, tag: &str) -> anyhow::Result<()> {
    if tag.trim().is_empty() {
        anyhow::bail!("{} must not be empty", scope);
    }
    if tag.trim() != tag {
        anyhow::bail!("{} must be trimmed: {}", scope, tag);
    }
    Ok(())
}

fn score_run(
    run: &AssistantRun,
    criteria_by_id: &BTreeMap<&str, &BenchmarkCriterion>,
) -> anyhow::Result<f64> {
    let total_weight: f64 = criteria_by_id
        .values()
        .map(|criterion| criterion.weight)
        .sum();
    let weighted: f64 = criteria_by_id
        .values()
        .map(|criterion| {
            let score = run
                .scores
                .get(&criterion.id)
                .expect("validate_suite ensures score presence");
            score * criterion.weight
        })
        .sum();
    Ok(round1((weighted / total_weight) * 100.0))
}

fn winner_for_case(scores: &[BenchmarkRunScore]) -> String {
    let Some(best) = scores
        .iter()
        .max_by(|left, right| left.score.total_cmp(&right.score))
    else {
        return "none".into();
    };
    let ties = scores
        .iter()
        .filter(|score| (score.score - best.score).abs() < f64::EPSILON)
        .count();
    if ties > 1 {
        "tie".into()
    } else {
        best.id.clone()
    }
}

fn score_by_id(scores: &[BenchmarkRunScore], id: &str) -> Option<f64> {
    scores
        .iter()
        .find(|score| score.id == id)
        .map(|score| score.score)
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_suite() -> SkillBenchmarkSuite {
        serde_json::from_str(
            r#"{
              "benchmark": "skill vs native",
              "scale": "scores are 0.0..1.0",
              "required_tags": ["scope-drift"],
              "criteria": [
                {"id":"correctness","weight":4},
                {"id":"evidence","weight":3},
                {"id":"scope","weight":2}
              ],
              "cases": [
                {
                  "id":"T01",
                  "name":"Review a spec-bound diff",
                  "tags":["scope-drift"],
                  "runs":[
                    {"id":"native","scores":{"correctness":0.6,"evidence":0.3,"scope":0.7}},
                    {"id":"skill","scores":{"correctness":0.9,"evidence":1.0,"scope":0.9}}
                  ]
                }
              ]
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn scores_weighted_skill_vs_native_delta() {
        let report = score_skill_benchmark(&sample_suite()).unwrap();
        let case = &report.cases[0];
        assert_eq!(case.winner, "skill");
        assert_eq!(case.skill_minus_native, Some(41.1));
        assert_eq!(report.means[0].id, "native");
        assert_eq!(report.means[0].score, 52.2);
        assert_eq!(report.means[1].id, "skill");
        assert_eq!(report.means[1].score, 93.3);
    }

    #[test]
    fn rejects_missing_criterion_scores() {
        let mut suite = sample_suite();
        suite.cases[0].runs[0].scores.remove("scope");
        let err = score_skill_benchmark(&suite).unwrap_err().to_string();
        assert!(err.contains("missing score for criterion scope"), "{err}");
    }

    #[test]
    fn rejects_missing_required_scenario_tags() {
        let mut suite = sample_suite();
        suite.required_tags = vec!["failure-handling".into()];
        let err = score_skill_benchmark(&suite).unwrap_err().to_string();
        assert!(
            err.contains("missing required scenario tag: failure-handling"),
            "{err}"
        );
    }

    #[test]
    fn renders_markdown_report() {
        let report = score_skill_benchmark(&sample_suite()).unwrap();
        let markdown = format_benchmark_markdown(&report);
        assert!(markdown.contains("Required scenario tags: `scope-drift`"));
        assert!(markdown.contains("| T01 | scope-drift | 52.2 | 93.3 | +41.1 | skill |"));
        assert!(markdown.contains("| skill | 1 | 93.3 |"));
    }
}
