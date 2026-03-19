use crate::models::*;
use std::collections::{HashMap, HashSet};

pub fn graph_check(spec: &Spec) -> Vec<String> {
    let mut errors = Vec::new();

    let links = match &spec.links {
        Some(l) => l,
        None => return errors,
    };

    let mut declared_ids: HashSet<String> = HashSet::new();
    if let Some(ref arts) = spec.artifacts {
        for p in &arts.products {
            declared_ids.insert(p.id.clone());
        }
        for d in &arts.domains {
            declared_ids.insert(d.id.clone());
        }
        for c in &arts.capabilities {
            declared_ids.insert(c.id.clone());
        }
        for e in &arts.entities {
            declared_ids.insert(e.id.clone());
        }
        for f in &arts.features {
            declared_ids.insert(f.id.clone());
        }
        for u in &arts.use_cases {
            declared_ids.insert(u.id.clone());
        }
        for s in &arts.systems {
            declared_ids.insert(s.id.clone());
        }
        for c in &arts.containers {
            declared_ids.insert(c.id.clone());
        }
        for c in &arts.components {
            declared_ids.insert(c.id.clone());
        }
        for r in &arts.roles {
            declared_ids.insert(r.id.clone());
        }
        for c in &arts.concerns {
            declared_ids.insert(c.id.clone());
        }
        for m in &arts.metrics {
            declared_ids.insert(m.id.clone());
        }
    }

    let sccs = tarjan_scc(spec);
    for scc in &sccs {
        errors.push(format!("cycle detected: {}", scc.join(" → ")));
    }

    for dep in &links.depends_on {
        if dep.feature.is_empty() {
            errors.push("depends_on: empty feature reference".into());
        }
    }

    validate_bindings(
        &links.feature_to_domain,
        "feature_to_domain",
        &declared_ids,
        &mut errors,
    );
    validate_bindings(
        &links.use_case_to_capability,
        "use_case_to_capability",
        &declared_ids,
        &mut errors,
    );
    validate_bindings(
        &links.use_case_to_entity,
        "use_case_to_entity",
        &declared_ids,
        &mut errors,
    );
    validate_bindings(
        &links.component_to_capability,
        "component_to_capability",
        &declared_ids,
        &mut errors,
    );
    validate_bindings(
        &links.component_to_entity,
        "component_to_entity",
        &declared_ids,
        &mut errors,
    );
    validate_bindings(
        &links.container_to_capability,
        "container_to_capability",
        &declared_ids,
        &mut errors,
    );
    validate_bindings(
        &links.concern_to_metric,
        "concern_to_metric",
        &declared_ids,
        &mut errors,
    );
    validate_bindings(
        &links.component_to_container,
        "component_to_container",
        &declared_ids,
        &mut errors,
    );

    // Domain layering check
    errors.extend(check_domain_layering(spec));

    errors
}

pub fn check_domain_layering(spec: &Spec) -> Vec<String> {
    let mut errors = Vec::new();

    let domains: HashMap<&str, &str> = spec
        .artifacts
        .as_ref()
        .map(|a| {
            a.domains
                .iter()
                .filter_map(|d| d.classification.as_deref().map(|c| (d.id.as_str(), c)))
                .collect()
        })
        .unwrap_or_default();

    fn level(classification: &str) -> u8 {
        match classification.to_lowercase().as_str() {
            "core" => 3,
            "supporting" => 2,
            "generic" => 1,
            _ => 0,
        }
    }

    if let Some(ref links) = spec.links {
        for edge in &links.edges {
            let from_level = domains
                .get(edge.from.as_str())
                .map(|c| level(c))
                .unwrap_or(0);
            let to_level = domains.get(edge.to.as_str()).map(|c| level(c)).unwrap_or(0);

            if from_level > 0 && to_level > 0 && from_level < to_level {
                let from_class = domains.get(edge.from.as_str()).unwrap_or(&"unknown");
                let to_class = domains.get(edge.to.as_str()).unwrap_or(&"unknown");
                errors.push(format!(
                    "domain layering violation: {} ({}) depends on {} ({}). Lower-level domains must not depend on higher-level.",
                    edge.from, from_class, edge.to, to_class
                ));
            }
        }
    }

    errors
}

