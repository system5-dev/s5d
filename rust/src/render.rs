use crate::models::*;
use std::collections::HashMap;

const DOMAIN_MAP_TEMPLATE: &str = include_str!("templates/domain_map.html");

pub fn render_domain_html(spec: &Spec) -> String {
    let arts = match &spec.artifacts {
        Some(a) => a,
        None => return "<html><body>No artifacts in spec.</body></html>".to_string(),
    };

    let mut caps_by_domain: HashMap<&str, Vec<&Capability>> = HashMap::new();
    for cap in &arts.capabilities {
        caps_by_domain
            .entry(cap.domain.as_str())
            .or_default()
            .push(cap);
    }
    let mut ents_by_domain: HashMap<&str, Vec<&Entity>> = HashMap::new();
    for ent in &arts.entities {
        ents_by_domain
            .entry(ent.domain.as_str())
            .or_default()
            .push(ent);
    }

    // D3 graph nodes JSON
    let graph_nodes_json = {
        let items: Vec<String> = arts
            .domains
            .iter()
            .map(|d| {
                let cls = match d
                    .classification
                    .as_deref()
                    .unwrap_or("generic")
                    .to_lowercase()
                    .as_str()
                {
                    "core" => "core",
                    "supporting" => "supporting",
                    _ => "generic",
                };
                let name = d.name.replace('"', "'");
                format!(r#"{{"id":"{}","name":"{}","cls":"{}"}}"#, d.id, name, cls)
            })
            .collect();
        format!("[{}]", items.join(","))
    };

    // Edges JSON
    let edges_json = {
        let items: Vec<String> = spec
            .links
            .as_ref()
            .map(|l| &l.edges)
            .unwrap_or(&vec![])
            .iter()
            .map(|e| {
                let desc = e
                    .description
                    .as_deref()
                    .unwrap_or("")
                    .replace('"', "'")
                    .replace('\n', " ");
                format!(
                    r#"{{"from":"{}","to":"{}","arch":"{}","desc":"{}"}}"#,
                    e.from,
                    e.to,
                    e.archetype,
                    desc.trim()
                )
            })
            .collect();
        format!("[{}]", items.join(","))
    };

    // Domain names JSON
    let domain_names_json = {
        let items: Vec<String> = arts
            .domains
            .iter()
            .map(|d| format!(r#""{}":"{}""#, d.id, d.name))
            .collect();
        format!("{{{}}}", items.join(","))
    };

    // Cards HTML
    let cards_html: String = arts.domains.iter().map(|d| {
        let class = d.classification.as_deref().unwrap_or("generic").to_lowercase();
        let badge_class = match class.as_str() {
            "core" => "badge-core",
            "supporting" => "badge-supporting",
            _ => "badge-generic",
        };
        let desc = d.description.as_deref().unwrap_or("").trim().replace('\n', " ");
        let empty_caps: Vec<&Capability> = vec![];
        let empty_ents: Vec<&Entity> = vec![];
        let caps = caps_by_domain.get(d.id.as_str()).unwrap_or(&empty_caps);
        let ents = ents_by_domain.get(d.id.as_str()).unwrap_or(&empty_ents);

        let search_parts: Vec<&str> = std::iter::once(d.name.as_str())
            .chain(caps.iter().map(|c| c.name.as_str()))
            .chain(ents.iter().map(|e| e.name.as_str()))
            .collect();
        let search_text = search_parts.join(" ").to_lowercase();

        let caps_html = if caps.is_empty() {
            String::new()
        } else {
            let items: String = caps.iter().map(|c| {
                let cap_desc: String = c.description.as_deref().unwrap_or("").trim().chars().take(70).collect();
                format!(
                    r#"<li><span class="cap-name">{}</span><span class="cap-desc">{}</span></li>"#,
                    c.name, cap_desc
                )
            }).collect();
            format!(r#"<div class="section-label">Capabilities</div><ul class="cap-list">{}</ul>"#, items)
        };

        let ents_html = if ents.is_empty() {
            String::new()
        } else {
            let tags: String = ents.iter().map(|e| format!(r#"<span class="ent-tag">{}</span>"#, e.name)).collect();
            format!(r#"<div class="section-label">Entities</div><div class="ent-list">{}</div>"#, tags)
        };

        let desc_html = if desc.is_empty() {
            String::new()
        } else {
            let trunc: String = desc.chars().take(160).collect();
            format!(r#"<p class="desc">{}</p>"#, trunc)
        };

        format!(
            r#"<div class="card card-{class}" data-id="{id}" data-cls="{class}" data-search="{search}" onclick="selectCard('{id}')">
  <div class="card-header">
    <span class="card-name">{name}</span>
    <span class="badge {badge_class}">{cls_upper}</span>
  </div>
  {desc_html}{caps_html}{ents_html}
  <div class="relations-strip"></div>
</div>"#,
            class = class,
            id = d.id,
            search = search_text,
            name = d.name,
            badge_class = badge_class,
            cls_upper = class.to_uppercase(),
            desc_html = desc_html,
            caps_html = caps_html,
            ents_html = ents_html,
        )
    }).collect();

    let title = spec
        .meta
        .as_ref()
        .map(|m| m.title.as_str())
        .unwrap_or(&spec.id);
    let date = spec
        .meta
        .as_ref()
        .and_then(|m| m.date.as_deref())
        .unwrap_or("");
    let product = spec.product.as_str();

    let core_count = arts
        .domains
        .iter()
        .filter(|d| d.classification.as_deref().unwrap_or("").to_lowercase() == "core")
        .count();
    let supporting_count = arts
        .domains
        .iter()
        .filter(|d| d.classification.as_deref().unwrap_or("").to_lowercase() == "supporting")
        .count();
    let generic_count = arts
        .domains
        .len()
        .saturating_sub(core_count + supporting_count);

    DOMAIN_MAP_TEMPLATE
        .replace("__TITLE__", title)
        .replace("__DATE__", date)
        .replace("__PRODUCT__", product)
        .replace("__CORE_COUNT__", &core_count.to_string())
        .replace("__SUPPORTING_COUNT__", &supporting_count.to_string())
        .replace("__GENERIC_COUNT__", &generic_count.to_string())
        .replace("__CAP_COUNT__", &arts.capabilities.len().to_string())
        .replace("__ENT_COUNT__", &arts.entities.len().to_string())
        .replace("__CARDS__", &cards_html)
        .replace("__GRAPH_NODES_JSON__", &graph_nodes_json)
        .replace("__EDGES_JSON__", &edges_json)
        .replace("__DOMAIN_NAMES_JSON__", &domain_names_json)
}

pub fn render_domain_map(spec: &Spec) -> String {
    let mut out = String::from("graph LR\n");

    if let Some(ref arts) = spec.artifacts {
        for d in &arts.domains {
            let class = d.classification.as_deref().unwrap_or("unknown");
            let maturity = d
                .maturity_level
                .as_deref()
                .map(|m| format!(" [{}]", m))
                .unwrap_or_default();
            out.push_str(&format!(
                "    {}[\"{} ({}){}\"]\n",
                d.id, d.name, class, maturity
            ));
        }
    }

    if let Some(ref links) = spec.links {
        for edge in &links.edges {
            let label = &edge.archetype;
            let cap = edge
                .downstream_capability
                .as_deref()
                .map(|c| format!("\\n→{}", c))
                .unwrap_or_default();
            let transport = edge
                .transport_ref
                .as_deref()
                .map(|t| format!("\\n[{}]", t))
                .unwrap_or_default();
            out.push_str(&format!(
                "    {} -->|\"{}{}{}\"| {}\n",
                edge.from, label, cap, transport, edge.to
            ));
        }
    }

    out.push_str("\n    classDef core fill:#e74c3c,color:#fff,stroke:#c0392b\n");
    out.push_str("    classDef supporting fill:#3498db,color:#fff,stroke:#2980b9\n");
    out.push_str("    classDef generic fill:#95a5a6,color:#fff,stroke:#7f8c8d\n");

    if let Some(ref arts) = spec.artifacts {
        for d in &arts.domains {
            if let Some(ref class) = d.classification {
                out.push_str(&format!("    class {} {}\n", d.id, class.to_lowercase()));
            }
        }
    }

    out
}

pub fn render_component_diagram(spec: &Spec) -> String {
    let mut out = String::from("graph TB\n");

    if let Some(ref arts) = spec.artifacts {
        let mut container_components: std::collections::HashMap<&str, Vec<&Component>> =
            std::collections::HashMap::new();
        for c in &arts.components {
            container_components
                .entry(c.container.as_str())
                .or_default()
                .push(c);
        }

        for ctr in &arts.containers {
            let tech = ctr.technology.as_deref().unwrap_or("");
            out.push_str(&format!(
                "    subgraph {}[\"{} ({})\"]\n",
                ctr.id, ctr.name, tech
            ));
            if let Some(components) = container_components.get(ctr.id.as_str()) {
                for comp in components {
                    out.push_str(&format!(
                        "        {}[\"{}\\n({})\"]\n",
                        comp.id, comp.name, comp.domain
                    ));
                }
            }
            out.push_str("    end\n");
        }

        for cap in &arts.capabilities {
            out.push_str(&format!(
                "    {}{{{{\"{}\\n({})\"}}}}\n",
                cap.id, cap.name, cap.domain
            ));
        }
    }

    if let Some(ref links) = spec.links {
        for binding in &links.component_to_capability {
            if let (Some(comp), Some(cap)) = (
                binding.fields.get("component"),
                binding.fields.get("capability"),
            ) {
                out.push_str(&format!("    {} -.->|implements| {}\n", comp, cap));
            }
        }
    }

    out
}

pub fn render_decision_map(spec: &Spec) -> String {
    let mut out = String::from("graph TD\n");

    if let Some(ref problem) = spec.problem {
        out.push_str(&format!(
            "    problem[\"PROBLEM\\n{}\"]\n",
            problem.signal().replace('"', "'")
        ));
        out.push_str("    style problem fill:#e74c3c,color:#fff\n");
    }

    for hyp in &spec.hypotheses {
        let evidence_count = hyp.evidence.len();
        let r_eff = hyp
            .r_eff
            .map(|r| format!(" R={:.2}", r))
            .unwrap_or_default();
        let spec_ref = if hyp.spec_ref.is_some() {
            " ✓spec"
        } else {
            ""
        };
        out.push_str(&format!(
            "    {}[\"{}\\n[{}] ev={}{}{}\"]",
            hyp.id,
            hyp.title.replace('"', "'"),
            hyp.layer,
            evidence_count,
            r_eff,
            spec_ref
        ));
        out.push('\n');

        if spec.problem.is_some() {
            out.push_str(&format!("    problem --> {}\n", hyp.id));
        }
    }

    if let Some(ref dec) = spec.decision {
        out.push_str(&format!(
            "    decision[\"DECISION\\n{}\"]\n",
            dec.title.replace('"', "'")
        ));
        out.push_str("    style decision fill:#27ae60,color:#fff\n");
        out.push_str(&format!("    {} ==>|winner| decision\n", dec.winner_id));
        for rej in &dec.rejected_ids {
            out.push_str(&format!("    {} -.->|rejected| decision\n", rej));
        }
    }

    out.push_str("\n    classDef l0 fill:#f39c12,color:#fff\n");
    out.push_str("    classDef l1 fill:#3498db,color:#fff\n");
    out.push_str("    classDef l2 fill:#27ae60,color:#fff\n");
    out.push_str("    classDef invalid fill:#95a5a6,color:#fff,stroke-dasharray: 5 5\n");

    for hyp in &spec.hypotheses {
        let class = match hyp.layer.as_str() {
            "L0" => "l0",
            "L1" => "l1",
            "L2" => "l2",
            "invalid" => "invalid",
            _ => "l0",
        };
        out.push_str(&format!("    class {} {}\n", hyp.id, class));
    }

    out
}

pub fn render(spec: &Spec, view: Option<&str>) -> String {
    match view {
        Some("domain") => render_domain_map(spec),
        Some("components") => render_component_diagram(spec),
        Some("decision") => render_decision_map(spec),
        None => {
            if matches!(spec.tier, crate::models::Tier::Decision) {
                render_decision_map(spec)
            } else {
                render_domain_map(spec)
            }
        }
        Some(other) => format!("Unknown view: {}. Use: domain, components, decision", other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::generate_spec;

    #[test]
    fn test_render_domain_map_basic() {
        let mut spec = generate_spec("feat.test", Tier::Standard, "Prod");
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
                id: "support-dom".into(),
                product: "Prod".into(),
                name: "Support".into(),
                classification: Some("supporting".into()),
                description: None,
                team: None,
                maturity_level: None,
            },
        ];
        spec.links = Some(Links {
            edges: vec![Edge {
                from: "core-dom".into(),
                to: "support-dom".into(),
                archetype: "customer_supplier".into(),
                ..Default::default()
            }],
            ..Default::default()
        });

        let result = render_domain_map(&spec);
        assert!(result.contains("graph LR"));
        assert!(result.contains("core-dom"));
        assert!(result.contains("support-dom"));
        assert!(result.contains("customer_supplier"));
        assert!(result.contains("classDef core"));
    }

    #[test]
    fn test_render_decision_map() {
        let mut spec = crate::template::generate_decision_spec("dec.test", "Prod", "Which DB?");
        spec.hypotheses = vec![
            Hypothesis {
                id: "pg".into(),
                title: "PostgreSQL".into(),
                content: "".into(),
                scope: "db".into(),
                kind: "system".into(),
                layer: "L2".into(),
                r_eff: Some(0.85),
                evidence: vec![],
                depends_on: vec![],
                rationale: None,
                spec_ref: Some("feat.pg".into()),
            },
            Hypothesis {
                id: "mysql".into(),
                title: "MySQL".into(),
                content: "".into(),
                scope: "db".into(),
                kind: "system".into(),
                layer: "invalid".into(),
                r_eff: None,
                evidence: vec![],
                depends_on: vec![],
                rationale: None,
                spec_ref: None,
            },
        ];
        spec.decision = Some(DecisionRecord {
            title: "Use PostgreSQL".into(),
            winner_id: "pg".into(),
            rejected_ids: vec!["mysql".into()],
            context: "".into(),
            decision: "".into(),
            rationale: "".into(),
            consequences: "".into(),
            decided_at: None,
            confirmed_by: None,
            expires_at: None,
            do_list: vec![],
            dont_list: vec![],
        });

        let result = render_decision_map(&spec);
        assert!(result.contains("PROBLEM"));
        assert!(result.contains("PostgreSQL"));
        assert!(result.contains("winner"));
        assert!(result.contains("rejected"));
        assert!(result.contains("✓spec"));
        assert!(result.contains("classDef l2"));
    }

    #[test]
    fn test_render_auto_detect() {
        let spec = generate_spec("feat.test", Tier::Standard, "Prod");
        let result = render(&spec, None);
        assert!(result.contains("graph LR"));

        let dec_spec = crate::template::generate_decision_spec("dec.test", "Prod", "Test?");
        let result = render(&dec_spec, None);
        assert!(result.contains("graph TD"));
    }
}
