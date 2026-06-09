//! DDD modeling analyzer: signals (detect) and smell findings (analyze).
//!
//! Covered stacks: TypeScript, JavaScript.
//! When neither is detected, `analyze` returns `status: StackNotCovered`.
//!
//! Site-specific artifacts removed from the original bash:
//! - `gigg-insurance-master` hard-coded path (dropped entirely)
//! - `tmhcc|stripe|box` seam whitelist → generic vendor-SDK seam check
//! - insurance vocabulary in detail/fix strings → neutral wording

use anyhow::Result;
use regex::Regex;
use std::path::Path;

use super::{
    AnalysisReport, CoverageStatus, DetectReport, DetectSignal, DetectSummary, Finding, Severity,
    Summary,
};
use crate::suite::scan::{RepoScan, Stack};

/// Stacks whose checks are meaningful for DDD analysis.
const COVERED: &[Stack] = &[Stack::TypeScript, Stack::JavaScript];

/// Emit detect signals for the repo at `root`.
pub fn detect(root: &Path) -> Result<DetectReport> {
    let scan = RepoScan::build(root)?;
    let root_str = root.to_string_lossy().to_string();
    let scanned = scan.files.len();
    let stacks: Vec<String> = scan.stacks.iter().map(|s| s.as_str().to_string()).collect();

    let signals = vec![
        detect_orm_entities(&scan),
        detect_domain_layer(&scan),
        detect_controllers(&scan),
        detect_integ_seams(&scan),
        detect_value_types(&scan),
        detect_events(&scan),
    ];

    let present = signals.iter().filter(|s| s.present).count();
    let total = signals.len();

    Ok(DetectReport {
        root: root_str,
        scanned_files: scanned,
        stacks,
        truncated: scan.truncated,
        signals,
        summary: DetectSummary {
            present,
            signals_total: total,
        },
    })
}

/// Run all DDD analyze checks on the repo at `root`.
pub fn analyze(root: &Path) -> Result<AnalysisReport> {
    let scan = RepoScan::build(root)?;
    let root_str = root.to_string_lossy().to_string();
    let stacks: Vec<String> = scan.stacks.iter().map(|s| s.as_str().to_string()).collect();

    // Stack coverage gate
    let covered = scan.stacks.iter().any(|s| COVERED.contains(s));
    if !covered {
        return Ok(AnalysisReport {
            root: root_str,
            scanned_files: 0,
            stacks,
            truncated: scan.truncated,
            status: CoverageStatus::StackNotCovered,
            findings: vec![],
            summary: Summary {
                high: 0,
                medium: 0,
                low: 0,
                total: 0,
            },
        });
    }

    let ts_js_files: Vec<&Path> = scan.files_with_ext(&["ts", "tsx", "js", "jsx"]);
    let scanned_files = ts_js_files.len();

    // A covered stack with zero matching files (e.g. package.json present for
    // tooling in a Rust repo) still means nothing was inspected — report it as
    // not covered, never as a clean scan.
    if scanned_files == 0 {
        return Ok(AnalysisReport {
            root: root_str,
            scanned_files: 0,
            stacks,
            truncated: scan.truncated,
            status: CoverageStatus::StackNotCovered,
            findings: vec![],
            summary: Summary {
                high: 0,
                medium: 0,
                low: 0,
                total: 0,
            },
        });
    }

    let mut findings = Vec::new();

    // 1 — anemic-domain
    if let Some(f) = check_anemic_domain(&scan) {
        findings.push(f);
    }

    // 2 — transaction-script
    if let Some(f) = check_transaction_script(&scan) {
        findings.push(f);
    }

    // 3 — domain-logic-in-controllers
    if let Some(f) = check_domain_logic_in_controllers(&scan) {
        findings.push(f);
    }

    // 4 — value-objects-as-primitives
    let has_vo = has_value_object_type(&scan);
    if let Some(f) = check_value_objects_as_primitives(&scan, has_vo) {
        findings.push(f);
    }

    // 5 — missing-acl (one finding per vendor with >5 scattered files)
    findings.extend(check_missing_acl(&scan));

    // 6 — repository-absence
    if let Some(f) = check_repository_absence(&scan) {
        findings.push(f);
    }

    // 7 — aggregate-boundary-leak
    if let Some(f) = check_aggregate_boundary_leak(&scan) {
        findings.push(f);
    }

    let summary = Summary::from_findings(&findings);

    Ok(AnalysisReport {
        root: root_str,
        scanned_files,
        stacks,
        truncated: scan.truncated,
        status: CoverageStatus::Scanned,
        findings,
        summary,
    })
}

