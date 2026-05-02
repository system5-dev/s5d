use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Spec ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub s5d: String,
    pub id: String,
    pub version: String,
    pub product: String,
    pub tier: Tier,
    #[serde(default)]
    pub allow_update: bool,
    #[serde(default)]
    pub meta: Option<Meta>,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub workflow: Option<Workflow>,
    #[serde(default)]
    pub artifacts: Option<Artifacts>,
    #[serde(default)]
    pub links: Option<Links>,
    #[serde(default)]
    pub contracts: Vec<Contract>,
    #[serde(default)]
    pub gates: Vec<Gate>,
    #[serde(default)]
    pub roc: Option<Roc>,
    #[serde(default)]
    pub problem: Option<ProblemField>,
    #[serde(default)]
    pub hypotheses: Vec<Hypothesis>,
    #[serde(default)]
    pub decision: Option<DecisionRecord>,
    #[serde(default)]
    pub note_rationale: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub auto_noted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Note,
    Decision,
    Lightweight,
    Standard,
    High,
}

impl std::fmt::Display for Tier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tier::Note => write!(f, "note"),
            Tier::Decision => write!(f, "decision"),
            Tier::Lightweight => write!(f, "lightweight"),
            Tier::Standard => write!(f, "standard"),
            Tier::High => write!(f, "high"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub title: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub tickets: Vec<String>,
    #[serde(default)]
    pub adrs: Vec<String>,
    #[serde(default)]
    pub renames: Vec<Rename>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rename {
    pub old_id: String,
    pub new_id: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(default)]
    pub old_package: Option<String>,
}

