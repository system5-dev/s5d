use crate::identity::AliasTable;
use crate::models::*;
use crate::project::S5dProject;
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct DiffActions {
    pub create: Vec<String>,
    pub link: Vec<String>,
    pub update: Vec<String>,
    pub delete: Vec<String>,
}

impl DiffActions {
    pub fn counts(&self) -> PreviewActions {
        PreviewActions {
            create: self.create.len() as u32,
            link: self.link.len() as u32,
            update: self.update.len() as u32,
            delete: self.delete.len() as u32,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.create.is_empty()
            && self.link.is_empty()
            && self.update.is_empty()
            && self.delete.is_empty()
    }

    /// Compute diff hash for staleness detection.
    /// Uses only action type + artifact type:id (not UUIDs) for determinism.
    pub fn sha256(&self) -> String {
        let mut hasher = Sha256::new();
        for item in &self.create {
            hasher.update(format!("C:{}\n", strip_uuid(item)));
        }
        for item in &self.link {
            hasher.update(format!("L:{}\n", item));
        }
        for item in &self.update {
            hasher.update(format!("U:{}\n", strip_uuid(item)));
        }
        for item in &self.delete {
            hasher.update(format!("D:{}\n", strip_uuid(item)));
        }
        format!("sha256:{:x}", hasher.finalize())
    }
}

/// Strip UUID suffix from action strings like "Type:id (abcd1234)" → "Type:id"
fn strip_uuid(s: &str) -> &str {
    s.rfind(" (").map_or(s, |i| &s[..i])
}

pub fn compute_diff(spec: &Spec, aliases: &mut AliasTable) -> DiffActions {
    let package_id = &spec.id;
    let mut actions = DiffActions::default();

    if let Some(ref arts) = spec.artifacts {
        resolve_artifacts(&arts.products, "Product", package_id, aliases, &mut actions);
        resolve_artifacts(&arts.domains, "Domain", package_id, aliases, &mut actions);
        resolve_artifacts(
            &arts.systems,
            "SoftwareSystem",
            package_id,
            aliases,
            &mut actions,
        );
        resolve_artifacts(
            &arts.containers,
            "Container",
            package_id,
            aliases,
            &mut actions,
        );
        resolve_artifacts(&arts.roles, "Role", package_id, aliases, &mut actions);
        resolve_artifacts(
            &arts.capabilities,
            "Capability",
            package_id,
            aliases,
            &mut actions,
        );
        resolve_artifacts(&arts.entities, "Entity", package_id, aliases, &mut actions);
        resolve_artifacts(&arts.features, "Feature", package_id, aliases, &mut actions);
        resolve_artifacts(
            &arts.use_cases,
            "UseCase",
            package_id,
            aliases,
            &mut actions,
        );
        resolve_artifacts(
            &arts.components,
            "Component",
            package_id,
            aliases,
            &mut actions,
        );
        resolve_artifacts(&arts.concerns, "Concern", package_id, aliases, &mut actions);
        resolve_artifacts(
            &arts.metrics,
            "AcceptanceMetric",
            package_id,
            aliases,
            &mut actions,
        );
        resolve_artifacts(
            &arts.supersystems,
            "SuperSystem",
            package_id,
            aliases,
            &mut actions,
        );
        resolve_artifacts(
            &arts.transports,
            "Transport",
            package_id,
            aliases,
            &mut actions,
        );
    }

    if let Some(ref links) = spec.links {
        count_links(&links.feature_to_domain, "feature_to_domain", &mut actions);
        count_links(
            &links.use_case_to_capability,
            "use_case_to_capability",
            &mut actions,
        );
        count_links(
            &links.use_case_to_entity,
            "use_case_to_entity",
            &mut actions,
        );
        count_links(
            &links.component_to_capability,
            "component_to_capability",
            &mut actions,
        );
        count_links(
            &links.component_to_entity,
            "component_to_entity",
            &mut actions,
        );
        count_links(
            &links.container_to_capability,
            "container_to_capability",
            &mut actions,
        );
        count_links(&links.concern_to_metric, "concern_to_metric", &mut actions);
        count_links(
            &links.component_to_container,
            "component_to_container",
            &mut actions,
        );
        count_links(
            &links.capability_to_entity,
            "capability_to_entity",
            &mut actions,
        );
        count_links(
            &links.capability_to_concern,
            "capability_to_concern",
            &mut actions,
        );
        for edge in &links.edges {
            actions.link.push(format!(
                "edge:{}→{}({})",
                edge.from, edge.to, edge.archetype
            ));
        }
        for er in &links.entity_relations {
            actions.link.push(format!(
                "entity_relation:{}→{}",
                er.entity, er.related_entity
            ));
        }
    }

    actions
}

trait HasId {
    fn artifact_id(&self) -> &str;
}

macro_rules! impl_has_id {
    ($($t:ty),*) => {
        $(impl HasId for $t {
            fn artifact_id(&self) -> &str { &self.id }
        })*
    }
}
impl_has_id!(
    Product,
    Domain,
    Capability,
    Entity,
    Feature,
    UseCase,
    SoftwareSystem,
    Container,
    Component,
    Role,
    Concern,
    AcceptanceMetric,
    SuperSystem,
    Transport
);

fn resolve_artifacts<T: HasId>(
    items: &[T],
    type_name: &str,
    package_id: &str,
    aliases: &mut AliasTable,
    actions: &mut DiffActions,
) {
    for item in items {
        let (uuid, is_new) = aliases.resolve(package_id, item.artifact_id(), type_name);
        if is_new {
            actions.create.push(format!(
                "{}:{} ({})",
                type_name,
                item.artifact_id(),
                &uuid[..8]
            ));
        } else {
            actions.update.push(format!(
                "{}:{} ({})",
                type_name,
                item.artifact_id(),
                &uuid[..8]
            ));
        }
    }
}

fn count_links(bindings: &[Binding], link_type: &str, actions: &mut DiffActions) {
    for _binding in bindings {
        actions.link.push(format!("{}:entry", link_type));
    }
}

pub fn compute_state_fingerprint(spec: &Spec, aliases: &AliasTable) -> String {
    let mut hasher = Sha256::new();
    if let Ok(yaml) = serde_yaml::to_string(spec) {
        hasher.update(yaml.as_bytes());
    }
    let package_id = &spec.id;
    for entry in &aliases.global {
        if entry.owning_package.as_deref() == Some(package_id) {
            hasher.update(format!(
                "G:{}:{}:{}\n",
                entry.artifact_id, entry.artifact_type, entry.uuid
            ));
        }
    }
    for entry in &aliases.packages {
        if entry.package_id.as_deref() == Some(package_id) {
            hasher.update(format!(
                "P:{}:{}:{}\n",
                entry.artifact_id, entry.artifact_type, entry.uuid
            ));
        }
    }
    format!("sha256:{:x}", hasher.finalize())
}

pub fn execute_import(
    project: &S5dProject,
    spec_path: &Path,
    spec: &Spec,
    spec_filename: &str,
) -> anyhow::Result<(DiffActions, String)> {
    let s5d_dir = project.s5d_dir();

    let mut aliases = AliasTable::load(&s5d_dir)?;

    if let Some(ref meta) = spec.meta {
        aliases.apply_renames(&spec.id, &meta.renames);
    }

    // Auto-infer links from artifact hierarchy
    let mut spec = spec.clone();
    let inferred = crate::infer::infer_links(&spec);
    if spec.links.is_none() {
        spec.links = Some(inferred);
    } else if let Some(ref mut links) = spec.links {
        crate::infer::merge_links(links, &inferred);
    }

    let actions = compute_diff(&spec, &mut aliases);

    aliases.save(&s5d_dir)?;

    let fingerprint = compute_state_fingerprint(&spec, &aliases);

    let mut ledger = project.load_ledger()?;
    let spec_sha = S5dProject::file_sha256(spec_path)?;
    ledger.entries.push(LedgerEntry {
        spec_sha256: spec_sha.clone(),
        state_fingerprint: fingerprint.clone(),
        package_id: spec.id.clone(),
        action: "import".into(),
        status: "success".into(),
        timestamp: Utc::now().to_rfc3339(),
        record_ref: Some(format!(
            "records/{}",
            spec_filename.replace(".s5d.yaml", ".record.yaml")
        )),
    });
    project.save_ledger(&ledger)?;

    let mut record = project
        .load_record(spec_filename)?
        .unwrap_or_else(|| crate::template::generate_record(spec_filename, &spec_sha));
    record.status = SpecStatus::Applied;
    record.sync_status = SyncStatus::Synced;
    record.status_history.push(StatusEntry {
        status: SpecStatus::Applied,
        timestamp: Utc::now().to_rfc3339(),
    });
    project.save_record(spec_filename, &record)?;

    let mut index = project.load_index()?;
    index.features.retain(|e| e.id != spec.id);
    index.features.push(IndexEntry {
        id: spec.id.clone(),
        spec_path: format!("packages/{}", spec_filename),
        status: SpecStatus::Applied,
        product: spec.product.clone(),
        version: spec.version.clone(),
    });
    project.save_index(&index)?;

    Ok((actions, fingerprint))
}