// ── detect helpers ────────────────────────────────────────────────────────────

fn detect_orm_entities(scan: &RepoScan) -> DetectSignal {
    let mut evidence_parts: Vec<String> = Vec::new();

    let prisma_path = scan.root.join("prisma/schema.prisma");
    if prisma_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&prisma_path) {
            let n = content
                .lines()
                .filter(|l| l.trim_start().starts_with("model "))
                .count();
            evidence_parts.push(format!("prisma:{} models", n));
        }
    }

    let re_orm =
        Regex::new(r"(?i)typeorm|sequelize|sqlalchemy|gorm\.io|jinzhu/gorm|diesel|sea-orm")
            .unwrap();
    if scan.dep_seen(&re_orm) {
        evidence_parts.push("dep-orm".to_string());
    }

    let present = !evidence_parts.is_empty();
    DetectSignal {
        id: "orm-entities".to_string(),
        present,
        evidence: evidence_parts.join(","),
    }
}

fn detect_domain_layer(scan: &RepoScan) -> DetectSignal {
    let dirs = [
        "domain",
        "src/domain",
        "app/domain",
        "lib/domain",
        "application",
        "src/application",
        "infrastructure",
        "src/infrastructure",
        "core/domain",
    ];
    let mut hits: Vec<&str> = Vec::new();
    for d in &dirs {
        if scan.root.join(d).is_dir() {
            hits.push(d);
        }
    }
    let present = !hits.is_empty();
    let evidence = if present {
        hits.join(",")
    } else {
        "flat (logic likely in controllers/services)".to_string()
    };
    DetectSignal {
        id: "domain-layer".to_string(),
        present,
        evidence,
    }
}

fn detect_controllers(scan: &RepoScan) -> DetectSignal {
    let api_route_count = scan
        .files
        .iter()
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            (name == "route.ts" || name == "route.js") && p.to_string_lossy().contains("/api/")
        })
        .count();

    let re_go = Regex::new(r"http\.(HandleFunc|Handle)|chi\.|gin\.").unwrap();
    let go_files: Vec<&std::path::Path> = scan.files_with_ext(&["go"]);
    let go_count = go_files
        .iter()
        .filter(|p| {
            let full = scan.root.join(p);
            std::fs::read_to_string(&full)
                .map(|c| re_go.is_match(&c))
                .unwrap_or(false)
        })
        .count();

    let re_py = Regex::new(r"@(app|router)\.(get|post|put|delete)").unwrap();
    let py_files: Vec<&std::path::Path> = scan.files_with_ext(&["py"]);
    let py_count = py_files
        .iter()
        .filter(|p| {
            let full = scan.root.join(p);
            std::fs::read_to_string(&full)
                .map(|c| re_py.is_match(&c))
                .unwrap_or(false)
        })
        .count();

    let total = api_route_count + go_count + py_count;
    let present = total > 0;
    DetectSignal {
        id: "controllers".to_string(),
        present,
        evidence: if present {
            format!("{} handlers", total)
        } else {
            String::new()
        },
    }
}

fn detect_integ_seams(scan: &RepoScan) -> DetectSignal {
    // Generic vendor-SDK seam detection: common cloud/payment/comms SDKs
    let vendors = [
        ("stripe", r"(?i)stripe"),
        ("aws-sdk", r"(?i)aws-sdk|@aws-sdk"),
        ("twilio", r"(?i)twilio"),
        ("sendgrid", r"(?i)sendgrid"),
        ("box", r"(?i)box-node-sdk"),
    ];
    let mut found: Vec<&str> = Vec::new();
    for (name, pattern) in &vendors {
        let re = Regex::new(pattern).unwrap();
        if scan.dep_seen(&re) {
            found.push(name);
        }
    }
    let present = !found.is_empty();
    DetectSignal {
        id: "integ-seams".to_string(),
        present,
        evidence: found.join(","),
    }
}

