//! `harness` command handlers.
//!
//! Operational worktree harness: starts an isolated git worktree for a work
//! state, reports status, and runs bounded commands against it. Shares three
//! crate-root helpers (`load_spec_context`, `project_relative_path`,
//! `ensure_phase_execution_ready`); everything else is harness-owned.

use crate::{ensure_phase_execution_ready, load_spec_context, project_relative_path};
use colored::Colorize;

fn harness_state_path(project: &s5d::S5dProject, name: &str) -> std::path::PathBuf {
    project
        .s5d_dir()
        .join("harness")
        .join(format!("{}.yaml", name))
}

fn load_harness_state(project: &s5d::S5dProject, name: &str) -> anyhow::Result<s5d::HarnessState> {
    s5d::sanitize_id(name)?;
    let path = harness_state_path(project, name);
    let content = std::fs::read_to_string(&path)
        .map_err(|err| anyhow::anyhow!("failed to read harness '{}': {}", name, err))?;
    serde_yaml::from_str(&content)
        .map_err(|err| anyhow::anyhow!("failed to parse harness '{}': {}", name, err))
}

fn save_harness_state(project: &s5d::S5dProject, state: &s5d::HarnessState) -> anyhow::Result<()> {
    let dir = project.s5d_dir().join("harness");
    std::fs::create_dir_all(&dir)?;
    let path = harness_state_path(project, &state.id);
    std::fs::write(path, serde_yaml::to_string(state)?)?;
    Ok(())
}

fn append_harness_event(
    state: &mut s5d::HarnessState,
    kind: &str,
    message: impl Into<String>,
    command: Vec<String>,
    exit_code: Option<i32>,
    stdout_path: Option<String>,
    stderr_path: Option<String>,
) {
    let now = chrono::Utc::now().to_rfc3339();
    let message = message.into();
    state.updated_at = now.clone();
    state.heartbeat_at = now.clone();
    state.last_event = Some(message.clone());
    state.events.push(s5d::HarnessEvent {
        timestamp: now,
        kind: kind.to_string(),
        message,
        command,
        exit_code,
        stdout_path,
        stderr_path,
    });
}

fn git_raw_output(
    project: &s5d::S5dProject,
    args: &[&str],
) -> anyhow::Result<std::process::Output> {
    std::process::Command::new("git")
        .args(args)
        .current_dir(&project.root)
        .output()
        .map_err(|err| anyhow::anyhow!("failed to run git {}: {}", args.join(" "), err))
}

