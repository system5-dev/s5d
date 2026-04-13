use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(name = "s5d", about = "S5D — decision and validation layer for repo changes")]
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
    /// Seed alias table from bootstrap manifest
    Bootstrap {
        /// Path to bootstrap manifest YAML
        manifest: String,
    },
    /// Print environment fingerprint (tool versions hash)
    Cg,
    /// Start stdio MCP server (for Claude Code integration)
    Mcp,
    /// Register s5d MCP server in project .mcp.json
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
    },
}

#[derive(Subcommand)]
enum IndexCommand {
    /// Check index.yaml consistency
    Check,
    /// Rebuild index.yaml from packages/
    Sync,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        S5dCommand::Init { claude, cursor, codex, gemini, all } => run_init(claude, cursor, codex, gemini, all),
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
        S5dCommand::Approve { spec, reviewer, require_owner } => run_approve(&spec, &reviewer, require_owner),
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
        S5dCommand::Bootstrap { manifest } => run_bootstrap(&manifest),
        S5dCommand::GraphCheck { spec } => run_graph_check(&spec),
        S5dCommand::DriftCheck { spec } => run_drift_check(spec.as_deref()),
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
        S5dCommand::Route { description, format } => run_route(&description.join(" "), &format),
        S5dCommand::Search { query } => run_search(&query),
        S5dCommand::Reflect {
            spec,
            summary,
            worked,
            issues,
            follow_ups,
            evidence,
            heuristics,
            structured_issues,
        } => run_reflect(
            &spec,
            &summary,
            &worked,
            &issues,
            &follow_ups,
            &evidence,
            &heuristics,
            &structured_issues,
        ),
        S5dCommand::Mcp => s5d::mcp::run_mcp_server(),
        S5dCommand::Install {
            s5d_path,
            dry_run,
            uninstall,
        } => run_install(s5d_path.as_deref(), dry_run, uninstall),
    }
}

// ── Install ───────────────────────────────────────────────────────────────────

fn run_install(
    s5d_path: Option<&str>,
    dry_run: bool,
    uninstall: bool,
) -> anyhow::Result<()> {
    // Always write to project-level .mcp.json (vendor-agnostic)
    let settings_path = std::path::PathBuf::from(".mcp.json");

    // Resolve s5d binary path
    let binary_path = if let Some(p) = s5d_path {
        std::path::PathBuf::from(p)
    } else {
        std::env::current_exe()?
    };
    let binary_str = binary_path.to_string_lossy().into_owned();

    // Read or create settings
    let mut settings: serde_json::Value = if settings_path.exists() {
        let raw = std::fs::read_to_string(&settings_path)?;
        serde_json::from_str(&raw).unwrap_or(serde_json::Value::Object(Default::default()))
    } else {
        serde_json::Value::Object(Default::default())
    };

    // Build desired entry (needed for comparison before mutating)
    let desired = serde_json::json!({
        "type": "stdio",
        "command": binary_str,
        "args": ["mcp"]
    });

    // Mutate settings in a block so the borrow ends before serialization
    enum Action {
        Write,
        DryRun(String),
        AlreadyDone,
        NotFound,
    }
    let action = {
        let mcp_servers = settings
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("settings.json root is not an object"))?
            .entry("mcpServers")
            .or_insert(serde_json::Value::Object(Default::default()))
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("mcpServers is not an object"))?;

        if uninstall {
            if mcp_servers.remove("s5d").is_some() {
                if dry_run {
                    Action::DryRun("remove".to_string())
                } else {
                    Action::Write
                }
            } else {
                Action::NotFound
            }
        } else if let Some(existing) = mcp_servers.get("s5d") {
            if existing == &desired {
                Action::AlreadyDone
            } else if dry_run {
                Action::DryRun("write".to_string())
            } else {
                mcp_servers.insert("s5d".to_string(), desired.clone());
                Action::Write
            }
        } else if dry_run {
            Action::DryRun("write".to_string())
        } else {
            mcp_servers.insert("s5d".to_string(), desired.clone());
            Action::Write
        }
    };

    match action {
        Action::Write => {
            if let Some(parent) = settings_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let out = serde_json::to_string_pretty(&settings)?;
            std::fs::write(&settings_path, out)?;
            if uninstall {
                println!("{} Removed s5d MCP server", "ok".green());
                println!("   Settings: {}", settings_path.display());
                println!();
                println!("   Restart Claude Code to deactivate.");
            } else {
                println!("{} Registered s5d MCP server", "ok".green());
                println!("   Settings: {}", settings_path.display());
                println!("   Command:  {} mcp", binary_str);
                println!();
                println!("   Restart Claude Code to activate.");
            }
        }
        Action::AlreadyDone => {
            println!("{} s5d already registered, no change", "ok".green());
            println!("   Settings: {}", settings_path.display());
            println!("   Command:  {} mcp", binary_str);
        }
        Action::NotFound => {
            println!(
                "{} s5d MCP entry not found, nothing to remove",
                "ok".green()
            );
        }
        Action::DryRun(kind) => {
            if kind == "remove" {
                println!("{} Would remove s5d MCP entry", "dry-run".yellow());
                println!("   Settings: {}", settings_path.display());
            } else {
                println!(
                    "{} Would write to {}",
                    "dry-run".yellow(),
                    settings_path.display()
                );
                println!("   mcpServers.s5d:");
                println!("     type:    stdio");
                println!("     command: {}", binary_str);
                println!("     args:    [\"mcp\"]");
            }
        }
    }
    Ok(())
}