fn detect_value_types(scan: &RepoScan) -> DetectSignal {
    let re = Regex::new(r"class Money|type Money|Money =|class Email|ValueObject").unwrap();
    let ts_go: Vec<&Path> = scan.files_with_ext(&["ts", "tsx", "go"]);
    let found = ts_go.iter().any(|p| {
        let full = scan.root.join(p);
        std::fs::read_to_string(&full)
            .map(|c| re.is_match(&c))
            .unwrap_or(false)
    });
    DetectSignal {
        id: "value-types".to_string(),
        present: found,
        evidence: if found {
            "found".to_string()
        } else {
            "domain concepts likely primitives (string/number)".to_string()
        },
    }
}

fn detect_events(scan: &RepoScan) -> DetectSignal {
    let re_dep = Regex::new(r"(?i)eventemitter|mitt|event-bus").unwrap();
    let re_code = Regex::new(r"DomainEvent|publishEvent|emitEvent").unwrap();

    let has_dep = scan.dep_seen(&re_dep);
    let ts_go: Vec<&Path> = scan.files_with_ext(&["ts", "tsx", "go"]);
    let has_code = ts_go.iter().any(|p| {
        let full = scan.root.join(p);
        std::fs::read_to_string(&full)
            .map(|c| re_code.is_match(&c))
            .unwrap_or(false)
    });

    let present = has_dep || has_code;
    let mut parts = Vec::new();
    if has_dep {
        parts.push("event-lib");
    }
    if has_code {
        parts.push("domain-events");
    }
    DetectSignal {
        id: "events".to_string(),
        present,
        evidence: if present {
            parts.join(",")
        } else {
            "no explicit domain events".to_string()
        },
    }
}

// ── analyze helpers ───────────────────────────────────────────────────────────

fn has_domain_layer(scan: &RepoScan) -> bool {
    ["domain", "src/domain", "app/domain", "lib/domain"]
        .iter()
        .any(|d| scan.root.join(d).is_dir())
}

fn prisma_model_count(scan: &RepoScan) -> usize {
    let p = scan.root.join("prisma/schema.prisma");
    std::fs::read_to_string(&p)
        .map(|c| {
            c.lines()
                .filter(|l| l.trim_start().starts_with("model "))
                .count()
        })
        .unwrap_or(0)
}

fn check_anemic_domain(scan: &RepoScan) -> Option<Finding> {
    let models = prisma_model_count(scan);
    if models == 0 || has_domain_layer(scan) {
        return None;
    }
    Some(Finding {
        check: "anemic-domain".to_string(),
        severity: Severity::High,
        path: "prisma/schema.prisma".to_string(),
        detail: format!(
            "{} Prisma models are data-only records with no behavior; there is no domain/ layer — \
business rules likely live in controllers/utils (anemic domain model)",
            models
        ),
        fix: "Introduce a domain layer: wrap key aggregates in rich types that own their \
invariants (state transitions, business rules), keep Prisma as persistence only"
            .to_string(),
        validate: "grep for core business logic in app/api — after the move it returns 0 hits in \
route handlers; invariants are unit-tested on the domain type, not the route."
            .to_string(),
        dimension: None,
    })
}

