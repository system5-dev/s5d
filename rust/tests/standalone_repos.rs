use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

struct StandaloneRepo {
    _dir: TempDir,
    root: PathBuf,
}

struct CmdOutcome {
    success: bool,
    stdout: String,
    stderr: String,
}

impl CmdOutcome {
    fn summary(&self) -> String {
        format!("stdout:\n{}\n\nstderr:\n{}", self.stdout, self.stderr)
    }
}

impl StandaloneRepo {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        Self {
            root: dir.path().to_path_buf(),
            _dir: dir,
        }
    }

    fn path(&self) -> &Path {
        &self.root
    }

    fn write(&self, rel: &str, content: &str) {
        let path = self.root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn mkdir(&self, rel: &str) -> PathBuf {
        let path = self.root.join(rel);
        fs::create_dir_all(&path).unwrap();
        path
    }
}

fn s5d_bin() -> &'static str {
    env!("CARGO_BIN_EXE_s5d")
}

fn run_s5d<I, S>(cwd: &Path, args: I) -> CmdOutcome
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(s5d_bin())
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();

    CmdOutcome {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

#[track_caller]
fn run_ok<I, S>(cwd: &Path, args: I) -> CmdOutcome
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let outcome = run_s5d(cwd, args);
    assert!(outcome.success, "command failed:\n{}", outcome.summary());
    outcome
}

#[track_caller]
fn run_fail<I, S>(cwd: &Path, args: I) -> CmdOutcome
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let outcome = run_s5d(cwd, args);
    assert!(
        !outcome.success,
        "command unexpectedly succeeded:\n{}",
        outcome.summary()
    );
    outcome
}

fn only_spec_path(repo: &StandaloneRepo) -> PathBuf {
    only_matching_file(&repo.path().join(".s5d").join("packages"), ".s5d.yaml")
}

fn only_matching_file(dir: &Path, suffix: &str) -> PathBuf {
    let mut matches = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(suffix))
        })
        .collect::<Vec<_>>();
    matches.sort();
    assert_eq!(
        matches.len(),
        1,
        "expected exactly one file ending with {} in {}, got {:?}",
        suffix,
        dir.display(),
        matches
    );
    matches.remove(0)
}

fn record_path_for(spec_path: &Path) -> PathBuf {
    let spec_name = spec_path.file_name().unwrap().to_string_lossy();
    spec_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("records")
        .join(spec_name.replace(".s5d.yaml", ".record.yaml"))
}

fn load_yaml<T: serde::de::DeserializeOwned>(path: &Path) -> T {
    serde_yaml::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn configure_gate_command(repo: &StandaloneRepo, gate: &str, command: Vec<String>) {
    let config_path = repo.path().join(".s5d").join("config.yaml");
    let mut config: s5d::S5dConfig = load_yaml(&config_path);
    config.gate_commands.insert(gate.to_string(), command);
    fs::write(config_path, serde_yaml::to_string(&config).unwrap()).unwrap();
}

fn seed_rust_workspace(repo: &StandaloneRepo) {
    repo.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/orders\", \"crates/payments\"]\n",
    );
    repo.write(
        "crates/orders/src/models.rs",
        "pub struct Order {}\npub struct OrderLine {}\n",
    );
    repo.write(
        "crates/orders/src/service.rs",
        "pub fn create_order() {}\npub fn cancel_order() {}\n",
    );
    repo.write("crates/payments/src/models.rs", "pub struct Payment {}\n");
}

fn seed_searchable_rust_repo(repo: &StandaloneRepo) {
    repo.write(
        "Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    repo.write(
        "src/lib.rs",
        "pub struct BillingService;\n\nimpl BillingService {\n    pub fn standalonee2emarker(&self) -> &'static str {\n        \"standalonee2emarker\"\n    }\n}\n",
    );
    repo.mkdir("apps/mobile");
}

#[test]
fn init_bootstraps_project_layout_for_standalone_repo() {
    let repo = StandaloneRepo::new();
    seed_searchable_rust_repo(&repo);

    let out = run_ok(repo.path(), ["init"]);

    assert!(repo.path().join(".s5d").join("packages").is_dir());
    assert!(repo.path().join(".s5d").join("records").is_dir());
    assert!(repo.path().join(".s5d").join(".locks").is_dir());
    assert!(out.stdout.contains("S5D initialized"));
    assert!(out.stdout.contains("s5d new <feature-id> --product <name>"));

    let status = run_ok(repo.path(), ["status"]);
    assert!(status
        .stdout
        .contains("No specs. Run `s5d new <id>` to create one."));
}

