//! `install`, `update`, `init` command handlers + AGENTS.md / MCP registration.
//!
//! Provisioning surface: links skills, installs/updates the binary, registers
//! the MCP server in AGENTS.md and agent config files. Self-contained except
//! for `install_pre_commit_hook` (lives with the git-hook handlers in s5d.rs).

use crate::{install_pre_commit_hook, HookInstallResult};
use colored::Colorize;

// ── Install ───────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn run_install(
    s5d_path: Option<&str>,
    dry_run: bool,
    uninstall: bool,
    claude: bool,
    cursor: bool,
    codex: bool,
    gemini: bool,
    all: bool,
    global: bool,
) -> anyhow::Result<()> {
    // Resolve s5d binary path
    let binary_path = if let Some(p) = s5d_path {
        std::path::PathBuf::from(p)
    } else {
        std::env::current_exe()?
    };
    let binary_str = binary_path.to_string_lossy().into_owned();

    // No flags → default Claude at project level (legacy behavior).
    let no_flags = !claude && !cursor && !codex && !gemini && !all;
    let do_claude = claude || all || no_flags;
    let do_cursor = cursor || all;
    let do_codex = codex || all;
    let do_gemini = gemini || all;

    // Resolve target paths based on --global vs project.
    let home = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .map_err(|_| anyhow::anyhow!("HOME env var not set"))?;
    let cwd = std::env::current_dir()?;

    struct Target {
        name: &'static str,
        path: std::path::PathBuf,
        kind: TargetKind,
    }
    enum TargetKind {
        Json, // mcpServers.<name> in JSON
        Toml, // [mcp_servers.<name>] in TOML
    }

    let mut targets: Vec<Target> = Vec::new();
    if do_claude {
        targets.push(Target {
            name: "Claude",
            path: if global {
                home.join(".claude/settings.json")
            } else {
                cwd.join(".mcp.json")
            },
            kind: TargetKind::Json,
        });
    }
    if do_cursor {
        targets.push(Target {
            name: "Cursor",
            path: if global {
                home.join(".cursor/mcp.json")
            } else {
                cwd.join(".cursor/mcp.json")
            },
            kind: TargetKind::Json,
        });
    }
    if do_codex {
        targets.push(Target {
            name: "Codex",
            path: if global {
                home.join(".codex/config.toml")
            } else {
                cwd.join(".codex/config.toml")
            },
            kind: TargetKind::Toml,
        });
    }
    if do_gemini {
        targets.push(Target {
            name: "Gemini",
            path: if global {
                home.join(".gemini/settings.json")
            } else {
                cwd.join(".gemini/settings.json")
            },
            kind: TargetKind::Json,
        });
    }

    let scope = if global { "global" } else { "project" };
    let verb = if uninstall {
        "Uninstalling"
    } else {
        "Installing"
    };
    println!(
        "{} s5d MCP server ({} scope)",
        verb.to_string().bold(),
        scope
    );

    for t in &targets {
        let rel = t
            .path
            .strip_prefix(&home)
            .ok()
            .map(|p| format!("~/{}", p.display()))
            .unwrap_or_else(|| t.path.display().to_string());

        if dry_run {
            println!("    {} {} — {} (dry-run)", "•".cyan(), t.name, rel);
            continue;
        }

        let result: anyhow::Result<bool> = if uninstall {
            match &t.kind {
                TargetKind::Json => unregister_mcp_json(&t.path),
                TargetKind::Toml => unregister_mcp_toml(&t.path),
            }
        } else {
            match &t.kind {
                TargetKind::Json => register_mcp_json(&t.path, &binary_str),
                TargetKind::Toml => register_mcp_toml(&t.path, &binary_str),
            }
        };

        match result {
            Ok(false) => println!(
                "    {} {} — {} (already {})",
                "✓".green(),
                t.name,
                rel,
                if uninstall { "removed" } else { "registered" }
            ),
            Ok(true) => println!("    {} {} — {}", "✓".green(), t.name, rel),
            Err(e) => println!("    {} {} — {}: {}", "⚠".yellow(), t.name, rel, e),
        }
    }

    if !dry_run && !uninstall {
        println!();
        println!("   Command: {} mcp", binary_str);
        println!();
        println!(
            "   {} Restart your agent session to activate.",
            "⚠".yellow()
        );
    }
    Ok(())
}

/// Remove s5d entry from a JSON config file. Returns Ok(true) if modified.
fn unregister_mcp_json(path: &std::path::Path) -> anyhow::Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    let raw = std::fs::read_to_string(path)?;
    let mut settings: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or(serde_json::json!({}));
    let servers = match settings
        .as_object_mut()
        .and_then(|o| o.get_mut("mcpServers"))
        .and_then(|v| v.as_object_mut())
    {
        Some(s) => s,
        None => return Ok(false),
    };
    if servers.remove("s5d").is_none() {
        return Ok(false);
    }
    std::fs::write(path, serde_json::to_string_pretty(&settings)?)?;
    Ok(true)
}

