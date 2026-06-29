use crate::models::*;
use chrono::Utc;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Maximum log output size (10 KB). Truncated if exceeded.
const MAX_LOG_BYTES: usize = 10 * 1024;

/// Evaluate the review gate given the spec, evidence, and phase history.
/// Returns `(status, log)` — does not touch the filesystem.
///
/// Pass conditions (OR):
/// 1. `review_count >= 1`: at least one evidence entry with `evidence_type =
///    "gate:review"` and `verdict = "pass"`.
/// 2. `phase_pass`: Standard or Lightweight spec whose phase history contains at
///    least one `WorkflowPhaseStatus::Accepted` entry.
///
/// Decision / High may NOT use path 2 — their review gate is evidence-only.
pub fn eval_review_gate(
    spec: &Spec,
    phase_history: &[crate::models::WorkflowPhaseRecord],
) -> (String, String) {
    use crate::models::{Tier, WorkflowPhaseStatus};

    let mut review_count: usize = 0;
    let mut findings: Vec<String> = Vec::new();
    for hyp in &spec.hypotheses {
        for ev in &hyp.evidence {
            let is_review =
                ev.evidence_type == "gate:review" || ev.evidence_type.starts_with("gate:review:");
            if is_review && ev.verdict == "pass" {
                review_count += 1;
                findings.push(format!(
                    "  - hypothesis={} evidence_id={} formality={}",
                    hyp.id,
                    ev.id,
                    ev.formality.map_or("?".into(), |f| f.to_string())
                ));
            }
        }
    }

    let is_feature_tier = matches!(spec.tier, Tier::Standard | Tier::Lightweight);
    let phase_pass = is_feature_tier
        && phase_history
            .iter()
            .any(|p| p.status == WorkflowPhaseStatus::Accepted);

    if review_count >= 1 {
        (
            "passed".into(),
            format!(
                "review gate passed (built-in): {} review-evidence with verdict=pass\n{}",
                review_count,
                findings.join("\n")
            ),
        )
    } else if phase_pass {
        let first_accepted = phase_history
            .iter()
            .find(|p| p.status == WorkflowPhaseStatus::Accepted)
            .expect("phase_pass true implies an Accepted entry exists");
        let reviewer_note = first_accepted
            .reviewer
            .as_deref()
            .map(|r| format!(", reviewer={}", r))
            .unwrap_or_default();
        (
            "passed".into(),
            format!(
                "review gate passed (built-in): accepted run phase (phase_id={}{})",
                first_accepted.phase_id, reviewer_note
            ),
        )
    } else {
        let extra = if is_feature_tier {
            " For standard/lightweight specs you may also close this gate by accepting \
a run phase: s5d run accept <spec> --id <phase> --reviewer <name>."
        } else {
            ""
        };
        (
            "failed".into(),
            format!(
                "review gate failed (built-in): no evidence with type=gate:review and verdict=pass found. \
                Decision/high-tier specs require >=1 code review recorded as evidence. \
                Use: s5d add-evidence <spec> --hypothesis-id <id> --evidence-type gate:review \
                --content '<reviewer findings>' --verdict pass --formality <1-5> \
                --claim-scope <scope> --reliability <0..1>{}",
                extra
            ),
        )
    }
}

