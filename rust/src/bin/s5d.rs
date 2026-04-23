use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(
    name = "s5d",
    about = "S5D — decision and validation layer for repo changes"
)]
struct Cli {
    #[command(subcommand)]
    command: S5dCommand,
}

// ── Core commands — the critical path ────────────────────────────────────────
//
// intent → validate → approve → apply → detect drift → rollback
//

#[derive(Subcommand)]
enum S5dCommand {
    /// Initialize .s5d/ directory structure
    Init {
        /// Register MCP server for Claude Code (.mcp.json)
        #[arg(long)]
        claude: bool,
        /// Register MCP server for Cursor (.cursor/mcp.json)
        #[arg(long)]
        cursor: bool,
        /// Register MCP server for Codex CLI (.codex/config.toml)
        #[arg(long)]
        codex: bool,
        /// Register MCP server for Gemini CLI (.gemini/settings.json)
        #[arg(long)]
        gemini: bool,
        /// Register MCP server for all supported tools
        #[arg(long)]
        all: bool,
    },
    /// Create a new spec from template
    New {
        /// Feature ID (e.g., "feat.orders.tracking")
        feature_id: String,
        /// Spec tier
        #[arg(long, default_value = "standard")]
        tier: String,
        /// Product name
        #[arg(long)]
        product: Option<String>,
        /// Decision question (required for --tier decision)
        #[arg(long)]
        question: Option<String>,
        /// Note rationale (required for --tier note)
        #[arg(long)]
        rationale: Option<String>,
        /// Hypothesis ID in a parent decision spec to auto-link this spec as spec_ref
        #[arg(long)]
        hypothesis_id: Option<String>,
    },
    /// Quick note — one-shot shorthand for `s5d new note.<slug> --tier note`
    Note {
        /// Note text (used as title and rationale)
        text: Vec<String>,
        /// Product name
        #[arg(long)]
        product: Option<String>,
    },
    /// Add a hypothesis to a decision spec
    AddHypothesis {
        /// Path to .s5d.yaml file
        spec: String,
        /// Custom hypothesis ID (auto-generated from title if omitted)
        #[arg(long)]
        id: Option<String>,
        /// Hypothesis title
        #[arg(long)]
        title: String,
        /// Hypothesis content/description
        #[arg(long)]
        content: String,
        /// Scope — where this applies
        #[arg(long)]
        scope: String,
        /// Kind: system (default) or episteme (knowledge/methodology)
        #[arg(long, default_value = "system")]
        kind: String,
        /// Rationale JSON (anomaly, approach, alternatives)
        #[arg(long)]
        rationale: Option<String>,
    },
    /// Add evidence to a hypothesis in a decision spec
    AddEvidence {
        /// Path to .s5d.yaml file
        spec: String,
        /// Hypothesis ID to attach evidence to
        #[arg(long)]
        hypothesis_id: String,
        /// Evidence type: internal, external, gate:test, etc.
        #[arg(long)]
        evidence_type: String,
        /// Evidence content (test output, research findings)
        #[arg(long)]
        content: String,
        /// Verdict: pass, fail, refine
        #[arg(long)]
        verdict: String,
        /// Carrier reference (test file path, URL, etc.)
        #[arg(long)]
        carrier_ref: Option<String>,
        #[arg(long, help = "Rigor of evidence method (1-5)")]
        formality: Option<u8>,
        #[arg(long, help = "What the claim covers (comma-separated)")]
        claim_scope: Option<String>,
        #[arg(long, help = "Confidence that the claim is true (0.0-1.0)")]
        reliability: Option<f64>,
    },
    /// Record a decision in a decision spec
    Decide {
        /// Path to .s5d.yaml file
        spec: String,
        /// Decision title
        #[arg(long)]
        title: String,
        /// Winner hypothesis ID
        #[arg(long)]
        winner: String,
        /// Rejected hypothesis IDs (comma-separated)
        #[arg(long)]
        rejected: Option<String>,
        /// Decision context
        #[arg(long)]
        context: String,
        /// What was decided
        #[arg(long)]
        decision: String,
        /// Why this was chosen
        #[arg(long)]
        rationale: String,
        /// Expected consequences
        #[arg(long)]
        consequences: String,
        /// Skip spec_ref enforcement for winner hypothesis
        #[arg(long)]
        force: bool,
        /// Human who confirms the decision (required — non-waivable)
        #[arg(long)]
        confirmed_by: Option<String>,
        /// Adversarial challenge summary (required unless --no-challenge)
        #[arg(long)]
        challenge_summary: Option<String>,
        /// Challenge mode: tactical (1 probe) or standard (5 probes). Default: auto-detect from tier.
        #[arg(long)]
        challenge_mode: Option<String>,
        /// Skip adversarial challenge gate (not recommended)
        #[arg(long)]
        no_challenge: bool,
    },
    /// Validate a spec file
    Validate {
        /// Path to .s5d.yaml file
        spec: String,
    },
    /// Graph/relation validation — DFS cycle detection
    GraphCheck {
        /// Path to .s5d.yaml file
        spec: String,
    },
    /// Architecture linter — spec shape, graph rules, component paths, and source dependencies
    Check {
        /// Path to .s5d.yaml file
        spec: String,
        /// Output format: text or json
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Dry-run import diff
    Preview {
        /// Path to .s5d.yaml file
        spec: String,
    },
    /// Record approval binding spec_sha256 + diff_sha256
    Approve {
        /// Path to .s5d.yaml file
        spec: String,
        /// Reviewer name
        #[arg(long, default_value = "self")]
        reviewer: String,
        /// Block approval if reviewer is not a domain owner
        #[arg(long)]
        require_owner: bool,
    },
    /// Execute configured gate commands
    RunGates {
        /// Path to .s5d.yaml file
        spec: String,
    },
    /// Transactional import (apply spec to alias table + ledger)
    Import {
        /// Path to .s5d.yaml file
        spec: String,
        /// Who independently verified gates passed (trust separation)
        #[arg(long)]
        verified_by: Option<String>,
        /// Override methodological checks
        #[arg(long)]
        force: bool,
    },
    /// Show status of all specs
    Status,
    /// Show spec details — decision trace, hypothesis tree, or feature summary
    Show {
        /// Path to .s5d.yaml file
        spec: String,
    },
    /// Search specs and decisions by keyword
    Search {
        /// Search query
        query: String,
    },
    /// Compare live state vs last applied fingerprint
    DriftCheck {
        /// Path to .s5d.yaml file (optional: check all if omitted)
        spec: Option<String>,
    },
    /// Phase lifecycle for workflow-driven execution
    Phase {
        #[command(subcommand)]
        command: PhaseCommand,
    },
    /// Execute a bounded loop inside an approved phase
    Execute {
        #[command(subcommand)]
        command: ExecuteCommand,
    },
    /// Re-import to fix drift (desired-state restore, bypasses diff_sha256)
    Reconcile {
        /// Path to .s5d.yaml file (optional: reconcile all drifted if omitted)
        spec: Option<String>,
    },
    /// Reverse last import for a spec
    Rollback {
        /// Path to .s5d.yaml file
        spec: String,
    },
    /// Record reflection for a spec (OPERATE stage) — closes lifecycle with production evidence
    Reflect {
        /// Path to .s5d.yaml file
        spec: String,
        /// Outcome verdict: confirmed, refuted, inconclusive, iterate, kill
        #[arg(long)]
        verdict: Option<String>,
        /// Measurement window used for the verdict
        #[arg(long)]
        measurement_window: Option<String>,
        /// Summary of what happened in production
        #[arg(long)]
        summary: String,
        /// What worked well (comma-separated)
        #[arg(long, default_value = "")]
        worked: String,
        /// Issues encountered (comma-separated)
        #[arg(long, default_value = "")]
        issues: String,
        /// Follow-up tasks (comma-separated)
        #[arg(long, default_value = "")]
        follow_ups: String,
        /// Production evidence: paths, URLs, or metric descriptions (repeatable)
        #[arg(long = "evidence")]
        evidence: Vec<String>,
        /// Telemetry references backing the verdict (repeatable)
        #[arg(long = "telemetry")]
        telemetry_refs: Vec<String>,
        /// Reusable rules learned from this spec (repeatable)
        #[arg(long = "heuristic")]
        heuristics: Vec<String>,
        /// Structured issue: "description|root_cause|fix|severity" (repeatable)
        #[arg(long = "issue")]
        structured_issues: Vec<String>,
    },
    /// Classify a request into tier, mode, and entry point
    Route {
        /// Request description to classify
        description: Vec<String>,
        /// Output format: text or json
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Index management
    Index {
        #[command(subcommand)]
        command: IndexCommand,
    },
    /// Codebase ownership and coverage snapshot
    Codebase {
        #[command(subcommand)]
        command: CodebaseCommand,
    },
    /// Git hook entrypoints implemented in Rust
    Hook {
        #[command(subcommand)]
        command: HookCommand,
    },
    /// Check for and apply S5D binary/skill updates
    Update {
        #[command(subcommand)]
        command: UpdateCommand,
    },
    /// Seed alias table from bootstrap manifest
    Bootstrap {
        /// Path to bootstrap manifest YAML
        manifest: String,
    },
    /// Print environment fingerprint (tool versions hash)
    Cg,
    /// Start stdio MCP server (for Claude Code integration)
    Mcp,
    /// Register s5d MCP server for supported assistants.
    /// Default: project-level for Claude (.mcp.json). Use flags to target
    /// other assistants or --global to install at user level.
    Install {
        /// Explicit path to s5d binary (default: current executable)
        #[arg(long)]
        s5d_path: Option<String>,
        /// Print what would change, don't write
        #[arg(long)]
        dry_run: bool,
        /// Remove the s5d MCP entry
        #[arg(long)]
        uninstall: bool,
        /// Register for Claude Code
        #[arg(long)]
        claude: bool,
        /// Register for Cursor
        #[arg(long)]
        cursor: bool,
        /// Register for Codex CLI
        #[arg(long)]
        codex: bool,
        /// Register for Gemini CLI
        #[arg(long)]
        gemini: bool,
        /// Register for all supported assistants
        #[arg(long)]
        all: bool,
        /// Install at user/global level (~/.claude, ~/.codex, ~/.gemini) instead of project
        #[arg(long)]
        global: bool,
    },
}

#[derive(Subcommand)]
enum IndexCommand {
    /// Check index.yaml consistency
    Check,
    /// Rebuild index.yaml from packages/
    Sync,
}

#[derive(Subcommand)]
enum CodebaseCommand {
    /// Check .s5d/codebase/*.yaml against current source and specs
    Check,
    /// Rebuild .s5d/codebase/*.yaml from current source and specs
    Sync,
}

#[derive(Subcommand)]
enum HookCommand {
    /// Run the S5D pre-commit check over staged specs and architecture-gated source
    PreCommit,
}

#[derive(Subcommand)]
enum UpdateCommand {
    /// Check whether the installed S5D checkout differs from origin
    Check {
        /// Emit plugin hook JSON and never fail on network/setup errors
        #[arg(long)]
        hook: bool,
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
    },
    /// Update the checkout, relink skills, and replace the installed binary
    Apply {
        /// Show what would be updated without writing files
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum PhaseCommand {
    /// List workflow phases and current phase state
    List {
        /// Path to .s5d.yaml file
        spec: String,
    },
    /// Mark a workflow phase as active
    Start {
        /// Path to .s5d.yaml file
        spec: String,
        /// Workflow phase ID
        #[arg(long)]
        id: String,
    },
    /// Human acceptance for a workflow phase
    Accept {
        /// Path to .s5d.yaml file
        spec: String,
        /// Workflow phase ID
        #[arg(long)]
        id: String,
        /// Reviewer who accepts the phase
        #[arg(long)]
        reviewer: String,
    },
}

#[derive(Subcommand)]
enum ExecuteCommand {
    /// Emit a bounded task package for a workflow phase
    Loop {
        /// Path to .s5d.yaml file
        spec: String,
        /// Workflow phase ID
        #[arg(long)]
        phase: String,
        /// Execution engine name
        #[arg(long, default_value = "ralph")]
        engine: String,
        /// Optional Ralph run mode (default inferred from phase)
        #[arg(long)]
        mode: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    if invoked_as_git_hook("pre-commit") {
        return run_hook_pre_commit();
    }

    let cli = Cli::parse();
    match cli.command {
        S5dCommand::Init {
            claude,
            cursor,
            codex,
            gemini,
            all,
        } => run_init(claude, cursor, codex, gemini, all),
        S5dCommand::New {
            feature_id,
            tier,
            product,
            question,
            rationale,
            hypothesis_id,
        } => run_new(
            &feature_id,
            &tier,
            product.as_deref(),
            question.as_deref(),
            rationale.as_deref(),
            hypothesis_id.as_deref(),
        ),
        S5dCommand::Note { text, product } => run_note(&text.join(" "), product.as_deref()),
        S5dCommand::Validate { spec } => run_validate(&spec),
        S5dCommand::Status => run_status(),
        S5dCommand::Cg => run_cg(),
        S5dCommand::Preview { spec } => run_preview(&spec),
        S5dCommand::Approve {
            spec,
            reviewer,
            require_owner,
        } => run_approve(&spec, &reviewer, require_owner),
        S5dCommand::RunGates { spec } => run_gates(&spec),
        S5dCommand::Import {
            spec,
            verified_by,
            force,
        } => run_import(&spec, &verified_by, force),
        S5dCommand::Rollback { spec } => run_rollback(&spec),
        S5dCommand::Index { command } => match command {
            IndexCommand::Check => run_index_check(),
            IndexCommand::Sync => run_index_sync(),
        },
        S5dCommand::Codebase { command } => match command {
            CodebaseCommand::Check => run_codebase_check(),
            CodebaseCommand::Sync => run_codebase_sync(),
        },
        S5dCommand::Hook { command } => match command {
            HookCommand::PreCommit => run_hook_pre_commit(),
        },
        S5dCommand::Update { command } => match command {
            UpdateCommand::Check { hook, json } => run_update_check(hook, json),
            UpdateCommand::Apply { dry_run } => run_update_apply(dry_run),
        },
        S5dCommand::Bootstrap { manifest } => run_bootstrap(&manifest),
        S5dCommand::GraphCheck { spec } => run_graph_check(&spec),
        S5dCommand::Check { spec, format } => run_check(&spec, &format),
        S5dCommand::DriftCheck { spec } => run_drift_check(spec.as_deref()),
        S5dCommand::Phase { command } => match command {
            PhaseCommand::List { spec } => run_phase_list(&spec),
            PhaseCommand::Start { spec, id } => run_phase_start(&spec, &id),
            PhaseCommand::Accept { spec, id, reviewer } => run_phase_accept(&spec, &id, &reviewer),
        },
        S5dCommand::Execute { command } => match command {
            ExecuteCommand::Loop {
                spec,
                phase,
                engine,
                mode,
            } => run_execute_loop(&spec, &phase, &engine, mode.as_deref()),
        },
        S5dCommand::Reconcile { spec } => run_reconcile(spec.as_deref()),
        S5dCommand::AddHypothesis {
            spec,
            id,
            title,
            content,
            scope,
            kind,
            rationale,
        } => run_add_hypothesis(
            &spec,
            id.as_deref(),
            &title,
            &content,
            &scope,
            &kind,
            rationale.as_deref(),
        ),
        S5dCommand::AddEvidence {
            spec,
            hypothesis_id,
            evidence_type,
            content,
            verdict,
            carrier_ref,
            formality,
            claim_scope,
            reliability,
        } => run_add_evidence(
            &spec,
            &hypothesis_id,
            &evidence_type,
            &content,
            &verdict,
            carrier_ref.as_deref(),
            formality,
            claim_scope,
            reliability,
        ),
        S5dCommand::Decide {
            spec,
            title,
            winner,
            rejected,
            context,
            decision,
            rationale,
            consequences,
            force,
            confirmed_by,
            challenge_summary,
            challenge_mode,
            no_challenge,
        } => run_decide(
            &spec,
            &title,
            &winner,
            rejected.as_deref(),
            &context,
            &decision,
            &rationale,
            &consequences,
            force,
            confirmed_by.as_deref(),
            challenge_summary.as_deref(),
            challenge_mode.as_deref(),
            no_challenge,
        ),
        S5dCommand::Show { spec } => run_show(&spec),
        S5dCommand::Route {
            description,
            format,
        } => run_route(&description.join(" "), &format),
        S5dCommand::Search { query } => run_search(&query),
        S5dCommand::Reflect {
            spec,
            verdict,
            measurement_window,
            summary,
            worked,
            issues,
            follow_ups,
            evidence,
            telemetry_refs,
            heuristics,
            structured_issues,
        } => run_reflect(
            &spec,
            verdict.as_deref(),
            measurement_window.as_deref(),
            &summary,
            &worked,
            &issues,
            &follow_ups,
            &evidence,
            &telemetry_refs,
            &heuristics,
            &structured_issues,
        ),
        S5dCommand::Mcp => s5d::mcp::run_mcp_server(),
        S5dCommand::Install {
            s5d_path,
            dry_run,
            uninstall,
            claude,
            cursor,
            codex,
            gemini,
            all,
            global,
        } => run_install(
            s5d_path.as_deref(),
            dry_run,
            uninstall,
            claude,
            cursor,
            codex,
            gemini,
            all,
            global,
        ),
    }
}

// ── Install ───────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn run_install(
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

// ── Git Hooks ────────────────────────────────────────────────────────────────

enum HookInstallResult {
    Installed(std::path::PathBuf),
    AlreadyInstalled(std::path::PathBuf),
    ExistingHook(std::path::PathBuf),
    NoGit,
}

fn invoked_as_git_hook(name: &str) -> bool {
    std::env::args_os()
        .next()
        .and_then(|arg| {
            std::path::PathBuf::from(arg)
                .file_name()
                .map(|name| name.to_owned())
        })
        .and_then(|file_name| file_name.to_str().map(|name| name.to_string()))
        .is_some_and(|file_name| file_name == name)
}

fn install_pre_commit_hook(
    project_root: &std::path::Path,
    binary: &str,
) -> anyhow::Result<HookInstallResult> {
    let Some(git_dir) = resolve_git_dir(project_root)? else {
        return Ok(HookInstallResult::NoGit);
    };
    let hooks_dir = git_dir.join("hooks");
    std::fs::create_dir_all(&hooks_dir)?;
    let hook_path = hooks_dir.join("pre-commit");
    let binary_path = std::path::PathBuf::from(binary);

    if hook_path.exists() || hook_path.symlink_metadata().is_ok() {
        if points_to_binary(&hook_path, &binary_path) {
            return Ok(HookInstallResult::AlreadyInstalled(hook_path));
        }
        if is_replaceable_s5d_hook(&hook_path) {
            std::fs::remove_file(&hook_path)?;
            install_binary_hook(&binary_path, &hook_path)?;
            return Ok(HookInstallResult::Installed(hook_path));
        }
        return Ok(HookInstallResult::ExistingHook(hook_path));
    }

    install_binary_hook(&binary_path, &hook_path)?;
    Ok(HookInstallResult::Installed(hook_path))
}

fn is_replaceable_s5d_hook(hook_path: &std::path::Path) -> bool {
    if let Ok(target) = std::fs::read_link(hook_path) {
        if target.to_string_lossy().contains("s5d-validate.sh") {
            return true;
        }
    }

    std::fs::read_to_string(hook_path)
        .map(|content| {
            content.contains("S5D pre-commit validation hook")
                || content.contains("s5d hook pre-commit")
        })
        .unwrap_or(false)
}

fn resolve_git_dir(project_root: &std::path::Path) -> anyhow::Result<Option<std::path::PathBuf>> {
    let dot_git = project_root.join(".git");
    if dot_git.is_dir() {
        return Ok(Some(dot_git));
    }
    if dot_git.is_file() {
        let raw = std::fs::read_to_string(&dot_git)?;
        if let Some(path) = raw.trim().strip_prefix("gitdir:") {
            let git_dir = std::path::PathBuf::from(path.trim());
            return Ok(Some(if git_dir.is_absolute() {
                git_dir
            } else {
                project_root.join(git_dir)
            }));
        }
    }
    Ok(None)
}

fn points_to_binary(hook_path: &std::path::Path, binary_path: &std::path::Path) -> bool {
    let Ok(target) = std::fs::read_link(hook_path) else {
        return false;
    };
    let target = if target.is_absolute() {
        target
    } else {
        hook_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(target)
    };
    match (target.canonicalize(), binary_path.canonicalize()) {
        (Ok(target), Ok(binary)) => target == binary,
        _ => false,
    }
}

#[cfg(unix)]
fn install_binary_hook(
    binary_path: &std::path::Path,
    hook_path: &std::path::Path,
) -> anyhow::Result<()> {
    std::os::unix::fs::symlink(binary_path, hook_path)?;
    Ok(())
}

#[cfg(not(unix))]
fn install_binary_hook(
    binary_path: &std::path::Path,
    hook_path: &std::path::Path,
) -> anyhow::Result<()> {
    std::fs::copy(binary_path, hook_path)?;
    Ok(())
}

fn run_hook_pre_commit() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let Some(project) = s5d::S5dProject::find(&cwd) else {
        return Ok(());
    };

    let staged = git_staged_files(&project.root)?;
    if staged.is_empty() {
        return Ok(());
    }

    let staged_specs: std::collections::HashSet<String> = staged
        .iter()
        .filter(|path| path.ends_with(".s5d.yaml"))
        .cloned()
        .collect();
    let has_source_changes = staged.iter().any(|path| is_source_path(path));
    let has_s5d_changes = staged.iter().any(|path| path.starts_with(".s5d/"));

    let specs = project.discover_specs()?;
    let mut failures = Vec::new();
    let mut checked_specs = 0usize;
    let mut checked_architecture = std::collections::HashSet::new();
    let mut checked_codebase = false;

    for (path, spec) in &specs {
        let rel = display_project_path(&project.root, path);
        let is_staged_spec = staged_specs.contains(&rel);
        if is_staged_spec {
            checked_specs += 1;
            for error in s5d::validate_spec(spec) {
                failures.push(format!("{}: {}", rel, error));
            }
            for error in s5d::graph_check(spec) {
                failures.push(format!("{}: {}", rel, error));
            }
        }

        if (is_staged_spec || has_source_changes)
            && spec.gates.iter().any(|gate| gate.kind == "architecture")
            && checked_architecture.insert(rel.clone())
        {
            let report = s5d::architecture_check(spec, &project.root)?;
            for error in report.errors {
                failures.push(format!("{}: {}", rel, error));
            }
        }
    }

    if (has_source_changes || has_s5d_changes) && codebase_snapshot_exists(&project) {
        checked_codebase = true;
        let expected = s5d::build_codebase_snapshot(&project)?;
        match s5d::load_codebase_snapshot(&project)? {
            Some(actual) if actual == expected => {}
            Some(_) => failures.push(
                ".s5d/codebase snapshot is stale. Fix: run `s5d codebase sync` and stage the result."
                    .to_string(),
            ),
            None => failures.push(
                ".s5d/codebase snapshot is missing. Fix: run `s5d codebase sync` and stage the result."
                    .to_string(),
            ),
        }
    }

    if checked_specs > 0 || !checked_architecture.is_empty() || checked_codebase {
        println!(
            "s5d pre-commit: checked {} staged spec(s), {} architecture spec(s), codebase={}",
            checked_specs,
            checked_architecture.len(),
            if checked_codebase { "yes" } else { "no" }
        );
    }

    if failures.is_empty() {
        return Ok(());
    }

    eprintln!("s5d pre-commit blocked commit:");
    for failure in failures {
        eprintln!("  - {}", failure);
    }
    std::process::exit(1);
}

fn codebase_snapshot_exists(project: &s5d::S5dProject) -> bool {
    let codebase = project.s5d_dir().join("codebase");
    codebase.join("modules.yaml").exists() || codebase.join("coverage.yaml").exists()
}

fn git_staged_files(project_root: &std::path::Path) -> anyhow::Result<Vec<String>> {
    let output = std::process::Command::new("git")
        .args(["diff", "--cached", "--name-only", "--diff-filter=ACMR"])
        .current_dir(project_root)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "git diff --cached failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn is_source_path(path: &str) -> bool {
    let Some(extension) = std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
    else {
        return false;
    };
    matches!(
        extension,
        "rs" | "py"
            | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "go"
            | "java"
            | "kt"
            | "swift"
            | "c"
            | "cc"
            | "cpp"
            | "h"
            | "hpp"
    )
}

fn display_project_path(project_root: &std::path::Path, path: &std::path::Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
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

fn run_update_check(hook: bool, json: bool) -> anyhow::Result<()> {
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
                "S5D update available: {} -> {}. Run: s5d update apply",
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
        println!("  run: s5d update apply");
    } else {
        println!("  update: current");
    }
    Ok(())
}

fn run_update_apply(dry_run: bool) -> anyhow::Result<()> {
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

    run_git_command(&repo_root, &["fetch", "--tags", "--prune"])?;
    run_git_command(&repo_root, &["pull", "--ff-only"])?;
    install_skills_from_repo(&repo_root)?;
    install_binary_from_repo(&repo_root, &destination)?;
    println!("{} S5D updated", "ok".green());
    Ok(())
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
    if home.join(".codex").is_dir() {
        targets.push(home.join(".codex/skills"));
    }
    if home.join(".gemini").is_dir() {
        targets.push(home.join(".gemini/skills"));
    }
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
    let source = if let Some(prebuilt) = prebuilt_binary(repo_root) {
        prebuilt
    } else {
        let status = std::process::Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(repo_root.join("rust"))
            .status()?;
        if !status.success() {
            anyhow::bail!("cargo build --release failed");
        }
        repo_root.join("rust/target/release/s5d")
    };

    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = destination.with_extension("s5d-update-tmp");
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

fn run_init(
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
    match ensure_agents_md(&cwd.join("AGENTS.md")) {
        Ok(AgentsUpdate::Created) => println!("    {} AGENTS.md — created", "✓".green()),
        Ok(AgentsUpdate::Inserted) => {
            println!("    {} AGENTS.md — s5d block appended", "✓".green())
        }
        Ok(AgentsUpdate::Replaced) => println!("    {} AGENTS.md — s5d block updated", "✓".green()),
        Ok(AgentsUpdate::Unchanged) => {
            println!("    {} AGENTS.md — already up to date", "✓".green())
        }
        Err(e) => println!("    {} AGENTS.md: {}", "⚠".yellow(), e),
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

    println!(
        "\n  {} Restart your agent session to activate s5d MCP tools.",
        "⚠".yellow()
    );

    println!("\n  {} Next steps:", "→".blue());
    println!("    1. s5d new <feature-id> --product <name>   Create your first spec");
    println!("    2. s5d validate <spec>                     Validate the spec");
    println!("    3. s5d preview <spec>                      Preview the import");
    println!();

    Ok(())
}

// ── AGENTS.md block ──────────────────────────────────────────────────────────

const AGENTS_BEGIN: &str = "<!-- s5d:begin -->";
const AGENTS_END: &str = "<!-- s5d:end -->";

fn agents_block() -> String {
    format!(
        "{}\n## S5D — Decision & Validation Layer\n\n\
This repo uses **S5D** (https://github.com/system5-dev/s5d) — a thin layer over git \
for recording architectural decisions and verifying that code still matches them.\n\n\
**Use S5D for non-trivial changes.** Skip for: bug fixes <30 LOC, config-only, docs-only.\n\n\
**Flow:** `s5d_new` → edit spec → `s5d_validate` → `s5d_preview` → `s5d_approve` \
→ implement → `s5d_run_gates` → `s5d_import` → `s5d_drift_check`.\n\n\
**MCP tools** (prefer over shell CLI when available):\n\
- `s5d_route` — classify a request into tier/mode/entry\n\
- `s5d_new` / `s5d_note` — create spec / quick note\n\
- `s5d_validate` / `s5d_preview` — dry-run checks before approval\n\
- `s5d_approve` / `s5d_import` — commit decision, bind SHA256 chain\n\
- `s5d_drift_check` / `s5d_reconcile` / `s5d_rollback` — verify & recover\n\
- `s5d_show` / `s5d_status` — inspect specs and project state\n\n\
Specs live in `.s5d/packages/`. Run `s5d --help` or read `skills/s5d/SKILL.md` for full reference.\n{}",
        AGENTS_BEGIN, AGENTS_END
    )
}

#[derive(Debug, PartialEq)]
enum AgentsUpdate {
    Created,
    Inserted,
    Replaced,
    Unchanged,
}

fn ensure_agents_md(path: &std::path::Path) -> anyhow::Result<AgentsUpdate> {
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

// ── New ───────────────────────────────────────────────────────────────────────

fn run_new(
    id: &str,
    tier_str: &str,
    product: Option<&str>,
    question: Option<&str>,
    rationale: Option<&str>,
    hypothesis_id: Option<&str>,
) -> anyhow::Result<()> {
    s5d::sanitize_id(id)?;
    let cwd = std::env::current_dir()?;
    let project = s5d::S5dProject::find(&cwd)
        .ok_or_else(|| anyhow::anyhow!("no .s5d/ found (run `s5d init` first)"))?;

    let product_name = product.unwrap_or("MyProduct");
    let today = chrono::Utc::now().format("%Y%m%d");
    let spec_filename = format!("{}__{}.s5d.yaml", id, today);

    let spec_path = project.s5d_dir().join("packages").join(&spec_filename);
    if spec_path.exists() {
        anyhow::bail!("spec already exists: {}", spec_path.display());
    }

    let spec = if tier_str == "note" {
        let title = question.unwrap_or(id);
        let r = rationale.unwrap_or("No rationale provided");
        s5d::generate_note_spec(id, product_name, title, r)
    } else if tier_str == "decision" {
        let q = question.unwrap_or(id);
        s5d::generate_decision_spec(id, product_name, q)
    } else {
        let tier = match tier_str {
            "lightweight" => s5d::Tier::Lightweight,
            "standard" => s5d::Tier::Standard,
            "high" => s5d::Tier::High,
            _ => anyhow::bail!(
                "invalid tier: {} (use lightweight|standard|high|decision|note)",
                tier_str
            ),
        };
        s5d::generate_spec(id, tier, product_name)
    };

    let yaml = serde_yaml::to_string(&spec)?;
    std::fs::write(&spec_path, &yaml)?;

    let sha = s5d::S5dProject::file_sha256(&spec_path)?;
    let record = s5d::generate_record(&spec_filename, &sha);
    project.save_record(&spec_filename, &record)?;

    println!("{} Created spec: {}", "ok".green(), spec_path.display());
    println!(
        "  Record: {}",
        project
            .s5d_dir()
            .join("records")
            .join(spec_filename.replace(".s5d.yaml", ".record.yaml"))
            .display()
    );

    // Auto-link spec_ref on the specified hypothesis in a parent decision spec
    if let Some(hyp_id) = hypothesis_id {
        let decision_specs = project.discover_specs()?;
        let mut linked = false;

        for (dec_path, mut dec_spec) in decision_specs {
            if !matches!(dec_spec.tier, s5d::Tier::Decision) {
                continue;
            }
            if let Some(hyp) = dec_spec.hypotheses.iter_mut().find(|h| h.id == hyp_id) {
                hyp.spec_ref = Some(spec_filename.clone());
                save_spec_yaml(&dec_path, &dec_spec)?;
                println!(
                    "  {} Set spec_ref on hypothesis '{}' in {}",
                    "ok".green(),
                    hyp_id,
                    dec_path.display()
                );
                linked = true;
                break;
            }
        }

        if !linked {
            eprintln!(
                "  {} hypothesis '{}' not found in any decision spec — spec_ref not set",
                "warn:".yellow(),
                hyp_id
            );
        }
    }

    Ok(())
}

// ── Note (shorthand) ─────────────────────────────────────────────────────────

fn run_note(text: &str, product: Option<&str>) -> anyhow::Result<()> {
    if text.is_empty() {
        anyhow::bail!("note text cannot be empty");
    }

    // Generate slug from text: lowercase, alphanumeric + dashes, max 40 chars
    let slug: String = text
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let slug = if slug.len() > 40 { &slug[..40] } else { &slug };
    let slug = slug.trim_end_matches('-');

    let id = format!("note.{}", slug);

    run_new(&id, "note", product, Some(text), Some(text), None)
}

// ── Validate ──────────────────────────────────────────────────────────────────

fn run_validate(spec_path: &str) -> anyhow::Result<()> {
    let path = std::path::Path::new(spec_path);
    if !path.exists() {
        anyhow::bail!("file not found: {}", spec_path);
    }
    let content = std::fs::read_to_string(path)?;
    let spec: s5d::Spec =
        serde_yaml::from_str(&content).map_err(|e| anyhow::anyhow!("YAML parse error: {}", e))?;

    let errors = s5d::validate_spec(&spec);
    if errors.is_empty() {
        println!("{} {} is valid", "ok".green(), spec_path);
        Ok(())
    } else {
        for e in &errors {
            eprintln!("  {} {}", "error:".red(), e);
        }
        std::process::exit(1);
    }
}

// ── Status ────────────────────────────────────────────────────────────────────

fn run_status() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let project = s5d::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;

    let specs = project.discover_specs()?;
    if specs.is_empty() {
        println!("  No specs. Run `s5d new <id>` to create one.");
        return Ok(());
    }

    let total = specs.len();
    let mut in_progress = 0usize;
    let mut decisions_pending = 0usize;

    for (path, spec) in &specs {
        let filename = path.file_name().unwrap().to_string_lossy();
        let record = project.load_record(&filename)?;
        let is_terminal = record.as_ref().is_some_and(|r| {
            matches!(
                r.status,
                s5d::SpecStatus::Applied
                    | s5d::SpecStatus::Operated
                    | s5d::SpecStatus::Deprecated
                    | s5d::SpecStatus::Removed
            )
        });
        if !is_terminal {
            in_progress += 1;
        }
        if matches!(spec.tier, s5d::Tier::Decision)
            && !spec.hypotheses.is_empty()
            && record.as_ref().is_none_or(|r| r.decision.is_none())
        {
            decisions_pending += 1;
        }
    }

    println!();
    println!("  {} ({})", "S5D Specs".cyan(), project.root.display());
    println!(
        "  Total: {}  In-progress: {}  Decisions pending: {}",
        total, in_progress, decisions_pending
    );
    println!("  {}", "─".repeat(80));
    println!(
        "  {:<30} {:<12} {:<10} {:<10} Sync",
        "ID", "Version", "Tier", "Status"
    );
    println!("  {}", "─".repeat(80));

    for (path, spec) in &specs {
        let filename = path.file_name().unwrap().to_string_lossy();
        let record = project.load_record(&filename)?;
        let (status, sync) = match &record {
            Some(r) => (format!("{}", r.status), format!("{}", r.sync_status)),
            None => ("?".into(), "?".into()),
        };
        let status_colored = match status.as_str() {
            "proposed" => status.white(),
            "approved" => status.green(),
            "applied" => status.cyan(),
            "deprecated" | "removed" => status.dimmed(),
            _ => status.yellow(),
        };
        let sync_colored = match sync.as_str() {
            "synced" => sync.green(),
            "drifted" => sync.red(),
            "degraded" => sync.red().bold(),
            _ => sync.dimmed(),
        };
        println!(
            "  {:<30} {:<12} {:<10} {:<10} {}",
            spec.id,
            spec.version,
            format!("{}", spec.tier),
            status_colored,
            sync_colored
        );

        if let Some(ref rec) = record {
            if let Some(ref active_phase) = rec.active_phase {
                println!("  {} {}", "Active phase:".dimmed(), active_phase);
            }
        }

        // Determine current phase and print next-action hint
        let phase: Option<s5d::Phase> = record.as_ref().and_then(|r| match &r.status {
            s5d::SpecStatus::Proposed => {
                let has_hypotheses = !spec.hypotheses.is_empty();
                if has_hypotheses {
                    Some(s5d::Phase::Preview)
                } else {
                    Some(s5d::Phase::Synthesize)
                }
            }
            s5d::SpecStatus::InReview => Some(s5d::Phase::Preview),
            s5d::SpecStatus::Previewed => Some(s5d::Phase::Approve),
            s5d::SpecStatus::Approved => Some(s5d::Phase::Execute),
            s5d::SpecStatus::Applied => {
                let gates_passed = !r.gate_results.is_empty()
                    && r.gate_results.iter().all(|g| g.status == "passed");
                if gates_passed {
                    Some(s5d::Phase::Learn)
                } else {
                    Some(s5d::Phase::Verify)
                }
            }
            s5d::SpecStatus::Operated | s5d::SpecStatus::Deprecated | s5d::SpecStatus::Removed => {
                None
            }
        });

        if let Some(p) = phase {
            println!("  {} {}", "Next:".dimmed(), p.cli_hint());
        }

        // Check decision staleness
        if let Some(ref rec) = record {
            if let Some(ref dec) = rec.decision {
                if let Some(ref exp) = dec.expires_at {
                    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                    if today.as_str() > exp.as_str() {
                        println!(
                            "  {} Decision expired {} — re-evaluate or extend",
                            "⚠ STALE".red().bold(),
                            exp
                        );
                    } else {
                        // Warn 14 days before expiry
                        let warn_date = chrono::Utc::now() + chrono::Duration::days(14);
                        let warn_str = warn_date.format("%Y-%m-%d").to_string();
                        if warn_str.as_str() > exp.as_str() {
                            println!(
                                "  {} Decision expires {} — consider refreshing",
                                "⚠ EXPIRING".yellow(),
                                exp
                            );
                        }
                    }
                }
            }
        }
    }
    println!();
    Ok(())
}

fn workflow_required(spec: &s5d::Spec) -> anyhow::Result<&s5d::Workflow> {
    spec.workflow.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "spec has no workflow block — add workflow before using phase or execute commands"
        )
    })
}

fn workflow_phase_by_id<'a>(
    workflow: &'a s5d::Workflow,
    phase_id: &str,
) -> anyhow::Result<&'a s5d::WorkflowPhase> {
    workflow
        .phases
        .iter()
        .find(|p| p.id == phase_id)
        .ok_or_else(|| anyhow::anyhow!("workflow phase not found: {}", phase_id))
}

fn latest_phase_status(record: &s5d::Record, phase_id: &str) -> s5d::WorkflowPhaseStatus {
    record
        .phase_history
        .iter()
        .rev()
        .find(|entry| entry.phase_id == phase_id)
        .map(|entry| entry.status.clone())
        .unwrap_or(s5d::WorkflowPhaseStatus::Planned)
}

fn append_phase_history(
    record: &mut s5d::Record,
    phase_id: &str,
    status: s5d::WorkflowPhaseStatus,
    reviewer: Option<String>,
    engine: Option<String>,
    notes: Option<String>,
) {
    record.phase_history.push(s5d::WorkflowPhaseRecord {
        phase_id: phase_id.to_string(),
        status,
        timestamp: chrono::Utc::now().to_rfc3339(),
        reviewer,
        engine,
        notes,
    });
}

fn ensure_phase_execution_ready(
    spec: &s5d::Spec,
    record: &s5d::Record,
    phase_id: &str,
) -> anyhow::Result<()> {
    if !matches!(
        record.status,
        s5d::SpecStatus::Approved | s5d::SpecStatus::Applied | s5d::SpecStatus::Operated
    ) {
        anyhow::bail!(
            "phase execution requires approved or later spec state (current: {})",
            record.status
        );
    }
    let workflow = workflow_required(spec)?;
    workflow_phase_by_id(workflow, phase_id)?;
    Ok(())
}

fn run_phase_list(spec_path: &str) -> anyhow::Result<()> {
    let (_path, spec) = load_spec_yaml(spec_path)?;
    let cwd = std::env::current_dir()?;
    let project = s5d::S5dProject::find(&cwd)
        .or_else(|| s5d::S5dProject::find(std::path::Path::new(spec_path)))
        .ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;
    let spec_filename = std::path::Path::new(spec_path)
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("cannot determine filename"))?
        .to_string_lossy()
        .into_owned();
    let record = project.load_record(&spec_filename)?.ok_or_else(|| {
        anyhow::anyhow!("no record found for {} — run preview first", spec_filename)
    })?;
    let workflow = workflow_required(&spec)?;

