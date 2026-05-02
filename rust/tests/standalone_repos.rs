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

fn run_git<I, S>(cwd: &Path, args: I) -> CmdOutcome
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new("git")
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
fn run_git_ok<I, S>(cwd: &Path, args: I) -> CmdOutcome
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let outcome = run_git(cwd, args);
    assert!(
        outcome.success,
        "git command failed:\n{}",
        outcome.summary()
    );
    outcome
}

fn init_git_repo(repo: &StandaloneRepo) {
    run_git_ok(repo.path(), ["init"]);
    run_git_ok(repo.path(), ["config", "user.email", "s5d@example.test"]);
    run_git_ok(repo.path(), ["config", "user.name", "S5D Test"]);
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

fn only_task_path(repo: &StandaloneRepo) -> PathBuf {
    only_matching_file(&repo.path().join(".s5d").join("tasks"), ".ralph.md")
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

fn configure_engine(repo: &StandaloneRepo, name: &str, approved: bool, command: Vec<String>) {
    let config_path = repo.path().join(".s5d").join("config.yaml");
    let mut config: s5d::S5dConfig = load_yaml(&config_path);
    config.engines.insert(
        name.to_string(),
        s5d::EngineConfig {
            approved,
            command,
            reasoning: Some("low".into()),
        },
    );
    fs::write(config_path, serde_yaml::to_string(&config).unwrap()).unwrap();
}

fn configure_gate_timeout(repo: &StandaloneRepo, timeout_s: u32) {
    let config_path = repo.path().join(".s5d").join("config.yaml");
    let mut config: s5d::S5dConfig = load_yaml(&config_path);
    let mut gate_runner = config.gate_runner.unwrap_or(s5d::GateRunner {
        cwd: None,
        timeout_s: None,
        env_inherit: None,
        env_deny: vec![],
    });
    gate_runner.timeout_s = Some(timeout_s);
    config.gate_runner = Some(gate_runner);
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

fn seed_architecture_lint_repo(repo: &StandaloneRepo) {
    repo.write(
        "Cargo.toml",
        "[package]\nname = \"shop\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    repo.write("src/lib.rs", "pub mod auth;\npub mod billing;\n");
    repo.write("src/auth/mod.rs", "pub mod tokens;\n");
    repo.write("src/auth/tokens.rs", "pub struct Token;\n");
    repo.write("src/billing/mod.rs", "pub mod service;\n");
    repo.write(
        "src/billing/service.rs",
        "use crate::auth::tokens::Token;\n\npub fn charge(_: Token) {}\n",
    );
}

fn write_architecture_lint_spec(repo: &StandaloneRepo, allow_billing_to_auth: bool) -> PathBuf {
    let spec_dir = repo.path().join(".s5d").join("packages");
    fs::create_dir_all(&spec_dir).unwrap();
    let spec_path = spec_dir.join("feat.shop.billing-boundary__20260423.s5d.yaml");

    let mut links = s5d::Links::default();
    if allow_billing_to_auth {
        links.edges.push(s5d::Edge {
            from: "dom.billing".into(),
            to: "dom.auth".into(),
            archetype: "customer_supplier".into(),
            description: Some("Billing may depend on auth tokens".into()),
            downstream_capability: None,
            waiver: None,
            transport_ref: None,
        });
    }

    let spec = s5d::Spec {
        s5d: "1.0".into(),
        id: "feat.shop.billing-boundary".into(),
        version: "1.0.0".into(),
        product: "shop".into(),
        tier: s5d::Tier::Standard,
        allow_update: false,
        meta: None,
        context: Some("Architecture lint fixture".into()),
        workflow: None,
        artifacts: Some(s5d::Artifacts {
            products: vec![s5d::Product {
                id: "shop".into(),
                name: "Shop".into(),
                organization: None,
            }],
            domains: vec![
                s5d::Domain {
                    id: "dom.auth".into(),
                    product: "shop".into(),
                    name: "Auth".into(),
                    classification: Some("supporting".into()),
                    description: None,
                    team: None,
                    maturity_level: None,
                },
                s5d::Domain {
                    id: "dom.billing".into(),
                    product: "shop".into(),
                    name: "Billing".into(),
                    classification: Some("core".into()),
                    description: None,
                    team: None,
                    maturity_level: None,
                },
            ],
            capabilities: vec![
                s5d::Capability {
                    id: "cap.auth.tokens".into(),
                    domain: "dom.auth".into(),
                    name: "Tokens".into(),
                    description: None,
                    since: None,
                },
                s5d::Capability {
                    id: "cap.billing.charge".into(),
                    domain: "dom.billing".into(),
                    name: "Charge".into(),
                    description: None,
                    since: None,
                },
            ],
            features: vec![s5d::Feature {
                id: "feat.shop.billing-boundary".into(),
                product: "shop".into(),
                name: "Billing Boundary".into(),
                description: None,
            }],
            systems: vec![s5d::SoftwareSystem {
                id: "sys.shop".into(),
                product: "shop".into(),
                name: "Shop".into(),
            }],
            containers: vec![s5d::Container {
                id: "ctr.backend".into(),
                system: "sys.shop".into(),
                name: "Backend".into(),
                technology: Some("Rust".into()),
            }],
            components: vec![
                s5d::Component {
                    id: "comp.auth".into(),
                    feature: "feat.shop.billing-boundary".into(),
                    domain: "dom.auth".into(),
                    container: "ctr.backend".into(),
                    name: "Auth".into(),
                    paths: vec!["src/auth".into()],
                },
                s5d::Component {
                    id: "comp.billing".into(),
                    feature: "feat.shop.billing-boundary".into(),
                    domain: "dom.billing".into(),
                    container: "ctr.backend".into(),
                    name: "Billing".into(),
                    paths: vec!["src/billing".into()],
                },
            ],
            ..Default::default()
        }),
        links: Some(links),
        contracts: vec![],
        gates: vec![
            s5d::Gate {
                kind: "schema".into(),
            },
            s5d::Gate {
                kind: "graph".into(),
            },
            s5d::Gate {
                kind: "architecture".into(),
            },
        ],
        roc: None,
        problem: None,
        hypotheses: vec![],
        decision: None,
        note_rationale: None,
        expires_at: None,
        auto_noted: false,
    };

    fs::write(&spec_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
    spec_path
}

fn write_codebase_governance_spec(repo: &StandaloneRepo) -> PathBuf {
    let spec_dir = repo.path().join(".s5d").join("packages");
    fs::create_dir_all(&spec_dir).unwrap();
    let spec_path = spec_dir.join("feat.app.self-governance__20260423.s5d.yaml");

    let spec = s5d::Spec {
        s5d: "1.0".into(),
        id: "feat.app.self-governance".into(),
        version: "1.0.0".into(),
        product: "app".into(),
        tier: s5d::Tier::Standard,
        allow_update: false,
        meta: None,
        context: Some("Self-governance fixture".into()),
        workflow: None,
        artifacts: Some(s5d::Artifacts {
            products: vec![s5d::Product {
                id: "app".into(),
                name: "App".into(),
                organization: None,
            }],
            domains: vec![s5d::Domain {
                id: "dom.app".into(),
                product: "app".into(),
                name: "App".into(),
                classification: Some("core".into()),
                description: None,
                team: None,
                maturity_level: None,
            }],
            capabilities: vec![s5d::Capability {
                id: "cap.app.source".into(),
                domain: "dom.app".into(),
                name: "Source".into(),
                description: None,
                since: None,
            }],
            features: vec![s5d::Feature {
                id: "feat.app.self-governance".into(),
                product: "app".into(),
                name: "Self Governance".into(),
                description: None,
            }],
            systems: vec![s5d::SoftwareSystem {
                id: "sys.app".into(),
                product: "app".into(),
                name: "App".into(),
            }],
            containers: vec![s5d::Container {
                id: "ctr.app".into(),
                system: "sys.app".into(),
                name: "Rust App".into(),
                technology: Some("Rust".into()),
            }],
            components: vec![s5d::Component {
                id: "comp.app".into(),
                feature: "feat.app.self-governance".into(),
                domain: "dom.app".into(),
                container: "ctr.app".into(),
                name: "App Source".into(),
                paths: vec!["app/src".into()],
            }],
            ..Default::default()
        }),
        links: Some(s5d::Links::default()),
        contracts: vec![],
        gates: vec![
            s5d::Gate {
                kind: "schema".into(),
            },
            s5d::Gate {
                kind: "graph".into(),
            },
            s5d::Gate {
                kind: "architecture".into(),
            },
        ],
        roc: None,
        problem: None,
        hypotheses: vec![],
        decision: None,
        note_rationale: None,
        expires_at: None,
        auto_noted: false,
    };

    fs::write(&spec_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
    spec_path
}

#[test]
fn init_bootstraps_project_layout_for_standalone_repo() {
    let repo = StandaloneRepo::new();
    seed_searchable_rust_repo(&repo);

    let out = run_ok(repo.path(), ["init"]);

    assert!(repo.path().join(".s5d").join("packages").is_dir());
    assert!(repo.path().join(".s5d").join("records").is_dir());
    assert!(repo.path().join(".s5d").join("tasks").is_dir());
    assert!(repo.path().join(".s5d").join(".locks").is_dir());
    assert!(out.stdout.contains("S5D initialized"));
    assert!(out.stdout.contains("s5d new <feature-id> --product <name>"));

    let status = run_ok(repo.path(), ["status"]);
    assert!(status
        .stdout
        .contains("No specs. Run `s5d new <id>` to create one."));
}

#[test]
fn public_help_hides_internal_commands() {
    let repo = StandaloneRepo::new();

    let help = run_ok(repo.path(), ["--help"]);
    for hidden in ["index", "gate", "bootstrap", "cg", "mcp"] {
        assert!(
            !help
                .stdout
                .lines()
                .any(|line| line.trim_start().starts_with(&format!("{hidden} "))),
            "top-level help should hide internal command `{hidden}`:\n{}",
            help.stdout
        );
    }
    for public in ["codebase", "hook", "update", "install"] {
        assert!(
            help.stdout
                .lines()
                .any(|line| line.trim_start().starts_with(&format!("{public} "))),
            "top-level help should keep public command `{public}`:\n{}",
            help.stdout
        );
    }

    let hook_help = run_ok(repo.path(), ["hook", "--help"]);
    assert!(
        hook_help
            .stdout
            .lines()
            .any(|line| line.trim_start().starts_with("pre-commit ")),
        "hook help should keep pre-commit public:\n{}",
        hook_help.stdout
    );
    for hidden in [
        "pretool-edit",
        "user-prompt-submit",
        "require-spec",
        "pre-commit-validate",
    ] {
        assert!(
            !hook_help
                .stdout
                .lines()
                .any(|line| line.trim_start().starts_with(&format!("{hidden} "))),
            "hook help should hide internal command `{hidden}`:\n{}",
            hook_help.stdout
        );
    }
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
    let approve_without_reviewer = run_fail(repo.path(), ["approve", spec_str]);
    assert!(
        approve_without_reviewer.stderr.contains("--reviewer"),
        "{}",
        approve_without_reviewer.summary()
    );
    run_ok(repo.path(), ["approve", spec_str, "--reviewer", "Roman"]);
    let gates = run_ok(repo.path(), ["run-gates", spec_str]);
    assert!(gates.stdout.contains("gate:schema"), "{}", gates.summary());

    let import_without_verifier = run_fail(repo.path(), ["import", spec_str]);
    assert!(
        import_without_verifier.stderr.contains("--verified-by"),
        "{}",
        import_without_verifier.summary()
    );
    let same_person_import = run_fail(repo.path(), ["import", spec_str, "--verified-by", "Roman"]);
    assert!(
        same_person_import
            .stderr
            .contains("verifier should differ from approver"),
        "{}",
        same_person_import.summary()
    );
    let import = run_ok(repo.path(), ["import", spec_str, "--verified-by", "Diana"]);
    assert!(import.stdout.contains("Imported"), "{}", import.summary());

    let record: s5d::Record = load_yaml(&record_path_for(&spec_path));
    assert_eq!(record.status, s5d::SpecStatus::Applied);
    assert_eq!(record.sync_status, s5d::SyncStatus::Synced);
    assert_eq!(record.verified_by.as_deref(), Some("Diana"));
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
fn workflow_phase_lifecycle_emits_ralph_task_package_and_records_outcome() {
    let repo = StandaloneRepo::new();
    seed_searchable_rust_repo(&repo);
    run_ok(repo.path(), ["init"]);

    run_ok(
        repo.path(),
        [
            "new",
            "feat.billing.operator-loop",
            "--tier",
            "lightweight",
            "--product",
            "Billing",
        ],
    );

    let spec_path = only_spec_path(&repo);
    let spec_str = spec_path.to_str().unwrap();

    {
        let mut spec: s5d::Spec = load_yaml(&spec_path);
        if let Some(ref mut artifacts) = spec.artifacts {
            artifacts.capabilities.push(s5d::Capability {
                id: "cap.RunOperatorLoop".into(),
                domain: "billing".into(),
                name: "RunOperatorLoop".into(),
                description: Some("Emit bounded Ralph task packages".into()),
                since: None,
            });
        }
        let workflow = spec
            .workflow
            .as_mut()
            .expect("feature template should carry workflow shell");
        workflow.mode = Some("implement".into());
        if let Some(ref mut execution_mode) = workflow.execution_mode {
            execution_mode.engine = "ralph".into();
            execution_mode.max_iterations = Some(3);
        }
        fs::write(&spec_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
    }

    run_ok(repo.path(), ["preview", spec_str]);
    run_ok(repo.path(), ["approve", spec_str, "--reviewer", "Roman"]);

    let phases = run_ok(repo.path(), ["phase", "list", spec_str]);
    assert!(phases.stdout.contains("Prototype"), "{}", phases.summary());
    assert!(phases.stdout.contains("prototype"), "{}", phases.summary());

    let execute_before_start = run_fail(
        repo.path(),
        [
            "execute",
            "loop",
            spec_str,
            "--phase",
            "prototype",
            "--engine",
            "ralph",
        ],
    );
    assert!(
        execute_before_start
            .stderr
            .contains("must be active before execute loop"),
        "{}",
        execute_before_start.summary()
    );

    let start = run_ok(
        repo.path(),
        ["phase", "start", spec_str, "--id", "prototype"],
    );
    assert!(start.stdout.contains("Active phase"), "{}", start.summary());

    let status = run_ok(repo.path(), ["status"]);
    assert!(
        status.stdout.contains("Active phase:"),
        "{}",
        status.summary()
    );
    assert!(status.stdout.contains("prototype"), "{}", status.summary());

    let execute = run_ok(
        repo.path(),
        [
            "execute",
            "loop",
            spec_str,
            "--phase",
            "prototype",
            "--engine",
            "ralph",
        ],
    );
    assert!(
        execute.stdout.contains("RALPH TASK PACKAGE"),
        "{}",
        execute.summary()
    );
    assert!(
        execute.stdout.contains("Task artifact:"),
        "{}",
        execute.summary()
    );
    assert!(
        execute.stdout.contains("Preset: ralph-init"),
        "{}",
        execute.summary()
    );
    assert!(
        execute.stdout.contains("Phase: Prototype (prototype)"),
        "{}",
        execute.summary()
    );
    assert!(
        execute.stdout.contains("Acceptance:"),
        "{}",
        execute.summary()
    );
    assert!(
        execute.stdout.contains(
            "Read user-facing docs and representative tests before inferring from code only"
        ),
        "{}",
        execute.summary()
    );
    assert!(
        execute.stdout.contains("Escalate Immediately If:"),
        "{}",
        execute.summary()
    );
    let task_path = only_task_path(&repo);
    let task_content = fs::read_to_string(&task_path).unwrap();
    assert!(
        task_content.contains("RALPH TASK PACKAGE"),
        "{}",
        task_content
    );
    assert!(
        task_content.contains("Preset: ralph-init"),
        "{}",
        task_content
    );

    let accept = run_ok(
        repo.path(),
        [
            "phase",
            "accept",
            spec_str,
            "--id",
            "prototype",
            "--reviewer",
            "Roman",
        ],
    );
    assert!(
        accept.stdout.contains("accepted by Roman"),
        "{}",
        accept.summary()
    );

    let gates = run_ok(repo.path(), ["run-gates", spec_str]);
    assert!(gates.stdout.contains("gate:schema"), "{}", gates.summary());

    let import = run_ok(repo.path(), ["import", spec_str, "--verified-by", "Diana"]);
    assert!(import.stdout.contains("Imported"), "{}", import.summary());

    let reflect = run_ok(
        repo.path(),
        [
            "reflect",
            spec_str,
            "--summary",
            "Telemetry stayed inside target bounds",
            "--verdict",
            "confirmed",
            "--measurement-window",
            "7d post-ship",
            "--telemetry",
            "grafana://billing-operator-loop",
            "--heuristic",
            "Keep Ralph scopes phase-bounded",
        ],
    );
    assert!(
        reflect.stdout.contains("Reflect recorded"),
        "{}",
        reflect.summary()
    );

    let record: s5d::Record = load_yaml(&record_path_for(&spec_path));
    assert_eq!(record.status, s5d::SpecStatus::Operated);
    assert_eq!(record.active_phase, None);
    assert!(record.phase_history.iter().any(|entry| {
        entry.phase_id == "prototype" && entry.status == s5d::WorkflowPhaseStatus::Active
    }));
    assert!(record.phase_history.iter().any(|entry| {
        entry.phase_id == "prototype"
            && entry.status == s5d::WorkflowPhaseStatus::Accepted
            && entry.reviewer.as_deref() == Some("Roman")
    }));

    let reflection = record.reflection.expect("reflection should be recorded");
    assert_eq!(reflection.verdict.as_deref(), Some("confirmed"));
    assert_eq!(
        reflection.measurement_window.as_deref(),
        Some("7d post-ship")
    );
    assert_eq!(
        reflection.telemetry_refs,
        vec!["grafana://billing-operator-loop".to_string()]
    );
}

#[test]
fn workflow_phase_run_records_external_engine_artifact() {
    let repo = StandaloneRepo::new();
    seed_searchable_rust_repo(&repo);
    run_ok(repo.path(), ["init"]);

    run_ok(
        repo.path(),
        [
            "new",
            "feat.billing.external-run",
            "--tier",
            "lightweight",
            "--product",
            "Billing",
        ],
    );

    let spec_path = only_spec_path(&repo);
    let spec_str = spec_path.to_str().unwrap();
    {
        let mut spec: s5d::Spec = load_yaml(&spec_path);
        if let Some(ref mut artifacts) = spec.artifacts {
            artifacts.capabilities.push(s5d::Capability {
                id: "cap.RunExternalEngine".into(),
                domain: "billing".into(),
                name: "RunExternalEngine".into(),
                description: Some("Record external engine phase artifacts".into()),
                since: None,
            });
        }
        fs::write(&spec_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
    }

    configure_engine(
        &repo,
        "local-s5d",
        true,
        vec![s5d_bin().into(), "show".into(), "{spec}".into()],
    );
    configure_engine(
        &repo,
        "blocked-s5d",
        false,
        vec![s5d_bin().into(), "show".into(), "{spec}".into()],
    );

    run_ok(repo.path(), ["preview", spec_str]);
    run_ok(repo.path(), ["approve", spec_str, "--reviewer", "Roman"]);

    let before_start = run_fail(
        repo.path(),
        [
            "phase",
            "run",
            spec_str,
            "--id",
            "prototype",
            "--engine",
            "local-s5d",
        ],
    );
    assert!(
        before_start
            .stderr
            .contains("must be active before phase run"),
        "{}",
        before_start.summary()
    );

    run_ok(
        repo.path(),
        ["phase", "start", spec_str, "--id", "prototype"],
    );

    let unapproved = run_fail(
        repo.path(),
        [
            "phase",
            "run",
            spec_str,
            "--id",
            "prototype",
            "--engine",
            "blocked-s5d",
        ],
    );
    assert!(
        unapproved.stderr.contains("not approved"),
        "{}",
        unapproved.summary()
    );

    let run = run_ok(
        repo.path(),
        [
            "phase",
            "run",
            spec_str,
            "--id",
            "prototype",
            "--engine",
            "local-s5d",
        ],
    );
    assert!(
        run.stdout.contains("Phase run completed"),
        "{}",
        run.summary()
    );
    assert!(run.stdout.contains("sha256:"), "{}", run.summary());

    let record: s5d::Record = load_yaml(&record_path_for(&spec_path));
    assert_eq!(record.phase_runs.len(), 1);
    let phase_run = &record.phase_runs[0];
    assert_eq!(phase_run.phase_id, "prototype");
    assert_eq!(phase_run.engine, "local-s5d");
    assert_eq!(phase_run.status, "completed");
    assert_eq!(phase_run.reasoning.as_deref(), Some("low"));
    assert!(phase_run.output_sha256.starts_with("sha256:"));
    assert!(repo.path().join(&phase_run.output_path).is_file());
    assert!(record.phase_history.iter().any(|entry| {
        entry.phase_id == "prototype"
            && entry.status == s5d::WorkflowPhaseStatus::Verified
            && entry.engine.as_deref() == Some("local-s5d")
    }));
}

#[test]
fn operational_harness_creates_worktree_and_journals_commands() {
    let repo = StandaloneRepo::new();
    init_git_repo(&repo);
    seed_searchable_rust_repo(&repo);
    run_ok(repo.path(), ["init"]);

    run_ok(
        repo.path(),
        [
            "new",
            "feat.billing.harness",
            "--tier",
            "lightweight",
            "--product",
            "Billing",
        ],
    );
    let spec_path = only_spec_path(&repo);
    let spec_str = spec_path.to_str().unwrap();
    {
        let mut spec: s5d::Spec = load_yaml(&spec_path);
        if let Some(ref mut artifacts) = spec.artifacts {
            artifacts.capabilities.push(s5d::Capability {
                id: "cap.OperationalHarness".into(),
                domain: "billing".into(),
                name: "OperationalHarness".into(),
                description: Some("Harness worktree and command journal".into()),
                since: None,
            });
        }
        fs::write(&spec_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
    }
    run_ok(repo.path(), ["preview", spec_str]);
    run_ok(repo.path(), ["approve", spec_str, "--reviewer", "Roman"]);
    run_git_ok(repo.path(), ["add", "."]);
    run_git_ok(repo.path(), ["commit", "-m", "seed approved harness spec"]);

    repo.write("dirty.txt", "dirty");
    let dirty = run_fail(
        repo.path(),
        [
            "harness",
            "start",
            spec_str,
            "--phase",
            "prototype",
            "--name",
            "dirty-check",
        ],
    );
    assert!(
        dirty.stderr.contains("source worktree is not clean"),
        "{}",
        dirty.summary()
    );
    fs::remove_file(repo.path().join("dirty.txt")).unwrap();

    let worktree = repo.path().parent().unwrap().join(format!(
        "{}-s5d-harness-test-wt",
        repo.path().file_name().unwrap().to_string_lossy()
    ));
    let worktree_str = worktree.to_str().unwrap();
    let start = run_ok(
        repo.path(),
        [
            "harness",
            "start",
            spec_str,
            "--phase",
            "prototype",
            "--name",
            "alpha",
            "--worktree",
            worktree_str,
        ],
    );
    assert!(
        start.stdout.contains("Harness started"),
        "{}",
        start.summary()
    );
    assert!(worktree.join(".s5d").is_dir());

    let status = run_ok(repo.path(), ["harness", "status", "alpha"]);
    assert!(status.stdout.contains("HARNESS"), "{}", status.summary());
    assert!(status.stdout.contains("prototype"), "{}", status.summary());
    assert!(status.stdout.contains("current:"), "{}", status.summary());
    assert!(
        status.stdout.contains("last event:"),
        "{}",
        status.summary()
    );

    let exec = run_ok(
        repo.path(),
        [
            "harness",
            "exec",
            "alpha",
            "--timeout-s",
            "10",
            "--",
            "{s5d}",
            "status",
        ],
    );
    assert!(
        exec.stdout.contains("Harness command completed"),
        "{}",
        exec.summary()
    );

    let state: s5d::HarnessState =
        load_yaml(&repo.path().join(".s5d").join("harness").join("alpha.yaml"));
    assert_eq!(state.id, "alpha");
    assert_eq!(state.phase_id, "prototype");
    assert!(state.current_command.is_none());
    assert!(state
        .events
        .iter()
        .any(|event| event.kind == "phase_started"));
    let completed = state
        .events
        .iter()
        .find(|event| event.kind == "command_completed")
        .expect("command completion should be journaled");
    assert!(completed
        .stdout_path
        .as_ref()
        .is_some_and(|path| { repo.path().join(path).is_file() }));

    let worktree_spec = worktree
        .join(".s5d")
        .join("packages")
        .join(spec_path.file_name().unwrap());
    let worktree_record: s5d::Record = load_yaml(&record_path_for(&worktree_spec));
    assert_eq!(worktree_record.active_phase.as_deref(), Some("prototype"));
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

    let import = run_fail(repo.path(), ["import", spec_str, "--verified-by", "Diana"]);
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
fn timeout_gate_is_recorded_and_blocks_import() {
    let repo = StandaloneRepo::new();
    seed_searchable_rust_repo(&repo);
    run_ok(repo.path(), ["init"]);
    configure_gate_command(
        &repo,
        "lint",
        vec!["/bin/sh".into(), "-c".into(), "sleep 2".into()],
    );
    configure_gate_timeout(&repo, 1);

    run_ok(
        repo.path(),
        [
            "new",
            "feat.billing.timeoutgate",
            "--tier",
            "lightweight",
            "--product",
            "Billing",
        ],
    );
    let spec_path = only_spec_path(&repo);
    let spec_str = spec_path.to_str().unwrap();

    let mut spec: s5d::Spec = load_yaml(&spec_path);
    if let Some(ref mut artifacts) = spec.artifacts {
        artifacts.capabilities.push(s5d::Capability {
            id: "cap.TimeoutGate".into(),
            domain: "billing".into(),
            name: "TimeoutGate".into(),
            description: None,
            since: None,
        });
    }
    spec.gates = vec![s5d::Gate {
        kind: "lint".to_string(),
    }];
    fs::write(&spec_path, serde_yaml::to_string(&spec).unwrap()).unwrap();

    run_ok(repo.path(), ["preview", spec_str]);
    run_ok(repo.path(), ["approve", spec_str, "--reviewer", "Roman"]);

    let gates = run_fail(repo.path(), ["run-gates", spec_str]);
    assert!(gates.stdout.contains("failed: 1"), "{}", gates.summary());
    assert!(gates.stdout.contains("gate:lint"), "{}", gates.summary());

    let record: s5d::Record = load_yaml(&record_path_for(&spec_path));
    assert!(
        record
            .gate_results
            .iter()
            .any(|result| result.kind == "lint" && result.status == "timeout"),
        "timeout gate result should be recorded"
    );

    let import = run_fail(repo.path(), ["import", spec_str, "--verified-by", "Diana"]);
    assert!(
        import
            .stderr
            .contains("all declared gates must pass before import"),
        "{}",
        import.summary()
    );
}

#[test]
fn init_is_idempotent() {
    let repo = StandaloneRepo::new();
    seed_searchable_rust_repo(&repo);

    // First init creates .s5d/
    let first = run_ok(repo.path(), ["init"]);
    assert!(
        first.stdout.contains("S5D initialized"),
        "{}",
        first.summary()
    );
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
    assert!(
        decide.stdout.contains("Decision recorded"),
        "{}",
        decide.summary()
    );

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
            ".s5d/packages/feat.testproduct.empty__20260410.s5d.yaml",
        ],
    );
    assert!(
        result.stderr.contains("metamodel: spec has no domains"),
        "should block on missing domains:\n{}",
        result.summary()
    );
    assert!(
        result
            .stderr
            .contains("metamodel: spec has no capabilities"),
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

#[test]
fn architecture_check_blocks_undeclared_cross_domain_import() {
    let repo = StandaloneRepo::new();
    seed_architecture_lint_repo(&repo);
    run_ok(repo.path(), ["init"]);

    let spec_path = write_architecture_lint_spec(&repo, false);
    let spec_str = spec_path.to_str().unwrap();

    let result = run_fail(repo.path(), ["check", spec_str]);
    assert!(
        result.stderr.contains("architecture:")
            && result.stderr.contains("dom.billing")
            && result.stderr.contains("dom.auth"),
        "architecture check should explain the forbidden dependency:\n{}",
        result.summary()
    );
}

#[test]
fn architecture_check_allows_declared_domain_edge() {
    let repo = StandaloneRepo::new();
    seed_architecture_lint_repo(&repo);
    run_ok(repo.path(), ["init"]);

    let spec_path = write_architecture_lint_spec(&repo, true);
    let spec_str = spec_path.to_str().unwrap();

    let result = run_ok(repo.path(), ["check", spec_str]);
    assert!(
        result.stdout.contains("architecture ok"),
        "architecture check should pass with declared edge:\n{}",
        result.summary()
    );
}

#[test]
fn architecture_check_accepts_markdown_component_paths() {
    let repo = StandaloneRepo::new();
    repo.write("skills/s5d/SKILL.md", "# S5D\n");
    run_ok(repo.path(), ["init"]);

    let spec_dir = repo.path().join(".s5d").join("packages");
    fs::create_dir_all(&spec_dir).unwrap();
    let spec_path = spec_dir.join("feat.s5d.skill-doc-governance__20260429.s5d.yaml");
    let spec = s5d::Spec {
        s5d: "1.0".into(),
        id: "feat.s5d.skill-doc-governance".into(),
        version: "1.0.0".into(),
        product: "s5d".into(),
        tier: s5d::Tier::Standard,
        allow_update: false,
        meta: None,
        context: Some("Skill documentation is an agent-facing architecture surface".into()),
        workflow: None,
        artifacts: Some(s5d::Artifacts {
            products: vec![s5d::Product {
                id: "s5d".into(),
                name: "S5D".into(),
                organization: None,
            }],
            domains: vec![s5d::Domain {
                id: "dom.s5d.execution".into(),
                product: "s5d".into(),
                name: "S5D Execution".into(),
                classification: Some("core".into()),
                description: None,
                team: None,
                maturity_level: None,
            }],
            capabilities: vec![s5d::Capability {
                id: "cap.s5d.shape-product-intent".into(),
                domain: "dom.s5d.execution".into(),
                name: "ShapeProductIntent".into(),
                description: None,
                since: None,
            }],
            features: vec![s5d::Feature {
                id: "feat.s5d.skill-doc-governance".into(),
                product: "s5d".into(),
                name: "Skill Doc Governance".into(),
                description: None,
            }],
            systems: vec![s5d::SoftwareSystem {
                id: "sys.s5d".into(),
                product: "s5d".into(),
                name: "S5D".into(),
            }],
            containers: vec![s5d::Container {
                id: "ctr.s5d.skill-docs".into(),
                system: "sys.s5d".into(),
                name: "Skill Docs".into(),
                technology: Some("Markdown".into()),
            }],
            components: vec![s5d::Component {
                id: "comp.s5d.skill-docs".into(),
                feature: "feat.s5d.skill-doc-governance".into(),
                domain: "dom.s5d.execution".into(),
                container: "ctr.s5d.skill-docs".into(),
                name: "S5D Skill Docs".into(),
                paths: vec!["skills/s5d/SKILL.md".into()],
            }],
            ..Default::default()
        }),
        links: Some(s5d::Links::default()),
        contracts: vec![],
        gates: vec![
            s5d::Gate {
                kind: "schema".into(),
            },
            s5d::Gate {
                kind: "graph".into(),
            },
            s5d::Gate {
                kind: "architecture".into(),
            },
        ],
        roc: None,
        problem: None,
        hypotheses: vec![],
        decision: None,
        note_rationale: None,
        expires_at: None,
        auto_noted: false,
    };
    fs::write(&spec_path, serde_yaml::to_string(&spec).unwrap()).unwrap();

    let spec_str = spec_path.to_str().unwrap();
    let result = run_ok(repo.path(), ["check", spec_str]);
    assert!(
        result.stdout.contains("architecture ok"),
        "architecture check should accept markdown component paths:\n{}",
        result.summary()
    );
}

#[test]
fn init_installs_rust_pre_commit_hook_and_hook_blocks_architecture_drift() {
    let repo = StandaloneRepo::new();
    seed_architecture_lint_repo(&repo);
    init_git_repo(&repo);

    run_ok(repo.path(), ["init"]);
    let hook_path = repo.path().join(".git").join("hooks").join("pre-commit");
    assert!(
        hook_path.symlink_metadata().is_ok(),
        "init should install a Rust-backed pre-commit hook"
    );

    write_architecture_lint_spec(&repo, false);
    run_git_ok(repo.path(), ["add", "."]);

    let result = run_fail(repo.path(), ["hook", "pre-commit"]);
    assert!(
        result.stderr.contains("s5d pre-commit blocked commit")
            && result.stderr.contains("dom.billing")
            && result.stderr.contains("dom.auth"),
        "hook should block staged architecture drift:\n{}",
        result.summary()
    );
}

#[test]
fn codebase_sync_check_and_hook_block_stale_snapshot() {
    let repo = StandaloneRepo::new();
    repo.write("app/src/lib.rs", "pub fn ok() {}\n");
    init_git_repo(&repo);

    run_ok(repo.path(), ["init"]);
    write_codebase_governance_spec(&repo);

    let sync = run_ok(repo.path(), ["codebase", "sync"]);
    assert!(
        sync.stdout.contains("1 governed, 0 partial, 0 blind"),
        "codebase sync should mark the module governed:\n{}",
        sync.summary()
    );
    let coverage = fs::read_to_string(repo.path().join(".s5d/codebase/coverage.yaml")).unwrap();
    assert!(
        coverage.contains("status: governed") && coverage.contains("feat.app.self-governance"),
        "coverage snapshot should link the governed module to the spec:\n{}",
        coverage
    );

    run_ok(repo.path(), ["codebase", "check"]);
    run_git_ok(repo.path(), ["add", "."]);
    run_ok(repo.path(), ["hook", "pre-commit"]);

    repo.write("app/src/extra.rs", "pub fn added() {}\n");
    run_git_ok(repo.path(), ["add", "app/src/extra.rs"]);

    let result = run_fail(repo.path(), ["hook", "pre-commit"]);
    assert!(
        result.stderr.contains(".s5d/codebase snapshot is stale"),
        "hook should block stale codebase coverage:\n{}",
        result.summary()
    );
}

#[test]
fn update_check_reports_remote_head_drift_without_network() {
    let remote = StandaloneRepo::new();
    init_git_repo(&remote);
    remote.write("install.sh", "#!/bin/sh\n");
    remote.write("skills/s5d/SKILL.md", "name: s5d\n");
    remote.write("skills/fpf/SKILL.md", "name: fpf\n");
    remote.write(
        "rust/Cargo.toml",
        "[package]\nname = \"s5d\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    run_git_ok(remote.path(), ["add", "."]);
    run_git_ok(remote.path(), ["commit", "-m", "initial"]);

    let clone = StandaloneRepo::new();
    run_git_ok(
        clone.path(),
        [
            "clone",
            remote.path().to_str().unwrap(),
            clone.path().join("s5d").to_str().unwrap(),
        ],
    );
    let clone_root = clone.path().join("s5d");

    remote.write("skills/s5d/SKILL.md", "name: s5d\nupdated: true\n");
    run_git_ok(remote.path(), ["add", "."]);
    run_git_ok(remote.path(), ["commit", "-m", "skill update"]);

    let output = Command::new(s5d_bin())
        .args(["update", "check", "--json"])
        .env("S5D_ROOT", &clone_root)
        .current_dir(&clone_root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "update check failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["update_available"], true, "{}", json);
    assert!(json["remote_commit"].as_str().is_some(), "{}", json);
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
        result
            .stderr
            .contains("metamodel: spec has no capabilities"),
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
    run_ok(repo.path(), ["validate", spec_path.to_str().unwrap()]);
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
    let import = run_ok(
        repo.path(),
        ["import", &spec_str, "--verified-by", "TestVerifier"],
    );
    assert!(
        import.stdout.contains("imported")
            || import.stdout.contains("state_fingerprint")
            || import.stdout.contains("Import"),
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
    let result = run_fail(
        repo.path(),
        ["import", &spec_str, "--verified-by", "TestVerifier"],
    );
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
    run_ok(
        repo.path(),
        ["import", &spec_str, "--verified-by", "TestVerifier"],
    );

    // Verify import created entries
    let s5d_dir = repo.path().join(".s5d");
    let aliases: s5d::AliasTable = s5d::AliasTable::load(&s5d_dir).unwrap();
    let active_before = aliases.packages.iter().filter(|e| !e.deprecated).count();
    assert!(active_before > 0, "should have active aliases after import");

    let index: s5d::Index = load_yaml(&s5d_dir.join("index.yaml"));
    assert!(
        !index.features.is_empty(),
        "index should have entries after import"
    );

    // Rollback
    run_ok(repo.path(), ["rollback", &spec_str]);

    // Check aliases are tombstoned
    let aliases_after: s5d::AliasTable = s5d::AliasTable::load(&s5d_dir).unwrap();
    let active_after = aliases_after
        .packages
        .iter()
        .filter(|e| e.package_id.as_deref() == Some("feat.rb1") && !e.deprecated)
        .count();
    assert_eq!(
        active_after, 0,
        "all package aliases should be deprecated after rollback"
    );

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
        result.stderr.contains("spec_ref") || result.stderr.contains("feature spec"),
        "decide should reject winner without spec_ref:\n{}",
        result.summary()
    );
}

// ── Reconcile test ──────────────────────────────────────────────────────────

#[test]
fn reconcile_fails_closed_on_deleted_alias() {
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
    run_ok(
        repo.path(),
        ["import", &spec_str, "--verified-by", "TestVerifier"],
    );

    // Delete one package alias entry — UUID is lost, can't restore
    let s5d_dir = repo.path().join(".s5d");
    let mut aliases = s5d::AliasTable::load(&s5d_dir).unwrap();
    if !aliases.packages.is_empty() {
        aliases.packages.remove(0);
    }
    aliases.save(&s5d_dir).unwrap();

    // Reconcile should fail closed — deleted UUID can't be regenerated
    let result = run_fail(repo.path(), ["reconcile", &spec_str]);
    assert!(
        result.stderr.contains("cannot be restored") || result.stderr.contains("Re-run"),
        "reconcile should fail closed on deleted alias:\n{}",
        result.summary()
    );
}

#[test]
fn reconcile_fails_closed_on_corrupted_uuid() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    let spec_str = setup_standard_spec(&repo, "feat.rec2");

    // Full lifecycle
    run_ok(repo.path(), ["preview", &spec_str]);
    run_ok(
        repo.path(),
        ["approve", &spec_str, "--reviewer", "TestReviewer"],
    );
    run_ok(repo.path(), ["run-gates", &spec_str]);
    run_ok(
        repo.path(),
        ["import", &spec_str, "--verified-by", "TestVerifier"],
    );

    // Corrupt a UUID — reconcile can't fix this from spec alone
    let s5d_dir = repo.path().join(".s5d");
    let mut aliases = s5d::AliasTable::load(&s5d_dir).unwrap();
    if let Some(entry) = aliases.packages.first_mut() {
        entry.uuid = "corrupted-0000-0000-0000-000000000000".into();
    }
    aliases.save(&s5d_dir).unwrap();
    let before = aliases.clone();

    // Reconcile should fail closed — can't restore imported baseline
    let result = run_fail(repo.path(), ["reconcile", &spec_str]);
    assert!(
        result.stderr.contains("cannot be restored") || result.stderr.contains("Re-run"),
        "reconcile should fail closed on corrupted UUID:\n{}",
        result.summary()
    );

    let after = s5d::AliasTable::load(&s5d_dir).unwrap();
    assert_eq!(
        before, after,
        "failed reconcile must not persist candidate alias state"
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
    run_ok(repo.path(), ["approve", &spec1_str, "--reviewer", "R"]);
    run_ok(repo.path(), ["run-gates", &spec1_str]);
    run_ok(repo.path(), ["import", &spec1_str, "--verified-by", "V"]);

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
    run_ok(repo.path(), ["approve", &spec2_str, "--reviewer", "R"]);
    run_ok(repo.path(), ["run-gates", &spec2_str]);
    run_ok(repo.path(), ["import", &spec2_str, "--verified-by", "V"]);

    // Both imported. Now rollback spec1.
    run_ok(repo.path(), ["rollback", &spec1_str]);

    // The shared Product "shop" global alias should NOT be tombstoned
    // because spec2 still uses it.
    let s5d_dir = repo.path().join(".s5d");
    let aliases = s5d::AliasTable::load(&s5d_dir).unwrap();

    let shop_product_alive = aliases
        .global
        .iter()
        .any(|e| e.artifact_id == "shop" && e.artifact_type == "Product" && !e.deprecated);

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

// ── Shared global drift visibility ───────────────────────────────────────────

#[test]
fn shared_global_drift_visible_for_non_owner_spec() {
    // Two specs share Product "shared". Spec1 owns the global alias.
    // Corrupt the shared global UUID. Drift-check on spec2 (non-owner)
    // should detect the drift.
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    // Spec 1 — owner of Product "shared"
    run_ok(
        repo.path(),
        [
            "new",
            "feat.owner",
            "--tier",
            "lightweight",
            "--product",
            "shared",
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
            name: "A".into(),
            description: None,
            since: None,
        });
        fs::write(&spec1_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
    }
    run_ok(repo.path(), ["validate", &spec1_str]);
    run_ok(repo.path(), ["preview", &spec1_str]);
    run_ok(repo.path(), ["approve", &spec1_str, "--reviewer", "R"]);
    run_ok(repo.path(), ["run-gates", &spec1_str]);
    run_ok(repo.path(), ["import", &spec1_str, "--verified-by", "V"]);

    // Spec 2 — non-owner, shares Product "shared"
    run_ok(
        repo.path(),
        [
            "new",
            "feat.consumer",
            "--tier",
            "lightweight",
            "--product",
            "shared",
        ],
    );
    let specs_dir = repo.path().join(".s5d").join("packages");
    let spec2_path = fs::read_dir(&specs_dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.to_string_lossy().contains("feat.consumer"))
        .expect("spec2 should exist");
    let spec2_str = spec2_path.to_str().unwrap().to_string();
    {
        let mut spec: s5d::Spec = load_yaml(&spec2_path);
        let arts = spec.artifacts.as_mut().unwrap();
        arts.capabilities.push(s5d::Capability {
            id: "cap.B".into(),
            domain: "".into(),
            name: "B".into(),
            description: None,
            since: None,
        });
        fs::write(&spec2_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
    }
    run_ok(repo.path(), ["validate", &spec2_str]);
    run_ok(repo.path(), ["preview", &spec2_str]);
    run_ok(repo.path(), ["approve", &spec2_str, "--reviewer", "R"]);
    run_ok(repo.path(), ["run-gates", &spec2_str]);
    run_ok(repo.path(), ["import", &spec2_str, "--verified-by", "V"]);

    // Both imported. Corrupt the shared Product global alias UUID.
    let s5d_dir = repo.path().join(".s5d");
    let mut aliases = s5d::AliasTable::load(&s5d_dir).unwrap();
    for entry in &mut aliases.global {
        if entry.artifact_id == "shared" && entry.artifact_type == "Product" {
            entry.uuid = "corrupted-0000-0000-0000-000000000000".into();
        }
    }
    aliases.save(&s5d_dir).unwrap();

    // Drift-check on spec2 (non-owner) should detect drift
    let drift = run_s5d(repo.path(), ["drift-check", &spec2_str]);
    assert!(
        drift.stdout.contains("drifted") || drift.stderr.contains("drifted"),
        "non-owner spec should detect drift in shared global artifact:\n{}",
        drift.summary()
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