/// Remove s5d entry from a TOML config file (Codex). Returns Ok(true) if modified.
fn unregister_mcp_toml(path: &std::path::Path) -> anyhow::Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    let raw = std::fs::read_to_string(path)?;
    // Naive line-based removal: drop [mcp_servers.s5d] block until next [section] or EOF.
    let mut out = String::with_capacity(raw.len());
    let mut skipping = false;
    let mut modified = false;
    for line in raw.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("[mcp_servers.s5d]") {
            skipping = true;
            modified = true;
            continue;
        }
        if skipping && trimmed.starts_with('[') && !trimmed.starts_with("[mcp_servers.s5d]") {
            skipping = false;
        }
        if !skipping {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !modified {
        return Ok(false);
    }
    std::fs::write(path, out)?;
    Ok(true)
}

// ── Update ───────────────────────────────────────────────────────────────────

const S5D_SKILLS: &[&str] = &["s5d", "fpf"];

#[derive(serde::Serialize)]
struct UpdateCheck {
    repo_root: Option<String>,
    current_version: String,
    current_commit: Option<String>,
    remote_commit: Option<String>,
    latest_tag: Option<String>,
    latest_version: Option<String>,
    update_available: bool,
    reason: Option<String>,
}

pub fn run_update_check(hook: bool, json: bool) -> anyhow::Result<()> {
    let check = match check_for_update() {
        Ok(check) => check,
        Err(error) if hook => {
            let check = UpdateCheck {
                repo_root: None,
                current_version: env!("CARGO_PKG_VERSION").to_string(),
                current_commit: None,
                remote_commit: None,
                latest_tag: None,
                latest_version: None,
                update_available: false,
                reason: Some(error.to_string()),
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&check)?);
            }
            return Ok(());
        }
        Err(error) => return Err(error),
    };

    if hook {
        if check.update_available {
            let latest = check
                .latest_tag
                .as_deref()
                .or(check.remote_commit.as_deref())
                .unwrap_or("newer upstream");
            let message = format!(
                "S5D update available: {} -> {}. Run: s5d admin update apply",
                check.current_version, latest
            );
            println!("{}", serde_json::json!({ "systemMessage": message }));
        }
        return Ok(());
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&check)?);
        return Ok(());
    }

    println!("S5D update check");
    println!("  current: {}", check.current_version);
    if let Some(ref root) = check.repo_root {
        println!("  repo: {}", root);
    }
    if let Some(ref commit) = check.current_commit {
        println!("  local: {}", short_commit(commit));
    }
    if let Some(ref commit) = check.remote_commit {
        println!("  remote: {}", short_commit(commit));
    }
    if let Some(ref tag) = check.latest_tag {
        println!("  latest tag: {}", tag);
    }
    if check.update_available {
        println!("  update: available");
        println!("  run: s5d admin update apply");
    } else {
        println!("  update: current");
    }
    Ok(())
}

/// Session-start self-update. Applies in a DETACHED background process when
/// every safety guard passes; otherwise degrades to the visible prompt of
/// `update check --hook`. Never fails the hosting hook: every error path in
/// the inner body is swallowed here — a broken log file or spawn failure must
/// not turn a session start into a hook error.
pub fn run_update_auto() -> anyhow::Result<()> {
    let _ = run_update_auto_inner();
    Ok(())
}

fn run_update_auto_inner() -> anyhow::Result<()> {
    if std::env::var("S5D_AUTO_UPDATE").as_deref() == Ok("0") {
        return run_update_check(true, false);
    }
    // No checkout / network trouble → behave exactly like the tolerant hook check.
    let Ok(check) = check_for_update() else {
        return Ok(());
    };
    if !check.update_available {
        return Ok(());
    }
    let Some(root) = check.repo_root.as_deref().map(std::path::PathBuf::from) else {
        return Ok(());
    };

    match auto_update_safety(&root) {
        None => {
            let log_path = home_dir()
                .map(|h| h.join(".s5d-update.log"))
                .unwrap_or_else(|| std::path::PathBuf::from("/tmp/s5d-update.log"));
            let log = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)?;
            let exe = std::env::current_exe()?;
            // --if-safe makes the child re-verify guards AFTER taking the
            // update lock: the pre-spawn check above is advisory only.
            std::process::Command::new(exe)
                .args(["admin", "update", "apply", "--if-safe"])
                .stdin(std::process::Stdio::null())
                .stdout(log.try_clone()?)
                .stderr(log)
                .spawn()?;
            // Version string only moves on releases; for commit-level updates
            // show the short SHAs so the line says what actually changes.
            let short = |c: &str| c[..8.min(c.len())].to_string();
            let (from, to) = if check.latest_version.as_deref() == Some(&check.current_version) {
                (
                    check.current_commit.as_deref().map(short),
                    check.remote_commit.as_deref().map(short),
                )
            } else {
                (
                    Some(check.current_version.clone()),
                    check.latest_version.clone(),
                )
            };
            println!(
                "s5d: update available ({} -> {}) — applying in background (log: {})",
                from.unwrap_or_else(|| "local".into()),
                to.unwrap_or_else(|| "origin HEAD".into()),
                log_path.display()
            );
        }
        Some(reason) => {
            println!("s5d: update available but auto-apply skipped: {}", reason);
            println!("  run manually: s5d admin update apply");
        }
    }
    Ok(())
}