pub fn run_gates(
    spec: &Spec,
    config: &S5dConfig,
    spec_path: &str,
    project_root: &Path,
    s5d_dir: &Path,
    phase_history: &[crate::models::WorkflowPhaseRecord],
) -> anyhow::Result<Vec<GateResult>> {
    let mut results = Vec::new();
    let timeout_secs = config
        .gate_runner
        .as_ref()
        .and_then(|r| r.timeout_s)
        .unwrap_or(120);
    let cwd = config
        .gate_runner
        .as_ref()
        .and_then(|r| r.cwd.as_deref())
        .unwrap_or("{project}");
    let cwd_resolved = cwd.replace("{project}", &project_root.to_string_lossy());
    let env_deny: Vec<&str> = config
        .gate_runner
        .as_ref()
        .map(|r| r.env_deny.iter().map(String::as_str).collect())
        .unwrap_or_default();

    // Evidence directory: .s5d/evidence/{spec_id}/
    // spec.id comes from file content — sanitize before using it as a path
    // segment (same confinement invariant as the review scaffold).
    crate::sanitize_id(&spec.id)?;
    let evidence_dir = s5d_dir.join("evidence").join(&spec.id);

    let effective_gates = effective_gates_for_spec(spec);

    // FPF C.22:5.4 — eligibility/acceptance order. Eligibility kinds run first; on fail, fast-exit.
    // Defaults when gate.kind_class unset: schema/graph/architecture → eligibility, others → acceptance.
    let mut sorted_gates: Vec<&Gate> = effective_gates.iter().collect();
    sorted_gates.sort_by_key(|g| {
        let elig = matches!(g.kind.as_str(), "schema" | "graph" | "architecture");
        if elig {
            0
        } else {
            1
        }
    });

    let mut eligibility_failed = false;

    for gate in sorted_gates {
        if eligibility_failed {
            // FPF C.22 fast-fail: skip remaining acceptance gates after eligibility failure
            results.push(GateResult {
                kind: gate.kind.clone(),
                status: "skipped".into(),
                attempt: count_attempts(&results, &gate.kind) + 1,
                timestamp: Utc::now().to_rfc3339(),
                log: Some(
                    "skipped: prior eligibility gate failed (FPF C.22:5.4 fast-fail). \
                    Acceptance gates run only after eligibility passes."
                        .to_string(),
                ),
                exit_code: None,
                evidence_path: None,
                command: None,
                duration_ms: None,
                waiver_expires_at: None,
                kind_class: Some("acceptance".into()),
            });
            continue;
        }
        let cmd_template = match config.gate_commands.get(&gate.kind) {
            Some(cmd) => cmd.clone(),
            None => {
                // Built-in gates: schema and graph run internal validation
                let builtin_result = match gate.kind.as_str() {
                    "schema" => {
                        let errors = crate::validate_spec(spec);
                        let attempt = count_attempts(&results, &gate.kind) + 1;
                        let start = Instant::now();
                        let (status, log) = if errors.is_empty() {
                            (
                                "passed".into(),
                                "schema validation passed (built-in)".to_string(),
                            )
                        } else {
                            (
                                "failed".into(),
                                format!(
                                    "schema validation failed (built-in):\n{}",
                                    errors.join("\n")
                                ),
                            )
                        };
                        let ev_path = save_evidence(&evidence_dir, &gate.kind, attempt, &log);
                        Some(GateResult {
                            kind: gate.kind.clone(),
                            status,
                            attempt,
                            timestamp: Utc::now().to_rfc3339(),
                            log: Some(log),
                            exit_code: None,
                            evidence_path: ev_path,
                            command: None,
                            duration_ms: Some(start.elapsed().as_millis() as u64),
                            waiver_expires_at: None,
                            kind_class: None,
                        })
                    }
                    "graph" => {
                        let errors = crate::graph_check(spec);
                        let attempt = count_attempts(&results, &gate.kind) + 1;
                        let start = Instant::now();
                        let (status, log) = if errors.is_empty() {
                            ("passed".into(), "graph check passed (built-in)".to_string())
                        } else {
                            (
                                "failed".into(),
                                format!("graph check failed (built-in):\n{}", errors.join("\n")),
                            )
                        };
                        let ev_path = save_evidence(&evidence_dir, &gate.kind, attempt, &log);
                        Some(GateResult {
                            kind: gate.kind.clone(),
                            status,
                            attempt,
                            timestamp: Utc::now().to_rfc3339(),
                            log: Some(log),
                            exit_code: None,
                            evidence_path: ev_path,
                            command: None,
                            duration_ms: Some(start.elapsed().as_millis() as u64),
                            waiver_expires_at: None,
                            kind_class: None,
                        })
                    }
                    "review" => {
                        let attempt = count_attempts(&results, &gate.kind) + 1;
                        let start = Instant::now();
                        let (status, log) = eval_review_gate(spec, phase_history);
                        let ev_path = save_evidence(&evidence_dir, &gate.kind, attempt, &log);
                        Some(GateResult {
                            kind: gate.kind.clone(),
                            status,
                            attempt,
                            timestamp: Utc::now().to_rfc3339(),
                            log: Some(log),
                            exit_code: None,
                            evidence_path: ev_path,
                            command: None,
                            duration_ms: Some(start.elapsed().as_millis() as u64),
                            waiver_expires_at: None,
                            kind_class: None,
                        })
                    }
                    "architecture" => {
                        let attempt = count_attempts(&results, &gate.kind) + 1;
                        let start = Instant::now();
                        let check = crate::architecture_check(spec, project_root);
                        let (status, log) = match check {
                            Ok(report) if report.errors.is_empty() => (
                                "passed".into(),
                                format!(
                                    "architecture check passed (built-in): {} component(s), {} dependency edge(s)",
                                    report.components.len(),
                                    report.dependencies.len()
                                ),
                            ),
                            Ok(report) => (
                                "failed".into(),
                                format!(
                                    "architecture check failed (built-in):\n{}",
                                    report.errors.join("\n")
                                ),
                            ),
                            Err(error) => (
                                "failed".into(),
                                format!("architecture check failed (built-in): {}", error),
                            ),
                        };
                        let ev_path = save_evidence(&evidence_dir, &gate.kind, attempt, &log);
                        Some(GateResult {
                            kind: gate.kind.clone(),
                            status,
                            attempt,
                            timestamp: Utc::now().to_rfc3339(),
                            log: Some(log),
                            exit_code: None,
                            evidence_path: ev_path,
                            command: None,
                            duration_ms: Some(start.elapsed().as_millis() as u64),
                            waiver_expires_at: None,
                            kind_class: None,
                        })
                    }
                    _ => None,
                };

                if let Some(result) = builtin_result {
                    let elig = matches!(result.kind.as_str(), "schema" | "graph" | "architecture");
                    if elig && result.status == "failed" {
                        eligibility_failed = true;
                    }
                    results.push(result);
                } else {
                    results.push(GateResult {
                        kind: gate.kind.clone(),
                        status: "skipped".into(),
                        attempt: count_attempts(&results, &gate.kind) + 1,
                        timestamp: Utc::now().to_rfc3339(),
                        log: Some(format!("no command configured for gate '{}'", gate.kind)),
                        exit_code: None,
                        evidence_path: None,
                        command: None,
                        duration_ms: None,
                        waiver_expires_at: None,
                        kind_class: None,
                    });
                }
                continue;
            }
        };

        let cmd_args: Vec<String> = cmd_template
            .iter()
            .map(|arg| {
                arg.replace("{package}", spec_path)
                    .replace("{project}", &project_root.to_string_lossy())
            })
            .collect();

        if cmd_args.is_empty() {
            anyhow::bail!("empty command for gate '{}'", gate.kind);
        }

        let mut command = Command::new(&cmd_args[0]);
        command.args(&cmd_args[1..]);
        command.current_dir(&cwd_resolved);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        for var in &env_deny {
            command.env_remove(var);
        }

        let attempt = count_attempts(&results, &gate.kind) + 1;
        let start = Instant::now();

        let gate_result = match command.spawn() {
            Ok(mut child) => {
                let timeout_dur = Duration::from_secs(timeout_secs as u64);
                let mut stdout_handle = child.stdout.take().map(spawn_pipe_reader);
                let mut stderr_handle = child.stderr.take().map(spawn_pipe_reader);

                loop {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            let duration_ms = start.elapsed().as_millis() as u64;
                            let status_str = if status.success() { "passed" } else { "failed" };
                            let stdout_buf = join_pipe_reader(stdout_handle.take());
                            let stderr_buf = join_pipe_reader(stderr_handle.take());

                            let full_log = format!(
                                "exit: {}\nstdout:\n{}\nstderr:\n{}",
                                status, stdout_buf, stderr_buf
                            );

                            // Write full evidence to disk
                            let ev_path =
                                save_evidence(&evidence_dir, &gate.kind, attempt, &full_log);

                            let mut truncated_log = full_log.clone();
                            truncate_log(&mut truncated_log);

                            break GateResult {
                                kind: gate.kind.clone(),
                                status: status_str.into(),
                                attempt,
                                timestamp: Utc::now().to_rfc3339(),
                                log: Some(truncated_log),
                                exit_code: status.code(),
                                evidence_path: ev_path,
                                command: Some(cmd_args.clone()),
                                duration_ms: Some(duration_ms),
                                waiver_expires_at: None,
                                kind_class: None,
                            };
                        }
                        Ok(None) => {
                            if start.elapsed() > timeout_dur {
                                let duration_ms = start.elapsed().as_millis() as u64;
                                let _ = child.kill();
                                let _ = child.wait();
                                let stdout_buf = join_pipe_reader(stdout_handle.take());
                                let stderr_buf = join_pipe_reader(stderr_handle.take());
                                let timeout_log = format!(
                                    "killed after {}s timeout\nstdout:\n{}\nstderr:\n{}",
                                    timeout_secs, stdout_buf, stderr_buf
                                );
                                let ev_path =
                                    save_evidence(&evidence_dir, &gate.kind, attempt, &timeout_log);
                                break GateResult {
                                    kind: gate.kind.clone(),
                                    status: "timeout".into(),
                                    attempt,
                                    timestamp: Utc::now().to_rfc3339(),
                                    log: Some(timeout_log),
                                    exit_code: None,
                                    evidence_path: ev_path,
                                    command: Some(cmd_args.clone()),
                                    duration_ms: Some(duration_ms),
                                    waiver_expires_at: None,
                                    kind_class: None,
                                };
                            }
                            std::thread::sleep(Duration::from_millis(100));
                        }
                        Err(e) => {
                            let duration_ms = start.elapsed().as_millis() as u64;
                            let _ = child.kill();
                            let _ = child.wait();
                            let stdout_buf = join_pipe_reader(stdout_handle.take());
                            let stderr_buf = join_pipe_reader(stderr_handle.take());
                            let err_log = format!(
                                "wait error: {}\nstdout:\n{}\nstderr:\n{}",
                                e, stdout_buf, stderr_buf
                            );
                            let ev_path =
                                save_evidence(&evidence_dir, &gate.kind, attempt, &err_log);
                            break GateResult {
                                kind: gate.kind.clone(),
                                status: "failed".into(),
                                attempt,
                                timestamp: Utc::now().to_rfc3339(),
                                log: Some(err_log),
                                exit_code: None,
                                evidence_path: ev_path,
                                command: Some(cmd_args.clone()),
                                duration_ms: Some(duration_ms),
                                waiver_expires_at: None,
                                kind_class: None,
                            };
                        }
                    }
                }
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                let err_log = format!("execution error: {}", e);
                let ev_path = save_evidence(&evidence_dir, &gate.kind, attempt, &err_log);
                GateResult {
                    kind: gate.kind.clone(),
                    status: "failed".into(),
                    attempt,
                    timestamp: Utc::now().to_rfc3339(),
                    log: Some(err_log),
                    exit_code: None,
                    evidence_path: ev_path,
                    command: Some(cmd_args.clone()),
                    duration_ms: Some(duration_ms),
                    waiver_expires_at: None,
                    kind_class: None,
                }
            }
        };

        results.push(gate_result);
    }

    Ok(results)
}