    println!("{}: {}", "PHASES".cyan().bold(), spec.id.bold());
    println!("{}", "━".repeat(60));
    for phase in &workflow.phases {
        let status = latest_phase_status(&record, &phase.id);
        let marker = if record.active_phase.as_deref() == Some(phase.id.as_str()) {
            "▶"
        } else {
            "•"
        };
        println!(
            "  {} {} [{}]",
            marker,
            phase.title.bold(),
            format!("{}", status).dimmed()
        );
        println!("    {} {}", "id:".dimmed(), phase.id);
        println!("    {} {}", "scope:".dimmed(), phase.scope);
    }
    Ok(())
}

fn run_phase_start(spec_path: &str, phase_id: &str) -> anyhow::Result<()> {
    let (project, _spec_path, spec, spec_filename) = load_spec_context(spec_path)?;
    let workflow = workflow_required(&spec)?;
    let phase = workflow_phase_by_id(workflow, phase_id)?;
    let mut record = project.load_record(&spec_filename)?.ok_or_else(|| {
        anyhow::anyhow!("no record found for {} — run preview first", spec_filename)
    })?;
    ensure_phase_execution_ready(&spec, &record, phase_id)?;

    if let Some(ref active_phase) = record.active_phase {
        if active_phase != phase_id {
            anyhow::bail!(
                "phase '{}' is already active — accept or clear it before starting '{}'",
                active_phase,
                phase_id
            );
        }
        anyhow::bail!("phase '{}' is already active", phase_id);
    }

    record.active_phase = Some(phase_id.to_string());
    append_phase_history(
        &mut record,
        phase_id,
        s5d::WorkflowPhaseStatus::Active,
        None,
        workflow
            .execution_mode
            .as_ref()
            .map(|mode| mode.engine.clone()),
        Some(format!("Started phase '{}'", phase.title)),
    );
    project.save_record(&spec_filename, &record)?;

    println!("{} Active phase → {}", "ok".green(), phase_id);
    println!("  {} {}", "Scope:".dimmed(), phase.scope);
    Ok(())
}

