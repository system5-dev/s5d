use std::collections::HashMap;
use std::path::{Component as PathComponent, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{DecisionRecord, Hypothesis, S5dProject, Spec, Tier};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeTrace {
    pub target: String,
    pub matches: Vec<CodeTraceMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeTraceMatch {
    pub spec_id: String,
    pub spec_path: String,
    pub tier: String,
    pub title: String,
    pub component_id: String,
    pub component_name: String,
    pub domain_id: String,
    pub domain_name: Option<String>,
    pub container_id: String,
    pub container_name: Option<String>,
    pub claimed_path: String,
    pub capabilities: Vec<CodeTraceCapability>,
    pub decisions: Vec<CodeTraceDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeTraceCapability {
    pub id: String,
    pub name: Option<String>,
    pub domain_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeTraceDecision {
    pub spec_id: String,
    pub spec_path: String,
    pub title: Option<String>,
    pub winner_id: Option<String>,
    pub hypothesis_id: String,
    pub hypothesis_title: String,
    pub accepted: bool,
    pub decision: Option<String>,
}

#[derive(Debug, Clone)]
struct DecisionLink {
    spec_id: String,
    spec_path: String,
    title: Option<String>,
    winner_id: Option<String>,
    hypothesis_id: String,
    hypothesis_title: String,
    accepted: bool,
    decision: Option<String>,
}

pub fn trace_code_path(project: &S5dProject, path_arg: &str) -> anyhow::Result<CodeTrace> {
    let target = normalize_target_path(project, path_arg)?;
    let specs = project.discover_specs()?;
    let decision_index = build_decision_index(project, &specs)?;
    let mut matches = Vec::new();

    for (spec_path, spec) in &specs {
        let Some(artifacts) = spec.artifacts.as_ref() else {
            continue;
        };

        let spec_keys = spec_match_keys(project, spec_path, spec);
        let mut decisions = Vec::new();
        for key in &spec_keys {
            if let Some(links) = decision_index.get(key) {
                decisions.extend(links.iter().cloned().map(CodeTraceDecision::from));
            }
        }
        dedup_decisions(&mut decisions);

        let capability_names: HashMap<_, _> = artifacts
            .capabilities
            .iter()
            .map(|c| (c.id.as_str(), c))
            .collect();
        let domain_names: HashMap<_, _> = artifacts
            .domains
            .iter()
            .map(|d| (d.id.as_str(), d.name.as_str()))
            .collect();
        let container_names: HashMap<_, _> = artifacts
            .containers
            .iter()
            .map(|c| (c.id.as_str(), c.name.as_str()))
            .collect();

        let links = spec.links.as_ref();
        for component in &artifacts.components {
            for claimed_path in &component.paths {
                if !path_claim_matches(claimed_path, &target) {
                    continue;
                }

                let capabilities = links
                    .into_iter()
                    .flat_map(|l| l.component_to_capability.iter())
                    .filter(|binding| binding.fields.get("component") == Some(&component.id))
                    .filter_map(|binding| binding.fields.get("capability"))
                    .map(|capability_id| {
                        let capability = capability_names.get(capability_id.as_str()).copied();
                        CodeTraceCapability {
                            id: capability_id.clone(),
                            name: capability.map(|c| c.name.clone()),
                            domain_id: capability.map(|c| c.domain.clone()),
                        }
                    })
                    .collect();

                let mut match_decisions = decisions.clone();
                if matches!(spec.tier, Tier::Decision) {
                    if let Some(decision) = load_effective_decision(project, spec_path, spec)? {
                        match_decisions.push(CodeTraceDecision {
                            spec_id: spec.id.clone(),
                            spec_path: project_relative_path(project, spec_path),
                            title: Some(decision.title.clone()),
                            winner_id: Some(decision.winner_id.clone()),
                            hypothesis_id: decision.winner_id.clone(),
                            hypothesis_title: decision.winner_id.clone(),
                            accepted: true,
                            decision: Some(decision.decision.clone()),
                        });
                    }
                }
                dedup_decisions(&mut match_decisions);

                matches.push(CodeTraceMatch {
                    spec_id: spec.id.clone(),
                    spec_path: project_relative_path(project, spec_path),
                    tier: spec.tier.to_string(),
                    title: spec_title(spec),
                    component_id: component.id.clone(),
                    component_name: component.name.clone(),
                    domain_id: component.domain.clone(),
                    domain_name: domain_names
                        .get(component.domain.as_str())
                        .map(|name| (*name).to_string()),
                    container_id: component.container.clone(),
                    container_name: container_names
                        .get(component.container.as_str())
                        .map(|name| (*name).to_string()),
                    claimed_path: normalize_claim_path(claimed_path),
                    capabilities,
                    decisions: match_decisions,
                });
            }
        }
    }

    matches.sort_by(|a, b| {
        a.spec_id
            .cmp(&b.spec_id)
            .then_with(|| a.component_id.cmp(&b.component_id))
            .then_with(|| a.claimed_path.cmp(&b.claimed_path))
    });

    Ok(CodeTrace { target, matches })
}

pub fn format_code_trace(trace: &CodeTrace) -> String {
    let mut out = format!("Trace: {}\n", trace.target);
    if trace.matches.is_empty() {
        out.push_str("No S5D component.paths claim this source path.\n");
        out.push_str("Hint: add component.paths in the owning spec, then run s5d validate.");
        return out;
    }

    out.push_str(&format!("Matches: {}\n", trace.matches.len()));
    for item in &trace.matches {
        out.push_str(&format!(
            "\n- {} [{}] {}\n",
            item.spec_id, item.tier, item.title
        ));
        out.push_str(&format!("  spec: {}\n", item.spec_path));
        out.push_str(&format!(
            "  component: {} - {}\n",
            item.component_id, item.component_name
        ));
        out.push_str(&format!("  claimed_path: {}\n", item.claimed_path));

        let domain = label_with_optional_name(&item.domain_id, item.domain_name.as_deref());
        let container =
            label_with_optional_name(&item.container_id, item.container_name.as_deref());
        out.push_str(&format!("  domain: {}\n", domain));
        out.push_str(&format!("  container: {}\n", container));

        if !item.capabilities.is_empty() {
            out.push_str("  capabilities:\n");
            for capability in &item.capabilities {
                let label = label_with_optional_name(&capability.id, capability.name.as_deref());
                if let Some(domain_id) = &capability.domain_id {
                    out.push_str(&format!("    - {} (domain: {})\n", label, domain_id));
                } else {
                    out.push_str(&format!("    - {}\n", label));
                }
            }
        }

        if !item.decisions.is_empty() {
            out.push_str("  decisions:\n");
            for decision in &item.decisions {
                let title = decision.title.as_deref().unwrap_or(&decision.spec_id);
                let status = if decision.accepted {
                    "accepted"
                } else {
                    "linked"
                };
                out.push_str(&format!(
                    "    - {} [{}]: {} via {}\n",
                    decision.spec_id, status, title, decision.hypothesis_id
                ));
                if let Some(text) = &decision.decision {
                    out.push_str(&format!("      decision: {}\n", first_line(text)));
                }
            }
        }
    }

    out.trim_end().to_string()
}

impl From<DecisionLink> for CodeTraceDecision {
    fn from(link: DecisionLink) -> Self {
        Self {
            spec_id: link.spec_id,
            spec_path: link.spec_path,
            title: link.title,
            winner_id: link.winner_id,
            hypothesis_id: link.hypothesis_id,
            hypothesis_title: link.hypothesis_title,
            accepted: link.accepted,
            decision: link.decision,
        }
    }
}

fn build_decision_index(
    project: &S5dProject,
    specs: &[(PathBuf, Spec)],
) -> anyhow::Result<HashMap<String, Vec<DecisionLink>>> {
    let mut index: HashMap<String, Vec<DecisionLink>> = HashMap::new();
    for (spec_path, spec) in specs {
        if !matches!(spec.tier, Tier::Decision) {
            continue;
        }

        let decision = load_effective_decision(project, spec_path, spec)?;
        for hypothesis in &spec.hypotheses {
            let Some(spec_ref) = hypothesis.spec_ref.as_deref() else {
                continue;
            };

            let link = build_decision_link(project, spec_path, spec, hypothesis, decision.as_ref());
            for key in reference_keys(spec_ref) {
                index.entry(key).or_default().push(link.clone());
            }
        }
    }
    Ok(index)
}

fn build_decision_link(
    project: &S5dProject,
    spec_path: &Path,
    spec: &Spec,
    hypothesis: &Hypothesis,
    decision: Option<&DecisionRecord>,
) -> DecisionLink {
    let winner_id = decision.map(|d| d.winner_id.clone());
    let accepted = winner_id.as_deref() == Some(hypothesis.id.as_str());
    DecisionLink {
        spec_id: spec.id.clone(),
        spec_path: project_relative_path(project, spec_path),
        title: decision
            .map(|d| d.title.clone())
            .or_else(|| spec.meta.as_ref().map(|m| m.title.clone())),
        winner_id,
        hypothesis_id: hypothesis.id.clone(),
        hypothesis_title: hypothesis.title.clone(),
        accepted,
        decision: decision.map(|d| d.decision.clone()),
    }
}

fn load_effective_decision(
    project: &S5dProject,
    spec_path: &Path,
    spec: &Spec,
) -> anyhow::Result<Option<DecisionRecord>> {
    let filename = spec_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("cannot determine spec filename: {}", spec_path.display()))?
        .to_string_lossy()
        .into_owned();
    Ok(project
        .load_record(&filename)?
        .and_then(|record| record.decision)
        .or_else(|| spec.decision.clone()))
}

fn spec_match_keys(project: &S5dProject, spec_path: &Path, spec: &Spec) -> Vec<String> {
    let mut keys = vec![spec.id.clone(), project_relative_path(project, spec_path)];
    if let Some(filename) = spec_path.file_name() {
        keys.push(filename.to_string_lossy().into_owned());
    }
    keys.sort();
    keys.dedup();
    keys
}

fn reference_keys(spec_ref: &str) -> Vec<String> {
    let normalized = spec_ref.replace('\\', "/");
    let mut keys = vec![normalized.clone()];
    if let Some(filename) = normalized.rsplit('/').next() {
        keys.push(filename.to_string());
        if let Some(id) = filename.strip_suffix(".s5d.yaml") {
            if let Some((stem, _date)) = id.rsplit_once("__") {
                keys.push(stem.to_string());
            }
        }
    }
    if let Some(id) = normalized.strip_suffix(".s5d.yaml") {
        if let Some((stem, _date)) = id.rsplit_once("__") {
            keys.push(stem.to_string());
        }
    }
    keys.sort();
    keys.dedup();
    keys
}

fn normalize_target_path(project: &S5dProject, path_arg: &str) -> anyhow::Result<String> {
    let trimmed = path_arg.trim();
    if trimmed.is_empty() {
        anyhow::bail!("path is required");
    }

    let path = Path::new(trimmed);
    let relative = if path.is_absolute() {
        match path.strip_prefix(&project.root) {
            Ok(stripped) => stripped.to_path_buf(),
            Err(_) => {
                let root = project
                    .root
                    .canonicalize()
                    .unwrap_or_else(|_| project.root.clone());
                let canonical = if path.exists() {
                    path.canonicalize()?
                } else {
                    path.to_path_buf()
                };
                canonical
                    .strip_prefix(&root)
                    .map(Path::to_path_buf)
                    .map_err(|_| {
                        anyhow::anyhow!(
                            "path '{}' is outside S5D project root '{}'",
                            path.display(),
                            project.root.display()
                        )
                    })?
            }
        }
    } else {
        let cwd = std::env::current_dir().unwrap_or_else(|_| project.root.clone());
        let candidate = cwd.join(path);
        if candidate.exists() {
            candidate
                .strip_prefix(&project.root)
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| path.to_path_buf())
        } else {
            path.to_path_buf()
        }
    };

    normalize_relative_path(&relative)
}

fn normalize_claim_path(path: &str) -> String {
    let path = path.trim().replace('\\', "/");
    path.trim_start_matches("./")
        .trim_end_matches('/')
        .to_string()
}

fn normalize_relative_path(path: &Path) -> anyhow::Result<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            PathComponent::CurDir => {}
            PathComponent::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            PathComponent::ParentDir => {
                if parts.pop().is_none() {
                    anyhow::bail!("path must stay inside the project: {}", path.display());
                }
            }
            PathComponent::RootDir | PathComponent::Prefix(_) => {
                anyhow::bail!("expected project-relative path, got: {}", path.display());
            }
        }
    }

    if parts.is_empty() {
        anyhow::bail!("path must not resolve to project root");
    }

    Ok(parts.join("/"))
}

