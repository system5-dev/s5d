//! Scaling/performance analyzer: dimensions (detect) and anti-pattern findings (analyze).
//!
//! Covered stacks for analyze: TypeScript, JavaScript.
//! detect is partially polyglot (counts Go/Python handlers too).

use anyhow::Result;
use regex::Regex;
use std::path::Path;

use super::{
    AnalysisReport, CoverageStatus, DetectReport, DetectSignal, DetectSummary, Finding, Severity,
    Summary,
};
use crate::suite::scan::{RepoScan, Stack};

const COVERED: &[Stack] = &[Stack::TypeScript, Stack::JavaScript];

/// Emit detect signals for the repo at `root`.
pub fn detect(root: &Path) -> Result<DetectReport> {
    let scan = RepoScan::build(root)?;
    let root_str = root.to_string_lossy().to_string();
    let scanned = scan.files.len();
    let stacks: Vec<String> = scan.stacks.iter().map(|s| s.as_str().to_string()).collect();

    let signals = vec![
        detect_orm(&scan),
        detect_db(&scan),
        detect_api(&scan),
        detect_cache(&scan),
        detect_queue(&scan),
        detect_realtime(&scan),
        detect_runtime(&scan),
        detect_heavy_libs(&scan),
    ];

    let present = signals.iter().filter(|s| s.present).count();
    let total = signals.len();

    Ok(DetectReport {
        root: root_str,
        scanned_files: scanned,
        stacks,
        uncovered_stacks: scan
            .stacks
            .iter()
            .filter(|s| !COVERED.contains(s))
            .map(|s| s.as_str().to_string())
            .collect(),
        truncated: scan.truncated,
        signals,
        summary: DetectSummary {
            present,
            signals_total: total,
        },
    })
}

/// Run all scaling analyze checks on the repo at `root`.
pub fn analyze(root: &Path) -> Result<AnalysisReport> {
    let scan = RepoScan::build(root)?;
    let root_str = root.to_string_lossy().to_string();
    let stacks: Vec<String> = scan.stacks.iter().map(|s| s.as_str().to_string()).collect();
    let uncovered_stacks: Vec<String> = scan
        .stacks
        .iter()
        .filter(|s| !COVERED.contains(s))
        .map(|s| s.as_str().to_string())
        .collect();

    let covered = scan.stacks.iter().any(|s| COVERED.contains(s));
    if !covered {
        return Ok(AnalysisReport {
            root: root_str,
            scanned_files: 0,
            stacks,
            uncovered_stacks: uncovered_stacks.clone(),
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
            uncovered_stacks: uncovered_stacks.clone(),
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

    if let Some(f) = check_n_plus_1(&scan) {
        findings.push(f);
    }
    if let Some(f) = check_unbounded_query(&scan) {
        findings.push(f);
    }
    if let Some(f) = check_sync_heavy_in_request(&scan) {
        findings.push(f);
    }
    if let Some(f) = check_missing_timeout(&scan) {
        findings.push(f);
    }
    if let Some(f) = check_local_fs_write(&scan) {
        findings.push(f);
    }
    if let Some(f) = check_no_cache_layer(&scan, scanned_files) {
        findings.push(f);
    }
    if let Some(f) = check_serverless_conn_pool(&scan) {
        findings.push(f);
    }

    let summary = Summary::from_findings(&findings);

    Ok(AnalysisReport {
        root: root_str,
        scanned_files,
        stacks,
        uncovered_stacks,
        truncated: scan.truncated,
        status: CoverageStatus::Scanned,
        findings,
        summary,
    })
}

// ── detect helpers ────────────────────────────────────────────────────────────

fn detect_orm(scan: &RepoScan) -> DetectSignal {
    let mut parts: Vec<String> = Vec::new();

    // prisma: schema file at any depth
    if scan.root.join("prisma/schema.prisma").exists()
        || scan
            .files
            .iter()
            .any(|p| p.file_name().and_then(|n| n.to_str()) == Some("schema.prisma"))
    {
        parts.push("prisma".to_string());
    }

    let re = Regex::new(r"(?i)typeorm|sequelize|sqlalchemy|gorm\.io|jinzhu/gorm").unwrap();
    if scan.dep_seen(&re) {
        if re.is_match("typeorm") { /* not needed — dep_seen checked */ }
        // Record which ones
        for name in &["typeorm", "sequelize", "sqlalchemy", "gorm"] {
            let r = Regex::new(&format!("(?i){}", name)).unwrap();
            if scan.dep_seen(&r) {
                parts.push(name.to_string());
            }
        }
    }

    // deduplicate
    parts.sort();
    parts.dedup();
    let present = !parts.is_empty();
    DetectSignal {
        id: "orm".to_string(),
        present,
        evidence: parts.join(","),
    }
}

fn detect_db(scan: &RepoScan) -> DetectSignal {
    let orm_present = detect_orm(scan).present;
    let mut parts: Vec<String> = Vec::new();
    if orm_present {
        parts.push("via-orm".to_string());
    }

    // Check .env.example / package.json for connection strings
    let conn_re = Regex::new(r"(?i)DATABASE_URL|POSTGRES|MYSQL").unwrap();
    for name in &[".env.example", "package.json"] {
        let path = scan.root.join(name);
        if let Ok(content) = std::fs::read_to_string(&path) {
            if conn_re.is_match(&content) {
                parts.push("conn-string".to_string());
                break;
            }
        }
    }

    parts.sort();
    parts.dedup();
    let present = !parts.is_empty();
    DetectSignal {
        id: "db".to_string(),
        present,
        evidence: parts.join(","),
    }
}

fn detect_api(scan: &RepoScan) -> DetectSignal {
    let api_route_count = scan
        .files
        .iter()
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            (name == "route.ts" || name == "route.js") && p.to_string_lossy().contains("/api/")
        })
        .count();

    let re_go = Regex::new(r"http\.(HandleFunc|Handle)|chi\.|gin\.|echo\.").unwrap();
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

    let re_py = Regex::new(r"@(app|router)\.(get|post|put|delete)|@(app)\.route").unwrap();
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
        id: "api".to_string(),
        present,
        evidence: if present {
            format!("{} handlers", total)
        } else {
            String::new()
        },
    }
}