/// Returns None when auto-apply is safe, otherwise the human-readable reason
/// it was skipped. Safe = clean working tree on the default branch — the only
/// state where `pull --ff-only` cannot collide with local work.
fn auto_update_safety(repo_root: &std::path::Path) -> Option<String> {
    match git_output(repo_root, &["status", "--porcelain"]) {
        Ok(out) if out.is_empty() => {}
        Ok(_) => return Some("working tree has local changes".into()),
        Err(e) => return Some(format!("cannot inspect working tree: {e}")),
    }
    let branch = match git_output(repo_root, &["rev-parse", "--abbrev-ref", "HEAD"]) {
        Ok(b) => b,
        Err(e) => return Some(format!("cannot resolve branch: {e}")),
    };
    // Fail closed when the default branch cannot be determined — guessing
    // "main" would auto-apply on the wrong branch for master/trunk repos.
    let Some(default_branch) = git_output(repo_root, &["symbolic-ref", "refs/remotes/origin/HEAD"])
        .ok()
        .and_then(|r| r.rsplit('/').next().map(str::to_string))
    else {
        return Some("cannot determine the default branch (origin/HEAD unset)".into());
    };
    if branch != default_branch {
        return Some(format!("checkout is on '{branch}', not '{default_branch}'"));
    }
    None
}

pub fn run_update_apply(dry_run: bool, if_safe: bool) -> anyhow::Result<()> {
    let repo_root = locate_s5d_repo_root().ok_or_else(|| {
        anyhow::anyhow!("cannot locate S5D checkout. Re-run install.sh from a cloned s5d repo.")
    })?;
    let destination = update_binary_destination(&repo_root)?;

    println!("S5D update apply");
    println!("  repo: {}", repo_root.display());
    println!("  binary: {}", destination.display());

    if dry_run {
        println!("  dry-run: would fetch, fast-forward, relink skills, and replace binary");
        return Ok(());
    }

    // One update at a time: concurrent session starts may each spawn an
    // apply; the loser exits gracefully instead of racing pull/build/install.
    let Some(_lock) = UpdateLock::acquire(&repo_root)? else {
        println!("another s5d update is already in progress — skipping");
        return Ok(());
    };

    // Auto mode re-verifies guards AFTER taking the lock: the spawner's
    // pre-check is advisory and the tree may have changed since.
    if if_safe {
        if let Some(reason) = auto_update_safety(&repo_root) {
            println!("auto-update skipped: {}", reason);
            return Ok(());
        }
    }

    run_git_command(&repo_root, &["fetch", "--tags", "--prune"])?;
    run_git_command(&repo_root, &["pull", "--ff-only"])?;
    install_skills_from_repo(&repo_root)?;
    install_binary_from_repo(&repo_root, &destination)?;
    println!("{} S5D updated", "ok".green());
    Ok(())
}

/// Advisory cross-process lock for update apply. Lives in .git/ (always
/// present in a checkout, never committed).
///
/// Owner-aware by token: Drop removes the file only while it still holds OUR
/// token, so a successor that legitimately broke a stale lock is never
/// unlocked by the previous holder exiting late (the A-breaks/B-takes/A-drops
/// TOCTOU a tribunal round caught). Staleness = holder process provably dead
/// (kill -0), with an mtime fallback only when liveness cannot be determined.
struct UpdateLock {
    path: std::path::PathBuf,
    token: String,
}

