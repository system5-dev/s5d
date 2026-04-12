use crate::identity::AliasTable;
use crate::import::{compute_diff, compute_state_fingerprint, DiffActions};
use crate::models::*;
use crate::project::S5dProject;
use chrono::Utc;

pub enum DriftResult {
    Synced,
    Drifted { expected: String, actual: String },
    Degraded { reason: String },
}

pub fn check_drift(
    project: &S5dProject,
    spec: &Spec,
    _spec_filename: &str,
) -> anyhow::Result<DriftResult> {
    let s5d_dir = project.s5d_dir();
    let ledger = project.load_ledger()?;

    let last_entry = ledger.entries.iter().rev().find(|e| {
        e.package_id == spec.id
            && (e.action == "import" || e.action == "reconcile")
            && e.status == "success"
    });

    let expected_fp = match last_entry {
        Some(entry) => &entry.state_fingerprint,
        None => {
            return Ok(DriftResult::Degraded {
                reason: format!("no successful import found for {}", spec.id),
            })
        }
    };

    let aliases = AliasTable::load(&s5d_dir)?;
    let spec = crate::infer::materialize_spec(spec);
    let actual_fp = compute_state_fingerprint(&spec, &aliases);

    if actual_fp == *expected_fp {
        Ok(DriftResult::Synced)
    } else {
        Ok(DriftResult::Drifted {
            expected: expected_fp.clone(),
            actual: actual_fp,
        })
    }
}

pub fn reconcile(
    project: &S5dProject,
    spec: &Spec,
    spec_path: &std::path::Path,
    spec_filename: &str,
) -> anyhow::Result<(DiffActions, String)> {
    let s5d_dir = project.s5d_dir();

    let record = project
        .load_record(spec_filename)?
        .ok_or_else(|| anyhow::anyhow!("no record found for {}", spec_filename))?;

    let spec_sha = S5dProject::file_sha256(spec_path)?;
    if let Some(approval) = record.approvals.last() {
        if spec_sha != approval.spec_sha256 {
            anyhow::bail!("spec has been modified since approval — cannot reconcile (use normal pipeline to re-approve)");
        }
    }

    let mut aliases = AliasTable::load(&s5d_dir)?;
    if let Some(ref meta) = spec.meta {
        aliases.apply_renames(&spec.id, &meta.renames);
    }
    let spec = crate::infer::materialize_spec(spec);
    let actions = compute_diff(&spec, &mut aliases);

    aliases.save(&s5d_dir)?;

    let fingerprint = compute_state_fingerprint(&spec, &aliases);

    let mut ledger = project.load_ledger()?;
    ledger.entries.push(LedgerEntry {
        spec_sha256: spec_sha,
        state_fingerprint: fingerprint.clone(),
        package_id: spec.id.clone(),
        action: "reconcile".into(),
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
        .unwrap_or_else(|| crate::template::generate_record(spec_filename, ""));
    record.sync_status = SyncStatus::Synced;
    record.status_history.push(StatusEntry {
        status: record.status.clone(),
        timestamp: Utc::now().to_rfc3339(),
    });
    project.save_record(spec_filename, &record)?;

    Ok((actions, fingerprint))
}