fn run_phase_accept(spec_path: &str, phase_id: &str, reviewer: &str) -> anyhow::Result<()> {
    let (project, _spec_path, spec, spec_filename) = load_spec_context(spec_path)?;
    let workflow = workflow_required(&spec)?;
    workflow_phase_by_id(workflow, phase_id)?;
    let mut record = project.load_record(&spec_filename)?.ok_or_else(|| {
        anyhow::anyhow!("no record found for {} — run preview first", spec_filename)
    })?;
    ensure_phase_execution_ready(&spec, &record, phase_id)?;

    if record.active_phase.as_deref() != Some(phase_id) {
        anyhow::bail!(
            "phase '{}' is not active — start it before acceptance",
            phase_id
        );
    }

    record.active_phase = None;
    append_phase_history(
        &mut record,
        phase_id,
        s5d::WorkflowPhaseStatus::Accepted,
        Some(reviewer.to_string()),
        None,
        Some("Human phase acceptance".into()),
    );
    project.save_record(&spec_filename, &record)?;

    println!(
        "{} Phase '{}' accepted by {}",
        "ok".green(),
        phase_id,
        reviewer
    );
    Ok(())
}

fn run_execute_loop(
    spec_path: &str,
    phase_id: &str,
    engine: &str,
    mode: Option<&str>,
) -> anyhow::Result<()> {
    let (_project, _spec_path, spec, spec_filename) = load_spec_context(spec_path)?;
    let workflow = workflow_required(&spec)?;
    let phase = workflow_phase_by_id(workflow, phase_id)?;
    let cwd = std::env::current_dir()?;
    let project = s5d::S5dProject::find(&cwd)
        .or_else(|| s5d::S5dProject::find(std::path::Path::new(spec_path)))
        .ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;
    let record = project.load_record(&spec_filename)?.ok_or_else(|| {
        anyhow::anyhow!("no record found for {} — run preview first", spec_filename)
    })?;
    ensure_phase_execution_ready(&spec, &record, phase_id)?;

    if record.active_phase.as_deref() != Some(phase_id) {
        anyhow::bail!("phase '{}' must be active before execute loop", phase_id);
    }

    let declared_engine = workflow
        .execution_mode
        .as_ref()
        .map(|mode| mode.engine.as_str())
        .unwrap_or("manual");
    if engine != declared_engine {
        anyhow::bail!(
            "requested engine '{}' does not match workflow execution mode '{}'",
            engine,
            declared_engine
        );
    }
    if engine != "ralph" {
        anyhow::bail!(
            "execute loop currently supports only engine=ralph (got '{}')",
            engine
        );
    }

    let preset = s5d::RalphPreset::resolve(mode, &phase.id)?;
    let package = s5d::build_ralph_task_package(&spec, workflow, phase, preset)?;
    let task_path = project.save_task_artifact(&spec_filename, &phase.id, preset.id(), &package)?;
    let display_path = task_path
        .strip_prefix(project.root.as_path())
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| task_path.display().to_string());
    println!("{} {}", "Task artifact:".dimmed(), display_path);
    println!();
    println!("{}", package);
    Ok(())
}