impl UpdateLock {
    fn acquire(repo_root: &std::path::Path) -> anyhow::Result<Option<Self>> {
        let path = repo_root.join(".git").join("s5d-update.lock");
        let token = format!(
            "{}:{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        for attempt in 0..2 {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(mut f) => {
                    use std::io::Write;
                    let _ = write!(f, "{token}");
                    return Ok(Some(Self { path, token }));
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    if attempt == 0 && Self::holder_is_stale(&path) {
                        let _ = std::fs::remove_file(&path);
                        continue;
                    }
                    return Ok(None);
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(None)
    }

    fn holder_is_stale(path: &std::path::Path) -> bool {
        match Self::holder_alive(path) {
            // Provably dead holder → stale regardless of age.
            Some(false) => true,
            // Provably alive holder → never stale (long release builds are legit).
            Some(true) => false,
            // Liveness unknown → conservative mtime fallback.
            None => std::fs::metadata(path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age.as_secs() > 30 * 60),
        }
    }

    #[cfg(unix)]
    fn holder_alive(path: &std::path::Path) -> Option<bool> {
        let token = std::fs::read_to_string(path).ok()?;
        let pid: u32 = token.split(':').next()?.trim().parse().ok()?;
        let status = std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .ok()?
            .status;
        Some(status.success())
    }

    #[cfg(not(unix))]
    fn holder_alive(_path: &std::path::Path) -> Option<bool> {
        None
    }
}

impl Drop for UpdateLock {
    fn drop(&mut self) {
        // Remove only OUR lock: if the file no longer holds our token, a
        // successor owns it and must keep running locked.
        if std::fs::read_to_string(&self.path)
            .map(|c| c == self.token)
            .unwrap_or(false)
        {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn check_for_update() -> anyhow::Result<UpdateCheck> {
    let repo_root =
        locate_s5d_repo_root().ok_or_else(|| anyhow::anyhow!("cannot locate S5D checkout"))?;
    let current_commit = git_output(&repo_root, &["rev-parse", "HEAD"]).ok();
    let remote_commit = git_output(
        &repo_root,
        &[
            "-c",
            "http.lowSpeedLimit=1",
            "-c",
            "http.lowSpeedTime=5",
            "ls-remote",
            "origin",
            "HEAD",
        ],
    )
    .ok()
    .and_then(|out| out.split_whitespace().next().map(str::to_string));
    let latest_tag = latest_remote_tag(&repo_root).ok().flatten();
    let latest_version = latest_tag
        .as_deref()
        .and_then(|tag| tag.strip_prefix('v'))
        .map(str::to_string);

    let version_update = latest_version
        .as_deref()
        .is_some_and(|latest| version_greater(latest, env!("CARGO_PKG_VERSION")));
    let commit_update = current_commit
        .as_deref()
        .zip(remote_commit.as_deref())
        .is_some_and(|(current, remote)| current != remote);

    Ok(UpdateCheck {
        repo_root: Some(repo_root.display().to_string()),
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        current_commit,
        remote_commit,
        latest_tag,
        latest_version,
        update_available: version_update || commit_update,
        reason: None,
    })
}

fn locate_s5d_repo_root() -> Option<std::path::PathBuf> {
    if let Ok(root) = std::env::var("S5D_ROOT") {
        let root = std::path::PathBuf::from(root);
        if is_s5d_repo_root(&root) {
            return Some(root);
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        for ancestor in exe.ancestors() {
            if is_s5d_repo_root(ancestor) {
                return Some(ancestor.to_path_buf());
            }
        }
    }

    for skill_root in installed_skill_roots() {
        if let Ok(canonical) = skill_root.canonicalize() {
            for ancestor in canonical.ancestors() {
                if is_s5d_repo_root(ancestor) {
                    return Some(ancestor.to_path_buf());
                }
            }
        }
    }

    None
}

fn installed_skill_roots() -> Vec<std::path::PathBuf> {
    let Some(home) = home_dir() else {
        return Vec::new();
    };
    vec![
        home.join(".agents/skills/s5d"),
        home.join(".claude/skills/s5d"),
        home.join(".codex/skills/s5d"),
        home.join(".gemini/skills/s5d"),
        home.join(".diana/src/skills/s5d"),
    ]
}

fn is_s5d_repo_root(path: &std::path::Path) -> bool {
    path.join("install.sh").is_file()
        && path.join("skills/s5d/SKILL.md").is_file()
        && path.join("rust/Cargo.toml").is_file()
}

fn latest_remote_tag(repo_root: &std::path::Path) -> anyhow::Result<Option<String>> {
    let out = git_output(
        repo_root,
        &[
            "-c",
            "http.lowSpeedLimit=1",
            "-c",
            "http.lowSpeedTime=5",
            "ls-remote",
            "--tags",
            "--refs",
            "origin",
            "v*",
        ],
    )?;
    Ok(out
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .filter_map(|reference| reference.strip_prefix("refs/tags/"))
        .max_by(|a, b| compare_versions(version_part(a), version_part(b)))
        .map(str::to_string))
}

fn git_output(repo_root: &std::path::Path, args: &[&str]) -> anyhow::Result<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()?;
    if !output.status.success() {
        anyhow::bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_git_command(repo_root: &std::path::Path, args: &[&str]) -> anyhow::Result<()> {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .status()?;
    if !status.success() {
        anyhow::bail!("git {} failed", args.join(" "));
    }
    Ok(())
}

fn install_skills_from_repo(repo_root: &std::path::Path) -> anyhow::Result<()> {
    let Some(home) = home_dir() else {
        anyhow::bail!("HOME is not set");
    };

    let mut targets = vec![home.join(".agents/skills")];
    if home.join(".claude").is_dir() {
        targets.push(home.join(".claude/skills"));
    }
    // Codex + Gemini: skipped — both read ~/.agents/skills natively.
    // Duplicating into ~/.codex/skills or ~/.gemini/skills triggers
    // "Skill conflict detected" warnings via double-load. Use codex-plugin /
    // gemini-extension.json manifests for native discovery.
    if home.join(".diana/src/skills").is_dir() {
        targets.push(home.join(".diana/src/skills"));
    }

    for target in targets {
        std::fs::create_dir_all(&target)?;
        for skill in S5D_SKILLS {
            replace_symlink(&repo_root.join("skills").join(skill), &target.join(skill))?;
        }
    }
    Ok(())
}

fn install_binary_from_repo(
    repo_root: &std::path::Path,
    destination: &std::path::Path,
) -> anyhow::Result<()> {
    // Build from the checked-out source first: it always matches the repo
    // revision. The tracked prebuilt is refreshed only at release time and
    // has shipped stale binaries silently (alpha.7 → alpha.8 both lagged) —
    // it is a fallback ONLY for hosts without a Rust toolchain. A failed
    // build on a host that HAS cargo is an error, not a fallback: silently
    // installing a lagging prebuilt over broken source recreates the very
    // divergence this path exists to prevent. The explicit --target-dir
    // pins the output location regardless of CARGO_TARGET_DIR/config.
    let cargo_build = std::process::Command::new("cargo")
        .args(["build", "--release", "--target-dir", "target"])
        .current_dir(repo_root.join("rust"))
        .status();
    let source = match cargo_build {
        Ok(status) if status.success() => repo_root.join("rust/target/release/s5d"),
        Ok(status) => anyhow::bail!(
            "cargo build --release failed ({status}) — fix the build; refusing to install a prebuilt over broken source"
        ),
        Err(e) => {
            let reason = format!("cargo unavailable ({e})");
            match prebuilt_binary(repo_root) {
                Some(prebuilt) => {
                    eprintln!(
                        "  warn: {} — installing tracked prebuilt {} (may lag the repo revision)",
                        reason,
                        prebuilt.display()
                    );
                    prebuilt
                }
                None => anyhow::bail!("{} and no tracked prebuilt for this platform", reason),
            }
        }
    };

    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = destination.with_extension(format!("s5d-update-tmp.{}", std::process::id()));
    std::fs::copy(&source, &tmp)?;
    make_executable(&tmp)?;
    std::fs::rename(&tmp, destination)?;
    Ok(())
}

fn update_binary_destination(repo_root: &std::path::Path) -> anyhow::Result<std::path::PathBuf> {
    if let Ok(path) = std::env::var("S5D_BIN_PATH") {
        return Ok(std::path::PathBuf::from(path));
    }
    let current = std::env::current_exe()?;
    if current.starts_with(repo_root.join("rust/target")) {
        let Some(home) = home_dir() else {
            anyhow::bail!("HOME is not set");
        };
        return Ok(home.join("bin/s5d"));
    }
    Ok(current)
}

fn prebuilt_binary(repo_root: &std::path::Path) -> Option<std::path::PathBuf> {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        "linux" => "linux",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        other => other,
    };
    let path = repo_root.join("bin").join(format!("s5d-{}-{}", os, arch));
    path.is_file().then_some(path)
}

fn replace_symlink(source: &std::path::Path, target: &std::path::Path) -> anyhow::Result<()> {
    if target.exists() || target.symlink_metadata().is_ok() {
        if target.is_dir() && !target.is_symlink() {
            std::fs::remove_dir_all(target)?;
        } else {
            std::fs::remove_file(target)?;
        }
    }
    install_skill_link(source, target)
}

#[cfg(unix)]
fn install_skill_link(source: &std::path::Path, target: &std::path::Path) -> anyhow::Result<()> {
    std::os::unix::fs::symlink(source, target)?;
    Ok(())
}

#[cfg(not(unix))]
fn install_skill_link(source: &std::path::Path, target: &std::path::Path) -> anyhow::Result<()> {
    copy_dir_all(source, target)
}

#[cfg(not(unix))]
fn copy_dir_all(source: &std::path::Path, target: &std::path::Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let destination = target.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &destination)?;
        } else {
            std::fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &std::path::Path) -> anyhow::Result<()> {
    Ok(())
}

fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from)
}

fn version_part(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}

fn version_greater(left: &str, right: &str) -> bool {
    compare_versions(left, right).is_gt()
}

fn compare_versions(left: &str, right: &str) -> std::cmp::Ordering {
    let left_parts = parse_version(left);
    let right_parts = parse_version(right);
    left_parts.cmp(&right_parts)
}

fn parse_version(version: &str) -> Vec<u64> {
    version
        .split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect()
}

fn short_commit(commit: &str) -> &str {
    commit.get(..8).unwrap_or(commit)
}

// ── Init ──────────────────────────────────────────────────────────────────────

pub fn run_init(
    claude: bool,
    cursor: bool,
    codex: bool,
    gemini: bool,
    all: bool,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let (project, report) = s5d::S5dProject::init(&cwd)?;

    println!(
        "\n{} S5D initialized at: {}\n",
        "✓".green(),
        project.s5d_dir().display()
    );

    println!("  {} Directories created:", "dirs".bold());
    for d in &report.dirs_created {
        println!("    {} {}", "✓".green(), d.display());
    }

    println!("\n  {} Files created:", "files".bold());
    for f in &report.files_created {
        println!("    {} {}", "✓".green(), f.display());
    }

    // Determine which tools to register for
    let no_flags = !claude && !cursor && !codex && !gemini && !all;
    let do_claude = claude || all || no_flags; // default: register for Claude
    let do_cursor = cursor || all;
    let do_codex = codex || all;
    let do_gemini = gemini || all;

    let binary_str = std::env::current_exe()?.to_string_lossy().into_owned();

    println!("\n  {} MCP server registration:", "mcp".bold());

    // Claude Code → .mcp.json
    if do_claude {
        match register_mcp_json(&cwd.join(".mcp.json"), &binary_str) {
            Ok(false) => println!(
                "    {} Claude (.mcp.json) — already registered",
                "✓".green()
            ),
            Ok(true) => println!("    {} Claude (.mcp.json)", "✓".green()),
            Err(e) => println!("    {} Claude: {}", "⚠".yellow(), e),
        }
    }

    // Cursor → .cursor/mcp.json
    if do_cursor {
        let cursor_path = cwd.join(".cursor").join("mcp.json");
        match register_mcp_json(&cursor_path, &binary_str) {
            Ok(false) => println!(
                "    {} Cursor (.cursor/mcp.json) — already registered",
                "✓".green()
            ),
            Ok(true) => println!("    {} Cursor (.cursor/mcp.json)", "✓".green()),
            Err(e) => println!("    {} Cursor: {}", "⚠".yellow(), e),
        }
    }

    // Codex CLI → .codex/config.toml
    if do_codex {
        let codex_path = cwd.join(".codex").join("config.toml");
        match register_mcp_toml(&codex_path, &binary_str) {
            Ok(false) => println!(
                "    {} Codex (.codex/config.toml) — already registered",
                "✓".green()
            ),
            Ok(true) => println!("    {} Codex (.codex/config.toml)", "✓".green()),
            Err(e) => println!("    {} Codex: {}", "⚠".yellow(), e),
        }
    }

    // Gemini CLI → .gemini/settings.json
    if do_gemini {
        let gemini_path = cwd.join(".gemini").join("settings.json");
        match register_mcp_json(&gemini_path, &binary_str) {
            Ok(false) => println!(
                "    {} Gemini (.gemini/settings.json) — already registered",
                "✓".green()
            ),
            Ok(true) => println!("    {} Gemini (.gemini/settings.json)", "✓".green()),
            Err(e) => println!("    {} Gemini: {}", "⚠".yellow(), e),
        }
    }

    // Auto-add mcp__s5d__* permission to .claude/settings.local.json (Claude-specific)
    if do_claude {
        let local_settings_path = cwd.join(".claude").join("settings.local.json");
        let perm_entry = "mcp__s5d__*";
        let needs_add = if local_settings_path.exists() {
            let raw = std::fs::read_to_string(&local_settings_path).unwrap_or_default();
            !raw.contains(perm_entry)
        } else {
            true
        };

        if needs_add {
            println!("\n  {} Permissions:", "perms".bold());
            let mut settings: serde_json::Value = if local_settings_path.exists() {
                let raw = std::fs::read_to_string(&local_settings_path)?;
                serde_json::from_str(&raw).unwrap_or(serde_json::json!({}))
            } else {
                serde_json::json!({})
            };

            let perms = settings
                .as_object_mut()
                .unwrap()
                .entry("permissions")
                .or_insert(serde_json::json!({}))
                .as_object_mut()
                .unwrap()
                .entry("allow")
                .or_insert(serde_json::json!([]))
                .as_array_mut()
                .unwrap();

            perms.push(serde_json::json!(perm_entry));

            if let Some(parent) = local_settings_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(
                &local_settings_path,
                serde_json::to_string_pretty(&settings)?,
            )?;
            println!(
                "    {} Added {} to {}",
                "✓".green(),
                perm_entry,
                local_settings_path.display()
            );
        }
    }

    println!("\n  {} Agent instructions:", "agents".bold());
    // AGENTS.md is universal (Codex, Junie, etc.) — always create.
    // CLAUDE.md / GEMINI.md are runtime-specific — inject only when the file
    // already exists, to avoid polluting projects that don't use that runtime.
    let agent_files = [
        ("AGENTS.md", true), // create_if_absent
        ("CLAUDE.md", false),
        ("GEMINI.md", false),
    ];
    for (filename, create_if_absent) in agent_files {
        let path = cwd.join(filename);
        if !create_if_absent && !path.exists() {
            println!("    {} {} — not present (skipped)", "·".dimmed(), filename);
            continue;
        }
        match ensure_agents_md(&path) {
            Ok(AgentsUpdate::Created) => println!("    {} {} — created", "✓".green(), filename),
            Ok(AgentsUpdate::Inserted) => {
                println!("    {} {} — s5d block appended", "✓".green(), filename)
            }
            Ok(AgentsUpdate::Replaced) => {
                println!("    {} {} — s5d block updated", "✓".green(), filename)
            }
            Ok(AgentsUpdate::Unchanged) => {
                println!("    {} {} — already up to date", "✓".green(), filename)
            }
            Err(e) => println!("    {} {}: {}", "⚠".yellow(), filename, e),
        }
    }

    println!("\n  {} Git hooks:", "hooks".bold());
    match install_pre_commit_hook(&cwd, &binary_str) {
        Ok(HookInstallResult::Installed(path)) => {
            println!("    {} pre-commit — {}", "✓".green(), path.display())
        }
        Ok(HookInstallResult::AlreadyInstalled(path)) => {
            println!(
                "    {} pre-commit — already installed ({})",
                "✓".green(),
                path.display()
            )
        }
        Ok(HookInstallResult::ExistingHook(path)) => {
            println!(
                "    {} pre-commit exists — left unchanged ({})",
                "⚠".yellow(),
                path.display()
            );
            println!("      Run `s5d hook pre-commit` from that hook to chain S5D checks.");
        }
        Ok(HookInstallResult::NoGit) => {
            println!(
                "    {} no .git directory — hook not installed",
                "skip".dimmed()
            )
        }
        Err(e) => println!("    {} pre-commit: {}", "⚠".yellow(), e),
    }

    // L1 + L2 enforcement hooks — pure Rust, no shell wrapper.
    // L1 = UserPromptSubmit advisory; L2 = PreToolUse(Edit|Write|MultiEdit) gate.
    // S5D-Spec: feat.s5d.pretool-enforcement
    println!("\n  {} S5D enforcement hooks:", "agents".bold());
    for path in s5d::hooks_json::target_hooks_paths(&cwd) {
        let rel = path.strip_prefix(&cwd).unwrap_or(&path);
        match s5d::hooks_json::ensure_all_s5d_hooks(&path) {
            Ok(s5d::hooks_json::HooksJsonUpdate::Created) => {
                println!("    {} {} — created", "✓".green(), rel.display())
            }
            Ok(s5d::hooks_json::HooksJsonUpdate::Inserted) => {
                println!("    {} {} — entry added", "✓".green(), rel.display())
            }
            Ok(s5d::hooks_json::HooksJsonUpdate::Unchanged) => {
                println!("    {} {} — already up to date", "✓".green(), rel.display())
            }
            Err(e) => println!("    {} {}: {}", "⚠".yellow(), rel.display(), e),
        }
    }

    println!(
        "\n  {} Restart your agent session to activate s5d MCP tools.",
        "⚠".yellow()
    );

    println!("\n  {} Next steps:", "→".blue());
    println!("    1. s5d new <feature-id> --product <name>   Create your first spec");
    println!("    2. s5d verify validate <spec>              Validate the spec");
    println!("    3. s5d state preview <spec>                Preview the import");
    println!();

    Ok(())
}

// ── AGENTS.md block ──────────────────────────────────────────────────────────

pub(crate) const AGENTS_BEGIN: &str = "<!-- s5d:begin -->";
pub(crate) const AGENTS_END: &str = "<!-- s5d:end -->";

pub(crate) fn agents_block() -> String {
    format!(
        "{}\n## S5D — Agentic Change Control Plane\n\n\
This repo uses **S5D** (https://github.com/system5-dev/s5d) to describe target state, \
record agent/tool evidence, bind architectural decisions, and verify that code still matches them.\n\n\
**⛔ S5D is MANDATORY for non-trivial work.** Architectural decisions, new features, \
refactors >30 LOC, and any change touching multiple modules MUST go through the S5D \
flow before implementation. Skip ONLY for: bug fixes <30 LOC, config-only, docs-only. \
`S5D_BYPASS=1` is an explicit break-glass escape hatch, not routine flow; document the \
justification when you use it. When in doubt, run `s5d_route` to classify the request.\n\n\
**Flow:** target state → edit spec → `s5d_validate` → `s5d_preview` → `s5d_approve` \
→ run/implement → `s5d_run_gates` → `s5d_import` → `s5d_drift_check`.\n\n\
**MCP tools** (prefer over shell CLI when available):\n\
- `s5d_route` — classify a request into tier/mode/entry\n\
- `s5d_codebase_sync` / `s5d_codebase_check` — codebase coverage snapshot\n\
- `s5d_discover_sync` / `s5d_discover_check` — discovery graph snapshot\n\
- `s5d_check` — architecture check (components vs. source paths)\n\
- `s5d_new` / `s5d_note` — create spec / quick note\n\
- `s5d_validate` / `s5d_preview` — dry-run checks before approval\n\
- `s5d_approve` / `s5d_import` — commit decision, bind SHA256 chain\n\
- `s5d_drift_check` / `s5d_reconcile` / `s5d_rollback` — verify & recover\n\
- `s5d_show` / `s5d_status` — inspect specs and project state\n\n\
**Commits reference specs.** When a change is governed by an S5D spec, include \
`S5D-Spec: <spec-id>` as a trailer in the commit body \
(e.g. `S5D-Spec: feat.s5d.structure-outline-and-vertical-phases`). \
This binds the commit to the decision record and lets `git log --grep='S5D-Spec:'` \
reconstruct the architectural rationale. Trivial changes that skipped S5D need no reference.\n\n\
Specs live in `.s5d/packages/`. Run `s5d --help` or read `skills/s5d/SKILL.md` for full reference.\n{}",
        AGENTS_BEGIN, AGENTS_END
    )
}

#[derive(Debug, PartialEq)]
pub(crate) enum AgentsUpdate {
    Created,
    Inserted,
    Replaced,
    Unchanged,
}

pub(crate) fn ensure_agents_md(path: &std::path::Path) -> anyhow::Result<AgentsUpdate> {
    let block = agents_block();

    if !path.exists() {
        std::fs::write(path, format!("{}\n", block))?;
        return Ok(AgentsUpdate::Created);
    }

    let existing = std::fs::read_to_string(path)?;

    if let (Some(start), Some(end_rel)) = (
        existing.find(AGENTS_BEGIN),
        existing[existing.find(AGENTS_BEGIN).unwrap_or(0)..].find(AGENTS_END),
    ) {
        let end = start + end_rel + AGENTS_END.len();
        if existing[start..end] == block {
            return Ok(AgentsUpdate::Unchanged);
        }
        let mut updated = String::with_capacity(existing.len() + block.len());
        updated.push_str(&existing[..start]);
        updated.push_str(&block);
        updated.push_str(&existing[end..]);
        std::fs::write(path, updated)?;
        return Ok(AgentsUpdate::Replaced);
    }

    let mut updated = existing;
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    if !updated.is_empty() && !updated.ends_with("\n\n") {
        updated.push('\n');
    }
    updated.push_str(&block);
    updated.push('\n');
    std::fs::write(path, updated)?;
    Ok(AgentsUpdate::Inserted)
}

/// Register s5d MCP server in a JSON config file (Claude, Cursor, Gemini format).
/// Returns Ok(true) if written, Ok(false) if already registered.
fn register_mcp_json(path: &std::path::Path, binary: &str) -> anyhow::Result<bool> {
    let desired = serde_json::json!({
        "command": binary,
        "args": ["mcp"]
    });

    let mut settings: serde_json::Value = if path.exists() {
        let raw = std::fs::read_to_string(path)?;
        serde_json::from_str(&raw).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let servers = settings
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("root is not an object"))?
        .entry("mcpServers")
        .or_insert(serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("mcpServers is not an object"))?;

    if let Some(existing) = servers.get("s5d") {
        if existing == &desired {
            return Ok(false);
        }
    }

    servers.insert("s5d".to_string(), desired);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(&settings)?)?;
    Ok(true)
}

/// Register s5d MCP server in a TOML config file (Codex CLI format).
/// Returns Ok(true) if written, Ok(false) if already registered.
fn register_mcp_toml(path: &std::path::Path, binary: &str) -> anyhow::Result<bool> {
    use toml_edit::{value, DocumentMut, Item, Table};

    let mut doc: DocumentMut = if path.exists() {
        let raw = std::fs::read_to_string(path)?;
        raw.parse::<DocumentMut>()?
    } else {
        DocumentMut::new()
    };

    // Check if [mcp_servers.s5d] already exists with correct values
    if let Some(mcp) = doc.get("mcp_servers") {
        if let Some(s5d) = mcp.get("s5d") {
            if let Some(cmd) = s5d.get("command") {
                if cmd.as_str() == Some(binary) {
                    return Ok(false);
                }
            }
        }
    }

    // Ensure [mcp_servers] table exists
    if doc.get("mcp_servers").is_none() {
        doc["mcp_servers"] = Item::Table(Table::new());
    }
    let mcp = doc["mcp_servers"].as_table_mut().unwrap();

    // Create [mcp_servers.s5d]
    if mcp.get("s5d").is_none() {
        mcp["s5d"] = Item::Table(Table::new());
    }
    let s5d = mcp["s5d"].as_table_mut().unwrap();
    s5d["command"] = value(binary);
    s5d["args"] = value(toml_edit::Array::from_iter(["mcp"]));

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, doc.to_string())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::auto_update_safety;
    use std::process::Command;