/// Effective gates = tier defaults UNION spec-declared gates, deduped by kind
/// (defaults first, then any extra spec gate, order-stable).
///
/// Tier defaults are ALWAYS enforced: a spec's `gates:` list can only ADD
/// assurance, never remove a tier-mandated gate. Previously a non-empty
/// `spec.gates` *replaced* the defaults, so a high-tier spec listing only
/// `schema` silently dropped the mandatory `graph` + `review` gates — a
/// control-plane bypass. An empty `spec.gates` still resolves to the defaults.
pub fn effective_gates_for_spec(spec: &crate::models::Spec) -> Vec<crate::models::Gate> {
    let mut gates = default_gates_for_tier(&spec.tier);
    let mut seen: std::collections::HashSet<String> =
        gates.iter().map(|g| g.kind.clone()).collect();
    for g in &spec.gates {
        if seen.insert(g.kind.clone()) {
            gates.push(g.clone());
        }
    }
    gates
}

fn spawn_pipe_reader<R>(mut reader: R) -> JoinHandle<String>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = reader.read_to_end(&mut bytes);
        String::from_utf8_lossy(&bytes).into_owned()
    })
}

fn join_pipe_reader(handle: Option<JoinHandle<String>>) -> String {
    handle
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default()
}

pub fn default_gates_for_tier(tier: &crate::models::Tier) -> Vec<crate::models::Gate> {
    use crate::models::{Gate, Tier};
    match tier {
        Tier::Note => vec![],
        Tier::Decision => vec![Gate {
            kind: "review".to_string(),
        }],
        Tier::Lightweight => vec![Gate {
            kind: "schema".to_string(),
        }],
        // Default gates only include kinds with built-in handlers (schema, graph, review, architecture).
        // lint, test, contract require user-configured commands in config.yaml —
        // add them to spec.gates manually when commands are set up.
        Tier::Standard => vec![
            Gate {
                kind: "schema".to_string(),
            },
            Gate {
                kind: "graph".to_string(),
            },
        ],
        Tier::High => vec![
            Gate {
                kind: "schema".to_string(),
            },
            Gate {
                kind: "graph".to_string(),
            },
            Gate {
                kind: "review".to_string(),
            },
        ],
    }
}