// ── Artifacts ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Artifacts {
    #[serde(default)]
    pub products: Vec<Product>,
    #[serde(default)]
    pub domains: Vec<Domain>,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    #[serde(default)]
    pub entities: Vec<Entity>,
    #[serde(default)]
    pub features: Vec<Feature>,
    #[serde(default)]
    pub use_cases: Vec<UseCase>,
    #[serde(default)]
    pub systems: Vec<SoftwareSystem>,
    #[serde(default)]
    pub containers: Vec<Container>,
    #[serde(default)]
    pub components: Vec<Component>,
    #[serde(default)]
    pub roles: Vec<Role>,
    #[serde(default)]
    pub concerns: Vec<Concern>,
    #[serde(default)]
    pub metrics: Vec<AcceptanceMetric>,
    #[serde(default)]
    pub supersystems: Vec<SuperSystem>,
    #[serde(default)]
    pub transports: Vec<Transport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transport {
    pub id: String,
    #[serde(rename = "type")]
    pub transport_type: String,
    #[serde(default)]
    pub serialization: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub organization: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    pub id: String,
    pub product: String,
    pub name: String,
    #[serde(default)]
    pub classification: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub team: Option<String>,
    #[serde(default)]
    pub maturity_level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub id: String,
    pub domain: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub since: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub domain: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub id: String,
    pub product: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseCase {
    pub id: String,
    pub feature: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareSystem {
    pub id: String,
    pub product: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: String,
    pub system: String,
    pub name: String,
    #[serde(default)]
    pub technology: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub feature: String,
    pub domain: String,
    pub container: String,
    pub name: String,
    #[serde(default)]
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concern {
    pub id: String,
    pub role: String,
    pub name: String,
    #[serde(default)]
    pub supersystem: Option<String>,
    #[serde(default)]
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceMetric {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub units: Option<String>,
    #[serde(default)]
    pub how_to_measure: Option<String>,
    #[serde(default)]
    pub supersystem: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperSystem {
    pub id: String,
    pub product: String,
    pub name: String,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRelation {
    pub entity: String,
    pub related_entity: String,
    #[serde(default)]
    pub cardinality: Option<String>,
    #[serde(default)]
    pub projection: bool,
    #[serde(default)]
    pub aggregate_root: bool,
}

// ── Links ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Links {
    #[serde(default)]
    pub feature_to_domain: Vec<Binding>,
    #[serde(default)]
    pub use_case_to_capability: Vec<Binding>,
    #[serde(default)]
    pub use_case_to_entity: Vec<Binding>,
    #[serde(default)]
    pub component_to_capability: Vec<Binding>,
    #[serde(default)]
    pub component_to_entity: Vec<Binding>,
    #[serde(default)]
    pub container_to_capability: Vec<Binding>,
    #[serde(default)]
    pub concern_to_metric: Vec<Binding>,
    #[serde(default)]
    pub component_to_container: Vec<Binding>,
    #[serde(default)]
    pub edges: Vec<Edge>,
    #[serde(default)]
    pub depends_on: Vec<Dependency>,
    #[serde(default)]
    pub entity_relations: Vec<EntityRelation>,
    #[serde(default)]
    pub capability_to_entity: Vec<Binding>,
    #[serde(default)]
    pub capability_to_concern: Vec<Binding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    #[serde(flatten)]
    pub fields: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub archetype: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub downstream_capability: Option<String>,
    #[serde(default)]
    pub waiver: Option<Waiver>,
    #[serde(default)]
    pub transport_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Waiver {
    pub reason: String,
    pub owner: String,
    pub expiry: String,
    #[serde(default)]
    pub plan: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub feature: String,
    #[serde(default)]
    pub version: Option<String>,
}

// ── Contracts & Gates ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub id: String,
    pub format: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub inline: Option<String>,
    #[serde(default)]
    pub binds_to: Vec<ContractBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractBinding {
    #[serde(default)]
    pub capability: Option<String>,
    #[serde(default)]
    pub entity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate {
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Roc {
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub max_calls: Option<u32>,
    #[serde(default)]
    pub max_time_s: Option<u32>,
    #[serde(default)]
    pub risk: Option<String>,
    #[serde(default)]
    pub escalation: Option<String>,
}

// ── Workflow ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub target_architecture: Option<TargetArchitecture>,
    #[serde(default)]
    pub delivery_strategy: Option<DeliveryStrategy>,
    #[serde(default)]
    pub resources: Option<WorkflowResources>,
    #[serde(default)]
    pub role_map: HashMap<String, String>,
    #[serde(default)]
    pub review_policy: Option<ReviewPolicy>,
    #[serde(default)]
    pub structure_outline: Option<StructureOutline>,
    #[serde(default)]
    pub execution_mode: Option<ExecutionMode>,
    #[serde(default)]
    pub phases: Vec<WorkflowPhase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetArchitecture {
    pub summary: String,
    #[serde(default)]
    pub invariants: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryStrategy {
    pub summary: String,
    #[serde(default)]
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResources {
    #[serde(default)]
    pub declared: bool,
    #[serde(default)]
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewPolicy {
    #[serde(default)]
    pub cross_model_required: bool,
    #[serde(default)]
    pub required_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureOutline {
    pub summary: String,
    #[serde(default)]
    pub signatures: Vec<String>,
    #[serde(default)]
    pub types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMode {
    pub engine: String,
    #[serde(default)]
    pub max_iterations: Option<u32>,
    #[serde(default)]
    pub stop_conditions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPhase {
    pub id: String,
    pub title: String,
    pub scope: String,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub acceptance: Vec<String>,
    #[serde(default)]
    pub rollback: Vec<String>,
}

// ── Decision tier types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub id: String,
    pub title: String,
    pub content: String,
    pub scope: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub layer: String,
    #[serde(default)]
    pub r_eff: Option<f64>,
    #[serde(default)]
    pub evidence: Vec<HypothesisEvidence>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub rationale: Option<String>,
    /// Link to feature spec that decomposes this hypothesis.
    /// Required for the winner hypothesis before /s5d-decide; optional for rejected hypotheses.
    #[serde(default)]
    pub spec_ref: Option<String>,
}

/// Problem field accepts either a structured ProblemCard or a plain string (for feature specs).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProblemField {
    Card(ProblemCard),
    Text(String),
}

impl ProblemField {
    pub fn as_card(&self) -> Option<&ProblemCard> {
        match self {
            ProblemField::Card(c) => Some(c),
            ProblemField::Text(_) => None,
        }
    }

    pub fn signal(&self) -> &str {
        match self {
            ProblemField::Card(c) => &c.signal,
            ProblemField::Text(t) => t.as_str(),
        }
    }
}

/// Structured problem framing — what's broken, what matters, how we'll know it's solved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemCard {
    /// What anomaly or pain triggered this? One sentence.
    pub signal: String,
    /// Hard constraints — must hold, binary pass/fail
    #[serde(default)]
    pub constraints: Vec<String>,
    /// Optimization targets — what we're actively improving (1-3 max)
    #[serde(default)]
    pub targets: Vec<String>,
    /// Observation indicators — monitor but don't optimize (Anti-Goodhart)
    #[serde(default)]
    pub indicators: Vec<String>,
    /// How will we know the problem is solved?
    #[serde(default)]
    pub acceptance: Option<String>,
    /// What's affected if we get this wrong?
    #[serde(default)]
    pub blast_radius: Option<String>,
    /// How easy to undo? (trivial / moderate / hard / irreversible)
    #[serde(default)]
    pub reversibility: Option<String>,
    /// Lifecycle status: backlog / in_progress / addressed
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypothesisEvidence {
    pub id: String,
    #[serde(rename = "type")]
    pub evidence_type: String,
    pub content: String,
    pub verdict: String,
    #[serde(default)]
    pub valid_until: Option<String>,
    #[serde(default)]
    pub carrier_ref: Option<String>,
    #[serde(default)]
    pub formality: Option<u8>,
    #[serde(default)]
    pub claim_scope: Vec<String>,
    #[serde(default)]
    pub congruence_level: Option<u8>,
    #[serde(default)]
    pub reliability: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub title: String,
    pub winner_id: String,
    #[serde(default)]
    pub rejected_ids: Vec<String>,
    pub context: String,
    pub decision: String,
    pub rationale: String,
    pub consequences: String,
    #[serde(default)]
    pub decided_at: Option<String>,
    #[serde(default)]
    pub confirmed_by: Option<String>,
    /// When this decision expires and must be re-evaluated. Default: decided_at + 90 days.
    #[serde(default)]
    pub expires_at: Option<String>,
    /// What this decision mandates — explicit obligations.
    #[serde(default)]
    pub do_list: Vec<String>,
    /// What this decision prohibits — explicit prohibitions.
    #[serde(default)]
    pub dont_list: Vec<String>,
    /// Adversarial challenge results — must be present before decide (methodological gate).
    #[serde(default)]
    pub challenge: Option<Challenge>,
}

/// Adversarial challenge — pre-decision verification.
/// 5 probes for standard/high, 1 counter-argument for lightweight/tactical.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    /// Challenge depth: tactical (1 probe) or standard (5 probes)
    pub mode: String,
    /// Did the decision survive the challenge?
    pub passed: bool,
    /// One-line summary of challenge outcome
    pub summary: String,
    /// Individual challenge checks
    #[serde(default)]
    pub checks: Vec<ChallengeCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeCheck {
    /// Kind: counter_argument, tail_failure, evidence_weakness, weakest_link, sota
    pub kind: String,
    /// What the probe found
    pub finding: String,
    /// pass, fail, or concern
    pub verdict: String,
}

// ── Phase ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Scaffold,
    Synthesize,
    Preview,
    Approve,
    Execute,
    Verify,
    Learn,
}

impl Phase {
    pub fn next(&self) -> Option<Phase> {
        match self {
            Phase::Scaffold => Some(Phase::Synthesize),
            Phase::Synthesize => Some(Phase::Preview),
            Phase::Preview => Some(Phase::Approve),
            Phase::Approve => Some(Phase::Execute),
            Phase::Execute => Some(Phase::Verify),
            Phase::Verify => Some(Phase::Learn),
            Phase::Learn => None,
        }
    }

    pub fn cli_hint(&self) -> &'static str {
        match self {
            Phase::Scaffold => "Run: s5d new <id> --tier <tier> — create a spec scaffold",
            Phase::Synthesize => "Run: s5d add-hypothesis <spec> — add at least 3 hypotheses",
            Phase::Preview => "Run: s5d preview <spec> — dry-run the import",
            Phase::Approve => "Run: s5d approve <spec> --reviewer <name>",
            Phase::Execute => "Run: s5d import <spec> — execute the approved spec",
            Phase::Verify => "Run: s5d run-gates <spec> — verify all gates pass",
            Phase::Learn => "Record reflection: what worked, what didn't, follow-ups",
        }
    }
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Phase::Scaffold => write!(f, "scaffold"),
            Phase::Synthesize => write!(f, "synthesize"),
            Phase::Preview => write!(f, "preview"),
            Phase::Approve => write!(f, "approve"),
            Phase::Execute => write!(f, "execute"),
            Phase::Verify => write!(f, "verify"),
            Phase::Learn => write!(f, "learn"),
        }
    }
}

// ── Record ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub spec_ref: String,
    pub spec_sha256: String,
    pub status: SpecStatus,
    #[serde(default)]
    pub sync_status: SyncStatus,
    #[serde(default)]
    pub status_history: Vec<StatusEntry>,
    #[serde(default)]
    pub active_phase: Option<String>,
    #[serde(default)]
    pub phase_history: Vec<WorkflowPhaseRecord>,
    #[serde(default)]
    pub phase_runs: Vec<WorkflowPhaseRun>,
    #[serde(default)]
    pub approvals: Vec<Approval>,
    #[serde(default)]
    pub preview: Option<PreviewResult>,
    #[serde(default)]
    pub reflection: Option<Reflection>,
    #[serde(default)]
    pub gate_results: Vec<GateResult>,
    #[serde(default)]
    pub decision: Option<DecisionRecord>,
    #[serde(default)]
    pub verified_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SpecStatus {
    Proposed,
    InReview,
    Previewed,
    Approved,
    Applied,
    /// Lifecycle closed by production evidence — reflect was called with evidence
    Operated,
    Deprecated,
    Removed,
}

impl std::fmt::Display for SpecStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpecStatus::Proposed => write!(f, "proposed"),
            SpecStatus::InReview => write!(f, "in_review"),
            SpecStatus::Previewed => write!(f, "previewed"),
            SpecStatus::Approved => write!(f, "approved"),
            SpecStatus::Applied => write!(f, "applied"),
            SpecStatus::Operated => write!(f, "operated"),
            SpecStatus::Deprecated => write!(f, "deprecated"),
            SpecStatus::Removed => write!(f, "removed"),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    #[default]
    Unknown,
    Synced,
    Drifted,
    Degraded,
}

impl std::fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncStatus::Unknown => write!(f, "unknown"),
            SyncStatus::Synced => write!(f, "synced"),
            SyncStatus::Drifted => write!(f, "drifted"),
            SyncStatus::Degraded => write!(f, "degraded"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowPhaseStatus {
    Planned,
    Active,
    Verified,
    Accepted,
    RolledBack,
}

impl std::fmt::Display for WorkflowPhaseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowPhaseStatus::Planned => write!(f, "planned"),
            WorkflowPhaseStatus::Active => write!(f, "active"),
            WorkflowPhaseStatus::Verified => write!(f, "verified"),
            WorkflowPhaseStatus::Accepted => write!(f, "accepted"),
            WorkflowPhaseStatus::RolledBack => write!(f, "rolled_back"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEntry {
    pub status: SpecStatus,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPhaseRecord {
    pub phase_id: String,
    pub status: WorkflowPhaseStatus,
    pub timestamp: String,
    #[serde(default)]
    pub reviewer: Option<String>,
    #[serde(default)]
    pub engine: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPhaseRun {
    pub run_id: String,
    pub phase_id: String,
    pub engine: String,
    pub status: String,
    pub timestamp: String,
    #[serde(default)]
    pub completed_at: Option<String>,
    #[serde(default)]
    pub reasoning: Option<String>,
    #[serde(default)]
    pub exit_code: Option<i32>,
    pub output_path: String,
    pub output_sha256: String,
    #[serde(default)]
    pub stderr_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessState {
    pub schema: String,
    pub id: String,
    pub spec_ref: String,
    pub phase_id: String,
    pub source_root: String,
    pub worktree_path: String,
    pub branch: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub heartbeat_at: String,
    #[serde(default)]
    pub last_event: Option<String>,
    #[serde(default)]
    pub current_command: Option<HarnessCommandState>,
    #[serde(default)]
    pub events: Vec<HarnessEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessCommandState {
    pub argv: Vec<String>,
    pub pid: u32,
    pub started_at: String,
    pub timeout_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessEvent {
    pub timestamp: String,
    pub kind: String,
    pub message: String,
    #[serde(default)]
    pub command: Vec<String>,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub stdout_path: Option<String>,
    #[serde(default)]
    pub stderr_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Approval {
    pub reviewer: String,
    pub date: String,
    pub spec_sha256: String,
    pub diff_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewResult {
    pub diff_sha256: String,
    #[serde(default)]
    pub previewed_spec_sha256: String,
    pub actions: PreviewActions,
    #[serde(default)]
    pub log: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewActions {
    #[serde(default)]
    pub create: u32,
    #[serde(default)]
    pub link: u32,
    #[serde(default)]
    pub update: u32,
    #[serde(default)]
    pub delete: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    #[serde(default)]
    pub verdict: Option<String>,
    #[serde(default)]
    pub measurement_window: Option<String>,
    #[serde(default)]
    pub telemetry_refs: Vec<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub worked: Vec<String>,
    #[serde(default)]
    pub issues: Vec<String>,
    #[serde(default)]
    pub structured_issues: Vec<Issue>,
    #[serde(default)]
    pub follow_ups: Vec<FollowUp>,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    #[serde(default)]
    pub heuristics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub description: String,
    #[serde(default)]
    pub root_cause: Option<String>,
    #[serde(default)]
    pub fix: Option<String>,
    #[serde(default)]
    pub severity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowUp {
    pub id: String,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub path: String,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub formality: Option<u8>,
    #[serde(default)]
    pub claim_scope: Option<Vec<String>>,
    #[serde(default)]
    pub reliability: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub kind: String,
    pub status: String,
    #[serde(default)]
    pub attempt: u32,
    pub timestamp: String,
    #[serde(default)]
    pub log: Option<String>,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub evidence_path: Option<String>,
    #[serde(default)]
    pub command: Option<Vec<String>>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

// ── Config, Ledger, Index ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S5dConfig {
    pub schema: String,
    #[serde(default)]
    pub gate_commands: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub engines: HashMap<String, EngineConfig>,
    #[serde(default)]
    pub gate_runner: Option<GateRunner>,
    #[serde(default)]
    pub defaults: Option<Defaults>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    #[serde(default)]
    pub approved: bool,
    #[serde(default)]
    pub command: Vec<String>,
    #[serde(default)]
    pub reasoning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateRunner {
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub timeout_s: Option<u32>,
    #[serde(default)]
    pub env_inherit: Option<bool>,
    #[serde(default)]
    pub env_deny: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defaults {
    #[serde(default)]
    pub tier: Option<String>,
    #[serde(default)]
    pub evidence_retention_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ledger {
    pub schema: String,
    #[serde(default)]
    pub entries: Vec<LedgerEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub spec_sha256: String,
    pub state_fingerprint: String,
    pub package_id: String,
    pub action: String,
    pub status: String,
    pub timestamp: String,
    #[serde(default)]
    pub record_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    #[serde(default)]
    pub features: Vec<IndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub id: String,
    pub spec_path: String,
    pub status: SpecStatus,
    pub product: String,
    pub version: String,
}