fn check_transaction_script(scan: &RepoScan) -> Option<Finding> {
    let re_handler = Regex::new(r"export (async )?function (GET|POST|PUT|PATCH|DELETE)").unwrap();

    let api_files: Vec<&Path> = scan
        .files
        .iter()
        .filter(|p| p.to_string_lossy().contains("/api/"))
        .map(|p| p.as_path())
        .collect();

    let mut big: Vec<String> = Vec::new();
    for rel in &api_files {
        let full = scan.root.join(rel);
        let content = match std::fs::read_to_string(&full) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if !re_handler.is_match(&content) {
            continue;
        }
        let lc = content.lines().count();
        if lc > 300 {
            big.push(format!("{}:{}", rel.display(), lc));
        }
    }

    if big.is_empty() {
        return None;
    }

    let sample = big
        .iter()
        .take(4)
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Some(Finding {
        check: "transaction-script".to_string(),
        severity: Severity::High,
        path: sample,
        detail: format!(
            "{} route handler(s) exceed 300 LOC — they orchestrate DB + external calls + \
business rules inline (transaction script, not a domain model)",
            big.len()
        ),
        fix: "Extract the procedure into a use-case/application service that calls domain methods \
+ a repository; the handler should only parse input, call the use case, and map output"
            .to_string(),
        validate: "Handler shrinks to <~60 LOC and contains no core business/branching logic; \
the extracted use case has unit tests; behavior unchanged (same e2e)."
            .to_string(),
        dimension: None,
    })
}

fn check_domain_logic_in_controllers(scan: &RepoScan) -> Option<Finding> {
    let re =
        Regex::new(r"calculatePremium|computePrice|price|amount.*total|taxRate|refund").unwrap();

    let api_files: Vec<&Path> = scan
        .files
        .iter()
        .filter(|p| {
            p.to_string_lossy().contains("/api/")
                && p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| matches!(e, "ts" | "tsx" | "js" | "jsx"))
                    .unwrap_or(false)
        })
        .map(|p| p.as_path())
        .collect();

    let hits = scan.grep_files(&api_files, &re);
    let mut hit_files: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for h in &hits {
        hit_files.insert(h.path.to_string_lossy().to_string());
    }

    if hit_files.is_empty() {
        return None;
    }

    let sample = hit_files
        .iter()
        .take(4)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    Some(Finding {
        check: "domain-logic-in-controllers".to_string(),
        severity: Severity::High,
        path: sample,
        detail: format!(
            "{} controller(s) contain domain calculations (pricing/tax/refund) — \
money rules are scattered across the HTTP layer instead of one domain module",
            hit_files.len()
        ),
        fix: "Move all pricing/business rules into a single domain service; \
controllers call it — they should not contain calculation logic"
            .to_string(),
        validate: "A grep for core money math in app/api returns 0; \
the domain service is the single source of truth and is unit-tested."
            .to_string(),
        dimension: None,
    })
}

fn has_value_object_type(scan: &RepoScan) -> bool {
    let re = Regex::new(r"class Money|type Money|Money =|class Email|ValueObject").unwrap();
    let ts_files: Vec<&Path> = scan.files_with_ext(&["ts", "tsx"]);
    ts_files.iter().any(|p| {
        let full = scan.root.join(p);
        std::fs::read_to_string(&full)
            .map(|c| re.is_match(&c))
            .unwrap_or(false)
    })
}

fn check_value_objects_as_primitives(scan: &RepoScan, has_vo: bool) -> Option<Finding> {
    if has_vo {
        return None;
    }
    let re = Regex::new(r"(amount|price|total|payout)[A-Za-z]*\s*:\s*number").unwrap();
    let ts_files: Vec<&Path> = scan.files_with_ext(&["ts", "tsx"]);
    let hits = scan.grep_files(&ts_files, &re);
    if hits.is_empty() {
        return None;
    }

    let mut hit_files: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for h in &hits {
        hit_files.insert(h.path.to_string_lossy().to_string());
    }
    let sample = hit_files
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");

    Some(Finding {
        check: "value-objects-as-primitives".to_string(),
        severity: Severity::Medium,
        path: sample,
        detail: "Money amounts and identifiers are passed as bare `number`/`string` — \
primitive obsession; no Money/Email value objects, so rounding/currency/format rules \
aren't enforced by the type"
            .to_string(),
        fix: "Introduce value objects: a Money type (integer cents + currency, with add/multiply) \
and typed identifiers; replace raw number arithmetic on currency"
            .to_string(),
        validate: "All currency math goes through Money; \
a test asserts Money rejects fractional cents and mismatched currencies."
            .to_string(),
        dimension: None,
    })
}