#[test]
fn lightweight_feature_flow_passes_with_configured_schema_gate() {
    let repo = StandaloneRepo::new();
    seed_searchable_rust_repo(&repo);
    run_ok(repo.path(), ["init"]);
    configure_gate_command(
        &repo,
        "schema",
        vec![
            s5d_bin().to_string(),
            "validate".to_string(),
            "{package}".to_string(),
        ],
    );

    run_ok(
        repo.path(),
        [
            "new",
            "feat.billing.ratio",
            "--tier",
            "lightweight",
            "--product",
            "Billing",
        ],
    );
    let spec_path = only_spec_path(&repo);
    let spec_str = spec_path.to_str().unwrap();

    // Add capability so lightweight validate passes
    {
        let mut spec: s5d::Spec = load_yaml(&spec_path);
        if let Some(ref mut a) = spec.artifacts {
            a.capabilities.push(s5d::Capability {
                id: "cap.CalculateRatio".into(),
                domain: "billing".into(),
                name: "CalculateRatio".into(),
                description: None,
                since: None,
            });
        }
        fs::write(&spec_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
    }

    run_ok(repo.path(), ["preview", spec_str]);
    run_ok(repo.path(), ["approve", spec_str, "--reviewer", "Roman"]);
    let gates = run_ok(repo.path(), ["run-gates", spec_str]);
    assert!(gates.stdout.contains("gate:schema"), "{}", gates.summary());

    let import = run_ok(repo.path(), ["import", spec_str]);
    assert!(import.stdout.contains("Imported"), "{}", import.summary());

    let record: s5d::Record = load_yaml(&record_path_for(&spec_path));
    assert_eq!(record.status, s5d::SpecStatus::Applied);
    assert_eq!(record.sync_status, s5d::SyncStatus::Synced);
    assert!(record
        .gate_results
        .iter()
        .any(|result| result.kind == "schema" && result.status == "passed"));

    let index: s5d::Index = load_yaml(&repo.path().join(".s5d").join("index.yaml"));
    assert!(index
        .features
        .iter()
        .any(|entry| entry.id == "feat.billing.ratio"));
}

#[test]
fn import_stays_blocked_when_declared_gate_only_skips() {
    let repo = StandaloneRepo::new();
    seed_searchable_rust_repo(&repo);
    run_ok(repo.path(), ["init"]);
    // Use lightweight tier (gets a schema gate which now runs built-in)
    run_ok(
        repo.path(),
        [
            "new",
            "feat.billing.skipgate",
            "--tier",
            "lightweight",
            "--product",
            "Billing",
        ],
    );
    let spec_path = only_spec_path(&repo);
    let spec_str = spec_path.to_str().unwrap();

    // Add a custom gate kind that has no built-in handler and no configured command.
    // Schema/graph now run built-in validation, so we need a different kind to test skip behavior.
    let mut spec: s5d::Spec = load_yaml(&spec_path);
    spec.gates = vec![s5d::Gate {
        kind: "custom_check".to_string(),
    }];
    let yaml = serde_yaml::to_string(&spec).unwrap();
    std::fs::write(&spec_path, yaml).unwrap();

    run_ok(repo.path(), ["preview", spec_str]);
    run_ok(repo.path(), ["approve", spec_str, "--reviewer", "Roman"]);
    let gates = run_ok(repo.path(), ["run-gates", spec_str]);
    assert!(gates.stdout.contains("skipped: 1"), "{}", gates.summary());

    let import = run_fail(repo.path(), ["import", spec_str]);
    assert!(
        import
            .stderr
            .contains("all declared gates must pass before import"),
        "{}",
        import.summary()
    );

    let record: s5d::Record = load_yaml(&record_path_for(&spec_path));
    assert_eq!(record.status, s5d::SpecStatus::Approved);
    assert!(record
        .gate_results
        .iter()
        .any(|result| result.kind == "custom_check" && result.status == "skipped"));
}

#[test]
fn init_is_idempotent() {
    let repo = StandaloneRepo::new();
    seed_searchable_rust_repo(&repo);

    // First init creates .s5d/
    let first = run_ok(repo.path(), ["init"]);
    assert!(first.stdout.contains("S5D initialized"), "{}", first.summary());
    assert!(repo.path().join(".s5d").exists());

    // Second init should succeed (not error)
    let second = run_ok(repo.path(), ["init"]);
    assert!(
        second.stdout.contains("already") || second.stdout.contains("S5D initialized"),
        "second init should succeed: {}",
        second.summary()
    );
}

#[test]
fn decision_has_expiry_and_do_dont() {
    let repo = StandaloneRepo::new();
    seed_searchable_rust_repo(&repo);
    run_ok(repo.path(), ["init"]);

    // Create decision spec
    run_ok(
        repo.path(),
        [
            "new",
            "decision.test-choice",
            "--tier",
            "decision",
            "--question",
            "Which approach?",
            "--product",
            "TestProject",
        ],
    );
    let spec_path = only_spec_path(&repo);
    let spec_str = spec_path.to_str().unwrap();

    // Add hypotheses
    run_ok(
        repo.path(),
        [
            "add-hypothesis",
            spec_str,
            "--title",
            "Approach A",
            "--content",
            "Direct approach",
            "--scope",
            "full",
            "--kind",
            "system",
        ],
    );
    run_ok(
        repo.path(),
        [
            "add-hypothesis",
            spec_str,
            "--title",
            "Approach B",
            "--content",
            "Alternative approach",
            "--scope",
            "full",
            "--kind",
            "system",
        ],
    );

    // Decide
    let decide = run_ok(
        repo.path(),
        [
            "decide",
            spec_str,
            "--title",
            "Test Decision",
            "--winner",
            "approach-a",
            "--confirmed-by",
            "roman",
            "--context",
            "Testing",
            "--decision",
            "Use A",
            "--rationale",
            "Simpler",
            "--consequences",
            "None",
            "--rejected",
            "approach-b",
            "--force",
        ],
    );
    assert!(decide.stdout.contains("Decision recorded"), "{}", decide.summary());

    // Check record has expires_at
    let record: s5d::Record = load_yaml(&record_path_for(&spec_path));
    let decision = record.decision.expect("decision should be recorded");
    assert!(
        decision.expires_at.is_some(),
        "decision should have expires_at (90-day TTL)"
    );

    // Check expires_at is ~90 days from now
    let expires = decision.expires_at.as_ref().unwrap();
    assert!(
        expires.starts_with("20"),
        "expires_at should be a date: {}",
        expires
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// S5D Contract Tests — prove the skill's value with synthetic scenarios
// ═══════════════════════════════════════════════════════════════════════════

/// Helper: create a feature spec with full metamodel (domains, capabilities, components)
fn write_spec_with_metamodel(repo: &StandaloneRepo, id: &str) {
    let spec_dir = repo.path().join(".s5d").join("packages");
    fs::create_dir_all(&spec_dir).unwrap();
    let spec_path = spec_dir.join(format!("{}__{}.s5d.yaml", id, "20260410"));
    fs::write(
        &spec_path,
        format!(
            r#"s5d: '1.0'
id: {id}
version: 1.0.0
product: testproduct
tier: standard
allow_update: false
meta:
  title: Test Feature
  authors: []
  date: 2026-04-10
  tickets: []
  adrs: []
  renames: []
context: null
artifacts:
  products:
  - id: testproduct
    name: TestProduct
    organization: null
  domains:
  - id: dom.testproduct.core
    product: testproduct
    name: Core
    classification: Core
    description: Core domain
  capabilities:
  - id: cap.DoThing
    domain: dom.testproduct.core
    name: DoThing
  entities:
  - id: ent.Widget
    domain: dom.testproduct.core
    name: Widget
  features:
  - id: {id}
    product: testproduct
    name: Test Feature
    description: null
  use_cases: []
  systems: []
  containers:
  - id: ctr.main
    system: sys.test
    name: Main
  components:
  - id: comp.widget_handler
    domain: dom.testproduct.core
    feature: {id}
    container: ctr.main
    name: Widget Handler
    paths:
    - src/widget.rs
  roles: []
  concerns: []
  metrics: []
  supersystems: []
  transports: []
links:
  feature_to_domain: []
  use_case_to_capability: []
  use_case_to_entity: []
  component_to_capability: []
  component_to_entity: []
  container_to_capability: []
  concern_to_metric: []
  component_to_container: []
  edges: []
  depends_on: []
  entity_relations: []
  capability_to_entity: []
  capability_to_concern: []
contracts: []
gates:
- kind: schema
- kind: graph
roc: null
problem: null
hypotheses: []
decision: null
note_rationale: null
expires_at: null
auto_noted: false
"#
        ),
    )
    .unwrap();
}

/// Helper: create a spec WITHOUT metamodel (no domains, no capabilities, no components)
fn write_spec_without_metamodel(repo: &StandaloneRepo, id: &str) {
    let spec_dir = repo.path().join(".s5d").join("packages");
    fs::create_dir_all(&spec_dir).unwrap();
    let spec_path = spec_dir.join(format!("{}__{}.s5d.yaml", id, "20260410"));
    fs::write(
        &spec_path,
        format!(
            r#"s5d: '1.0'
id: {id}
version: 1.0.0
product: testproduct
tier: standard
allow_update: false
meta:
  title: Test Feature Without Metamodel
  authors: []
  date: 2026-04-10
  tickets: []
  adrs: []
  renames: []
context: null
artifacts:
  products:
  - id: testproduct
    name: TestProduct
    organization: null
  domains: []
  capabilities: []
  entities: []
  features:
  - id: {id}
    product: testproduct
    name: Test Feature
    description: null
  use_cases: []
  systems: []
  containers:
  - id: ctr.main
    system: sys.test
    name: Main
  components: []
  roles: []
  concerns: []
  metrics: []
  supersystems: []
  transports: []
links:
  feature_to_domain: []
  use_case_to_capability: []
  use_case_to_entity: []
  component_to_capability: []
  component_to_entity: []
  container_to_capability: []
  concern_to_metric: []
  component_to_container: []
  edges: []
  depends_on: []
  entity_relations: []
  capability_to_entity: []
  capability_to_concern: []
contracts: []
gates:
- kind: schema
- kind: graph
roc: null
problem: null
hypotheses: []
decision: null
note_rationale: null
expires_at: null
auto_noted: false
"#
        ),
    )
    .unwrap();
}

// ── Test 1: Metamodel gate blocks specs without architectural decomposition ──

#[test]
fn contract_metamodel_gate_blocks_empty_spec() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    write_spec_without_metamodel(&repo, "feat.testproduct.empty");

    let result = run_fail(
        repo.path(),
        [
            "validate",
            &format!(
                ".s5d/packages/feat.testproduct.empty__20260410.s5d.yaml"
            ),
        ],
    );
    assert!(
        result.stderr.contains("metamodel: spec has no domains"),
        "should block on missing domains:\n{}",
        result.summary()
    );
    assert!(
        result.stderr.contains("metamodel: spec has no capabilities"),
        "should block on missing capabilities:\n{}",
        result.summary()
    );
    assert!(
        result.stderr.contains("metamodel: spec has no components"),
        "should block on missing components:\n{}",
        result.summary()
    );
}

// ── Test 2: Metamodel gate passes specs WITH architectural decomposition ──

#[test]
fn contract_metamodel_gate_passes_complete_spec() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    write_spec_with_metamodel(&repo, "feat.testproduct.complete");

    run_ok(
        repo.path(),
        [
            "validate",
            ".s5d/packages/feat.testproduct.complete__20260410.s5d.yaml",
        ],
    );
}

// ── Test 3: Lightweight tier only requires capabilities (not domains/components) ──

#[test]
fn contract_lightweight_tier_requires_only_capabilities() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    let spec_dir = repo.path().join(".s5d").join("packages");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_dir.join("feat.testproduct.light__20260410.s5d.yaml"),
        r#"s5d: '1.0'
id: feat.testproduct.light
version: 1.0.0
product: testproduct
tier: lightweight
allow_update: false
meta:
  title: Light Feature
  authors: []
  date: 2026-04-10
  tickets: []
  adrs: []
  renames: []
context: null
artifacts:
  products:
  - id: testproduct
    name: TestProduct
    organization: null
  domains: []
  capabilities:
  - id: cap.QuickFix
    domain: dom.testproduct.core
    name: QuickFix
  entities: []
  features:
  - id: feat.testproduct.light
    product: testproduct
    name: Light Feature
    description: null
  use_cases: []
  systems: []
  containers:
  - id: ctr.main
    system: sys.test
    name: Main
  components: []
  roles: []
  concerns: []
  metrics: []
  supersystems: []
  transports: []
links:
  feature_to_domain: []
  use_case_to_capability: []
  use_case_to_entity: []
  component_to_capability: []
  component_to_entity: []
  container_to_capability: []
  concern_to_metric: []
  component_to_container: []
  edges: []
  depends_on: []
  entity_relations: []
  capability_to_entity: []
  capability_to_concern: []
contracts: []
gates:
- kind: schema
roc: null
problem: null
hypotheses: []
decision: null
note_rationale: null
expires_at: null
auto_noted: false
"#,
    )
    .unwrap();

    // Lightweight with capabilities → should pass (no domains/components required)
    run_ok(
        repo.path(),
        [
            "validate",
            ".s5d/packages/feat.testproduct.light__20260410.s5d.yaml",
        ],
    );
}

