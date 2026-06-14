//! Autonomous-loop adjudication — the single control-step decision shared by the
//! CLI (`s5d mandate run`) and the MCP tool (`s5d_mandate_run`).
//!
//! S5D is a control plane, not a task orchestrator. [`adjudicate_mandate`] only
//! DECIDES whether the loop may continue and which phase is next — it never
//! executes gates and never runs an engine. The gate floor is checked against
//! gate results the agent already recorded (`record.gate_results`); the agent
//! self-gates and records, the controller adjudicates. Both call sites share this
//! one function so a fix can never land on one surface and miss the other.

use crate::models::{Record, Spec, WorkflowPhaseStatus};
use crate::{check_drift, DriftResult, S5dProject};
use std::path::Path;

/// Outcome of one autonomous-loop control step.
pub enum MandateStep {
    /// Authorize the next phase. The caller persists `iteration` into the record
    /// (`mandate_iterations = iteration`) and prints the authorization.
    Continue {
        phase_id: String,
        scope: String,
        iteration: u32,
        max_calls: Option<u32>,
        stop_conditions: Vec<String>,
    },
    /// All phases accepted — scope exhausted, clean stop.
    Complete,
    /// Hand the loop back to a human.
    Escalate { reason: String },
}

/// Adjudicate one control step. Read-only with respect to the record — the caller
/// persists the iteration bump on [`MandateStep::Continue`]. The order encodes the
/// decided contract: admission (SHA bind) → recorded gate floor → drift → budget →
/// next unaccepted phase.
pub fn adjudicate_mandate(
    project: &S5dProject,
    spec: &Spec,
    spec_path: &Path,
    spec_filename: &str,
    record: &Record,
) -> anyhow::Result<MandateStep> {
    let Some(mandate) = spec.mandate.as_ref() else {
        return Ok(MandateStep::Escalate {
            reason: "spec has no `mandate:` envelope".into(),
        });
    };

    // 1. Admission must exist and still bind the live spec.
    let spec_sha = S5dProject::file_sha256(spec_path)?;
    let Some(admission) = record.mandate_admission.as_ref() else {
        return Ok(MandateStep::Escalate {
            reason: "mandate not admitted — run `mandate admit` first".into(),
        });
    };
    if admission.spec_sha256 != spec_sha {
        return Ok(MandateStep::Escalate {
            reason: "spec changed since admission — re-admit before resuming the loop".into(),
        });
    }

    // 2. Gate floor — adjudicate RECORDED gate state. The controller never runs
    //    gates itself; the agent must have run them and recorded a pass.
    for kind in &mandate.min_gate_floor {
        let passed = record
            .gate_results
            .iter()
            .rev()
            .find(|r| &r.kind == kind)
            .is_some_and(|r| r.status == "passed");
        if !passed {
            return Ok(MandateStep::Escalate {
                reason: format!(
                    "gate floor '{kind}' has no recorded pass — agent must run gates and record before continuing"
                ),
            });
        }
    }

    // 3. Drift is always a stop-condition (constraint a).
    if let DriftResult::Drifted { expected, actual } = check_drift(project, spec, spec_filename)? {
        return Ok(MandateStep::Escalate {
            reason: format!("drift detected (expected {expected}, actual {actual})"),
        });
    }

    // 4. Budget — call count and wall-clock since admission.
    if let Some(max) = mandate.budget.max_calls {
        if record.mandate_iterations >= max {
            return Ok(MandateStep::Escalate {
                reason: format!(
                    "budget exhausted ({}/{} calls)",
                    record.mandate_iterations, max
                ),
            });
        }
    }
    if let Some(max_s) = mandate.budget.max_time_s {
        match chrono::DateTime::parse_from_rfc3339(&admission.date) {
            Ok(start) => {
                let elapsed =
                    (chrono::Utc::now() - start.with_timezone(&chrono::Utc)).num_seconds();
                if elapsed >= i64::from(max_s) {
                    return Ok(MandateStep::Escalate {
                        reason: format!("time budget exhausted ({elapsed}s/{max_s}s)"),
                    });
                }
            }
            // A time budget is set but its start point is unreadable — do not
            // silently skip the bound; hand back to a human.
            Err(_) => {
                return Ok(MandateStep::Escalate {
                    reason: "admission timestamp unreadable but a time budget is set — cannot verify the bound".into(),
                });
            }
        }
    }

    // 5. Pull the next unaccepted workflow phase.
    let Some(workflow) = spec.workflow.as_ref() else {
        return Ok(MandateStep::Escalate {
            reason: "spec has no workflow phases".into(),
        });
    };
    let next = workflow.phases.iter().find(|p| {
        let status = record
            .phase_history
            .iter()
            .rev()
            .find(|e| e.phase_id == p.id)
            .map(|e| e.status.clone())
            .unwrap_or(WorkflowPhaseStatus::Planned);
        !matches!(status, WorkflowPhaseStatus::Accepted)
    });
    let Some(phase) = next else {
        return Ok(MandateStep::Complete);
    };

    Ok(MandateStep::Continue {
        phase_id: phase.id.clone(),
        scope: phase.scope.clone(),
        iteration: record.mandate_iterations + 1,
        max_calls: mandate.budget.max_calls,
        stop_conditions: mandate.stop_conditions.clone(),
    })
}
