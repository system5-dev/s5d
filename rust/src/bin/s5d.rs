use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(name = "s5d", about = "S5D — Decision & Feature Framework")]
struct Cli {
    #[command(subcommand)]
    command: S5dCommand,
}

#[derive(Subcommand)]
enum S5dCommand {
    /// Initialize .s5d/ directory structure
    Init,
    /// Create a new spec from template
    New {
        /// Feature ID (e.g., "feat.orders.tracking")
        feature_id: String,
        /// Assurance tier
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
    /// Validate a spec file
    Validate {
        /// Path to .s5d.yaml file
        spec: String,
    },
    /// Show status of all specs
    Status,
    /// Print Comparability Gauge ID
    Cg,
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
    /// Reverse last import for a spec
    Rollback {
        /// Path to .s5d.yaml file
        spec: String,
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
    /// Graph/relation validation — DFS cycle detection
    GraphCheck {
        /// Path to .s5d.yaml file
        spec: String,
    },
    /// Architecture health check — coupling metrics, cycles, quality gate
    Check {
        /// Path to .s5d.yaml file
        spec: String,
        /// Minimum health score to pass (exit 1 if below)
        #[arg(long, default_value = "0")]
        threshold: u8,
        /// Compare against previous baseline snapshot
        #[arg(long)]
        compare: bool,
        /// Output format: text or json
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Display per-domain coupling metrics
    Metrics {
        /// Path to .s5d.yaml file
        spec: String,
        /// Output format: text or json
        #[arg(long, default_value = "text")]
        format: String,
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
    /// Validate contract sha256 digests and format structure
    ContractCheck {
        /// Path to .s5d.yaml file
        spec: String,
    },
    /// Generate architecture rules from a spec (layering, allowed edges, no cycles)
    GenerateRules {
        /// Path to .s5d.yaml file
        spec: String,
        /// Output format: yaml or json
        #[arg(long, default_value = "yaml")]
        format: String,
    },
    /// Generate REPORT.md with adoption metrics
    Report,
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
        /// Kind: system or episteme
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
        #[arg(long, help = "F-G-R Formality: rigor of evidence method (1-5)")]
        formality: Option<u8>,
        #[arg(
            long,
            help = "F-G-R ClaimScope: what the claim covers (comma-separated)"
        )]
        claim_scope: Option<String>,
        #[arg(long, help = "F-G-R Reliability: probability claim is true (0.0-1.0)")]
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
    },
    /// Generate diagram or interactive HTML from spec
    Render {
        /// Path to .s5d.yaml file
        spec: String,
        /// View type: domain, components, decision (auto-detect if omitted)
        #[arg(long)]
        view: Option<String>,
        /// Output format: mermaid (default) or html
        #[arg(long, default_value = "mermaid")]
        format: String,
        /// Output to file instead of stdout
        #[arg(long, short)]
        output: Option<String>,
    },
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
    /// Analyze a codebase and generate draft domain map
    Analyze {
        /// Path to project root (default: current directory)
        #[arg(default_value = ".")]
        path: String,
        /// Product name
        #[arg(long)]
        product: Option<String>,
        /// Spec ID prefix
        #[arg(long, default_value = "analysis")]
        id: String,
        /// Output file (default: stdout as YAML)
        #[arg(long, short)]
        output: Option<String>,
    },
    /// Rename a domain ID across the entire spec (cascades to capabilities, components, entities, links)
    RenameDomain {
        /// Path to .s5d.yaml file
        spec: String,
        /// Old domain ID
        #[arg(long)]
        old: String,
        /// New domain ID
        #[arg(long)]
        new: String,
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
    /// Show git state for S5D specs
    GitStatus {
        /// Path to .s5d.yaml file (optional — checks all if omitted)
        spec: Option<String>,
    },
    /// Codebase index management
    Codebase {
        #[command(subcommand)]
        command: CodebaseCommand,
    },
    /// Trace: map spec artifacts to code
    Trace {
        #[command(subcommand)]
        command: TraceCommand,
    },
    /// Learn phase — aggregate reflections, extract patterns, feed heuristics
    Learn {
        #[command(subcommand)]
        command: LearnCommand,
    },
    /// FPF spec search and management
    Fpf {
        #[command(subcommand)]
        command: FpfCommand,
    },
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
enum CodebaseCommand {
    /// Build or rebuild the full codebase index
    Index {
        /// Path to repository root (default: current dir)
        #[arg(default_value = ".")]
        path: String,
    },
    /// Incrementally update index using git diff
    Update {
        /// Path to repository root (default: current dir)
        #[arg(default_value = ".")]
        path: String,
    },
    /// Search the codebase index
    Search {
        /// Search query
        query: String,
        /// Path to repository root (default: current dir)
        #[arg(long, default_value = ".")]
        path: String,
        /// Number of results
        #[arg(long, default_value = "10")]
        top_k: usize,
    },
    /// Suggest domain boundaries from coupling analysis
    SuggestDomains {
        /// Path to repository root (default: current dir)
        #[arg(default_value = ".")]
        path: String,
        /// Directory depth for module grouping (1=top-level, 2=nested)
        #[arg(long, default_value = "1")]
        depth: usize,
        /// Minimum files per module to be considered a domain
        #[arg(long, default_value = "2")]
        min_files: usize,
        /// Output format
        #[arg(long, default_value = "text")]
        format: String,
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
enum TraceCommand {
    /// Build trace links from code to spec artifacts
    Build {
        /// Path to .s5d.yaml spec file
        spec: String,
    },
    /// Check trace coverage — which spec artifacts have code bindings
    Check {
        /// Path to .s5d.yaml spec file
        spec: String,
    },
    /// Show trace report
    Report {
        /// Filter by spec ID
        #[arg(long)]
        spec: Option<String>,
    },
    /// Compute blast radius — domains, files, symbols, WLNK
    BlastRadius {
        /// Path to .s5d.yaml spec file
        spec: String,
    },
}

#[derive(Subcommand)]
enum LearnCommand {
    /// Aggregate reflections from all operated specs
    Report,
    /// Show relevant heuristics from past specs for a new spec
    Feed {
        /// Path to .s5d.yaml spec file
        spec: String,
    },
}

#[derive(Subcommand)]
enum FpfCommand {
    /// Download FPF spec from GitHub and rebuild the search index
    Sync {
        /// Re-index even if the upstream version hasn't changed
        #[arg(short, long)]
        force: bool,
        /// Download embedding model (~640MB) and enable hybrid search
        #[arg(long)]
        embed: bool,
    },
    /// Search the FPF spec (BM25 full-text)
    Search {
        /// Query terms
        query: Vec<String>,
        /// Maximum results to return
        #[arg(short = 'n', long, default_value = "5")]
        limit: usize,
        /// Print the full section body instead of a snippet
        #[arg(long)]
        full: bool,
    },
    /// Look up a section by heading (partial match)
    Section {
        /// Heading text to search for
        heading: Vec<String>,
    },
    /// Show index status
    Status,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        S5dCommand::Init => run_init(),
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
        S5dCommand::Check {
            spec,
            threshold,
            compare,
            format,
        } => run_check(&spec, threshold, compare, &format),
        S5dCommand::Metrics { spec, format } => run_metrics(&spec, &format),
        S5dCommand::DriftCheck { spec } => run_drift_check(spec.as_deref()),
        S5dCommand::Reconcile { spec } => run_reconcile(spec.as_deref()),
        S5dCommand::ContractCheck { spec } => run_contract_check(&spec),
        S5dCommand::GenerateRules { spec, format } => run_generate_rules(&spec, &format),
        S5dCommand::Report => run_report(),
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
        ),
        S5dCommand::Render {
            spec,
            view,
            format,
            output,
        } => run_render(&spec, view.as_deref(), format.as_str(), output.as_deref()),
        S5dCommand::Show { spec } => run_show(&spec),
        S5dCommand::Search { query } => run_search(&query),
        S5dCommand::Analyze {
            path,
            product,
            id,
            output,
        } => run_analyze(&path, product.as_deref(), &id, output.as_deref()),
        S5dCommand::RenameDomain { spec, old, new } => run_rename_domain(&spec, &old, &new),
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
        S5dCommand::GitStatus { spec } => run_git_status(spec.as_deref()),
        S5dCommand::Codebase { command } => match command {
            CodebaseCommand::Index { path } => run_codebase_index(&path),
            CodebaseCommand::Update { path } => run_codebase_update(&path),
            CodebaseCommand::Search { query, path, top_k } => {
                run_codebase_search(&query, &path, top_k)
            }
            CodebaseCommand::SuggestDomains {
                path,
                depth,
                min_files,
                format,
            } => run_suggest_domains(&path, depth, min_files, &format),
        },
        S5dCommand::Trace { command } => match command {
            TraceCommand::Build { spec } => run_trace_build(&spec),
            TraceCommand::Check { spec } => run_trace_check(&spec),
            TraceCommand::Report { spec } => run_trace_report(spec.as_deref()),
            TraceCommand::BlastRadius { spec } => run_trace_blast_radius(&spec),
        },
        S5dCommand::Learn { command } => match command {
            LearnCommand::Report => run_learn_report(),
            LearnCommand::Feed { spec } => run_learn_feed(&spec),
        },
        S5dCommand::Fpf { command } => match command {
            FpfCommand::Sync { force, embed } => s5d::fpf::cmd_sync(force, embed),
            FpfCommand::Search { query, limit, full } => s5d::fpf::cmd_search(&query, limit, full),
            FpfCommand::Section { heading } => s5d::fpf::cmd_section(&heading),
            FpfCommand::Status => s5d::fpf::cmd_status(),
        },
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

fn run_init() -> anyhow::Result<()> {
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

    // Auto-register MCP server in project settings
    println!("\n  {} MCP server:", "mcp".bold());
    match run_install(None, false, false) {
        Ok(()) => {
            println!(
                "\n  {} Restart your Claude Code / agent session to activate s5d MCP tools.",
                "⚠".yellow()
            );
        }
        Err(e) => {
            println!(
                "    {} Could not auto-register MCP: {}",
                "⚠".yellow(),
                e
            );
            println!("    Run `s5d install --claude-project` manually.");
        }
    }

    // Auto-add mcp__s5d__* permission to .claude/settings.local.json
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

    println!("\n  {} Next steps:", "→".blue());
    println!("    1. s5d analyze <path> --product <name>   Build draft domain map");
    println!("    2. s5d codebase index <path>             Build codebase index for trace/search");
    println!("    3. s5d new <feature-id> --product <name> Create your first spec");
    println!();

    Ok(())
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

    // Auto-render after creating spec
    let _ = run_render(spec_path.to_str().unwrap_or(""), None, "mermaid", None);

    Ok(())
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

    // Recompute diff and verify diff_sha256
    let s5d_dir = project.s5d_dir();
    let mut aliases_check = s5d::AliasTable::load(&s5d_dir)?;
    if let Some(ref meta) = spec.meta {
        aliases_check.apply_renames(&spec.id, &meta.renames);
    }
    let fresh_actions = s5d::compute_diff(&spec, &mut aliases_check);
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

    // Auto-render after successful import
    let _ = run_render(spec_arg, None, "mermaid", None);

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

    // Tombstone all alias entries for this package
    let mut aliases = s5d::AliasTable::load(&s5d_dir)?;
    for entry in &mut aliases.packages {
        if entry.package_id.as_deref() == Some(&spec.id) && !entry.deprecated {
            entry.deprecated = true;
        }
    }
    for entry in &mut aliases.global {
        if entry.owning_package.as_deref() == Some(&spec.id) && !entry.deprecated {
            entry.deprecated = true;
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

fn run_analyze(
    path: &str,
    product: Option<&str>,
    id: &str,
    output: Option<&str>,
) -> anyhow::Result<()> {
    let path = std::path::Path::new(path)
        .canonicalize()
        .map_err(|_| anyhow::anyhow!("path not found: {}", path))?;

    let product_name = product.unwrap_or_else(|| {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Project")
    });

    let config = s5d::AnalysisConfig {
        path: path.clone(),
        product: product_name.to_string(),
        spec_id: id.to_string(),
    };

    let result = s5d::analyze(&config)?;

    println!("{} Scaffold generated: {}", "ok".green(), path.display());
    println!(
        "  Language: {}{}",
        result.language,
        result
            .framework
            .as_ref()
            .map(|f| format!(" ({})", f))
            .unwrap_or_default()
    );
    println!("  Domains:      {}", result.stats.domains_found);
    println!("  Entities:     {}", result.stats.entities_found);
    println!("  Capabilities: {}", result.stats.capabilities_found);
    println!("  Components:   {}", result.stats.components_found);
    println!("  Edges:        {}", result.stats.edges_found);

    let yaml = serde_yaml::to_string(&result.spec)?;

    if let Some(out_path) = output {
        std::fs::write(out_path, &yaml)?;
        println!("  Output: {}", out_path);
        println!();
        println!("  {} Next: review the draft spec, then run:", "hint".cyan());
        println!("    s5d validate {}", out_path);
        println!("    s5d graph-check {}", out_path);
        println!(
            "    # Or import directly: s5d preview {} && s5d approve {} && s5d import {}",
            out_path, out_path, out_path
        );
    } else {
        println!("\n{}", yaml);
        println!();
        println!(
            "  {} Next: save the output with -o <file.s5d.yaml>, then:",
            "hint".cyan()
        );
        println!(
            "    s5d analyze {} --product {} -o draft.s5d.yaml",
            path.display(),
            product_name
        );
    }

    Ok(())
}

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

fn run_contract_check(spec_arg: &str) -> anyhow::Result<()> {
    let (project, _spec_path, spec, _spec_filename) = load_spec_context(spec_arg)?;

    let errors = s5d::check_contracts(&spec, &project.root);
    if errors.is_empty() {
        println!("{} {} contracts ok", "ok".green(), spec_arg);
        Ok(())
    } else {
        for e in &errors {
            eprintln!("  {} {}", "error:".red(), e);
        }
        std::process::exit(7);
    }
}

// ── Generate Rules ────────────────────────────────────────────────────────────

fn run_generate_rules(spec_arg: &str, format: &str) -> anyhow::Result<()> {
    let (_project, _spec_path, spec, _spec_filename) = load_spec_context(spec_arg)?;

    let rules = s5d::generate_rules(&spec);
    if rules.is_empty() {
        eprintln!(
            "{} no rules generated (spec has no domains or classifications)",
            "warn:".yellow()
        );
        return Ok(());
    }

    let output = match format {
        "json" => s5d::format_rules_json(&rules, &spec.id),
        _ => s5d::format_rules_yaml(&rules, &spec.id),
    };
    print!("{}", output);
    eprintln!(
        "\n{} {} rules generated from {}",
        "ok".green(),
        rules.len(),
        spec.id
    );
    Ok(())
}

// ── Report ────────────────────────────────────────────────────────────────────

fn run_report() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let project = s5d::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;

    let data = s5d::compute_report(&project)?;
    let content = s5d::render_report(&data);

    let report_path = project.s5d_dir().join("REPORT.md");
    std::fs::write(&report_path, &content)?;

    println!(
        "{} Report written to: {}",
        "ok".green(),
        report_path.display()
    );
    println!();
    println!("  Total specs:    {}", data.total_specs);
    println!("  New (30d):      {}", data.leading.new_specs_30d);
    println!(
        "  Applied rate:   {:.0}%",
        data.lagging.applied_rate * 100.0
    );
    println!("  Synced rate:    {:.0}%", data.lagging.synced_rate * 100.0);
    println!("  Drift rate:     {:.0}%", data.anti.drift_rate * 100.0);
    println!("  Stale specs:    {}", data.anti.stale_specs);
    println!("  Operated:       {}", data.learn.operated_count);
    println!("  Heuristics:     {}", data.learn.total_heuristics);
    println!("  Follow-ups:     {}", data.learn.open_follow_ups);

    Ok(())
}

// ── Learn ────────────────────────────────────────────────────────────────────

fn run_learn_report() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let project = s5d::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;

    let report = s5d::aggregate_reflections(&project)?;

    println!("  Operated specs: {}", report.operated_count);

    if !report.heuristics.is_empty() {
        println!();
        println!("  Heuristics ({}):", report.heuristics.len());
        for h in &report.heuristics {
            println!("    {} [{}]", h.text, h.source_spec);
        }
    }

    if !report.recurring_issues.is_empty() {
        let recurring: Vec<_> = report
            .recurring_issues
            .iter()
            .filter(|(_, c)| *c > 1)
            .collect();
        if !recurring.is_empty() {
            println!();
            println!("  Recurring issues:");
            for (issue, count) in &recurring {
                println!("    \"{}\" ({} specs)", issue, count);
            }
        }
    }

    if !report.open_follow_ups.is_empty() {
        println!();
        println!("  Follow-ups pending ({}):", report.open_follow_ups.len());
        for fu in &report.open_follow_ups {
            let priority = fu.priority.as_deref().unwrap_or("-");
            println!("    {} [{}]  [{}]", fu.id, priority, fu.source_spec);
        }
    }

    if report.operated_count == 0 {
        println!();
        println!("  No operated specs yet.");
    }

    Ok(())
}

fn run_learn_feed(spec_arg: &str) -> anyhow::Result<()> {
    let (_project, _spec_path, spec, _spec_filename) = load_spec_context(spec_arg)?;
    let cwd = std::env::current_dir()?;
    let project = s5d::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;

    let feed = s5d::feed_heuristics(&project, &spec)?;

    if feed.is_empty() {
        println!("  No relevant heuristics from past specs.");
        return Ok(());
    }

    println!("  Relevant heuristics ({}):", feed.len());
    for h in &feed {
        println!("    {} [{}]", h.text, h.source_spec);
    }

    Ok(())
}

// ── Show ──────────────────────────────────────────────────────────────────────

fn run_show(spec_path: &str) -> anyhow::Result<()> {
    let (_path, spec) = load_spec_yaml(spec_path)?;

    let is_decision = matches!(spec.tier, s5d::Tier::Decision);

    // Header
    let tier_label = format!("{}", spec.tier).to_uppercase();
    let title = spec
        .decision
        .as_ref()
        .map(|d| d.title.as_str())
        .unwrap_or(&spec.id);
    println!("{}: {}", tier_label.cyan().bold(), title.bold());
    println!("{}", "━".repeat(60));

    if is_decision {
        show_decision(&spec);
    } else {
        show_feature(&spec);
    }

    Ok(())
}

fn show_decision(spec: &s5d::Spec) {
    // Problem/context
    if let Some(ref problem) = spec.problem {
        println!();
        println!("  {}: {}", "Signal".dimmed(), problem.signal());
    }
    if let Some(ref dec) = spec.decision {
        if !dec.context.is_empty() {
            println!("  {}: {}", "Context".dimmed(), dec.context);
        }
    }
    println!();

    // Hypothesis tree
    let winner_id = spec
        .decision
        .as_ref()
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

    // Decision summary
    if let Some(ref dec) = spec.decision {
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
) -> anyhow::Result<()> {
    let (path, spec) = load_spec_yaml(spec_path)?;

    // Load record — decision state lives in record, not spec
    let record_path = path.with_file_name(
        path.file_name().unwrap().to_string_lossy().replace(".s5d.yaml", ".record.yaml"),
    );
    let record: Option<s5d::Record> = if record_path.exists() {
        Some(serde_yaml::from_str(&std::fs::read_to_string(&record_path)?)?)
    } else {
        None
    };

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
    };

    // Two-File Model: spec is immutable after approve; write decision to record
    let abs_path = path.canonicalize()?;
    let project = s5d::S5dProject::find(&abs_path)
        .ok_or_else(|| anyhow::anyhow!("no .s5d/ found — is this file inside an s5d project?"))?;
    let spec_filename = abs_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("cannot determine filename from path"))?
        .to_string_lossy()
        .into_owned();

    let spec_sha = s5d::S5dProject::file_sha256(&abs_path)?;
    let mut record = project
        .load_record(&spec_filename)?
        .unwrap_or_else(|| s5d::generate_record(&spec_filename, &spec_sha));
    record.decision = Some(decision_record);
    project.save_record(&spec_filename, &record)?;

    println!(
        "{} Decision recorded: {} (winner: {})",
        "ok".green(),
        title,
        winner
    );

    // Auto-render after decision
    let _ = run_render(spec_path, None, "mermaid", None);

    Ok(())
}

// ── Rename Domain ─────────────────────────────────────────────────────────────

fn run_rename_domain(spec_path: &str, old_id: &str, new_id: &str) -> anyhow::Result<()> {
    let path = std::path::Path::new(spec_path).canonicalize()?;
    let content = std::fs::read_to_string(&path)?;
    let mut spec: s5d::models::Spec = serde_yaml::from_str(&content)?;

    s5d::infer::rename_domain_in_spec(&mut spec, old_id, new_id);

    // Also rename in domains list
    if let Some(ref mut artifacts) = spec.artifacts {
        for domain in &mut artifacts.domains {
            if domain.id == old_id {
                domain.id = new_id.to_string();
            }
        }
        // Deduplicate domains by id
        let mut seen = std::collections::HashSet::new();
        artifacts.domains.retain(|d| seen.insert(d.id.clone()));
    }

    // Deduplicate links
    if let Some(ref mut links) = spec.links {
        let mut seen = std::collections::HashSet::new();
        links.feature_to_domain.retain(|b| {
            let key = format!(
                "{}:{}",
                b.fields.get("feature").unwrap_or(&String::new()),
                b.fields.get("domain").unwrap_or(&String::new())
            );
            seen.insert(key)
        });
    }

    let yaml = serde_yaml::to_string(&spec)?;
    std::fs::write(&path, yaml)?;

    println!(
        "{} Renamed domain: {} → {} in {}",
        "ok".green(),
        old_id,
        new_id,
        spec_path
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
    let (project, spec_path, mut spec, spec_filename) = load_spec_context(spec_arg)?;

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

    // For decision specs: write production evidence back into winner hypothesis in spec YAML.
    if !evidence_list.is_empty() {
        if let Some(ref dec) = spec.decision.clone() {
            let winner_id = dec.winner_id.clone();
            let now_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
            if let Some(hyp) = spec.hypotheses.iter_mut().find(|h| h.id == winner_id) {
                for (i, ev) in evidence_list.iter().enumerate() {
                    hyp.evidence.push(s5d::HypothesisEvidence {
                        id: format!("prod-{}-{}", now_date, i + 1),
                        evidence_type: "external".to_string(),
                        content: format!("Production observation: {}", ev.path),
                        verdict: "pass".to_string(),
                        valid_until: None,
                        carrier_ref: Some(ev.path.clone()),
                        formality: Some(1),
                        claim_scope: vec!["operate".to_string()],
                        congruence_level: None,
                        reliability: None,
                    });
                }
                let yaml = serde_yaml::to_string(&spec)
                    .map_err(|e| anyhow::anyhow!("YAML serialization error: {}", e))?;
                std::fs::write(&spec_path, yaml)?;
                println!(
                    "{} Production evidence written to winner hypothesis '{}'",
                    "ok".green(),
                    winner_id
                );
            }
        }
    }

    println!(
        "{} Lifecycle closed → operated. Reflect recorded.",
        "ok".green()
    );
    Ok(())
}

// ── Render ────────────────────────────────────────────────────────────────────

fn run_render(
    spec_path: &str,
    view: Option<&str>,
    format: &str,
    output: Option<&str>,
) -> anyhow::Result<()> {
    let (_, spec) = load_spec_yaml(spec_path)?;
    let diagram = if format == "html" {
        s5d::render::render_domain_html(&spec)
    } else {
        s5d::render::render(&spec, view)
    };

    if let Some(out_path) = output {
        std::fs::write(out_path, &diagram)?;
        println!("{} Rendered to {}", "ok".green(), out_path);
    } else {
        println!("{}", diagram);
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

// ── GitStatus ────────────────────────────────────────────────────────────────

fn run_git_status(spec_path: Option<&str>) -> anyhow::Result<()> {
    use s5d::git_state;

    if let Some(path) = spec_path {
        let p = std::path::Path::new(path);
        let state = git_state::check_git_state(p)?;
        println!("Spec: {}", path);
        println!("  Tracked: {}", state.spec_tracked);
        println!("  On main: {}", state.on_main);
        println!("  Merged:  {}", state.merged_to_main);
        if let Some(msg) = &state.last_commit_msg {
            println!("  Last commit: {}", msg);
        }
        println!("  → {}", git_state::suggested_phase(&state));
    } else {
        // Check all specs in .s5d/packages/
        let cwd = std::env::current_dir()?;
        let project =
            s5d::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;
        let packages = project.s5d_dir().join("packages");
        if packages.exists() {
            for entry in std::fs::read_dir(&packages)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
                    let state = git_state::check_git_state(&path)?;
                    let name = path.file_stem().unwrap_or_default().to_str().unwrap_or("");
                    let phase = git_state::suggested_phase(&state);
                    println!("  {} → {}", name, phase);
                }
            }
        }
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

// ── Codebase ──────────────────────────────────────────────────────────────────

fn resolve_codebase_project(
    path: &str,
) -> anyhow::Result<(std::path::PathBuf, std::path::PathBuf)> {
    let abs_path = std::path::Path::new(path)
        .canonicalize()
        .map_err(|_| anyhow::anyhow!("path not found: {}", path))?;
    let project = s5d::S5dProject::find(&abs_path).ok_or_else(|| {
        anyhow::anyhow!(
            "no .s5d/ found for {} or its parent directories — run `s5d init` first",
            abs_path.display()
        )
    })?;
    let project_root = project.root.clone();
    let db_path = project.s5d_dir().join("codebase.db");
    Ok((project_root, db_path))
}

fn run_codebase_index(path: &str) -> anyhow::Result<()> {
    let (project_root, db_path) = resolve_codebase_project(path)?;
    let config = s5d::IndexConfig {
        db_path,
        ..Default::default()
    };
    let stats = s5d::index_codebase(&project_root, &config)?;
    println!(
        "{} Codebase indexed at: {}",
        "✓".green(),
        config.db_path.display()
    );
    println!("  Files:       {}", stats.files_indexed);
    println!("  Chunks:      {}", stats.chunks_indexed);
    println!("  Symbols:     {}", stats.symbols_found);
    println!("  Embeddings:  {}", stats.embeddings_generated);
    Ok(())
}

fn run_codebase_update(path: &str) -> anyhow::Result<()> {
    let (project_root, db_path) = resolve_codebase_project(path)?;
    let config = s5d::IndexConfig {
        db_path,
        ..Default::default()
    };
    let stats = s5d::update_codebase(&project_root, &config)?;
    println!("{} Codebase index updated", "ok".green());
    println!("  Files:       {}", stats.files_indexed);
    println!("  Chunks:      {}", stats.chunks_indexed);
    println!("  Symbols:     {}", stats.symbols_found);
    println!("  Embeddings:  {}", stats.embeddings_generated);
    Ok(())
}

fn run_codebase_search(query: &str, path: &str, top_k: usize) -> anyhow::Result<()> {
    let (_project_root, db_path) = resolve_codebase_project(path)?;
    if !db_path.exists() {
        anyhow::bail!(
            "no codebase index at {}; run `s5d codebase index` first",
            db_path.display()
        );
    }
    let index = s5d::CodebaseIndex::open(&db_path)?;
    let results = s5d::codebase::search::search(
        &index,
        query,
        s5d::codebase::search::SearchMode::Fts,
        top_k,
        None,
    )?;
    if results.is_empty() {
        println!("No results for {:?}", query);
        return Ok(());
    }
    for r in &results {
        println!(
            "{}:{}-{}  [score={:.4}]",
            r.file_path, r.start_line, r.end_line, r.score
        );
        let preview: String = r.text.lines().take(3).collect::<Vec<_>>().join(" | ");
        println!("  {}", preview);
    }
    Ok(())
}

fn run_suggest_domains(
    path: &str,
    depth: usize,
    min_files: usize,
    format: &str,
) -> anyhow::Result<()> {
    let (_project_root, db_path) = resolve_codebase_project(path)?;
    if !db_path.exists() {
        anyhow::bail!(
            "no codebase index at {}; run `s5d codebase index` first",
            db_path.display()
        );
    }
    let index = s5d::CodebaseIndex::open(&db_path)?;
    let report = s5d::codebase::suggest::suggest_domains(&index, depth, min_files)?;

    if report.domains.is_empty() {
        println!("No domains found. Ensure codebase is indexed with `s5d codebase index .`");
        return Ok(());
    }

    if format == "json" {
        // Simple JSON output
        println!("{{");
        println!("  \"total_files\": {},", report.total_files);
        println!("  \"total_edges\": {},", report.total_edges);
        println!("  \"domains\": [");
        for (i, d) in report.domains.iter().enumerate() {
            let comma = if i + 1 < report.domains.len() { "," } else { "" };
            println!(
                "    {{\"name\": {:?}, \"classification\": {:?}, \"files\": {}, \"symbols\": {}, \"ca\": {}, \"ce\": {}, \"instability\": {:.2}, \"cohesion\": {:.2}, \"confidence\": {:.2}}}{}",
                d.name, d.classification, d.files.len(), d.symbols, d.ca, d.ce, d.instability, d.cohesion, d.confidence, comma
            );
        }
        println!("  ],");
        println!("  \"merge_candidates\": [");
        for (i, m) in report.merge_candidates.iter().enumerate() {
            let comma = if i + 1 < report.merge_candidates.len() { "," } else { "" };
            println!(
                "    {{\"a\": {:?}, \"b\": {:?}, \"mutual_edges\": {}, \"coupling_ratio\": {:.2}, \"reason\": {:?}}}{}",
                m.module_a, m.module_b, m.mutual_edges, m.coupling_ratio, m.reason, comma
            );
        }
        println!("  ]");
        println!("}}");
        return Ok(());
    }

    // Text output
    println!(
        "{} Coupling-based domain suggestion (depth={}, min_files={})\n",
        "ok".green(),
        depth,
        min_files
    );
    println!(
        "  Files: {}  Import edges: {}\n",
        report.total_files, report.total_edges
    );

    // Header
    println!(
        "  {:<20} {:<12} {:>5} {:>5} {:>4} {:>4} {:>6} {:>7} {:>6}",
        "Domain", "Class", "Files", "Syms", "Ca", "Ce", "I", "Coh", "Conf"
    );
    println!("  {}", "─".repeat(85));

    for d in &report.domains {
        let class_colored = match d.classification.as_str() {
            "core" => d.classification.red().bold(),
            "supporting" => d.classification.yellow(),
            _ => d.classification.dimmed(),
        };
        println!(
            "  {:<20} {:<12} {:>5} {:>5} {:>4} {:>4} {:>5.2} {:>6.2} {:>5.2}",
            d.name,
            class_colored,
            d.files.len(),
            d.symbols,
            d.ca,
            d.ce,
            d.instability,
            d.cohesion,
            d.confidence
        );
    }

    if !report.merge_candidates.is_empty() {
        println!("\n  {} Merge candidates (high mutual coupling):", "warn:".yellow());
        for m in &report.merge_candidates {
            println!(
                "    {} ↔ {} — {}",
                m.module_a, m.module_b, m.reason
            );
        }
    }

    Ok(())
}

// ── Trace ────────────────────────────────────────────────────────────────────

fn run_trace_build(spec_arg: &str) -> anyhow::Result<()> {
    let (project, _spec_path, spec, _spec_filename) = load_spec_context(spec_arg)?;
    let root = project.root.clone();
    let db_path = root.join(".s5d/codebase.db");

    if !db_path.exists() {
        anyhow::bail!("codebase index not found — run `s5d codebase index .` first");
    }

    let index = s5d::CodebaseIndex::open(&db_path)?;
    let links = s5d::build_trace(&index, &spec, &root)?;

    println!("  trace links built: {}", links.len());

    let mut by_source: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for link in &links {
        *by_source.entry(&link.source).or_insert(0) += 1;
    }
    for (source, count) in &by_source {
        println!("    {}: {}", source, count);
    }

    Ok(())
}

fn run_trace_check(spec_arg: &str) -> anyhow::Result<()> {
    let (project, _spec_path, spec, _spec_filename) = load_spec_context(spec_arg)?;
    let db_path = project.root.clone().join(".s5d/codebase.db");

    if !db_path.exists() {
        anyhow::bail!("codebase index not found — run `s5d codebase index .` first");
    }

    let index = s5d::CodebaseIndex::open(&db_path)?;
    let report = s5d::check_trace(&index, &spec)?;

    if report.matched.is_empty() && report.unmatched_spec.is_empty() {
        println!("  no trace data — run `s5d trace build` first");
        return Ok(());
    }

    println!("  MATCHED ({}):", report.matched.len());
    for link in &report.matched {
        println!(
            "    {} {} → {}:{} ({}  conf={:.2})",
            link.artifact_kind,
            link.artifact_id,
            link.file_path,
            link.line_start,
            link.source,
            link.confidence
        );
    }

    if !report.unmatched_spec.is_empty() {
        println!("\n  UNMATCHED ({}):", report.unmatched_spec.len());
        for (kind, id) in &report.unmatched_spec {
            println!("    {} {} — no code binding found", kind, id);
        }
    }

    let total = report.matched.len() + report.unmatched_spec.len();
    if total > 0 {
        let coverage = report.matched.len() as f64 / total as f64 * 100.0;
        println!(
            "\n  coverage: {:.0}% ({}/{})",
            coverage,
            report.matched.len(),
            total
        );
    }

    Ok(())
}

fn run_trace_report(spec_filter: Option<&str>) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let project = s5d::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;
    let db_path = project.root.clone().join(".s5d/codebase.db");

    if !db_path.exists() {
        anyhow::bail!("codebase index not found — run `s5d codebase index .` first");
    }

    let index = s5d::CodebaseIndex::open(&db_path)?;

    if let Some(spec_id) = spec_filter {
        let links = index.get_trace_links_for_spec(spec_id)?;
        if links.is_empty() {
            println!("  no trace links for spec '{}'", spec_id);
        } else {
            println!("  trace links for '{}':", spec_id);
            for link in &links {
                println!(
                    "    {} {} → {}:{} [{}  {:.2}]",
                    link.artifact_kind,
                    link.artifact_id,
                    link.file_path,
                    link.line_start,
                    link.source,
                    link.confidence
                );
            }
        }
    } else {
        println!("  use --spec <id> to filter by spec");
    }

    Ok(())
}

fn run_trace_blast_radius(spec_arg: &str) -> anyhow::Result<()> {
    let (project, _spec_path, spec, _spec_filename) = load_spec_context(spec_arg)?;
    let db_path = project.root.clone().join(".s5d/codebase.db");

    if !db_path.exists() {
        anyhow::bail!("codebase index not found — run `s5d codebase index .` first");
    }

    let index = s5d::CodebaseIndex::open(&db_path)?;
    let br = s5d::compute_blast_radius(&index, &spec)?;

    println!("  Blast Radius for '{}':", spec.id);
    println!();
    println!(
        "  Domains touched: {}",
        if br.domains_touched.is_empty() {
            "none".into()
        } else {
            br.domains_touched.join(", ")
        }
    );
    println!(
        "  Capabilities affected: {}",
        if br.capabilities_affected.is_empty() {
            "none".into()
        } else {
            br.capabilities_affected.join(", ")
        }
    );
    println!(
        "  Components affected: {}",
        if br.components_affected.is_empty() {
            "none".into()
        } else {
            br.components_affected.join(", ")
        }
    );
    println!("  Files: {}", br.files.len());
    println!("  Symbols: {}", br.symbols);

    if !br.cross_domain_edges.is_empty() {
        println!();
        println!("  Cross-domain edges:");
        for (from, to, archetype) in &br.cross_domain_edges {
            println!("    {} → {} [{}]", from, to, archetype);
        }
    }

    if let Some(ref wlnk) = br.weakest_link {
        println!();
        let kind_str = match &wlnk.kind {
            s5d::WeakestLinkKind::UnmatchedArtifact => "UNMATCHED — no code binding".into(),
            s5d::WeakestLinkKind::LowConfidenceMatch(c) => format!("LOW CONFIDENCE — {:.2}", c),
            s5d::WeakestLinkKind::CrossDomainWithoutTrace => {
                "CROSS-DOMAIN — missing trace on one side".into()
            }
        };
        println!(
            "  ⚠ Weakest link: {} {} — {}",
            wlnk.artifact_kind, wlnk.artifact_id, kind_str
        );
    }

    Ok(())
}

// ── Health check & metrics ───────────────────────────────────────────────────

fn run_check(spec_arg: &str, threshold: u8, compare: bool, format: &str) -> anyhow::Result<()> {
    let (project, _spec_path, spec, spec_filename) = load_spec_context(spec_arg)?;

    let report = s5d::compute_health_report(&spec);

    // Save snapshot for future comparisons
    let spec_id = spec_filename.replace(".s5d.yaml", "");
    let _ = s5d::save_snapshot(&project.s5d_dir(), &spec_id, &report);

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "  {} {} health score: {}",
            if report.aggregate_score >= threshold {
                "ok".green()
            } else {
                "FAIL".red()
            },
            spec_arg,
            format_score(report.aggregate_score)
        );

        if !report.cycles.is_empty() {
            println!("  Cycles:");
            for scc in &report.cycles {
                println!("    {} {}", "●".red(), scc.join(" → "));
            }
        }

        if !report.violations.is_empty() {
            println!("  Violations:");
            for v in &report.violations {
                println!("    {} [{}] {} (−{})", "●".yellow(), v.kind, v.message, v.penalty);
            }
        }

        println!();
        println!(
            "  {:<40} {:>4} {:>4} {:>6} {:>6}",
            "Domain", "Ca", "Ce", "I", "Score"
        );
        println!("  {}", "─".repeat(64));
        for dm in &report.domain_metrics {
            println!(
                "  {:<40} {:>4} {:>4} {:>6.2} {:>6}",
                dm.domain_id,
                dm.ca,
                dm.ce,
                dm.instability,
                format_score(dm.health_score)
            );
        }
    }

    if compare {
        match s5d::load_snapshot(&project.s5d_dir(), &spec_id)? {
            Some(baseline) => {
                let deg = s5d::detect_degradation(&report, &baseline);
                println!();
                println!(
                    "  Degradation: {} → {} ({}{})",
                    baseline.aggregate_score,
                    report.aggregate_score,
                    if deg.delta >= 0 { "+" } else { "" },
                    deg.delta
                );
                println!("  Status: {}", format_status(&deg.status));
                for dd in &deg.domain_deltas {
                    if dd.delta != 0 {
                        println!(
                            "    {} {}: {} → {} ({})",
                            status_icon(&dd.status),
                            dd.domain_id,
                            dd.score_before,
                            dd.score_after,
                            format_delta(dd.delta)
                        );
                    }
                }
            }
            None => {
                println!();
                println!("  {} no baseline found, snapshot saved for next comparison", "info:".cyan());
            }
        }
    }

    if report.aggregate_score < threshold {
        std::process::exit(1);
    }

    Ok(())
}

fn run_metrics(spec_arg: &str, format: &str) -> anyhow::Result<()> {
    let (_project, _spec_path, spec, _spec_filename) = load_spec_context(spec_arg)?;

    let report = s5d::compute_health_report(&spec);

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&report.domain_metrics)?);
    } else {
        println!(
            "  {:<40} {:>4} {:>4} {:>6} {:>6} Violations",
            "Domain", "Ca", "Ce", "I", "Score"
        );
        println!("  {}", "─".repeat(80));
        for dm in &report.domain_metrics {
            let violations_str = if dm.violations.is_empty() {
                "—".into()
            } else {
                dm.violations.join(", ")
            };
            println!(
                "  {:<40} {:>4} {:>4} {:>6.2} {:>6} {}",
                dm.domain_id,
                dm.ca,
                dm.ce,
                dm.instability,
                format_score(dm.health_score),
                violations_str
            );
        }
        println!();
        println!("  Aggregate health score: {}", format_score(report.aggregate_score));
    }

    Ok(())
}

fn format_score(score: u8) -> colored::ColoredString {
    use colored::Colorize;
    let s = score.to_string();
    if score >= 80 {
        s.green()
    } else if score >= 60 {
        s.yellow()
    } else {
        s.red()
    }
}

fn format_status(status: &str) -> colored::ColoredString {
    use colored::Colorize;
    match status {
        "improved" => status.green(),
        "stable" => status.normal(),
        "degraded" => status.yellow(),
        "critical" => status.red(),
        _ => status.normal(),
    }
}

fn status_icon(status: &str) -> &str {
    match status {
        "improved" => "↑",
        "stable" => "=",
        "degraded" => "↓",
        "critical" => "⚠",
        "removed" => "✕",
        _ => "?",
    }
}

fn format_delta(delta: i16) -> String {
    if delta >= 0 {
        format!("+{delta}")
    } else {
        format!("{delta}")
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_codebase_project;

    #[test]
    fn resolve_codebase_project_finds_parent_s5d() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".s5d")).unwrap();
        let nested = root.join("apps").join("svc");
        std::fs::create_dir_all(&nested).unwrap();

        let (project_root, db_path) = resolve_codebase_project(nested.to_str().unwrap()).unwrap();
        let expected_root = root.canonicalize().unwrap();

        assert_eq!(project_root, expected_root);
        assert_eq!(db_path, expected_root.join(".s5d").join("codebase.db"));
    }

    #[test]
    fn resolve_codebase_project_fails_without_s5d() {
        let dir = tempfile::TempDir::new().unwrap();
        let nested = dir.path().join("apps").join("svc");
        std::fs::create_dir_all(&nested).unwrap();

        let err = resolve_codebase_project(nested.to_str().unwrap()).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("no .s5d/ found for"),
            "unexpected error message: {}",
            msg
        );
    }
}