fn detect_cache(scan: &RepoScan) -> DetectSignal {
    let re =
        Regex::new(r#"(?i)ioredis|"redis"|node-redis|go-redis|redis/go-redis|memcached"#).unwrap();
    let mut parts: Vec<String> = Vec::new();
    for name in &["redis", "memcached", "go-redis"] {
        let r = Regex::new(&format!("(?i){}", name)).unwrap();
        if scan.dep_seen(&r) {
            parts.push(name.to_string());
        }
    }
    // ioredis
    let r_io = Regex::new(r"(?i)ioredis").unwrap();
    if scan.dep_seen(&r_io) && !parts.contains(&"redis".to_string()) {
        parts.push("redis".to_string());
    }
    let _ = re; // used for pattern construction above
    parts.sort();
    parts.dedup();
    let present = !parts.is_empty();
    DetectSignal {
        id: "cache".to_string(),
        present,
        evidence: if present {
            parts.join(",")
        } else {
            "none-detected".to_string()
        },
    }
}

fn detect_queue(scan: &RepoScan) -> DetectSignal {
    let matchers: &[(&str, &str)] = &[
        ("bull", r#"(?i)bullmq|"bull""#),
        ("sqs", r"(?i)aws-sdk.*sqs|@aws-sdk/client-sqs"),
        ("kafka", r"(?i)kafkajs|segmentio/kafka|sarama"),
        ("celery", r"(?i)celery"),
        ("rabbitmq", r"(?i)amqplib|rabbitmq|streadway/amqp"),
    ];
    let mut parts: Vec<String> = Vec::new();
    for (name, pattern) in matchers {
        let re = Regex::new(pattern).unwrap();
        if scan.dep_seen(&re) {
            parts.push(name.to_string());
        }
    }
    // vercel crons
    let vercel_path = scan.root.join("vercel.json");
    if let Ok(content) = std::fs::read_to_string(&vercel_path) {
        if content.contains("\"crons\"") {
            parts.push("vercel-cron".to_string());
        }
    }
    let present = !parts.is_empty();
    DetectSignal {
        id: "queue".to_string(),
        present,
        evidence: if present {
            parts.join(",")
        } else {
            "none-detected".to_string()
        },
    }
}

fn detect_realtime(scan: &RepoScan) -> DetectSignal {
    let re_dep = Regex::new(r#"(?i)socket\.io|ws"|websocket"#).unwrap();
    let re_code = Regex::new(r"text/event-stream|EventSource").unwrap();

    let has_dep = scan.dep_seen(&re_dep);
    let ts_js: Vec<&Path> = scan.files_with_ext(&["ts", "tsx", "js", "jsx"]);
    let has_sse = ts_js.iter().any(|p| {
        let full = scan.root.join(p);
        std::fs::read_to_string(&full)
            .map(|c| re_code.is_match(&c))
            .unwrap_or(false)
    });

    let present = has_dep || has_sse;
    let mut parts = Vec::new();
    if has_dep {
        parts.push("ws");
    }
    if has_sse {
        parts.push("sse");
    }
    DetectSignal {
        id: "realtime".to_string(),
        present,
        evidence: if present {
            parts.join(",")
        } else {
            String::new()
        },
    }
}

fn detect_runtime(scan: &RepoScan) -> DetectSignal {
    let mut parts: Vec<String> = Vec::new();

    if scan.root.join("vercel.json").exists() {
        parts.push("serverless:vercel".to_string());
    }

    let re_lambda = Regex::new(r"(?i)lambda|serverless").unwrap();
    for name in &["package.json", "serverless.yml"] {
        let path = scan.root.join(name);
        if let Ok(content) = std::fs::read_to_string(&path) {
            if re_lambda.is_match(&content) {
                parts.push("serverless".to_string());
                break;
            }
        }
    }

    let has_docker = scan.files.iter().any(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with("Dockerfile"))
            .unwrap_or(false)
    });
    if has_docker {
        parts.push("container:long-running".to_string());
    }

    parts.sort();
    parts.dedup();
    let present = !parts.is_empty();
    DetectSignal {
        id: "runtime".to_string(),
        present,
        evidence: parts.join(","),
    }
}

fn detect_heavy_libs(scan: &RepoScan) -> DetectSignal {
    let matchers: &[(&str, &str)] = &[
        ("puppeteer", r"(?i)puppeteer|playwright-core"),
        ("sharp", r#"(?i)"sharp""#),
        ("pdf", r"(?i)pdfkit|pdf-lib|@react-pdf"),
    ];
    let mut parts: Vec<String> = Vec::new();
    for (name, pattern) in matchers {
        let re = Regex::new(pattern).unwrap();
        if scan.dep_seen(&re) {
            parts.push(name.to_string());
        }
    }
    let present = !parts.is_empty();
    DetectSignal {
        id: "heavy-libs".to_string(),
        present,
        evidence: parts.join(","),
    }
}

// ── analyze helpers ───────────────────────────────────────────────────────────

fn api_ts_js_files(scan: &RepoScan) -> Vec<&Path> {
    scan.files
        .iter()
        .filter(|p| {
            let s = p.to_string_lossy();
            (s.contains("/api/") || {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                name.starts_with("route.")
            }) && p
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| matches!(e, "ts" | "tsx" | "js" | "jsx"))
                .unwrap_or(false)
        })
        .map(|p| p.as_path())
        .collect()
}

fn check_n_plus_1(scan: &RepoScan) -> Option<Finding> {
    let re =
        Regex::new(r"\.map\([^)]*await (prisma|db)\.|for *\([^)]*\) *\{[^}]*await (prisma|db)\.")
            .unwrap();
    let ts_js: Vec<&Path> = scan.files_with_ext(&["ts", "tsx", "js", "jsx"]);
    let hits = scan.grep_files(&ts_js, &re);
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
        check: "n-plus-1".to_string(),
        severity: Severity::High,
        path: sample,
        detail: format!(
            "{} site(s) await a DB call inside a loop/map — query count grows with row count",
            hits.len()
        ),
        fix: "Batch with a single findMany({{where:{{id:{{in:[...]}}}}}}) or use include/select \
to join in one query"
            .to_string(),
        validate: "Enable query logging; the endpoint issues 1-2 queries regardless of N rows. \
Load-test with 10x rows: p95 must stay flat, not climb linearly."
            .to_string(),
        dimension: Some("db".to_string()),
    })
}