fn check_missing_acl(scan: &RepoScan) -> Vec<Finding> {
    let vendors = [
        ("stripe", r"(?i)stripe"),
        ("aws-sdk", r"(?i)aws-sdk|@aws-sdk"),
        ("twilio", r"(?i)twilio"),
        ("sendgrid", r"(?i)sendgrid"),
        ("box", r"(?i)box-node-sdk"),
    ];

    let ts_js_all: Vec<&Path> = scan.files_with_ext(&["ts", "tsx", "js", "jsx"]);

    let mut findings = Vec::new();
    for (vendor_name, dep_pattern) in &vendors {
        let re_dep = Regex::new(dep_pattern).unwrap();
        if !scan.dep_seen(&re_dep) {
            continue;
        }

        // Count ts/js files referencing this vendor outside adapter/acl paths
        let re_ref = Regex::new(&format!("(?i){}", regex::escape(vendor_name))).unwrap();
        let outside_adapter: Vec<String> = ts_js_all
            .iter()
            .filter(|p| {
                let s = p.to_string_lossy();
                !s.contains("/adapter")
                    && !s.contains("/acl/")
                    && !s.contains(&format!("/lib/{}/", vendor_name))
            })
            .filter(|p| {
                let full = scan.root.join(p);
                std::fs::read_to_string(&full)
                    .map(|c| re_ref.is_match(&c))
                    .unwrap_or(false)
            })
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        if outside_adapter.len() > 5 {
            let n = outside_adapter.len();
            findings.push(Finding {
                check: "missing-acl".to_string(),
                severity: Severity::High,
                path: format!("({} files touch {})", n, vendor_name),
                detail: format!(
                    "The {} integration is referenced directly in {} files outside a dedicated \
adapter — no anti-corruption layer; the external vendor's vocabulary/shape leaks into the domain",
                    vendor_name, n
                ),
                fix: format!(
                    "Put one adapter/ACL module (lib/{}/adapter) that translates {} DTOs ↔ domain \
types; everything else depends on the domain interface, not the vendor SDK",
                    vendor_name, vendor_name
                ),
                validate: format!(
                    "Only the adapter imports the {} SDK (grep for the SDK import outside lib/{} \
returns 0); domain code references domain types, swapping the vendor touches one module.",
                    vendor_name, vendor_name
                ),
                dimension: None,
            });
        }
    }
    findings
}

fn check_repository_absence(scan: &RepoScan) -> Option<Finding> {
    let re = Regex::new(r"prisma\.[a-zA-Z]+\.(find|create|update|delete|upsert)").unwrap();
    let api_ts: Vec<&Path> = scan
        .files
        .iter()
        .filter(|p| {
            p.to_string_lossy().contains("/api/")
                && p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| matches!(e, "ts" | "tsx" | "js" | "jsx"))
                    .unwrap_or(false)
        })
        .map(|p| p.as_path())
        .collect();

    let hits = scan.grep_files(&api_ts, &re);
    let mut hit_files: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for h in &hits {
        hit_files.insert(h.path.to_string_lossy().to_string());
    }

    if hit_files.len() <= 5 {
        return None;
    }

    let n = hit_files.len();
    Some(Finding {
        check: "repository-absence".to_string(),
        severity: Severity::Medium,
        path: format!("({} handlers call prisma directly)", n),
        detail: format!(
            "{} controllers call Prisma directly — persistence concerns are spread across \
the HTTP layer, no repository abstraction; aggregates can be mutated from anywhere",
            n
        ),
        fix: "Introduce repositories per aggregate that encapsulate Prisma; \
controllers/use-cases depend on the repository interface"
            .to_string(),
        validate: "Route handlers contain no `prisma.` calls (grep returns 0 in app/api); \
persistence is swappable behind the repository interface; aggregate writes go through one place."
            .to_string(),
        dimension: None,
    })
}

