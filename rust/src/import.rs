use crate::identity::AliasTable;
use crate::models::*;
use crate::project::S5dProject;
use chrono::Utc;
use serde_json::Value;
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

/// Collect all (artifact_type, artifact_id) pairs that would be global aliases for a spec.
pub fn collect_global_artifact_ids(spec: &Spec) -> std::collections::HashSet<(String, String)> {
    let mut ids = std::collections::HashSet::new();
    if let Some(ref arts) = spec.artifacts {
        for p in &arts.products {
            ids.insert(("Product".into(), p.id.clone()));
        }
        for d in &arts.domains {
            ids.insert(("Domain".into(), d.id.clone()));
        }
        for s in &arts.systems {
            ids.insert(("SoftwareSystem".into(), s.id.clone()));
        }
        for c in &arts.containers {
            ids.insert(("Container".into(), c.id.clone()));
        }
        for r in &arts.roles {
            ids.insert(("Role".into(), r.id.clone()));
        }
    }
    ids
}

pub fn compute_state_fingerprint(spec: &Spec, aliases: &AliasTable) -> String {
    let mut hasher = Sha256::new();
    if let Ok(value) = serde_json::to_value(spec) {
        hash_canonical_json(&mut hasher, &value);
        hasher.update(b"\n");
    }
    let package_id = &spec.id;

    // Include ALL global aliases that this spec references (not just owned).
    // A spec "references" a global artifact if it declares it in artifacts.
    let referenced_globals = collect_global_artifact_ids(spec);
    let mut global_entries: Vec<String> = aliases
        .global
        .iter()
        .filter(|entry| !entry.deprecated)
        .filter(|entry| {
            let key = (entry.artifact_type.clone(), entry.artifact_id.clone());
            referenced_globals.contains(&key)
        })
        .map(|entry| {
            format!(
                "G:{}:{}:{}\n",
                entry.artifact_type, entry.artifact_id, entry.uuid
            )
        })
        .collect();
    global_entries.sort();
    for entry in global_entries {
        hasher.update(entry.as_bytes());
    }

    let mut package_entries: Vec<String> = aliases
        .packages
        .iter()
        .filter(|entry| entry.package_id.as_deref() == Some(package_id))
        .map(|entry| {
            format!(
                "P:{}:{}:{}\n",
                entry.artifact_type, entry.artifact_id, entry.uuid
            )
        })
        .collect();
    package_entries.sort();
    for entry in package_entries {
        hasher.update(entry.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn hash_canonical_json(hasher: &mut Sha256, value: &Value) {
    match value {
        Value::Null => hasher.update(b"null"),
        Value::Bool(flag) => {
            if *flag {
                hasher.update(b"true");
            } else {
                hasher.update(b"false");
            }
        }
        Value::Number(number) => hasher.update(number.to_string().as_bytes()),
        Value::String(text) => {
            let encoded = serde_json::to_string(text).expect("string serialization cannot fail");
            hasher.update(encoded.as_bytes());
        }
        Value::Array(items) => {
            hasher.update(b"[");
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    hasher.update(b",");
                }
                hash_canonical_json(hasher, item);
            }
            hasher.update(b"]");
        }
        Value::Object(map) => {
            hasher.update(b"{");
            let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
            keys.sort_unstable();
            for (idx, key) in keys.iter().enumerate() {
                if idx > 0 {
                    hasher.update(b",");
                }
                let encoded = serde_json::to_string(key).expect("key serialization cannot fail");
                hasher.update(encoded.as_bytes());
                hasher.update(b":");
                hash_canonical_json(hasher, &map[*key]);
            }
            hasher.update(b"}");
        }
    }
}

pub fn execute_import(
    project: &S5dProject,
    spec_path: &Path,
    spec: &Spec,
    spec_filename: &str,
) -> anyhow::Result<(DiffActions, String)> {
    let _lock = project.acquire_lock(&format!("import.{}", spec.id))?;
    let s5d_dir = project.s5d_dir();

    let mut aliases = AliasTable::load(&s5d_dir)?;

    if let Some(ref meta) = spec.meta {
        aliases.apply_renames(&spec.id, &meta.renames);
    }

    // Materialize spec: merge inferred links into a clone
    let spec = crate::infer::materialize_spec(spec);

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

/// Per-key outcome of the ledger ownership derivation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DerivedOwnership {
    /// The owner is certain: a sha-verified claim exists and every unverifiable
    /// ledger event that could have claimed the key first belongs to the same
    /// package — whichever import was actually first, the owner is identical.
    Owner(String),
    /// No verified claim, and exactly one package's unverifiable imports could
    /// have created the key. Not proof — the key might equally have no ledger
    /// origin — but the only candidate the ledger offers.
    UnverifiedCandidate(String),
    /// Multiple packages could own the key (a tainted entry from a DIFFERENT
    /// package precedes or shadows the verified claim) — nothing destructive
    /// may rely on it.
    Unverifiable,
}

/// Derive the owner of every global artifact from the append-only ledger plus the
/// stored packages, replaying the alias-table epoch semantics:
///
/// - AliasTable::resolve is first-creator-wins **within the current active epoch**:
///   a rollback tombstones the entry, and the next import creates a fresh one with
///   a new owner. The replay therefore releases a package's ownerships when its
///   ledger `rollback` entry is seen.
/// - An import/reconcile entry only contributes claims when the current package
///   file's sha256 matches `LedgerEntry.spec_sha256`. A mismatching or missing file
///   means we cannot know what that import declared — every key unowned at that
///   point becomes poisoned (Unverifiable) until the tainting package is rolled
///   back, because the unknown import may have been the true first creator.
///
/// Keys absent from the result have no ledger trace at all (legacy aliases).
/// Renames are approximated through the current artifact ids; at worst they
/// degrade to a non-destructive mismatch or Unverifiable, never a wrong tombstone.
pub fn derive_global_owners(
    ledger: &Ledger,
    specs: &[(std::path::PathBuf, Spec)],
    universe: &std::collections::HashSet<(String, String)>,
) -> std::collections::HashMap<(String, String), DerivedOwnership> {
    let mut by_id: std::collections::HashMap<&str, (&Spec, String)> =
        std::collections::HashMap::new();
    for (path, spec) in specs {
        let sha = S5dProject::file_sha256(path).unwrap_or_default();
        by_id.insert(spec.id.as_str(), (spec, sha));
    }

    let mut owners: std::collections::HashMap<(String, String), String> =
        std::collections::HashMap::new();
    // key → set of packages whose unverifiable imports might own it
    let mut poison: std::collections::HashMap<(String, String), std::collections::HashSet<String>> =
        std::collections::HashMap::new();

    for entry in &ledger.entries {
        if entry.status != "success" {
            continue;
        }
        match entry.action.as_str() {
            "import" | "reconcile" => {
                let verified = by_id
                    .get(entry.package_id.as_str())
                    .filter(|(_, sha)| *sha == entry.spec_sha256)
                    .map(|(spec, _)| *spec);
                match verified {
                    Some(spec) => {
                        for key in collect_global_artifact_ids(spec) {
                            if universe.contains(&key) {
                                owners
                                    .entry(key)
                                    .or_insert_with(|| entry.package_id.clone());
                            }
                        }
                    }
                    None => {
                        // Unknown claims: poison every key unowned at this point.
                        for key in universe {
                            if !owners.contains_key(key) {
                                poison
                                    .entry(key.clone())
                                    .or_default()
                                    .insert(entry.package_id.clone());
                            }
                        }
                    }
                }
            }
            "rollback" => {
                owners.retain(|_, owner| owner != &entry.package_id);
                for set in poison.values_mut() {
                    set.remove(&entry.package_id);
                }
                poison.retain(|_, set| !set.is_empty());
            }
            _ => {}
        }
    }

    let mut result = std::collections::HashMap::new();
    for key in universe {
        let poison_set = poison.get(key);
        match owners.get(key) {
            Some(owner) => {
                // Self-poison is harmless: a package's own stale-sha entries
                // (normal edit + re-import lifecycle) cannot change WHO owns
                // the key — whichever of its imports was first, the owner is
                // the same package. Only foreign unknowns break certainty.
                let foreign = poison_set.is_some_and(|set| set.iter().any(|p| p != owner));
                if foreign {
                    result.insert(key.clone(), DerivedOwnership::Unverifiable);
                } else {
                    result.insert(key.clone(), DerivedOwnership::Owner(owner.clone()));
                }
            }
            None => match poison_set {
                Some(set) if set.len() == 1 => {
                    let candidate = set.iter().next().expect("len checked").clone();
                    result.insert(
                        key.clone(),
                        DerivedOwnership::UnverifiedCandidate(candidate),
                    );
                }
                Some(_) => {
                    result.insert(key.clone(), DerivedOwnership::Unverifiable);
                }
                None => {} // no trace — legacy
            },
        }
    }
    result
}

#[derive(Debug, Default)]
pub struct RollbackReport {
    pub spec_id: String,
    /// Globals tombstoned ("Type:id"), derivation agreed or artifact was underivable.
    pub tombstoned_globals: Vec<String>,
    /// Globals kept because another spec still references them.
    pub kept_referenced: Vec<String>,
    /// Stored field claims this spec but the ledger derivation names another owner —
    /// possible owning_package corruption. Skipped, nothing tombstoned.
    pub ownership_mismatches: Vec<String>,
    /// Ledger derivation attributes the global to this spec but the stored field
    /// names someone else — possible tamper to dodge rollback. Reported, no action.
    pub suspected_tampers: Vec<String>,
    /// Tombstoned via stored-field trust because the ledger has no trace (legacy).
    pub underivable_fallbacks: Vec<String>,
    /// Skipped because ownership could not be verified — a package file was edited
    /// or deleted after import, so the ledger evidence is inconclusive. Tombstoning
    /// on inconclusive evidence would let a two-file tamper destroy another spec's
    /// global, so the entry is kept and reported.
    pub ownership_unverifiable: Vec<String>,
}

/// Roll back the last successful import of a spec. Ownership of global aliases is
/// cross-checked against the ledger derivation (decision.s5d.ownership-derivation):
/// the derivation confirms or vetoes tombstoning, never expands it. Disagreements
/// surface in the report for the caller to print loudly.
pub fn rollback_spec(
    project: &S5dProject,
    spec: &Spec,
    spec_filename: &str,
) -> anyhow::Result<RollbackReport> {
    let s5d_dir = project.s5d_dir();
    let mut ledger = project.load_ledger()?;

    let has_import = ledger
        .entries
        .iter()
        .any(|e| e.package_id == spec.id && e.action == "import" && e.status == "success");
    if !has_import {
        anyhow::bail!("no successful import found for {} to roll back", spec.id);
    }

    // Globals still referenced by other LIVE specs are never tombstoned.
    // A rolled-back spec (record status Deprecated) whose package file is still
    // on disk must not pin globals alive — its claim ended with its rollback,
    // matching the epoch semantics of the ownership derivation below.
    let all_specs = project.discover_specs()?;
    let mut referenced_globals = std::collections::HashSet::new();
    for (path, other) in &all_specs {
        if other.id == spec.id {
            continue;
        }
        let other_filename = path
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        let rolled_back = project
            .load_record(&other_filename)
            .ok()
            .flatten()
            .is_some_and(|r| r.status == SpecStatus::Deprecated);
        if !rolled_back {
            referenced_globals.extend(collect_global_artifact_ids(other));
        }
    }

    let mut aliases = AliasTable::load(&s5d_dir)?;

    // Derivation is scoped to the keys rollback can actually touch.
    let universe: std::collections::HashSet<(String, String)> = aliases
        .global
        .iter()
        .filter(|e| !e.deprecated)
        .map(|e| (e.artifact_type.clone(), e.artifact_id.clone()))
        .collect();
    let derived_owners = derive_global_owners(&ledger, &all_specs, &universe);

    let mut report = RollbackReport {
        spec_id: spec.id.clone(),
        ..Default::default()
    };

    for entry in &mut aliases.packages {
        if entry.package_id.as_deref() == Some(&spec.id) && !entry.deprecated {
            entry.deprecated = true;
        }
    }
    for entry in &mut aliases.global {
        if entry.deprecated {
            continue;
        }
        let key = (entry.artifact_type.clone(), entry.artifact_id.clone());
        let label = format!("{}:{}", entry.artifact_type, entry.artifact_id);
        let stored_claims_spec = entry.owning_package.as_deref() == Some(&spec.id);
        let derived = derived_owners.get(&key);

        if stored_claims_spec {
            // Definite-corruption check first — a stored owner the ledger contradicts
            // is reported even when the referenced-guard would keep the entry anyway.
            if let Some(DerivedOwnership::Owner(other_owner)) =
                derived.filter(|d| !matches!(d, DerivedOwnership::Owner(o) if o == &spec.id))
            {
                report.ownership_mismatches.push(format!(
                    "{} — stored owner is {} but ledger derivation says {}",
                    label, spec.id, other_owner
                ));
                continue;
            }
            if referenced_globals.contains(&key) {
                report.kept_referenced.push(label);
                continue;
            }
            match derived {
                Some(DerivedOwnership::Owner(_)) => {
                    entry.deprecated = true;
                    report.tombstoned_globals.push(label);
                }
                // The only ledger candidate is this spec itself — unverified
                // but consistent with the stored field; nothing contradicts.
                Some(DerivedOwnership::UnverifiedCandidate(c)) if c == &spec.id => {
                    entry.deprecated = true;
                    report.tombstoned_globals.push(label.clone());
                    report.underivable_fallbacks.push(label);
                }
                Some(DerivedOwnership::UnverifiedCandidate(_))
                | Some(DerivedOwnership::Unverifiable) => {
                    // Inconclusive evidence must not destroy data: a two-file
                    // tamper (alias field + any package file) would otherwise
                    // route a foreign global into the fallback tombstone.
                    report.ownership_unverifiable.push(label);
                }
                None => {
                    entry.deprecated = true;
                    report.tombstoned_globals.push(label.clone());
                    report.underivable_fallbacks.push(label);
                }
            }
        } else if matches!(derived, Some(DerivedOwnership::Owner(o)) if o == &spec.id) {
            report.suspected_tampers.push(format!(
                "{} — ledger derivation says {} owns it but stored owner is {}",
                label,
                spec.id,
                entry.owning_package.as_deref().unwrap_or("<none>")
            ));
        }
    }
    aliases.save(&s5d_dir)?;

    ledger.entries.push(LedgerEntry {
        spec_sha256: "rollback".into(),
        state_fingerprint: "rollback".into(),
        package_id: spec.id.clone(),
        action: "rollback".into(),
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
        .ok_or_else(|| anyhow::anyhow!("no record found for {}", spec_filename))?;
    record.status = SpecStatus::Deprecated;
    record.sync_status = SyncStatus::Unknown;
    record.status_history.push(StatusEntry {
        status: SpecStatus::Deprecated,
        timestamp: Utc::now().to_rfc3339(),
    });
    project.save_record(spec_filename, &record)?;

    let mut index = project.load_index()?;
    index.features.retain(|e| e.id != spec.id);
    project.save_index(&index)?;

    Ok(report)
}