/// Tarjan's algorithm — returns all strongly connected components with size > 1.
pub fn tarjan_scc(spec: &Spec) -> Vec<Vec<String>> {
    let links = match &spec.links {
        Some(l) => l,
        None => return Vec::new(),
    };

    let bidirectional = ["shared_kernel", "partnership", "separate_ways"];
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    let mut all_nodes: HashSet<String> = HashSet::new();

    for edge in &links.edges {
        if bidirectional.contains(&edge.archetype.as_str()) {
            continue;
        }
        let from_key = edge.from.clone();
        let to_key = edge.to.clone();
        all_nodes.insert(from_key.clone());
        all_nodes.insert(to_key.clone());
        adj.entry(from_key).or_default().push(to_key);
    }

    let mut index_counter: usize = 0;
    let mut stack: Vec<String> = Vec::new();
    let mut on_stack: HashSet<String> = HashSet::new();
    let mut indices: HashMap<String, usize> = HashMap::new();
    let mut lowlinks: HashMap<String, usize> = HashMap::new();
    let mut sccs: Vec<Vec<String>> = Vec::new();

    #[allow(clippy::too_many_arguments)]
    fn strongconnect(
        v: &str,
        adj: &HashMap<String, Vec<String>>,
        index_counter: &mut usize,
        stack: &mut Vec<String>,
        on_stack: &mut HashSet<String>,
        indices: &mut HashMap<String, usize>,
        lowlinks: &mut HashMap<String, usize>,
        sccs: &mut Vec<Vec<String>>,
    ) {
        indices.insert(v.to_string(), *index_counter);
        lowlinks.insert(v.to_string(), *index_counter);
        *index_counter += 1;
        stack.push(v.to_string());
        on_stack.insert(v.to_string());

        if let Some(neighbors) = adj.get(v) {
            for w in neighbors {
                if !indices.contains_key(w.as_str()) {
                    strongconnect(
                        w,
                        adj,
                        index_counter,
                        stack,
                        on_stack,
                        indices,
                        lowlinks,
                        sccs,
                    );
                    let w_low = lowlinks[w.as_str()];
                    let v_low = lowlinks[v];
                    if w_low < v_low {
                        lowlinks.insert(v.to_string(), w_low);
                    }
                } else if on_stack.contains(w.as_str()) {
                    let w_idx = indices[w.as_str()];
                    let v_low = lowlinks[v];
                    if w_idx < v_low {
                        lowlinks.insert(v.to_string(), w_idx);
                    }
                }
            }
        }

        if lowlinks[v] == indices[v] {
            let mut scc = Vec::new();
            loop {
                let w = stack.pop().unwrap();
                on_stack.remove(&w);
                scc.push(w.clone());
                if w == v {
                    break;
                }
            }
            if scc.len() > 1 {
                scc.sort();
                sccs.push(scc);
            }
        }
    }

    for node in &all_nodes {
        if !indices.contains_key(node.as_str()) {
            strongconnect(
                node,
                &adj,
                &mut index_counter,
                &mut stack,
                &mut on_stack,
                &mut indices,
                &mut lowlinks,
                &mut sccs,
            );
        }
    }

    sccs
}

fn validate_bindings(
    bindings: &[Binding],
    link_type: &str,
    declared_ids: &HashSet<String>,
    errors: &mut Vec<String>,
) {
    for binding in bindings {
        for (key, value) in &binding.fields {
            if !declared_ids.contains(value) {
                errors.push(format!(
                    "{}: {} '{}' not declared in artifacts",
                    link_type, key, value
                ));
            }
        }
    }
}