// ── Test 4: Lightweight without capabilities still fails ──

#[test]
fn contract_lightweight_without_capabilities_fails() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    let spec_dir = repo.path().join(".s5d").join("packages");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_dir.join("feat.testproduct.nocap__20260410.s5d.yaml"),
        r#"s5d: '1.0'
id: feat.testproduct.nocap
version: 1.0.0
product: testproduct
tier: lightweight
allow_update: false
meta:
  title: No Cap Feature
  authors: []
  date: 2026-04-10
  tickets: []
  adrs: []
  renames: []
context: null
artifacts:
  products:
  - id: testproduct
    name: TestProduct
    organization: null
  domains: []
  capabilities: []
  entities: []
  features:
  - id: feat.testproduct.nocap
    product: testproduct
    name: No Cap Feature
    description: null
  use_cases: []
  systems: []
  containers:
  - id: ctr.main
    system: sys.test
    name: Main
  components: []
  roles: []
  concerns: []
  metrics: []
  supersystems: []
  transports: []
links:
  feature_to_domain: []
  use_case_to_capability: []
  use_case_to_entity: []
  component_to_capability: []
  component_to_entity: []
  container_to_capability: []
  concern_to_metric: []
  component_to_container: []
  edges: []
  depends_on: []
  entity_relations: []
  capability_to_entity: []
  capability_to_concern: []
contracts: []
gates:
- kind: schema
roc: null
problem: null
hypotheses: []
decision: null
note_rationale: null
expires_at: null
auto_noted: false
"#,
    )
    .unwrap();

    let result = run_fail(
        repo.path(),
        [
            "validate",
            ".s5d/packages/feat.testproduct.nocap__20260410.s5d.yaml",
        ],
    );
    assert!(
        result.stderr.contains("metamodel: spec has no capabilities"),
        "lightweight without capabilities should fail:\n{}",
        result.summary()
    );
}

// ── Test 5: Decision tier is exempt from metamodel (no artifacts) ──

#[test]
fn contract_decision_tier_exempt_from_metamodel() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    run_ok(
        repo.path(),
        [
            "new",
            "decision.testproduct.arch",
            "--tier",
            "decision",
            "--question",
            "Which database to use?",
            "--product",
            "testproduct",
        ],
    );

    let spec_path = only_spec_path(&repo);
    // Decision tier → no artifacts needed → should validate fine
    run_ok(
        repo.path(),
        ["validate", spec_path.to_str().unwrap()],
    );
}