fn check_unbounded_query(scan: &RepoScan) -> Option<Finding> {
    let re_findmany = Regex::new(r"findMany\(").unwrap();
    let re_take = Regex::new(r"take:").unwrap();

    let ts_js: Vec<&Path> = scan.files_with_ext(&["ts", "tsx", "js", "jsx"]);

    let mut hit_lines: Vec<(String, usize, String)> = Vec::new();
    for rel in &ts_js {
        let full = scan.root.join(rel);
        let content = match std::fs::read_to_string(&full) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for (i, line) in content.lines().enumerate() {
            if re_findmany.is_match(line) && !re_take.is_match(line) {
                hit_lines.push((rel.to_string_lossy().to_string(), i + 1, line.to_string()));
            }
        }
    }

    if hit_lines.is_empty() {
        return None;
    }

    let mut hit_files: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (f, _, _) in &hit_lines {
        hit_files.insert(f.clone());
    }
    let sample = hit_files
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");

    Some(Finding {
        check: "unbounded-query".to_string(),
        severity: Severity::High,
        path: sample,
        detail: format!(
            "{} findMany call(s) without take/cursor — full-table reads that grow unbounded with data",
            hit_lines.len()
        ),
        fix: "Add take + cursor/offset pagination; cap default page size; index the order-by column"
            .to_string(),
        validate: "Seed 100k rows; endpoint returns <= page size and p95 stays <300ms with flat \
memory. EXPLAIN shows an index scan, not a seq scan."
            .to_string(),
        dimension: Some("db".to_string()),
    })
}

