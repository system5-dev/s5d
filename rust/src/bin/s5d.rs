use clap::{Args, Parser, Subcommand};
use colored::Colorize;

#[path = "s5d/cmd_codebase.rs"]
mod cmd_codebase;

#[path = "s5d/cmd_harness.rs"]
mod cmd_harness;

#[path = "s5d/cmd_provision.rs"]
mod cmd_provision;

#[path = "s5d/cmd_skill.rs"]
mod cmd_skill;

#[path = "s5d/cmd_ci.rs"]
mod cmd_ci;

#[derive(Parser)]
#[command(
    name = "s5d",
    version,
    about = "S5D — control plane for agentic repo changes"
)]
struct Cli {
    #[command(subcommand)]
    command: S5dCommand,
}

// ── Core commands — the critical path ────────────────────────────────────────
//
// desired state → evidence → transition → verify → restore
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
    #[command(hide = true)]
    Note {
        /// Note text (used as title and rationale)
        text: Vec<String>,
        /// Product name
        #[arg(long)]
        product: Option<String>,
    },
    /// Decision authoring: hypotheses, evidence, and confirmed choices
    Decision {
        #[command(subcommand)]
        command: DecisionCommand,
    },
    /// Verification gates: schema, graph, architecture, and configured checks
    Verify {
        #[command(subcommand)]
        command: VerifyCommand,
    },
    /// State transitions: preview, approval, import, drift, rollback, and outcome evidence
    State {
        #[command(subcommand)]
        command: ApplyCommand,
    },
    /// Apply lifecycle: preview, approval, import, drift, rollback, and reflection
    #[command(hide = true)]
    Apply {
        #[command(subcommand)]
        command: ApplyCommand,
    },
    /// Add a hypothesis to a decision spec
    #[command(hide = true)]
    AddHypothesis(AddHypothesisArgs),
    /// Add evidence to a hypothesis in a decision spec
    #[command(hide = true)]
    AddEvidence(AddEvidenceArgs),
    /// Record a decision in a decision spec
    #[command(hide = true)]
    Decide(DecideArgs),
    /// Validate a spec file
    #[command(hide = true)]
    Validate(SpecArg),
    /// Graph/relation validation — DFS cycle detection
    #[command(hide = true)]
    GraphCheck(SpecArg),
    /// Architecture linter — spec shape, graph rules, component paths, and source dependencies
    #[command(hide = true)]
    Check(CheckArgs),
    /// Dry-run import diff
    #[command(hide = true)]
    Preview(SpecArg),
    /// Record approval binding spec_sha256 + diff_sha256
    #[command(hide = true)]
    Approve(ApproveArgs),
    /// Execute configured gate commands
    #[command(hide = true)]
    RunGates(SpecArg),
    /// Transactional import (apply spec to alias table + ledger)
    #[command(hide = true)]
    Import(ImportArgs),
    /// Show status of all specs
    Status,
    /// Show spec details — decision trace, hypothesis tree, or feature summary
    Show {
        /// Path to .s5d.yaml file
        spec: String,
    },
    /// Trace a source path to S5D specs, components, capabilities, and decisions
    Trace {
        /// Source path inside the project
        path: String,
    },
    /// Search specs and decisions by keyword
    #[command(hide = true)]
    Search {
        /// Search query
        query: String,
    },
    /// Compare live state vs last applied fingerprint
    #[command(hide = true)]
    DriftCheck(OptionalSpecArg),
    /// Agent run control: active work, approved engines, task packages, and harnesses
    Run {
        #[command(subcommand)]
        command: RunCommand,
    },
    /// Phase lifecycle for workflow-driven execution
    #[command(hide = true)]
    Phase {
        #[command(subcommand)]
        command: PhaseCommand,
    },
    /// Execute a bounded loop inside an approved work state
    #[command(hide = true)]
    Execute {
        #[command(subcommand)]
        command: ExecuteCommand,
    },
    /// Operational harness around isolated worktrees and S5D run commands
    #[command(hide = true)]
    Harness {
        #[command(subcommand)]
        command: HarnessCommand,
    },
    /// Re-import to fix drift (desired-state restore, bypasses diff_sha256)
    #[command(hide = true)]
    Reconcile(OptionalSpecArg),
    /// Reverse last import for a spec
    #[command(hide = true)]
    Rollback(SpecArg),
    /// Record reflection for a spec (OPERATE stage) — closes lifecycle with production evidence
    #[command(hide = true)]
    Reflect(ReflectArgs),
    /// Classify a request into tier, mode, and entry point
    #[command(hide = true)]
    Route {
        /// Request description to classify
        description: Vec<String>,
        /// Output format: text or json
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Index management
    #[command(hide = true)]
    Index {
        #[command(subcommand)]
        command: IndexCommand,
    },
    /// Codebase ownership and coverage snapshot
    #[command(hide = true)]
    Codebase {
        #[command(subcommand)]
        command: CodebaseCommand,
    },
    /// Stack-agnostic repository discovery index and evidence graph
    #[command(hide = true)]
    Discover {
        #[command(subcommand)]
        command: DiscoverCommand,
    },
    /// Git hook entrypoints implemented in Rust
    #[command(hide = true)]
    Hook {
        #[command(subcommand)]
        command: HookCommand,
    },
    /// PreToolUse enforcement gate (S5D-Spec: feat.s5d.pretool-enforcement)
    #[command(hide = true)]
    Gate {
        #[command(subcommand)]
        command: GateCommand,
    },
    /// Check for and apply S5D binary/skill updates
    #[command(hide = true)]
    Update {
        #[command(subcommand)]
        command: UpdateCommand,
    },
    /// Administrative setup and maintenance commands
    Admin {
        #[command(subcommand)]
        command: AdminCommand,
    },
    /// Seed alias table from bootstrap manifest
    #[command(hide = true)]
    Bootstrap {
        /// Path to bootstrap manifest YAML
        manifest: String,
    },
    /// Print environment fingerprint (tool versions hash)
    #[command(hide = true)]
    Cg,
    /// DDD modeling and scaling anti-pattern analyzers
    Skill {
        #[command(subcommand)]
        command: SkillCommand,
    },
    /// Generated CI enforcement (GitHub Actions / GitLab CI)
    Ci {
        #[command(subcommand)]
        command: CiCommand,
    },
    /// Shape an intake kernel: route it and check readiness
    Shape(ShapeArgs),
    /// Review helpers (adversarial review scaffold)
    Review {
        #[command(subcommand)]
        command: ReviewCommand,
    },
    /// Planning helpers (story-shaped workflow phases)
    Plan {
        #[command(subcommand)]
        command: PlanCommand,
    },
    /// Start stdio MCP server (for Claude Code integration)
    #[command(hide = true)]
    Mcp,
    /// Start the LSP server over stdio (live diagnostics for .s5d.yaml specs)
    Lsp,
    /// Register s5d MCP server for supported assistants.
    /// Default: project-level for Claude (.mcp.json). Use flags to target
    /// other assistants or --global to install at user level.
    #[command(hide = true)]
    Install(InstallArgs),
}

#[derive(Subcommand)]
enum DecisionCommand {
    /// Add a hypothesis to a decision spec
    AddHypothesis(AddHypothesisArgs),
    /// Add evidence to a hypothesis in a decision spec
    AddEvidence(AddEvidenceArgs),
    /// Set the problem-card acceptance criteria (required before deciding)
    SetAcceptance(SetAcceptanceArgs),
    /// Record a decision in a decision spec
    Decide(Box<DecideArgs>),
}

#[derive(Subcommand)]
enum VerifyCommand {
    /// Validate a spec file
    Validate(SpecArg),
    /// Graph/relation validation — DFS cycle detection
    GraphCheck(SpecArg),
    /// Architecture linter — spec shape, graph rules, component paths, and source dependencies
    Check(CheckArgs),
    /// Execute configured gate commands
    RunGates(SpecArg),
}

#[derive(Subcommand)]
enum ApplyCommand {
    /// Dry-run import diff
    Preview(SpecArg),
    /// Record approval binding spec_sha256 + diff_sha256
    Approve(ApproveArgs),
    /// Transactional import (apply spec to alias table + ledger)
    Import(ImportArgs),
    /// Compare live state vs last applied fingerprint
    DriftCheck(OptionalSpecArg),
    /// Re-import to fix drift (desired-state restore, bypasses diff_sha256)
    Reconcile(OptionalSpecArg),
    /// Reverse last import for a spec
    Rollback(SpecArg),
    /// Record reflection for a spec (OPERATE stage) — closes lifecycle with production evidence
    Reflect(Box<ReflectArgs>),
}

#[derive(Subcommand)]
enum RunCommand {
    /// List executable work states and current active state
    List {
        /// Path to .s5d.yaml file
        spec: String,
    },
    /// Mark an executable work state as active
    Start {
        /// Path to .s5d.yaml file
        spec: String,
        /// Work state ID
        #[arg(long)]
        id: String,
    },
    /// Human acceptance for an executable work state
    Accept {
        /// Path to .s5d.yaml file
        spec: String,
        /// Work state ID
        #[arg(long)]
        id: String,
        /// Reviewer who accepts the resulting evidence
        #[arg(long)]
        reviewer: String,
    },
    /// Run an approved external engine for the active work state
    Exec {
        /// Path to .s5d.yaml file
        spec: String,
        /// Work state ID
        #[arg(long)]
        id: String,
        /// Approved engine name from .s5d/config.yaml
        #[arg(long)]
        engine: String,
    },
    /// Emit a bounded task package for an active work state
    #[command(alias = "loop")]
    Task(ExecuteLoopArgs),
    /// Operational harness around isolated worktrees and S5D run commands
    Harness {
        #[command(subcommand)]
        command: HarnessCommand,
    },
}

#[derive(Subcommand)]
enum AdminCommand {
    /// Register s5d MCP server for supported assistants
    Install(InstallArgs),
    /// Check for and apply S5D binary/skill updates
    Update {
        #[command(subcommand)]
        command: UpdateCommand,
    },
}

// ── Skill subcommand tree ─────────────────────────────────────────────────────

