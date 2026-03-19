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
        artifacts: None,
        links: None,
        contracts: vec![],
        gates: vec![],
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
        approvals: vec![],
        preview: None,
        reflection: None,
        gate_results: vec![],
        decision: None,
        verified_by: None,
    }
}
