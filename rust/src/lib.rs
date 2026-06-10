#![recursion_limit = "512"]

// ── Core ─────────────────────────────────────────────────────────────────────
// Modules on the critical path: intent → validate → approve → apply → drift → rollback

pub mod arch;
pub mod codebase;
pub mod discovery;
pub mod drift;
pub mod gate;
pub mod gates;
pub mod graph;
pub mod hooks_json;
pub mod identity;
pub mod import;
pub mod lsp;
pub mod mcp;
pub mod models;
pub mod phase_gates;
pub mod project;
pub mod ralph;
pub mod router;
pub mod template;
pub mod trace;
pub mod validate;

pub use arch::{architecture_check, ArchitectureCheckReport, ComponentCoverage, SourceDependency};
pub use codebase::{
    build_codebase_snapshot, load_codebase_snapshot, write_codebase_snapshot, CodebaseCoverage,
    CodebaseModule, CodebaseSnapshot, CoverageStatus, ModuleCoverage,
};
pub use discovery::{
    build_discovery_snapshot, read_discovery_snapshot, write_discovery_snapshot,
    DiscoveryArtifactPaths, DiscoveryClaimStatus, DiscoveryClaimView, DiscoveryEdge,
    DiscoveryEvidence, DiscoveryFile, DiscoveryGraph, DiscoveryManifest, DiscoveryMetamodel,
    DiscoveryNode, DiscoveryProvenance, DiscoverySnapshot,
};
pub use drift::{check_drift, reconcile, DriftResult};
pub use gates::{effective_gates_for_spec, run_gates};
pub use graph::{check_domain_layering, graph_check, tarjan_scc};
pub use identity::{AliasEntry, AliasTable};
pub use import::{compute_diff, compute_state_fingerprint, execute_import, DiffActions};
pub use models::*;
pub use phase_gates::{
    check_add_hypothesis, check_approve, check_challenge, check_decide, check_import,
    enforce_checks, PhaseCheck, Severity,
};
pub use project::S5dProject;
pub use ralph::{build_ralph_task_package, RalphPreset};
pub use router::{route, RouteMode, RouteResult};
pub use template::*;
pub use trace::{
    format_code_trace, trace_code_path, CodeTrace, CodeTraceCapability, CodeTraceDecision,
    CodeTraceMatch,
};
pub use validate::validate_spec;

pub mod infer;
pub mod suite;