#[derive(Subcommand)]
enum CiCommand {
    /// Generate a PR pipeline: install pinned s5d binary, run `s5d ci exec`
    Init {
        /// Generate GitHub Actions workflow (.github/workflows/s5d.yml) — default
        #[arg(long)]
        github: bool,
        /// Generate GitLab CI fragment (.s5d/ci/s5d.gitlab-ci.yml + include stub)
        #[arg(long)]
        gitlab: bool,
        /// Generate for all supported CI systems
        #[arg(long)]
        all: bool,
        /// Overwrite user-owned (marker-less) files
        #[arg(long)]
        force: bool,
    },
    /// Report stale or unmanaged generated CI config
    Check,
    /// Run built-in checks (validate, architecture, drift) — called by generated pipelines
    Exec,
}

#[derive(Args)]
struct ShapeArgs {
    /// Path to a kernel YAML file (shape-layer fields: why, capabilities,
    /// constraints, non_goals, success_signal, assumptions, open_questions,
    /// companions)
    kernel: String,
    /// Also scaffold a spec with the kernel embedded (feature id, e.g. feat.x.y)
    #[arg(long)]
    emit_spec: Option<String>,
    /// Tier for the emitted spec (lightweight|standard|high)
    #[arg(long, default_value = "standard")]
    tier: String,
    /// Product for the emitted spec
    #[arg(long)]
    product: Option<String>,
}

#[derive(Subcommand)]
enum ReviewCommand {
    /// Scaffold the 3-layer adversarial review report into .s5d/evidence/<spec>/
    Adversarial {
        /// Spec path or id
        spec: String,
    },
}

#[derive(Subcommand)]
enum PlanCommand {
    /// Append story-shaped workflow phases to a spec from a YAML list
    Stories {
        /// Spec path or id
        spec: String,
        /// YAML file with a list of stories ({id,title,scope,acceptance,rollback})
        #[arg(long)]
        from: String,
    },
}

#[derive(Subcommand)]
enum SkillCommand {
    /// DDD modeling analyzer
    Ddd {
        #[command(subcommand)]
        command: SkillDddCommand,
    },
    /// Scaling anti-pattern analyzer
    Scaling {
        #[command(subcommand)]
        command: SkillScalingCommand,
    },
    /// Flatten analyze JSON from stdin into markdown anomaly bullets
    Flatten {
        /// Section label (default: "findings")
        #[arg(long, default_value = "findings")]
        label: String,
        /// Minimum severity to include: info|low|medium|high|critical
        #[arg(long, default_value = "medium")]
        min_severity: String,
    },
}

#[derive(Subcommand)]
enum SkillDddCommand {
    /// Detect DDD-relevant structure in repo
    Detect {
        /// Root path to analyze (default: current dir)
        #[arg(long)]
        root: Option<String>,
    },
    /// Run DDD modeling checks
    Analyze {
        /// Root path to analyze (default: current dir)
        #[arg(long)]
        root: Option<String>,
        /// Flatten JSON output to markdown anomaly bullets
        #[arg(long)]
        flatten: bool,
        /// Minimum severity for flatten output: info|low|medium|high|critical
        #[arg(long, default_value = "medium")]
        min_severity: String,
    },
}

#[derive(Subcommand)]
enum SkillScalingCommand {
    /// Detect scaling-relevant dimensions in repo
    Detect {
        /// Root path to analyze (default: current dir)
        #[arg(long)]
        root: Option<String>,
    },
    /// Run scaling anti-pattern checks
    Analyze {
        /// Root path to analyze (default: current dir)
        #[arg(long)]
        root: Option<String>,
        /// Flatten JSON output to markdown anomaly bullets
        #[arg(long)]
        flatten: bool,
        /// Minimum severity for flatten output: info|low|medium|high|critical
        #[arg(long, default_value = "medium")]
        min_severity: String,
    },
}

#[derive(Args)]
struct SpecArg {
    /// Path to .s5d.yaml file
    spec: String,
}

#[derive(Args)]
struct OptionalSpecArg {
    /// Path to .s5d.yaml file (optional: operate on all if omitted)
    spec: Option<String>,
}

#[derive(Args)]
struct CheckArgs {
    /// Path to .s5d.yaml file
    spec: String,
    /// Output format: text or json
    #[arg(long, default_value = "text")]
    format: String,
}

#[derive(Args)]
struct ApproveArgs {
    /// Path to .s5d.yaml file
    spec: String,
    /// Reviewer name
    #[arg(long)]
    reviewer: String,
    /// Block approval if reviewer is not a domain owner
    #[arg(long)]
    require_owner: bool,
}

#[derive(Args)]
struct ImportArgs {
    /// Path to .s5d.yaml file
    spec: String,
    /// Who independently verified gates passed (trust separation)
    #[arg(long)]
    verified_by: String,
    /// Override methodological checks
    #[arg(long)]
    force: bool,
}

#[derive(Args)]
struct AddHypothesisArgs {
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
    /// FPF B.5.2:13.3 prompt — explicit question this hypothesis answers (cite of problem.signal)
    #[arg(long)]
    prompt: Option<String>,
    /// FPF B.5.2:13.3 next downstream move (deduction|probe|build|defer)
    #[arg(long)]
    next_move: Option<String>,
}

#[derive(Args)]
struct AddEvidenceArgs {
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
    /// FPF C.2:4.2 Δ-move kind (formalise|generalise|specialise|calibrate|validate|congrue). Required when verdict=refine.
    #[arg(long)]
    refine_kind: Option<String>,
    /// Provenance: which cluster skill produced this evidence (e.g. "security-scan").
    #[arg(long)]
    skill: Option<String>,
    /// Provenance: which assess agent produced this evidence (e.g. "security-scan-assess").
    #[arg(long)]
    agent: Option<String>,
}

#[derive(Args)]
struct SetAcceptanceArgs {
    /// Path to .s5d.yaml file
    spec: String,
    /// Acceptance criteria — how we'll know the problem is solved
    #[arg(long)]
    acceptance: String,
}

#[derive(Args)]
struct DecideArgs {
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
    /// FPF C.11 Decsn-CAL — DecisionSubject (who/what is the decision about)
    #[arg(long)]
    decision_subject: Option<String>,
    /// FPF C.11 — DecisionSubjectGranularity (system|component|module|line|...)
    #[arg(long)]
    decision_subject_granularity: Option<String>,
    /// FPF C.11 — evaluative surface (named axes + policy used to compare options)
    #[arg(long)]
    evaluative_surface: Option<String>,
    /// FPF C.11 — belief_state at decision time (what was assumed true)
    #[arg(long)]
    belief_state: Option<String>,
    /// FPF C.11 — outcome_model (what the decision predicts will happen)
    #[arg(long)]
    outcome_model: Option<String>,
    /// FPF C.18 NQD-CAL — Pareto-non-dominated alternatives (comma-separated hypothesis IDs)
    #[arg(long)]
    pareto_set: Option<String>,
    /// FPF C.11 — explicit choice rule (e.g. "lex-order(thinness>auditability)", "policy:minimize-coupling")
    #[arg(long)]
    choice_rule: Option<String>,
}

#[derive(Args)]
struct ReflectArgs {
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
}

#[derive(Args)]
struct InstallArgs {
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
enum DiscoverCommand {
    /// Rebuild .s5d/discovery/* from files and S5D specs
    Sync {
        /// Target path to scan. Defaults to project root.
        path: Option<String>,
        /// Output directory. Defaults to .s5d/discovery.
        #[arg(long, default_value = ".s5d/discovery")]
        out: String,
    },
    /// Check .s5d/discovery/* against current files and specs
    Check {
        /// Target path to scan. Defaults to project root.
        path: Option<String>,
        /// Output directory. Defaults to .s5d/discovery.
        #[arg(long, default_value = ".s5d/discovery")]
        out: String,
    },
}

#[derive(Subcommand)]
enum HookCommand {
    /// Run the S5D pre-commit check over staged specs and architecture-gated source
    PreCommit,
    /// PreToolUse(Edit|Write|MultiEdit) hook entrypoint.
    /// Reads Claude Code hook JSON from stdin, returns decision JSON to stdout.
    /// S5D-Spec: feat.s5d.pretool-enforcement
    #[command(hide = true)]
    PretoolEdit,
    /// UserPromptSubmit hook entrypoint (L1 advisory).
    /// Reads Claude Code hook JSON from stdin, classifies the prompt via
    /// `s5d::route`, and emits a reminder on stdout (which Claude injects
    /// into context) when the request looks non-trivial and no spec is
    /// in `.s5d/packages/` yet. Never blocks — non-zero noise budget.
    /// S5D-Spec: feat.s5d.pretool-enforcement
    #[command(hide = true)]
    UserPromptSubmit,
    /// PreToolUse(Bash) require-spec hook (L3 — final commit-time net).
    /// Reads PreToolUse JSON from stdin; if the command is `git commit`
    /// and the change is non-trivial without a spec reference, blocks.
    /// Pure Rust replacement for hooks/require-spec.sh.
    /// S5D-Spec: feat.s5d.pretool-enforcement
    #[command(hide = true)]
    RequireSpec,
    /// PreToolUse(Bash) staged-spec validation hook.
    /// On `git commit`, validates every staged .s5d.yaml file via
    /// `s5d validate`. Pure Rust replacement for hooks/pre-commit-validate.sh.
    /// S5D-Spec: feat.s5d.pretool-enforcement
    #[command(hide = true)]
    PreCommitValidate,
}

/// PreToolUse enforcement gate subcommands.
/// S5D-Spec: feat.s5d.pretool-enforcement
#[derive(Subcommand)]
enum GateCommand {
    /// Decide whether an Edit/Write/MultiEdit tool call should be allowed.
    /// Reads project state, returns JSON GateDecision on stdout.
    Edit {
        /// File path the tool would modify
        #[arg(long)]
        file: std::path::PathBuf,
        /// Estimated lines-of-code delta the edit will introduce (default 0 — caller may not know yet)
        #[arg(long, default_value = "0")]
        loc_delta: usize,
        /// Claude Code session id (from hook stdin) — keys per-session counter
        #[arg(long)]
        session_id: Option<String>,
        /// Bypass via env var alternative — also honored: S5D_BYPASS=1
        #[arg(long)]
        bypass: bool,
    },
    /// Reset the session counter for a given session id (or all if --all).
    Reset {
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long)]
        all: bool,
    },
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
        /// Re-verify auto-update guards under the update lock; skip gracefully if unsafe
        #[arg(long, hide = true)]
        if_safe: bool,
    },
    /// Session-start self-update: apply in the background when safe, else prompt
    Auto,
}