fn check_sync_heavy_in_request(scan: &RepoScan) -> Option<Finding> {
    let re = Regex::new(r"puppeteer|generatePdf|pdfkit|sharp\(|setContent\(").unwrap();
    let api_files = api_ts_js_files(scan);
    let hits = scan.grep_files(&api_files, &re);
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
        check: "sync-heavy-in-request".to_string(),
        severity: Severity::High,
        path: sample,
        detail: format!(
            "{} route handler(s) run heavy work (PDF/Puppeteer/image) inline — \
ties up the request, no backpressure, serverless timeout risk",
            hit_files.len()
        ),
        fix: "Move to a background job/queue; return 202 + a status URL; render off the hot path"
            .to_string(),
        validate:
            "Load-test the endpoint at concurrency C: request p95 drops to the enqueue time; \
queue depth is observable; a failed job retries without losing the request."
                .to_string(),
        dimension: Some("api".to_string()),
    })
}

fn check_missing_timeout(scan: &RepoScan) -> Option<Finding> {
    let re_call = Regex::new(r"fetch\(|axios\.").unwrap();
    let re_timeout = Regex::new(r"AbortController|signal:|timeout").unwrap();

    let api_files = api_ts_js_files(scan);

    let mut hit_lines: Vec<(String, usize)> = Vec::new();
    for rel in &api_files {
        let full = scan.root.join(rel);
        let content = match std::fs::read_to_string(&full) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for (i, line) in content.lines().enumerate() {
            if re_call.is_match(line) && !re_timeout.is_match(line) {
                hit_lines.push((rel.to_string_lossy().to_string(), i + 1));
            }
        }
    }

    if hit_lines.is_empty() {
        return None;
    }

    let mut hit_files: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (f, _) in &hit_lines {
        hit_files.insert(f.clone());
    }
    let sample = hit_files
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");

    Some(Finding {
        check: "missing-timeout".to_string(),
        severity: Severity::Medium,
        path: sample,
        detail: format!(
            "{} outbound call(s) in request handlers with no visible timeout/AbortController — \
a slow upstream hangs the worker and cascades",
            hit_lines.len()
        ),
        fix: "Wrap every outbound call in AbortController with a budget (e.g. 5s); \
fail fast; add a circuit breaker on repeated upstream failure"
            .to_string(),
        validate: "Inject a slow upstream stub: the request must fail at the timeout, not hang; \
worker pool stays available for other requests."
            .to_string(),
        dimension: Some("concurrency".to_string()),
    })
}

fn check_local_fs_write(scan: &RepoScan) -> Option<Finding> {
    let re = Regex::new(r"writeFile|createWriteStream|os\.tmpdir|/tmp/").unwrap();

    // Files in /api/, route.*, lib/, utils/
    let target_files: Vec<&Path> = scan
        .files
        .iter()
        .filter(|p| {
            let s = p.to_string_lossy();
            (s.contains("/api/")
                || p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("route."))
                    .unwrap_or(false)
                || s.contains("/lib/")
                || s.contains("/utils/"))
                && p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| matches!(e, "ts" | "tsx" | "js" | "jsx"))
                    .unwrap_or(false)
        })
        .map(|p| p.as_path())
        .collect();

    let hits = scan.grep_files(&target_files, &re);
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
        check: "local-fs-write".to_string(),
        severity: Severity::Medium,
        path: sample,
        detail: format!(
            "{} local-disk write(s) on a request/serverless path — \
state is lost between invocations and not shared across instances",
            hits.len()
        ),
        fix: "Write to object storage (S3/GCS) and stream from there; \
treat local disk as ephemeral scratch only"
            .to_string(),
        validate: "Deploy 2+ instances behind the LB; an artifact created on instance A is \
retrievable via instance B; redeploy mid-flight loses nothing."
            .to_string(),
        dimension: Some("runtime".to_string()),
    })
}

