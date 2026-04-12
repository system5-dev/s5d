use crate::models::*;
use chrono::Utc;
use std::path::Path;
use std::process::{Command, Stdio};
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

    for gate in &spec.gates {
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
                            ("passed".into(), "schema validation passed (built-in)".to_string())
                        } else {
                            ("failed".into(), format!("schema validation failed (built-in):\n{}", errors.join("\n")))
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
                        })
                    }
                    "graph" => {
                        let errors = crate::graph_check(spec);
                        let attempt = count_attempts(&results, &gate.kind) + 1;
                        let start = Instant::now();
                        let (status, log) = if errors.is_empty() {
                            ("passed".into(), "graph check passed (built-in)".to_string())
                        } else {
                            ("failed".into(), format!("graph check failed (built-in):\n{}", errors.join("\n")))
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
                        })
                    }
                    _ => None,
                };

                if let Some(result) = builtin_result {
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

                loop {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            let duration_ms = start.elapsed().as_millis() as u64;
                            let status_str = if status.success() { "passed" } else { "failed" };
                            let mut stdout_buf = String::new();
                            let mut stderr_buf = String::new();
                            if let Some(ref mut out) = child.stdout {
                                let _ = std::io::Read::read_to_string(out, &mut stdout_buf);
                            }
                            if let Some(ref mut err) = child.stderr {
                                let _ = std::io::Read::read_to_string(err, &mut stderr_buf);
                            }

                            let full_log = format!(
                                "exit: {}\nstdout:\n{}\nstderr:\n{}",
                                status, stdout_buf, stderr_buf
                            );

                            // Write full evidence to disk
                            let ev_path = save_evidence(
                                &evidence_dir,
                                &gate.kind,
                                attempt,
                                &full_log,
                            );

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
                            };
                        }
                        Ok(None) => {
                            if start.elapsed() > timeout_dur {
                                let duration_ms = start.elapsed().as_millis() as u64;
                                let _ = child.kill();
                                let _ = child.wait();
                                let timeout_log =
                                    format!("killed after {}s timeout", timeout_secs);
                                let ev_path = save_evidence(
                                    &evidence_dir,
                                    &gate.kind,
                                    attempt,
                                    &timeout_log,
                                );
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
                                };
                            }
                            std::thread::sleep(Duration::from_millis(100));
                        }
                        Err(e) => {
                            let duration_ms = start.elapsed().as_millis() as u64;
                            let err_log = format!("wait error: {}", e);
                            let ev_path = save_evidence(
                                &evidence_dir,
                                &gate.kind,
                                attempt,
                                &err_log,
                            );
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
                            };
                        }
                    }
                }
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                let err_log = format!("execution error: {}", e);
                let ev_path =
                    save_evidence(&evidence_dir, &gate.kind, attempt, &err_log);
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
                }
            }
        };

        results.push(gate_result);
    }

    Ok(results)
}

pub fn default_gates_for_tier(tier: &crate::models::Tier) -> Vec<crate::models::Gate> {
    use crate::models::{Gate, Tier};
    match tier {
        Tier::Note | Tier::Decision => vec![],
        Tier::Lightweight => vec![Gate {
            kind: "schema".to_string(),
        }],
        Tier::Standard => vec![
            Gate {
                kind: "schema".to_string(),
            },
            Gate {
                kind: "graph".to_string(),
            },
            Gate {
                kind: "lint".to_string(),
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
                kind: "lint".to_string(),
            },
            Gate {
                kind: "test".to_string(),
            },
            Gate {
                kind: "contract".to_string(),
            },
        ],
    }
}

/// Save full gate output to .s5d/evidence/{spec_id}/{kind}_{attempt}.log
/// Returns relative path on success, None on failure.
fn save_evidence(
    evidence_dir: &Path,
    kind: &str,
    attempt: u32,
    content: &str,
) -> Option<String> {
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