fn path_claim_matches(claimed_path: &str, target: &str) -> bool {
    let claim = normalize_claim_path(claimed_path);
    if claim.is_empty() {
        return false;
    }
    if has_glob_meta(&claim) {
        return glob::Pattern::new(&claim)
            .map(|pattern| pattern.matches(target) || pattern.matches_path(Path::new(target)))
            .unwrap_or(false);
    }
    target == claim || target.starts_with(&format!("{}/", claim))
}

fn has_glob_meta(path: &str) -> bool {
    path.contains('*') || path.contains('?') || path.contains('[')
}

fn project_relative_path(project: &S5dProject, path: &Path) -> String {
    path.strip_prefix(&project.root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn spec_title(spec: &Spec) -> String {
    spec.meta
        .as_ref()
        .map(|m| m.title.clone())
        .unwrap_or_else(|| spec.id.clone())
}

fn label_with_optional_name(id: &str, name: Option<&str>) -> String {
    match name {
        Some(name) if name != id => format!("{} ({})", id, name),
        _ => id.to_string(),
    }
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or(text)
}

fn dedup_decisions(decisions: &mut Vec<CodeTraceDecision>) {
    decisions.sort_by(|a, b| {
        a.spec_id
            .cmp(&b.spec_id)
            .then_with(|| a.hypothesis_id.cmp(&b.hypothesis_id))
    });
    decisions.dedup_by(|a, b| a.spec_id == b.spec_id && a.hypothesis_id == b.hypothesis_id);
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;

    use tempfile::TempDir;

    use super::*;
    use crate::{
        Artifacts, Binding, Capability, Component, DecisionRecord, Domain, Hypothesis, Links, Meta,
        Product, Record, SoftwareSystem, SpecStatus,
    };

    #[test]
    fn traces_source_path_to_capability_and_accepted_decision() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        let (project, _report) = S5dProject::init(temp.path())?;

        let feature_filename = "feat.auth__20260503.s5d.yaml";
        let decision_filename = "decision.auth__20260503.s5d.yaml";

        let feature_spec = crate::Spec {
            s5d: "1.0".into(),
            id: "feat.auth".into(),
            version: "1.0.0".into(),
            product: "demo".into(),
            tier: Tier::Standard,
            allow_update: false,
            meta: Some(Meta {
                title: "Auth".into(),
                authors: vec![],
                date: None,
                tickets: vec![],
                adrs: vec![],
                renames: vec![],
            }),
            context: None,
            workflow: None,
            artifacts: Some(Artifacts {
                products: vec![Product {
                    id: "demo".into(),
                    name: "Demo".into(),
                    organization: None,
                }],
                domains: vec![Domain {
                    id: "dom.auth".into(),
                    product: "demo".into(),
                    name: "Auth".into(),
                    classification: Some("core".into()),
                    description: None,
                    team: None,
                    maturity_level: None,
                }],
                capabilities: vec![Capability {
                    id: "cap.login".into(),
                    domain: "dom.auth".into(),
                    name: "Login".into(),
                    description: None,
                    since: None,
                }],
                systems: vec![SoftwareSystem {
                    id: "sys.app".into(),
                    product: "demo".into(),
                    name: "App".into(),
                }],
                containers: vec![crate::Container {
                    id: "ctr.api".into(),
                    system: "sys.app".into(),
                    name: "API".into(),
                    technology: Some("Rust".into()),
                }],
                components: vec![Component {
                    id: "cmp.login".into(),
                    feature: "feat.auth".into(),
                    domain: "dom.auth".into(),
                    container: "ctr.api".into(),
                    name: "Login handler".into(),
                    paths: vec!["src/auth.rs".into()],
                }],
                ..Default::default()
            }),
            links: Some(Links {
                component_to_capability: vec![Binding {
                    fields: HashMap::from([
                        ("component".into(), "cmp.login".into()),
                        ("capability".into(), "cap.login".into()),
                    ]),
                }],
                ..Default::default()
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
            intent_kernel: None,
        };

        let decision_spec = crate::Spec {
            s5d: "1.0".into(),
            id: "decision.auth".into(),
            version: "1.0.0".into(),
            product: "demo".into(),
            tier: Tier::Decision,
            allow_update: false,
            meta: Some(Meta {
                title: "Choose auth shape".into(),
                authors: vec![],
                date: None,
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
            hypotheses: vec![Hypothesis {
                id: "server-session".into(),
                title: "Server session".into(),
                content: "Use server-side sessions".into(),
                scope: "auth".into(),
                spec_ref: Some(feature_filename.into()),
                ..Default::default()
            }],
            decision: None,
            note_rationale: None,
            expires_at: None,
            auto_noted: false,
            intent_kernel: None,
        };

        fs::write(
            project.s5d_dir().join("packages").join(feature_filename),
            serde_yaml::to_string(&feature_spec)?,
        )?;
        fs::write(
            project.s5d_dir().join("packages").join(decision_filename),
            serde_yaml::to_string(&decision_spec)?,
        )?;

        let mut record = Record {
            spec_ref: decision_filename.into(),
            spec_sha256: "sha256:test".into(),
            status: SpecStatus::Approved,
            ..Default::default()
        };
        record.decision = Some(DecisionRecord {
            title: "Auth decision".into(),
            winner_id: "server-session".into(),
            rejected_ids: vec![],
            context: "Need login state".into(),
            decision: "Use server-side sessions".into(),
            rationale: "Lower client complexity".into(),
            consequences: "Session storage required".into(),
            ..Default::default()
        });
        project.save_record(decision_filename, &record)?;

        let trace = trace_code_path(&project, "src/auth.rs")?;
        assert_eq!(trace.matches.len(), 1);
        assert_eq!(trace.matches[0].component_id, "cmp.login");
        assert_eq!(trace.matches[0].capabilities[0].id, "cap.login");
        assert_eq!(trace.matches[0].decisions[0].spec_id, "decision.auth");
        assert!(trace.matches[0].decisions[0].accepted);

        Ok(())
    }
}