#[derive(Subcommand)]
enum PhaseCommand {
    /// List executable work states and current active state
    List {
        /// Path to .s5d.yaml file
        spec: String,
    },
    /// Mark an executable work state as active
    Start {
        /// Path to .s5d.yaml file
        spec: String,
        /// Work state ID
        #[arg(long)]
        id: String,
    },
    /// Human acceptance for an executable work state
    Accept {
        /// Path to .s5d.yaml file
        spec: String,
        /// Work state ID
        #[arg(long)]
        id: String,
        /// Reviewer who accepts the run evidence
        #[arg(long)]
        reviewer: String,
    },
    /// Run a configured external engine for an active work state
    Run {
        /// Path to .s5d.yaml file
        spec: String,
        /// Work state ID
        #[arg(long)]
        id: String,
        /// Approved engine name from .s5d/config.yaml
        #[arg(long)]
        engine: String,
    },
    /// Emit a bounded task package for a work state
    #[command(alias = "execute")]
    Loop(ExecuteLoopArgs),
    /// Operational harness around isolated worktrees and S5D run commands
    Harness {
        #[command(subcommand)]
        command: HarnessCommand,
    },
}

#[derive(Subcommand)]
enum ExecuteCommand {
    /// Emit a bounded task package for a work state
    Loop(ExecuteLoopArgs),
}

#[derive(Args)]
struct ExecuteLoopArgs {
    /// Path to .s5d.yaml file
    spec: String,
    /// Work state ID
    #[arg(long)]
    phase: String,
    /// Execution engine name
    #[arg(long, default_value = "ralph")]
    engine: String,
    /// Optional Ralph run mode (default inferred from work state)
    #[arg(long)]
    mode: Option<String>,
}

#[derive(Subcommand)]
enum HarnessCommand {
    /// Create an isolated worktree and start an approved work state there
    Start {
        /// Path to .s5d.yaml file
        spec: String,
        /// Work state ID
        #[arg(long)]
        phase: String,
        /// Harness run ID
        #[arg(long)]
        name: String,
        /// Git branch to create for the worktree
        #[arg(long)]
        branch: Option<String>,
        /// Worktree path. Defaults next to the repository root.
        #[arg(long)]
        worktree: Option<String>,
        /// Allow start when source worktree is not clean
        #[arg(long)]
        force: bool,
    },
    /// Show harness status, last event, heartbeat, and current command
    Status {
        /// Harness run ID
        name: String,
        /// Seconds after which heartbeat is considered stale
        #[arg(long, default_value_t = 300)]
        stale_after_s: i64,
    },
    /// Run a command inside the harness worktree with timeout and journal capture
    Exec {
        /// Harness run ID
        name: String,
        /// Timeout in seconds
        #[arg(long, default_value_t = 3600)]
        timeout_s: u64,
        /// Command and args. Use `--` before the command.
        #[arg(required = true, last = true)]
        command: Vec<String>,
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
        } => cmd_provision::run_init(claude, cursor, codex, gemini, all),
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
        S5dCommand::Decision { command } => run_decision_command(command),
        S5dCommand::Verify { command } => run_verify_command(command),
        S5dCommand::State { command } => run_apply_command(command),
        S5dCommand::Apply { command } => run_apply_command(command),
        S5dCommand::Validate(args) => run_validate(&args.spec),
        S5dCommand::Status => run_status(),
        S5dCommand::Cg => run_cg(),
        S5dCommand::Preview(args) => run_preview(&args.spec),
        S5dCommand::Approve(args) => run_approve(&args.spec, &args.reviewer, args.require_owner),
        S5dCommand::RunGates(args) => run_gates(&args.spec),
        S5dCommand::Import(args) => run_import(&args.spec, &args.verified_by, args.force),
        S5dCommand::Rollback(args) => run_rollback(&args.spec),
        S5dCommand::Index { command } => match command {
            IndexCommand::Check => run_index_check(),
            IndexCommand::Sync => run_index_sync(),
        },
        S5dCommand::Codebase { command } => match command {
            CodebaseCommand::Check => cmd_codebase::run_codebase_check(),
            CodebaseCommand::Sync => cmd_codebase::run_codebase_sync(),
        },
        S5dCommand::Discover { command } => match command {
            DiscoverCommand::Sync { path, out } => {
                cmd_codebase::run_discover_sync(path.as_deref(), std::path::Path::new(&out))
            }
            DiscoverCommand::Check { path, out } => {
                cmd_codebase::run_discover_check(path.as_deref(), std::path::Path::new(&out))
            }
        },
        S5dCommand::Hook { command } => match command {
            HookCommand::PreCommit => run_hook_pre_commit(),
            HookCommand::PretoolEdit => run_hook_pretool_edit(),
            HookCommand::UserPromptSubmit => run_hook_user_prompt_submit(),
            HookCommand::RequireSpec => run_hook_require_spec(),
            HookCommand::PreCommitValidate => run_hook_pre_commit_validate(),
        },
        S5dCommand::Update { command } => match command {
            UpdateCommand::Check { hook, json } => cmd_provision::run_update_check(hook, json),
            UpdateCommand::Apply { dry_run, if_safe } => {
                cmd_provision::run_update_apply(dry_run, if_safe)
            }
            UpdateCommand::Auto => cmd_provision::run_update_auto(),
        },
        S5dCommand::Admin { command } => run_admin_command(command),
        S5dCommand::Bootstrap { manifest } => run_bootstrap(&manifest),
        S5dCommand::GraphCheck(args) => run_graph_check(&args.spec),
        S5dCommand::Check(args) => run_check(&args.spec, &args.format),
        S5dCommand::DriftCheck(args) => run_drift_check(args.spec.as_deref()),
        S5dCommand::Run { command } => run_run_command(command),
        S5dCommand::Phase { command } => match command {
            PhaseCommand::List { spec } => run_phase_list(&spec),
            PhaseCommand::Start { spec, id } => run_phase_start(&spec, &id),
            PhaseCommand::Accept { spec, id, reviewer } => run_phase_accept(&spec, &id, &reviewer),
            PhaseCommand::Run { spec, id, engine } => run_phase_run(&spec, &id, &engine),
            PhaseCommand::Loop(args) => {
                run_execute_loop(&args.spec, &args.phase, &args.engine, args.mode.as_deref())
            }
            PhaseCommand::Harness { command } => run_harness_command(command),
        },
        S5dCommand::Execute { command } => match command {
            ExecuteCommand::Loop(args) => {
                run_execute_loop(&args.spec, &args.phase, &args.engine, args.mode.as_deref())
            }
        },
        S5dCommand::Harness { command } => run_harness_command(command),
        S5dCommand::Reconcile(args) => run_reconcile(args.spec.as_deref()),
        S5dCommand::AddHypothesis(args) => run_add_hypothesis_command(args),
        S5dCommand::AddEvidence(args) => run_add_evidence_command(args),
        S5dCommand::Decide(args) => run_decide_command(args),
        S5dCommand::Show { spec } => run_show(&spec),
        S5dCommand::Trace { path } => run_trace(&path),
        S5dCommand::Route {
            description,
            format,
        } => run_route(&description.join(" "), &format),
        S5dCommand::Search { query } => run_search(&query),
        S5dCommand::Reflect(args) => run_reflect_command(args),
        S5dCommand::Mcp => s5d::mcp::run_mcp_server(),
        S5dCommand::Lsp => s5d::lsp::run(),
        S5dCommand::Install(args) => run_install_command(args),
        S5dCommand::Gate { command } => run_gate(command),
        S5dCommand::Skill { command } => run_skill_command(command),
        S5dCommand::Ci { command } => match command {
            CiCommand::Init {
                github,
                gitlab,
                all,
                force,
            } => cmd_ci::run_ci_init(github, gitlab, all, force),
            CiCommand::Check => cmd_ci::run_ci_check(),
            CiCommand::Exec => cmd_ci::run_ci_exec(),
        },
        S5dCommand::Shape(args) => run_shape(&args),
        S5dCommand::Review { command } => match command {
            ReviewCommand::Adversarial { spec } => run_review_adversarial(&spec),
        },
        S5dCommand::Plan { command } => match command {
            PlanCommand::Stories { spec, from } => run_plan_stories(&spec, &from),
        },
    }
}

// ── Shape / Review / Plan (decision.s5d.bmad-native-runtime) ─────────────────

fn run_shape(args: &ShapeArgs) -> anyhow::Result<()> {
    let yaml = std::fs::read_to_string(&args.kernel)
        .map_err(|e| anyhow::anyhow!("cannot read kernel file {}: {}", args.kernel, e))?;
    let kernel = s5d::parse_kernel(&yaml)?;
    let outcome = s5d::shape_kernel(&kernel);

    println!("{}", outcome.route);
    if outcome.readiness_errors.is_empty() {
        println!("{} Readiness: ready", "ok".green());
    } else {
        println!("Readiness: not ready");
        for e in &outcome.readiness_errors {
            eprintln!("  {} {}", "error:".red(), e);
        }
    }

    if let Some(ref id) = args.emit_spec {
        if !outcome.readiness_errors.is_empty() {
            anyhow::bail!("kernel is not ready — fix readiness errors before emitting a spec");
        }
        run_new(id, &args.tier, args.product.as_deref(), None, None, None)?;
        // Embed the kernel into the freshly scaffolded spec.
        let cwd = std::env::current_dir()?;
        let project = s5d::S5dProject::find(&cwd)
            .ok_or_else(|| anyhow::anyhow!("no .s5d/ found (run `s5d init` first)"))?;
        let today = chrono::Utc::now().format("%Y%m%d");
        let spec_filename = format!("{}__{}.s5d.yaml", id, today);
        let spec_path = project.s5d_dir().join("packages").join(&spec_filename);
        let mut spec: s5d::Spec = serde_yaml::from_str(&std::fs::read_to_string(&spec_path)?)?;
        spec.intent_kernel = Some(kernel);
        save_spec_yaml(&spec_path, &spec)?;
        // The record was generated before the kernel embed — refresh its sha
        // so the draft record matches the file on disk.
        let sha = s5d::S5dProject::file_sha256(&spec_path)?;
        let record = s5d::generate_record(&spec_filename, &sha);
        project.save_record(&spec_filename, &record)?;
        println!(
            "  {} Embedded intent_kernel into {}",
            "ok".green(),
            spec_path.display()
        );
    }
    Ok(())
}