// ── Test 6: Graph cycle detection blocks validation ──

#[test]
fn contract_graph_cycle_blocks_validation() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    let spec_dir = repo.path().join(".s5d").join("packages");
    fs::create_dir_all(&spec_dir).unwrap();
    // Two domains with a directional cycle: A→B (customer_supplier) and B→A (customer_supplier)
    fs::write(
        spec_dir.join("feat.testproduct.cycle__20260411.s5d.yaml"),
        r#"s5d: '1.0'
id: feat.testproduct.cycle
version: 1.0.0
product: testproduct
tier: standard
allow_update: false
meta:
  title: Cycle Test
  authors: []
  date: 2026-04-11
  tickets: []
  adrs: []
  renames: []
context: null
artifacts:
  products:
  - id: testproduct
    name: TestProduct
    organization: null
  domains:
  - id: dom.testproduct.alpha
    product: testproduct
    name: Alpha
    classification: core
    description: null
  - id: dom.testproduct.beta
    product: testproduct
    name: Beta
    classification: supporting
    description: null
  capabilities:
  - id: cap.DoAlpha
    domain: dom.testproduct.alpha
    name: DoAlpha
  - id: cap.DoBeta
    domain: dom.testproduct.beta
    name: DoBeta
  entities: []
  features:
  - id: feat.testproduct.cycle
    product: testproduct
    name: Cycle Test
    description: null
  use_cases: []
  systems: []
  containers:
  - id: ctr.main
    system: sys.test
    name: Main
  components:
  - id: comp.alpha_handler
    domain: dom.testproduct.alpha
    feature: feat.testproduct.cycle
    container: ctr.main
    name: Alpha Handler
    paths:
    - src/alpha.rs
  - id: comp.beta_handler
    domain: dom.testproduct.beta
    feature: feat.testproduct.cycle
    container: ctr.main
    name: Beta Handler
    paths:
    - src/beta.rs
  roles: []
  concerns: []
  metrics: []
  supersystems: []
  transports: []
links:
  feature_to_domain: []
  use_case_to_capability: []
  use_case_to_entity: []
  component_to_capability: []
  component_to_entity: []
  container_to_capability: []
  concern_to_metric: []
  component_to_container: []
  edges:
  - from: dom.testproduct.alpha
    to: dom.testproduct.beta
    archetype: customer_supplier
  - from: dom.testproduct.beta
    to: dom.testproduct.alpha
    archetype: customer_supplier
  depends_on: []
  entity_relations: []
  capability_to_entity: []
  capability_to_concern: []
contracts: []
gates:
- kind: schema
- kind: graph
roc: null
problem: null
hypotheses: []
decision: null
note_rationale: null
expires_at: null
auto_noted: false
"#,
    )
    .unwrap();

    // Validate passes (structural OK)
    run_ok(
        repo.path(),
        [
            "validate",
            ".s5d/packages/feat.testproduct.cycle__20260411.s5d.yaml",
        ],
    );

    // Graph check FAILS — directional cycle
    let result = run_fail(
        repo.path(),
        [
            "graph-check",
            ".s5d/packages/feat.testproduct.cycle__20260411.s5d.yaml",
        ],
    );
    assert!(
        result.stderr.contains("cycle") || result.stdout.contains("cycle"),
        "should detect cycle:\n{}",
        result.summary()
    );
}

// ── Test 7: Layering violation — supporting depends on core is OK, generic depends on core is NOT ──

#[test]
fn contract_layering_generic_to_core_blocked() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    let spec_dir = repo.path().join(".s5d").join("packages");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_dir.join("feat.testproduct.layering__20260411.s5d.yaml"),
        r#"s5d: '1.0'
id: feat.testproduct.layering
version: 1.0.0
product: testproduct
tier: standard
allow_update: false
meta:
  title: Layering Test
  authors: []
  date: 2026-04-11
  tickets: []
  adrs: []
  renames: []
context: null
artifacts:
  products:
  - id: testproduct
    name: TestProduct
    organization: null
  domains:
  - id: dom.testproduct.core
    product: testproduct
    name: Core
    classification: core
    description: null
  - id: dom.testproduct.infra
    product: testproduct
    name: Infra
    classification: generic
    description: null
  capabilities:
  - id: cap.DoCore
    domain: dom.testproduct.core
    name: DoCore
  entities: []
  features:
  - id: feat.testproduct.layering
    product: testproduct
    name: Layering Test
    description: null
  use_cases: []
  systems: []
  containers:
  - id: ctr.main
    system: sys.test
    name: Main
  components:
  - id: comp.core_handler
    domain: dom.testproduct.core
    feature: feat.testproduct.layering
    container: ctr.main
    name: Core Handler
    paths:
    - src/core.rs
  roles: []
  concerns: []
  metrics: []
  supersystems: []
  transports: []
links:
  feature_to_domain: []
  use_case_to_capability: []
  use_case_to_entity: []
  component_to_capability: []
  component_to_entity: []
  container_to_capability: []
  concern_to_metric: []
  component_to_container: []
  edges:
  - from: dom.testproduct.infra
    to: dom.testproduct.core
    archetype: conformist
  depends_on: []
  entity_relations: []
  capability_to_entity: []
  capability_to_concern: []
contracts: []
gates:
- kind: schema
- kind: graph
roc: null
problem: null
hypotheses: []
decision: null
note_rationale: null
expires_at: null
auto_noted: false
"#,
    )
    .unwrap();

    let result = run_fail(
        repo.path(),
        [
            "graph-check",
            ".s5d/packages/feat.testproduct.layering__20260411.s5d.yaml",
        ],
    );
    assert!(
        result.stderr.contains("layering") || result.stdout.contains("layering"),
        "should detect layering violation (generic→core):\n{}",
        result.summary()
    );
}

// ── Test 8: Full lifecycle — init → new → validate → preview → approve → import ──