// ── Cg ────────────────────────────────────────────────────────────────────────

fn run_cg() -> anyhow::Result<()> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();

    let tools = ["ruff", "mypy", "pytest", "python3", "rustc", "cargo"];
    for tool in &tools {
        if let Ok(output) = std::process::Command::new(tool).arg("--version").output() {
            if output.status.success() {
                hasher.update(&output.stdout);
            }
        }
    }

    let hash = format!("{:x}", hasher.finalize());
    println!("cg:{}", &hash[..16]);
    Ok(())
}

// ── Preview ───────────────────────────────────────────────────────────────────

fn run_preview(spec_arg: &str) -> anyhow::Result<()> {
    let (project, spec_path, spec, spec_filename) = load_spec_context(spec_arg)?;

    let s5d_dir = project.s5d_dir();
    let spec = s5d::infer::materialize_spec(&spec);
    // Clone alias table — do not persist changes
    let mut aliases = s5d::AliasTable::load(&s5d_dir)?;
    if let Some(ref meta) = spec.meta {
        aliases.apply_renames(&spec.id, &meta.renames);
    }
    let actions = s5d::compute_diff(&spec, &mut aliases);
    let diff_sha = actions.sha256();
    let counts = actions.counts();

    println!("{} Preview: {}", "ok".green(), spec.id);
    println!(
        "  create: {}  link: {}  update: {}  delete: {}",
        counts.create, counts.link, counts.update, counts.delete
    );
    println!("  diff_sha256: {}", diff_sha);

    if !actions.create.is_empty() {
        println!("\n  {} New artifacts:", "+".green());
        for item in &actions.create {
            println!("    + {}", item);
        }
    }
    if !actions.update.is_empty() {
        println!("\n  {} Existing artifacts:", "~".yellow());
        for item in &actions.update {
            println!("    ~ {}", item);
        }
    }
    if !actions.link.is_empty() {
        println!("\n  {} Links:", "→".cyan());
        for item in &actions.link {
            println!("    → {}", item);
        }
    }

    // Store preview result in record
    let spec_sha = s5d::S5dProject::file_sha256(&spec_path)?;
    let mut record = project
        .load_record(&spec_filename)?
        .unwrap_or_else(|| s5d::generate_record(&spec_filename, &spec_sha));

    record.preview = Some(s5d::PreviewResult {
        diff_sha256: diff_sha,
        previewed_spec_sha256: spec_sha.clone(),
        actions: counts,
        log: None,
    });
    record.status = s5d::SpecStatus::Previewed;
    record.status_history.push(s5d::StatusEntry {
        status: s5d::SpecStatus::Previewed,
        timestamp: chrono::Utc::now().to_rfc3339(),
    });
    project.save_record(&spec_filename, &record)?;

    Ok(())
}