fn check_aggregate_boundary_leak(scan: &RepoScan) -> Option<Finding> {
    let re_write = Regex::new(r"prisma\.([a-zA-Z]+)\.(create|update|upsert|delete)").unwrap();

    let api_ts: Vec<&Path> = scan
        .files
        .iter()
        .filter(|p| {
            p.to_string_lossy().contains("/api/")
                && p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| matches!(e, "ts" | "tsx" | "js" | "jsx"))
                    .unwrap_or(false)
        })
        .map(|p| p.as_path())
        .collect();

    let mut leaky: Vec<String> = Vec::new();
    for rel in &api_ts {
        let full = scan.root.join(rel);
        let content = match std::fs::read_to_string(&full) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let mut models: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for cap in re_write.captures_iter(&content) {
            if let Some(m) = cap.get(1) {
                models.insert(m.as_str().to_string());
            }
        }
        if models.len() >= 3 {
            leaky.push(format!("{}:{}models", rel.display(), models.len()));
        }
    }

    if leaky.is_empty() {
        return None;
    }

    let sample = leaky
        .iter()
        .take(4)
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Some(Finding {
        check: "aggregate-boundary-leak".to_string(),
        severity: Severity::Medium,
        path: sample,
        detail: format!(
            "{} handler(s) write 3+ different models in one operation with no aggregate root \
coordinating them — invariants spanning those entities aren't enforced together; \
partial writes leave inconsistent state",
            leaky.len()
        ),
        fix: "Define aggregate roots and mutate child entities only through the root \
inside a single transaction"
            .to_string(),
        validate: "Cross-entity writes happen inside one `prisma.$transaction` via the aggregate; \
a test forces a mid-write failure and asserts no partial state persists."
            .to_string(),
        dimension: None,
    })
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(dir: &std::path::Path, rel: &str, content: &str) {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, content).unwrap();
    }

    #[test]
    fn ts_fixture_orm_entities_detected() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "package.json",
            r#"{"dependencies":{"@prisma/client":"*"}}"#,
        );
        write(
            root,
            "prisma/schema.prisma",
            "model User {}\nmodel Post {}\n",
        );
        write(
            root,
            "app/api/things/route.ts",
            "export async function GET() {\n  const x = await prisma.user.findMany();\n}\n",
        );

        let report = detect(root).unwrap();
        let orm = report
            .signals
            .iter()
            .find(|s| s.id == "orm-entities")
            .unwrap();
        assert!(orm.present, "orm-entities should be present");
        assert!(orm.evidence.contains("prisma:2 models"));
    }

    #[test]
    fn ts_fixture_anemic_domain_fired() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "package.json",
            r#"{"dependencies":{"@prisma/client":"*"}}"#,
        );
        write(
            root,
            "prisma/schema.prisma",
            "model User {}\nmodel Post {}\n",
        );
        write(
            root,
            "app/api/things/route.ts",
            "export async function GET() {\n  const x = await prisma.user.findMany();\n}\n",
        );

        let report = analyze(root).unwrap();
        assert_eq!(report.status, CoverageStatus::Scanned);
        assert!(report.scanned_files > 0);
        assert!(
            report.findings.iter().any(|f| f.check == "anemic-domain"),
            "anemic-domain must fire when no domain layer exists"
        );
    }

    #[test]
    fn rust_fixture_stack_not_covered() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "Cargo.toml",
            "[package]\nname = \"foo\"\nversion = \"0.1.0\"\n",
        );
        write(root, "src/main.rs", "fn main() {}\n");

        let report = analyze(root).unwrap();
        assert_eq!(report.status, CoverageStatus::StackNotCovered);
        assert!(report.findings.is_empty());
        assert!(report.stacks.iter().any(|s| s == "rust"));
        assert_eq!(report.scanned_files, 0);
    }

    #[test]
    fn prune_node_modules() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "package.json", "{}");
        write(root, "node_modules/x.ts", "export const x = 1;");
        write(root, "src/index.ts", "const y = 1;");

        let scan = crate::suite::scan::RepoScan::build(root).unwrap();
        let node_mod_files: Vec<_> = scan
            .files
            .iter()
            .filter(|p| p.to_string_lossy().contains("node_modules"))
            .collect();
        assert!(
            node_mod_files.is_empty(),
            "node_modules files must not be scanned"
        );
    }
}
