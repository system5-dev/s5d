use crate::models::*;
use std::path::{Component as PathComponent, Path};

/// Scaffold placeholder for component source paths. `s5d new` emits it so a
/// fresh spec validates; import refuses it so the placeholder cannot become
/// recorded architecture on the default (non-architecture-gated) path.
pub const SCAFFOLD_PATH_TODO: &str = "TODO-set-source-paths/";

/// Components whose paths still carry the scaffold placeholder.
pub fn placeholder_path_components(spec: &crate::models::Spec) -> Vec<String> {
    spec.artifacts
        .as_ref()
        .map(|a| {
            a.components
                .iter()
                .filter(|c| c.paths.iter().any(|p| p.contains("TODO-set-source-paths")))
                .map(|c| c.id.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Readiness rules for an intake kernel (shape-layer.md): structural
/// completeness only — content truthfulness stays with the agent. Empty
/// result = ready. Used by `s5d shape` pre-spec and by validate_spec once
/// the kernel is embedded.
pub fn intent_kernel_readiness_errors(kernel: &IntentKernel) -> Vec<String> {
    let mut errors = Vec::new();
    if kernel.why.trim().is_empty() {
        errors
            .push("intent_kernel.why must be non-empty — name the reason this work matters".into());
    }
    if kernel.success_signal.trim().is_empty() {
        errors.push(
            "intent_kernel.success_signal must be non-empty — name the observable outcome that makes the work worth doing"
                .into(),
        );
    }
    let list_fields: [(&str, &Vec<String>); 6] = [
        ("capabilities", &kernel.capabilities),
        ("constraints", &kernel.constraints),
        ("non_goals", &kernel.non_goals),
        ("assumptions", &kernel.assumptions),
        ("open_questions", &kernel.open_questions),
        ("companions", &kernel.companions),
    ];
    for (name, values) in list_fields {
        if values.iter().any(|v| v.trim().is_empty()) {
            errors.push(format!(
                "intent_kernel.{} contains an empty entry — remove it or fill it in",
                name
            ));
        }
    }
    errors
}

pub fn validate_spec(spec: &Spec) -> Vec<String> {
    let mut errors = Vec::new();

    if spec.s5d != "1.0" {
        errors.push(format!("unsupported s5d version: {}", spec.s5d));
    }

    if let Some(ref kernel) = spec.intent_kernel {
        errors.extend(intent_kernel_readiness_errors(kernel));
    }

    if !is_valid_id(&spec.id) {
        errors.push(format!("invalid spec ID: {}", spec.id));
    }

    // For feature tiers, decision state lives in record only — spec.decision must be null
    // For decision tier, spec.decision is written by `s5d decide` (legacy compat)
    if spec.decision.is_some() && !matches!(spec.tier, Tier::Decision) {
        errors.push(
            "spec.decision must be null for feature tiers — decision state lives in .record.yaml"
                .into(),
        );
    }

    if matches!(spec.tier, Tier::Note) {
        if spec.note_rationale.as_ref().is_none_or(|r| r.is_empty()) {
            errors.push("Note tier requires note_rationale".into());
        }
        return errors;
    }

    if let Some(ref workflow) = spec.workflow {
        validate_workflow(workflow, &mut errors);
    }

    if let Some(ref mandate) = spec.mandate {
        validate_mandate(spec, mandate, &mut errors);
    }

    // Check for duplicate hypothesis IDs (applies to all tiers with hypotheses)
    {
        let mut seen_hyp_ids = std::collections::HashSet::new();
        for h in &spec.hypotheses {
            if h.id.trim().is_empty() {
                errors.push(
                    "hypothesis has an empty id (slug from an all-non-ASCII title); \
                     ids must be non-empty — the generator falls back to h-<hash>"
                        .into(),
                );
            }
            if !seen_hyp_ids.insert(&h.id) {
                errors.push(format!("duplicate hypothesis ID: {}", h.id));
            }
        }
    }

    // Gate-kind validity applies to every tier (Decision returns early below, so
    // this must run before the tier branches to cover decision specs too).
    let valid_gates = [
        "schema",
        "graph",
        "contract",
        "lint",
        "test",
        "typecheck",
        "policy",
        "architecture",
        "review",
    ];
    for gate in &spec.gates {
        if !valid_gates.contains(&gate.kind.as_str()) {
            errors.push(format!("invalid gate kind: {}", gate.kind));
        }
    }

    if let Tier::Decision = spec.tier {
        if spec.problem.as_ref().is_none_or(|p| p.signal().is_empty()) {
            errors.push("decision tier requires a non-empty 'problem.signal' field".into());
        }
        if spec.artifacts.is_some() {
            errors.push("decision tier must not have artifacts".into());
        }
        for h in &spec.hypotheses {
            // FPF B.5.2:13.3 — next_move enum (Decision-tier hypotheses only).
            if let Some(ref nm) = h.next_move {
                let allowed = ["deduction", "probe", "build", "defer"];
                if !allowed.contains(&nm.as_str()) {
                    errors.push(format!(
                        "hypothesis '{}' has invalid next_move '{}' (allowed: {})",
                        h.id,
                        nm,
                        allowed.join(", ")
                    ));
                }
            }
            for ev in &h.evidence {
                if let Some(f) = ev.formality {
                    if f > 9 {
                        errors.push(format!(
                            "evidence {}: formality {} out of range (0-9)",
                            ev.id, f
                        ));
                    }
                }
                if let Some(c) = ev.congruence_level {
                    if c > 3 {
                        errors.push(format!(
                            "evidence {}: congruence_level {} out of range (0-3)",
                            ev.id, c
                        ));
                    }
                }
                // FPF C.2:4.2 — a Δ-move kind belongs with verdict=refine. But the
                // recorded decision.fpf.hypothesis-record-extension (hypothesis
                // 'minimal-additions-only') deliberately deferred the Δ-moves enum and
                // accepted opaque refine evidence ("FPF compliance partial"). Hard-
                // erroring a MISSING refine_kind here would make the runtime stricter
                // than that recorded policy and retroactively invalidate existing
                // decision records — a policy change, not a bugfix. Full Δ-move adoption
                // must come through a superseding decision. So a missing refine_kind is
                // allowed; an INVALID one (below) is still rejected.
                // s5d:debt(ceiling="validate allows opaque verdict=refine evidence without refine_kind per minimal-additions-only decision", trigger="when Δ-moves are adopted by a superseding decision")
                if let Some(ref rk) = ev.refine_kind {
                    let allowed = [
                        "formalise",
                        "generalise",
                        "specialise",
                        "calibrate",
                        "validate",
                        "congrue",
                    ];
                    if !allowed.contains(&rk.as_str()) {
                        errors.push(format!(
                            "hypothesis '{}' evidence '{}' has invalid refine_kind '{}' (allowed: {})",
                            h.id,
                            ev.id,
                            rk,
                            allowed.join(", ")
                        ));
                    }
                }
            }
        }
        return errors;
    }

    // artifacts MUST exist for feature tiers — no draft bypass
    if spec.artifacts.is_none() {
        errors.push(format!("tier {:?} requires artifacts section", spec.tier));
        return errors;
    }

    if let Some(ref artifacts) = spec.artifacts {
        if artifacts.features.len() != 1 {
            errors.push(format!(
                "exactly 1 feature required, found {}",
                artifacts.features.len()
            ));
        } else if artifacts.features[0].id != spec.id {
            errors.push(format!(
                "single-feature invariant: spec.id ({}) != feature.id ({})",
                spec.id, artifacts.features[0].id
            ));
        }

        for p in &artifacts.products {
            validate_id(&p.id, "product", &mut errors);
        }
        for d in &artifacts.domains {
            validate_id(&d.id, "domain", &mut errors);
            // classification is REQUIRED
            match &d.classification {
                None => errors.push(format!(
                    "domain {}: classification is required (core, supporting, generic)",
                    d.id
                )),
                Some(cl) => {
                    let valid = ["core", "supporting", "generic"];
                    if !valid.contains(&cl.to_lowercase().as_str()) {
                        errors.push(format!("domain {}: invalid classification '{}' (expected: core, supporting, generic)", d.id, cl));
                    }
                }
            }
            if let Some(ref ml) = d.maturity_level {
                let valid = ["genesis", "custom", "product", "commodity"];
                if !valid.contains(&ml.to_lowercase().as_str()) {
                    errors.push(format!("domain {}: invalid maturity_level '{}' (expected: genesis, custom, product, commodity)", d.id, ml));
                }
            }
        }
        for c in &artifacts.capabilities {
            validate_id(&c.id, "capability", &mut errors);
        }
        for e in &artifacts.entities {
            validate_id(&e.id, "entity", &mut errors);
        }
        for f in &artifacts.features {
            validate_id(&f.id, "feature", &mut errors);
        }
        for u in &artifacts.use_cases {
            validate_id(&u.id, "use_case", &mut errors);
        }
        for s in &artifacts.systems {
            validate_id(&s.id, "system", &mut errors);
        }
        for c in &artifacts.containers {
            validate_id(&c.id, "container", &mut errors);
        }
        for c in &artifacts.components {
            validate_id(&c.id, "component", &mut errors);
            if c.paths.is_empty() {
                errors.push(format!(
                    "component {}: paths is required and must not be empty",
                    c.id
                ));
            }
            for path in &c.paths {
                validate_component_path(&c.id, path, &mut errors);
            }
        }
        for r in &artifacts.roles {
            validate_id(&r.id, "role", &mut errors);
        }
        for c in &artifacts.concerns {
            validate_id(&c.id, "concern", &mut errors);
        }
        for m in &artifacts.metrics {
            validate_id(&m.id, "metric", &mut errors);
        }
        for ss in &artifacts.supersystems {
            validate_id(&ss.id, "supersystem", &mut errors);
        }

        // Metamodel enforcement — always active, no draft bypass.
        // Each error carries the minimal shape so a cold user can fix it in
        // one edit instead of discovering the field cascade one error at a time.
        match spec.tier {
            Tier::Standard | Tier::High => {
                if artifacts.domains.is_empty() {
                    errors.push(
                        "metamodel: spec has no domains — required for standard/high tier\n    shape: domains: [{id, product, name, classification: core|supporting|generic}]".into(),
                    );
                }
                if artifacts.capabilities.is_empty() {
                    errors.push(
                        "metamodel: spec has no capabilities — required for standard/high tier\n    shape: capabilities: [{id, domain, name}]"
                            .into(),
                    );
                }
                if artifacts.components.is_empty() {
                    errors.push(
                        "metamodel: spec has no components — required for standard/high tier\n    shape: components: [{id, feature, domain, container, name, paths: [..]}] (container needs systems/containers entries: {id, product, name} / {id, system, name})"
                            .into(),
                    );
                }
            }
            Tier::Lightweight if artifacts.capabilities.is_empty() => {
                errors.push(
                    "metamodel: spec has no capabilities — required for lightweight tier\n    shape: capabilities: [{id, domain, name}] (plus a matching domains entry: {id, product, name, classification})".into(),
                );
            }
            _ => {}
        }

        // P0-3: Reference integrity — all refs must resolve to declared artifacts
        let domain_ids: std::collections::HashSet<&str> =
            artifacts.domains.iter().map(|d| d.id.as_str()).collect();
        let _capability_ids: std::collections::HashSet<&str> = artifacts
            .capabilities
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        let _entity_ids: std::collections::HashSet<&str> =
            artifacts.entities.iter().map(|e| e.id.as_str()).collect();
        let container_ids: std::collections::HashSet<&str> =
            artifacts.containers.iter().map(|c| c.id.as_str()).collect();
        let feature_ids: std::collections::HashSet<&str> =
            artifacts.features.iter().map(|f| f.id.as_str()).collect();

        // capability.domain → must be declared domain
        for cap in &artifacts.capabilities {
            if !domain_ids.contains(cap.domain.as_str()) && !domain_ids.is_empty() {
                errors.push(format!(
                    "capability {}: domain '{}' not declared in artifacts.domains",
                    cap.id, cap.domain
                ));
            }
        }
        // entity.domain → must be declared domain
        for ent in &artifacts.entities {
            if !domain_ids.contains(ent.domain.as_str()) && !domain_ids.is_empty() {
                errors.push(format!(
                    "entity {}: domain '{}' not declared in artifacts.domains",
                    ent.id, ent.domain
                ));
            }
        }
        // component.domain → must be declared domain
        for comp in &artifacts.components {
            if !domain_ids.contains(comp.domain.as_str()) && !domain_ids.is_empty() {
                errors.push(format!(
                    "component {}: domain '{}' not declared in artifacts.domains",
                    comp.id, comp.domain
                ));
            }
            // component.feature → must be declared feature
            if !feature_ids.contains(comp.feature.as_str()) {
                errors.push(format!(
                    "component {}: feature '{}' not declared in artifacts.features",
                    comp.id, comp.feature
                ));
            }
            // component.container → must be declared container (no domain fallback)
            if !container_ids.contains(comp.container.as_str()) {
                errors.push(format!(
                    "component {}: container '{}' not declared in artifacts.containers",
                    comp.id, comp.container
                ));
            }
        }

        let valid_transport_types = ["rest", "grpc", "messaging", "graphql", "websocket"];
        for t in &artifacts.transports {
            validate_id(&t.id, "transport", &mut errors);
            if !valid_transport_types.contains(&t.transport_type.to_lowercase().as_str()) {
                errors.push(format!(
                    "transport {}: invalid transport type '{}' (expected one of: {})",
                    t.id,
                    t.transport_type,
                    valid_transport_types.join(", ")
                ));
            }
        }

        // container.system → must resolve to declared system (skip if no systems declared)
        let system_ids: std::collections::HashSet<&str> =
            artifacts.systems.iter().map(|s| s.id.as_str()).collect();
        for ctr in &artifacts.containers {
            if !system_ids.is_empty() && !system_ids.contains(ctr.system.as_str()) {
                errors.push(format!(
                    "container {}: system '{}' not declared in artifacts.systems",
                    ctr.id, ctr.system
                ));
            }
        }

        // feature.product → must match spec.product
        for f in &artifacts.features {
            if f.product != spec.product {
                errors.push(format!(
                    "feature {}: product '{}' doesn't match spec product '{}'",
                    f.id, f.product, spec.product
                ));
            }
        }

        // use_case.feature → must resolve to declared feature
        let uc_feature_ids: std::collections::HashSet<&str> =
            artifacts.features.iter().map(|f| f.id.as_str()).collect();
        for uc in &artifacts.use_cases {
            if !uc_feature_ids.contains(uc.feature.as_str()) {
                errors.push(format!(
                    "use_case {}: feature '{}' not declared",
                    uc.id, uc.feature
                ));
            }
        }

        let supersystem_ids: std::collections::HashSet<&str> = artifacts
            .supersystems
            .iter()
            .map(|ss| ss.id.as_str())
            .collect();
        for c in &artifacts.concerns {
            if let Some(ref ss_ref) = c.supersystem {
                if !supersystem_ids.contains(ss_ref.as_str()) {
                    errors.push(format!(
                        "concern {}: supersystem '{}' not declared in artifacts.supersystems",
                        c.id, ss_ref
                    ));
                }
            }
        }
        for m in &artifacts.metrics {
            if let Some(ref ss_ref) = m.supersystem {
                if !supersystem_ids.contains(ss_ref.as_str()) {
                    errors.push(format!(
                        "metric {}: supersystem '{}' not declared in artifacts.supersystems",
                        m.id, ss_ref
                    ));
                }
            }
        }
    }

    validate_component_capability_traceability(spec, &mut errors);

    if let Some(ref links) = spec.links {
        let transport_ids: std::collections::HashSet<&str> = spec
            .artifacts
            .as_ref()
            .map(|a| a.transports.iter().map(|t| t.id.as_str()).collect())
            .unwrap_or_default();
        let link_domain_ids: std::collections::HashSet<&str> = spec
            .artifacts
            .as_ref()
            .map(|a| a.domains.iter().map(|d| d.id.as_str()).collect())
            .unwrap_or_default();

        // Build typed ID sets for link validation
        let link_feature_ids: std::collections::HashSet<&str> = spec
            .artifacts
            .as_ref()
            .map(|a| a.features.iter().map(|f| f.id.as_str()).collect())
            .unwrap_or_default();
        let link_capability_ids: std::collections::HashSet<&str> = spec
            .artifacts
            .as_ref()
            .map(|a| a.capabilities.iter().map(|c| c.id.as_str()).collect())
            .unwrap_or_default();
        let link_entity_ids: std::collections::HashSet<&str> = spec
            .artifacts
            .as_ref()
            .map(|a| a.entities.iter().map(|e| e.id.as_str()).collect())
            .unwrap_or_default();
        let link_component_ids: std::collections::HashSet<&str> = spec
            .artifacts
            .as_ref()
            .map(|a| a.components.iter().map(|c| c.id.as_str()).collect())
            .unwrap_or_default();
        let link_container_ids: std::collections::HashSet<&str> = spec
            .artifacts
            .as_ref()
            .map(|a| a.containers.iter().map(|c| c.id.as_str()).collect())
            .unwrap_or_default();
        let link_concern_ids: std::collections::HashSet<&str> = spec
            .artifacts
            .as_ref()
            .map(|a| a.concerns.iter().map(|c| c.id.as_str()).collect())
            .unwrap_or_default();
        let link_metric_ids: std::collections::HashSet<&str> = spec
            .artifacts
            .as_ref()
            .map(|a| a.metrics.iter().map(|m| m.id.as_str()).collect())
            .unwrap_or_default();
        let link_usecase_ids: std::collections::HashSet<&str> = spec
            .artifacts
            .as_ref()
            .map(|a| a.use_cases.iter().map(|u| u.id.as_str()).collect())
            .unwrap_or_default();

        // Typed link validation — each binding must reference the correct artifact type
        #[allow(clippy::too_many_arguments)]
        fn validate_typed_binding(
            bindings: &[crate::models::Binding],
            link_name: &str,
            left_key: &str,
            left_ids: &std::collections::HashSet<&str>,
            left_type: &str,
            right_key: &str,
            right_ids: &std::collections::HashSet<&str>,
            right_type: &str,
            errors: &mut Vec<String>,
        ) {
            for b in bindings {
                match (b.fields.get(left_key), b.fields.get(right_key)) {
                    (Some(left_val), Some(right_val)) => {
                        if !left_ids.is_empty() && !left_ids.contains(left_val.as_str()) {
                            errors.push(format!(
                                "{}: '{}' = '{}' is not a declared {}",
                                link_name, left_key, left_val, left_type
                            ));
                        }
                        if !right_ids.is_empty() && !right_ids.contains(right_val.as_str()) {
                            errors.push(format!(
                                "{}: '{}' = '{}' is not a declared {}",
                                link_name, right_key, right_val, right_type
                            ));
                        }
                    }
                    (None, _) => errors.push(format!(
                        "{}: missing required field '{}'",
                        link_name, left_key
                    )),
                    (_, None) => errors.push(format!(
                        "{}: missing required field '{}'",
                        link_name, right_key
                    )),
                }
            }
        }

        validate_typed_binding(
            &links.feature_to_domain,
            "feature_to_domain",
            "feature",
            &link_feature_ids,
            "feature",
            "domain",
            &link_domain_ids,
            "domain",
            &mut errors,
        );
        validate_typed_binding(
            &links.use_case_to_capability,
            "use_case_to_capability",
            "use_case",
            &link_usecase_ids,
            "use_case",
            "capability",
            &link_capability_ids,
            "capability",
            &mut errors,
        );
        validate_typed_binding(
            &links.use_case_to_entity,
            "use_case_to_entity",
            "use_case",
            &link_usecase_ids,
            "use_case",
            "entity",
            &link_entity_ids,
            "entity",
            &mut errors,
        );
        validate_typed_binding(
            &links.component_to_capability,
            "component_to_capability",
            "component",
            &link_component_ids,
            "component",
            "capability",
            &link_capability_ids,
            "capability",
            &mut errors,
        );
        validate_typed_binding(
            &links.component_to_entity,
            "component_to_entity",
            "component",
            &link_component_ids,
            "component",
            "entity",
            &link_entity_ids,
            "entity",
            &mut errors,
        );
        validate_typed_binding(
            &links.component_to_container,
            "component_to_container",
            "component",
            &link_component_ids,
            "component",
            "container",
            &link_container_ids,
            "container",
            &mut errors,
        );
        validate_typed_binding(
            &links.container_to_capability,
            "container_to_capability",
            "container",
            &link_container_ids,
            "container",
            "capability",
            &link_capability_ids,
            "capability",
            &mut errors,
        );
        validate_typed_binding(
            &links.concern_to_metric,
            "concern_to_metric",
            "concern",
            &link_concern_ids,
            "concern",
            "metric",
            &link_metric_ids,
            "metric",
            &mut errors,
        );

        // Semantic: component and capability must be in same domain
        if let Some(ref arts) = spec.artifacts {
            let comp_domain: std::collections::HashMap<&str, &str> = arts
                .components
                .iter()
                .map(|c| (c.id.as_str(), c.domain.as_str()))
                .collect();
            let cap_domain: std::collections::HashMap<&str, &str> = arts
                .capabilities
                .iter()
                .map(|c| (c.id.as_str(), c.domain.as_str()))
                .collect();
            for b in &links.component_to_capability {
                if let (Some(comp_id), Some(cap_id)) =
                    (b.fields.get("component"), b.fields.get("capability"))
                {
                    if let (Some(&cd), Some(&capd)) = (
                        comp_domain.get(comp_id.as_str()),
                        cap_domain.get(cap_id.as_str()),
                    ) {
                        if cd != capd {
                            errors.push(format!(
                                "component_to_capability: component '{}' (domain {}) must match capability '{}' (domain {})",
                                comp_id, cd, cap_id, capd
                            ));
                        }
                    }
                }
            }
        }

        let valid_archetypes = [
            "ohs",
            "customer_supplier",
            "conformist",
            "acl",
            "published_language",
            "shared_kernel",
            "partnership",
            "separate_ways",
        ];
        for edge in &links.edges {
            if !valid_archetypes.contains(&edge.archetype.as_str()) {
                errors.push(format!("invalid edge archetype: {}", edge.archetype));
            }
            // P0-3: edge.from/to must reference declared domains
            if !link_domain_ids.contains(edge.from.as_str()) && !link_domain_ids.is_empty() {
                errors.push(format!(
                    "edge {}→{}: 'from' domain '{}' not declared in artifacts.domains",
                    edge.from, edge.to, edge.from
                ));
            }
            if !link_domain_ids.contains(edge.to.as_str()) && !link_domain_ids.is_empty() {
                errors.push(format!(
                    "edge {}→{}: 'to' domain '{}' not declared in artifacts.domains",
                    edge.from, edge.to, edge.to
                ));
            }
            if let Some(ref tref) = edge.transport_ref {
                if !transport_ids.contains(tref.as_str()) {
                    errors.push(format!(
                        "edge {}→{}: transport_ref '{}' not declared in artifacts.transports",
                        edge.from, edge.to, tref
                    ));
                }
            }
        }

        let valid_cardinalities = ["1:1", "1:N", "M:N", "1:0..1", "1:0..N", "N:0..N"];
        for er in &links.entity_relations {
            if let Some(ref card) = er.cardinality {
                if !valid_cardinalities.contains(&card.as_str()) {
                    errors.push(format!(
                        "entity_relation {}→{}: invalid cardinality '{}' (expected one of: {})",
                        er.entity,
                        er.related_entity,
                        card,
                        valid_cardinalities.join(", ")
                    ));
                }
            }
        }
    }

    let valid_contract_formats = ["openapi", "json_schema", "protobuf", "typespec"];
    for contract in &spec.contracts {
        // format must be valid enum
        if !valid_contract_formats.contains(&contract.format.to_lowercase().as_str()) {
            errors.push(format!(
                "contract {}: invalid format '{}' (expected: {})",
                contract.id,
                contract.format,
                valid_contract_formats.join(", ")
            ));
        }
        // must have path or inline (at least one)
        if contract.path.is_none() && contract.inline.is_none() {
            errors.push(format!(
                "contract {}: must have 'path' or 'inline' — contract without content is empty",
                contract.id
            ));
        }
        // path requires sha256
        if contract.path.is_some() && contract.sha256.is_none() {
            errors.push(format!(
                "contract {}: has path but missing sha256 — integrity check required",
                contract.id
            ));
        }
        // binds_to should not be empty
        if contract.binds_to.is_empty() {
            errors.push(format!(
                "contract {}: binds_to is empty — contract must bind to at least one capability or entity",
                contract.id
            ));
        }
    }

    if let Tier::High = spec.tier {
        if spec
            .context
            .as_ref()
            .is_none_or(|c| !c.to_lowercase().contains("privacy"))
        {
            errors.push("high tier requires privacy note in context".into());
        }
    }

    errors
}

fn validate_component_path(component_id: &str, path: &str, errors: &mut Vec<String>) {
    let invalid = path.is_empty()
        || path.contains('\0')
        || Path::new(path).is_absolute()
        || Path::new(path).components().any(|component| {
            matches!(
                component,
                PathComponent::ParentDir | PathComponent::RootDir | PathComponent::Prefix(_)
            )
        });

    if invalid {
        errors.push(format!(
            "component {component_id}: path '{path}' must be a relative source path under the project root (no absolute paths, '..', or null bytes)"
        ));
    }
}

fn validate_component_capability_traceability(spec: &Spec, errors: &mut Vec<String>) {
    let Some(artifacts) = spec.artifacts.as_ref() else {
        return;
    };
    if artifacts.components.is_empty() || artifacts.capabilities.is_empty() {
        return;
    }

    let linked_components: std::collections::HashSet<&str> = spec
        .links
        .as_ref()
        .map(|links| {
            links
                .component_to_capability
                .iter()
                .filter_map(|binding| binding.fields.get("component").map(String::as_str))
                .collect()
        })
        .unwrap_or_default();

    for component in &artifacts.components {
        if !linked_components.contains(component.id.as_str()) {
            errors.push(format!(
                "architecture: component '{}' is not linked to any capability via links.component_to_capability — code cannot trace to target state",
                component.id
            ));
        }
    }
}

fn validate_workflow(workflow: &Workflow, errors: &mut Vec<String>) {
    if let Some(ref mode) = workflow.mode {
        let valid = ["research", "plan", "implement", "operate"];
        if !valid.contains(&mode.to_lowercase().as_str()) {
            errors.push(format!(
                "workflow.mode: invalid value '{}' (expected: research, plan, implement, operate)",
                mode
            ));
        }
    }

    if let Some(ref target) = workflow.target_architecture {
        if target.summary.trim().is_empty() {
            errors.push("workflow.target_architecture.summary must not be empty".into());
        }
    }

    if let Some(ref strategy) = workflow.delivery_strategy {
        if strategy.summary.trim().is_empty() {
            errors.push("workflow.delivery_strategy.summary must not be empty".into());
        }
    }

    for (role, owner) in &workflow.role_map {
        if role.trim().is_empty() {
            errors.push("workflow.role_map contains an empty role key".into());
        }
        if owner.trim().is_empty() {
            errors.push(format!(
                "workflow.role_map.{} must not map to an empty owner",
                role
            ));
        }
    }

    if let Some(ref execution_mode) = workflow.execution_mode {
        let valid = ["manual", "ralph"];
        if !valid.contains(&execution_mode.engine.to_lowercase().as_str()) {
            errors.push(format!(
                "workflow.execution_mode.engine: invalid value '{}' (expected: manual, ralph)",
                execution_mode.engine
            ));
        }
        if execution_mode.stop_conditions.is_empty() {
            errors.push("workflow.execution_mode.stop_conditions must not be empty".into());
        }
    }

    let mut seen_phase_ids = std::collections::HashSet::new();
    for phase in &workflow.phases {
        if phase.id.trim().is_empty() {
            errors.push("workflow.phases[].id must not be empty".into());
        } else if !seen_phase_ids.insert(phase.id.as_str()) {
            errors.push(format!("duplicate workflow phase ID: {}", phase.id));
        }
        if phase.title.trim().is_empty() {
            errors.push(format!(
                "workflow phase {}: title must not be empty",
                phase.id
            ));
        }
        if phase.scope.trim().is_empty() {
            errors.push(format!(
                "workflow phase {}: scope must not be empty",
                phase.id
            ));
        }
        if phase.roles.is_empty() {
            errors.push(format!(
                "workflow phase {}: roles must not be empty",
                phase.id
            ));
        }
        if phase.acceptance.is_empty() {
            errors.push(format!(
                "workflow phase {}: acceptance must not be empty",
                phase.id
            ));
        }
        if phase.rollback.is_empty() {
            errors.push(format!(
                "workflow phase {}: rollback must not be empty",
                phase.id
            ));
        }
    }
}

fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
}

fn validate_id(id: &str, kind: &str, errors: &mut Vec<String>) {
    if !is_valid_id(id) {
        errors.push(format!("invalid {} ID: {}", kind, id));
    }
}

/// Autonomous-loop envelope validation (decision.s5d.autonomous-loop-mandate).
/// Enforces the challenge constraints at the schema gate:
///   (b) min_gate_floor must be declared in spec.gates,
///   (c) high/decision blast-radius work stays human-gated (no mandate),
///   plus: non-empty scope and a bounded budget (no unbounded autonomy).
/// Constraint (a) — drift is always a stop-condition — needs no flag; the loop
/// driver (`mandate::adjudicate_mandate`) escalates on any detected drift.
fn validate_mandate(spec: &Spec, m: &crate::models::Mandate, errors: &mut Vec<String>) {
    if matches!(spec.tier, Tier::High | Tier::Decision) {
        errors.push(format!(
            "mandate not allowed on {} tier — high/decision work stays human-gated per action",
            spec.tier
        ));
    }
    if m.scope.trim().is_empty() {
        errors.push(
            "mandate.scope must be non-empty — an unscoped autonomous loop is rejected".into(),
        );
    }
    if m.budget.max_calls.is_none() && m.budget.max_time_s.is_none() {
        errors.push(
            "mandate.budget must bound the loop (set max_calls and/or max_time_s) — unbounded autonomy is rejected".into(),
        );
    }
    let gate_kinds: std::collections::HashSet<&str> =
        spec.gates.iter().map(|g| g.kind.as_str()).collect();
    for g in &m.min_gate_floor {
        if !gate_kinds.contains(g.as_str()) {
            errors.push(format!(
                "mandate.min_gate_floor gate '{}' not declared in spec.gates",
                g
            ));
        }
    }
}

#[cfg(test)]
mod mandate_tests {
    use super::validate_spec;
    use crate::models::Spec;

    fn errs(yaml: &str) -> Vec<String> {
        let spec: Spec = serde_yaml::from_str(yaml).expect("spec parses");
        validate_spec(&spec)
            .into_iter()
            .filter(|e| e.contains("mandate"))
            .collect()
    }

    const BASE: &str = r#"
s5d: "1.0"
id: feat.x.y
version: "1.0.0"
product: x
tier: standard
gates:
  - kind: test
  - kind: review
"#;

    #[test]
    fn valid_mandate_has_no_mandate_errors() {
        let y = format!(
            "{BASE}mandate:\n  scope: src/x\n  budget:\n    max_calls: 50\n  min_gate_floor: [test]\n"
        );
        assert!(errs(&y).is_empty(), "unexpected: {:?}", errs(&y));
    }

    #[test]
    fn mandate_on_high_tier_rejected() {
        let y = BASE.replace("tier: standard", "tier: high")
            + "mandate:\n  scope: src/x\n  budget:\n    max_calls: 5\n";
        assert!(errs(&y)
            .iter()
            .any(|e| e.contains("not allowed on high tier")));
    }

    #[test]
    fn unbounded_budget_rejected() {
        let y = format!("{BASE}mandate:\n  scope: src/x\n  budget: {{}}\n");
        assert!(errs(&y).iter().any(|e| e.contains("unbounded autonomy")));
    }

    #[test]
    fn gate_floor_must_be_declared() {
        let y = format!(
            "{BASE}mandate:\n  scope: src/x\n  budget:\n    max_calls: 5\n  min_gate_floor: [contract]\n"
        );
        assert!(errs(&y)
            .iter()
            .any(|e| e.contains("not declared in spec.gates")));
    }
}

#[cfg(test)]
mod decision_validation_tests {
    //! Regression lock for the decision-tier early-return bug: the Decision
    //! branch returned before the next_move / refine_kind / gate-kind checks, so
    //! they were dead code for the only tier they applied to. These exercise the
    //! checks now that they run inside the Decision branch.
    use super::validate_spec;
    use crate::models::{Gate, Hypothesis, HypothesisEvidence, Spec};

    fn decision_spec() -> Spec {
        crate::generate_decision_spec("decision.x", "x", "why are we here?")
    }

    fn ev(verdict: &str, refine_kind: Option<&str>) -> HypothesisEvidence {
        HypothesisEvidence {
            id: "e1".into(),
            evidence_type: "internal".into(),
            content: "x".into(),
            verdict: verdict.into(),
            valid_until: None,
            carrier_ref: None,
            formality: None,
            claim_scope: vec![],
            congruence_level: None,
            reliability: None,
            refine_kind: refine_kind.map(Into::into),
            skill: None,
            agent: None,
        }
    }

    fn hyp(id: &str, evidence: Vec<HypothesisEvidence>, next_move: Option<&str>) -> Hypothesis {
        Hypothesis {
            id: id.into(),
            title: "t".into(),
            content: "c".into(),
            scope: "s".into(),
            kind: String::new(),
            layer: String::new(),
            r_eff: None,
            evidence,
            depends_on: vec![],
            rationale: None,
            spec_ref: None,
            prompt: None,
            next_move: next_move.map(Into::into),
        }
    }

    #[test]
    fn baseline_decision_scaffold_is_clean() {
        let errs = validate_spec(&decision_spec());
        assert!(errs.is_empty(), "scaffold should validate: {errs:?}");
    }

    #[test]
    fn refine_verdict_without_refine_kind_is_allowed_by_recorded_policy() {
        // decision.fpf.hypothesis-record-extension (hypothesis minimal-additions-only)
        // deliberately deferred the Δ-moves enum and accepted opaque refine evidence.
        // A missing refine_kind must therefore NOT be a hard error — enforcing it
        // would make the runtime stricter than recorded policy and invalidate
        // existing decision records. (Full Δ-move adoption = a superseding decision.)
        let mut spec = decision_spec();
        spec.hypotheses
            .push(hyp("h1", vec![ev("refine", None)], None));
        let errs = validate_spec(&spec);
        assert!(
            !errs.iter().any(|e| e.contains("refine_kind")),
            "missing refine_kind must be allowed per recorded policy: {errs:?}"
        );
    }

    #[test]
    fn valid_refine_kind_is_accepted() {
        let mut spec = decision_spec();
        spec.hypotheses
            .push(hyp("h1", vec![ev("refine", Some("calibrate"))], None));
        spec.hypotheses
            .push(hyp("h2", vec![ev("pass", None)], None));
        let errs = validate_spec(&spec);
        assert!(errs.is_empty(), "{errs:?}");
    }

    #[test]
    fn invalid_refine_kind_is_rejected_when_present() {
        // The boundary the recorded policy draws: a MISSING refine_kind is allowed,
        // but a PRESENT-but-garbage one is still rejected — opaque is fine, wrong is not.
        let mut spec = decision_spec();
        spec.hypotheses
            .push(hyp("h1", vec![ev("refine", Some("bogus"))], None));
        let errs = validate_spec(&spec);
        assert!(
            errs.iter().any(|e| e.contains("invalid refine_kind")),
            "{errs:?}"
        );
    }

    #[test]
    fn invalid_next_move_and_gate_kind_rejected_on_decision() {
        let mut spec = decision_spec();
        spec.gates.push(Gate {
            kind: "bogus".into(),
        });
        spec.hypotheses.push(hyp("h1", vec![], Some("teleport")));
        let errs = validate_spec(&spec);
        assert!(
            errs.iter().any(|e| e.contains("invalid next_move")),
            "{errs:?}"
        );
        assert!(
            errs.iter().any(|e| e.contains("invalid gate kind")),
            "{errs:?}"
        );
    }

    #[test]
    fn empty_hypothesis_id_is_rejected() {
        let mut spec = decision_spec();
        spec.hypotheses.push(hyp("", vec![], None));
        let errs = validate_spec(&spec);
        assert!(errs.iter().any(|e| e.contains("empty id")), "{errs:?}");
    }
}