fn run_review_adversarial(spec_arg: &str) -> anyhow::Result<()> {
    let (project, _spec_path, spec, _spec_filename) = load_spec_context(spec_arg)?;
    let (path, binding) = s5d::scaffold_adversarial_review(&project, &spec)?;
    println!(
        "{} Adversarial review scaffold: {}",
        "ok".green(),
        path.display()
    );
    println!("  Binding: {}", binding);
    Ok(())
}

fn run_plan_stories(spec_arg: &str, from: &str) -> anyhow::Result<()> {
    let (_project, spec_path, mut spec, _spec_filename) = load_spec_context(spec_arg)?;
    let yaml = std::fs::read_to_string(from)
        .map_err(|e| anyhow::anyhow!("cannot read stories file {}: {}", from, e))?;
    let stories: Vec<s5d::StoryInput> = serde_yaml::from_str(&yaml)
        .map_err(|e| anyhow::anyhow!("stories file does not parse as a story list: {}", e))?;
    let added = s5d::apply_stories(&mut spec, stories)?;

    let errors = s5d::validate_spec(&spec);
    if !errors.is_empty() {
        anyhow::bail!(
            "spec would not validate after adding stories:\n  {}",
            errors.join("\n  ")
        );
    }
    save_spec_yaml(&spec_path, &spec)?;
    println!(
        "{} Added {} story phase(s) to {}: {}",
        "ok".green(),
        added.len(),
        spec.id,
        added.join(", ")
    );
    println!("  note: editing a spec changes its sha — re-run preview/approve if it was already approved");
    Ok(())
}

fn run_decision_command(command: DecisionCommand) -> anyhow::Result<()> {
    match command {
        DecisionCommand::AddHypothesis(args) => run_add_hypothesis_command(args),
        DecisionCommand::AddEvidence(args) => run_add_evidence_command(args),
        DecisionCommand::SetAcceptance(args) => run_set_acceptance(&args.spec, &args.acceptance),
        DecisionCommand::Decide(args) => run_decide_command(*args),
    }
}

fn run_skill_command(command: SkillCommand) -> anyhow::Result<()> {
    let resolve_root = |root: Option<String>| -> anyhow::Result<std::path::PathBuf> {
        match root {
            Some(r) => Ok(std::path::PathBuf::from(r)),
            None => Ok(std::env::current_dir()?),
        }
    };
    match command {
        SkillCommand::Ddd { command } => match command {
            SkillDddCommand::Detect { root } => {
                cmd_skill::run_skill_ddd_detect(&resolve_root(root)?)
            }
            SkillDddCommand::Analyze {
                root,
                flatten,
                min_severity,
            } => cmd_skill::run_skill_ddd_analyze(
                &resolve_root(root)?,
                flatten,
                cmd_skill::parse_severity(&min_severity),
            ),
        },
        SkillCommand::Scaling { command } => match command {
            SkillScalingCommand::Detect { root } => {
                cmd_skill::run_skill_scaling_detect(&resolve_root(root)?)
            }
            SkillScalingCommand::Analyze {
                root,
                flatten,
                min_severity,
            } => cmd_skill::run_skill_scaling_analyze(
                &resolve_root(root)?,
                flatten,
                cmd_skill::parse_severity(&min_severity),
            ),
        },
        SkillCommand::Flatten {
            label,
            min_severity,
        } => cmd_skill::run_skill_flatten(&label, cmd_skill::parse_severity(&min_severity)),
    }
}

fn run_verify_command(command: VerifyCommand) -> anyhow::Result<()> {
    match command {
        VerifyCommand::Validate(args) => run_validate(&args.spec),
        VerifyCommand::GraphCheck(args) => run_graph_check(&args.spec),
        VerifyCommand::Check(args) => run_check(&args.spec, &args.format),
        VerifyCommand::RunGates(args) => run_gates(&args.spec),
    }
}

fn run_apply_command(command: ApplyCommand) -> anyhow::Result<()> {
    match command {
        ApplyCommand::Preview(args) => run_preview(&args.spec),
        ApplyCommand::Approve(args) => run_approve(&args.spec, &args.reviewer, args.require_owner),
        ApplyCommand::Import(args) => run_import(&args.spec, &args.verified_by, args.force),
        ApplyCommand::DriftCheck(args) => run_drift_check(args.spec.as_deref()),
        ApplyCommand::Reconcile(args) => run_reconcile(args.spec.as_deref()),
        ApplyCommand::Rollback(args) => run_rollback(&args.spec),
        ApplyCommand::Reflect(args) => run_reflect_command(*args),
    }
}

fn run_run_command(command: RunCommand) -> anyhow::Result<()> {
    match command {
        RunCommand::List { spec } => run_phase_list(&spec),
        RunCommand::Start { spec, id } => run_phase_start(&spec, &id),
        RunCommand::Accept { spec, id, reviewer } => run_phase_accept(&spec, &id, &reviewer),
        RunCommand::Exec { spec, id, engine } => run_phase_run(&spec, &id, &engine),
        RunCommand::Task(args) => {
            run_execute_loop(&args.spec, &args.phase, &args.engine, args.mode.as_deref())
        }
        RunCommand::Harness { command } => run_harness_command(command),
    }
}

fn run_admin_command(command: AdminCommand) -> anyhow::Result<()> {
    match command {
        AdminCommand::Install(args) => run_install_command(args),
        AdminCommand::Update { command } => run_update_command(command),
    }
}

fn run_update_command(command: UpdateCommand) -> anyhow::Result<()> {
    match command {
        UpdateCommand::Check { hook, json } => cmd_provision::run_update_check(hook, json),
        UpdateCommand::Apply { dry_run, if_safe } => {
            cmd_provision::run_update_apply(dry_run, if_safe)
        }
        UpdateCommand::Auto => cmd_provision::run_update_auto(),
    }
}

fn run_harness_command(command: HarnessCommand) -> anyhow::Result<()> {
    match command {
        HarnessCommand::Start {
            spec,
            phase,
            name,
            branch,
            worktree,
            force,
        } => cmd_harness::run_harness_start(
            &spec,
            &phase,
            &name,
            branch.as_deref(),
            worktree.as_deref(),
            force,
        ),
        HarnessCommand::Status {
            name,
            stale_after_s,
        } => cmd_harness::run_harness_status(&name, stale_after_s),
        HarnessCommand::Exec {
            name,
            timeout_s,
            command,
        } => cmd_harness::run_harness_exec(&name, timeout_s, &command),
    }
}

fn run_add_hypothesis_command(args: AddHypothesisArgs) -> anyhow::Result<()> {
    run_add_hypothesis(
        &args.spec,
        args.id.as_deref(),
        &args.title,
        &args.content,
        &args.scope,
        &args.kind,
        args.rationale.as_deref(),
        args.prompt.as_deref(),
        args.next_move.as_deref(),
    )
}

fn run_add_evidence_command(args: AddEvidenceArgs) -> anyhow::Result<()> {
    run_add_evidence(
        &args.spec,
        &args.hypothesis_id,
        &args.evidence_type,
        &args.content,
        &args.verdict,
        args.carrier_ref.as_deref(),
        args.formality,
        args.claim_scope,
        args.reliability,
        args.refine_kind.as_deref(),
        args.skill,
        args.agent,
    )
}

fn run_set_acceptance(spec_path: &str, acceptance: &str) -> anyhow::Result<()> {
    if acceptance.trim().is_empty() {
        anyhow::bail!("--acceptance cannot be empty");
    }
    let (path, mut spec) = load_spec_yaml(spec_path)?;
    let card = spec
        .problem
        .as_mut()
        .and_then(|p| p.as_card_mut())
        .ok_or_else(|| anyhow::anyhow!("spec has no structured problem card — cannot set acceptance"))?;
    card.acceptance = Some(acceptance.to_string());
    save_spec_yaml(&path, &spec)?;
    println!("{} Set acceptance on {}", "ok".green(), spec.id);
    Ok(())
}

fn run_decide_command(args: DecideArgs) -> anyhow::Result<()> {
    run_decide(
        &args.spec,
        &args.title,
        &args.winner,
        args.rejected.as_deref(),
        &args.context,
        &args.decision,
        &args.rationale,
        &args.consequences,
        args.force,
        args.confirmed_by.as_deref(),
        args.challenge_summary.as_deref(),
        args.challenge_mode.as_deref(),
        args.no_challenge,
        args.decision_subject.as_deref(),
        args.decision_subject_granularity.as_deref(),
        args.evaluative_surface.as_deref(),
        args.belief_state.as_deref(),
        args.outcome_model.as_deref(),
        args.pareto_set.as_deref(),
        args.choice_rule.as_deref(),
    )
}

fn run_reflect_command(args: ReflectArgs) -> anyhow::Result<()> {
    run_reflect(
        &args.spec,
        args.verdict.as_deref(),
        args.measurement_window.as_deref(),
        &args.summary,
        &args.worked,
        &args.issues,
        &args.follow_ups,
        &args.evidence,
        &args.telemetry_refs,
        &args.heuristics,
        &args.structured_issues,
    )
}

fn run_install_command(args: InstallArgs) -> anyhow::Result<()> {
    cmd_provision::run_install(
        args.s5d_path.as_deref(),
        args.dry_run,
        args.uninstall,
        args.claude,
        args.cursor,
        args.codex,
        args.gemini,
        args.all,
        args.global,
    )
}