// ── Approve ───────────────────────────────────────────────────────────────────

fn run_approve(spec_arg: &str, reviewer: &str, require_owner: bool) -> anyhow::Result<()> {
    let (project, spec_path, spec, spec_filename) = load_spec_context(spec_arg)?;

    // Check reviewer against domain owners
    let domain_owners: Vec<&str> = spec
        .artifacts
        .as_ref()
        .map(|a| {
            a.domains
                .iter()
                .filter_map(|d| d.team.as_deref())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !domain_owners.is_empty()
        && !domain_owners
            .iter()
            .any(|o| o.eq_ignore_ascii_case(reviewer))
    {
        if require_owner {
            eprintln!(
                "  {} reviewer '{}' is not a domain owner ({})",
                "error:".red(),
                reviewer,
                domain_owners.join(", ")
            );
            eprintln!(
                "  Domain owners must approve their domains. Remove --require-owner to allow."
            );
            std::process::exit(1);
        } else {
            eprintln!(
                "  {} reviewer '{}' is not a domain owner ({})",
                "warning:".yellow(),
                reviewer,
                domain_owners.join(", ")
            );
        }
    }

    let spec_sha = s5d::S5dProject::file_sha256(&spec_path)?;

    let mut record = project
        .load_record(&spec_filename)?
        .ok_or_else(|| anyhow::anyhow!("no record found for {}", spec_filename))?;

    if record.status != s5d::SpecStatus::Previewed {
        eprintln!(
            "  {} spec must be in 'previewed' state before approval (current: {})",
            "error:".red(),
            record.status
        );
        std::process::exit(1);
    }

    // Check that spec hasn't changed since preview
    if let Some(ref preview) = record.preview {
        if !preview.previewed_spec_sha256.is_empty() && preview.previewed_spec_sha256 != spec_sha {
            eprintln!(
                "  {} spec modified since preview — re-run `s5d preview` before approving",
                "error:".red()
            );
            std::process::exit(1);
        }
    }

    let diff_sha = record
        .preview
        .as_ref()
        .map(|p| p.diff_sha256.clone())
        .ok_or_else(|| anyhow::anyhow!("no preview result on record — run `s5d preview` first"))?;

    record.approvals.push(s5d::Approval {
        reviewer: reviewer.into(),
        date: chrono::Utc::now().to_rfc3339(),
        spec_sha256: spec_sha,
        diff_sha256: diff_sha.clone(),
    });
    record.status = s5d::SpecStatus::Approved;
    record.status_history.push(s5d::StatusEntry {
        status: s5d::SpecStatus::Approved,
        timestamp: chrono::Utc::now().to_rfc3339(),
    });
    project.save_record(&spec_filename, &record)?;

    println!(
        "{} Approved: {} (reviewer: {})",
        "ok".green(),
        spec_filename,
        reviewer
    );
    println!("  diff_sha256: {}", diff_sha);
    Ok(())
}

// ── RunGates ──────────────────────────────────────────────────────────────────

fn run_gates(spec_arg: &str) -> anyhow::Result<()> {
    let (project, _spec_path, spec, spec_filename) = load_spec_context(spec_arg)?;

    let config = project.load_config()?;
    let results = s5d::run_gates(&spec, &config, spec_arg, &project.root, &project.s5d_dir())?;

    let passed = results.iter().filter(|r| r.status == "passed").count();
    let failed = results
        .iter()
        .filter(|r| r.status != "passed" && r.status != "skipped")
        .count();
    let skipped = results.iter().filter(|r| r.status == "skipped").count();

    for r in &results {
        let marker = match r.status.as_str() {
            "passed" => "pass".green(),
            "skipped" => "skip".dimmed(),
            _ => "fail".red(),
        };
        println!("  [{}] gate:{} (attempt {})", marker, r.kind, r.attempt);
    }
    println!(
        "\n  passed: {}  failed: {}  skipped: {}",
        passed, failed, skipped
    );

    // Append gate results to record
    let mut record = project
        .load_record(&spec_filename)?
        .ok_or_else(|| anyhow::anyhow!("no record found for {}", spec_filename))?;
    record.gate_results.extend(results);
    project.save_record(&spec_filename, &record)?;

    if failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

// ── Import ────────────────────────────────────────────────────────────────────

fn run_import(spec_arg: &str, verified_by: &Option<String>, force: bool) -> anyhow::Result<()> {
    let (project, spec_path, spec, spec_filename) = load_spec_context(spec_arg)?;

    // Check record status == Approved
    let mut record = project.load_record(&spec_filename)?.ok_or_else(|| {
        anyhow::anyhow!(
            "no record found for {} — run preview and approve first",
            spec_filename
        )
    })?;

    if record.status != s5d::SpecStatus::Approved {
        eprintln!(
            "  {} spec must be approved before import (current: {})",
            "error:".red(),
            record.status
        );
        std::process::exit(4);
    }

    // Phase gate checks (including verifier ≠ approver)
    let import_checks = s5d::check_import(&Some(record.clone()), verified_by);
    s5d::enforce_checks(&import_checks, force)?;

    if !spec.gates.is_empty() {
        // Check that the LATEST result for each declared gate kind is "passed".
        let all_latest_passed = spec.gates.iter().all(|g| {
            record
                .gate_results
                .iter()
                .rev()
                .find(|r| r.kind == g.kind)
                .is_some_and(|r| r.status == "passed" || r.status == "waived")
        });
        if !all_latest_passed {
            eprintln!(
                "  {} all declared gates must pass before import — run `s5d run-gates` first",
                "error:".red()
            );
            std::process::exit(7);
        }
    }

    // Get last approval
    let approval = record
        .approvals
        .last()
        .ok_or_else(|| anyhow::anyhow!("no approval found on record"))?;

    // Verify spec_sha256 matches
    let current_sha = s5d::S5dProject::file_sha256(&spec_path)?;
    if current_sha != approval.spec_sha256 {
        eprintln!("  {} spec file has changed since approval", "error:".red());
        eprintln!("    approved: {}", approval.spec_sha256);
        eprintln!("    current:  {}", current_sha);
        std::process::exit(4);
    }

    // Recompute diff and verify diff_sha256 (materialize to match preview)
    let s5d_dir = project.s5d_dir();
    let materialized = s5d::infer::materialize_spec(&spec);
    let mut aliases_check = s5d::AliasTable::load(&s5d_dir)?;
    if let Some(ref meta) = materialized.meta {
        aliases_check.apply_renames(&materialized.id, &meta.renames);
    }
    let fresh_actions = s5d::compute_diff(&materialized, &mut aliases_check);
    let fresh_diff_sha = fresh_actions.sha256();

    if fresh_diff_sha != approval.diff_sha256 {
        eprintln!(
            "  {} diff has changed since approval — re-run preview and approve",
            "error:".red()
        );
        eprintln!("    approved: {}", approval.diff_sha256);
        eprintln!("    current:  {}", fresh_diff_sha);
        std::process::exit(5);
    }

    // Store verified_by before import execution
    if verified_by.is_some() {
        record.verified_by = verified_by.clone();
        project.save_record(&spec_filename, &record)?;
    }

    let (actions, fingerprint) = s5d::execute_import(&project, &spec_path, &spec, &spec_filename)?;
    let counts = actions.counts();

    println!("{} Imported: {}", "ok".green(), spec.id);
    println!(
        "  create: {}  link: {}  update: {}  delete: {}",
        counts.create, counts.link, counts.update, counts.delete
    );
    println!("  state_fingerprint: {}", fingerprint);

    Ok(())
}

// ── Rollback ──────────────────────────────────────────────────────────────────

fn run_rollback(spec_arg: &str) -> anyhow::Result<()> {
    let (project, _spec_path, spec, spec_filename) = load_spec_context(spec_arg)?;
    let _lock = project.acquire_lock(&format!("rollback.{}", spec.id))?;

    let s5d_dir = project.s5d_dir();

    // Load ledger and find last successful import for this package
    let mut ledger = project.load_ledger()?;
    let last_import_idx = ledger
        .entries
        .iter()
        .rposition(|e| e.package_id == spec.id && e.action == "import" && e.status == "success");

    if last_import_idx.is_none() {
        anyhow::bail!("no successful import found for {} to roll back", spec.id);
    }

    // Collect global artifact IDs still referenced by other specs
    let other_specs = project.discover_specs()?;
    let mut referenced_globals = std::collections::HashSet::new();
    for (_, other) in &other_specs {
        if other.id != spec.id {
            referenced_globals.extend(s5d::import::collect_global_artifact_ids(other));
        }
    }

    // Tombstone alias entries for this package
    let mut aliases = s5d::AliasTable::load(&s5d_dir)?;
    for entry in &mut aliases.packages {
        if entry.package_id.as_deref() == Some(&spec.id) && !entry.deprecated {
            entry.deprecated = true;
        }
    }
    for entry in &mut aliases.global {
        if entry.owning_package.as_deref() == Some(&spec.id) && !entry.deprecated {
            // Skip if another spec still references this global artifact
            let key = (entry.artifact_type.clone(), entry.artifact_id.clone());
            if !referenced_globals.contains(&key) {
                entry.deprecated = true;
            }
        }
    }
    aliases.save(&s5d_dir)?;

    // Append rollback ledger entry
    ledger.entries.push(s5d::LedgerEntry {
        spec_sha256: "rollback".into(),
        state_fingerprint: "rollback".into(),
        package_id: spec.id.clone(),
        action: "rollback".into(),
        status: "success".into(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        record_ref: Some(format!(
            "records/{}",
            spec_filename.replace(".s5d.yaml", ".record.yaml")
        )),
    });
    project.save_ledger(&ledger)?;

    // Update record status
    let mut record = project
        .load_record(&spec_filename)?
        .ok_or_else(|| anyhow::anyhow!("no record found for {}", spec_filename))?;
    record.status = s5d::SpecStatus::Deprecated;
    record.sync_status = s5d::SyncStatus::Unknown;
    record.status_history.push(s5d::StatusEntry {
        status: s5d::SpecStatus::Deprecated,
        timestamp: chrono::Utc::now().to_rfc3339(),
    });
    project.save_record(&spec_filename, &record)?;

    // Remove from index
    let mut index = project.load_index()?;
    index.features.retain(|e| e.id != spec.id);
    project.save_index(&index)?;

    println!("{} Rolled back: {}", "ok".green(), spec.id);
    Ok(())
}

// ── Index check ───────────────────────────────────────────────────────────────

fn run_index_check() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let project = s5d::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;

    let index = project.load_index()?;
    let specs = project.discover_specs()?;

    let mut mismatches = 0;

    // Check all index entries have a corresponding spec
    for entry in &index.features {
        let spec_path = project.s5d_dir().join(&entry.spec_path);
        if !spec_path.exists() {
            eprintln!(
                "  {} index entry {} points to missing spec: {}",
                "warn:".yellow(),
                entry.id,
                entry.spec_path
            );
            mismatches += 1;
        }
    }

    // Check all discovered specs are in the index
    for (path, spec) in &specs {
        let filename = path.file_name().unwrap().to_string_lossy();
        let spec_path_rel = format!("packages/{}", filename);
        if !index
            .features
            .iter()
            .any(|e| e.id == spec.id || e.spec_path == spec_path_rel)
        {
            eprintln!("  {} spec {} not in index", "warn:".yellow(), filename);
            mismatches += 1;
        }
    }

    if mismatches == 0 {
        println!(
            "{} index.yaml is consistent ({} entries)",
            "ok".green(),
            index.features.len()
        );
    } else {
        eprintln!(
            "  {} {} mismatch(es) found — run `s5d index sync` to fix",
            "error:".red(),
            mismatches
        );
        std::process::exit(1);
    }
    Ok(())
}

// ── Index sync ────────────────────────────────────────────────────────────────

fn run_index_sync() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let project = s5d::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;

    let specs = project.discover_specs()?;
    let mut features = Vec::new();

    for (path, spec) in &specs {
        let filename = path.file_name().unwrap().to_string_lossy();
        let record = project.load_record(&filename)?;
        let status = record
            .map(|r| r.status)
            .unwrap_or(s5d::SpecStatus::Proposed);
        features.push(s5d::IndexEntry {
            id: spec.id.clone(),
            spec_path: format!("packages/{}", filename),
            status,
            product: spec.product.clone(),
            version: spec.version.clone(),
        });
    }

    let count = features.len();
    let index = s5d::Index { features };
    project.save_index(&index)?;

    println!("{} index.yaml rebuilt ({} entries)", "ok".green(), count);
    Ok(())
}

// ── Codebase coverage ────────────────────────────────────────────────────────

fn run_codebase_sync() -> anyhow::Result<()> {
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

fn run_codebase_check() -> anyhow::Result<()> {
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

// ── Analyze ───────────────────────────────────────────────────────────────────

// ── Bootstrap ─────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct BootstrapManifest {
    #[serde(default)]
    entries: Vec<BootstrapEntry>,
}

#[derive(serde::Deserialize)]
struct BootstrapEntry {
    id: String,
    #[serde(rename = "type")]
    type_: String,
    uuid: String,
    #[serde(default)]
    package_id: Option<String>,
}

fn run_bootstrap(manifest_path: &str) -> anyhow::Result<()> {
    let path = std::path::Path::new(manifest_path);
    if !path.exists() {
        anyhow::bail!("manifest not found: {}", manifest_path);
    }
    let content = std::fs::read_to_string(path)?;
    let manifest: BootstrapManifest =
        serde_yaml::from_str(&content).map_err(|e| anyhow::anyhow!("YAML parse error: {}", e))?;

    let cwd = std::env::current_dir()?;
    let project = s5d::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;

    let s5d_dir = project.s5d_dir();
    let mut aliases = s5d::AliasTable::load(&s5d_dir)?;

    const GLOBAL_TYPES: &[&str] = &["Product", "SoftwareSystem", "Container", "Domain", "Role"];
    let mut added = 0usize;

    for entry in &manifest.entries {
        let is_global = GLOBAL_TYPES.contains(&entry.type_.as_str());
        if is_global {
            let already = aliases.global.iter().any(|e| {
                e.artifact_id == entry.id && e.artifact_type == entry.type_ && !e.deprecated
            });
            if !already {
                aliases.global.push(s5d::AliasEntry {
                    uuid: entry.uuid.clone(),
                    artifact_id: entry.id.clone(),
                    artifact_type: entry.type_.clone(),
                    package_id: None,
                    owning_package: entry.package_id.clone(),
                    deprecated: false,
                });
                added += 1;
            }
        } else {
            let pkg = entry.package_id.as_deref().unwrap_or("");
            let already = aliases.packages.iter().any(|e| {
                e.package_id.as_deref() == Some(pkg)
                    && e.artifact_id == entry.id
                    && e.artifact_type == entry.type_
                    && !e.deprecated
            });
            if !already {
                aliases.packages.push(s5d::AliasEntry {
                    uuid: entry.uuid.clone(),
                    artifact_id: entry.id.clone(),
                    artifact_type: entry.type_.clone(),
                    package_id: entry.package_id.clone(),
                    owning_package: None,
                    deprecated: false,
                });
                added += 1;
            }
        }
    }

    aliases.save(&s5d_dir)?;
    println!(
        "{} Bootstrap complete: {} entries added ({} total in manifest)",
        "ok".green(),
        added,
        manifest.entries.len()
    );
    Ok(())
}

// ── GraphCheck ────────────────────────────────────────────────────────────────

fn run_graph_check(spec_arg: &str) -> anyhow::Result<()> {
    let (_project, _spec_path, spec, _spec_filename) = load_spec_context(spec_arg)?;

    let errors = s5d::graph_check(&spec);
    if errors.is_empty() {
        println!("{} {} graph ok", "ok".green(), spec_arg);
        Ok(())
    } else {
        for e in &errors {
            eprintln!("  {} {}", "error:".red(), e);
        }
        std::process::exit(3);
    }
}

// ── Check ─────────────────────────────────────────────────────────────────────

fn run_check(spec_arg: &str, format: &str) -> anyhow::Result<()> {
    let (project, _spec_path, spec, _spec_filename) = load_spec_context(spec_arg)?;
    let report = s5d::architecture_check(&spec, &project.root)?;

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        "text" => {
            println!(
                "{} Architecture check: {}",
                "S5D".cyan().bold(),
                spec.id.bold()
            );
            println!(
                "  components: {}  dependencies: {}  warnings: {}  errors: {}",
                report.components.len(),
                report.dependencies.len(),
                report.warnings.len(),
                report.errors.len()
            );

            for component in &report.components {
                println!(
                    "  {} {} ({}) — {} file(s)",
                    "component".dimmed(),
                    component.component,
                    component.domain,
                    component.files.len()
                );
            }
            for dependency in &report.dependencies {
                println!(
                    "  {} {} -> {} via {}",
                    "edge".dimmed(),
                    dependency.from_component,
                    dependency.to_component,
                    dependency.reference
                );
            }
            for warning in &report.warnings {
                eprintln!("  {} {}", "warn:".yellow(), warning);
            }
            for error in &report.errors {
                eprintln!("  {} {}", "error:".red(), error);
            }
            if report.errors.is_empty() {
                println!("{} {} architecture ok", "ok".green(), spec_arg);
            }
        }
        other => anyhow::bail!("invalid --format '{}': expected text or json", other),
    }

    if !report.errors.is_empty() {
        std::process::exit(3);
    }
    Ok(())
}

// ── DriftCheck ────────────────────────────────────────────────────────────────

fn run_drift_check(spec_arg: Option<&str>) -> anyhow::Result<()> {
    if let Some(arg) = spec_arg {
        let (project, _spec_path, spec, spec_filename) = load_spec_context(arg)?;
        let result = s5d::check_drift(&project, &spec, &spec_filename)?;
        let drifted = print_drift_result(&spec.id, result, &project, &spec_filename)?;
        if drifted {
            std::process::exit(6);
        }
    } else {
        let cwd = std::env::current_dir()?;
        let project =
            s5d::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;

        let specs = project.discover_specs()?;
        let mut any_drifted = false;

        for (path, spec) in &specs {
            let filename = path.file_name().unwrap().to_string_lossy();
            let record = project.load_record(&filename)?;
            let is_applied = record
                .as_ref()
                .is_some_and(|r| r.status == s5d::SpecStatus::Applied);
            if !is_applied {
                continue;
            }
            let result = s5d::check_drift(&project, spec, &filename)?;
            let drifted = print_drift_result(&spec.id, result, &project, &filename)?;
            if drifted {
                any_drifted = true;
            }
        }

        if any_drifted {
            std::process::exit(6);
        }
    }
    Ok(())
}

fn print_drift_result(
    spec_id: &str,
    result: s5d::DriftResult,
    project: &s5d::S5dProject,
    spec_filename: &str,
) -> anyhow::Result<bool> {
    match result {
        s5d::DriftResult::Synced => {
            println!("  {} {} — synced", "ok".green(), spec_id);
            if let Ok(Some(mut record)) = project.load_record(spec_filename) {
                record.sync_status = s5d::SyncStatus::Synced;
                let _ = project.save_record(spec_filename, &record);
            }
            Ok(false)
        }
        s5d::DriftResult::Drifted { expected, actual } => {
            eprintln!("  {} {} — drifted", "warn:".yellow(), spec_id);
            eprintln!("    expected: {}", expected);
            eprintln!("    actual:   {}", actual);
            if let Ok(Some(mut record)) = project.load_record(spec_filename) {
                record.sync_status = s5d::SyncStatus::Drifted;
                let _ = project.save_record(spec_filename, &record);
            }
            Ok(true)
        }
        s5d::DriftResult::Degraded { reason } => {
            eprintln!("  {} {} — degraded: {}", "warn:".yellow(), spec_id, reason);
            if let Ok(Some(mut record)) = project.load_record(spec_filename) {
                record.sync_status = s5d::SyncStatus::Degraded;
                let _ = project.save_record(spec_filename, &record);
            }
            Ok(true)
        }
    }
}

// ── Reconcile ─────────────────────────────────────────────────────────────────

fn run_reconcile(spec_arg: Option<&str>) -> anyhow::Result<()> {
    if let Some(arg) = spec_arg {
        let (project, spec_path, spec, spec_filename) = load_spec_context(arg)?;
        let (actions, fingerprint) = s5d::reconcile(&project, &spec, &spec_path, &spec_filename)?;
        let counts = actions.counts();
        println!("{} Reconciled: {}", "ok".green(), spec.id);
        println!(
            "  create: {}  link: {}  update: {}  delete: {}",
            counts.create, counts.link, counts.update, counts.delete
        );
        println!("  state_fingerprint: {}", fingerprint);
    } else {
        let cwd = std::env::current_dir()?;
        let project =
            s5d::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;

        let specs = project.discover_specs()?;
        let mut reconciled = 0usize;

        for (path, spec) in &specs {
            let filename = path.file_name().unwrap().to_string_lossy();
            let record = project.load_record(&filename)?;
            let is_applied = record
                .as_ref()
                .is_some_and(|r| r.status == s5d::SpecStatus::Applied);
            if !is_applied {
                continue;
            }

            let drift = s5d::check_drift(&project, spec, &filename)?;
            let needs_reconcile = matches!(
                drift,
                s5d::DriftResult::Drifted { .. } | s5d::DriftResult::Degraded { .. }
            );
            if !needs_reconcile {
                continue;
            }

            match s5d::reconcile(&project, spec, path, &filename) {
                Ok((actions, fingerprint)) => {
                    let counts = actions.counts();
                    println!("{} Reconciled: {}", "ok".green(), spec.id);
                    println!(
                        "  create: {}  link: {}  update: {}  delete: {}",
                        counts.create, counts.link, counts.update, counts.delete
                    );
                    println!("  state_fingerprint: {}", fingerprint);
                    reconciled += 1;
                }
                Err(e) => {
                    eprintln!(
                        "  {} reconcile failed for {}: {}",
                        "error:".red(),
                        spec.id,
                        e
                    );
                }
            }
        }

        println!("\n  {} spec(s) reconciled", reconciled);
    }
    Ok(())
}

// ── ContractCheck ─────────────────────────────────────────────────────────────

// ── Show ──────────────────────────────────────────────────────────────────────

fn run_show(spec_path: &str) -> anyhow::Result<()> {
    let (path, spec) = load_spec_yaml(spec_path)?;

    // Load record for decision/approval/gate truth
    let record = if let Ok(abs) = path.canonicalize() {
        if let Some(project) = s5d::S5dProject::find(&abs) {
            let fname = abs.file_name().unwrap().to_string_lossy().into_owned();
            project.load_record(&fname)?
        } else {
            None
        }
    } else {
        None
    };
    let effective_decision = record
        .as_ref()
        .and_then(|r| r.decision.as_ref())
        .or(spec.decision.as_ref());

    let is_decision = matches!(spec.tier, s5d::Tier::Decision);

    // Header
    let tier_label = format!("{}", spec.tier).to_uppercase();
    let title = effective_decision
        .as_ref()
        .map(|d| d.title.as_str())
        .unwrap_or(&spec.id);
    println!("{}: {}", tier_label.cyan().bold(), title.bold());
    println!("{}", "━".repeat(60));

    if is_decision {
        show_decision(&spec, effective_decision);
    } else {
        show_feature(&spec, record.as_ref());
    }

    Ok(())
}

fn show_decision(spec: &s5d::Spec, decision: Option<&s5d::DecisionRecord>) {
    // Problem/context
    if let Some(ref problem) = spec.problem {
        println!();
        println!("  {}: {}", "Signal".dimmed(), problem.signal());
    }
    if let Some(dec) = decision {
        if !dec.context.is_empty() {
            println!("  {}: {}", "Context".dimmed(), dec.context);
        }
    }
    println!();

    // Hypothesis tree — use decision from record (or spec fallback)
    let winner_id = decision.map(|d| d.winner_id.as_str()).unwrap_or("");

    for (i, hyp) in spec.hypotheses.iter().enumerate() {
        let is_last = i == spec.hypotheses.len() - 1;
        let connector = if is_last { "└─" } else { "├─" };
        let pipe = if is_last { "   " } else { "│  " };

        let is_winner = hyp.id == winner_id;
        let is_invalid = hyp.layer == "invalid";

        // Hypothesis header
        let status = if is_winner {
            format!("{} [{}]", "WINNER".green().bold(), hyp.layer.green())
        } else if is_invalid {
            "ELIMINATED".red().to_string()
        } else {
            format!("[{}]", hyp.layer)
        };

        let marker = if is_winner {
            "✅"
        } else if is_invalid {
            "❌"
        } else {
            "⬜"
        };
        println!(
            "  {} {} {}  {}",
            connector,
            marker,
            hyp.title.bold(),
            status
        );

        // Evidence
        for (j, ev) in hyp.evidence.iter().enumerate() {
            let ev_last = j == hyp.evidence.len() - 1;
            let ev_connector = if ev_last { "└─" } else { "├─" };

            let verdict_colored = match ev.verdict.as_str() {
                "pass" => "PASS".green().to_string(),
                "fail" => "FAIL".red().to_string(),
                "refine" => "REFINE".yellow().to_string(),
                other => other.to_string(),
            };

            // Truncate content to first line or 80 chars
            let content_short = ev.content.lines().next().unwrap_or("");
            let content_display = if content_short.len() > 72 {
                format!("{}…", &content_short[..72])
            } else {
                content_short.to_string()
            };

            println!(
                "  {} {} {}: {}  {}",
                pipe,
                ev_connector,
                ev.evidence_type.dimmed(),
                content_display,
                verdict_colored
            );
        }

        if !is_last {
            println!("  │");
        }
    }

    // Decision summary — from record (or spec fallback)
    if let Some(dec) = decision {
        println!();
        println!("  {}: {}", "Decision".cyan(), dec.decision);
        if !dec.consequences.is_empty() {
            println!("  {}: {}", "Consequences".dimmed(), dec.consequences);
        }
        if let Some(ref ts) = dec.decided_at {
            let date = ts.split('T').next().unwrap_or(ts);
            println!("  {}: {}", "Decided".dimmed(), date);
        }
    }
}

fn show_feature(spec: &s5d::Spec, record: Option<&s5d::Record>) {
    println!("  {}: {}", "Tier".dimmed(), spec.tier);
    println!("  {}: {}", "Product".dimmed(), spec.product);

    if let Some(ref problem) = spec.problem {
        if let Some(card) = problem.as_card() {
            if let Some(ref status) = card.status {
                println!("  {}: {}", "Status".dimmed(), status);
            }
        }
    }

    if let Some(ref workflow) = spec.workflow {
        if let Some(ref mode) = workflow.mode {
            println!("  {}: {}", "Workflow".dimmed(), mode);
        }
        if !workflow.phases.is_empty() {
            println!("  {}: {}", "Phases".dimmed(), workflow.phases.len());
        }
    }

    if let Some(record) = record {
        if let Some(ref active_phase) = record.active_phase {
            println!("  {}: {}", "Active phase".dimmed(), active_phase);
        }
    }

    if let Some(ref artifacts) = spec.artifacts {
        let domains = artifacts.domains.len();
        let capabilities = artifacts.capabilities.len();
        let entities = artifacts.entities.len();
        let components = artifacts.components.len();

        if domains + capabilities + entities + components > 0 {
            println!();
            println!(
                "  {}: {} domains, {} capabilities, {} entities, {} components",
                "Artifacts".cyan(),
                domains,
                capabilities,
                entities,
                components
            );
        }
    }

    if !spec.gates.is_empty() {
        println!("  {}: {}", "Gates".dimmed(), spec.gates.len());
    }

    if let Some(reflection) = record.and_then(|r| r.reflection.as_ref()) {
        if let Some(ref verdict) = reflection.verdict {
            println!(
                "  {}: {}",
                "Verdict".dimmed(),
                colorize_outcome_verdict(verdict)
            );
        }
        if let Some(ref window) = reflection.measurement_window {
            println!("  {}: {}", "Measure".dimmed(), window);
        }
        if !reflection.telemetry_refs.is_empty() {
            println!(
                "  {}: {}",
                "Telemetry".dimmed(),
                reflection.telemetry_refs.len()
            );
        }
    }
}

// ── AddHypothesis ─────────────────────────────────────────────────────────────

fn slugify(title: &str) -> String {
    let re = regex::Regex::new("[^a-zA-Z0-9]+").unwrap();
    let lower = title.to_lowercase();
    let slug = re.replace_all(&lower, "-");
    slug.trim_matches('-').to_string()
}

fn run_add_hypothesis(
    spec_path: &str,
    custom_id: Option<&str>,
    title: &str,
    content: &str,
    scope: &str,
    kind: &str,
    rationale: Option<&str>,
) -> anyhow::Result<()> {
    let (path, mut spec) = load_spec_yaml(spec_path)?;

    if !matches!(spec.tier, s5d::Tier::Decision) {
        anyhow::bail!(
            "add-hypothesis only works on decision-tier specs (this spec is {})",
            spec.tier
        );
    }

    if let Some(cid) = custom_id {
        s5d::sanitize_id(cid)?;
    }

    let hyp_id = custom_id
        .map(|s| s.to_string())
        .unwrap_or_else(|| slugify(title));

    if spec.hypotheses.iter().any(|h| h.id == hyp_id) {
        anyhow::bail!("hypothesis '{}' already exists in this spec", hyp_id);
    }

    let hyp = s5d::Hypothesis {
        id: hyp_id.clone(),
        title: title.into(),
        content: content.into(),
        scope: scope.into(),
        kind: kind.into(),
        layer: "L0".into(),
        r_eff: None,
        evidence: vec![],
        depends_on: vec![],
        rationale: rationale.map(|s| s.into()),
        spec_ref: None,
    };

    spec.hypotheses.push(hyp);
    save_spec_yaml(&path, &spec)?;

    println!("{} Added hypothesis: {} (L0)", "ok".green(), hyp_id);
    Ok(())
}

// ── AddEvidence ───────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn run_add_evidence(
    spec_path: &str,
    hypothesis_id: &str,
    evidence_type: &str,
    content: &str,
    verdict: &str,
    carrier_ref: Option<&str>,
    formality: Option<u8>,
    claim_scope: Option<String>,
    reliability: Option<f64>,
) -> anyhow::Result<()> {
    if let Some(f) = formality {
        if !(1..=5).contains(&f) {
            eprintln!("error: --formality must be between 1 and 5");
            std::process::exit(1);
        }
    }
    if let Some(r) = reliability {
        if !(0.0..=1.0).contains(&r) {
            eprintln!("error: --reliability must be between 0.0 and 1.0");
            std::process::exit(1);
        }
    }
    s5d::sanitize_id(evidence_type)?;
    let (path, mut spec) = load_spec_yaml(spec_path)?;

    if !matches!(spec.tier, s5d::Tier::Decision) {
        anyhow::bail!(
            "add-evidence only works on decision-tier specs (this spec is {})",
            spec.tier
        );
    }

    let hyp = spec
        .hypotheses
        .iter_mut()
        .find(|h| h.id == hypothesis_id)
        .ok_or_else(|| anyhow::anyhow!("hypothesis not found: {}", hypothesis_id))?;

    let ev_id = format!(
        "{}-{}-{}",
        chrono::Utc::now().format("%Y%m%d"),
        evidence_type.replace(':', "-"),
        hypothesis_id,
    );

    let valid_until = (chrono::Utc::now() + chrono::Duration::days(90)).to_rfc3339();

    let ev = s5d::HypothesisEvidence {
        id: ev_id,
        evidence_type: evidence_type.into(),
        content: content.into(),
        verdict: verdict.into(),
        valid_until: Some(valid_until),
        carrier_ref: carrier_ref.map(|s| s.into()),
        formality,
        claim_scope: claim_scope
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default(),
        congruence_level: None,
        reliability,
    };
    hyp.evidence.push(ev);

    let new_layer = match verdict {
        "pass" if hyp.layer == "L0" => {
            hyp.layer = "L1".into();
            "L1"
        }
        "pass" if hyp.layer == "L1" => {
            hyp.layer = "L2".into();
            "L2"
        }
        "fail" => {
            hyp.layer = "invalid".into();
            "invalid"
        }
        _ => hyp.layer.as_str(),
    };

    let new_layer = new_layer.to_string();
    save_spec_yaml(&path, &spec)?;

    println!(
        "{} Added evidence to {}: {} → layer {}",
        "ok".green(),
        hypothesis_id,
        verdict,
        new_layer
    );
    Ok(())
}

// ── Decide ────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn run_decide(
    spec_path: &str,
    title: &str,
    winner: &str,
    rejected: Option<&str>,
    context: &str,
    decision: &str,
    rationale: &str,
    consequences: &str,
    force: bool,
    confirmed_by: Option<&str>,
    challenge_summary: Option<&str>,
    challenge_mode: Option<&str>,
    no_challenge: bool,
) -> anyhow::Result<()> {
    let (path, spec) = load_spec_yaml(spec_path)?;

    // Resolve project and load record from correct path (.s5d/records/, not packages/)
    let abs_path = path.canonicalize()?;
    let project = s5d::S5dProject::find(&abs_path)
        .ok_or_else(|| anyhow::anyhow!("no .s5d/ found — is this file inside an s5d project?"))?;
    let spec_filename = abs_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("cannot determine filename from path"))?
        .to_string_lossy()
        .into_owned();
    let record: Option<s5d::Record> = project.load_record(&spec_filename)?;

    let checks = s5d::check_decide(&spec, &record, &confirmed_by.map(|s| s.to_string()));
    s5d::enforce_checks(&checks, force)?;

    if !matches!(spec.tier, s5d::Tier::Decision) {
        anyhow::bail!(
            "decide only works on decision-tier specs (this spec is {})",
            spec.tier
        );
    }

    let winner_hyp = spec
        .hypotheses
        .iter()
        .find(|h| h.id == winner)
        .ok_or_else(|| anyhow::anyhow!("winner hypothesis not found: {}", winner))?;

    if winner_hyp.layer == "L1" {
        eprintln!(
            "  {} winner hypothesis {} is only L1 — L2 recommended before deciding",
            "warn:".yellow(),
            winner
        );
    } else if winner_hyp.layer != "L2" {
        eprintln!(
            "  {} winner hypothesis {} has layer {} (expected L2)",
            "warn:".yellow(),
            winner,
            winner_hyp.layer
        );
    }

    // Enforce spec_ref on winner hypothesis (feature decomposition)
    if winner_hyp.spec_ref.is_none() {
        if force {
            eprintln!(
                "  {} Winner hypothesis '{}' has no spec_ref — proceeding anyway (--force)",
                "warn:".yellow(),
                winner
            );
        } else {
            eprintln!(
                "  {} Winner hypothesis '{}' has no spec_ref (feature decomposition).",
                "error:".red(),
                winner
            );
            eprintln!("  Decision without modeling the implementation is guesswork.");
            eprintln!(
                "  Create a feature spec: s5d new <id> --tier standard --hypothesis-id {}",
                winner
            );
            eprintln!("  Then re-run decide. Use --force to skip this check.");
            anyhow::bail!(
                "winner hypothesis '{}' missing spec_ref — use --force to override",
                winner
            );
        }
    }

    // ── Adversarial challenge gate (centralized) ────────────────────────────
    let challenge = if no_challenge {
        eprintln!(
            "  {} Adversarial challenge skipped (--no-challenge)",
            "warn:".yellow()
        );
        None
    } else if let Some(summary) = challenge_summary {
        let mode = challenge_mode.unwrap_or("standard").to_string();
        Some(s5d::Challenge {
            mode,
            passed: true,
            summary: summary.to_string(),
            checks: vec![],
        })
    } else if force {
        eprintln!(
            "  {} No adversarial challenge provided — proceeding anyway (--force)",
            "warn:".yellow()
        );
        None
    } else {
        None
    };

    if let Some(msg) = s5d::check_challenge(&challenge, &spec.tier, force, no_challenge) {
        anyhow::bail!("{}", msg);
    }

    let rejected_ids: Vec<String> = match rejected {
        Some(s) => s
            .split(',')
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .collect(),
        None => vec![],
    };

    let now = chrono::Utc::now();
    let expires = now + chrono::Duration::days(90);
    let decision_record = s5d::DecisionRecord {
        title: title.into(),
        winner_id: winner.into(),
        rejected_ids,
        context: context.into(),
        decision: decision.into(),
        rationale: rationale.into(),
        consequences: consequences.into(),
        decided_at: Some(now.to_rfc3339()),
        confirmed_by: confirmed_by.map(|s| s.to_string()),
        expires_at: Some(expires.format("%Y-%m-%d").to_string()),
        do_list: vec![],
        dont_list: vec![],
        challenge,
    };

    // Two-File Model: spec is immutable after approve; write decision to record
    let spec_sha = s5d::S5dProject::file_sha256(&abs_path)?;
    let mut record = record.unwrap_or_else(|| s5d::generate_record(&spec_filename, &spec_sha));
    record.decision = Some(decision_record);
    project.save_record(&spec_filename, &record)?;

    println!(
        "{} Decision recorded: {} (winner: {})",
        "ok".green(),
        title,
        winner
    );

    Ok(())
}

