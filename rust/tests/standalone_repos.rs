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

fn spec_path_by_id(repo: &StandaloneRepo, id: &str) -> PathBuf {
    let dir = repo.path().join(".s5d").join("packages");
    let prefix = format!("{id}__");
    let mut matches = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix) && name.ends_with(".s5d.yaml"))
        })
        .collect::<Vec<_>>();
    matches.sort();
    assert_eq!(
        matches.len(),
        1,
        "expected exactly one spec for {} in {}, got {:?}",
        id,
        dir.display(),
        matches
    );
    matches.remove(0)
}

fn binding(fields: &[(&str, &str)]) -> s5d::Binding {
    s5d::Binding {
        fields: fields
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect(),
    }
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

/// Replace the scaffold's TODO path placeholder with a real path. Import
/// (correctly) refuses placeholder paths; tests that exercise the lifecycle
/// must materialize them like a user would.
fn materialize_scaffold_paths(spec: &mut s5d::Spec) {
    if let Some(ref mut a) = spec.artifacts {
        for c in &mut a.components {
            if c.paths.iter().any(|p| p.contains("TODO-set-source-paths")) {
                c.paths = vec!["src/".into()];
            }
        }
    }
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

#[test]
fn discovery_sync_builds_stack_agnostic_index_graph_and_projection() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    run_ok(
        repo.path(),
        [
            "new",
            "feat.orders.discovery",
            "--tier",
            "lightweight",
            "--product",
            "s5d",
        ],
    );
    repo.write("backend/orders.py", "class Order:\n    pass\n");
    repo.write("web/features/orders.ts", "export const orderList = [];\n");
    repo.write("README.md", "# Shop\n");

    let spec = only_spec_path(&repo);
    let mut feature: s5d::Spec = load_yaml(&spec);
    let artifacts = feature.artifacts.as_mut().unwrap();
    artifacts.domains.push(s5d::Domain {
        id: "orders".into(),
        product: "s5d".into(),
        name: "Orders".into(),
        classification: Some("core".into()),
        description: None,
        team: None,
        maturity_level: None,
    });
    artifacts.capabilities.push(s5d::Capability {
        id: "manage-orders".into(),
        domain: "orders".into(),
        name: "Manage Orders".into(),
        description: None,
        since: None,
    });
    artifacts.entities.push(s5d::Entity {
        id: "order".into(),
        domain: "orders".into(),
        name: "Order".into(),
    });
    artifacts.systems.push(s5d::SoftwareSystem {
        id: "shop".into(),
        product: "s5d".into(),
        name: "Shop".into(),
    });
    artifacts.containers.push(s5d::Container {
        id: "app".into(),
        system: "shop".into(),
        name: "App".into(),
        technology: None,
    });
    artifacts.components.push(s5d::Component {
        id: "orders-backend".into(),
        feature: feature.id.clone(),
        domain: "orders".into(),
        container: "app".into(),
        name: "Orders backend".into(),
        paths: vec!["backend/**".into()],
    });
    fs::write(&spec, serde_yaml::to_string(&feature).unwrap()).unwrap();

    run_ok(repo.path(), ["discover", "sync"]);
    run_ok(repo.path(), ["discover", "check"]);

    let discovery_dir = repo.path().join(".s5d").join("discovery");
    assert!(discovery_dir.join("manifest.yaml").exists());
    assert!(discovery_dir.join("files.jsonl").exists());
    assert!(discovery_dir.join("evidence.jsonl").exists());
    assert!(discovery_dir.join("graph.json").exists());
    assert!(discovery_dir.join("metamodel.yaml").exists());

    let files = fs::read_to_string(discovery_dir.join("files.jsonl")).unwrap();
    assert!(files.contains("backend/orders.py"));
    assert!(files.contains("web/features/orders.ts"));
    assert!(files.contains(".s5d/packages/"));

    let graph: s5d::DiscoveryGraph =
        serde_json::from_str(&fs::read_to_string(discovery_dir.join("graph.json")).unwrap())
            .unwrap();
    assert!(graph.nodes.iter().any(|node| node.id == "domain:orders"));
    assert!(graph
        .nodes
        .iter()
        .any(|node| node.id == "component_candidate:backend"));
    assert!(graph
        .edges
        .iter()
        .any(|edge| edge.kind == "claims_path" && edge.to == "path:backend/**"));
    assert!(graph.nodes.iter().any(|node| node.id == "path:backend/**"));
    assert!(graph
        .edges
        .iter()
        .any(|edge| edge.kind == "contains_component"
            && edge.from == "container:app"
            && edge.to == "component:orders-backend"));

    let metamodel: s5d::DiscoveryMetamodel =
        serde_yaml::from_str(&fs::read_to_string(discovery_dir.join("metamodel.yaml")).unwrap())
            .unwrap();
    assert!(metamodel
        .domains
        .iter()
        .any(|domain| domain.id == "domain:orders"));
    assert!(metamodel
        .components
        .iter()
        .any(|component| component.id == "component:orders-backend"));
    assert!(metamodel
        .systems
        .iter()
        .any(|system| system.id == "system:shop"));
    assert!(metamodel
        .containers
        .iter()
        .any(|container| container.id == "container:app"));
}

#[test]
fn discovery_respects_ignore_files_without_hiding_dotfiles_by_default() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    repo.write(".gitignore", ".uv-cache/\nignored.py\n");
    repo.write(".ignore", "scratch/\n");
    repo.write("src/lib.rs", "pub fn marker() {}\n");
    repo.write(".github/workflows/ci.yml", "name: ci\n");
    repo.write(".uv-cache/archive/generated.py", "print('cache')\n");
    repo.write("scratch/generated.py", "print('scratch')\n");
    repo.write("ignored.py", "print('ignored')\n");

    run_ok(repo.path(), ["discover", "sync"]);

    let discovery_dir = repo.path().join(".s5d").join("discovery");
    let files = fs::read_to_string(discovery_dir.join("files.jsonl")).unwrap();
    assert!(files.contains("src/lib.rs"));
    assert!(files.contains(".github/workflows/ci.yml"));
    assert!(!files.contains(".uv-cache/archive/generated.py"));
    assert!(!files.contains("scratch/generated.py"));
    assert!(!files.contains("ignored.py"));
}

#[test]
fn discovery_check_fails_when_snapshot_is_missing() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    repo.write("src/lib.rs", "pub fn marker() {}\n");

    let outcome = run_fail(repo.path(), ["discover", "check"]);
    assert!(outcome.stderr.contains("snapshot missing"));
    assert!(outcome.stderr.contains("s5d discover sync"));
}

#[test]
fn discovery_check_fails_when_snapshot_is_stale() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    repo.write("src/lib.rs", "pub fn first() {}\n");

    run_ok(repo.path(), ["discover", "sync"]);
    run_ok(repo.path(), ["discover", "check"]);

    repo.write("src/second.rs", "pub fn second() {}\n");
    let outcome = run_fail(repo.path(), ["discover", "check"]);
    assert!(outcome.stderr.contains("discovery is stale"));
    assert!(outcome.stderr.contains("s5d discover sync"));
}

#[test]
fn discovery_sync_supports_target_path_and_custom_output_dir() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    repo.write("backend/orders.py", "class Order:\n    pass\n");
    repo.write("web/orders.ts", "export const orderList = [];\n");

    run_ok(
        repo.path(),
        [
            "discover",
            "sync",
            "backend",
            "--out",
            ".s5d/discovery-backend",
        ],
    );
    run_ok(
        repo.path(),
        [
            "discover",
            "check",
            "backend",
            "--out",
            ".s5d/discovery-backend",
        ],
    );

    let discovery_dir = repo.path().join(".s5d").join("discovery-backend");
    let manifest: s5d::DiscoveryManifest =
        serde_yaml::from_str(&fs::read_to_string(discovery_dir.join("manifest.yaml")).unwrap())
            .unwrap();
    assert_eq!(manifest.target, "backend");

    let files = fs::read_to_string(discovery_dir.join("files.jsonl")).unwrap();
    assert!(files.contains("backend/orders.py"));
    assert!(!files.contains("web/orders.ts"));
}

