use chrono::Utc;

use crate::models::*;

pub fn generate_spec(id: &str, tier: Tier, product: &str) -> Spec {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let links = if matches!(tier, Tier::Lightweight) {
        None
    } else {
        Some(Links::default())
    };
    let gates = crate::gates::default_gates_for_tier(&tier);
    Spec {
        s5d: "1.0".into(),
        id: id.into(),
        version: "1.0.0".into(),
        product: product.into(),
        tier,
        allow_update: true, // drafts start editable; metamodel enforcement skipped until finalized
        meta: Some(Meta {
            title: id.rsplit('.').next().unwrap_or(id).into(),
            authors: vec![],
            date: Some(today),
            tickets: vec![],
            adrs: vec![],
            renames: vec![],
        }),
        context: Some("## Problem\n\n## Goal\n\n## Privacy\nClassification: Internal\n".into()),
        workflow: Some(Workflow {
            mode: Some("plan".into()),
            target_architecture: Some(TargetArchitecture {
                summary: "Describe the steady-state design this change should reach".into(),
                invariants: vec![],
            }),
            delivery_strategy: Some(DeliveryStrategy {
                summary:
                    "Describe how this change moves current code toward the target architecture"
                        .into(),
                rationale: None,
            }),
            resources: None,
            role_map: std::collections::HashMap::from([
                ("owner".into(), "human".into()),
                ("implementer".into(), "coder".into()),
                ("reviewer".into(), "reviewer".into()),
                ("verifier".into(), "reviewer".into()),
            ]),
            review_policy: Some(ReviewPolicy {
                cross_model_required: false,
                required_on: vec![],
            }),
            structure_outline: None,
            execution_mode: Some(ExecutionMode {
                engine: "manual".into(),
                max_iterations: None,
                stop_conditions: vec![
                    "Phase acceptance reached".into(),
                    "Approved scope exhausted".into(),
                    "New decision required".into(),
                ],
            }),
            phases: vec![
                WorkflowPhase {
                    id: "prototype".into(),
                    title: "Prototype".into(),
                    scope: "Reduce unknowns before committed implementation".into(),
                    roles: vec!["owner".into(), "implementer".into()],
                    acceptance: vec![
                        "Critical feasibility questions answered".into(),
                        "Telemetry approach identified".into(),
                    ],
                    rollback: vec!["Discard spike output if assumptions fail".into()],
                },
                WorkflowPhase {
                    id: "build".into(),
                    title: "Build".into(),
                    scope: "Implement approved scope and pass local gates".into(),
                    roles: vec!["implementer".into(), "reviewer".into()],
                    acceptance: vec![
                        "Declared gates pass".into(),
                        "Change matches approved scope".into(),
                    ],
                    rollback: vec!["Revert or reconcile if gates or review fail".into()],
                },
                WorkflowPhase {
                    id: "measure".into(),
                    title: "Measure".into(),
                    scope: "Collect telemetry and close the loop with an explicit verdict".into(),
                    roles: vec!["owner".into(), "verifier".into()],
                    acceptance: vec![
                        "Telemetry collected against success criteria".into(),
                        "Outcome verdict recorded".into(),
                    ],
                    rollback: vec!["Iterate or kill if evidence stays weak".into()],
                },
            ],
        }),
        artifacts: Some(Artifacts {
            products: vec![Product {
                id: product.into(),
                name: product.into(),
                organization: None,
            }],
            features: vec![Feature {
                id: id.into(),
                product: product.into(),
                name: id.rsplit('.').next().unwrap_or(id).into(),
                description: None,
            }],
            ..Default::default()
        }),
        links,
        contracts: vec![],
        gates,
        roc: None,
        problem: None,
        hypotheses: vec![],
        decision: None,
        note_rationale: None,
        expires_at: None,
        auto_noted: false,
    }
}

pub fn generate_decision_spec(id: &str, product: &str, question: &str) -> Spec {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    Spec {
        s5d: "1.0".into(),
        id: id.into(),
        version: "1.0.0".into(),
        product: product.into(),
        tier: Tier::Decision,
        allow_update: true,
        meta: Some(Meta {
            title: id.rsplit('.').next().unwrap_or(id).into(),
            authors: vec![],
            date: Some(today),
            tickets: vec![],
            adrs: vec![],
            renames: vec![],
        }),
        context: None,
        workflow: None,
        artifacts: None,
        links: None,
        contracts: vec![],
        gates: crate::gates::default_gates_for_tier(&Tier::Decision),
        roc: None,
        problem: Some(ProblemField::Card(ProblemCard {
            signal: question.into(),
            constraints: vec![],
            targets: vec![],
            indicators: vec![],
            acceptance: None,
            blast_radius: None,
            reversibility: None,
            status: Some("backlog".into()),
            goodhart_guard: None,
        })),
        hypotheses: vec![],
        decision: None,
        note_rationale: None,
        expires_at: None,
        auto_noted: false,
    }
}

pub fn generate_note_spec(id: &str, product: &str, title: &str, rationale: &str) -> Spec {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let expires = (Utc::now() + chrono::Duration::days(90))
        .format("%Y-%m-%d")
        .to_string();
    Spec {
        s5d: "1.0".into(),
        id: id.into(),
        version: "1.0.0".into(),
        product: product.into(),
        tier: Tier::Note,
        allow_update: true,
        meta: Some(Meta {
            title: title.into(),
            authors: vec![],
            date: Some(today),
            tickets: vec![],
            adrs: vec![],
            renames: vec![],
        }),
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
        note_rationale: Some(rationale.into()),
        expires_at: Some(expires),
        auto_noted: false,
    }
}

pub fn generate_record(spec_filename: &str, spec_sha256: &str) -> Record {
    let now = Utc::now().to_rfc3339();
    Record {
        spec_ref: spec_filename.into(),
        spec_sha256: spec_sha256.into(),
        status: SpecStatus::Proposed,
        sync_status: SyncStatus::Unknown,
        status_history: vec![StatusEntry {
            status: SpecStatus::Proposed,
            timestamp: now,
        }],
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
    }
}