// ── Reflect ───────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn run_reflect(
    spec_arg: &str,
    verdict: Option<&str>,
    measurement_window: Option<&str>,
    summary: &str,
    worked: &str,
    issues: &str,
    follow_ups: &str,
    evidence: &[String],
    telemetry_refs: &[String],
    heuristics: &[String],
    structured_issues_raw: &[String],
) -> anyhow::Result<()> {
    let (project, _spec_path, _spec, spec_filename) = load_spec_context(spec_arg)?;

    let mut record = project.load_record(&spec_filename)?.ok_or_else(|| {
        anyhow::anyhow!("no record found for {} — run preview first", spec_filename)
    })?;

    let worked_list: Vec<String> = worked
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let issues_list: Vec<String> = issues
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let follow_up_list: Vec<s5d::FollowUp> = follow_ups
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(|id| s5d::FollowUp {
            id,
            priority: None,
            description: None,
        })
        .collect();

    let evidence_list: Vec<s5d::Evidence> = evidence
        .iter()
        .map(|e| s5d::Evidence {
            path: e.clone(),
            sha256: None,
            formality: None,
            claim_scope: None,
            reliability: None,
        })
        .collect();

    let parsed_issues: Vec<s5d::Issue> = structured_issues_raw
        .iter()
        .map(|raw| {
            let parts: Vec<&str> = raw.splitn(4, '|').collect();
            s5d::Issue {
                description: parts.first().unwrap_or(&"").to_string(),
                root_cause: parts
                    .get(1)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string()),
                fix: parts
                    .get(2)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string()),
                severity: parts
                    .get(3)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string()),
            }
        })
        .collect();

    let normalized_verdict = verdict.map(normalize_outcome_verdict).transpose()?;

    record.reflection = Some(s5d::Reflection {
        verdict: normalized_verdict,
        measurement_window: measurement_window.map(|s| s.to_string()),
        telemetry_refs: telemetry_refs.to_vec(),
        summary: Some(summary.to_string()),
        worked: worked_list,
        issues: issues_list,
        structured_issues: parsed_issues,
        follow_ups: follow_up_list,
        evidence: evidence_list.clone(),
        heuristics: heuristics.to_vec(),
    });

    // Transition lifecycle status to Operated — closed by fact, not by timeout
    record.status = s5d::SpecStatus::Operated;
    record.status_history.push(s5d::StatusEntry {
        status: s5d::SpecStatus::Operated,
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    project.save_record(&spec_filename, &record)?;

    println!(
        "{} Lifecycle closed → operated. Reflect recorded.",
        "ok".green()
    );
    Ok(())
}

fn normalize_outcome_verdict(verdict: &str) -> anyhow::Result<String> {
    let normalized = verdict.trim().to_lowercase();
    let valid = ["confirmed", "refuted", "inconclusive", "iterate", "kill"];
    if valid.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        anyhow::bail!(
            "invalid verdict '{}' (use confirmed, refuted, inconclusive, iterate, kill)",
            verdict
        )
    }
}

