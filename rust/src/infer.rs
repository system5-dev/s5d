use crate::models::*;
use std::collections::HashMap;

pub fn infer_links(spec: &Spec) -> Links {
    let mut links = Links::default();

    let artifacts = match spec.artifacts.as_ref() {
        Some(a) => a,
        None => return links,
    };

    // component_to_capability: for each Component, find capabilities in the same domain
    // component_to_container: for each Component, bind it to its declared container
    for component in &artifacts.components {
        for capability in &artifacts.capabilities {
            if capability.domain == component.domain {
                links.component_to_capability.push(Binding {
                    fields: [
                        ("component".to_string(), component.id.clone()),
                        ("capability".to_string(), capability.id.clone()),
                    ]
                    .into_iter()
                    .collect::<HashMap<_, _>>(),
                });
            }
        }

        links.component_to_container.push(Binding {
            fields: [
                ("component".to_string(), component.id.clone()),
                ("container".to_string(), component.container.clone()),
            ]
            .into_iter()
            .collect::<HashMap<_, _>>(),
        });
    }

    // Build feature → domains map from component memberships
    let mut feature_domains: HashMap<&str, Vec<&str>> = HashMap::new();
    for component in &artifacts.components {
        feature_domains
            .entry(component.feature.as_str())
            .or_default()
            .push(component.domain.as_str());
    }

    // use_case_to_capability: for each UseCase, find capabilities in domains of that feature
    for use_case in &artifacts.use_cases {
        if let Some(domains) = feature_domains.get(use_case.feature.as_str()) {
            for capability in &artifacts.capabilities {
                if domains.contains(&capability.domain.as_str()) {
                    links.use_case_to_capability.push(Binding {
                        fields: [
                            ("use_case".to_string(), use_case.id.clone()),
                            ("capability".to_string(), capability.id.clone()),
                        ]
                        .into_iter()
                        .collect::<HashMap<_, _>>(),
                    });
                }
            }
        }
    }

    // feature_to_domain: for each Feature, bind it to each unique domain its components belong to
    for feature in &artifacts.features {
        if let Some(domains) = feature_domains.get(feature.id.as_str()) {
            let mut seen = std::collections::HashSet::new();
            for domain in domains {
                if seen.insert(*domain) {
                    links.feature_to_domain.push(Binding {
                        fields: [
                            ("feature".to_string(), feature.id.clone()),
                            ("domain".to_string(), domain.to_string()),
                        ]
                        .into_iter()
                        .collect::<HashMap<_, _>>(),
                    });
                }
            }
        }
    }

    links
}

pub fn merge_links(existing: &mut Links, inferred: &Links) {
    merge_bindings(
        &mut existing.component_to_capability,
        &inferred.component_to_capability,
        &["component", "capability"],
    );
    merge_bindings(
        &mut existing.component_to_container,
        &inferred.component_to_container,
        &["component", "container"],
    );
    merge_bindings(
        &mut existing.use_case_to_capability,
        &inferred.use_case_to_capability,
        &["use_case", "capability"],
    );
    merge_bindings(
        &mut existing.feature_to_domain,
        &inferred.feature_to_domain,
        &["feature", "domain"],
    );
}

/// Clone the spec and merge inferred links into it.
/// Use this everywhere that needs a "materialized" spec (preview, import, drift, reconcile).
pub fn materialize_spec(spec: &Spec) -> Spec {
    let mut spec = spec.clone();
    let inferred = infer_links(&spec);
    if spec.links.is_none() {
        spec.links = Some(inferred);
    } else if let Some(ref mut links) = spec.links {
        merge_links(links, &inferred);
    }
    spec
}

fn merge_bindings(existing: &mut Vec<Binding>, inferred: &[Binding], keys: &[&str]) {
    for binding in inferred {
        let already_exists = existing.iter().any(|e| {
            keys.iter()
                .all(|k| e.fields.get(*k) == binding.fields.get(*k))
        });
        if !already_exists {
            existing.push(binding.clone());
        }
    }
}
