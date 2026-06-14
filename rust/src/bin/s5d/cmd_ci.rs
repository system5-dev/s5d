//! `ci` command handlers — generated CI enforcement (init / check / exec).
//!
//! The generated pipeline contains no check logic: it installs a pinned
//! release binary and calls `s5d ci exec`, which runs built-in read-only
//! checks only: spec validation, component path/architecture marker checks,
//! and drift checks. Config-driven gate_commands never execute here — a fork
//! PR must not be able to run repo-configured commands on the CI runner.

use colored::Colorize;

pub fn run_ci_init(github: bool, gitlab: bool, all: bool, force: bool) -> anyhow::Result<()> {
    let root = std::env::current_dir()?;
    let none = !github && !gitlab && !all;
    let mut targets = Vec::new();
    if github || all || none {
        targets.push(s5d::ci::CiTarget::Github);
    }
    if gitlab || all {
        targets.push(s5d::ci::CiTarget::Gitlab);
    }

    let written = s5d::ci::ci_init(&root, &targets, force)?;
    println!("{} CI config generated:", "ok".green());
    for p in &written {
        let rel = p.strip_prefix(&root).unwrap_or(p);
        println!("    {} {}", "✓".green(), rel.display());
    }

    let root_ci = root.join(".gitlab-ci.yml");
    if (gitlab || all) && !written.contains(&root_ci) {
        println!(
            "  {} .gitlab-ci.yml already exists — add this include to it:\n      include:\n        - local: .s5d/ci/s5d.gitlab-ci.yml",
            "note:".yellow()
        );
    }
    Ok(())
}

pub fn run_ci_check() -> anyhow::Result<()> {
    let root = std::env::current_dir()?;
    let report = s5d::ci::ci_check(&root)?;
    for f in &report.findings {
        println!("  {}", f);
    }
    if report.stale {
        anyhow::bail!("generated CI config is stale — re-run `s5d ci init`");
    }
    if report.findings.is_empty() {
        println!("{} generated CI config is current", "ok".green());
    }
    Ok(())
}

pub fn run_ci_exec() -> anyhow::Result<()> {
    let root = std::env::current_dir()?;
    if s5d::ci::ci_exec(&root)? {
        Ok(())
    } else {
        std::process::exit(1);
    }
}