// ── Gate (S5D-Spec: feat.s5d.pretool-enforcement) ────────────────────────────

fn malformed_hook_block(
    hook_name: &str,
    detail: impl std::fmt::Display,
) -> s5d::gate::GateDecision {
    s5d::gate::GateDecision::Block {
        reason: format!(
            "S5D enforcement: malformed {} hook input ({}). Cannot verify scope; fix hook JSON/configuration or use S5D_BYPASS=1 as explicit break-glass.",
            hook_name, detail
        ),
    }
}

fn parse_required_hook_json(
    buf: &str,
    hook_name: &str,
) -> Result<serde_json::Value, s5d::gate::GateDecision> {
    serde_json::from_str(buf).map_err(|err| malformed_hook_block(hook_name, err))
}

fn required_tool_file_path(
    tool_input: &serde_json::Value,
    hook_name: &str,
) -> Result<String, s5d::gate::GateDecision> {
    tool_input
        .get("file_path")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| malformed_hook_block(hook_name, "missing tool_input.file_path"))
}

fn required_bash_command(
    value: &serde_json::Value,
    hook_name: &str,
) -> Result<String, s5d::gate::GateDecision> {
    value
        .get("tool_input")
        .and_then(|t| t.get("command"))
        .and_then(|c| c.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| malformed_hook_block(hook_name, "missing tool_input.command"))
}

fn print_gate_decision(decision: s5d::gate::GateDecision) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string(&decision)?);
    Ok(())
}

/// Pure-Rust PreToolUse(Edit|Write|MultiEdit) hook entrypoint.
/// Reads Claude Code hook JSON from stdin, returns decision JSON to stdout.
/// Wired into project hooks.json by `s5d init` — no shell wrapper.
/// S5D-Spec: feat.s5d.pretool-enforcement
fn run_hook_pretool_edit() -> anyhow::Result<()> {
    use s5d::gate::{
        evaluate_edit, load_session_state, matches_trivial_allowlist, save_session_state,
        GateDecision, ThresholdConfig,
    };
    use std::io::Read;

    // Bypass first — never block when explicitly requested
    if std::env::var("S5D_BYPASS")
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        println!(r#"{{"decision":"approve"}}"#);
        return Ok(());
    }

    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf).ok();

    // PreToolUse(Edit|Write|MultiEdit) is an enforcement hook. Malformed
    // envelopes must fail closed because the gate cannot verify scope.
    let value = match parse_required_hook_json(&buf, "PreToolUse(Edit|Write|MultiEdit)") {
        Ok(v) => v,
        Err(decision) => return print_gate_decision(decision),
    };

    let session_id = value
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("default")
        .to_string();
    let tool_input = value
        .get("tool_input")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let file_str = match required_tool_file_path(&tool_input, "PreToolUse(Edit|Write|MultiEdit)") {
        Ok(path) => path,
        Err(decision) => return print_gate_decision(decision),
    };
    let file = std::path::PathBuf::from(file_str);

    // Compute loc_delta from tool_input. Edit: |new_string| - |old_string|.
    // Write: |content|. MultiEdit: sum across edits. Best-effort.
    let loc_delta = compute_loc_delta(&tool_input);

    let cwd = std::env::current_dir()?;
    let s5d_dir_path = cwd.join(".s5d");
    let s5d_dir_opt = if s5d_dir_path.exists() {
        Some(s5d_dir_path.as_path())
    } else {
        None
    };

    let state = if let Some(s) = s5d_dir_opt {
        load_session_state(s, &session_id)?
    } else {
        Default::default()
    };

    let decision = evaluate_edit(
        s5d_dir_opt,
        &file,
        loc_delta,
        &state,
        &ThresholdConfig::default(),
    );

    if matches!(decision, GateDecision::Approve) {
        if let Some(s) = s5d_dir_opt {
            use s5d::gate::covered_by_spec;
            let counts = !matches_trivial_allowlist(&file) && covered_by_spec(s, &file).is_none();
            if counts {
                let mut new_state = state;
                new_state.session_id = session_id;
                new_state.loc_delta += loc_delta;
                new_state.files.insert(file);
                save_session_state(s, &new_state).ok();
            }
        }
    }

    println!("{}", serde_json::to_string(&decision)?);
    Ok(())
}

/// Pure-Rust UserPromptSubmit hook entrypoint (L1 advisory).
/// Reads Claude Code hook JSON from stdin, classifies the prompt, and
/// emits a reminder on stdout if the request is in S5D scope but no spec
/// exists yet. Never blocks (always exits 0 with empty or reminder text).
/// S5D-Spec: feat.s5d.pretool-enforcement
fn run_hook_user_prompt_submit() -> anyhow::Result<()> {
    use std::io::Read;

    // Bypass — silent
    if std::env::var("S5D_BYPASS")
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        return Ok(());
    }

    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf).ok();
    let value: serde_json::Value = match serde_json::from_str(&buf) {
        Ok(v) => v,
        Err(_) => return Ok(()), // fail-silent on malformed envelope
    };

    let prompt = value
        .get("prompt")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if prompt.is_empty() {
        return Ok(());
    }

    // Classify via the existing router (same logic `s5d route` uses)
    let result = s5d::route(prompt);
    if !result.in_scope {
        return Ok(()); // trivial — no noise
    }

    // Project must have opted into s5d to receive reminders
    let cwd = std::env::current_dir()?;
    let s5d_dir = cwd.join(".s5d");
    if !s5d_dir.exists() {
        return Ok(());
    }

    // If at least one spec already exists, assume the user is mid-flow and
    // don't pile on reminders. (Refinement target for Phase 4: only specs
    // covering THIS prompt's likely scope should suppress.)
    let packages_dir = s5d_dir.join("packages");
    if let Ok(mut entries) = std::fs::read_dir(&packages_dir) {
        if entries.any(|e| {
            e.as_ref()
                .ok()
                .and_then(|x| {
                    x.path()
                        .extension()
                        .and_then(|s| s.to_str())
                        .map(str::to_string)
                })
                .as_deref()
                == Some("yaml")
        }) {
            return Ok(());
        }
    }

    // Non-trivial scope, no spec — emit advisory. Claude injects stdout
    // text into the agent's context for this turn.
    let tier = result
        .tier
        .as_ref()
        .map(|t| format!("{:?}", t).to_lowercase())
        .unwrap_or_else(|| "standard".to_string());
    println!(
        "[S5D advisory] This request looks non-trivial (tier: {}). \
S5D is mandatory for non-trivial work in this project — run `s5d new <feature-id> --product <name>` \
before any Edit/Write tool call. Use `S5D_BYPASS=1` only as explicit break-glass for justified one-offs.",
        tier
    );
    Ok(())
}