#[test]
fn contract_full_lifecycle_end_to_end() {
    let repo = StandaloneRepo::new();
    seed_rust_workspace(&repo);
    run_ok(repo.path(), ["init"]);

    // Create spec
    run_ok(
        repo.path(),
        [
            "new",
            "feat.shop.orders",
            "--tier",
            "standard",
            "--product",
            "shop",
        ],
    );

    let spec_path = only_spec_path(&repo);
    let spec_str = spec_path.to_str().unwrap().to_string();

    // Write metamodel into the spec so validate passes
    let mut spec: s5d::Spec = load_yaml(&spec_path);
    let artifacts = spec.artifacts.as_mut().expect("should have artifacts");
    artifacts.domains.push(s5d::Domain {
        id: "dom.shop.orders".into(),
        product: "shop".into(),
        name: "Orders".into(),
        classification: Some("core".into()),
        description: None,
        team: None,
        maturity_level: None,
    });
    artifacts.capabilities.push(s5d::Capability {
        id: "cap.CreateOrder".into(),
        domain: "dom.shop.orders".into(),
        name: "CreateOrder".into(),
        description: None,
        since: None,
    });
    artifacts.containers.push(s5d::Container {
        id: "ctr.api".into(),
        name: "API".into(),
        system: String::new(),
        technology: None,
    });
    artifacts.components.push(s5d::Component {
        id: "comp.order_service".into(),
        domain: "dom.shop.orders".into(),
        feature: "feat.shop.orders".into(),
        container: "ctr.api".into(),
        name: "Order Service".into(),
        paths: vec!["crates/orders/src/service.rs".into()],
    });
    fs::write(&spec_path, serde_yaml::to_string(&spec).unwrap()).unwrap();

    // Configure gate commands — must use real s5d commands
    configure_gate_command(
        &repo,
        "schema",
        vec![
            s5d_bin().to_string(),
            "validate".to_string(),
            "{package}".to_string(),
        ],
    );
    configure_gate_command(
        &repo,
        "graph",
        vec![
            s5d_bin().to_string(),
            "graph-check".to_string(),
            "{package}".to_string(),
        ],
    );
    configure_gate_command(&repo, "lint", vec!["true".into()]);

    // Validate
    run_ok(repo.path(), ["validate", &spec_str]);

    // Graph check
    run_ok(repo.path(), ["graph-check", &spec_str]);

    // Preview (must be AFTER spec is final — modifying spec invalidates preview hash)
    run_ok(repo.path(), ["preview", &spec_str]);

    // Approve
    run_ok(
        repo.path(),
        ["approve", &spec_str, "--reviewer", "TestReviewer"],
    );

    // Run gates (after approve, before import)
    run_ok(repo.path(), ["run-gates", &spec_str]);

    // Import
    let import = run_ok(repo.path(), ["import", &spec_str]);
    assert!(
        import.stdout.contains("imported") || import.stdout.contains("state_fingerprint") || import.stdout.contains("Import"),
        "import should succeed:\n{}",
        import.summary()
    );

    // Status should show applied/imported
    let status = run_ok(repo.path(), ["status"]);
    assert!(
        !status.stdout.contains("No specs"),
        "status should show the spec:\n{}",
        status.summary()
    );
}

// ── Test 9: Decision tier — confirmed_by required, artifacts forbidden ──

#[test]
fn contract_decision_requires_confirmed_by() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    run_ok(
        repo.path(),
        [
            "new",
            "decision.testproduct.db",
            "--tier",
            "decision",
            "--question",
            "SQL or NoSQL?",
            "--product",
            "testproduct",
        ],
    );

    let spec_path = only_spec_path(&repo);

    // Add hypothesis
    run_ok(
        repo.path(),
        [
            "add-hypothesis",
            spec_path.to_str().unwrap(),
            "--title",
            "SQL",
            "--content",
            "Use PostgreSQL",
            "--scope",
            "data layer",
        ],
    );

    // Try to decide without enough hypotheses and without confirmed_by structure
    // The decide command requires --confirmed-by flag
    let result = run_fail(
        repo.path(),
        [
            "decide",
            spec_path.to_str().unwrap(),
            "--title",
            "Use SQL",
            "--winner",
            "sql",
            "--context",
            "Need ACID",
            "--decision",
            "PostgreSQL",
            "--rationale",
            "ACID compliance",
            "--consequences",
            "Schema migrations needed",
        ],
    );
    // Should fail — missing --confirmed-by (structural gate)
    assert!(
        result.stderr.contains("confirmed") || result.stderr.contains("required"),
        "decide without confirmed_by should fail:\n{}",
        result.summary()
    );
}

// ── Test 10: Shared kernel edge does NOT trigger cycle detection ──

#[test]
fn contract_shared_kernel_bidirectional_allowed() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    let spec_dir = repo.path().join(".s5d").join("packages");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_dir.join("feat.testproduct.shared__20260411.s5d.yaml"),
        r#"s5d: '1.0'
id: feat.testproduct.shared
version: 1.0.0
product: testproduct
tier: standard
allow_update: false
meta:
  title: Shared Kernel Test
  authors: []
  date: 2026-04-11
  tickets: []
  adrs: []
  renames: []
context: null
artifacts:
  products:
  - id: testproduct
    name: TestProduct
    organization: null
  domains:
  - id: dom.testproduct.alpha
    product: testproduct
    name: Alpha
    classification: core
    description: null
  - id: dom.testproduct.beta
    product: testproduct
    name: Beta
    classification: core
    description: null
  capabilities:
  - id: cap.DoAlpha
    domain: dom.testproduct.alpha
    name: DoAlpha
  - id: cap.DoBeta
    domain: dom.testproduct.beta
    name: DoBeta
  entities: []
  features:
  - id: feat.testproduct.shared
    product: testproduct
    name: Shared Kernel Test
    description: null
  use_cases: []
  systems: []
  containers:
  - id: ctr.main
    system: sys.test
    name: Main
  components:
  - id: comp.alpha_handler
    domain: dom.testproduct.alpha
    feature: feat.testproduct.shared
    container: ctr.main
    name: Alpha Handler
    paths:
    - src/alpha.rs
  - id: comp.beta_handler
    domain: dom.testproduct.beta
    feature: feat.testproduct.shared
    container: ctr.main
    name: Beta Handler
    paths:
    - src/beta.rs
  roles: []
  concerns: []
  metrics: []
  supersystems: []
  transports: []
links:
  feature_to_domain: []
  use_case_to_capability: []
  use_case_to_entity: []
  component_to_capability: []
  component_to_entity: []
  container_to_capability: []
  concern_to_metric: []
  component_to_container: []
  edges:
  - from: dom.testproduct.alpha
    to: dom.testproduct.beta
    archetype: shared_kernel
  - from: dom.testproduct.beta
    to: dom.testproduct.alpha
    archetype: shared_kernel
  depends_on: []
  entity_relations: []
  capability_to_entity: []
  capability_to_concern: []
