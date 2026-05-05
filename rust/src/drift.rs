use crate::identity::AliasTable;
use crate::import::{compute_diff, compute_state_fingerprint, DiffActions};
use crate::models::*;
use crate::project::S5dProject;
use chrono::Utc;

pub enum DriftResult {
    Synced,
    Drifted {
        expected: String,
        actual: String,
    },
    /// FPF A.3.3:9.3 partial — drift detected but tolerated by record.drift_tolerance policy.
    Partial {
        policy: String,
        note: String,
    },
    Degraded {
        reason: String,
    },
}

pub fn check_drift(
    project: &S5dProject,
    spec: &Spec,
    spec_filename: &str,
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
    let spec_mat = crate::infer::materialize_spec(spec);
    let actual_fp = compute_state_fingerprint(&spec_mat, &aliases);

    if actual_fp == *expected_fp {
        return Ok(DriftResult::Synced);
    }

    // FPF A.3.3:9.3 — fits(D, trace, tol) → {pass|fail|partial} under tolerance policy.
    // record.drift_tolerance is free-form prose like "schema=block, code=block, doc=warn".
    // If all rules are warn/allow → Partial; if any block → Drifted.
    let record = project.load_record(spec_filename)?;
    if let Some(rec) = record {
        if let Some(policy) = rec.drift_tolerance.as_ref() {
            let parsed = parse_tolerance(policy);
            // Policy is free-form. Conservative interpretation: if every rule is warn|allow
            // (no "block"), treat drift as Partial. If any rule is block (or no rules),
            // keep Drifted.
            let any_block = parsed.iter().any(|(_, action)| action == "block");
            let has_warn_or_allow = parsed
                .iter()
                .any(|(_, action)| action == "warn" || action == "allow");
            if !any_block && has_warn_or_allow {
                return Ok(DriftResult::Partial {
                    policy: policy.clone(),
                    note: format!(
                        "drift detected (expected={} actual={}) tolerated by policy",
                        &expected_fp[..16.min(expected_fp.len())],
                        &actual_fp[..16.min(actual_fp.len())],
                    ),
                });
            }
        }
    }

    Ok(DriftResult::Drifted {
        expected: expected_fp.clone(),
        actual: actual_fp,
    })
}

/// Parse a drift_tolerance string like "schema=block, code=warn, doc=allow"
/// into a Vec of (artifact, action) pairs. Allowed actions: block|warn|allow.
fn parse_tolerance(policy: &str) -> Vec<(String, String)> {
    policy
        .split(',')
        .filter_map(|pair| {
            let mut parts = pair.split('=');
            let key = parts.next()?.trim().to_lowercase();
            let val = parts.next()?.trim().to_lowercase();
            if key.is_empty() || val.is_empty() {
                return None;
            }
            if !["block", "warn", "allow"].contains(&val.as_str()) {
                return None;
            }
            Some((key, val))
        })
        .collect()
}

pub fn reconcile(
    project: &S5dProject,
    spec: &Spec,
    spec_path: &std::path::Path,
    spec_filename: &str,
) -> anyhow::Result<(DiffActions, String)> {
    let _lock = project.acquire_lock(&format!("reconcile.{}", spec.id))?;
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

    // Find the expected fingerprint from the last successful import/reconcile
    let ledger = project.load_ledger()?;
    let expected_fp = ledger
        .entries
        .iter()
        .rev()
        .find(|e| {
            e.package_id == spec.id
                && (e.action == "import" || e.action == "reconcile")
                && e.status == "success"
        })
        .map(|e| e.state_fingerprint.clone())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no successful import found for {} — cannot reconcile",
                spec.id
            )
        })?;

    let mut aliases = AliasTable::load(&s5d_dir)?;
    if let Some(ref meta) = spec.meta {
        aliases.apply_renames(&spec.id, &meta.renames);
    }
    let spec = crate::infer::materialize_spec(spec);
    let mut aliases_candidate = aliases.clone();
    let actions = compute_diff(&spec, &mut aliases_candidate);
    let fingerprint = compute_state_fingerprint(&spec, &aliases_candidate);

    // Fail closed: if compute_diff couldn't restore the expected fingerprint,
    // the alias state is corrupted in a way we can't fix from spec alone.
    // User must re-run the full pipeline (preview → approve → import).
    if fingerprint != expected_fp {
        anyhow::bail!(
            "reconcile failed: alias state cannot be restored to imported baseline.\n\
             expected: {}\n\
             actual:   {}\n\
             Alias UUIDs may be corrupted. Re-run: preview → approve → import.",
            expected_fp,
            fingerprint
        );
    }

    aliases_candidate.save(&s5d_dir)?;

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
