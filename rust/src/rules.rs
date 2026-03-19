use crate::models::*;
use std::collections::HashMap;

/// An architecture rule derived from an S5D spec.
#[derive(Debug, Clone)]
pub struct ArchRule {
    pub id: String,
    pub kind: RuleKind,
    pub description: String,
    pub source: String,      // which spec artifact this came from
    pub severity: String,    // error, warning
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuleKind {
    /// Domain A must not depend on Domain B
    ForbiddenDependency { from: String, to: String },
    /// Only these edges are allowed between domains
    AllowedEdges { from: String, allowed_targets: Vec<String> },
    /// No cycles between these domains
    NoCycle { domains: Vec<String> },
    /// Domain layering: lower must not import higher
    Layering { lower: String, higher: String, lower_level: String, higher_level: String },
}

/// Generate architecture rules from an S5D spec.
pub fn generate_rules(spec: &Spec) -> Vec<ArchRule> {
    let mut rules = Vec::new();

    let arts = match &spec.artifacts {
        Some(a) => a,
        None => return rules,
    };

    let links = spec.links.as_ref();

    // Collect domain classifications
    let domain_classes: HashMap<&str, &str> = arts
        .domains
        .iter()
        .filter_map(|d| d.classification.as_deref().map(|c| (d.id.as_str(), c)))
        .collect();

    let domain_ids: Vec<&str> = arts.domains.iter().map(|d| d.id.as_str()).collect();

    // Rule 1: Layering — lower cannot depend on higher
    rules.extend(generate_layering_rules(&domain_classes));

    // Rule 2: Allowed edges — only declared edges are legal cross-domain deps
    if let Some(l) = links {
        rules.extend(generate_allowed_edge_rules(&domain_ids, &l.edges));
    }

    // Rule 3: No cycles — based on domain graph
    rules.push(ArchRule {
        id: format!("{}/no-cycles", spec.id),
        kind: RuleKind::NoCycle {
            domains: domain_ids.iter().map(|s| s.to_string()).collect(),
        },
        description: "No circular dependencies between domains".into(),
        source: "spec.links.edges".into(),
        severity: "error".into(),
    });

    rules
}

fn level(classification: &str) -> u8 {
    match classification.to_lowercase().as_str() {
        "core" => 3,
        "supporting" => 2,
        "generic" => 1,
        _ => 0,
    }
}

/// Generate layering rules: for every pair where lower-level domain exists,
/// forbid dependency from lower to higher.
fn generate_layering_rules(domain_classes: &HashMap<&str, &str>) -> Vec<ArchRule> {
    let mut rules = Vec::new();

    let domains: Vec<(&&str, &&str)> = domain_classes.iter().collect();
    for (i, (id_a, class_a)) in domains.iter().enumerate() {
        for (id_b, class_b) in domains.iter().skip(i + 1) {
            let level_a = level(class_a);
            let level_b = level(class_b);

            if level_a > 0 && level_b > 0 && level_a != level_b {
                let (lower_id, lower_class, higher_id, higher_class) = if level_a < level_b {
                    (**id_a, **class_a, **id_b, **class_b)
                } else {
                    (**id_b, **class_b, **id_a, **class_a)
                };

                rules.push(ArchRule {
                    id: format!("layering/{}-!->{}", lower_id, higher_id),
                    kind: RuleKind::Layering {
                        lower: lower_id.to_string(),
                        higher: higher_id.to_string(),
                        lower_level: lower_class.to_string(),
                        higher_level: higher_class.to_string(),
                    },
                    description: format!(
                        "{} ({}) must not depend on {} ({})",
                        lower_id, lower_class, higher_id, higher_class
                    ),
                    source: "domain.classification".into(),
                    severity: "error".into(),
                });
            }
        }
    }

    rules
}

/// Generate allowed-edge rules: each domain can only depend on domains
/// explicitly listed in spec.links.edges.
fn generate_allowed_edge_rules(domain_ids: &[&str], edges: &[Edge]) -> Vec<ArchRule> {
    let mut rules = Vec::new();

    // Build allowed targets per domain
    let mut allowed: HashMap<&str, Vec<String>> = HashMap::new();
    for edge in edges {
        allowed
            .entry(edge.from.as_str())
            .or_default()
            .push(edge.to.clone());
    }

    // For each domain, the allowed targets are explicitly declared edges.
    // Any other cross-domain dependency is forbidden.
    for &domain in domain_ids {
        let targets = allowed
            .get(domain)
            .cloned()
            .unwrap_or_default();

        rules.push(ArchRule {
            id: format!("edges/{}", domain),
            kind: RuleKind::AllowedEdges {
                from: domain.to_string(),
                allowed_targets: targets.clone(),
            },
            description: if targets.is_empty() {
                format!("{} must not have cross-domain dependencies", domain)
            } else {
                format!(
                    "{} may only depend on: {}",
                    domain,
                    targets.join(", ")
                )
            },
            source: "spec.links.edges".into(),
            severity: "warning".into(),
        });
    }

    rules
}

/// Format rules as YAML for S5D-native consumption.
pub fn format_yaml(rules: &[ArchRule], spec_id: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Architecture rules generated from {}\n", spec_id));
    out.push_str(&format!("spec_id: {}\n", spec_id));
    out.push_str("rules:\n");

    for rule in rules {
        out.push_str(&format!("  - id: {}\n", rule.id));
        out.push_str(&format!("    severity: {}\n", rule.severity));
        out.push_str(&format!("    description: {:?}\n", rule.description));
        match &rule.kind {
            RuleKind::ForbiddenDependency { from, to } => {
                out.push_str("    type: forbidden_dependency\n");
                out.push_str(&format!("    from: {}\n", from));
                out.push_str(&format!("    to: {}\n", to));
            }
            RuleKind::AllowedEdges { from, allowed_targets } => {
                out.push_str("    type: allowed_edges\n");
                out.push_str(&format!("    from: {}\n", from));
                out.push_str("    allowed:\n");
                for t in allowed_targets {
                    out.push_str(&format!("      - {}\n", t));
                }
            }
            RuleKind::NoCycle { domains } => {
                out.push_str("    type: no_cycle\n");
                out.push_str("    domains:\n");
                for d in domains {
                    out.push_str(&format!("      - {}\n", d));
                }
            }
            RuleKind::Layering { lower, higher, lower_level, higher_level } => {
                out.push_str("    type: layering\n");
                out.push_str(&format!("    lower: {} # {}\n", lower, lower_level));
                out.push_str(&format!("    higher: {} # {}\n", higher, higher_level));
            }
        }
    }

    out
}

/// Format rules as JSON for generic consumption.
pub fn format_json(rules: &[ArchRule], spec_id: &str) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"spec_id\": {:?},\n", spec_id));
    out.push_str("  \"rules\": [\n");

    for (i, rule) in rules.iter().enumerate() {
        let comma = if i + 1 < rules.len() { "," } else { "" };
        let kind_str = match &rule.kind {
            RuleKind::ForbiddenDependency { from, to } => {
                format!("\"type\": \"forbidden_dependency\", \"from\": {:?}, \"to\": {:?}", from, to)
            }
            RuleKind::AllowedEdges { from, allowed_targets } => {
                let targets: Vec<String> = allowed_targets.iter().map(|t| format!("{:?}", t)).collect();
                format!("\"type\": \"allowed_edges\", \"from\": {:?}, \"allowed\": [{}]", from, targets.join(", "))
            }
            RuleKind::NoCycle { domains } => {
                let ds: Vec<String> = domains.iter().map(|d| format!("{:?}", d)).collect();
                format!("\"type\": \"no_cycle\", \"domains\": [{}]", ds.join(", "))
            }
            RuleKind::Layering { lower, higher, lower_level, higher_level } => {
                format!(
                    "\"type\": \"layering\", \"lower\": {:?}, \"higher\": {:?}, \"lower_level\": {:?}, \"higher_level\": {:?}",
                    lower, higher, lower_level, higher_level
                )
            }
        };
        out.push_str(&format!(
            "    {{\"id\": {:?}, \"severity\": {:?}, \"description\": {:?}, {}}}{}\n",
            rule.id, rule.severity, rule.description, kind_str, comma
        ));
    }

    out.push_str("  ]\n");
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_spec_with_domains() -> Spec {
        Spec {
            s5d: "1.0".into(),
            id: "test-spec".into(),
            version: "1.0.0".into(),
            product: "TestProduct".into(),
            tier: Tier::Standard,
            allow_update: false,
            meta: None,
            context: None,
            artifacts: Some(Artifacts {
                products: vec![],
                domains: vec![
                    Domain {
                        id: "billing".into(),
                        product: "test".into(),
                        name: "Billing".into(),
                        classification: Some("core".into()),
                        description: None,
                        team: None,
                        maturity_level: None,
                    },
                    Domain {
                        id: "auth".into(),
                        product: "test".into(),
                        name: "Auth".into(),
                        classification: Some("supporting".into()),
                        description: None,
                        team: None,
                        maturity_level: None,
                    },
                    Domain {
                        id: "utils".into(),
                        product: "test".into(),
                        name: "Utils".into(),
                        classification: Some("generic".into()),
                        description: None,
                        team: None,
                        maturity_level: None,
                    },
                ],
                capabilities: vec![],
                entities: vec![],
                features: vec![],
                use_cases: vec![],
                systems: vec![],
                containers: vec![],
                components: vec![],
                roles: vec![],
                concerns: vec![],
                metrics: vec![],
                supersystems: vec![],
                transports: vec![],
            }),
            links: Some(Links {
                edges: vec![
                    Edge {
                        from: "auth".into(),
                        to: "billing".into(),
                        archetype: "customer_supplier".into(),
                        ..Default::default()
                    },
                ],
                depends_on: vec![],
                feature_to_domain: vec![],
                use_case_to_capability: vec![],
                use_case_to_entity: vec![],
                component_to_capability: vec![],
                component_to_entity: vec![],
                component_to_container: vec![],
                container_to_capability: vec![],
                capability_to_entity: vec![],
                capability_to_concern: vec![],
                concern_to_metric: vec![],
                entity_relations: vec![],
            }),
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

    #[test]
    fn test_generate_layering_rules() {
        let spec = make_spec_with_domains();
        let rules = generate_rules(&spec);

        let layering: Vec<&ArchRule> = rules
            .iter()
            .filter(|r| matches!(&r.kind, RuleKind::Layering { .. }))
            .collect();

        // utils(generic) !-> billing(core), utils !-> auth(supporting), auth !-> billing
        // But auth(supporting) -> billing(core) IS allowed (higher can depend on lower)
        // Forbidden: generic->core, generic->supporting
        assert!(
            layering.len() >= 2,
            "expected at least 2 layering rules, got {}",
            layering.len()
        );

        // Check utils cannot depend on billing (generic !-> core)
        assert!(layering.iter().any(|r| {
            if let RuleKind::Layering { lower, higher, .. } = &r.kind {
                lower == "utils" && higher == "billing"
            } else {
                false
            }
        }));
    }

    #[test]
    fn test_generate_allowed_edges() {
        let spec = make_spec_with_domains();
        let rules = generate_rules(&spec);

        let edge_rules: Vec<&ArchRule> = rules
            .iter()
            .filter(|r| matches!(&r.kind, RuleKind::AllowedEdges { .. }))
            .collect();

        // auth -> billing is declared, so auth's allowed list includes billing
        let auth_rule = edge_rules
            .iter()
            .find(|r| {
                if let RuleKind::AllowedEdges { from, .. } = &r.kind {
                    from == "auth"
                } else {
                    false
                }
            })
            .expect("should have auth edge rule");

        if let RuleKind::AllowedEdges { allowed_targets, .. } = &auth_rule.kind {
            assert!(allowed_targets.contains(&"billing".to_string()));
        }

        // billing has no outgoing edges declared
        let billing_rule = edge_rules
            .iter()
            .find(|r| {
                if let RuleKind::AllowedEdges { from, .. } = &r.kind {
                    from == "billing"
                } else {
                    false
                }
            })
            .expect("should have billing edge rule");

        if let RuleKind::AllowedEdges { allowed_targets, .. } = &billing_rule.kind {
            assert!(allowed_targets.is_empty());
        }
    }

    #[test]
    fn test_no_cycle_rule() {
        let spec = make_spec_with_domains();
        let rules = generate_rules(&spec);

        let cycle_rules: Vec<&ArchRule> = rules
            .iter()
            .filter(|r| matches!(&r.kind, RuleKind::NoCycle { .. }))
            .collect();

        assert_eq!(cycle_rules.len(), 1);
        if let RuleKind::NoCycle { domains } = &cycle_rules[0].kind {
            assert_eq!(domains.len(), 3);
        }
    }

    #[test]
    fn test_format_yaml_roundtrip() {
        let spec = make_spec_with_domains();
        let rules = generate_rules(&spec);
        let yaml = format_yaml(&rules, &spec.id);

        assert!(yaml.contains("spec_id: test-spec"));
        assert!(yaml.contains("type: layering"));
        assert!(yaml.contains("type: allowed_edges"));
        assert!(yaml.contains("type: no_cycle"));
    }

    #[test]
    fn test_empty_spec_no_rules() {
        let spec = Spec {
            s5d: "1.0".into(),
            id: "empty".into(),
            version: "1.0.0".into(),
            product: "test".into(),
            tier: Tier::Lightweight,
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
        };
        let rules = generate_rules(&spec);
        assert!(rules.is_empty());
    }
}