/// Validate that an ID is safe for use in file paths.
/// Rejects IDs containing path separators, `..`, null bytes, or empty strings.
pub fn sanitize_id(id: &str) -> anyhow::Result<&str> {
    if id.is_empty() {
        anyhow::bail!("spec ID must not be empty");
    }
    if id.contains('/') || id.contains('\\') || id.contains("..") || id.contains('\0') {
        anyhow::bail!(
            "invalid spec ID '{}': must not contain path separators, '..', or null bytes",
            id
        );
    }
    Ok(id)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn build_notifications_spec() -> Spec {
        Spec {
            s5d: "1.0".into(),
            id: "feat.notifications".into(),
            version: "1.0.0".into(),
            product: "MyApp".into(),
            tier: Tier::Standard,
            allow_update: false,
            meta: Some(Meta {
                title: "Notifications System".into(),
                authors: vec!["team-backend".into()],
                date: Some("2026-03-18".into()),
                tickets: vec!["PROJ-123".into()],
                adrs: vec![],
                renames: vec![],
            }),
            context: Some(
                "## Problem\nUsers miss important updates\n\n## Goal\nReal-time multi-channel notifications\n\n## Privacy\nClassification: Internal\nPII: email addresses stored, subject to GDPR\n".into(),
            ),
            artifacts: Some(Artifacts {
                products: vec![Product {
                    id: "myapp".into(),
                    name: "MyApp".into(),
                    organization: None,
                }],
                domains: vec![
                    Domain {
                        id: "notifications".into(),
                        product: "myapp".into(),
                        name: "Notifications".into(),
                        classification: Some("Core".into()),
                        description: None,
                        team: Some("backend".into()),
                        maturity_level: Some("product".into()),
                    },
                    Domain {
                        id: "users".into(),
                        product: "myapp".into(),
                        name: "Users".into(),
                        classification: Some("Supporting".into()),
                        description: None,
                        team: Some("platform".into()),
                        maturity_level: Some("commodity".into()),
                    },
                ],
                capabilities: vec![
                    Capability {
                        id: "send-notification".into(),
                        domain: "notifications".into(),
                        name: "SendNotification".into(),
                        description: None,
                        since: Some("1.0.0".into()),
                    },
                    Capability {
                        id: "manage-preferences".into(),
                        domain: "notifications".into(),
                        name: "ManagePreferences".into(),
                        description: None,
                        since: Some("1.0.0".into()),
                    },
                    Capability {
                        id: "resolve-user".into(),
                        domain: "users".into(),
                        name: "ResolveUser".into(),
                        description: None,
                        since: None,
                    },
                ],
                entities: vec![
                    Entity {
                        id: "notification".into(),
                        domain: "notifications".into(),
                        name: "Notification".into(),
                    },
                    Entity {
                        id: "preference".into(),
                        domain: "notifications".into(),
                        name: "NotificationPreference".into(),
                    },
                    Entity {
                        id: "user".into(),
                        domain: "users".into(),
                        name: "User".into(),
                    },
                ],
                features: vec![Feature {
                    id: "feat.notifications".into(),
                    product: "MyApp".into(),
                    name: "Notifications System".into(),
                    description: Some("Multi-channel notification delivery".into()),
                }],
                use_cases: vec![
                    UseCase {
                        id: "uc.send-push".into(),
                        feature: "feat.notifications".into(),
                        name: "Send Push Notification".into(),
                    },
                    UseCase {
                        id: "uc.send-email".into(),
                        feature: "feat.notifications".into(),
                        name: "Send Email Notification".into(),
                    },
                    UseCase {
                        id: "uc.update-prefs".into(),
                        feature: "feat.notifications".into(),
                        name: "Update Notification Preferences".into(),
                    },
                ],
                systems: vec![SoftwareSystem {
                    id: "myapp-backend".into(),
                    product: "myapp".into(),
                    name: "MyApp Backend".into(),
                }],
                containers: vec![
                    Container {
                        id: "api-gateway".into(),
                        system: "myapp-backend".into(),
                        name: "API Gateway".into(),
                        technology: Some("Rust/Axum".into()),
                    },
                    Container {
                        id: "notification-worker".into(),
                        system: "myapp-backend".into(),
                        name: "Notification Worker".into(),
                        technology: Some("Rust/Tokio".into()),
                    },
                    Container {
                        id: "user-service".into(),
                        system: "myapp-backend".into(),
                        name: "User Service".into(),
                        technology: Some("Rust/Axum".into()),
                    },
                ],
                components: vec![
                    Component {
                        id: "notif-handler".into(),
                        feature: "feat.notifications".into(),
                        domain: "notifications".into(),
                        container: "api-gateway".into(),
                        name: "NotificationHandler".into(),
                        paths: vec!["src/handlers/notification.rs".into()],
                    },
                    Component {
                        id: "notif-dispatcher".into(),
                        feature: "feat.notifications".into(),
                        domain: "notifications".into(),
                        container: "notification-worker".into(),
                        name: "NotificationDispatcher".into(),
                        paths: vec!["src/workers/dispatch.rs".into()],
                    },
                    Component {
                        id: "pref-handler".into(),
                        feature: "feat.notifications".into(),
                        domain: "notifications".into(),
                        container: "api-gateway".into(),
                        name: "PreferenceHandler".into(),
                        paths: vec!["src/handlers/preference.rs".into()],
                    },
                    Component {
                        id: "user-resolver".into(),
                        feature: "feat.notifications".into(),
                        domain: "users".into(),
                        container: "user-service".into(),
                        name: "UserResolver".into(),
                        paths: vec!["src/resolvers/user.rs".into()],
                    },
                ],
                roles: vec![
                    Role {
                        id: "end-user".into(),
                        name: "End User".into(),
                        kind: Some("human".into()),
                    },
                    Role {
                        id: "admin".into(),
                        name: "Admin".into(),
                        kind: Some("human".into()),
                    },
                ],
                concerns: vec![
                    Concern {
                        id: "concern-delivery-speed".into(),
                        role: "end-user".into(),
                        name: "Notification arrives within 5 seconds".into(),
                        supersystem: Some("vs-user-engagement".into()),
                        confirmed: true,
                    },
                    Concern {
                        id: "concern-no-spam".into(),
                        role: "end-user".into(),
                        name: "Respect opt-out preferences".into(),
                        supersystem: None,
                        confirmed: false,
                    },
                ],
                metrics: vec![
                    AcceptanceMetric {
                        id: "metric-p99-delivery".into(),
                        name: "Delivery latency p99".into(),
                        units: Some("ms".into()),
                        how_to_measure: Some("End-to-end from API call to device receipt".into()),
                        supersystem: Some("vs-user-engagement".into()),
                    },
                    AcceptanceMetric {
                        id: "metric-opt-out-rate".into(),
                        name: "Opt-out compliance".into(),
                        units: Some("%".into()),
                        how_to_measure: Some("Notifications sent to opted-out users / total sent".into()),
                        supersystem: None,
                    },
                ],
                supersystems: vec![SuperSystem {
                    id: "vs-user-engagement".into(),
                    product: "myapp".into(),
                    name: "User Engagement Value Stream".into(),
                    kind: Some("value_stream".into()),
                    description: Some("Keep users informed and engaged with timely notifications".into()),
                }],
                transports: vec![Transport {
                    id: "rest-json".into(),
                    transport_type: "rest".into(),
                    serialization: Some("json".into()),
                    description: None,
                }],
            }),
            links: Some(Links {
                feature_to_domain: vec![
                    Binding {
                        fields: [
                            ("feature".into(), "feat.notifications".into()),
                            ("domain".into(), "notifications".into()),
                        ]
                        .into_iter()
                        .collect::<HashMap<_, _>>(),
                    },
                    Binding {
                        fields: [
                            ("feature".into(), "feat.notifications".into()),
                            ("domain".into(), "users".into()),
                        ]
                        .into_iter()
                        .collect::<HashMap<_, _>>(),
                    },
                ],
                use_case_to_capability: vec![
                    Binding {
                        fields: [
                            ("use_case".into(), "uc.send-push".into()),
                            ("capability".into(), "send-notification".into()),
                        ]
                        .into_iter()
                        .collect::<HashMap<_, _>>(),
                    },
                    Binding {
                        fields: [
                            ("use_case".into(), "uc.send-email".into()),
                            ("capability".into(), "send-notification".into()),
                        ]
                        .into_iter()
                        .collect::<HashMap<_, _>>(),
                    },
                    Binding {
                        fields: [
                            ("use_case".into(), "uc.update-prefs".into()),
                            ("capability".into(), "manage-preferences".into()),
                        ]
                        .into_iter()
                        .collect::<HashMap<_, _>>(),
                    },
                ],
                use_case_to_entity: vec![
                    Binding {
                        fields: [
                            ("use_case".into(), "uc.send-push".into()),
                            ("entity".into(), "notification".into()),
                        ]
                        .into_iter()
                        .collect::<HashMap<_, _>>(),
                    },
                    Binding {
                        fields: [
                            ("use_case".into(), "uc.update-prefs".into()),
                            ("entity".into(), "preference".into()),
                        ]
                        .into_iter()
                        .collect::<HashMap<_, _>>(),
                    },
                ],
                component_to_capability: vec![
                    Binding {
                        fields: [
                            ("component".into(), "notif-handler".into()),
                            ("capability".into(), "send-notification".into()),
                        ]
                        .into_iter()
                        .collect::<HashMap<_, _>>(),
                    },
                    Binding {
                        fields: [
                            ("component".into(), "notif-dispatcher".into()),
                            ("capability".into(), "send-notification".into()),
                        ]
                        .into_iter()
                        .collect::<HashMap<_, _>>(),
                    },
                    Binding {
                        fields: [
                            ("component".into(), "pref-handler".into()),
                            ("capability".into(), "manage-preferences".into()),
                        ]
                        .into_iter()
                        .collect::<HashMap<_, _>>(),
                    },
                    Binding {
                        fields: [
                            ("component".into(), "user-resolver".into()),
                            ("capability".into(), "resolve-user".into()),
                        ]
                        .into_iter()
                        .collect::<HashMap<_, _>>(),
                    },
                ],
                component_to_entity: vec![Binding {
                    fields: [
                        ("component".into(), "notif-dispatcher".into()),
                        ("entity".into(), "notification".into()),
                    ]
                    .into_iter()
                    .collect::<HashMap<_, _>>(),
                }],
                container_to_capability: vec![],
                concern_to_metric: vec![
                    Binding {
                        fields: [
                            ("concern".into(), "concern-delivery-speed".into()),
                            ("metric".into(), "metric-p99-delivery".into()),
                        ]
                        .into_iter()
                        .collect::<HashMap<_, _>>(),
                    },
                    Binding {
                        fields: [
                            ("concern".into(), "concern-no-spam".into()),
                            ("metric".into(), "metric-opt-out-rate".into()),
                        ]
                        .into_iter()
                        .collect::<HashMap<_, _>>(),
                    },
                ],
                component_to_container: vec![],
                edges: vec![Edge {
                    from: "notifications".into(),
                    to: "users".into(),
                    archetype: "customer_supplier".into(),
                    description: None,
                    downstream_capability: Some("resolve-user".into()),
                    waiver: None,
                    transport_ref: None,
                }],
                depends_on: vec![],
                entity_relations: vec![EntityRelation {
                    entity: "user".into(),
                    related_entity: "notification".into(),
                    cardinality: Some("1:N".into()),
                    projection: false,
                    aggregate_root: false,
                }],
                capability_to_entity: vec![Binding {
                    fields: [
                        ("capability".into(), "send-notification".into()),
                        ("entity".into(), "notification".into()),
                    ]
                    .into_iter()
                    .collect::<HashMap<_, _>>(),
                }],
                capability_to_concern: vec![],
            }),
            contracts: vec![Contract {
                id: "contract-notification-api".into(),
                format: "openapi".into(),
                path: Some("contracts/notification-api.yaml".into()),
                sha256: Some(
                    "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".into(),
                ),
                inline: None,
                binds_to: vec![ContractBinding {
                    capability: Some("send-notification".into()),
                    entity: None,
                }],
            }],
            gates: vec![
                Gate { kind: "schema".into() },
                Gate { kind: "graph".into() },
                Gate { kind: "contract".into() },
                Gate { kind: "test".into() },
                Gate { kind: "lint".into() },
            ],
            roc: None,
            problem: None,
            hypotheses: vec![],
            decision: None,
            workflow: None,
            note_rationale: None,
            expires_at: None,
            auto_noted: false,
        }
    }

    #[test]
    fn test_init_and_find() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        assert!(S5dProject::find(root).is_none());

        S5dProject::init(root).unwrap();

        assert!(S5dProject::find(root).is_some());

        let sub = root.join("a/b/c");
        std::fs::create_dir_all(&sub).unwrap();
        let found = S5dProject::find(&sub).unwrap();
        assert_eq!(found.root, root);
    }

    #[test]
    fn test_init_report() {
        let dir = TempDir::new().unwrap();
        let (project, report) = S5dProject::init(dir.path()).unwrap();

        assert_eq!(report.root, dir.path());
        assert_eq!(report.dirs_created.len(), 6);
        assert_eq!(report.files_created.len(), 3);

        // Verify all dirs exist
        for d in &report.dirs_created {
            assert!(d.is_dir(), "dir should exist: {}", d.display());
        }
        // Verify all files exist
        for f in &report.files_created {
            assert!(f.is_file(), "file should exist: {}", f.display());
        }

        // Verify structure
        assert!(project.s5d_dir().join("packages").is_dir());
        assert!(project.s5d_dir().join("records").is_dir());
        assert!(project.s5d_dir().join("tasks").is_dir());
        assert!(project.s5d_dir().join("harness").is_dir());
        assert!(project.s5d_dir().join("discovery").is_dir());
        assert!(project.s5d_dir().join(".locks").is_dir());
        assert!(project.s5d_dir().join("config.yaml").is_file());
        assert!(project.s5d_dir().join("ledger.yaml").is_file());
        assert!(project.s5d_dir().join("index.yaml").is_file());
    }

    #[test]
    fn test_init_twice_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let (_, first_report) = S5dProject::init(dir.path()).unwrap();
        assert!(
            !first_report.dirs_created.is_empty(),
            "first init should create dirs"
        );

        // Second init should succeed with empty report (idempotent)
        let (_, second_report) = S5dProject::init(dir.path()).unwrap();
        assert!(
            second_report.dirs_created.is_empty(),
            "second init should create nothing"
        );
        assert!(
            second_report.files_created.is_empty(),
            "second init should create nothing"
        );
    }

    #[test]
    fn test_ensure_dirs_recovers() {
        let dir = TempDir::new().unwrap();
        let (project, _) = S5dProject::init(dir.path()).unwrap();

        // Delete a subdirectory
        std::fs::remove_dir_all(project.s5d_dir().join("tasks")).unwrap();
        assert!(!project.s5d_dir().join("tasks").exists());

        // ensure_dirs recovers it
        project.ensure_dirs().unwrap();
        assert!(project.s5d_dir().join("tasks").is_dir());
    }

    #[test]
    fn test_project_lock_blocks_concurrent_mutation() {
        let dir = TempDir::new().unwrap();
        let (project, _) = S5dProject::init(dir.path()).unwrap();

        let lock = project.acquire_lock("import.feat.locked").unwrap();
        let second = project.acquire_lock("import.feat.locked");
        assert!(
            second.is_err(),
            "second lock acquisition should fail while held"
        );
        drop(lock);

        let third = project.acquire_lock("import.feat.locked");
        assert!(third.is_ok(), "lock should be acquirable after release");
    }

    #[test]
    fn test_template_generation() {
        let spec = generate_spec("feat.orders.tracking", Tier::Standard, "MyProduct");
        assert_eq!(spec.s5d, "1.0");
        assert_eq!(spec.id, "feat.orders.tracking");
        assert_eq!(spec.version, "1.0.0");
        assert_eq!(spec.product, "MyProduct");

        let arts = spec.artifacts.as_ref().unwrap();
        assert_eq!(arts.features.len(), 1);
        assert_eq!(arts.features[0].id, "feat.orders.tracking");
        assert_eq!(arts.features[0].name, "tracking");
    }

    #[test]
    fn test_validate_catches_bad_spec() {
        let mut spec = generate_spec("feat.test", Tier::Standard, "Prod");
        // Break the single-feature invariant
        spec.artifacts.as_mut().unwrap().features[0].id = "feat.other".into();
        let errors = validate_spec(&spec);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("single-feature invariant")),
            "Expected single-feature invariant error, got: {:?}",
            errors
        );
    }

    fn add_minimal_metamodel(spec: &mut Spec) {
        if let Some(ref mut a) = spec.artifacts {
            a.domains.push(Domain {
                id: "dom.test.core".into(),
                product: "test".into(),
                name: "Core".into(),
                classification: Some("core".into()),
                description: None,
                team: None,
                maturity_level: None,
            });
            a.capabilities.push(Capability {
                id: "cap.DoThing".into(),
                domain: "dom.test.core".into(),
                name: "DoThing".into(),
                description: None,
                since: None,
            });
            a.systems.push(SoftwareSystem {
                id: "sys.test".into(),
                product: "test".into(),
                name: "Test system".into(),
            });
            a.containers.push(Container {
                id: "ctr.api".into(),
                name: "API Server".into(),
                system: "sys.test".into(),
                technology: None,
            });
            a.components.push(Component {
                id: "comp.handler".into(),
                domain: "dom.test.core".into(),
                feature: spec.id.clone(),
                container: "ctr.api".into(),
                name: "Handler".into(),
                paths: vec!["src/handler.rs".into()],
            });
        }
        let links = spec.links.get_or_insert_with(Links::default);
        links.component_to_capability.push(Binding {
            fields: [
                ("component".into(), "comp.handler".into()),
                ("capability".into(), "cap.DoThing".into()),
            ]
            .into_iter()
            .collect::<HashMap<_, _>>(),
        });
    }

    #[test]
    fn scaffold_placeholder_paths_detected_until_replaced() {
        let mut spec = generate_spec("feat.test.thing", Tier::Standard, "Prod");
        let flagged = crate::validate::placeholder_path_components(&spec);
        assert_eq!(flagged, vec!["comp.thing".to_string()]);

        spec.artifacts.as_mut().unwrap().components[0].paths = vec!["src/real/".into()];
        assert!(crate::validate::placeholder_path_components(&spec).is_empty());
    }

    #[test]
    fn test_validate_valid_spec() {
        let mut spec = generate_spec("feat.test.thing", Tier::Standard, "Prod");
        add_minimal_metamodel(&mut spec);
        let errors = validate_spec(&spec);
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_validate_high_tier_requires_privacy() {
        let mut spec = generate_spec("feat.test.high", Tier::High, "Prod");
        add_minimal_metamodel(&mut spec);
        // Default template includes "Privacy" in context — should pass
        let errors = validate_spec(&spec);
        assert!(
            errors.is_empty(),
            "Expected no errors for high tier with privacy context, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validate_high_tier_missing_privacy_fails() {
        let mut spec = generate_spec("feat.test.high", Tier::High, "Prod");
        add_minimal_metamodel(&mut spec);
        spec.context = Some("## Problem\n\n## Goal\n".into()); // no privacy
        let errors = validate_spec(&spec);
        assert!(
            errors.iter().any(|e| e.contains("privacy")),
            "Expected privacy error for high tier, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_graph_check_no_cycle() {
        let mut spec = generate_spec("feat.test.graph", Tier::Standard, "Prod");
        spec.links = Some(Links {
            edges: vec![
                Edge {
                    from: "A".into(),
                    to: "B".into(),
                    archetype: "upstream_downstream".into(),
                    ..Default::default()
                },
                Edge {
                    from: "B".into(),
                    to: "C".into(),
                    archetype: "upstream_downstream".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        let errors = graph_check(&spec);
        assert!(
            errors.is_empty(),
            "Expected no cycle errors, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_graph_check_detects_cycle() {
        let mut spec = generate_spec("feat.test.cycle", Tier::Standard, "Prod");
        spec.links = Some(Links {
            edges: vec![
                Edge {
                    from: "A".into(),
                    to: "B".into(),
                    archetype: "upstream_downstream".into(),
                    ..Default::default()
                },
                Edge {
                    from: "B".into(),
                    to: "C".into(),
                    archetype: "upstream_downstream".into(),
                    ..Default::default()
                },
                Edge {
                    from: "C".into(),
                    to: "A".into(),
                    archetype: "upstream_downstream".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        let errors = graph_check(&spec);
        assert!(
            errors.iter().any(|e| e.contains("cycle detected")),
            "Expected cycle error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_graph_check_bidirectional_not_cycle() {
        let mut spec = generate_spec("feat.test.bidir", Tier::Standard, "Prod");
        spec.links = Some(Links {
            edges: vec![
                Edge {
                    from: "A".into(),
                    to: "B".into(),
                    archetype: "shared_kernel".into(),
                    ..Default::default()
                },
                Edge {
                    from: "B".into(),
                    to: "A".into(),
                    archetype: "shared_kernel".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        let errors = graph_check(&spec);
        assert!(
            errors.is_empty(),
            "Bidirectional archetypes should not trigger cycle detection, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_multidomain_spec_validates() {
        let spec = build_notifications_spec();
        let errors = validate_spec(&spec);
        assert!(errors.is_empty(), "Expected 0 errors, got: {:?}", errors);
    }

    #[test]
    fn test_multidomain_yaml_roundtrip() {
        let spec = build_notifications_spec();
        let yaml = serde_yaml::to_string(&spec).unwrap();
        let loaded: Spec = serde_yaml::from_str(&yaml).unwrap();

        let arts = loaded.artifacts.as_ref().unwrap();
        assert_eq!(arts.domains.len(), 2, "expected 2 domains");
        assert_eq!(arts.capabilities.len(), 3, "expected 3 capabilities");
        assert_eq!(arts.entities.len(), 3, "expected 3 entities");
        assert_eq!(arts.components.len(), 4, "expected 4 components");
        assert_eq!(arts.use_cases.len(), 3, "expected 3 use_cases");
        assert_eq!(arts.roles.len(), 2, "expected 2 roles");
        assert_eq!(arts.concerns.len(), 2, "expected 2 concerns");
        assert_eq!(arts.metrics.len(), 2, "expected 2 metrics");

        let links = loaded.links.as_ref().unwrap();
        assert_eq!(
            links.feature_to_domain.len(),
            2,
            "expected 2 feature_to_domain"
        );
        assert_eq!(
            links.use_case_to_capability.len(),
            3,
            "expected 3 use_case_to_capability"
        );
        assert_eq!(
            links.use_case_to_entity.len(),
            2,
            "expected 2 use_case_to_entity"
        );
        assert_eq!(
            links.component_to_capability.len(),
            4,
            "expected 4 component_to_capability"
        );
        assert_eq!(
            links.component_to_entity.len(),
            1,
            "expected 1 component_to_entity"
        );
        assert_eq!(links.edges.len(), 1, "expected 1 edge");
        assert_eq!(
            links.entity_relations.len(),
            1,
            "expected 1 entity_relation"
        );
        assert_eq!(
            links.capability_to_entity.len(),
            1,
            "expected 1 capability_to_entity"
        );
        assert_eq!(
            links.edges[0].downstream_capability.as_deref(),
            Some("resolve-user"),
            "expected downstream_capability"
        );

        assert_eq!(arts.supersystems.len(), 1, "expected 1 supersystem");
        assert_eq!(arts.supersystems[0].kind.as_deref(), Some("value_stream"));
        assert_eq!(arts.domains[0].maturity_level.as_deref(), Some("product"));
        assert_eq!(arts.domains[1].maturity_level.as_deref(), Some("commodity"));

        assert_eq!(loaded.contracts.len(), 1, "expected 1 contract");
        assert_eq!(loaded.gates.len(), 5, "expected 5 gates");
    }

    #[test]
    fn test_multidomain_graph_check() {
        let spec = build_notifications_spec();
        let errors = graph_check(&spec);
        assert!(
            errors.is_empty(),
            "Expected no graph errors for customer_supplier edge, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_multidomain_graph_cross_domain_cycle() {
        let mut spec = build_notifications_spec();
        let links = spec.links.as_mut().unwrap();
        links.edges.push(Edge {
            from: "users".into(),
            to: "notifications".into(),
            archetype: "upstream_downstream".into(),
            ..Default::default()
        });
        let errors = graph_check(&spec);
        assert!(
            errors.iter().any(|e| e.contains("cycle detected")),
            "Expected cycle error from users→notifications back-edge, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_multidomain_import_creates_uuids() {
        let dir = TempDir::new().unwrap();
        let (project, _) = S5dProject::init(dir.path()).unwrap();

        let spec = build_notifications_spec();
        let yaml = serde_yaml::to_string(&spec).unwrap();
        let spec_filename = "feat.notifications__20260318.s5d.yaml";
        let spec_path = project.s5d_dir().join("packages").join(spec_filename);
        std::fs::write(&spec_path, &yaml).unwrap();

        let (actions, _fp) = execute_import(&project, &spec_path, &spec, spec_filename).unwrap();

        let total_created = actions.create.len();
        // 1 Product + 2 Domains + 3 Capabilities + 3 Entities + 1 Feature + 3 UseCases
        // + 1 SoftwareSystem + 3 Containers + 4 Components + 2 Roles + 2 Concerns + 2 AcceptanceMetrics
        // + 1 SuperSystem + 1 Transport
        // = 29 artifacts
        assert_eq!(
            total_created, 29,
            "Expected 29 UUIDs created, got: {}",
            total_created
        );

        let aliases = AliasTable::load(&project.s5d_dir()).unwrap();
        let type_names = [
            "Product",
            "Domain",
            "Capability",
            "Entity",
            "Feature",
            "UseCase",
            "SoftwareSystem",
            "Container",
            "Component",
            "Role",
            "Concern",
            "AcceptanceMetric",
            "SuperSystem",
            "Transport",
        ];
        for type_name in type_names {
            let in_global = aliases
                .global
                .iter()
                .any(|e| e.artifact_type == type_name && !e.deprecated);
            let in_packages = aliases
                .packages
                .iter()
                .any(|e| e.artifact_type == type_name && !e.deprecated);
            assert!(
                in_global || in_packages,
                "No alias found for type: {}",
                type_name
            );
        }
    }

    #[test]
    fn test_multidomain_import_idempotent() {
        let dir = TempDir::new().unwrap();
        let (project, _) = S5dProject::init(dir.path()).unwrap();

        let spec = build_notifications_spec();
        let yaml = serde_yaml::to_string(&spec).unwrap();
        let spec_filename = "feat.notifications__20260318.s5d.yaml";
        let spec_path = project.s5d_dir().join("packages").join(spec_filename);
        std::fs::write(&spec_path, &yaml).unwrap();

        let (first_actions, _) =
            execute_import(&project, &spec_path, &spec, spec_filename).unwrap();
        let (second_actions, _) =
            execute_import(&project, &spec_path, &spec, spec_filename).unwrap();

        assert_eq!(
            second_actions.create.len(),
            0,
            "Second import should not create new artifacts, got creates: {:?}",
            second_actions.create
        );
        assert_eq!(
            second_actions.update.len(),
            first_actions.create.len(),
            "Second import should produce updates equal to first import's creates"
        );
    }

    #[test]
    fn test_multidomain_drift_detection() {
        let dir = TempDir::new().unwrap();
        let (project, _) = S5dProject::init(dir.path()).unwrap();

        let spec = build_notifications_spec();
        let yaml = serde_yaml::to_string(&spec).unwrap();
        let spec_filename = "feat.notifications__20260318.s5d.yaml";
        let spec_path = project.s5d_dir().join("packages").join(spec_filename);
        std::fs::write(&spec_path, &yaml).unwrap();

        execute_import(&project, &spec_path, &spec, spec_filename).unwrap();

        let mut modified_spec = spec.clone();
        modified_spec
            .artifacts
            .as_mut()
            .unwrap()
            .capabilities
            .push(Capability {
                id: "archive-notification".into(),
                domain: "notifications".into(),
                name: "ArchiveNotification".into(),
                description: None,
                since: None,
            });

        let drift = check_drift(&project, &modified_spec, spec_filename).unwrap();
        assert!(
            matches!(drift, DriftResult::Drifted { .. }),
            "Expected Drifted after adding a capability"
        );
    }

    #[test]
    fn test_multidomain_contract_missing_sha_fails() {
        let mut spec = build_notifications_spec();
        spec.contracts[0].sha256 = None;
        let errors = validate_spec(&spec);
        assert!(
            errors.iter().any(|e| e.contains("missing sha256")),
            "Expected missing sha256 error for contract with path, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_multidomain_invalid_edge_archetype() {
        let mut spec = build_notifications_spec();
        spec.links.as_mut().unwrap().edges[0].archetype = "bogus".into();
        let errors = validate_spec(&spec);
        assert!(
            errors.iter().any(|e| e.contains("invalid edge archetype")),
            "Expected invalid edge archetype error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_decision_spec_with_hypotheses_roundtrip() {
        let spec = Spec {
            s5d: "1.0".into(),
            id: "dec.cache-strategy".into(),
            version: "1.0.0".into(),
            product: "MyApp".into(),
            tier: Tier::Decision,
            allow_update: false,
            meta: Some(Meta {
                title: "Cache Strategy Decision".into(),
                authors: vec!["arch-team".into()],
                date: Some("2026-03-18".into()),
                tickets: vec!["PROJ-456".into()],
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
                signal: "API p99 latency exceeds 500ms under peak load".into(),
                constraints: vec!["Must not increase infra cost by more than 10%".into()],
                targets: vec!["Reduce p99 latency below 200ms".into()],
                indicators: vec!["Cache hit rate".into()],
                acceptance: Some("p99 < 200ms on load test at 10k RPS".into()),
                blast_radius: Some("All API consumers".into()),
                reversibility: Some("moderate".into()),
                status: Some("in_progress".into()),
                goodhart_guard: None,
            })),
            hypotheses: vec![
                Hypothesis {
                    id: "h1-redis".into(),
                    title: "Redis distributed cache".into(),
                    content: "Use Redis as a shared cache layer in front of the DB".into(),
                    scope: "infrastructure".into(),
                    kind: "solution".into(),
                    layer: "data".into(),
                    r_eff: Some(0.85),
                    evidence: vec![HypothesisEvidence {
                        id: "ev1".into(),
                        evidence_type: "benchmark".into(),
                        content: "Redis benchmarks show sub-ms reads at 100k ops/s".into(),
                        verdict: "supports".into(),
                        valid_until: Some("2027-03-18".into()),
                        carrier_ref: None,
                        formality: Some(7),
                        claim_scope: vec!["throughput".into(), "latency".into()],
                        congruence_level: Some(3),
                        reliability: Some(0.85),
                        refine_kind: None,
                        skill: None,
                        agent: None,
                    }],
                    depends_on: vec![],
                    rationale: Some("Industry standard, well-understood operationally".into()),
                    spec_ref: None,
                    prompt: None,
                    next_move: None,
                },
                Hypothesis {
                    id: "h2-in-process".into(),
                    title: "In-process LRU cache".into(),
                    content: "Cache hot data in each service instance using an LRU map".into(),
                    scope: "application".into(),
                    kind: "solution".into(),
                    layer: "application".into(),
                    r_eff: Some(0.60),
                    evidence: vec![HypothesisEvidence {
                        id: "ev2".into(),
                        evidence_type: "analysis".into(),
                        content: "No network hop but inconsistency across instances".into(),
                        verdict: "partially_supports".into(),
                        valid_until: None,
                        carrier_ref: None,
                        formality: Some(3),
                        claim_scope: vec!["consistency".into()],
                        congruence_level: Some(2),
                        reliability: Some(0.6),
                        refine_kind: None,
                        skill: None,
                        agent: None,
                    }],
                    depends_on: vec![],
                    rationale: None,
                    spec_ref: None,
                    prompt: None,
                    next_move: None,
                },
            ],
            decision: Some(DecisionRecord {
                title: "Use Redis for distributed caching".into(),
                winner_id: "h1-redis".into(),
                rejected_ids: vec!["h2-in-process".into()],
                context: "High-traffic API with multiple service instances".into(),
                decision: "Adopt Redis as shared cache layer".into(),
                rationale: "Consistency across instances outweighs operational overhead".into(),
                consequences: "Need Redis cluster, adds infra complexity".into(),
                decided_at: Some("2026-03-18".into()),
                confirmed_by: None,
                expires_at: Some("2026-06-16".into()),
                do_list: vec!["Use Redis for all shared cache".into()],
                dont_list: vec!["Don't use in-process cache for shared state".into()],
                challenge: None,
                decision_subject: None,
                decision_subject_granularity: None,
                evaluative_surface: None,
                belief_state: None,
                outcome_model: None,
                pareto_set: vec![],
                choice_rule: None,
            }),
            note_rationale: None,
            expires_at: None,
            workflow: None,
            auto_noted: false,
        };

        let errors = validate_spec(&spec);
        assert!(
            errors.is_empty(),
            "Decision spec should be valid, got: {:?}",
            errors
        );

        let yaml = serde_yaml::to_string(&spec).unwrap();
        let loaded: Spec = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(loaded.hypotheses.len(), 2, "expected 2 hypotheses");
        assert_eq!(loaded.hypotheses[0].id, "h1-redis");
        assert_eq!(loaded.hypotheses[0].evidence.len(), 1);
        assert_eq!(loaded.hypotheses[0].evidence[0].verdict, "supports");
        assert_eq!(loaded.hypotheses[1].id, "h2-in-process");

        let dr = loaded.decision.as_ref().unwrap();
        assert_eq!(dr.winner_id, "h1-redis");
        assert_eq!(dr.rejected_ids, vec!["h2-in-process"]);
        assert_eq!(dr.decided_at.as_deref(), Some("2026-03-18"));

        let pc = loaded.problem.as_ref().unwrap().as_card().unwrap();
        assert_eq!(pc.reversibility.as_deref(), Some("moderate"));
        assert_eq!(pc.constraints.len(), 1);
    }

    #[test]
    fn test_entity_relation_roundtrip() {
        let spec = Spec {
            s5d: "1.0".into(),
            id: "feat.er-test".into(),
            version: "1.0.0".into(),
            product: "Prod".into(),
            tier: Tier::Standard,
            allow_update: false,
            meta: None,
            context: Some("## Problem\n\n## Goal\n\n## Privacy\nClassification: Internal\n".into()),
            artifacts: Some(Artifacts {
                products: vec![Product {
                    id: "prod".into(),
                    name: "Prod".into(),
                    organization: None,
                }],
                features: vec![Feature {
                    id: "feat.er-test".into(),
                    product: "prod".into(),
                    name: "er-test".into(),
                    description: None,
                }],
                entities: vec![
                    Entity {
                        id: "order".into(),
                        domain: "orders".into(),
                        name: "Order".into(),
                    },
                    Entity {
                        id: "line-item".into(),
                        domain: "orders".into(),
                        name: "LineItem".into(),
                    },
                ],
                ..Default::default()
            }),
            links: Some(Links {
                entity_relations: vec![EntityRelation {
                    entity: "order".into(),
                    related_entity: "line-item".into(),
                    cardinality: Some("1:N".into()),
                    projection: false,
                    aggregate_root: true,
                }],
                ..Default::default()
            }),
            contracts: vec![],
            gates: vec![],
            roc: None,
            problem: None,
            hypotheses: vec![],
            decision: None,
            workflow: None,
            note_rationale: None,
            expires_at: None,
            auto_noted: false,
        };

        let yaml = serde_yaml::to_string(&spec).unwrap();
        let loaded: Spec = serde_yaml::from_str(&yaml).unwrap();

        let er = &loaded.links.as_ref().unwrap().entity_relations[0];
        assert_eq!(er.entity, "order");
        assert_eq!(er.related_entity, "line-item");
        assert_eq!(er.cardinality.as_deref(), Some("1:N"));
        assert!(!er.projection);
        assert!(er.aggregate_root);
    }

    #[test]
    fn test_supersystem_import() {
        let dir = TempDir::new().unwrap();
        let (project, _) = S5dProject::init(dir.path()).unwrap();

        let spec = Spec {
            s5d: "1.0".into(),
            id: "feat.ss-test".into(),
            version: "1.0.0".into(),
            product: "Prod".into(),
            tier: Tier::Standard,
            allow_update: false,
            meta: None,
            context: Some("## Problem\n\n## Goal\n\n## Privacy\nClassification: Internal\n".into()),
            artifacts: Some(Artifacts {
                products: vec![Product {
                    id: "prod".into(),
                    name: "Prod".into(),
                    organization: None,
                }],
                features: vec![Feature {
                    id: "feat.ss-test".into(),
                    product: "prod".into(),
                    name: "ss-test".into(),
                    description: None,
                }],
                supersystems: vec![SuperSystem {
                    id: "vs-checkout".into(),
                    product: "prod".into(),
                    name: "Checkout Value Stream".into(),
                    kind: Some("value_stream".into()),
                    description: None,
                }],
                ..Default::default()
            }),
            links: Some(Links::default()),
            contracts: vec![],
            gates: vec![],
            roc: None,
            problem: None,
            hypotheses: vec![],
            decision: None,
            workflow: None,
            note_rationale: None,
            expires_at: None,
            auto_noted: false,
        };

        let yaml = serde_yaml::to_string(&spec).unwrap();
        let spec_filename = "feat.ss-test__20260318.s5d.yaml";
        let spec_path = project.s5d_dir().join("packages").join(spec_filename);
        std::fs::write(&spec_path, &yaml).unwrap();

        let (actions, _fp) = execute_import(&project, &spec_path, &spec, spec_filename).unwrap();

        assert!(
            actions
                .create
                .iter()
                .any(|s| s.starts_with("SuperSystem:vs-checkout")),
            "Expected SuperSystem:vs-checkout in created actions, got: {:?}",
            actions.create
        );

        let aliases = AliasTable::load(&project.s5d_dir()).unwrap();
        let has_ss = aliases
            .global
            .iter()
            .any(|e| e.artifact_type == "SuperSystem" && e.artifact_id == "vs-checkout")
            || aliases
                .packages
                .iter()
                .any(|e| e.artifact_type == "SuperSystem" && e.artifact_id == "vs-checkout");
        assert!(
            has_ss,
            "Expected SuperSystem alias for vs-checkout in alias table"
        );
    }

    #[test]
    fn test_note_spec_roundtrip() {
        let spec = Spec {
            s5d: "1.0".into(),
            id: "note.auth-approach".into(),
            version: "1.0.0".into(),
            product: "MyApp".into(),
            tier: Tier::Note,
            allow_update: false,
            meta: Some(Meta {
                title: "Auth Approach Note".into(),
                authors: vec!["roman".into()],
                date: Some("2026-03-18".into()),
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
            note_rationale: Some("Using JWT for stateless auth across services — no session store needed. Decision deferred from PROJ-100.".into()),
            expires_at: Some("2027-03-18".into()),
            auto_noted: false,
        };

        let errors = validate_spec(&spec);
        assert!(
            errors.is_empty(),
            "Note spec should be valid, got: {:?}",
            errors
        );

        let yaml = serde_yaml::to_string(&spec).unwrap();
        let loaded: Spec = serde_yaml::from_str(&yaml).unwrap();

        assert!(
            matches!(loaded.tier, Tier::Note),
            "Expected Note tier after roundtrip"
        );
        assert_eq!(
            loaded.note_rationale.as_deref(),
            Some("Using JWT for stateless auth across services — no session store needed. Decision deferred from PROJ-100.")
        );
        assert_eq!(loaded.expires_at.as_deref(), Some("2027-03-18"));
        assert!(loaded.artifacts.is_none());
    }

    #[test]
    fn test_feature_workflow_template_validates() {
        let mut spec = generate_spec("feat.workflow-template", Tier::Lightweight, "Prod");
        spec.artifacts
            .as_mut()
            .unwrap()
            .capabilities
            .push(Capability {
                id: "cap.prototype-loop".into(),
                domain: "dom.Prod.core".into(),
                name: "PrototypeLoop".into(),
                description: None,
                since: None,
            });

        let errors = validate_spec(&spec);
        assert!(
            errors.is_empty(),
            "feature template workflow should validate, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_invalid_workflow_mode_fails_validation() {
        let mut spec = generate_spec("feat.workflow-invalid", Tier::Lightweight, "Prod");
        spec.artifacts
            .as_mut()
            .unwrap()
            .capabilities
            .push(Capability {
                id: "cap.prototype-loop".into(),
                domain: "dom.workflow".into(),
                name: "PrototypeLoop".into(),
                description: None,
                since: None,
            });
        spec.workflow.as_mut().unwrap().mode = Some("bogus".into());

        let errors = validate_spec(&spec);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("workflow.mode: invalid value")),
            "expected workflow mode validation error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_state_fingerprint_is_stable_across_map_insertion_order() {
        let mut spec_a = generate_spec("feat.fingerprint", Tier::Standard, "Prod");
        let mut spec_b = spec_a.clone();

        let workflow_a = spec_a.workflow.as_mut().unwrap();
        workflow_a.role_map = HashMap::from([
            ("owner".into(), "human".into()),
            ("implementer".into(), "coder".into()),
            ("reviewer".into(), "reviewer".into()),
            ("verifier".into(), "reviewer".into()),
        ]);

        let workflow_b = spec_b.workflow.as_mut().unwrap();
        workflow_b.role_map = HashMap::from([
            ("verifier".into(), "reviewer".into()),
            ("reviewer".into(), "reviewer".into()),
            ("implementer".into(), "coder".into()),
            ("owner".into(), "human".into()),
        ]);

        let mut binding_a = HashMap::new();
        binding_a.insert("feature".into(), "feat.fingerprint".into());
        binding_a.insert("domain".into(), "dom.workflow".into());

        let mut binding_b = HashMap::new();
        binding_b.insert("domain".into(), "dom.workflow".into());
        binding_b.insert("feature".into(), "feat.fingerprint".into());

        spec_a
            .links
            .as_mut()
            .unwrap()
            .feature_to_domain
            .push(Binding { fields: binding_a });
        spec_b
            .links
            .as_mut()
            .unwrap()
            .feature_to_domain
            .push(Binding { fields: binding_b });

        let aliases = AliasTable::default();
        let fp_a = compute_state_fingerprint(&spec_a, &aliases);
        let fp_b = compute_state_fingerprint(&spec_b, &aliases);

        assert_eq!(fp_a, fp_b, "fingerprint should ignore HashMap order");
    }

    #[test]
    fn test_record_and_spec_roundtrip() {
        let dir = TempDir::new().unwrap();
        let (project, _) = S5dProject::init(dir.path()).unwrap();

        let spec = generate_spec("feat.roundtrip", Tier::Lightweight, "TestProd");
        let yaml = serde_yaml::to_string(&spec).unwrap();
        let spec_filename = "feat.roundtrip__20260101.s5d.yaml";
        let spec_path = project.s5d_dir().join("packages").join(spec_filename);
        std::fs::write(&spec_path, &yaml).unwrap();

        let sha = S5dProject::file_sha256(&spec_path).unwrap();
        assert!(sha.starts_with("sha256:"));

        let record = generate_record(spec_filename, &sha);
        project.save_record(spec_filename, &record).unwrap();

        let loaded = project.load_record(spec_filename).unwrap().unwrap();
        assert_eq!(loaded.spec_ref, spec_filename);
        assert_eq!(loaded.status, SpecStatus::Proposed);
    }

    #[test]
    fn test_domain_layering_valid() {
        let spec = build_notifications_spec();
        // notifications=Core, users=Supporting, edge notifications→users = valid (Core→Supporting ok)
        let errors = check_domain_layering(&spec);
        assert!(
            errors.is_empty(),
            "Expected no layering errors for Core→Supporting edge, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_domain_layering_violation() {
        let mut spec = generate_spec("feat.test.layer", Tier::Standard, "Prod");
        let arts = spec.artifacts.as_mut().unwrap();
        arts.domains = vec![
            Domain {
                id: "core-dom".into(),
                product: "Prod".into(),
                name: "Core".into(),
                classification: Some("core".into()),
                description: None,
                team: None,
                maturity_level: None,
            },
            Domain {
                id: "generic-dom".into(),
                product: "Prod".into(),
                name: "Generic".into(),
                classification: Some("generic".into()),
                description: None,
                team: None,
                maturity_level: None,
            },
        ];
        spec.links = Some(Links {
            edges: vec![Edge {
                from: "generic-dom".into(),
                to: "core-dom".into(),
                archetype: "conformist".into(),
                ..Default::default()
            }],
            ..Default::default()
        });
        let errors = check_domain_layering(&spec);
        assert!(!errors.is_empty(), "Expected layering violation");
        assert!(errors[0].contains("layering violation"));
    }

    #[test]
    fn test_transport_validation() {
        let mut spec = build_notifications_spec();
        let links = spec.links.as_mut().unwrap();
        links.edges[0].transport_ref = Some("rest-json".into());
        let errors = validate_spec(&spec);
        assert!(
            errors.is_empty(),
            "Valid transport should pass: {:?}",
            errors
        );
    }

    #[test]
    fn test_transport_invalid_type() {
        let mut spec = build_notifications_spec();
        let arts = spec.artifacts.as_mut().unwrap();
        arts.transports = vec![Transport {
            id: "bogus".into(),
            transport_type: "smoke_signal".into(),
            serialization: None,
            description: None,
        }];
        let errors = validate_spec(&spec);
        assert!(
            errors.iter().any(|e| e.contains("invalid transport type")),
            "Expected invalid transport type error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_transport_ref_missing() {
        let mut spec = build_notifications_spec();
        let links = spec.links.as_mut().unwrap();
        links.edges[0].transport_ref = Some("nonexistent".into());
        let errors = validate_spec(&spec);
        assert!(
            errors.iter().any(|e| e.contains("transport_ref")),
            "Expected transport_ref error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_evidence_fgr_roundtrip() {
        let mut spec = generate_decision_spec("dec.test", "Prod", "Test?");
        spec.hypotheses = vec![Hypothesis {
            id: "h1".into(),
            title: "Test".into(),
            content: "".into(),
            scope: "test".into(),
            kind: "system".into(),
            layer: "L2".into(),
            r_eff: Some(0.85),
            depends_on: vec![],
            rationale: None,
            spec_ref: None,
            evidence: vec![HypothesisEvidence {
                id: "ev1".into(),
                evidence_type: "internal".into(),
                content: "Benchmark: 50k RPS".into(),
                verdict: "pass".into(),
                valid_until: Some("2026-06-18".into()),
                carrier_ref: Some("bench.log".into()),
                formality: Some(7),
                claim_scope: vec!["throughput".into(), "latency".into()],
                congruence_level: Some(3),
                reliability: Some(0.85),
                refine_kind: None,
                skill: None,
                agent: None,
            }],
            prompt: None,
            next_move: None,
        }];
        let yaml = serde_yaml::to_string(&spec).unwrap();
        let parsed: Spec = serde_yaml::from_str(&yaml).unwrap();
        let ev = &parsed.hypotheses[0].evidence[0];
        assert_eq!(ev.formality, Some(7));
        assert_eq!(ev.claim_scope, vec!["throughput", "latency"]);
        assert_eq!(ev.congruence_level, Some(3));
    }
}