#[test]
fn discovery_rejects_target_outside_project_root() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    let outside = TempDir::new().unwrap();
    fs::write(outside.path().join("outside.py"), "print('outside')\n").unwrap();
    let outside_path = outside.path().to_string_lossy().to_string();

    let outcome = run_fail(repo.path(), ["discover", "sync", outside_path.as_str()]);
    assert!(outcome.stderr.contains("outside project root"));
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

    let mut links = s5d::Links {
        component_to_capability: vec![
            binding(&[
                ("component", "comp.auth"),
                ("capability", "cap.auth.tokens"),
            ]),
            binding(&[
                ("component", "comp.billing"),
                ("capability", "cap.billing.charge"),
            ]),
        ],
        ..Default::default()
    };
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
        intent_kernel: None,
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
        links: Some(s5d::Links {
            component_to_capability: vec![binding(&[
                ("component", "comp.app"),
                ("capability", "cap.app.source"),
            ])],
            ..Default::default()
        }),
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
        intent_kernel: None,
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
    for public in [
        "init", "new", "decision", "verify", "state", "run", "status", "show", "trace", "admin",
        "ci",
    ] {
        assert!(
            help.stdout
                .lines()
                .any(|line| line.trim_start().starts_with(&format!("{public} "))),
            "top-level help should keep public command `{public}`:\n{}",
            help.stdout
        );
    }
    for hidden in [
        "note",
        "add-hypothesis",
        "add-evidence",
        "decide",
        "validate",
        "graph-check",
        "check",
        "preview",
        "approve",
        "run-gates",
        "import",
        "apply",
        "phase",
        "search",
        "drift-check",
        "execute",
        "harness",
        "reconcile",
        "rollback",
        "reflect",
        "route",
        "codebase",
        "discover",
        "hook",
        "update",
        "install",
        "index",
        "gate",
        "bootstrap",
        "cg",
        "mcp",
    ] {
        assert!(
            !help
                .stdout
                .lines()
                .any(|line| line.trim_start().starts_with(&format!("{hidden} "))),
            "top-level help should hide internal command `{hidden}`:\n{}",
            help.stdout
        );
    }
    let visible_public_count = [
        "init", "new", "decision", "verify", "state", "run", "status", "show", "trace", "admin",
    ]
    .iter()
    .filter(|command| {
        help.stdout
            .lines()
            .any(|line| line.trim_start().starts_with(&format!("{command} ")))
    })
    .count();
    assert_eq!(
        visible_public_count, 10,
        "top-level public surface should be exactly 10 commands:\n{}",
        help.stdout
    );

    let decision_help = run_ok(repo.path(), ["decision", "--help"]);
    for public in ["add-hypothesis", "add-evidence", "decide"] {
        assert!(
            decision_help
                .stdout
                .lines()
                .any(|line| line.trim_start().starts_with(&format!("{public} "))),
            "decision help should expose `{public}`:\n{}",
            decision_help.stdout
        );
    }

    let verify_help = run_ok(repo.path(), ["verify", "--help"]);
    for public in ["validate", "graph-check", "check", "run-gates"] {
        assert!(
            verify_help
                .stdout
                .lines()
                .any(|line| line.trim_start().starts_with(&format!("{public} "))),
            "verify help should expose `{public}`:\n{}",
            verify_help.stdout
        );
    }

    let state_help = run_ok(repo.path(), ["state", "--help"]);
    for public in [
        "preview",
        "approve",
        "import",
        "drift-check",
        "reconcile",
        "rollback",
        "reflect",
    ] {
        assert!(
            state_help
                .stdout
                .lines()
                .any(|line| line.trim_start().starts_with(&format!("{public} "))),
            "state help should expose `{public}`:\n{}",
            state_help.stdout
        );
    }

    let run_help = run_ok(repo.path(), ["run", "--help"]);
    for public in ["list", "start", "accept", "exec", "task", "harness"] {
        assert!(
            run_help
                .stdout
                .lines()
                .any(|line| line.trim_start().starts_with(&format!("{public} "))),
            "run help should expose `{public}`:\n{}",
            run_help.stdout
        );
    }

    let admin_help = run_ok(repo.path(), ["admin", "--help"]);
    for public in ["install", "update"] {
        assert!(
            admin_help
                .stdout
                .lines()
                .any(|line| line.trim_start().starts_with(&format!("{public} "))),
            "admin help should expose `{public}`:\n{}",
            admin_help.stdout
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
fn decision_group_and_legacy_alias_mutate_same_spec() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    run_ok(
        repo.path(),
        [
            "new",
            "decision.cli-group",
            "--tier",
            "decision",
            "--question",
            "How should grouped commands be exposed?",
            "--product",
            "s5d",
        ],
    );
    let spec_path = only_spec_path(&repo);
    let spec_str = spec_path.to_str().unwrap();

    run_ok(
        repo.path(),
        [
            "decision",
            "add-hypothesis",
            spec_str,
            "--title",
            "Grouped commands",
            "--content",
            "Expose decision/verify/apply as command groups",
            "--scope",
            "CLI",
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
            "Legacy aliases",
            "--content",
            "Keep old top-level command names callable",
            "--scope",
            "CLI compatibility",
            "--kind",
            "system",
        ],
    );
    run_ok(
        repo.path(),
        [
            "decision",
            "add-evidence",
            spec_str,
            "--hypothesis-id",
            "grouped-commands",
            "--evidence-type",
            "synthetic:test",
            "--content",
            "Grouped and legacy command paths write through the same implementation",
            "--verdict",
            "pass",
            "--formality",
            "4",
            "--claim-scope",
            "cli",
            "--reliability",
            "0.9",
        ],
    );

    let spec: s5d::Spec = load_yaml(&spec_path);
    assert_eq!(spec.hypotheses.len(), 2);
    assert_eq!(spec.hypotheses[0].evidence.len(), 1);
}

#[test]
fn verify_and_apply_groups_preserve_legacy_aliases() {
    let repo = StandaloneRepo::new();
    seed_searchable_rust_repo(&repo);
    run_ok(repo.path(), ["init"]);
    configure_gate_command(
        &repo,
        "schema",
        vec![
            s5d_bin().to_string(),
            "verify".to_string(),
            "validate".to_string(),
            "{package}".to_string(),
        ],
    );

    run_ok(
        repo.path(),
        [
            "new",
            "feat.cli.grouped-apply",
            "--tier",
            "lightweight",
            "--product",
            "s5d",
        ],
    );
    let spec_path = only_spec_path(&repo);
    let spec_str = spec_path.to_str().unwrap();
    {
        let mut spec: s5d::Spec = load_yaml(&spec_path);
        materialize_scaffold_paths(&mut spec);
        if let Some(ref mut artifacts) = spec.artifacts {
            artifacts.capabilities.push(s5d::Capability {
                id: "cap.GroupedApplyFlow".into(),
                domain: "dom.s5d.core".into(),
                name: "GroupedApplyFlow".into(),
                description: Some("Exercise grouped verify/apply CLI".into()),
                since: None,
            });
        }
        fs::write(&spec_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
    }

    run_ok(repo.path(), ["validate", spec_str]);
    run_ok(repo.path(), ["verify", "validate", spec_str]);
    run_ok(repo.path(), ["verify", "graph-check", spec_str]);
    run_ok(repo.path(), ["preview", spec_str]);
    run_ok(repo.path(), ["state", "preview", spec_str]);
    run_ok(
        repo.path(),
        ["state", "approve", spec_str, "--reviewer", "Roman"],
    );
    let gates = run_ok(repo.path(), ["verify", "run-gates", spec_str]);
    assert!(gates.stdout.contains("gate:schema"), "{}", gates.summary());

    let legacy_import_without_verifier = run_fail(repo.path(), ["import", spec_str]);
    assert!(
        legacy_import_without_verifier
            .stderr
            .contains("--verified-by"),
        "{}",
        legacy_import_without_verifier.summary()
    );
    let import = run_ok(
        repo.path(),
        ["state", "import", spec_str, "--verified-by", "Diana"],
    );
    assert!(import.stdout.contains("Imported"), "{}", import.summary());

    let drift = run_ok(repo.path(), ["state", "drift-check", spec_str]);
    assert!(drift.stdout.contains("synced"), "{}", drift.summary());
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
        materialize_scaffold_paths(&mut spec);
        if let Some(ref mut a) = spec.artifacts {
            a.capabilities.push(s5d::Capability {
                id: "cap.CalculateRatio".into(),
                domain: "dom.Billing.core".into(),
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
        materialize_scaffold_paths(&mut spec);
        if let Some(ref mut artifacts) = spec.artifacts {
            artifacts.capabilities.push(s5d::Capability {
                id: "cap.RunOperatorLoop".into(),
                domain: "dom.Billing.core".into(),
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

    let phases = run_ok(repo.path(), ["run", "list", spec_str]);
    assert!(phases.stdout.contains("Prototype"), "{}", phases.summary());
    assert!(phases.stdout.contains("prototype"), "{}", phases.summary());

    let execute_before_start = run_fail(
        repo.path(),
        [
            "run",
            "task",
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

    let start = run_ok(repo.path(), ["run", "start", spec_str, "--id", "prototype"]);
    assert!(
        start.stdout.contains("Active work state"),
        "{}",
        start.summary()
    );

    let status = run_ok(repo.path(), ["status"]);
    assert!(
        status.stdout.contains("Active work state:"),
        "{}",
        status.summary()
    );
    assert!(status.stdout.contains("prototype"), "{}", status.summary());

    let execute = run_ok(
        repo.path(),
        [
            "run",
            "task",
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
            "run",
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
        materialize_scaffold_paths(&mut spec);
        if let Some(ref mut artifacts) = spec.artifacts {
            artifacts.capabilities.push(s5d::Capability {
                id: "cap.RunExternalEngine".into(),
                domain: "dom.Billing.core".into(),
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
            "run",
            "exec",
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
            .contains("must be active before run exec"),
        "{}",
        before_start.summary()
    );

    run_ok(repo.path(), ["run", "start", spec_str, "--id", "prototype"]);

    let unapproved = run_fail(
        repo.path(),
        [
            "run",
            "exec",
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
            "run",
            "exec",
            spec_str,
            "--id",
            "prototype",
            "--engine",
            "local-s5d",
        ],
    );
    assert!(run.stdout.contains("Run completed"), "{}", run.summary());
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
        materialize_scaffold_paths(&mut spec);
        if let Some(ref mut artifacts) = spec.artifacts {
            artifacts.capabilities.push(s5d::Capability {
                id: "cap.OperationalHarness".into(),
                domain: "dom.Billing.core".into(),
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
            "run",
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
            "run",
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

    let status = run_ok(repo.path(), ["run", "harness", "status", "alpha"]);
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
            "run",
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
        .any(|event| event.kind == "work_state_started"));
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
    materialize_scaffold_paths(&mut spec);
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
        import.stderr.contains("all effective gates must pass"),
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
    materialize_scaffold_paths(&mut spec);
    if let Some(ref mut artifacts) = spec.artifacts {
        artifacts.capabilities.push(s5d::Capability {
            id: "cap.TimeoutGate".into(),
            domain: "dom.Billing.core".into(),
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
        import.stderr.contains("all effective gates must pass"),
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
fn init_recovers_missing_core_files_without_touching_corrupted_ones() {
    // Scenario #14 (partial corruption): deleting config/ledger/index must
    // not brick the project — re-running init restores the missing files.
    // Corrupted (present but unparseable) files are NOT overwritten:
    // repairing those is a data-loss decision that stays with the user.
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    let s5d_dir = repo.path().join(".s5d");

    for f in ["config.yaml", "ledger.yaml", "index.yaml"] {
        fs::remove_file(s5d_dir.join(f)).unwrap();
    }

    let recovered = run_ok(repo.path(), ["init"]);
    for f in ["config.yaml", "ledger.yaml", "index.yaml"] {
        assert!(
            s5d_dir.join(f).exists(),
            "init must restore missing {f}:\n{}",
            recovered.summary()
        );
        assert!(
            recovered.stdout.contains(f),
            "restored {f} must be reported in init output:\n{}",
            recovered.summary()
        );
    }

    // Restored ledger is functional: a spec can run the full lifecycle.
    let spec_str = setup_standard_spec(&repo, "feat.recovery");
    run_ok(repo.path(), ["preview", &spec_str]);
    run_ok(repo.path(), ["approve", &spec_str, "--reviewer", "R"]);
    run_ok(repo.path(), ["run-gates", &spec_str]);
    run_ok(repo.path(), ["import", &spec_str, "--verified-by", "V"]);

    // Corrupted file: init must leave the bytes exactly as found.
    let corrupted = "{{{{ not yaml at all";
    fs::write(s5d_dir.join("config.yaml"), corrupted).unwrap();
    run_ok(repo.path(), ["init"]);
    assert_eq!(
        fs::read_to_string(s5d_dir.join("config.yaml")).unwrap(),
        corrupted,
        "init must never overwrite an existing (even corrupted) core file"
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
  component_to_capability:
  - component: comp.widget_handler
    capability: cap.DoThing
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
        links: Some(s5d::Links {
            component_to_capability: vec![binding(&[
                ("component", "comp.s5d.skill-docs"),
                ("capability", "cap.s5d.shape-product-intent"),
            ])],
            ..Default::default()
        }),
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
        intent_kernel: None,
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
fn architecture_check_blocks_component_path_escape() {
    let repo = StandaloneRepo::new();
    seed_architecture_lint_repo(&repo);
    run_ok(repo.path(), ["init"]);

    let spec_path = write_architecture_lint_spec(&repo, true);
    let mut spec: s5d::Spec = load_yaml(&spec_path);
    materialize_scaffold_paths(&mut spec);
    let artifacts = spec.artifacts.as_mut().unwrap();
    let billing = artifacts
        .components
        .iter_mut()
        .find(|component| component.id == "comp.billing")
        .unwrap();
    billing.paths = vec!["../outside.rs".into()];
    fs::write(&spec_path, serde_yaml::to_string(&spec).unwrap()).unwrap();

    let result = run_fail(repo.path(), ["check", spec_path.to_str().unwrap()]);
    assert!(
        result.stderr.contains("project root") && result.stderr.contains("../outside.rs"),
        "architecture check should block component paths outside repo root:\n{}",
        result.summary()
    );
}

#[test]
fn validate_blocks_component_without_capability_binding() {
    let repo = StandaloneRepo::new();
    seed_architecture_lint_repo(&repo);
    run_ok(repo.path(), ["init"]);

    let spec_path = write_architecture_lint_spec(&repo, true);
    let mut spec: s5d::Spec = load_yaml(&spec_path);
    materialize_scaffold_paths(&mut spec);
    spec.links
        .as_mut()
        .unwrap()
        .component_to_capability
        .retain(|binding| {
            binding
                .fields
                .get("component")
                .is_none_or(|component| component != "comp.billing")
        });
    fs::write(&spec_path, serde_yaml::to_string(&spec).unwrap()).unwrap();

    let result = run_fail(repo.path(), ["validate", spec_path.to_str().unwrap()]);
    assert!(
        result.stderr.contains("comp.billing")
            && result.stderr.contains("component_to_capability")
            && result.stderr.contains("code cannot trace"),
        "validation should block untraceable components:\n{}",
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
    remote.write(
        "skills/s5d/references/fpf/agent/load-policy.md",
        "load policy\n",
    );
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
  component_to_capability:
  - component: comp.alpha_handler
    capability: cap.DoAlpha
  - component: comp.beta_handler
    capability: cap.DoBeta
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
    materialize_scaffold_paths(&mut spec);
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
    artifacts.systems.push(s5d::SoftwareSystem {
        id: "sys.test".into(),
        product: "shop".into(),
        name: "Test system".into(),
    });
    artifacts.containers.push(s5d::Container {
        id: "ctr.api".into(),
        name: "API".into(),
        system: "sys.test".into(),
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
    let links = spec.links.get_or_insert_with(s5d::Links::default);
    links.component_to_capability.push(binding(&[
        ("component", "comp.order_service"),
        ("capability", "cap.CreateOrder"),
    ]));
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

#[test]
fn synthetic_project_development_cycle_tracks_multiple_features_and_code() {
    let repo = StandaloneRepo::new();
    repo.write(
        "Cargo.toml",
        "[package]\nname = \"minishop\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    repo.write("src/lib.rs", "pub fn health() -> &'static str { \"ok\" }\n");
    run_ok(repo.path(), ["init"]);

    run_ok(
        repo.path(),
        [
            "new",
            "decision.minishop.checkout-risk",
            "--tier",
            "decision",
            "--question",
            "How should MiniShop score checkout risk?",
            "--product",
            "minishop",
        ],
    );
    let decision_path = spec_path_by_id(&repo, "decision.minishop.checkout-risk");
    let decision_str = decision_path.to_str().unwrap();
    {
        let mut decision: s5d::Spec = load_yaml(&decision_path);
        let problem = match decision.problem.as_mut() {
            Some(s5d::ProblemField::Card(card)) => card,
            other => panic!("decision spec should have a problem card, got {other:?}"),
        };
        problem.acceptance = Some(
            "Checkout risk can be traced to component paths; chosen policy has synthetic evidence; feature implementation is validated by gates"
                .into(),
        );
        fs::write(&decision_path, serde_yaml::to_string(&decision).unwrap()).unwrap();
    }

    for (id, title, content) in [
        (
            "inline-risk-policy",
            "Inline risk policy",
            "Score risk synchronously inside checkout.",
        ),
        (
            "async-risk-queue",
            "Async risk queue",
            "Emit checkout events and score risk asynchronously.",
        ),
        (
            "manual-review-only",
            "Manual review only",
            "Route suspicious orders to manual review without automated scoring.",
        ),
    ] {
        run_ok(
            repo.path(),
            [
                "decision",
                "add-hypothesis",
                decision_str,
                "--id",
                id,
                "--title",
                title,
                "--content",
                content,
                "--scope",
                "checkout",
                "--kind",
                "system",
                "--prompt",
                "How should MiniShop score checkout risk?",
                "--next-move",
                "build",
            ],
        );
        run_ok(
            repo.path(),
            [
                "decision",
                "add-evidence",
                decision_str,
                "--hypothesis-id",
                id,
                "--evidence-type",
                "synthetic:test",
                "--content",
                "Synthetic product fixture compares traceability, latency, and implementation radius.",
                "--verdict",
                "pass",
                "--formality",
                "4",
                "--claim-scope",
                "minishop.checkout",
                "--reliability",
                "0.82",
            ],
        );
    }

    run_ok(
        repo.path(),
        [
            "new",
            "feat.minishop.checkout-risk",
            "--tier",
            "standard",
            "--product",
            "minishop",
            "--hypothesis-id",
            "inline-risk-policy",
        ],
    );
    let risk_spec_path = spec_path_by_id(&repo, "feat.minishop.checkout-risk");

    run_ok(
        repo.path(),
        [
            "new",
            "feat.minishop.receipts",
            "--tier",
            "standard",
            "--product",
            "minishop",
        ],
    );
    let receipts_spec_path = spec_path_by_id(&repo, "feat.minishop.receipts");

    populate_minishop_feature_spec(
        &risk_spec_path,
        "dom.minishop.checkout",
        "Checkout",
        "cap.minishop.score-risk",
        "Score Checkout Risk",
        "comp.minishop.risk",
        "Checkout Risk Engine",
        vec!["src/lib.rs", "src/risk.rs"],
    );
    populate_minishop_feature_spec(
        &receipts_spec_path,
        "dom.minishop.receipts",
        "Receipts",
        "cap.minishop.issue-receipt",
        "Issue Receipt",
        "comp.minishop.receipts",
        "Receipt Formatter",
        vec!["src/receipts.rs"],
    );

    run_ok(
        repo.path(),
        [
            "decision",
            "decide",
            decision_str,
            "--title",
            "Use inline checkout risk policy",
            "--winner",
            "inline-risk-policy",
            "--rejected",
            "async-risk-queue,manual-review-only",
            "--confirmed-by",
            "SyntheticReviewer",
            "--context",
            "MiniShop needs deterministic local behavior in the synthetic fixture.",
            "--decision",
            "Score checkout risk inline and bind the implementation to src/risk.rs.",
            "--rationale",
            "Inline scoring gives the smallest implementation radius while preserving code traceability.",
            "--consequences",
            "Checkout stays synchronous; async queue can be reconsidered if latency evidence changes.",
            "--challenge-summary",
            "Checked async and manual-review alternatives against traceability and latency constraints.",
            "--decision-subject",
            "checkout risk policy",
            "--decision-subject-granularity",
            "component",
            "--evaluative-surface",
            "Pareto dimensions: traceability, latency, implementation radius",
            "--belief-state",
            "Synthetic checkout volume fits inline scoring.",
            "--outcome-model",
            "cargo check and S5D trace should prove the mapping after implementation.",
            "--pareto-set",
            "inline-risk-policy,async-risk-queue",
            "--choice-rule",
            "Choose the candidate with direct traceability and no extra runtime worker.",
        ],
    );

    let risk_spec = risk_spec_path.to_str().unwrap();
    run_ok(repo.path(), ["verify", "validate", risk_spec]);
    run_ok(repo.path(), ["verify", "graph-check", risk_spec]);
    run_ok(repo.path(), ["state", "preview", risk_spec]);
    run_ok(
        repo.path(),
        [
            "state",
            "approve",
            risk_spec,
            "--reviewer",
            "SyntheticReviewer",
        ],
    );
    configure_engine(
        &repo,
        "synthetic-agent",
        true,
        vec![s5d_bin().into(), "show".into(), "{spec}".into()],
    );
    run_ok(repo.path(), ["run", "start", risk_spec, "--id", "build"]);
    let engine_run = run_ok(
        repo.path(),
        [
            "run",
            "exec",
            risk_spec,
            "--id",
            "build",
            "--engine",
            "synthetic-agent",
        ],
    );
    assert!(
        engine_run.stdout.contains("Run completed"),
        "{}",
        engine_run.summary()
    );
    run_ok(
        repo.path(),
        [
            "run",
            "accept",
            risk_spec,
            "--id",
            "build",
            "--reviewer",
            "SyntheticReviewer",
        ],
    );

    repo.write(
        "src/lib.rs",
        "pub mod risk;\n\npub fn health() -> &'static str { \"ok\" }\n\npub fn checkout_risk_cents(total_cents: u32) -> u8 {\n    risk::checkout_risk_cents(total_cents)\n}\n",
    );
    repo.write(
        "src/risk.rs",
        "pub fn checkout_risk_cents(total_cents: u32) -> u8 {\n    if total_cents > 10_000 { 80 } else { 10 }\n}\n",
    );
    let cargo_check = Command::new("cargo")
        .arg("check")
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(
        cargo_check.status.success(),
        "synthetic project should compile:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&cargo_check.stdout),
        String::from_utf8_lossy(&cargo_check.stderr)
    );
    run_ok(repo.path(), ["verify", "check", risk_spec]);
    run_ok(repo.path(), ["verify", "run-gates", risk_spec]);
    run_ok(
        repo.path(),
        [
            "state",
            "import",
            risk_spec,
            "--verified-by",
            "SyntheticVerifier",
        ],
    );
    run_ok(
        repo.path(),
        [
            "state",
            "reflect",
            risk_spec,
            "--summary",
            "Synthetic checkout risk feature compiles and traces to its decision.",
            "--verdict",
            "confirmed",
            "--measurement-window",
            "synthetic fixture",
            "--telemetry",
            "cargo-check:pass",
            "--heuristic",
            "Decision-backed code must trace from component path to accepted decision.",
        ],
    );

    repo.write(
        "src/lib.rs",
        "pub mod receipts;\npub mod risk;\n\npub fn health() -> &'static str { \"ok\" }\n\npub fn checkout_risk_cents(total_cents: u32) -> u8 {\n    risk::checkout_risk_cents(total_cents)\n}\n\npub fn receipt_for(order_id: u64) -> String {\n    receipts::receipt_for(order_id)\n}\n",
    );
    repo.write(
        "src/receipts.rs",
        "pub fn receipt_for(order_id: u64) -> String {\n    format!(\"receipt-{order_id}\")\n}\n",
    );
    let cargo_check = Command::new("cargo")
        .arg("check")
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(
        cargo_check.status.success(),
        "synthetic project with two features should compile:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&cargo_check.stdout),
        String::from_utf8_lossy(&cargo_check.stderr)
    );

    let receipts_spec = receipts_spec_path.to_str().unwrap();
    run_ok(repo.path(), ["verify", "validate", receipts_spec]);
    run_ok(repo.path(), ["verify", "graph-check", receipts_spec]);
    run_ok(repo.path(), ["state", "preview", receipts_spec]);
    run_ok(
        repo.path(),
        [
            "state",
            "approve",
            receipts_spec,
            "--reviewer",
            "SyntheticReviewer",
        ],
    );
    run_ok(repo.path(), ["verify", "check", receipts_spec]);
    run_ok(repo.path(), ["verify", "run-gates", receipts_spec]);
    run_ok(
        repo.path(),
        [
            "state",
            "import",
            receipts_spec,
            "--verified-by",
            "SyntheticVerifier",
        ],
    );

    let risk_trace = run_ok(repo.path(), ["trace", "src/risk.rs"]);
    assert!(
        risk_trace.stdout.contains("feat.minishop.checkout-risk")
            && risk_trace.stdout.contains("comp.minishop.risk")
            && risk_trace.stdout.contains("cap.minishop.score-risk")
            && risk_trace
                .stdout
                .contains("decision.minishop.checkout-risk [accepted]"),
        "risk trace should connect code to component, capability, and accepted decision:\n{}",
        risk_trace.summary()
    );
    let receipts_trace = run_ok(repo.path(), ["trace", "src/receipts.rs"]);
    assert!(
        receipts_trace.stdout.contains("feat.minishop.receipts")
            && receipts_trace.stdout.contains("comp.minishop.receipts")
            && receipts_trace.stdout.contains("cap.minishop.issue-receipt"),
        "receipts trace should connect code to its feature metamodel:\n{}",
        receipts_trace.summary()
    );

    let codebase_sync = run_ok(repo.path(), ["codebase", "sync"]);
    assert!(
        codebase_sync
            .stdout
            .contains("1 governed, 0 partial, 0 blind"),
        "codebase coverage should be fully governed after both feature specs:\n{}",
        codebase_sync.summary()
    );
    run_ok(repo.path(), ["codebase", "check"]);
    run_ok(repo.path(), ["discover", "sync"]);
    run_ok(repo.path(), ["discover", "check"]);
    let drift = run_ok(repo.path(), ["state", "drift-check"]);
    assert!(drift.stdout.contains("synced"), "{}", drift.summary());

    let risk_record: s5d::Record = load_yaml(&record_path_for(&risk_spec_path));
    assert_eq!(risk_record.status, s5d::SpecStatus::Operated);
    assert_eq!(risk_record.sync_status, s5d::SyncStatus::Synced);
    assert!(risk_record.decision.is_none());
    assert!(risk_record.reflection.is_some());
    assert!(risk_record
        .phase_runs
        .iter()
        .any(|run| run.engine == "synthetic-agent" && run.status == "completed"));

    let decision_record: s5d::Record = load_yaml(&record_path_for(&decision_path));
    assert!(decision_record.decision.is_some());
}

#[allow(clippy::too_many_arguments)] // test-scaffold helper; a params struct adds no value here
fn populate_minishop_feature_spec(
    spec_path: &Path,
    domain_id: &str,
    domain_name: &str,
    capability_id: &str,
    capability_name: &str,
    component_id: &str,
    component_name: &str,
    paths: Vec<&str>,
) {
    let mut spec: s5d::Spec = load_yaml(spec_path);
    materialize_scaffold_paths(&mut spec);
    let artifacts = spec
        .artifacts
        .as_mut()
        .expect("feature should have artifacts");
    // Replace the scaffold's placeholder chain — this test composes its own
    // architecture, and the TODO-paths scaffold component would (correctly)
    // fail the architecture check.
    artifacts.domains.clear();
    artifacts.capabilities.clear();
    artifacts.systems.clear();
    artifacts.containers.clear();
    artifacts.components.clear();
    if let Some(ref mut links) = spec.links {
        links.feature_to_domain.clear();
        links.component_to_capability.clear();
    }
    let artifacts = spec
        .artifacts
        .as_mut()
        .expect("feature should have artifacts");
    artifacts.domains.push(s5d::Domain {
        id: domain_id.into(),
        product: "minishop".into(),
        name: domain_name.into(),
        classification: Some("core".into()),
        description: None,
        team: None,
        maturity_level: None,
    });
    artifacts.capabilities.push(s5d::Capability {
        id: capability_id.into(),
        domain: domain_id.into(),
        name: capability_name.into(),
        description: None,
        since: None,
    });
    artifacts.systems.push(s5d::SoftwareSystem {
        id: "sys.minishop".into(),
        product: "minishop".into(),
        name: "MiniShop".into(),
    });
    artifacts.containers.push(s5d::Container {
        id: "ctr.minishop.lib".into(),
        system: "sys.minishop".into(),
        name: "MiniShop Library".into(),
        technology: Some("Rust".into()),
    });
    artifacts.components.push(s5d::Component {
        id: component_id.into(),
        feature: spec.id.clone(),
        domain: domain_id.into(),
        container: "ctr.minishop.lib".into(),
        name: component_name.into(),
        paths: paths.into_iter().map(str::to_string).collect(),
    });
    let links = spec.links.get_or_insert_with(s5d::Links::default);
    links.component_to_capability.push(s5d::Binding {
        fields: std::collections::HashMap::from([
            ("component".into(), component_id.into()),
            ("capability".into(), capability_id.into()),
        ]),
    });
    if !spec.gates.iter().any(|gate| gate.kind == "architecture") {
        spec.gates.push(s5d::Gate {
            kind: "architecture".into(),
        });
    }
    fs::write(spec_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
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

#[test]
fn high_tier_full_flow_with_configured_test_and_contract_gates() {
    // Scenario #4: a high-assurance spec passes the WHOLE flow — declared
    // test/contract gates run real commands, the built-in review gate is
    // satisfied by recorded review evidence, and import needs no --force.
    let repo = StandaloneRepo::new();
    seed_searchable_rust_repo(&repo);
    run_ok(repo.path(), ["init"]);
    for gate in ["test", "contract"] {
        configure_gate_command(
            &repo,
            gate,
            vec![
                s5d_bin().to_string(),
                "validate".to_string(),
                "{package}".to_string(),
            ],
        );
    }

    run_ok(
        repo.path(),
        [
            "new",
            "feat.demo.payments",
            "--tier",
            "high",
            "--product",
            "demo",
        ],
    );
    let spec_path = only_spec_path(&repo);
    let spec_str = spec_path.to_str().unwrap().to_string();

    {
        let mut spec: s5d::Spec = load_yaml(&spec_path);
        materialize_scaffold_paths(&mut spec);
        // High scaffold ships schema+graph+review; declare the configured
        // command gates on top.
        spec.gates.push(s5d::Gate {
            kind: "test".into(),
        });
        spec.gates.push(s5d::Gate {
            kind: "contract".into(),
        });
        fs::write(&spec_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
    }

    // The built-in review gate requires recorded review evidence on a
    // hypothesis — model the implementation approach and review it.
    run_ok(
        repo.path(),
        [
            "add-hypothesis",
            &spec_str,
            "--title",
            "Charge via provider tokens",
            "--content",
            "Tokenized charges, no PAN storage",
            "--scope",
            "payments",
        ],
    );
    run_ok(
        repo.path(),
        [
            "add-evidence",
            &spec_str,
            "--hypothesis-id",
            "charge-via-provider-tokens",
            "--evidence-type",
            "gate:review",
            "--content",
            "cross-review: token flow audited, no PAN at rest",
            "--verdict",
            "pass",
        ],
    );

    run_ok(repo.path(), ["preview", &spec_str]);
    run_ok(
        repo.path(),
        ["approve", &spec_str, "--reviewer", "TestReviewer"],
    );
    let gates = run_ok(repo.path(), ["run-gates", &spec_str]);
    assert!(
        gates.stdout.contains("passed: 5"),
        "schema+graph+review+test+contract must all pass:\n{}",
        gates.summary()
    );
    run_ok(
        repo.path(),
        ["import", &spec_str, "--verified-by", "TestVerifier"],
    );

    // Scenario #11 (show smoke): the imported spec renders.
    let shown = run_ok(repo.path(), ["show", &spec_str]);
    assert!(
        shown.stdout.contains("feat.demo.payments"),
        "show must render the imported high spec:\n{}",
        shown.summary()
    );

    let drift = run_ok(repo.path(), ["drift-check", &spec_str]);
    assert!(
        drift.stdout.contains("synced"),
        "fresh import must be synced:\n{}",
        drift.summary()
    );
}

#[test]
fn decision_tier_clean_end_to_end_without_force() {
    // Scenario #5: the full decision lifecycle must be passable WITHOUT
    // --force. The methodological prerequisites (≥3 hypotheses, evidence on
    // each, acceptance criteria on the problem card, challenge summary,
    // gate:review evidence) define the canonical clean path — if this test
    // ever needs --force, the framework's own happy path is broken.
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    run_ok(
        repo.path(),
        [
            "new",
            "decision.demo.store",
            "--tier",
            "decision",
            "--question",
            "Which store backs the tracker?",
            "--product",
            "demo",
        ],
    );
    let spec_path = only_spec_path(&repo);
    let spec_str = spec_path.to_str().unwrap().to_string();

    // Acceptance criteria on the problem card (required before deciding).
    {
        let mut spec: s5d::Spec = load_yaml(&spec_path);
        if let Some(s5d::ProblemField::Card(ref mut card)) = spec.problem {
            card.acceptance = Some("Store survives 10k events/day with zero data loss".into());
        } else {
            panic!("decision scaffold must carry a problem card");
        }
        fs::write(&spec_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
    }

    // Three genuinely distinct hypotheses.
    for (title, content) in [
        ("SQLite", "Single-file embedded store"),
        ("Postgres", "Dedicated relational server"),
        ("JSONL", "Append-only log files"),
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
                "tracker storage",
            ],
        );
    }

    // Evidence on every hypothesis; the winner carries a gate:review pass.
    run_ok(
        repo.path(),
        [
            "add-evidence",
            &spec_str,
            "--hypothesis-id",
            "sqlite",
            "--evidence-type",
            "gate:review",
            "--content",
            "cross-review: fits constraints, single-writer acceptable",
            "--verdict",
            "pass",
        ],
    );
    for loser in ["postgres", "jsonl"] {
        run_ok(
            repo.path(),
            [
                "add-evidence",
                &spec_str,
                "--hypothesis-id",
                loser,
                "--evidence-type",
                "internal",
                "--content",
                "probe: operational cost exceeds budget",
                "--verdict",
                "fail",
            ],
        );
    }

    // The winner needs a spec_ref: model the implementation as a feature
    // spec auto-linked to the hypothesis (decision without decomposition
    // is rejected — that's the decide_rejects_winner_without_spec_ref rule).
    run_ok(
        repo.path(),
        [
            "new",
            "feat.demo.store-sqlite",
            "--tier",
            "standard",
            "--product",
            "demo",
            "--hypothesis-id",
            "sqlite",
        ],
    );

    // Decide cleanly — no --force.
    let decided = run_ok(
        repo.path(),
        [
            "decide",
            &spec_str,
            "--title",
            "Pick tracker store",
            "--winner",
            "sqlite",
            "--confirmed-by",
            "TestOwner",
            "--context",
            "tracker storage",
            "--decision",
            "SQLite file",
            "--rationale",
            "simplest fit for single-writer load",
            "--consequences",
            "no concurrent writers",
            "--challenge-summary",
            "tactical challenge run: decision survives",
            "--challenge-mode",
            "tactical",
        ],
    );
    assert!(
        !decided.stdout.contains("--force") && !decided.stderr.contains("--force"),
        "clean decide must not mention force:\n{}",
        decided.summary()
    );

    // Full chain to import — still no --force anywhere.
    run_ok(repo.path(), ["preview", &spec_str]);
    run_ok(
        repo.path(),
        ["approve", &spec_str, "--reviewer", "TestReviewer"],
    );
    let gates = run_ok(repo.path(), ["run-gates", &spec_str]);
    assert!(
        gates.stdout.contains("passed: 1"),
        "gate:review must pass on review evidence:\n{}",
        gates.summary()
    );
    run_ok(
        repo.path(),
        ["import", &spec_str, "--verified-by", "TestVerifier"],
    );

    let shown = run_ok(repo.path(), ["show", &spec_str]);
    assert!(
        shown.stdout.contains("sqlite") || shown.stdout.contains("Pick tracker store"),
        "show must render the recorded decision:\n{}",
        shown.summary()
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
    materialize_scaffold_paths(&mut spec);
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
    artifacts.systems.push(s5d::SoftwareSystem {
        id: "sys.test".into(),
        product: "shop".into(),
        name: "Test system".into(),
    });
    artifacts.containers.push(s5d::Container {
        id: "ctr.api".into(),
        name: "API".into(),
        system: "sys.test".into(),
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
    let links = spec.links.get_or_insert_with(s5d::Links::default);
    links.component_to_capability.push(binding(&[
        ("component", "comp.handler"),
        ("capability", "cap.Do"),
    ]));
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
    materialize_scaffold_paths(&mut spec);
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
    materialize_scaffold_paths(&mut spec);
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
fn reconcile_failure_does_not_rebaseline_or_stamp_ledger() {
    // Scenario #9: the historical fear was that reconcile would silently
    // accept drifted state as the new baseline. Pin the opposite: a failed
    // reconcile leaves no ledger entry, and the drift stays visible.
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    let spec_str = setup_standard_spec(&repo, "feat.rec3");

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

    // Drift the recorded state: rename an alias artifact_id (UUID intact,
    // but the fingerprint no longer matches the imported baseline).
    let s5d_dir = repo.path().join(".s5d");
    let mut aliases = s5d::AliasTable::load(&s5d_dir).unwrap();
    let entry = aliases
        .packages
        .iter_mut()
        .find(|e| e.artifact_type == "Component")
        .expect("imported spec has a component alias");
    entry.artifact_id = format!("{}-drifted", entry.artifact_id);
    aliases.save(&s5d_dir).unwrap();

    let drifted = run_fail(repo.path(), ["drift-check", &spec_str]);
    assert!(
        drifted.stdout.contains("drift") || drifted.stderr.contains("drift"),
        "drift must be visible before reconcile:\n{}",
        drifted.summary()
    );

    run_fail(repo.path(), ["reconcile", &spec_str]);

    // No re-baseline: the ledger must NOT contain a reconcile entry...
    let ledger = fs::read_to_string(s5d_dir.join("ledger.yaml")).unwrap();
    assert!(
        !ledger.contains("action: reconcile"),
        "failed reconcile must not stamp the ledger:\n{}",
        ledger
    );
    // ...and the drift must still be visible afterwards.
    let still_drifted = run_fail(repo.path(), ["drift-check", &spec_str]);
    assert!(
        still_drifted.stdout.contains("drift") || still_drifted.stderr.contains("drift"),
        "drift must survive a failed reconcile:\n{}",
        still_drifted.summary()
    );
}

#[test]
fn bulk_reconcile_reports_failure_in_exit_code() {
    // `s5d reconcile` without a spec walks all applied specs. A spec that
    // fails to reconcile must surface in the exit code — an agent running
    // the bulk form must not read "success" while drift persists.
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    let spec_str = setup_standard_spec(&repo, "feat.rec4");

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

    // Unrecoverable drift: corrupt the component alias UUID.
    let s5d_dir = repo.path().join(".s5d");
    let mut aliases = s5d::AliasTable::load(&s5d_dir).unwrap();
    let entry = aliases
        .packages
        .iter_mut()
        .find(|e| e.artifact_type == "Component")
        .expect("imported spec has a component alias");
    entry.uuid = "00000000-0000-0000-0000-000000000000".into();
    aliases.save(&s5d_dir).unwrap();

    let outcome = run_fail(repo.path(), ["reconcile"]);
    assert!(
        outcome.stdout.contains("failed") || outcome.stderr.contains("failed"),
        "bulk reconcile must report the failure:\n{}",
        outcome.summary()
    );
}

#[test]
fn owning_package_corruption_is_invisible_to_drift_check() {
    // BY DESIGN — resolved by decision.s5d.ownership-derivation (RAN-491):
    // compute_state_fingerprint hashes global aliases as type:id:uuid —
    // owning_package is deliberately outside the fingerprint, because
    // including it would invalidate every existing baseline. Instead the
    // field is a verified cache: rollback derives ownership from the
    // ledger + packages and cross-checks the stored value (see the
    // rollback_* ownership tests below). Drift-check staying "synced"
    // here is the intended behavior, not a blind spot.
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    let spec_str = setup_standard_spec(&repo, "feat.rec5");

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

    let s5d_dir = repo.path().join(".s5d");
    let mut aliases = s5d::AliasTable::load(&s5d_dir).unwrap();
    let entry = aliases
        .global
        .iter_mut()
        .find(|e| e.owning_package.is_some())
        .expect("imported spec owns at least one global alias");
    entry.owning_package = Some("feat.someone.else".into());
    aliases.save(&s5d_dir).unwrap();

    let outcome = run_ok(repo.path(), ["drift-check", &spec_str]);
    assert!(
        outcome.stdout.contains("synced"),
        "ownership is outside the fingerprint by design — rollback owns the check:\n{}",
        outcome.summary()
    );
}

// ── Rollback ownership integrity (decision.s5d.ownership-derivation) ─────────

#[test]
fn rollback_vetoes_tombstoning_when_ledger_contradicts_stored_owner() {
    // GWT: a global's owning_package is tampered to claim spec2 while the
    // ledger derivation attributes it to spec1 (first importer). Rolling
    // back spec2 must NOT tombstone the global and must warn loudly.
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    let spec1_str = setup_standard_spec(&repo, "feat.owner1");
    run_ok(repo.path(), ["preview", &spec1_str]);
    run_ok(repo.path(), ["approve", &spec1_str, "--reviewer", "R"]);
    run_ok(repo.path(), ["run-gates", &spec1_str]);
    run_ok(repo.path(), ["import", &spec1_str, "--verified-by", "V"]);

    // Second spec in the same repo, sharing the product global.
    run_ok(
        repo.path(),
        [
            "new",
            "feat.owner2",
            "--tier",
            "lightweight",
            "--product",
            "shop",
        ],
    );
    let specs_dir = repo.path().join(".s5d").join("packages");
    let spec2_path = fs::read_dir(&specs_dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.to_string_lossy().contains("feat.owner2"))
        .expect("spec2 should exist");
    let spec2_str = spec2_path.to_str().unwrap().to_string();
    {
        let mut spec: s5d::Spec = load_yaml(&spec2_path);
        materialize_scaffold_paths(&mut spec);
        fs::write(&spec2_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
    }
    run_ok(repo.path(), ["preview", &spec2_str]);
    run_ok(repo.path(), ["approve", &spec2_str, "--reviewer", "R"]);
    run_ok(repo.path(), ["run-gates", &spec2_str]);
    run_ok(repo.path(), ["import", &spec2_str, "--verified-by", "V"]);

    // Tamper: the shared Product global is owned by spec1 (first creator);
    // rewrite the stored owner to claim spec2.
    let s5d_dir = repo.path().join(".s5d");
    let mut aliases = s5d::AliasTable::load(&s5d_dir).unwrap();
    let entry = aliases
        .global
        .iter_mut()
        .find(|e| {
            e.artifact_type == "Product" && e.owning_package.as_deref() == Some("feat.owner1")
        })
        .expect("spec1 owns the Product global");
    let tampered_key = (entry.artifact_type.clone(), entry.artifact_id.clone());
    entry.owning_package = Some("feat.owner2".into());
    aliases.save(&s5d_dir).unwrap();

    let outcome = run_ok(repo.path(), ["rollback", &spec2_str]);
    assert!(
        outcome.stderr.contains("ownership mismatch"),
        "tampered owner must surface as a mismatch warning:\n{}",
        outcome.summary()
    );

    let after = s5d::AliasTable::load(&s5d_dir).unwrap();
    let alive = after
        .global
        .iter()
        .any(|e| (e.artifact_type.clone(), e.artifact_id.clone()) == tampered_key && !e.deprecated);
    assert!(
        alive,
        "global with contradicted ownership must NOT be tombstoned:\n{:?}",
        after.global
    );
}

#[test]
fn rollback_reports_suspected_tamper_without_destroying_global() {
    // GWT: the ledger derivation attributes a global to the spec being
    // rolled back, but the stored field was edited to name someone else
    // (dodge attempt). Rollback must report it loudly and take no
    // destructive action on that alias.
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    let spec_str = setup_standard_spec(&repo, "feat.dodge");

    run_ok(repo.path(), ["preview", &spec_str]);
    run_ok(repo.path(), ["approve", &spec_str, "--reviewer", "R"]);
    run_ok(repo.path(), ["run-gates", &spec_str]);
    run_ok(repo.path(), ["import", &spec_str, "--verified-by", "V"]);

    let s5d_dir = repo.path().join(".s5d");
    let mut aliases = s5d::AliasTable::load(&s5d_dir).unwrap();
    let entry = aliases
        .global
        .iter_mut()
        .find(|e| e.owning_package.as_deref() == Some("feat.dodge"))
        .expect("imported spec owns at least one global alias");
    let dodged_key = (entry.artifact_type.clone(), entry.artifact_id.clone());
    entry.owning_package = Some("feat.someone.else".into());
    aliases.save(&s5d_dir).unwrap();

    let outcome = run_ok(repo.path(), ["rollback", &spec_str]);
    assert!(
        outcome.stderr.contains("suspected ownership tamper"),
        "dodge attempt must be reported:\n{}",
        outcome.summary()
    );

    let after = s5d::AliasTable::load(&s5d_dir).unwrap();
    let alive = after
        .global
        .iter()
        .any(|e| (e.artifact_type.clone(), e.artifact_id.clone()) == dodged_key && !e.deprecated);
    assert!(
        alive,
        "derivation must never tombstone an alias the stored field does not claim:\n{:?}",
        after.global
    );
}

#[test]
fn rollback_falls_back_to_stored_owner_when_ledger_has_no_trace() {
    // GWT: legacy alias with no ledger trace (created before ledger
    // discipline or by hand). Rollback of its stored owner proceeds via
    // stored-field trust with a warning — never a hard failure.
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    let spec_str = setup_standard_spec(&repo, "feat.legacy");

    run_ok(repo.path(), ["preview", &spec_str]);
    run_ok(repo.path(), ["approve", &spec_str, "--reviewer", "R"]);
    run_ok(repo.path(), ["run-gates", &spec_str]);
    run_ok(repo.path(), ["import", &spec_str, "--verified-by", "V"]);

    // Hand-append a global alias no spec declares and no ledger entry covers.
    let s5d_dir = repo.path().join(".s5d");
    let mut aliases = s5d::AliasTable::load(&s5d_dir).unwrap();
    aliases.global.push(s5d::AliasEntry {
        uuid: "11111111-1111-1111-1111-111111111111".into(),
        artifact_id: "role.ghost".into(),
        artifact_type: "Role".into(),
        package_id: None,
        owning_package: Some("feat.legacy".into()),
        deprecated: false,
    });
    aliases.save(&s5d_dir).unwrap();

    let outcome = run_ok(repo.path(), ["rollback", &spec_str]);
    assert!(
        outcome.stderr.contains("no ledger trace"),
        "legacy fallback must be announced:\n{}",
        outcome.summary()
    );

    let after = s5d::AliasTable::load(&s5d_dir).unwrap();
    let ghost = after
        .global
        .iter()
        .find(|e| e.artifact_id == "role.ghost")
        .expect("ghost alias still present");
    assert!(
        ghost.deprecated,
        "underivable alias claimed by the stored field must tombstone via fallback"
    );
}

#[test]
fn rollback_after_reimport_does_not_false_flag_new_owner() {
    // Epoch semantics (tribunal counterexample): A imports global X, A is
    // rolled back (X tombstoned), B imports X and legitimately owns the new
    // active alias entry. Rolling back B must tombstone X cleanly — the
    // derivation must NOT resurrect A as "first ever importer" and veto.
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    let spec1_str = setup_standard_spec(&repo, "feat.epoch1");
    run_ok(repo.path(), ["preview", &spec1_str]);
    run_ok(repo.path(), ["approve", &spec1_str, "--reviewer", "R"]);
    run_ok(repo.path(), ["run-gates", &spec1_str]);
    run_ok(repo.path(), ["import", &spec1_str, "--verified-by", "V"]);
    run_ok(repo.path(), ["rollback", &spec1_str]);

    // Clean up the rolled-back spec entirely (normal user hygiene). Its ledger
    // import entry now has no matching file — the replay must neutralize that
    // taint via the rollback entry instead of poisoning the key forever.
    fs::remove_file(&spec1_str).unwrap();

    run_ok(
        repo.path(),
        [
            "new",
            "feat.epoch2",
            "--tier",
            "lightweight",
            "--product",
            "shop",
        ],
    );
    let spec2_path = only_spec_path(&repo);
    let spec2_str = spec2_path.to_str().unwrap().to_string();
    {
        let mut spec: s5d::Spec = load_yaml(&spec2_path);
        materialize_scaffold_paths(&mut spec);
        fs::write(&spec2_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
    }
    run_ok(repo.path(), ["preview", &spec2_str]);
    run_ok(repo.path(), ["approve", &spec2_str, "--reviewer", "R"]);
    run_ok(repo.path(), ["run-gates", &spec2_str]);
    run_ok(repo.path(), ["import", &spec2_str, "--verified-by", "V"]);

    let outcome = run_ok(repo.path(), ["rollback", &spec2_str]);
    assert!(
        !outcome.stderr.contains("ownership mismatch")
            && !outcome.stderr.contains("ownership unverifiable"),
        "untampered epoch transition must not be flagged:\n{}",
        outcome.summary()
    );

    let s5d_dir = repo.path().join(".s5d");
    let aliases = s5d::AliasTable::load(&s5d_dir).unwrap();
    let shop_active = aliases
        .global
        .iter()
        .any(|e| e.artifact_type == "Product" && e.artifact_id == "shop" && !e.deprecated);
    assert!(
        !shop_active,
        "epoch-2 owner rollback must tombstone the global it legitimately owns:\n{:?}",
        aliases.global
    );
}

#[test]
fn rollback_after_edit_and_reimport_tombstones_own_globals() {
    // Normal evolution lifecycle (tribunal round-2 blocker 1): a spec is
    // imported, edited, re-approved, re-imported. The FIRST ledger entry's
    // sha no longer matches the current file — that self-stale entry must
    // not poison the spec's own ownership: whichever of its imports was
    // first, the owner is the same package. Rollback must tombstone cleanly.
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    let spec_str = setup_standard_spec(&repo, "feat.evolve");

    run_ok(repo.path(), ["preview", &spec_str]);
    run_ok(repo.path(), ["approve", &spec_str, "--reviewer", "R"]);
    run_ok(repo.path(), ["run-gates", &spec_str]);
    run_ok(repo.path(), ["import", &spec_str, "--verified-by", "V"]);

    // Edit (version bump) and run the pipeline again — second import entry.
    {
        let spec_path = PathBuf::from(&spec_str);
        let mut spec: s5d::Spec = load_yaml(&spec_path);
        spec.version = "1.0.1".into();
        fs::write(&spec_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
    }
    run_ok(repo.path(), ["preview", &spec_str]);
    run_ok(repo.path(), ["approve", &spec_str, "--reviewer", "R"]);
    run_ok(repo.path(), ["run-gates", &spec_str]);
    run_ok(repo.path(), ["import", &spec_str, "--verified-by", "V"]);

    let outcome = run_ok(repo.path(), ["rollback", &spec_str]);
    assert!(
        !outcome.stderr.contains("ownership unverifiable")
            && !outcome.stderr.contains("ownership mismatch"),
        "self-stale ledger entries must not block the owner's own rollback:\n{}",
        outcome.summary()
    );

    let s5d_dir = repo.path().join(".s5d");
    let aliases = s5d::AliasTable::load(&s5d_dir).unwrap();
    let shop_active = aliases
        .global
        .iter()
        .any(|e| e.artifact_type == "Product" && e.artifact_id == "shop" && !e.deprecated);
    assert!(
        !shop_active,
        "evolved spec's rollback must still tombstone its own globals:\n{:?}",
        aliases.global
    );
}

#[test]
fn rollback_epoch_transition_works_with_old_spec_file_left_on_disk() {
    // Tribunal round-2 blocker 2: same epoch transition as the test above,
    // but the rolled-back spec's package file is NOT deleted. Its record
    // says Deprecated, so it must neither pin the global via the
    // referenced-globals guard nor distort the derivation.
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    let spec1_str = setup_standard_spec(&repo, "feat.keep1");
    run_ok(repo.path(), ["preview", &spec1_str]);
    run_ok(repo.path(), ["approve", &spec1_str, "--reviewer", "R"]);
    run_ok(repo.path(), ["run-gates", &spec1_str]);
    run_ok(repo.path(), ["import", &spec1_str, "--verified-by", "V"]);
    run_ok(repo.path(), ["rollback", &spec1_str]);
    // spec1's file stays on disk — only its record marks it Deprecated.

    run_ok(
        repo.path(),
        [
            "new",
            "feat.keep2",
            "--tier",
            "lightweight",
            "--product",
            "shop",
        ],
    );
    let specs_dir = repo.path().join(".s5d").join("packages");
    let spec2_path = fs::read_dir(&specs_dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.to_string_lossy().contains("feat.keep2"))
        .expect("spec2 should exist");
    let spec2_str = spec2_path.to_str().unwrap().to_string();
    {
        let mut spec: s5d::Spec = load_yaml(&spec2_path);
        materialize_scaffold_paths(&mut spec);
        fs::write(&spec2_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
    }
    run_ok(repo.path(), ["preview", &spec2_str]);
    run_ok(repo.path(), ["approve", &spec2_str, "--reviewer", "R"]);
    run_ok(repo.path(), ["run-gates", &spec2_str]);
    run_ok(repo.path(), ["import", &spec2_str, "--verified-by", "V"]);

    let outcome = run_ok(repo.path(), ["rollback", &spec2_str]);
    assert!(
        !outcome.stderr.contains("ownership mismatch")
            && !outcome.stderr.contains("ownership unverifiable"),
        "epoch transition with the old file kept must not be flagged:\n{}",
        outcome.summary()
    );

    let s5d_dir = repo.path().join(".s5d");
    let aliases = s5d::AliasTable::load(&s5d_dir).unwrap();
    let shop_active = aliases
        .global
        .iter()
        .any(|e| e.artifact_type == "Product" && e.artifact_id == "shop" && !e.deprecated);
    assert!(
        !shop_active,
        "rolled-back spec1 must not pin the global alive via referenced-globals:\n{:?}",
        aliases.global
    );
}

#[test]
fn rollback_skips_tombstoning_when_ownership_unverifiable() {
    // Sha anchoring (tribunal counterexample): a package file edited after
    // import makes its historical claims unknowable. Keys that were unowned
    // at that point are poisoned — rollback must neither veto-with-a-named-
    // owner nor fall back to a destructive tombstone; it keeps the alias and
    // reports the evidence gap.
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    let spec1_str = setup_standard_spec(&repo, "feat.taint1");
    run_ok(repo.path(), ["preview", &spec1_str]);
    run_ok(repo.path(), ["approve", &spec1_str, "--reviewer", "R"]);
    run_ok(repo.path(), ["run-gates", &spec1_str]);
    run_ok(repo.path(), ["import", &spec1_str, "--verified-by", "V"]);

    // Backdate-edit: change spec1's file AFTER its import (sha now mismatches
    // the ledger entry, so its import claims become unknowable).
    {
        let mut content = fs::read_to_string(&spec1_str).unwrap();
        content.push_str("\n# edited after import\n");
        fs::write(&spec1_str, content).unwrap();
    }

    // Second spec with its own product global, imported after the tainted one.
    run_ok(
        repo.path(),
        [
            "new",
            "feat.taint2",
            "--tier",
            "lightweight",
            "--product",
            "mart",
        ],
    );
    let specs_dir = repo.path().join(".s5d").join("packages");
    let spec2_path = fs::read_dir(&specs_dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.to_string_lossy().contains("feat.taint2"))
        .expect("spec2 should exist");
    let spec2_str = spec2_path.to_str().unwrap().to_string();
    {
        let mut spec: s5d::Spec = load_yaml(&spec2_path);
        materialize_scaffold_paths(&mut spec);
        fs::write(&spec2_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
    }
    run_ok(repo.path(), ["preview", &spec2_str]);
    run_ok(repo.path(), ["approve", &spec2_str, "--reviewer", "R"]);
    run_ok(repo.path(), ["run-gates", &spec2_str]);
    run_ok(repo.path(), ["import", &spec2_str, "--verified-by", "V"]);

    let outcome = run_ok(repo.path(), ["rollback", &spec2_str]);
    assert!(
        outcome.stderr.contains("ownership unverifiable"),
        "evidence gap must be reported:\n{}",
        outcome.summary()
    );

    let s5d_dir = repo.path().join(".s5d");
    let aliases = s5d::AliasTable::load(&s5d_dir).unwrap();
    let mart_alive = aliases
        .global
        .iter()
        .any(|e| e.artifact_type == "Product" && e.artifact_id == "mart" && !e.deprecated);
    assert!(
        mart_alive,
        "inconclusive ownership evidence must never tombstone:\n{:?}",
        aliases.global
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
        materialize_scaffold_paths(&mut spec);
        let arts = spec.artifacts.as_mut().unwrap();
        arts.capabilities.push(s5d::Capability {
            id: "cap.A".into(),
            domain: "dom.shop.core".into(),
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
        materialize_scaffold_paths(&mut spec);
        let arts = spec.artifacts.as_mut().unwrap();
        arts.capabilities.push(s5d::Capability {
            id: "cap.B".into(),
            domain: "dom.shop.core".into(),
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
        materialize_scaffold_paths(&mut spec);
        let arts = spec.artifacts.as_mut().unwrap();
        arts.capabilities.push(s5d::Capability {
            id: "cap.A".into(),
            domain: "dom.shared.core".into(),
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
        materialize_scaffold_paths(&mut spec);
        let arts = spec.artifacts.as_mut().unwrap();
        arts.capabilities.push(s5d::Capability {
            id: "cap.B".into(),
            domain: "dom.shared.core".into(),
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

#[test]
fn import_rejects_scaffold_placeholder_paths() {
    let repo = StandaloneRepo::new();
    seed_searchable_rust_repo(&repo);
    run_ok(repo.path(), ["init"]);
    run_ok(
        repo.path(),
        [
            "new",
            "feat.shop.todo",
            "--tier",
            "standard",
            "--product",
            "shop",
        ],
    );
    let spec_path = only_spec_path(&repo);
    let spec_str = spec_path.to_str().unwrap();
    run_ok(repo.path(), ["preview", spec_str]);
    run_ok(repo.path(), ["approve", spec_str, "--reviewer", "R"]);
    run_ok(repo.path(), ["run-gates", spec_str]);
    let denied = run_fail(repo.path(), ["import", spec_str, "--verified-by", "V"]);
    assert!(
        denied.stderr.contains("scaffold placeholder paths"),
        "import must refuse the TODO scaffold path: {}",
        denied.summary()
    );
}

// ── CI enforcement generation (feat.s5d.ci-init) ──────────────────────────────

#[test]
fn ci_init_generates_marked_templates_idempotently() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    run_ok(repo.path(), ["ci", "init", "--all"]);
    let gh = repo.path().join(".github/workflows/s5d.yml");
    let gl_fragment = repo.path().join(".s5d/ci/s5d.gitlab-ci.yml");
    let gl_root = repo.path().join(".gitlab-ci.yml");
    let version = env!("CARGO_PKG_VERSION");
    for f in [&gh, &gl_fragment, &gl_root] {
        let content = fs::read_to_string(f).unwrap();
        assert!(
            content.starts_with("# s5d-ci-template: v"),
            "{} must start with the template marker:\n{}",
            f.display(),
            content
        );
        assert!(
            content.contains(version),
            "{} must pin the generating version {version}",
            f.display()
        );
    }
    let gh_content = fs::read_to_string(&gh).unwrap();
    assert!(
        gh_content.contains("s5d ci exec") && gh_content.contains("permissions:"),
        "workflow must call ci exec under explicit permissions:\n{gh_content}"
    );

    // Idempotent: second run regenerates identical managed content.
    run_ok(repo.path(), ["ci", "init", "--all"]);
    assert_eq!(fs::read_to_string(&gh).unwrap(), gh_content);

    // User-owned (marker stripped) → refuse without --force.
    let stripped = gh_content.lines().skip(1).collect::<Vec<_>>().join("\n");
    fs::write(&gh, &stripped).unwrap();
    let denied = run_fail(repo.path(), ["ci", "init", "--github"]);
    assert!(
        denied.stderr.contains("user-owned"),
        "must refuse to overwrite marker-less file:\n{}",
        denied.summary()
    );
    run_ok(repo.path(), ["ci", "init", "--github", "--force"]);
    assert!(fs::read_to_string(&gh)
        .unwrap()
        .starts_with("# s5d-ci-template: v"));
}

#[test]
fn ci_exec_passes_clean_repo_and_blocks_drift() {
    let repo = StandaloneRepo::new();
    seed_searchable_rust_repo(&repo);
    run_ok(repo.path(), ["init"]);
    let spec_str = setup_standard_spec(&repo, "feat.ciexec");

    run_ok(repo.path(), ["preview", &spec_str]);
    run_ok(repo.path(), ["approve", &spec_str, "--reviewer", "R"]);
    run_ok(repo.path(), ["run-gates", &spec_str]);
    run_ok(repo.path(), ["import", &spec_str, "--verified-by", "V"]);

    let clean = run_ok(repo.path(), ["ci", "exec"]);
    assert!(
        clean.stdout.contains("all checks passed"),
        "clean repo must pass ci exec:\n{}",
        clean.summary()
    );

    // Drift the recorded state — ci exec must turn red.
    let s5d_dir = repo.path().join(".s5d");
    let mut aliases = s5d::AliasTable::load(&s5d_dir).unwrap();
    let entry = aliases
        .packages
        .iter_mut()
        .find(|e| e.artifact_type == "Component")
        .expect("imported spec has a component alias");
    entry.uuid = "00000000-0000-0000-0000-000000000000".into();
    aliases.save(&s5d_dir).unwrap();

    let drifted = run_fail(repo.path(), ["ci", "exec"]);
    assert!(
        drifted.stderr.contains("[drift]"),
        "drift must fail ci exec:\n{}",
        drifted.summary()
    );
}

#[test]
fn ci_exec_blocks_invalid_spec() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    run_ok(
        repo.path(),
        [
            "new",
            "feat.broken.one",
            "--tier",
            "standard",
            "--product",
            "demo",
        ],
    );
    let spec_path = only_spec_path(&repo);
    {
        let mut spec: s5d::Spec = load_yaml(&spec_path);
        materialize_scaffold_paths(&mut spec);
        // Break the single-feature invariant
        spec.artifacts.as_mut().unwrap().features[0].id = "feat.other".into();
        fs::write(&spec_path, serde_yaml::to_string(&spec).unwrap()).unwrap();
    }
    let outcome = run_fail(repo.path(), ["ci", "exec"]);
    assert!(
        outcome.stderr.contains("[validate]"),
        "invalid spec must fail ci exec:\n{}",
        outcome.summary()
    );
}

#[test]
fn ci_check_reports_stale_template() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    run_ok(repo.path(), ["ci", "init", "--github"]);
    let gh = repo.path().join(".github/workflows/s5d.yml");
    let content = fs::read_to_string(&gh).unwrap();
    let mut lines: Vec<&str> = content.lines().collect();
    let downgraded = "# s5d-ci-template: v0 (generated by s5d 0.0.0; verify with s5d ci check)";
    lines[0] = downgraded;
    fs::write(&gh, lines.join("\n")).unwrap();

    let outcome = run_fail(repo.path(), ["ci", "check"]);
    assert!(
        outcome.stdout.contains("stale") || outcome.stderr.contains("stale"),
        "downgraded marker must report stale:\n{}",
        outcome.summary()
    );
}

#[test]
fn ci_init_rewires_managed_stub_and_check_flags_unwired_gitlab() {
    // Managed root stub regenerates on re-init (tribunal: a stale managed
    // stub was un-fixable); a user-owned .gitlab-ci.yml without the include
    // is a silent enforcement gap that ci check must fail on.
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    // Managed stub: downgrade its marker, re-init must restore it.
    run_ok(repo.path(), ["ci", "init", "--gitlab"]);
    let root_ci = repo.path().join(".gitlab-ci.yml");
    let stub = fs::read_to_string(&root_ci).unwrap();
    let downgraded = stub.replace("# s5d-ci-template: v", "# s5d-ci-template: v0-was-");
    fs::write(
        &root_ci,
        stub.replacen(
            &stub.lines().next().unwrap().to_string(),
            "# s5d-ci-template: v0 (old)",
            1,
        ),
    )
    .unwrap();
    let _ = downgraded;
    run_ok(repo.path(), ["ci", "init", "--gitlab"]);
    assert!(
        fs::read_to_string(&root_ci)
            .unwrap()
            .starts_with(&format!("# s5d-ci-template: v{}", 1)),
        "managed stub must be regenerated to the current template version"
    );

    // User-owned root pipeline without the include → fragment generated but
    // never wired: ci check must fail loudly.
    fs::write(&root_ci, "stages: [test]\n").unwrap();
    let unwired = run_fail(repo.path(), ["ci", "check"]);
    assert!(
        unwired.stdout.contains("does not include") || unwired.stderr.contains("does not include"),
        "unwired GitLab fragment must fail ci check:\n{}",
        unwired.summary()
    );

    // User-owned root stays untouched on re-init; the include note is the
    // only output (root stub is skip-and-note, not bail — bail is the
    // contract for the fragment/workflow files).
    let user_owned = "## docs mention s5d-ci-template: here\ninclude: []\n";
    fs::write(&root_ci, user_owned).unwrap();
    let noted = run_ok(repo.path(), ["ci", "init", "--gitlab"]);
    assert_eq!(
        fs::read_to_string(&root_ci).unwrap(),
        user_owned,
        "user-owned root .gitlab-ci.yml must never be modified"
    );
    assert!(
        noted.stdout.contains("add this include"),
        "init must print the include instruction for user-owned root:\n{}",
        noted.summary()
    );

    // Tightened ownership on managed files: a first line merely MENTIONING
    // the marker text mid-line is user-owned — workflow init must refuse.
    let gh = repo.path().join(".github/workflows/s5d.yml");
    fs::create_dir_all(gh.parent().unwrap()).unwrap();
    fs::write(&gh, "## docs mention s5d-ci-template: here\nname: mine\n").unwrap();
    let denied = run_fail(repo.path(), ["ci", "init", "--github"]);
    assert!(
        denied.stderr.contains("user-owned"),
        "mid-line marker mention must not count as managed:\n{}",
        denied.summary()
    );
}

// ── Shape / Review / Plan (decision.s5d.bmad-native-runtime) ─────────────────

#[test]
fn shape_routes_kernel_and_reports_readiness() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    // Incomplete kernel: missing success_signal → reported, exit stays 0
    // (shape is a report; only --emit-spec hard-fails on readiness).
    repo.write(
        "kernel-partial.yaml",
        "why: checkout abandons spike on slow payment confirmation\ncapabilities:\n  - payment processing\n",
    );
    let outcome = run_ok(repo.path(), ["shape", "kernel-partial.yaml"]);
    assert!(
        outcome.stdout.contains("Route:"),
        "shape must print the routing block:\n{}",
        outcome.summary()
    );
    assert!(
        outcome.stderr.contains("intent_kernel.success_signal"),
        "missing readiness field must be named:\n{}",
        outcome.summary()
    );

    // Complete kernel → ready.
    repo.write(
        "kernel-full.yaml",
        "why: checkout abandons spike on slow payment confirmation\nsuccess_signal: p95 payment confirmation under 2s\ncapabilities:\n  - payment processing\nconstraints:\n  - PCI scope must not grow\n",
    );
    let outcome = run_ok(repo.path(), ["shape", "kernel-full.yaml"]);
    assert!(
        outcome.stdout.contains("Readiness: ready"),
        "complete kernel must be ready:\n{}",
        outcome.summary()
    );
}

#[test]
fn shape_emit_spec_embeds_kernel_and_validates() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);

    repo.write(
        "kernel.yaml",
        "why: orders need partial refunds\nsuccess_signal: refunds complete without support tickets\nassumptions:\n  - refund provider API supports partial amounts\n",
    );
    let outcome = run_ok(
        repo.path(),
        [
            "shape",
            "kernel.yaml",
            "--emit-spec",
            "feat.refunds",
            "--product",
            "shop",
        ],
    );
    assert!(
        outcome.stdout.contains("Embedded intent_kernel"),
        "emit must embed the kernel:\n{}",
        outcome.summary()
    );

    let spec_path = only_spec_path(&repo);
    let spec: s5d::Spec = load_yaml(&spec_path);
    let kernel = spec.intent_kernel.as_ref().expect("kernel embedded");
    assert_eq!(kernel.why, "orders need partial refunds");
    assert_eq!(kernel.assumptions.len(), 1);

    // Emitted spec validates; corrupting the kernel is caught by validate.
    let spec_str = spec_path.to_str().unwrap().to_string();
    run_ok(repo.path(), ["validate", &spec_str]);
    {
        let mut broken = spec.clone();
        broken.intent_kernel.as_mut().unwrap().why = "".into();
        fs::write(&spec_path, serde_yaml::to_string(&broken).unwrap()).unwrap();
    }
    let failed = run_fail(repo.path(), ["validate", &spec_str]);
    assert!(
        failed.stdout.contains("intent_kernel.why") || failed.stderr.contains("intent_kernel.why"),
        "validate must name the empty kernel field:\n{}",
        failed.summary()
    );
}

#[test]
fn review_adversarial_scaffolds_layered_report() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    let spec_str = setup_standard_spec(&repo, "feat.reviewme");

    let outcome = run_ok(repo.path(), ["review", "adversarial", &spec_str]);
    assert!(
        outcome.stdout.contains("Binding:"),
        "binding instruction must be printed:\n{}",
        outcome.summary()
    );
    let report = repo
        .path()
        .join(".s5d")
        .join("evidence")
        .join("feat.reviewme")
        .join("adversarial-review-1.md");
    let content = fs::read_to_string(&report).expect("report scaffold written");
    for layer in [
        "Blind Diff Hunter",
        "Edge Case Hunter",
        "Acceptance Auditor",
    ] {
        assert!(content.contains(layer), "missing layer {layer}");
    }

    // Second run numbers the next report instead of overwriting.
    run_ok(repo.path(), ["review", "adversarial", &spec_str]);
    assert!(repo
        .path()
        .join(".s5d")
        .join("evidence")
        .join("feat.reviewme")
        .join("adversarial-review-2.md")
        .exists());
}

#[test]
fn plan_stories_appends_phases_and_rejects_collisions() {
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    let spec_str = setup_standard_spec(&repo, "feat.storied");

    repo.write(
        "stories.yaml",
        "- id: story-cart\n  title: Cart line items\n  scope: Render and mutate cart lines\n  acceptance:\n    - line items render with quantity controls\n- id: story-checkout\n  title: Checkout handoff\n  scope: Hand the cart to the payment flow\n  acceptance:\n    - cart total matches payment intent amount\n",
    );
    let outcome = run_ok(
        repo.path(),
        ["plan", "stories", &spec_str, "--from", "stories.yaml"],
    );
    assert!(
        outcome.stdout.contains("story-cart") && outcome.stdout.contains("story-checkout"),
        "added story ids must be reported:\n{}",
        outcome.summary()
    );

    let spec: s5d::Spec = load_yaml(std::path::Path::new(&spec_str));
    let phases = &spec.workflow.as_ref().unwrap().phases;
    assert!(phases.iter().any(|p| p.id == "story-cart"));
    assert!(phases.iter().any(|p| p.id == "story-checkout"));

    // Same file again → id collision must fail loudly.
    let failed = run_fail(
        repo.path(),
        ["plan", "stories", &spec_str, "--from", "stories.yaml"],
    );
    assert!(
        failed.stderr.contains("collides"),
        "duplicate story ids must be rejected:\n{}",
        failed.summary()
    );

    // Omitted rollback is defaulted (validator requires non-empty), and a
    // story without acceptance is rejected with the real reason.
    let checkout = spec
        .workflow
        .as_ref()
        .unwrap()
        .phases
        .iter()
        .find(|p| p.id == "story-checkout")
        .unwrap();
    assert!(!checkout.rollback.is_empty(), "rollback must be defaulted");

    repo.write(
        "stories-bad.yaml",
        "- id: story-empty\n  title: No acceptance\n  scope: Should be rejected\n",
    );
    let failed = run_fail(
        repo.path(),
        ["plan", "stories", &spec_str, "--from", "stories-bad.yaml"],
    );
    assert!(
        failed.stderr.contains("acceptance criterion"),
        "story without acceptance must be rejected:\n{}",
        failed.summary()
    );
}

#[test]
fn review_adversarial_rejects_path_traversal_spec_id() {
    // spec.id comes from file content — a crafted id must not become a path
    // segment that escapes .s5d/evidence/ (tribunal round-1 blocker).
    let repo = StandaloneRepo::new();
    run_ok(repo.path(), ["init"]);
    repo.write(
        ".s5d/packages/evil__20260612.s5d.yaml",
        "s5d: '1.0'\nid: ../../escape\nversion: 1.0.0\nproduct: shop\ntier: standard\n",
    );
    let failed = run_fail(
        repo.path(),
        [
            "review",
            "adversarial",
            ".s5d/packages/evil__20260612.s5d.yaml",
        ],
    );
    assert!(
        failed.stderr.contains("invalid spec ID"),
        "traversal id must be rejected before any write:\n{}",
        failed.summary()
    );
    assert!(
        !repo.path().join("escape").exists()
            && !repo.path().parent().unwrap().join("escape").exists(),
        "no directory may be created outside .s5d/evidence"
    );
}