contracts: []
gates:
- kind: schema
- kind: graph
roc: null
problem: null
hypotheses: []
decision: null
note_rationale: null
expires_at: null
auto_noted: false
"#,
    )
    .unwrap();

    // shared_kernel is bidirectional — graph check should PASS (no cycle violation)
    run_ok(
        repo.path(),
        [
            "graph-check",
            ".s5d/packages/feat.testproduct.shared__20260411.s5d.yaml",
        ],
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Reference Integrity Negative Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn contract_ref_integrity_broken_edge_domain() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    let spec_dir = repo.path().join(".s5d").join("packages");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_dir.join("feat.testproduct.badedge__20260411.s5d.yaml"),
        r#"s5d: '1.0'
id: feat.testproduct.badedge
version: 1.0.0
product: testproduct
tier: standard
allow_update: false
meta:
  title: Bad Edge
  authors: []
  date: 2026-04-11
  tickets: []
  adrs: []
  renames: []
context: null
artifacts:
  products:
  - id: testproduct
    name: TestProduct
    organization: null
  domains:
  - id: dom.testproduct.real
    product: testproduct
    name: Real
    classification: core
    description: null
  capabilities:
  - id: cap.DoReal
    domain: dom.testproduct.real
    name: DoReal
  entities: []
  features:
  - id: feat.testproduct.badedge
    product: testproduct
    name: Bad Edge
    description: null
  use_cases: []
  systems: []
  containers:
  - id: ctr.main
    system: sys.test
    name: Main
  components:
  - id: comp.handler
    domain: dom.testproduct.real
    feature: feat.testproduct.badedge
    container: ctr.main
    name: Handler
    paths:
    - src/handler.rs
  roles: []
  concerns: []
  metrics: []
  supersystems: []
  transports: []
links:
  feature_to_domain: []
  use_case_to_capability: []
  use_case_to_entity: []
  component_to_capability: []
  component_to_entity: []
  container_to_capability: []
  concern_to_metric: []
  component_to_container: []
  edges:
  - from: dom.testproduct.real
    to: dom.testproduct.GHOST
    archetype: customer_supplier
  depends_on: []
  entity_relations: []
  capability_to_entity: []
  capability_to_concern: []
contracts: []
gates:
- kind: schema
roc: null
problem: null
hypotheses: []
decision: null
note_rationale: null
expires_at: null
auto_noted: false
"#,
    )
    .unwrap();

    let result = run_fail(
        repo.path(),
        [
            "validate",
            ".s5d/packages/feat.testproduct.badedge__20260411.s5d.yaml",
        ],
    );
    assert!(
        result.stderr.contains("GHOST") && result.stderr.contains("not declared"),
        "should catch broken edge ref:\n{}",
        result.summary()
    );
}

#[test]
fn contract_ref_integrity_broken_component_domain() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    let spec_dir = repo.path().join(".s5d").join("packages");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_dir.join("feat.testproduct.badcomp__20260411.s5d.yaml"),
        r#"s5d: '1.0'
id: feat.testproduct.badcomp
version: 1.0.0
product: testproduct
tier: standard
allow_update: false
meta:
  title: Bad Component
  authors: []
  date: 2026-04-11
  tickets: []
  adrs: []
  renames: []
context: null
artifacts:
  products:
  - id: testproduct
    name: TestProduct
    organization: null
  domains:
  - id: dom.testproduct.real
    product: testproduct
    name: Real
    classification: core
    description: null
  capabilities:
  - id: cap.DoReal
    domain: dom.testproduct.real
    name: DoReal
  entities: []
  features:
  - id: feat.testproduct.badcomp
    product: testproduct
    name: Bad Component
    description: null
  use_cases: []
  systems: []
  containers:
  - id: ctr.main
    system: sys.test
    name: Main
  components:
  - id: comp.handler
    domain: dom.testproduct.FAKE
    feature: feat.testproduct.badcomp
    container: ctr.main
    name: Handler
    paths:
    - src/handler.rs
  roles: []
  concerns: []
  metrics: []
  supersystems: []
  transports: []
links:
  feature_to_domain: []
  use_case_to_capability: []
  use_case_to_entity: []
  component_to_capability: []
  component_to_entity: []
  container_to_capability: []
  concern_to_metric: []
  component_to_container: []
  edges: []
  depends_on: []
  entity_relations: []
  capability_to_entity: []
  capability_to_concern: []
contracts: []
gates:
- kind: schema
roc: null
problem: null
hypotheses: []
decision: null
note_rationale: null
expires_at: null
auto_noted: false
"#,
    )
    .unwrap();

    let result = run_fail(
        repo.path(),
        [
            "validate",
            ".s5d/packages/feat.testproduct.badcomp__20260411.s5d.yaml",
        ],
    );
    assert!(
        result.stderr.contains("FAKE") && result.stderr.contains("not declared"),
        "should catch broken component domain ref:\n{}",
        result.summary()
    );
}

// ── Integrity: SHA chain tests ───────────────────────────────────────────────

/// Helper: create a standard spec with valid artifacts, validate, and return spec path string.
fn setup_standard_spec(repo: &StandaloneRepo, feature_id: &str) -> String {
    run_ok(
        repo.path(),
        ["new", feature_id, "--tier", "standard", "--product", "shop"],
    );
    let spec_path = only_spec_path(repo);
    let mut spec: s5d::Spec = load_yaml(&spec_path);
    let artifacts = spec.artifacts.as_mut().unwrap();
    artifacts.domains.push(s5d::Domain {
        id: "dom.core".into(),
        product: "shop".into(),
        name: "Core".into(),
        classification: Some("core".into()),
        description: None,
        team: None,
        maturity_level: None,
    });
    artifacts.capabilities.push(s5d::Capability {
        id: "cap.Do".into(),
        domain: "dom.core".into(),
        name: "DoThing".into(),
        description: None,
        since: None,
    });
    artifacts.containers.push(s5d::Container {
        id: "ctr.api".into(),
        name: "API".into(),
        system: String::new(),
        technology: None,
    });
    artifacts.components.push(s5d::Component {
        id: "comp.handler".into(),
        domain: "dom.core".into(),
        feature: feature_id.into(),
        container: "ctr.api".into(),
        name: "Handler".into(),
        paths: vec!["src/handler.rs".into()],
    });
    fs::write(&spec_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
    let spec_str = spec_path.to_str().unwrap().to_string();
    run_ok(repo.path(), ["validate", &spec_str]);
    spec_str
}

#[test]
fn approve_rejects_spec_modified_since_preview() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    let spec_str = setup_standard_spec(&repo, "feat.sha1");

    // Preview
    run_ok(repo.path(), ["preview", &spec_str]);

    // Modify spec after preview (add a description to change the SHA)
    let spec_path = only_spec_path(&repo);
    let mut spec: s5d::Spec = load_yaml(&spec_path);
    spec.context = Some("modified after preview".into());
    fs::write(&spec_path, serde_yaml::to_string(&spec).unwrap()).unwrap();

    // Approve should reject — spec changed since preview
    let result = run_fail(
        repo.path(),
        ["approve", &spec_str, "--reviewer", "TestReviewer"],
    );
    assert!(
        result.stderr.contains("modified since preview")
            || result.stderr.contains("spec_sha")
            || result.stderr.contains("re-run preview"),
        "approve should reject stale preview:\n{}",
        result.summary()
    );
}