/// Save full gate output to .s5d/evidence/{spec_id}/{kind}_{attempt}.log
/// Returns relative path on success, None on failure.
fn save_evidence(evidence_dir: &Path, kind: &str, attempt: u32, content: &str) -> Option<String> {
    if std::fs::create_dir_all(evidence_dir).is_err() {
        return None;
    }
    let filename = format!("{}_{}.log", kind, attempt);
    let full_path = evidence_dir.join(&filename);
    if std::fs::write(&full_path, content).is_ok() {
        // Return path relative to .s5d parent (project root)
        // evidence_dir is .s5d/evidence/{spec_id}, so relative = evidence/{spec_id}/{file}
        if let Some(s5d_parent) = evidence_dir.parent().and_then(|p| p.parent()) {
            full_path
                .strip_prefix(s5d_parent)
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        } else {
            Some(full_path.to_string_lossy().to_string())
        }
    } else {
        None
    }
}

fn truncate_log(log: &mut String) {
    if log.len() > MAX_LOG_BYTES {
        log.truncate(MAX_LOG_BYTES);
        log.push_str("\n... [truncated at 10KB]");
    }
}

fn count_attempts(results: &[GateResult], kind: &str) -> u32 {
    results.iter().filter(|r| r.kind == kind).count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Gate, Tier, WorkflowPhaseRecord, WorkflowPhaseStatus};

    fn kinds(spec: &crate::models::Spec) -> Vec<String> {
        effective_gates_for_spec(spec)
            .iter()
            .map(|g| g.kind.clone())
            .collect()
    }

    #[test]
    fn high_spec_cannot_drop_mandatory_gates_via_narrow_gate_list() {
        // Regression lock for the gate-bypass: effective gates used to *replace*
        // tier defaults when spec.gates was non-empty, letting a high-tier spec
        // listing only `schema` shed the mandatory `graph` + `review` gates.
        let mut spec = crate::generate_spec("feat.x.y", Tier::High, "prod");
        spec.gates = vec![Gate {
            kind: "schema".into(),
        }];
        let k = kinds(&spec);
        assert!(k.contains(&"schema".to_string()), "{k:?}");
        assert!(
            k.contains(&"graph".to_string()),
            "high must keep graph: {k:?}"
        );
        assert!(
            k.contains(&"review".to_string()),
            "high must keep review: {k:?}"
        );
    }

    #[test]
    fn spec_gate_adds_assurance_without_duplicating_defaults() {
        let mut spec = crate::generate_spec("feat.x.y", Tier::High, "prod");
        spec.gates = vec![
            Gate {
                kind: "schema".into(),
            },
            Gate {
                kind: "test".into(),
            },
        ];
        let k = kinds(&spec);
        assert_eq!(
            k.iter().filter(|x| *x == "schema").count(),
            1,
            "no duplicate schema: {k:?}"
        );
        assert!(k.contains(&"test".to_string()), "extra gate added: {k:?}");
        assert!(k.contains(&"review".to_string()), "default kept: {k:?}");
    }

    #[test]
    fn empty_spec_gates_still_resolves_to_tier_defaults() {
        let spec = crate::generate_spec("feat.x.y", Tier::Standard, "prod");
        // generate_spec may seed gates; force the empty case explicitly.
        let mut spec = spec;
        spec.gates = vec![];
        let k = kinds(&spec);
        assert!(
            k.contains(&"schema".to_string()) && k.contains(&"graph".to_string()),
            "{k:?}"
        );
    }

    fn accepted_phase_record() -> WorkflowPhaseRecord {
        WorkflowPhaseRecord {
            phase_id: "implement".into(),
            status: WorkflowPhaseStatus::Accepted,
            timestamp: "2026-01-01T00:00:00Z".into(),
            reviewer: Some("alice".into()),
            engine: None,
            notes: None,
        }
    }

    #[test]
    fn review_gate_closes_on_accepted_phase_for_standard_tier() {
        let mut spec = crate::generate_spec("feat.x.y", Tier::Standard, "prod");
        spec.gates = vec![Gate {
            kind: "review".into(),
        }];
        let phase_history = vec![accepted_phase_record()];
        let (status, log) = eval_review_gate(&spec, &phase_history);
        assert_eq!(status, "passed", "expected passed, got log: {log}");
        assert!(
            log.contains("accepted run phase"),
            "log should mention accepted run phase: {log}"
        );
    }

    #[test]
    fn review_gate_fails_without_accepted_phase_for_standard_tier() {
        let mut spec = crate::generate_spec("feat.x.y", Tier::Standard, "prod");
        spec.gates = vec![Gate {
            kind: "review".into(),
        }];
        // empty phase history — no Accepted entries
        let (status, _log) = eval_review_gate(&spec, &[]);
        assert_eq!(status, "failed");
    }

    #[test]
    fn accepted_phase_does_not_close_review_for_high_tier() {
        let mut spec = crate::generate_spec("feat.x.y", Tier::High, "prod");
        spec.gates = vec![Gate {
            kind: "review".into(),
        }];
        // Phase history has an Accepted entry but no gate:review evidence
        let phase_history = vec![accepted_phase_record()];
        let (status, log) = eval_review_gate(&spec, &phase_history);
        assert_eq!(
            status, "failed",
            "High tier must not close review via phase acceptance; got log: {log}"
        );
    }

    #[test]
    fn feature_tier_review_gate_requires_workflow() {
        use crate::validate_spec;

        // Standard spec with review gate and no workflow → should have the error
        let mut spec = crate::generate_spec("feat.x.y", Tier::Standard, "prod");
        spec.gates = vec![Gate {
            kind: "review".into(),
        }];
        spec.workflow = None;
        let errors = validate_spec(&spec);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("declares a review gate but has no workflow")),
            "expected workflow-required error, got: {:?}",
            errors
        );

        // Standard spec with review gate AND a workflow → no such error
        let spec_with_workflow = crate::generate_spec("feat.x.y", Tier::Standard, "prod");
        // generate_spec already sets workflow = Some(...); confirm review gate doesn't error
        let mut spec_wf = spec_with_workflow;
        spec_wf.gates = vec![Gate {
            kind: "review".into(),
        }];
        assert!(
            spec_wf.workflow.is_some(),
            "generate_spec should set workflow"
        );
        let errors2 = validate_spec(&spec_wf);
        assert!(
            !errors2
                .iter()
                .any(|e| e.contains("declares a review gate but has no workflow")),
            "should not error when workflow present: {:?}",
            errors2
        );
    }
}
