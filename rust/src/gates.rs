use crate::models::*;
use chrono::Utc;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Maximum log output size (10 KB). Truncated if exceeded.
const MAX_LOG_BYTES: usize = 10 * 1024;

pub fn run_gates(
    spec: &Spec,
    config: &S5dConfig,
    spec_path: &str,
    project_root: &Path,
    s5d_dir: &Path,
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
                        let mut review_count: usize = 0;
                        let mut findings: Vec<String> = Vec::new();
                        for hyp in &spec.hypotheses {
                            for ev in &hyp.evidence {
                                let is_review = ev.evidence_type == "gate:review"
                                    || ev.evidence_type.starts_with("gate:review:");
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
                        let (status, log) = if review_count >= 1 {
                            (
                                "passed".into(),
                                format!(
                                    "review gate passed (built-in): {} review-evidence with verdict=pass\n{}",
                                    review_count,
                                    findings.join("\n")
                                ),
                            )
                        } else {
                            (
                                "failed".into(),
                                "review gate failed (built-in): no evidence with type=gate:review and verdict=pass found. \
                                Decision/high-tier specs require >=1 code review recorded as evidence. \
                                Use: s5d add-evidence <spec> --hypothesis-id <id> --evidence-type gate:review \
                                --content '<reviewer findings>' --verdict pass --formality <1-5> \
                                --claim-scope <scope> --reliability <0..1>".into(),
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

pub fn effective_gates_for_spec(spec: &crate::models::Spec) -> Vec<crate::models::Gate> {
    if spec.gates.is_empty() {
        default_gates_for_tier(&spec.tier)
    } else {
        spec.gates.clone()
    }
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