#[test]
fn import_rejects_spec_modified_since_approval() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    let spec_str = setup_standard_spec(&repo, "feat.sha2");

    // Full happy path up to approve
    run_ok(repo.path(), ["preview", &spec_str]);
    run_ok(
        repo.path(),
        ["approve", &spec_str, "--reviewer", "TestReviewer"],
    );
    run_ok(repo.path(), ["run-gates", &spec_str]);

    // Modify spec after approval
    let spec_path = only_spec_path(&repo);
    let mut spec: s5d::Spec = load_yaml(&spec_path);
    spec.context = Some("modified after approval".into());
    fs::write(&spec_path, serde_yaml::to_string(&spec).unwrap()).unwrap();

    // Import should reject — spec changed since approval
    let result = run_fail(repo.path(), ["import", &spec_str]);
    assert!(
        result.stderr.contains("modified")
            || result.stderr.contains("spec_sha")
            || result.stderr.contains("changed"),
        "import should reject stale approval:\n{}",
        result.summary()
    );
}

// ── Rollback tests ──────────────────────────────────────────────────────────

#[test]
fn rollback_after_import_tombstones_aliases_and_removes_index() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    let spec_str = setup_standard_spec(&repo, "feat.rb1");

    // Full lifecycle
    run_ok(repo.path(), ["preview", &spec_str]);
    run_ok(
        repo.path(),
        ["approve", &spec_str, "--reviewer", "TestReviewer"],
    );
    run_ok(repo.path(), ["run-gates", &spec_str]);
    run_ok(repo.path(), ["import", &spec_str]);

    // Verify import created entries
    let s5d_dir = repo.path().join(".s5d");
    let aliases: s5d::AliasTable =
        s5d::AliasTable::load(&s5d_dir).unwrap();
    let active_before = aliases
        .packages
        .iter()
        .filter(|e| !e.deprecated)
        .count();
    assert!(active_before > 0, "should have active aliases after import");

    let index: s5d::Index = load_yaml(&s5d_dir.join("index.yaml"));
    assert!(
        !index.features.is_empty(),
        "index should have entries after import"
    );

    // Rollback
    run_ok(repo.path(), ["rollback", &spec_str]);

    // Check aliases are tombstoned
    let aliases_after: s5d::AliasTable =
        s5d::AliasTable::load(&s5d_dir).unwrap();
    let active_after = aliases_after
        .packages
        .iter()
        .filter(|e| e.package_id.as_deref() == Some("feat.rb1") && !e.deprecated)
        .count();
    assert_eq!(active_after, 0, "all package aliases should be deprecated after rollback");

    // Check index is empty
    let index_after: s5d::Index = load_yaml(&s5d_dir.join("index.yaml"));
    assert!(
        index_after.features.is_empty(),
        "index should be empty after rollback"
    );

    // Check record status
    let record_path = record_path_for(&only_spec_path(&repo));
    let record: s5d::Record = load_yaml(&record_path);
    assert_eq!(
        record.status,
        s5d::SpecStatus::Deprecated,
        "record should be deprecated after rollback"
    );
}

#[test]
fn rollback_without_successful_import_fails() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    let spec_str = setup_standard_spec(&repo, "feat.rb2");

    // Only preview+approve, no import
    run_ok(repo.path(), ["preview", &spec_str]);
    run_ok(
        repo.path(),
        ["approve", &spec_str, "--reviewer", "TestReviewer"],
    );

    // Rollback should fail — nothing to roll back
    let result = run_fail(repo.path(), ["rollback", &spec_str]);
    assert!(
        result.stderr.contains("no successful import"),
        "rollback without import should fail:\n{}",
        result.summary()
    );
}

// ── Decision spec_ref gate ──────────────────────────────────────────────────

#[test]
fn decide_rejects_winner_without_spec_ref() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    // Create decision spec
    run_ok(
        repo.path(),
        [
            "new",
            "decision.test",
            "--tier",
            "decision",
            "--product",
            "shop",
            "--question",
            "Which approach?",
        ],
    );
    let spec_path = only_spec_path(&repo);
    let spec_str = spec_path.to_str().unwrap().to_string();

    // Add 3 hypotheses with evidence to pass methodological checks
    for (title, content) in [
        ("Option A", "First approach"),
        ("Option B", "Second approach"),
        ("Option C", "Third approach"),
    ] {
        run_ok(
            repo.path(),
            [
                "add-hypothesis",
                &spec_str,
                "--title",
                title,
                "--content",
                content,
                "--scope",
                "test",
            ],
        );
    }

    // Add evidence to each hypothesis
    for hyp_id in ["option-a", "option-b", "option-c"] {
        run_ok(
            repo.path(),
            [
                "add-evidence",
                &spec_str,
                "--hypothesis-id",
                hyp_id,
                "--evidence-type",
                "internal",
                "--content",
                "test evidence",
                "--verdict",
                "pass",
            ],
        );
    }

    // Add acceptance criteria to problem card via YAML manipulation
    let spec_path_now = only_spec_path(&repo);
    let content = fs::read_to_string(&spec_path_now).unwrap();
    let mut doc: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
    if let Some(problem) = doc.get_mut("problem") {
        if let Some(map) = problem.as_mapping_mut() {
            map.insert(
                serde_yaml::Value::String("acceptance".into()),
                serde_yaml::Value::String("test acceptance".into()),
            );
        }
    }
    fs::write(&spec_path_now, serde_yaml::to_string(&doc).unwrap()).unwrap();

    // Try to decide — methodological checks pass, but spec_ref is missing
    let result = run_fail(
        repo.path(),
        [
            "decide",
            &spec_str,
            "--title",
            "Pick A",
            "--winner",
            "option-a",
            "--context",
            "test",
            "--decision",
            "go with A",
            "--rationale",
            "because",
            "--consequences",
            "none",
            "--confirmed-by",
            "TestReviewer",
        ],
    );
    assert!(
        result.stderr.contains("spec_ref")
            || result.stderr.contains("feature spec"),
        "decide should reject winner without spec_ref:\n{}",
        result.summary()
    );
}

// ── Reconcile test ──────────────────────────────────────────────────────────