// ── Init ──────────────────────────────────────────────────────────────────────

fn run_init(claude: bool, cursor: bool, codex: bool, gemini: bool, all: bool) -> anyhow::Result<()> {
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
            Ok(false) => println!("    {} Claude (.mcp.json) — already registered", "✓".green()),
            Ok(true) => println!("    {} Claude (.mcp.json)", "✓".green()),
            Err(e) => println!("    {} Claude: {}", "⚠".yellow(), e),
        }
    }

    // Cursor → .cursor/mcp.json
    if do_cursor {
        let cursor_path = cwd.join(".cursor").join("mcp.json");
        match register_mcp_json(&cursor_path, &binary_str) {
            Ok(false) => println!("    {} Cursor (.cursor/mcp.json) — already registered", "✓".green()),
            Ok(true) => println!("    {} Cursor (.cursor/mcp.json)", "✓".green()),
            Err(e) => println!("    {} Cursor: {}", "⚠".yellow(), e),
        }
    }

    // Codex CLI → .codex/config.toml
    if do_codex {
        let codex_path = cwd.join(".codex").join("config.toml");
        match register_mcp_toml(&codex_path, &binary_str) {
            Ok(false) => println!("    {} Codex (.codex/config.toml) — already registered", "✓".green()),
            Ok(true) => println!("    {} Codex (.codex/config.toml)", "✓".green()),
            Err(e) => println!("    {} Codex: {}", "⚠".yellow(), e),
        }
    }

    // Gemini CLI → .gemini/settings.json
    if do_gemini {
        let gemini_path = cwd.join(".gemini").join("settings.json");
        match register_mcp_json(&gemini_path, &binary_str) {
            Ok(false) => println!("    {} Gemini (.gemini/settings.json) — already registered", "✓".green()),
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
            std::fs::write(&local_settings_path, serde_json::to_string_pretty(&settings)?)?;
            println!(
                "    {} Added {} to {}",
                "✓".green(),
                perm_entry,
                local_settings_path.display()
            );
        }
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
    use toml_edit::{DocumentMut, Item, Table, value};

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
            eprintln!("  Domain owners must approve their domains. Remove --require-owner to allow.");
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
    let failed = results.iter().filter(|r| r.status == "failed").count();
    let skipped = results.iter().filter(|r| r.status == "skipped").count();

    for r in &results {
        let marker = match r.status.as_str() {
            "passed" => "pass".green(),
            "failed" => "fail".red(),
            _ => "skip".dimmed(),
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
    let effective_decision = if let Ok(abs) = path.canonicalize() {
        if let Some(project) = s5d::S5dProject::find(&abs) {
            let fname = abs.file_name().unwrap().to_string_lossy().into_owned();
            project.load_record(&fname)?.and_then(|r| r.decision)
        } else { None }
    } else { None }
    .or_else(|| spec.decision.clone());

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
        show_decision(&spec, effective_decision.as_ref());
    } else {
        show_feature(&spec);
    }

    Ok(())
}

fn show_decision(spec: &s5d::Spec, decision: Option<&s5d::DecisionRecord>) {
    // Problem/context
    if let Some(ref problem) = spec.problem {
        println!();
        println!("  {}: {}", "Signal".dimmed(), problem.signal());
    }
    if let Some(ref dec) = decision {
        if !dec.context.is_empty() {
            println!("  {}: {}", "Context".dimmed(), dec.context);
        }
    }
    println!();

    // Hypothesis tree — use decision from record (or spec fallback)
    let winner_id = decision
        .map(|d| d.winner_id.as_str())
        .unwrap_or("");

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
    if let Some(ref dec) = decision {
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

fn show_feature(spec: &s5d::Spec) {
    println!("  {}: {}", "Tier".dimmed(), spec.tier);
    println!("  {}: {}", "Product".dimmed(), spec.product);

    if let Some(ref problem) = spec.problem {
        if let Some(card) = problem.as_card() {
            if let Some(ref status) = card.status {
                println!("  {}: {}", "Status".dimmed(), status);
            }
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
    summary: &str,
    worked: &str,
    issues: &str,
    follow_ups: &str,
    evidence: &[String],
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
                root_cause: parts.get(1).filter(|s| !s.is_empty()).map(|s| s.to_string()),
                fix: parts.get(2).filter(|s| !s.is_empty()).map(|s| s.to_string()),
                severity: parts.get(3).filter(|s| !s.is_empty()).map(|s| s.to_string()),
            }
        })
        .collect();

    record.reflection = Some(s5d::Reflection {
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