/// Pure-Rust PreToolUse(Bash) require-spec hook (L3 final net).
/// Replaces `hooks/require-spec.sh`. Blocks `git commit` when the change
/// is non-trivial and the commit message lacks a spec reference.
/// S5D-Spec: feat.s5d.pretool-enforcement
fn run_hook_require_spec() -> anyhow::Result<()> {
    use std::io::Read;

    if std::env::var("S5D_BYPASS")
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        println!(r#"{{"decision":"approve"}}"#);
        return Ok(());
    }

    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf).ok();
    let value = match parse_required_hook_json(&buf, "PreToolUse(Bash require-spec)") {
        Ok(v) => v,
        Err(decision) => return print_gate_decision(decision),
    };

    let command = match required_bash_command(&value, "PreToolUse(Bash require-spec)") {
        Ok(command) => command,
        Err(decision) => return print_gate_decision(decision),
    };

    // Only care about git commit
    if !is_git_commit(&command) {
        println!(r#"{{"decision":"approve"}}"#);
        return Ok(());
    }

    // Skip if project hasn't opted in
    let cwd = std::env::current_dir()?;
    if !cwd.join(".s5d/packages").is_dir() {
        println!(r#"{{"decision":"approve"}}"#);
        return Ok(());
    }

    // Count staged source files (excludes config/docs/tests/lockfiles via existing is_source_path)
    let staged = git_staged_files(&cwd).unwrap_or_default();
    let source_files: Vec<&String> = staged.iter().filter(|p| is_source_path(p)).collect();
    let loc_delta = git_staged_insertions().unwrap_or(0);

    // Trivial change → allow
    if source_files.len() <= 1 && loc_delta <= 30 {
        println!(r#"{{"decision":"approve"}}"#);
        return Ok(());
    }

    // Non-trivial: any approved spec OR commit-message ref → allow
    if has_any_feature_spec(&cwd) || has_applied_record(&cwd) {
        println!(r#"{{"decision":"approve"}}"#);
        return Ok(());
    }
    if commit_msg_has_spec_ref(&command) {
        println!(r#"{{"decision":"approve"}}"#);
        return Ok(());
    }

    let reason = format!(
        "Non-trivial change ({} source files, +{} LOC) without S5D spec. \
Run /s5d to create a spec first, or add 'S5D-Spec: <spec-id>' to commit message if spec exists elsewhere.",
        source_files.len(), loc_delta
    );
    println!(
        "{}",
        serde_json::json!({"decision": "block", "reason": reason})
    );
    Ok(())
}

fn is_git_commit(command: &str) -> bool {
    // Match leading `git commit` or `git -c ... commit`
    let trimmed = command.trim_start();
    trimmed.starts_with("git commit")
        || (trimmed.starts_with("git ") && trimmed.contains(" commit "))
}

fn git_staged_insertions() -> Option<usize> {
    let output = std::process::Command::new("git")
        .args(["diff", "--cached", "--shortstat"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout);
    // e.g. " 3 files changed, 47 insertions(+), 5 deletions(-)"
    s.split(',').find_map(|part| {
        let p = part.trim();
        p.strip_suffix(" insertions(+)")
            .or_else(|| p.strip_suffix(" insertion(+)"))
            .and_then(|n| n.trim().parse::<usize>().ok())
    })
}

fn has_any_feature_spec(cwd: &std::path::Path) -> bool {
    let dir = cwd.join(".s5d/packages");
    std::fs::read_dir(&dir)
        .map(|entries| {
            entries.flatten().any(|e| {
                e.file_name().to_string_lossy().starts_with("feat.")
                    && e.file_name().to_string_lossy().ends_with(".s5d.yaml")
            })
        })
        .unwrap_or(false)
}

fn has_applied_record(cwd: &std::path::Path) -> bool {
    let dir = cwd.join(".s5d/records");
    std::fs::read_dir(&dir)
        .map(|entries| {
            entries.flatten().any(|e| {
                let path = e.path();
                if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
                    return false;
                }
                std::fs::read_to_string(&path)
                    .map(|c| {
                        c.contains("status: applied")
                            || c.contains("status: operated")
                            || c.contains("status: approved")
                    })
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn commit_msg_has_spec_ref(command: &str) -> bool {
    // Extract -m "..." payload — handles single and double quotes
    let mut chars = command.chars().peekable();
    let mut found_dash_m = false;
    let mut msg = String::new();
    let mut in_quote: Option<char> = None;
    while let Some(c) = chars.next() {
        if !found_dash_m {
            if c == '-' && chars.peek() == Some(&'m') {
                chars.next();
                found_dash_m = true;
                while let Some(&p) = chars.peek() {
                    if p == ' ' || p == '\t' {
                        chars.next();
                    } else {
                        break;
                    }
                }
                if let Some(&q) = chars.peek() {
                    if q == '"' || q == '\'' {
                        in_quote = Some(q);
                        chars.next();
                    }
                }
            }
            continue;
        }
        if let Some(q) = in_quote {
            if c == q {
                break;
            }
            msg.push(c);
        } else {
            if c == ' ' || c == '\t' {
                break;
            }
            msg.push(c);
        }
    }
    msg.contains("S5D-Spec:") || msg.contains("spec://")
}

/// Pure-Rust PreToolUse(Bash) staged-spec validation hook.
/// On `git commit`, validates every staged .s5d.yaml file via the same
/// validate path used by `s5d validate`. Replaces hooks/pre-commit-validate.sh.
/// S5D-Spec: feat.s5d.pretool-enforcement
fn run_hook_pre_commit_validate() -> anyhow::Result<()> {
    use std::io::Read;

    if std::env::var("S5D_BYPASS")
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        println!(r#"{{"decision":"approve"}}"#);
        return Ok(());
    }

    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf).ok();
    let value = match parse_required_hook_json(&buf, "PreToolUse(Bash staged-spec validation)") {
        Ok(v) => v,
        Err(decision) => return print_gate_decision(decision),
    };
    let command = match required_bash_command(&value, "PreToolUse(Bash staged-spec validation)") {
        Ok(command) => command,
        Err(decision) => return print_gate_decision(decision),
    };
    if !is_git_commit(&command) {
        println!(r#"{{"decision":"approve"}}"#);
        return Ok(());
    }

    let cwd = std::env::current_dir()?;
    let staged = git_staged_files(&cwd).unwrap_or_default();
    let specs: Vec<&String> = staged.iter().filter(|p| p.ends_with(".s5d.yaml")).collect();
    if specs.is_empty() {
        println!(r#"{{"decision":"approve"}}"#);
        return Ok(());
    }

    let mut failures = Vec::new();
    for spec_rel in specs {
        let spec_path = cwd.join(spec_rel);
        if !spec_path.exists() {
            continue;
        }
        // Reuse the validate path the `s5d validate` CLI uses
        match validate_spec_file(&spec_path) {
            Ok(()) => {}
            Err(e) => failures.push(format!("  {}: {}", spec_rel, e)),
        }
    }

    if failures.is_empty() {
        println!(r#"{{"decision":"approve"}}"#);
    } else {
        let reason = format!("S5D spec validation failed:\n{}", failures.join("\n"));
        println!(
            "{}",
            serde_json::json!({"decision": "block", "reason": reason})
        );
    }
    Ok(())
}

/// Validate a single spec yaml — same logic as `s5d validate`. Returns Err
/// with the human-readable failure on parse/schema/metamodel error.
fn validate_spec_file(path: &std::path::Path) -> anyhow::Result<()> {
    let raw = std::fs::read_to_string(path)?;
    let spec: s5d::Spec =
        serde_yaml::from_str(&raw).map_err(|e| anyhow::anyhow!("YAML parse error: {}", e))?;
    let errors = s5d::validate::validate_spec(&spec);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("{}", errors.join("; ")))
    }
}

/// Best-effort line-count delta from a tool_input JSON value.
fn compute_loc_delta(tool_input: &serde_json::Value) -> usize {
    fn count_lines(s: &str) -> usize {
        s.lines().count().max(1)
    }

    // MultiEdit shape: { edits: [{old_string, new_string}, ...] }
    if let Some(edits) = tool_input.get("edits").and_then(|v| v.as_array()) {
        return edits
            .iter()
            .map(|e| {
                let old = e.get("old_string").and_then(|v| v.as_str()).unwrap_or("");
                let new = e.get("new_string").and_then(|v| v.as_str()).unwrap_or("");
                count_lines(new).saturating_sub(count_lines(old))
            })
            .sum();
    }
    // Edit shape: { old_string, new_string }
    if let (Some(old), Some(new)) = (
        tool_input.get("old_string").and_then(|v| v.as_str()),
        tool_input.get("new_string").and_then(|v| v.as_str()),
    ) {
        return count_lines(new).saturating_sub(count_lines(old));
    }
    // Write shape: { content }
    if let Some(content) = tool_input.get("content").and_then(|v| v.as_str()) {
        return count_lines(content);
    }
    1
}

fn run_gate(command: GateCommand) -> anyhow::Result<()> {
    use s5d::gate::{
        evaluate_edit, load_session_state, save_session_state, GateDecision, ThresholdConfig,
    };

    match command {
        GateCommand::Edit {
            file,
            loc_delta,
            session_id,
            bypass,
        } => {
            // Bypass: explicit flag OR env var (h2)
            let env_bypass = std::env::var("S5D_BYPASS")
                .map(|v| v == "1")
                .unwrap_or(false);
            if bypass || env_bypass {
                println!(
                    "{}",
                    serde_json::to_string(&GateDecision::Approve)
                        .unwrap_or_else(|_| r#"{"decision":"approve"}"#.to_string())
                );
                return Ok(());
            }

            let cwd = std::env::current_dir()?;
            let s5d_dir_path = cwd.join(".s5d");
            let s5d_dir_opt = if s5d_dir_path.exists() {
                Some(s5d_dir_path.as_path())
            } else {
                None
            };

            let sid = session_id.unwrap_or_else(|| "default".to_string());
            let state = if let Some(s) = s5d_dir_opt {
                load_session_state(s, &sid)?
            } else {
                Default::default()
            };

            let decision = evaluate_edit(
                s5d_dir_opt,
                &file,
                loc_delta,
                &state,
                &ThresholdConfig::default(),
            );

            // On approve, persist updated counter — but only for non-trivial files
            // not covered by an existing spec. Trivial allowlist hits and
            // spec-covered files don't count against the session threshold.
            if matches!(decision, GateDecision::Approve) {
                if let Some(s) = s5d_dir_opt {
                    use s5d::gate::{covered_by_spec, matches_trivial_allowlist};
                    let counts =
                        !matches_trivial_allowlist(&file) && covered_by_spec(s, &file).is_none();
                    if counts {
                        let mut new_state = state;
                        new_state.session_id = sid;
                        new_state.loc_delta += loc_delta;
                        new_state.files.insert(file.clone());
                        save_session_state(s, &new_state).ok();
                    }
                }
            }

            println!("{}", serde_json::to_string(&decision)?);
            Ok(())
        }
        GateCommand::Reset { session_id, all } => {
            let cwd = std::env::current_dir()?;
            let s5d_dir = cwd.join(".s5d");
            if !s5d_dir.exists() {
                println!(r#"{{"reset":"noop","reason":"no .s5d/ directory"}}"#);
                return Ok(());
            }
            let mut removed = 0usize;
            if all {
                for entry in std::fs::read_dir(&s5d_dir)?.flatten() {
                    let name = entry.file_name();
                    let s = name.to_string_lossy();
                    if s.starts_with(".session-counter-")
                        && s.ends_with(".json")
                        && std::fs::remove_file(entry.path()).is_ok()
                    {
                        removed += 1;
                    }
                }
            } else if let Some(sid) = session_id {
                let path = s5d_dir.join(format!(".session-counter-{}.json", sid));
                if path.exists() && std::fs::remove_file(&path).is_ok() {
                    removed = 1;
                }
            } else {
                anyhow::bail!("specify --session-id <id> or --all");
            }
            println!(r#"{{"reset":"ok","removed":{}}}"#, removed);
            Ok(())
        }
    }
}

// install command handlers → mod cmd_provision (src/bin/s5d/cmd_provision.rs)

// ── Git Hooks ────────────────────────────────────────────────────────────────

pub(crate) enum HookInstallResult {
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

pub(crate) fn install_pre_commit_hook(
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

// update / init / AGENTS.md handlers → mod cmd_provision (src/bin/s5d/cmd_provision.rs)

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

    // The scaffold must be valid out of the box. The only way generation can
    // produce an invalid spec is pathological input (e.g. an id long enough
    // that derived artifact ids overflow the 64-char cap) — fail loud before
    // writing instead of handing the user a broken file.
    let scaffold_errors = s5d::validate_spec(&spec);
    if !scaffold_errors.is_empty() {
        anyhow::bail!(
            "generated scaffold would not validate (likely the feature/product id is too long for derived artifact ids):\n  {}",
            scaffold_errors.join("\n  ")
        );
    }

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
    let slug = s5d::truncate_chars(&slug, 40);
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
                println!("  {} {}", "Active work state:".dimmed(), active_phase);
            }
        }

        // Determine current lifecycle step and print next-action hint
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
        .ok_or_else(|| anyhow::anyhow!("work state not found: {}", phase_id))
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

pub(crate) fn ensure_phase_execution_ready(
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
        Some(format!("Started work state '{}'", phase.title)),
    );
    project.save_record(&spec_filename, &record)?;

    println!("{} Active work state → {}", "ok".green(), phase_id);
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
        Some("Human run evidence acceptance".into()),
    );
    project.save_record(&spec_filename, &record)?;

    println!(
        "{} Work state '{}' accepted by {}",
        "ok".green(),
        phase_id,
        reviewer
    );
    Ok(())
}

fn run_phase_run(spec_path: &str, phase_id: &str, engine_name: &str) -> anyhow::Result<()> {
    let (project, spec_path_buf, spec, spec_filename) = load_spec_context(spec_path)?;
    let workflow = workflow_required(&spec)?;
    workflow_phase_by_id(workflow, phase_id)?;
    let mut record = project.load_record(&spec_filename)?.ok_or_else(|| {
        anyhow::anyhow!("no record found for {} — run preview first", spec_filename)
    })?;
    ensure_phase_execution_ready(&spec, &record, phase_id)?;

    if record.active_phase.as_deref() != Some(phase_id) {
        anyhow::bail!("work state '{}' must be active before run exec", phase_id);
    }

    s5d::sanitize_id(phase_id)?;
    s5d::sanitize_id(engine_name)?;

    let config = project.load_config()?;
    let engine = config
        .engines
        .get(engine_name)
        .ok_or_else(|| anyhow::anyhow!("engine '{}' is not configured", engine_name))?;
    if !engine.approved {
        anyhow::bail!("engine '{}' is configured but not approved", engine_name);
    }
    if engine.command.is_empty() {
        anyhow::bail!("engine '{}' has no command template", engine_name);
    }

    let spec_stem = spec_filename
        .strip_suffix(".s5d.yaml")
        .unwrap_or(&spec_filename);
    let timestamp = chrono::Utc::now();
    let run_id = format!(
        "{}-{}-{}-{}",
        phase_id,
        engine_name,
        timestamp.format("%Y%m%dT%H%M%SZ"),
        std::process::id()
    );
    s5d::sanitize_id(&run_id)?;

    let run_dir = project.s5d_dir().join("runs").join(spec_stem).join(&run_id);
    std::fs::create_dir_all(&run_dir)?;
    let stdout_path = run_dir.join("stdout.txt");
    let stderr_path = run_dir.join("stderr.txt");

    let spec_abs = spec_path_buf
        .canonicalize()
        .unwrap_or_else(|_| spec_path_buf.clone());
    let replacements = [
        ("{spec}", spec_abs.display().to_string()),
        ("{spec_filename}", spec_filename.clone()),
        ("{phase}", phase_id.to_string()),
        ("{engine}", engine_name.to_string()),
        ("{run_id}", run_id.clone()),
        ("{run_dir}", run_dir.display().to_string()),
        ("{stdout}", stdout_path.display().to_string()),
        ("{stderr}", stderr_path.display().to_string()),
        ("{s5d_dir}", project.s5d_dir().display().to_string()),
        ("{root}", project.root.display().to_string()),
    ];
    let render_arg = |arg: &str| {
        replacements
            .iter()
            .fold(arg.to_string(), |acc, (from, to)| acc.replace(from, to))
    };
    let command = engine
        .command
        .iter()
        .map(|arg| render_arg(arg))
        .collect::<Vec<_>>();

    let output = std::process::Command::new(&command[0])
        .args(&command[1..])
        .current_dir(&project.root)
        .output()
        .map_err(|err| anyhow::anyhow!("engine '{}' failed to start: {}", engine_name, err))?;

    std::fs::write(&stdout_path, &output.stdout)?;
    std::fs::write(&stderr_path, &output.stderr)?;
    let output_sha256 = s5d::S5dProject::file_sha256(&stdout_path)?;
    let status = if output.status.success() {
        "completed"
    } else {
        "failed"
    };
    let completed_at = chrono::Utc::now().to_rfc3339();
    let output_ref = project_relative_path(&project, &stdout_path);
    let stderr_ref = project_relative_path(&project, &stderr_path);

    record.phase_runs.push(s5d::WorkflowPhaseRun {
        run_id: run_id.clone(),
        phase_id: phase_id.to_string(),
        engine: engine_name.to_string(),
        status: status.to_string(),
        timestamp: timestamp.to_rfc3339(),
        completed_at: Some(completed_at),
        reasoning: engine.reasoning.clone(),
        exit_code: output.status.code(),
        output_path: output_ref.clone(),
        output_sha256: output_sha256.clone(),
        stderr_path: Some(stderr_ref.clone()),
    });

    if output.status.success() {
        append_phase_history(
            &mut record,
            phase_id,
            s5d::WorkflowPhaseStatus::Verified,
            None,
            Some(engine_name.to_string()),
            Some(format!("External engine run completed: {}", run_id)),
        );
    }
    project.save_record(&spec_filename, &record)?;

    println!("{} Run {}", "ok".green(), status);
    println!("  {} {}", "run_id:".dimmed(), run_id);
    println!("  {} {}", "engine:".dimmed(), engine_name);
    println!("  {} {}", "output:".dimmed(), output_ref);
    println!("  {} {}", "sha256:".dimmed(), output_sha256);
    if !output.status.success() {
        anyhow::bail!(
            "engine '{}' exited with status {:?}; stderr captured at {}",
            engine_name,
            output.status.code(),
            stderr_ref
        );
    }
    Ok(())
}

pub(crate) fn project_relative_path(project: &s5d::S5dProject, path: &std::path::Path) -> String {
    path.strip_prefix(project.root.as_path())
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
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

// harness command handlers → mod cmd_harness (src/bin/s5d/cmd_harness.rs)

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
    let reviewer = reviewer.trim();
    if reviewer.is_empty() {
        eprintln!("  {} --reviewer must not be empty", "error:".red());
        std::process::exit(2);
    }

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
                "  {} spec modified since preview — re-run `s5d state preview` before approving",
                "error:".red()
            );
            std::process::exit(1);
        }
    }

    let diff_sha = record
        .preview
        .as_ref()
        .map(|p| p.diff_sha256.clone())
        .ok_or_else(|| {
            anyhow::anyhow!("no preview result on record — run `s5d state preview` first")
        })?;

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

fn run_import(spec_arg: &str, verified_by: &str, force: bool) -> anyhow::Result<()> {
    let verified_by = verified_by.trim();
    if verified_by.is_empty() {
        eprintln!("  {} --verified-by must not be empty", "error:".red());
        std::process::exit(2);
    }

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

    let effective_gates = s5d::effective_gates_for_spec(&spec);
    if !effective_gates.is_empty() {
        // Check that the LATEST result for each declared gate kind is "passed".
        // FPF B.3.4:5 (CC-ED.5) — waivers expire; expired waivers are auto-revoked here.
        let now_rfc = chrono::Utc::now().to_rfc3339();
        let all_latest_passed = effective_gates.iter().all(|g| {
            let latest = record
                .gate_results
                .iter()
                .rev()
                .find(|r| r.kind == g.kind);
            match latest {
                Some(r) if r.status == "passed" => true,
                Some(r) if r.status == "waived" => {
                    // Check waiver expiry per FPF B.3.4:5
                    match r.waiver_expires_at.as_ref() {
                        None => {
                            eprintln!(
                                "  {} waiver for gate '{}' has no expiry (FPF B.3.4:5 violation) — re-issue with --expires-at",
                                "error:".red(),
                                g.kind
                            );
                            false
                        }
                        Some(exp) => {
                            if exp.as_str() < now_rfc.as_str() {
                                eprintln!(
                                    "  {} waiver for gate '{}' expired at {} — auto-revoked. Re-run-gates or re-issue waiver.",
                                    "error:".red(),
                                    g.kind,
                                    exp
                                );
                                false
                            } else {
                                true
                            }
                        }
                    }
                }
                _ => false,
            }
        });
        if !all_latest_passed {
            eprintln!(
                "  {} all effective gates must pass (or have non-expired waivers) before import — run `s5d verify run-gates` first",
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

    // Scaffold placeholders must not become recorded architecture: the default
    // standard/high gate set (schema+graph) does not inspect source paths.
    let placeholders = s5d::validate::placeholder_path_components(&spec);
    if !placeholders.is_empty() {
        eprintln!(
            "  {} component(s) still carry scaffold placeholder paths ({}): {} — set real source paths before import",
            "error:".red(),
            s5d::validate::SCAFFOLD_PATH_TODO,
            placeholders.join(", ")
        );
        std::process::exit(8);
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
    record.verified_by = Some(verified_by.to_string());
    project.save_record(&spec_filename, &record)?;

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

    let report = s5d::import::rollback_spec(&project, &spec, &spec_filename)?;

    for warning in &report.ownership_mismatches {
        eprintln!(
            "{} ownership mismatch (possible owning_package corruption), NOT tombstoned: {}",
            "warn".yellow(),
            warning
        );
    }
    for warning in &report.suspected_tampers {
        eprintln!(
            "{} suspected ownership tamper, no action taken: {}",
            "warn".yellow(),
            warning
        );
    }
    for label in &report.ownership_unverifiable {
        eprintln!(
            "{} ownership unverifiable (package files edited or deleted since import), NOT tombstoned: {}",
            "warn".yellow(),
            label
        );
    }
    for label in &report.underivable_fallbacks {
        eprintln!(
            "{} no ledger trace for {} — tombstoned via stored owning_package (legacy fallback)",
            "warn".yellow(),
            label
        );
    }

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

// codebase / discover command handlers → mod cmd_codebase (src/bin/s5d/cmd_codebase.rs)

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
        s5d::DriftResult::Partial { policy, note } => {
            println!(
                "  {} {} — partial drift (tolerated by policy: {})\n    {}",
                "warn:".yellow(),
                spec_id,
                policy,
                note
            );
            if let Ok(Some(mut record)) = project.load_record(spec_filename) {
                record.sync_status = s5d::SyncStatus::Synced;
                let _ = project.save_record(spec_filename, &record);
            }
            Ok(false)
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
        let mut failed = 0usize;

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
                    failed += 1;
                }
            }
        }

        println!("\n  {} spec(s) reconciled, {} failed", reconciled, failed);
        if failed > 0 {
            anyhow::bail!("{} spec(s) failed to reconcile", failed);
        }
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

            // Truncate content to first line or 72 chars (char-safe — content is often Cyrillic)
            let content_short = ev.content.lines().next().unwrap_or("");
            let content_display = if content_short.chars().count() > 72 {
                format!("{}…", s5d::truncate_chars(content_short, 72))
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
            println!("  {}: {}", "Work states".dimmed(), workflow.phases.len());
        }
    }

    if let Some(record) = record {
        if let Some(ref active_phase) = record.active_phase {
            println!("  {}: {}", "Active work state".dimmed(), active_phase);
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

    let effective_gates = s5d::effective_gates_for_spec(spec);
    if !effective_gates.is_empty() {
        let source = if spec.gates.is_empty() {
            "effective, tier defaults"
        } else {
            "declared"
        };
        println!(
            "  {}: {} ({})",
            "Gates".dimmed(),
            effective_gates.len(),
            source
        );
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

#[allow(clippy::too_many_arguments)]
fn run_add_hypothesis(
    spec_path: &str,
    custom_id: Option<&str>,
    title: &str,
    content: &str,
    scope: &str,
    kind: &str,
    rationale: Option<&str>,
    prompt: Option<&str>,
    next_move: Option<&str>,
) -> anyhow::Result<()> {
    if let Some(nm) = next_move {
        let allowed = ["deduction", "probe", "build", "defer"];
        if !allowed.contains(&nm) {
            eprintln!("error: --next-move must be one of: {}", allowed.join(", "));
            std::process::exit(1);
        }
    }
    let (path, mut spec) = load_spec_yaml(spec_path)?;

    if !matches!(spec.tier, s5d::Tier::Decision | s5d::Tier::High) {
        anyhow::bail!(
            "add-hypothesis only works on decision- or high-tier specs (this spec is {})",
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
        prompt: prompt.map(|s| s.into()),
        next_move: next_move.map(|s| s.into()),
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
    refine_kind: Option<&str>,
    skill: Option<String>,
    agent: Option<String>,
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
    if verdict == "refine" && refine_kind.is_none() {
        eprintln!(
            "error: --refine-kind is required when verdict=refine (FPF C.2:4.2 Δ-move). \
            Allowed: formalise, generalise, specialise, calibrate, validate, congrue."
        );
        std::process::exit(1);
    }
    if let Some(rk) = refine_kind {
        let allowed = [
            "formalise",
            "generalise",
            "specialise",
            "calibrate",
            "validate",
            "congrue",
        ];
        if !allowed.contains(&rk) {
            eprintln!(
                "error: --refine-kind must be one of: {}",
                allowed.join(", ")
            );
            std::process::exit(1);
        }
    }
    s5d::sanitize_id(evidence_type)?;
    let (path, mut spec) = load_spec_yaml(spec_path)?;

    if !matches!(spec.tier, s5d::Tier::Decision | s5d::Tier::High) {
        anyhow::bail!(
            "add-evidence only works on decision- or high-tier specs (this spec is {})",
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
        refine_kind: refine_kind.map(|s| s.into()),
        skill,
        agent,
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
    decision_subject: Option<&str>,
    decision_subject_granularity: Option<&str>,
    evaluative_surface: Option<&str>,
    belief_state: Option<&str>,
    outcome_model: Option<&str>,
    pareto_set: Option<&str>,
    choice_rule: Option<&str>,
) -> anyhow::Result<()> {
    // FPF C.18 — forbid weighted scalarization in evaluative_surface (NQD requires partial order, no scalar fold).
    if let Some(es) = evaluative_surface {
        let lower = es.to_lowercase();
        let scalar_markers = [
            "weighted sum",
            "weighted-sum",
            "weighted_sum",
            "scalar fold",
            "scalar-fold",
            "0.5*",
            "0.3*",
        ];
        if scalar_markers.iter().any(|m| lower.contains(m)) {
            eprintln!(
                "  {} evaluative_surface contains scalarization marker. FPF C.18 (NQD-CAL) forbids weighted sums across mixed scales — return Pareto set with tie notes instead.",
                "warn:".yellow()
            );
            if !force {
                anyhow::bail!(
                    "scalarization detected in evaluative_surface (FPF C.18 violation). Use --force to override."
                );
            }
        }
    }

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

    let pareto_ids: Vec<String> = match pareto_set {
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
        decision_subject: decision_subject.map(|s| s.to_string()),
        decision_subject_granularity: decision_subject_granularity.map(|s| s.to_string()),
        evaluative_surface: evaluative_surface.map(|s| s.to_string()),
        belief_state: belief_state.map(|s| s.to_string()),
        outcome_model: outcome_model.map(|s| s.to_string()),
        pareto_set: pareto_ids,
        choice_rule: choice_rule.map(|s| s.to_string()),
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

// ── Trace ─────────────────────────────────────────────────────────────────────

fn run_trace(path: &str) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let project = s5d::S5dProject::find(&cwd)
        .ok_or_else(|| anyhow::anyhow!("no .s5d/ found (run `s5d init` first)"))?;
    let trace = s5d::trace_code_path(&project, path)?;
    println!("{}", s5d::format_code_trace(&trace));
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

pub(crate) fn load_spec_context(
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
    use crate::cmd_provision::{
        agents_block, ensure_agents_md, AgentsUpdate, AGENTS_BEGIN, AGENTS_END,
    };
    use clap::CommandFactory;
    use tempfile::tempdir;

    #[test]
    fn cli_public_help_keeps_grouped_surface_small() {
        let mut command = Cli::command();
        let mut buf = Vec::new();
        command.write_long_help(&mut buf).unwrap();
        let help = String::from_utf8(buf).unwrap();

        for public in [
            "init", "new", "decision", "verify", "state", "run", "status", "show", "trace", "admin",
        ] {
            assert!(
                help.lines()
                    .any(|line| line.trim_start().starts_with(&format!("{public} "))),
                "top-level help should expose `{public}`:\n{}",
                help
            );
        }

        for hidden in [
            "add-hypothesis",
            "validate",
            "preview",
            "run-gates",
            "apply",
            "execute",
            "harness",
            "phase",
            "codebase",
            "discover",
            "hook",
            "install",
            "update",
        ] {
            assert!(
                !help
                    .lines()
                    .any(|line| line.trim_start().starts_with(&format!("{hidden} "))),
                "top-level help should hide `{hidden}`:\n{}",
                help
            );
        }
    }

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

    #[test]
    fn block_marks_s5d_mandatory() {
        // The agent block must declare S5D as mandatory, not just suggested.
        let block = agents_block();
        assert!(
            block.contains("MANDATORY"),
            "agents block lost MANDATORY marker"
        );
        assert!(
            block.contains("S5D-Spec:"),
            "agents block lost commit trailer rule"
        );
        assert!(
            block.contains("break-glass"),
            "agents block must describe bypass as break-glass"
        );
        assert!(
            !block.contains("no exceptions"),
            "agents block must not overstate enforcement when break-glass exists"
        );
    }

    #[test]
    fn commit_spec_ref_detection_requires_explicit_marker() {
        assert!(commit_msg_has_spec_ref(
            r#"git commit -m "tighten hook detection

S5D-Spec: feat.s5d.pretool-enforcement""#
        ));
        assert!(commit_msg_has_spec_ref(
            r#"git commit -m "implement timeout

Implements: spec://s5d/PROP-003#verification.timeout""#
        ));
        assert!(!commit_msg_has_spec_ref(
            r#"git commit -m "feat.s5d cleanup without trailer""#
        ));
        assert!(!commit_msg_has_spec_ref(
            r#"git commit -m "Implements: local cleanup""#
        ));
    }

    #[test]
    fn malformed_enforcement_hook_json_blocks() {
        let decision = parse_required_hook_json("{not json", "PreToolUse(Edit)")
            .expect_err("malformed enforcement JSON should fail closed");
        match decision {
            s5d::gate::GateDecision::Block { reason } => {
                assert!(reason.contains("malformed PreToolUse(Edit) hook input"));
                assert!(reason.contains("break-glass"));
            }
            s5d::gate::GateDecision::Approve => panic!("malformed hook input approved"),
        }
    }

    #[test]
    fn missing_pretool_file_path_blocks() {
        let tool_input = serde_json::json!({});
        let decision = required_tool_file_path(&tool_input, "PreToolUse(Edit|Write|MultiEdit)")
            .expect_err("missing file path should fail closed");
        match decision {
            s5d::gate::GateDecision::Block { reason } => {
                assert!(reason.contains("missing tool_input.file_path"));
            }
            s5d::gate::GateDecision::Approve => panic!("missing file path approved"),
        }
    }

    #[test]
    fn missing_bash_command_blocks() {
        let envelope = serde_json::json!({ "tool_input": {} });
        let decision = required_bash_command(&envelope, "PreToolUse(Bash require-spec)")
            .expect_err("missing bash command should fail closed");
        match decision {
            s5d::gate::GateDecision::Block { reason } => {
                assert!(reason.contains("missing tool_input.command"));
            }
            s5d::gate::GateDecision::Approve => panic!("missing bash command approved"),
        }
    }

    #[test]
    fn injects_into_existing_runtime_files() {
        // CLAUDE.md / GEMINI.md exist in user's project — block must be appended,
        // not refuse. Using ensure_agents_md directly (init flow uses same fn).
        let dir = tempdir().unwrap();
        for filename in ["CLAUDE.md", "GEMINI.md"] {
            let path = dir.path().join(filename);
            std::fs::write(&path, "# Project\n\nExisting rules.\n").unwrap();
            assert_eq!(
                ensure_agents_md(&path).unwrap(),
                AgentsUpdate::Inserted,
                "expected inserted block for {}",
                filename
            );
            let body = std::fs::read_to_string(&path).unwrap();
            assert!(body.contains("Existing rules."));
            assert!(body.contains(AGENTS_BEGIN));
            assert!(body.contains("MANDATORY"));
        }
    }
}