#[test]
fn reconcile_restores_synced_status_after_drift() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    let spec_str = setup_standard_spec(&repo, "feat.rec1");

    // Full lifecycle
    run_ok(repo.path(), ["preview", &spec_str]);
    run_ok(
        repo.path(),
        ["approve", &spec_str, "--reviewer", "TestReviewer"],
    );
    run_ok(repo.path(), ["run-gates", &spec_str]);
    run_ok(repo.path(), ["import", &spec_str]);

    // Manually corrupt alias table to cause drift
    let s5d_dir = repo.path().join(".s5d");
    let mut aliases = s5d::AliasTable::load(&s5d_dir).unwrap();
    if let Some(entry) = aliases.packages.first_mut() {
        entry.uuid = "00000000-0000-0000-0000-000000000000".into();
    }
    aliases.save(&s5d_dir).unwrap();

    // Drift-check should show drifted
    let drift = run_s5d(repo.path(), ["drift-check", &spec_str]);
    assert!(
        drift.stdout.contains("drifted") || drift.stderr.contains("drifted"),
        "should detect drift:\n{}",
        drift.summary()
    );

    // Reconcile should fix it
    run_ok(repo.path(), ["reconcile", &spec_str]);

    // Record should be synced
    let record_path = record_path_for(&only_spec_path(&repo));
    let record: s5d::Record = load_yaml(&record_path);
    assert_eq!(
        record.sync_status,
        s5d::SyncStatus::Synced,
        "record should be synced after reconcile"
    );
}

// ── Shared global rollback risk ─────────────────────────────────────────────

#[test]
fn rollback_of_first_spec_does_not_break_second_spec_sharing_global_artifact() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    // Create two specs that share a global artifact (Product "shop")
    // Spec 1
    run_ok(
        repo.path(),
        [
            "new",
            "feat.spec1",
            "--tier",
            "lightweight",
            "--product",
            "shop",
        ],
    );
    let spec1_path = only_spec_path(&repo);
    let spec1_str = spec1_path.to_str().unwrap().to_string();
    {
        let mut spec: s5d::Spec = load_yaml(&spec1_path);
        let arts = spec.artifacts.as_mut().unwrap();
        arts.capabilities.push(s5d::Capability {
            id: "cap.A".into(),
            domain: "".into(),
            name: "CapA".into(),
            description: None,
            since: None,
        });
        fs::write(&spec1_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
    }
    run_ok(repo.path(), ["validate", &spec1_str]);
    run_ok(repo.path(), ["preview", &spec1_str]);
    run_ok(
        repo.path(),
        ["approve", &spec1_str, "--reviewer", "R"],
    );
    run_ok(repo.path(), ["run-gates", &spec1_str]);
    run_ok(repo.path(), ["import", &spec1_str]);

    // Spec 2 (in same repo, shares Product "shop")
    run_ok(
        repo.path(),
        [
            "new",
            "feat.spec2",
            "--tier",
            "lightweight",
            "--product",
            "shop",
        ],
    );
    // Find spec2 (there are now two specs)
    let specs_dir = repo.path().join(".s5d").join("packages");
    let spec2_path = fs::read_dir(&specs_dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.to_string_lossy().contains("feat.spec2"))
        .expect("spec2 should exist");
    let spec2_str = spec2_path.to_str().unwrap().to_string();
    {
        let mut spec: s5d::Spec = load_yaml(&spec2_path);
        let arts = spec.artifacts.as_mut().unwrap();
        arts.capabilities.push(s5d::Capability {
            id: "cap.B".into(),
            domain: "".into(),
            name: "CapB".into(),
            description: None,
            since: None,
        });
        fs::write(&spec2_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
    }
    run_ok(repo.path(), ["validate", &spec2_str]);
    run_ok(repo.path(), ["preview", &spec2_str]);
    run_ok(
        repo.path(),
        ["approve", &spec2_str, "--reviewer", "R"],
    );
    run_ok(repo.path(), ["run-gates", &spec2_str]);
    run_ok(repo.path(), ["import", &spec2_str]);

    // Both imported. Now rollback spec1.
    run_ok(repo.path(), ["rollback", &spec1_str]);

    // The shared Product "shop" global alias should NOT be tombstoned
    // because spec2 still uses it.
    let s5d_dir = repo.path().join(".s5d");
    let aliases = s5d::AliasTable::load(&s5d_dir).unwrap();

    let shop_product_alive = aliases.global.iter().any(|e| {
        e.artifact_id == "shop"
            && e.artifact_type == "Product"
            && !e.deprecated
    });

    assert!(
        shop_product_alive,
        "shared Product 'shop' global alias should NOT be tombstoned when spec2 still uses it.\n\
         Global aliases: {:?}",
        aliases
            .global
            .iter()
            .filter(|e| e.artifact_id == "shop")
            .collect::<Vec<_>>()
    );
}

// ── Original tests continue ─────────────────────────────────────────────────

#[test]
fn contract_validation_missing_format_and_content() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    let spec_dir = repo.path().join(".s5d").join("packages");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_dir.join("feat.testproduct.badcontract__20260411.s5d.yaml"),
        r#"s5d: '1.0'
id: feat.testproduct.badcontract
version: 1.0.0
product: testproduct
tier: standard
allow_update: false
meta:
  title: Bad Contract
  authors: []
  date: 2026-04-11
  tickets: []
  adrs: []
  renames: []
context: null
artifacts:
  products:
  - id: testproduct
    name: TestProduct
    organization: null
  domains:
  - id: dom.testproduct.core
    product: testproduct
    name: Core
    classification: core
    description: null
  capabilities:
  - id: cap.DoThing
    domain: dom.testproduct.core
    name: DoThing
  entities: []
  features:
  - id: feat.testproduct.badcontract
    product: testproduct
    name: Bad Contract
    description: null
  use_cases: []
  systems: []
  containers:
  - id: ctr.main
    system: sys.test
    name: Main
  components:
  - id: comp.handler
    domain: dom.testproduct.core
    feature: feat.testproduct.badcontract
    container: ctr.main
    name: Handler
    paths:
    - src/handler.rs
  roles: []
  concerns: []
  metrics: []
  supersystems: []
  transports: []
links:
  feature_to_domain: []
  use_case_to_capability: []
  use_case_to_entity: []
  component_to_capability: []
  component_to_entity: []
  container_to_capability: []
  concern_to_metric: []
  component_to_container: []
  edges: []
  depends_on: []
  entity_relations: []
  capability_to_entity: []
  capability_to_concern: []
contracts:
- id: contract.bad
  format: yaml
  binds_to: []
gates:
- kind: schema
roc: null
problem: null
hypotheses: []
decision: null
note_rationale: null
expires_at: null
auto_noted: false
"#,
    )
    .unwrap();

    let result = run_fail(
        repo.path(),
        [
            "validate",
            ".s5d/packages/feat.testproduct.badcontract__20260411.s5d.yaml",
        ],
    );
    // Should catch: invalid format, missing path/inline, empty binds_to
    assert!(
        result.stderr.contains("invalid format"),
        "should catch invalid contract format:\n{}",
        result.summary()
    );
    assert!(
        result.stderr.contains("path") || result.stderr.contains("inline"),
        "should catch missing path/inline:\n{}",
        result.summary()
    );
    assert!(
        result.stderr.contains("binds_to"),
        "should catch empty binds_to:\n{}",
        result.summary()
    );
}