fn check_no_cache_layer(scan: &RepoScan, scanned_files: usize) -> Option<Finding> {
    let _ = scanned_files;
    let re_cache = Regex::new(r#"(?i)ioredis|"redis"|node-redis|go-redis|redis/go-redis"#).unwrap();
    if scan.dep_seen(&re_cache) {
        return None;
    }

    // Count api handler files exporting GET/POST
    let re_handler = Regex::new(r"export (async )?function (GET|POST)").unwrap();
    let ts_js_api: Vec<&Path> = scan
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

    let handler_count = ts_js_api
        .iter()
        .filter(|p| {
            let full = scan.root.join(p);
            std::fs::read_to_string(&full)
                .map(|c| re_handler.is_match(&c))
                .unwrap_or(false)
        })
        .count();

    if handler_count <= 20 {
        return None;
    }

    Some(Finding {
        check: "no-cache-layer".to_string(),
        severity: Severity::Medium,
        path: "(repo)".to_string(),
        detail: format!(
            "No cache layer detected (no redis) but {} API handlers — \
every read hits the DB; repeated reads aren't memoized",
            handler_count
        ),
        fix: "Add Redis (or equivalent) for hot reads; set TTLs per data volatility".to_string(),
        validate: "Measure DB QPS before/after under load: origin QPS drops, \
cache hit ratio >70% on hot keys, p95 improves; cache-miss path still correct."
            .to_string(),
        dimension: Some("cache".to_string()),
    })
}

fn check_serverless_conn_pool(scan: &RepoScan) -> Option<Finding> {
    // Only fires if postgresql AND vercel AND no pooler config
    let re_pg = Regex::new(r#"(?i)provider\s*=\s*"postgresql"|postgresql://"#).unwrap();

    let prisma_path = scan.root.join("prisma/schema.prisma");
    let env_path = scan.root.join(".env.example");

    let pg_in_prisma = std::fs::read_to_string(&prisma_path)
        .map(|c| re_pg.is_match(&c))
        .unwrap_or(false);
    let pg_in_env = std::fs::read_to_string(&env_path)
        .map(|c| re_pg.is_match(&c))
        .unwrap_or(false);

    if !pg_in_prisma && !pg_in_env {
        return None;
    }
    if !scan.root.join("vercel.json").exists() {
        return None;
    }

    // Check pooler config in schema or env
    let re_pooler =
        Regex::new(r"(?i)pgbouncer|connection_limit|accelerate|prisma://|pooler").unwrap();
    let has_pooler = std::fs::read_to_string(&prisma_path)
        .map(|c| re_pooler.is_match(&c))
        .unwrap_or(false)
        || std::fs::read_to_string(&env_path)
            .map(|c| re_pooler.is_match(&c))
            .unwrap_or(false);

    if has_pooler {
        return None;
    }

    Some(Finding {
        check: "serverless-conn-pool".to_string(),
        severity: Severity::High,
        path: "prisma/schema.prisma".to_string(),
        detail: "Postgres + Prisma on Vercel serverless with no visible pooler \
(pgbouncer/Accelerate/connection_limit) — each cold function opens its own connection; \
bursts exhaust the DB connection cap"
            .to_string(),
        fix: "Front Postgres with a pooler (pgbouncer in transaction mode, Supabase pooler, \
or Prisma Accelerate); set connection_limit=1 per function"
            .to_string(),
        validate: "Load-test at concurrency >> DB max_connections: no 'too many clients' errors; \
active connections stay bounded at the pooler, not the function count."
            .to_string(),
        dimension: Some("db".to_string()),
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
    fn ts_fixture_unbounded_query_fired() {
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
            report.findings.iter().any(|f| f.check == "unbounded-query"),
            "unbounded-query must fire"
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
        assert_eq!(report.scanned_files, 0);
        assert!(report.stacks.iter().any(|s| s == "rust"));
    }
}
