//! `codebase` and `discover` command handlers.
//!
//! Thin presentation wrappers over the lib snapshot functions
//! (`build_/write_/load_codebase_snapshot`, `build_/write_/read_discovery_snapshot`).
//! The same logic is exposed over MCP in `s5d::mcp`.

use colored::Colorize;

pub fn run_codebase_sync() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let project = s5d::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;
    let snapshot = s5d::build_codebase_snapshot(&project)?;
    s5d::write_codebase_snapshot(&project, &snapshot)?;

    println!(
        "{} .s5d/codebase rebuilt ({} module(s): {} governed, {} partial, {} blind)",
        "ok".green(),
        snapshot.coverage.total_modules,
        snapshot.coverage.governed,
        snapshot.coverage.partial,
        snapshot.coverage.blind
    );
    Ok(())
}

pub fn run_codebase_check() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let project = s5d::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;
    let expected = s5d::build_codebase_snapshot(&project)?;
    let Some(actual) = s5d::load_codebase_snapshot(&project)? else {
        eprintln!(
            "  {} .s5d/codebase snapshot missing — run `s5d codebase sync`",
            "error:".red()
        );
        std::process::exit(1);
    };

    if actual == expected {
        println!(
            "{} .s5d/codebase is current ({} module(s): {} governed, {} partial, {} blind)",
            "ok".green(),
            expected.coverage.total_modules,
            expected.coverage.governed,
            expected.coverage.partial,
            expected.coverage.blind
        );
    } else {
        eprintln!(
            "  {} .s5d/codebase is stale — run `s5d codebase sync`",
            "error:".red()
        );
        std::process::exit(1);
    }
    Ok(())
}

pub fn run_discover_sync(path: Option<&str>, out: &std::path::Path) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let project = s5d::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;
    let target = path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| project.root.clone());
    let snapshot = s5d::build_discovery_snapshot(&project, &target)?;
    s5d::write_discovery_snapshot(&project, out, &snapshot)?;

    println!(
        "{} .s5d/discovery rebuilt ({} file(s), {} node(s), {} edge(s), {} evidence item(s))",
        "ok".green(),
        snapshot.manifest.files,
        snapshot.manifest.nodes,
        snapshot.manifest.edges,
        snapshot.manifest.evidence
    );
    Ok(())
}

pub fn run_discover_check(path: Option<&str>, out: &std::path::Path) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let project = s5d::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;
    let target = path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| project.root.clone());
    let expected = s5d::build_discovery_snapshot(&project, &target)?;
    let out_dir = if out.is_absolute() {
        out.to_path_buf()
    } else {
        project.root.join(out)
    };
    let Some(actual) = s5d::read_discovery_snapshot(&out_dir)? else {
        eprintln!(
            "  {} .s5d/discovery snapshot missing — run `s5d discover sync`",
            "error:".red()
        );
        std::process::exit(1);
    };

    if actual == expected {
        println!(
            "{} .s5d/discovery is current ({} file(s), {} node(s), {} edge(s))",
            "ok".green(),
            expected.manifest.files,
            expected.manifest.nodes,
            expected.manifest.edges
        );
    } else {
        eprintln!(
            "  {} .s5d/discovery is stale — run `s5d discover sync`",
            "error:".red()
        );
        std::process::exit(1);
    }
    Ok(())
}