fn colorize_outcome_verdict(verdict: &str) -> colored::ColoredString {
    match verdict {
        "confirmed" => "CONFIRMED".green(),
        "refuted" => "REFUTED".red(),
        "inconclusive" => "INCONCLUSIVE".yellow(),
        "iterate" => "ITERATE".yellow(),
        "kill" => "KILL".red().bold(),
        other => other.normal(),
    }
}

// ── Route ────────────────────────────────────────────────────────────────────

fn run_route(description: &str, format: &str) -> anyhow::Result<()> {
    if description.is_empty() {
        anyhow::bail!("description is required — describe the task to classify");
    }
    let result = s5d::route(description);
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        _ => {
            println!("{}", result);
        }
    }
    Ok(())
}

// ── Search ───────────────────────────────────────────────────────────────────

fn run_search(query: &str) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let project = s5d::S5dProject::find(&cwd)
        .ok_or_else(|| anyhow::anyhow!("no .s5d/ found (run `s5d init` first)"))?;

    let query_lower = query.to_lowercase();
    let mut found = 0;

    for (path, spec) in project.discover_specs()? {
        let yaml = std::fs::read_to_string(&path).unwrap_or_default();
        if yaml.to_lowercase().contains(&query_lower) {
            found += 1;
            let tier = spec.tier.to_string();
            let title = spec
                .meta
                .as_ref()
                .map(|m| m.title.as_str())
                .unwrap_or(&spec.id);
            println!("{} [{}] {} — {}", "match".green(), tier, spec.id, title);
            for (i, line) in yaml.lines().enumerate() {
                if line.to_lowercase().contains(&query_lower) {
                    println!("  :{} {}", i + 1, line.trim());
                }
            }
            println!();
        }
    }

    // Also search records (.s5d/records/)
    let records_dir = project.s5d_dir().join("records");
    if records_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&records_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "yaml") {
                    let content = std::fs::read_to_string(&path).unwrap_or_default();
                    if content.to_lowercase().contains(&query_lower) {
                        found += 1;
                        println!(
                            "{} [record] {}",
                            "match".green(),
                            path.file_name().unwrap_or_default().to_string_lossy()
                        );
                        for (i, line) in content.lines().enumerate() {
                            if line.to_lowercase().contains(&query_lower) {
                                println!("  :{} {}", i + 1, line.trim());
                            }
                        }
                        println!();
                    }
                }
            }
        }
    }

    let knowledge_dirs = ["knowledge/L0", "knowledge/L1", "knowledge/L2", "decisions"];
    for dir in &knowledge_dirs {
        let dir_path = project.s5d_dir().join(dir);
        if dir_path.exists() {
            if let Ok(entries) = std::fs::read_dir(&dir_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "md") {
                        let content = std::fs::read_to_string(&path).unwrap_or_default();
                        if content.to_lowercase().contains(&query_lower) {
                            found += 1;
                            println!(
                                "{} [{}] {}",
                                "match".green(),
                                dir,
                                path.file_name().unwrap_or_default().to_string_lossy()
                            );
                            for (i, line) in content.lines().enumerate() {
                                if line.to_lowercase().contains(&query_lower) {
                                    println!("  :{} {}", i + 1, line.trim());
                                }
                            }
                            println!();
                        }
                    }
                }
            }
        }
    }

    if found == 0 {
        println!("No matches found.");
    } else {
        println!("{} match(es) found.", found);
    }
    Ok(())
}