    fn git(dir: &std::path::Path, args: &[&str]) {
        assert!(Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap()
            .status
            .success());
    }

    fn seeded_repo() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        git(dir.path(), &["init", "-q", "-b", "main"]);
        git(dir.path(), &["config", "user.email", "t@example.test"]);
        git(dir.path(), &["config", "user.name", "T"]);
        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        git(dir.path(), &["add", "."]);
        git(dir.path(), &["commit", "-q", "-m", "init"]);
        // Self-remote so origin/HEAD resolves — the guard fails closed without it.
        git(dir.path(), &["remote", "add", "origin", "."]);
        git(dir.path(), &["fetch", "-q", "origin"]);
        git(
            dir.path(),
            &[
                "symbolic-ref",
                "refs/remotes/origin/HEAD",
                "refs/remotes/origin/main",
            ],
        );
        dir
    }

    #[test]
    fn auto_update_fails_closed_without_origin_head() {
        let dir = tempfile::TempDir::new().unwrap();
        git(dir.path(), &["init", "-q", "-b", "main"]);
        git(dir.path(), &["config", "user.email", "t@example.test"]);
        git(dir.path(), &["config", "user.name", "T"]);
        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        git(dir.path(), &["add", "."]);
        git(dir.path(), &["commit", "-q", "-m", "init"]);
        let reason = auto_update_safety(dir.path()).expect("no origin/HEAD must skip");
        assert!(reason.contains("default branch"), "{reason}");
    }

    #[test]
    fn auto_update_safe_on_clean_default_branch() {
        let repo = seeded_repo();
        // No origin/HEAD symbolic ref in a local-only repo → falls back to "main".
        assert_eq!(auto_update_safety(repo.path()), None);
    }

    #[test]
    fn auto_update_skipped_on_dirty_tree() {
        let repo = seeded_repo();
        std::fs::write(repo.path().join("b.txt"), "dirty").unwrap();
        let reason = auto_update_safety(repo.path()).expect("dirty tree must skip");
        assert!(reason.contains("local changes"), "{reason}");
    }

    #[test]
    fn update_lock_is_exclusive_and_released_on_drop() {
        let repo = seeded_repo();
        let first = super::UpdateLock::acquire(repo.path()).unwrap();
        assert!(first.is_some(), "first acquire must succeed");
        assert!(
            super::UpdateLock::acquire(repo.path()).unwrap().is_none(),
            "second concurrent acquire must be refused"
        );
        drop(first);
        assert!(
            super::UpdateLock::acquire(repo.path()).unwrap().is_some(),
            "lock must be released on drop"
        );
    }

    #[test]
    fn update_lock_breaks_lock_of_dead_holder() {
        let repo = seeded_repo();
        let lock_path = repo.path().join(".git").join("s5d-update.lock");
        // PID 999999 is far above macOS/Linux defaults — provably dead,
        // so the lock is stale regardless of its age.
        std::fs::write(&lock_path, "999999:0").unwrap();
        assert!(
            super::UpdateLock::acquire(repo.path()).unwrap().is_some(),
            "lock of a dead holder must be broken and retaken"
        );
    }

    #[test]
    fn update_lock_drop_spares_successor_lock() {
        let repo = seeded_repo();
        let lock_path = repo.path().join(".git").join("s5d-update.lock");
        let ours = super::UpdateLock::acquire(repo.path()).unwrap().unwrap();
        // A successor (simulated) broke and retook the lock with its token.
        std::fs::write(&lock_path, format!("{}:42", std::process::id())).unwrap();
        drop(ours);
        assert!(
            lock_path.exists(),
            "dropping a superseded lock must not unlock the successor"
        );
    }

    #[test]
    fn auto_update_skipped_on_feature_branch() {
        let repo = seeded_repo();
        git(repo.path(), &["checkout", "-q", "-b", "feature/x"]);
        let reason = auto_update_safety(repo.path()).expect("branch must skip");
        assert!(reason.contains("feature/x"), "{reason}");
    }
}