fn ensure_source_clean(project: &s5d::S5dProject) -> anyhow::Result<()> {
    let output = git_raw_output(project, &["status", "--porcelain"])?;
    if !output.status.success() {
        anyhow::bail!(
            "git status failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    if !output.stdout.is_empty() {
        anyhow::bail!(
            "source worktree is not clean; commit/stash local changes or rerun with --force\n{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
    Ok(())
}

fn default_harness_worktree(project: &s5d::S5dProject, name: &str) -> std::path::PathBuf {
    let repo_name = project
        .root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo");
    project
        .root
        .parent()
        .unwrap_or(project.root.as_path())
        .join(format!("{}-{}", repo_name, name))
}

fn spec_path_in_worktree(
    project: &s5d::S5dProject,
    spec_path: &std::path::Path,
    worktree: &std::path::Path,
) -> std::path::PathBuf {
    let absolute = if spec_path.is_absolute() {
        spec_path.to_path_buf()
    } else {
        project.root.join(spec_path)
    };
    absolute
        .strip_prefix(&project.root)
        .map(|relative| worktree.join(relative))
        .unwrap_or_else(|_| worktree.join(spec_path))
}

pub fn run_harness_start(
    spec_path: &str,
    phase_id: &str,
    name: &str,
    branch: Option<&str>,
    worktree: Option<&str>,
    force: bool,
) -> anyhow::Result<()> {
    s5d::sanitize_id(name)?;
    s5d::sanitize_id(phase_id)?;
    let cwd = std::env::current_dir()?;
    let project = s5d::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;
    project.ensure_dirs()?;
    if !force {
        ensure_source_clean(&project)?;
    }

    let (_project, spec_path_buf, spec, spec_filename) = load_spec_context(spec_path)?;
    let record = project.load_record(&spec_filename)?.ok_or_else(|| {
        anyhow::anyhow!("no record found for {} — run preview first", spec_filename)
    })?;
    ensure_phase_execution_ready(&spec, &record, phase_id)?;

    let state_path = harness_state_path(&project, name);
    if state_path.exists() {
        anyhow::bail!("harness '{}' already exists", name);
    }

    let branch = branch
        .map(str::to_string)
        .unwrap_or_else(|| format!("s5d/harness/{}", name));
    let worktree_path = worktree
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| default_harness_worktree(&project, name));
    if worktree_path.exists() {
        anyhow::bail!("worktree path already exists: {}", worktree_path.display());
    }
    if let Some(parent) = worktree_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let worktree_arg = worktree_path.to_string_lossy().into_owned();
    let output = git_raw_output(
        &project,
        &["worktree", "add", "-b", &branch, &worktree_arg, "HEAD"],
    )?;
    if !output.status.success() {
        anyhow::bail!(
            "git worktree add failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let now = chrono::Utc::now().to_rfc3339();
    let worktree_spec = spec_path_in_worktree(&project, &spec_path_buf, &worktree_path);
    let mut state = s5d::HarnessState {
        schema: "1.0".into(),
        id: name.to_string(),
        spec_ref: project_relative_path(&project, &spec_path_buf),
        phase_id: phase_id.to_string(),
        source_root: project.root.display().to_string(),
        worktree_path: worktree_path.display().to_string(),
        branch,
        status: "running".into(),
        created_at: now.clone(),
        updated_at: now.clone(),
        heartbeat_at: now,
        last_event: None,
        current_command: None,
        events: vec![],
    };
    append_harness_event(
        &mut state,
        "worktree_created",
        format!("Created worktree {}", worktree_path.display()),
        vec![],
        None,
        None,
        None,
    );
    save_harness_state(&project, &state)?;

    let s5d_bin = std::env::current_exe()?;
    let phase_output = std::process::Command::new(&s5d_bin)
        .args([
            "phase",
            "start",
            worktree_spec.to_string_lossy().as_ref(),
            "--id",
            phase_id,
        ])
        .current_dir(&worktree_path)
        .output()
        .map_err(|err| anyhow::anyhow!("failed to start phase in harness worktree: {}", err))?;
    let phase_command = vec![
        s5d_bin.display().to_string(),
        "run".into(),
        "start".into(),
        worktree_spec.display().to_string(),
        "--id".into(),
        phase_id.into(),
    ];
    if phase_output.status.success() {
        append_harness_event(
            &mut state,
            "work_state_started",
            format!("Started work state '{}'", phase_id),
            phase_command,
            phase_output.status.code(),
            None,
            None,
        );
        save_harness_state(&project, &state)?;
    } else {
        state.status = "failed".into();
        append_harness_event(
            &mut state,
            "phase_start_failed",
            String::from_utf8_lossy(&phase_output.stderr)
                .trim()
                .to_string(),
            phase_command,
            phase_output.status.code(),
            None,
            None,
        );
        save_harness_state(&project, &state)?;
        anyhow::bail!(
            "phase start failed in harness worktree:\n{}",
            String::from_utf8_lossy(&phase_output.stderr)
        );
    }

    println!("{} Harness started", "ok".green());
    println!("  {} {}", "id:".dimmed(), state.id);
    println!("  {} {}", "phase:".dimmed(), state.phase_id);
    println!("  {} {}", "worktree:".dimmed(), state.worktree_path);
    println!("  {} {}", "branch:".dimmed(), state.branch);
    println!(
        "  {} {}",
        "state:".dimmed(),
        project_relative_path(&project, &state_path)
    );
    Ok(())
}

pub fn run_harness_status(name: &str, stale_after_s: i64) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let project = s5d::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;
    let state = load_harness_state(&project, name)?;
    let heartbeat = chrono::DateTime::parse_from_rfc3339(&state.heartbeat_at)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .ok();
    let stale = heartbeat
        .map(|dt| chrono::Utc::now().signed_duration_since(dt).num_seconds() > stale_after_s)
        .unwrap_or(true);

    println!("{}: {}", "HARNESS".cyan().bold(), state.id.bold());
    println!("  {} {}", "status:".dimmed(), state.status);
    println!("  {} {}", "phase:".dimmed(), state.phase_id);
    println!("  {} {}", "spec:".dimmed(), state.spec_ref);
    println!("  {} {}", "worktree:".dimmed(), state.worktree_path);
    println!("  {} {}", "branch:".dimmed(), state.branch);
    println!(
        "  {} {}{}",
        "heartbeat:".dimmed(),
        state.heartbeat_at,
        if stale { " (stale)" } else { "" }
    );
    if let Some(ref current) = state.current_command {
        println!("  {} pid {}", "current:".dimmed(), current.pid);
        println!("    {}", current.argv.join(" "));
    } else {
        println!("  {} none", "current:".dimmed());
    }
    if let Some(ref event) = state.last_event {
        println!("  {} {}", "last event:".dimmed(), event);
    }
    if let Some(last) = state.events.last() {
        println!("  {} {}", "last kind:".dimmed(), last.kind);
    }
    Ok(())
}

fn render_harness_arg(state: &s5d::HarnessState, s5d_bin: &std::path::Path, arg: &str) -> String {
    arg.replace("{s5d}", &s5d_bin.display().to_string())
        .replace("{spec}", &state.spec_ref)
        .replace("{phase}", &state.phase_id)
        .replace("{worktree}", &state.worktree_path)
}

pub fn run_harness_exec(name: &str, timeout_s: u64, command: &[String]) -> anyhow::Result<()> {
    if command.is_empty() {
        anyhow::bail!("harness exec requires a command");
    }
    let cwd = std::env::current_dir()?;
    let project = s5d::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;
    let mut state = load_harness_state(&project, name)?;
    let s5d_bin = std::env::current_exe()?;
    let rendered = command
        .iter()
        .map(|arg| render_harness_arg(&state, &s5d_bin, arg))
        .collect::<Vec<_>>();

    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let command_dir = project
        .s5d_dir()
        .join("harness")
        .join(&state.id)
        .join("commands")
        .join(&timestamp);
    std::fs::create_dir_all(&command_dir)?;
    let stdout_path = command_dir.join("stdout.txt");
    let stderr_path = command_dir.join("stderr.txt");
    let stdout_file = std::fs::File::create(&stdout_path)?;
    let stderr_file = std::fs::File::create(&stderr_path)?;

    let mut child = std::process::Command::new(&rendered[0])
        .args(&rendered[1..])
        .current_dir(&state.worktree_path)
        .stdout(stdout_file)
        .stderr(stderr_file)
        .spawn()
        .map_err(|err| anyhow::anyhow!("failed to start harness command: {}", err))?;

    state.current_command = Some(s5d::HarnessCommandState {
        argv: rendered.clone(),
        pid: child.id(),
        started_at: chrono::Utc::now().to_rfc3339(),
        timeout_s,
    });
    append_harness_event(
        &mut state,
        "command_started",
        format!("Started command pid {}", child.id()),
        rendered.clone(),
        None,
        Some(project_relative_path(&project, &stdout_path)),
        Some(project_relative_path(&project, &stderr_path)),
    );
    save_harness_state(&project, &state)?;

    let started = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() > std::time::Duration::from_secs(timeout_s) {
            let _ = child.kill();
            let _ = child.wait();
            state.current_command = None;
            state.status = "timeout".into();
            append_harness_event(
                &mut state,
                "command_timeout",
                format!("Command killed after {}s timeout", timeout_s),
                rendered,
                None,
                Some(project_relative_path(&project, &stdout_path)),
                Some(project_relative_path(&project, &stderr_path)),
            );
            save_harness_state(&project, &state)?;
            anyhow::bail!("harness command timed out after {}s", timeout_s);
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    };

    state.current_command = None;
    if status.success() {
        state.status = "running".into();
        append_harness_event(
            &mut state,
            "command_completed",
            "Command completed",
            rendered,
            status.code(),
            Some(project_relative_path(&project, &stdout_path)),
            Some(project_relative_path(&project, &stderr_path)),
        );
        save_harness_state(&project, &state)?;
        println!("{} Harness command completed", "ok".green());
    } else {
        state.status = "failed".into();
        append_harness_event(
            &mut state,
            "command_failed",
            format!("Command exited with {:?}", status.code()),
            rendered,
            status.code(),
            Some(project_relative_path(&project, &stdout_path)),
            Some(project_relative_path(&project, &stderr_path)),
        );
        save_harness_state(&project, &state)?;
        anyhow::bail!("harness command failed with {:?}", status.code());
    }
    println!(
        "  {} {}",
        "stdout:".dimmed(),
        project_relative_path(&project, &stdout_path)
    );
    println!(
        "  {} {}",
        "stderr:".dimmed(),
        project_relative_path(&project, &stderr_path)
    );
    Ok(())
}
