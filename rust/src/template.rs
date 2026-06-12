use chrono::Utc;

use crate::models::*;

pub fn generate_spec(id: &str, tier: Tier, product: &str) -> Spec {
    let today = Utc::now().format("%Y-%m-%d").to_string();

    // The scaffold must validate out of the box for its tier: a cold user
    // edits obvious placeholders instead of authoring the artifact chain
    // against one missing-field error per attempt (measured: 6 validate
    // round-trips to first green before this). Ids derive from the real
    // product/feature; only `paths` carries an explicit TODO.
    let feature_name = id.rsplit('.').next().unwrap_or(id).to_string();
    let domain_id = format!("dom.{}.core", product);
    let capability_id = format!("cap.{}", feature_name);
    let system_id = format!("sys.{}", product);
    let container_id = format!("ctr.{}.app", product);
    let component_id = format!("comp.{}", feature_name);

    let binding = |pairs: &[(&str, &str)]| Binding {
        fields: pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    };
    let links = Some(Links {
        feature_to_domain: vec![binding(&[("feature", id), ("domain", &domain_id)])],
        component_to_capability: vec![binding(&[
            ("component", &component_id),
            ("capability", &capability_id),
        ])],
        ..Default::default()
    });
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
                name: feature_name.clone(),
                description: None,
            }],
            domains: vec![Domain {
                id: domain_id.clone(),
                product: product.into(),
                name: format!("{} core domain", product),
                classification: Some("supporting".into()),
                description: Some(
                    "Scaffold default — rename or split into real bounded contexts".into(),
                ),
                team: None,
                maturity_level: None,
            }],
            capabilities: vec![Capability {
                id: capability_id.clone(),
                domain: domain_id.clone(),
                name: feature_name.replace('-', " "),
                description: None,
                since: None,
            }],
            systems: vec![SoftwareSystem {
                id: system_id.clone(),
                product: product.into(),
                name: product.into(),
            }],
            containers: vec![Container {
                id: container_id.clone(),
                system: system_id,
                name: format!("{} app", product),
                technology: None,
            }],
            components: vec![Component {
                id: component_id,
                feature: id.into(),
                domain: domain_id,
                container: container_id,
                name: format!("{} component", feature_name.replace('-', " ")),
                paths: vec![crate::validate::SCAFFOLD_PATH_TODO.into()],
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
        intent_kernel: None,
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
        intent_kernel: None,
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
        intent_kernel: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_placeholder_paths_detected_until_replaced() {
        let mut spec = generate_spec("feat.test.thing", Tier::Standard, "Prod");
        let flagged = crate::validate::placeholder_path_components(&spec);
        assert_eq!(flagged, vec!["comp.thing".to_string()]);

        spec.artifacts.as_mut().unwrap().components[0].paths = vec!["src/real/".into()];
        assert!(crate::validate::placeholder_path_components(&spec).is_empty());
    }

    #[test]
    fn intent_kernel_is_fingerprint_neutral_when_absent() {
        // decision.s5d.bmad-native-runtime hard requirement: specs without a
        // kernel must keep their serialized form (YAML and canonical JSON)
        // free of the new key, so existing fingerprints and approval shas
        // do not move on upgrade.
        let spec = generate_spec("feat.test.compat", Tier::Standard, "Prod");
        assert!(spec.intent_kernel.is_none());

        let yaml = serde_yaml::to_string(&spec).unwrap();
        assert!(
            !yaml.contains("intent_kernel"),
            "kernel-less spec YAML must not carry the key:\n{}",
            yaml
        );

        let json = serde_json::to_value(&spec).unwrap();
        assert!(
            json.get("intent_kernel").is_none(),
            "kernel-less spec canonical JSON must not carry the key"
        );

        // Round-trip: the key must stay absent after parse + re-serialize.
        // (Byte-identity is NOT asserted — Binding map entries have unstable
        // key order in YAML; fingerprints are immune because canonical JSON
        // hashing sorts keys.)
        let reparsed: Spec = serde_yaml::from_str(&yaml).unwrap();
        assert!(reparsed.intent_kernel.is_none());
        assert!(!serde_yaml::to_string(&reparsed)
            .unwrap()
            .contains("intent_kernel"));

        // And with a kernel, the key appears with skip-serialized empties.
        let mut with_kernel = generate_spec("feat.test.compat2", Tier::Standard, "Prod");
        with_kernel.intent_kernel = Some(IntentKernel {
            why: "because".into(),
            success_signal: "observable win".into(),
            ..Default::default()
        });
        let yaml2 = serde_yaml::to_string(&with_kernel).unwrap();
        assert!(yaml2.contains("intent_kernel"));
        assert!(
            !yaml2.contains("non_goals"),
            "empty kernel lists must be skip-serialized:\n{}",
            yaml2
        );
    }
}