// ── Spec YAML load/save helpers ───────────────────────────────────────────────

fn load_spec_yaml(spec_path: &str) -> anyhow::Result<(std::path::PathBuf, s5d::Spec)> {
    let path = std::path::Path::new(spec_path);
    if !path.exists() {
        anyhow::bail!("spec not found: {}", spec_path);
    }
    let content = std::fs::read_to_string(path)?;
    let spec: s5d::Spec =
        serde_yaml::from_str(&content).map_err(|e| anyhow::anyhow!("YAML parse error: {}", e))?;
    Ok((path.to_path_buf(), spec))
}

fn save_spec_yaml(path: &std::path::Path, spec: &s5d::Spec) -> anyhow::Result<()> {
    let yaml = serde_yaml::to_string(spec)?;
    std::fs::write(path, yaml)?;
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn load_spec_context(
    spec_arg: &str,
) -> anyhow::Result<(s5d::S5dProject, std::path::PathBuf, s5d::Spec, String)> {
    let path = std::path::Path::new(spec_arg);
    if !path.exists() {
        anyhow::bail!("file not found: {}", spec_arg);
    }
    let content = std::fs::read_to_string(path)?;
    let spec: s5d::Spec =
        serde_yaml::from_str(&content).map_err(|e| anyhow::anyhow!("YAML parse error: {}", e))?;

    let abs_path = path.canonicalize()?;
    let project = s5d::S5dProject::find(&abs_path)
        .ok_or_else(|| anyhow::anyhow!("no .s5d/ found — is this file inside an s5d project?"))?;

    let spec_filename = abs_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("cannot determine filename from path"))?
        .to_string_lossy()
        .into_owned();

    Ok((project, abs_path, spec, spec_filename))
}

#[cfg(test)]
mod agents_md_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_file_when_absent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        assert_eq!(ensure_agents_md(&path).unwrap(), AgentsUpdate::Created);
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains(AGENTS_BEGIN));
        assert!(body.contains(AGENTS_END));
        assert!(body.contains("s5d_route"));
    }

    #[test]
    fn appends_when_no_markers() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        std::fs::write(&path, "# Project\n\nExisting content.\n").unwrap();
        assert_eq!(ensure_agents_md(&path).unwrap(), AgentsUpdate::Inserted);
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.starts_with("# Project\n"));
        assert!(body.contains("Existing content."));
        assert!(body.contains(AGENTS_BEGIN));
    }

    #[test]
    fn idempotent_on_second_run() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        ensure_agents_md(&path).unwrap();
        assert_eq!(ensure_agents_md(&path).unwrap(), AgentsUpdate::Unchanged);
    }

    #[test]
    fn replaces_stale_block() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        let stale = format!("# X\n\n{}\nold stuff\n{}\n", AGENTS_BEGIN, AGENTS_END);
        std::fs::write(&path, &stale).unwrap();
        assert_eq!(ensure_agents_md(&path).unwrap(), AgentsUpdate::Replaced);
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.starts_with("# X\n"));
        assert!(!body.contains("old stuff"));
        assert!(body.contains("s5d_route"));
    }

    #[test]
    fn preserves_surrounding_content_on_replace() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        let stale = format!(
            "# Top\n\n{}\nold\n{}\n\n## Footer section\n",
            AGENTS_BEGIN, AGENTS_END
        );
        std::fs::write(&path, &stale).unwrap();
        ensure_agents_md(&path).unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("# Top"));
        assert!(body.contains("## Footer section"));
    }
}
