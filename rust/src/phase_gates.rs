use crate::models::{Record, Spec};
use colored::Colorize;

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Structural,
    Methodological,
}

#[derive(Debug, Clone)]
pub struct PhaseCheck {
    pub passed: bool,
    pub message: String,
    pub severity: Severity,
}

impl PhaseCheck {
    fn structural(passed: bool, message: impl Into<String>) -> Self {
        PhaseCheck {
            passed,
            message: message.into(),
            severity: Severity::Structural,
        }
    }
    fn methodological(passed: bool, message: impl Into<String>) -> Self {
        PhaseCheck {
            passed,
            message: message.into(),
            severity: Severity::Methodological,
        }
    }
}

pub fn check_decide(
    spec: &Spec,
    record: &Option<Record>,
    confirmed_by: &Option<String>,
) -> Vec<PhaseCheck> {
    let mut checks = Vec::new();

    // Structural: --confirmed-by required
    checks.push(PhaseCheck::structural(
        confirmed_by.as_ref().is_some_and(|s| !s.is_empty()),
        "--confirmed-by is required — human must confirm decisions",
    ));

    // Structural: decision not already recorded — check RECORD (source of truth), not spec
    let already_decided = record.as_ref().is_some_and(|r| r.decision.is_some());
    if already_decided {
        checks.push(PhaseCheck::structural(
            false,
            "decision already recorded in .record.yaml — cannot decide twice on same spec",
        ));
    }

    // Methodological: ≥3 hypotheses
    checks.push(PhaseCheck::methodological(
        spec.hypotheses.len() >= 3,
        format!(
            "≥3 hypotheses required for fair comparison ({} found)",
            spec.hypotheses.len()
        ),
    ));

    // Methodological: each surviving hypothesis has evidence
    for hyp in &spec.hypotheses {
        if hyp.evidence.is_empty() {
            checks.push(PhaseCheck::methodological(
                false,
                format!(
                    "hypothesis '{}' has no evidence — decomposition required before deciding",
                    hyp.id
                ),
            ));
        }
    }

    // Methodological: problem card has acceptance criteria
    let has_acceptance = spec.problem.as_ref().is_some_and(|p| {
        p.as_card()
            .is_some_and(|c| c.acceptance.as_ref().is_some_and(|a| !a.is_empty()))
    });
    checks.push(PhaseCheck::methodological(
        has_acceptance,
        "problem card should have acceptance criteria defined before deciding",
    ));

    checks
}

/// Validate that adversarial challenge has been provided.
/// Returns None if OK, Some(message) if challenge is missing/insufficient.
pub fn check_challenge(
    challenge: &Option<crate::Challenge>,
    tier: &crate::Tier,
    force: bool,
    no_challenge: bool,
) -> Option<String> {
    if no_challenge || force {
        return None;
    }
    if challenge.is_none() {
        return Some(
            "adversarial challenge required — use --challenge-summary, --no-challenge, or --force"
                .to_string(),
        );
    }
    let ch = challenge.as_ref().unwrap();
    if matches!(tier, crate::Tier::Standard | crate::Tier::High) && ch.mode == "tactical" {
        return Some(
            "standard/high tier requires standard challenge mode (5 probes), not tactical (1 probe)"
                .to_string(),
        );
    }
    None
}

pub fn check_add_hypothesis(spec: &Spec, new_id: &str) -> Vec<PhaseCheck> {
    let mut checks = Vec::new();

    // Structural: no duplicate IDs
    let duplicate = spec.hypotheses.iter().any(|h| h.id == new_id);
    checks.push(PhaseCheck::structural(
        !duplicate,
        format!("hypothesis '{}' already exists", new_id),
    ));

    checks
}

pub fn check_approve(record: &Option<Record>) -> Vec<PhaseCheck> {
    let mut checks = Vec::new();

    // Structural: preview MUST exist before approval — SHA256 chain depends on it
    let has_preview = record.as_ref().is_some_and(|r| r.preview.is_some());
    checks.push(PhaseCheck::structural(
        has_preview,
        "preview required before approval — SHA256 chain integrity depends on preview. Run `s5d state preview` first",
    ));

    checks
}

pub fn check_import(record: &Option<Record>, verified_by: &str) -> Vec<PhaseCheck> {
    let mut checks = Vec::new();
    let verified_by = verified_by.trim();

    // Structural: must have approval
    let has_approval = record.as_ref().is_some_and(|r| !r.approvals.is_empty());
    checks.push(PhaseCheck::structural(
        has_approval,
        "spec must be approved before import — run `s5d state approve` first",
    ));

    checks.push(PhaseCheck::structural(
        !verified_by.is_empty(),
        "import verifier required — pass `--verified-by <name>`",
    ));

    // Methodological: verifier ≠ approver (trust separation)
    if !verified_by.is_empty() {
        if let Some(rec) = record {
            if let Some(last_approval) = rec.approvals.last() {
                checks.push(PhaseCheck::methodological(
                    verified_by != last_approval.reviewer,
                    "verifier should differ from approver for trust separation — use --force to override",
                ));
            }
        }
    }

    checks
}

/// Process phase checks: print results, return whether to proceed.
/// Returns Ok(true) to proceed, Err for structural or unforced methodological failures.
pub fn enforce_checks(checks: &[PhaseCheck], force: bool) -> anyhow::Result<bool> {
    let mut structural_failures: Vec<&str> = Vec::new();
    let mut methodological_failures: Vec<&str> = Vec::new();

    for check in checks {
        if check.passed {
            continue;
        }
        match check.severity {
            Severity::Structural => {
                eprintln!("  {} {}", "error:".red(), check.message);
                structural_failures.push(check.message.as_str());
            }
            Severity::Methodological => {
                if force {
                    eprintln!(
                        "  {} {} (overridden with --force)",
                        "warn:".yellow(),
                        check.message
                    );
                } else {
                    eprintln!("  {} {}", "warn:".yellow(), check.message);
                    methodological_failures.push(check.message.as_str());
                }
            }
        }
    }

    // Include the specific failed checks in the error so callers that don't see
    // stderr (e.g. the MCP server) get an actionable message, not a bare verdict.
    if !structural_failures.is_empty() {
        anyhow::bail!(
            "structural prerequisite failed — cannot proceed:\n  - {}",
            structural_failures.join("\n  - ")
        );
    }

    if !methodological_failures.is_empty() && !force {
        eprintln!("  use --force to override methodological checks");
        anyhow::bail!(
            "methodological prerequisites not met (use --force to override):\n  - {}",
            methodological_failures.join("\n  - ")
        );
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;

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
            workflow: None,
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
            intent_kernel: None,
        }
    }

    fn make_hypothesis(id: &str) -> Hypothesis {
        Hypothesis {
            id: id.into(),
            title: "Option A".into(),
            content: "Description".into(),
            scope: "infrastructure".into(),
            kind: "solution".into(),
            layer: "data".into(),
            r_eff: None,
            evidence: vec![],
            depends_on: vec![],
            rationale: None,
            spec_ref: None,
            prompt: None,
            next_move: None,
        }
    }

    fn make_hypothesis_with_evidence(id: &str) -> Hypothesis {
        let mut h = make_hypothesis(id);
        h.evidence = vec![HypothesisEvidence {
            id: "ev1".into(),
            evidence_type: "analysis".into(),
            content: "Some evidence".into(),
            verdict: "supports".into(),
            valid_until: None,
            carrier_ref: None,
            formality: None,
            claim_scope: vec![],
            congruence_level: None,
            reliability: None,
            refine_kind: None,
            skill: None,
            agent: None,
        }];
        h
    }

    #[test]
    fn test_check_decide_no_confirmed_by() {
        let spec = make_spec("test");
        let checks = check_decide(&spec, &None, &None);
        // First check: confirmed_by is required
        assert!(!checks[0].passed);
        assert_eq!(checks[0].severity, Severity::Structural);
    }

    #[test]
    fn test_check_decide_already_decided() {
        let spec = make_spec("test");
        // Decision lives in RECORD, not spec — this is the source of truth fix
        let mut record = Record {
            spec_ref: "test".into(),
            spec_sha256: "".into(),
            status: crate::models::SpecStatus::Proposed,
            sync_status: crate::models::SyncStatus::Unknown,
            status_history: vec![],
            active_phase: None,
            phase_history: vec![],
            phase_runs: vec![],
            approvals: vec![],
            preview: None,
            reflection: None,
            gate_results: vec![],
            decision: None,
            verified_by: None,
            drift_tolerance: None,
        };
        record.decision = Some(DecisionRecord {
            title: "Decision".into(),
            winner_id: "h1".into(),
            rejected_ids: vec![],
            context: "".into(),
            decision: "".into(),
            rationale: "".into(),
            consequences: "".into(),
            decided_at: None,
            confirmed_by: None,
            expires_at: None,
            do_list: vec![],
            dont_list: vec![],
            challenge: None,
            decision_subject: None,
            decision_subject_granularity: None,
            evaluative_surface: None,
            belief_state: None,
            outcome_model: None,
            pareto_set: vec![],
            choice_rule: None,
        });

        let checks = check_decide(&spec, &Some(record), &Some("roman".into()));
        let already_decided = checks
            .iter()
            .find(|c| c.message.contains("already recorded"));
        assert!(already_decided.is_some());
        assert!(!already_decided.unwrap().passed);
        assert_eq!(already_decided.unwrap().severity, Severity::Structural);
    }

    #[test]
    fn test_check_decide_few_hypotheses() {
        let mut spec = make_spec("test");
        spec.hypotheses = vec![make_hypothesis("h1"), make_hypothesis("h2")];

        let checks = check_decide(&spec, &None, &Some("roman".into()));
        let hypothesis_check = checks.iter().find(|c| c.message.contains("≥3 hypotheses"));
        assert!(hypothesis_check.is_some());
        assert!(!hypothesis_check.unwrap().passed);
        assert_eq!(hypothesis_check.unwrap().severity, Severity::Methodological);
    }

    #[test]
    fn test_check_decide_full_pass() {
        let mut spec = make_spec("test");
        spec.hypotheses = vec![
            make_hypothesis_with_evidence("h1"),
            make_hypothesis_with_evidence("h2"),
            make_hypothesis_with_evidence("h3"),
        ];
        spec.problem = Some(ProblemField::Card(ProblemCard {
            signal: "Something is wrong".into(),
            constraints: vec![],
            targets: vec![],
            indicators: vec![],
            acceptance: Some("Fix it".into()),
            blast_radius: None,
            reversibility: None,
            status: None,
            goodhart_guard: None,
        }));

        let checks = check_decide(&spec, &None, &Some("roman".into()));
        let all_pass = checks.iter().all(|c| c.passed);
        assert!(
            all_pass,
            "expected all checks to pass, failures: {:?}",
            checks
                .iter()
                .filter(|c| !c.passed)
                .map(|c| &c.message)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_check_add_hypothesis_duplicate() {
        let mut spec = make_spec("test");
        spec.hypotheses = vec![make_hypothesis("h1")];

        let checks = check_add_hypothesis(&spec, "h1");
        assert!(!checks[0].passed);
        assert_eq!(checks[0].severity, Severity::Structural);
    }

    #[test]
    fn test_check_add_hypothesis_unique() {
        let mut spec = make_spec("test");
        spec.hypotheses = vec![make_hypothesis("h1")];

        let checks = check_add_hypothesis(&spec, "h2");
        assert!(checks[0].passed);
    }

    #[test]
    fn test_enforce_checks_structural_fails() {
        let checks = vec![PhaseCheck {
            passed: false,
            message: "structural failure".into(),
            severity: Severity::Structural,
        }];
        // force=true still fails for Structural
        let result = enforce_checks(&checks, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_enforce_checks_methodological_force() {
        let checks = vec![PhaseCheck {
            passed: false,
            message: "methodological warning".into(),
            severity: Severity::Methodological,
        }];
        // force=true → Ok
        let result = enforce_checks(&checks, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_enforce_checks_methodological_no_force() {
        let checks = vec![PhaseCheck {
            passed: false,
            message: "methodological warning".into(),
            severity: Severity::Methodological,
        }];
        // force=false → Err
        let result = enforce_checks(&checks, false);
        assert!(result.is_err());
    }
}
