use serde_json::{json, Value};
use std::io::{BufRead, Write};

pub fn run_mcp_server() -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let method = req["method"].as_str().unwrap_or("");
        let id = req.get("id").cloned();

        // Notifications have no id — don't respond
        if id.is_none() {
            continue;
        }

        let result = dispatch(method, &req["params"]);

        let response = match result {
            Ok(r) => json!({"jsonrpc":"2.0","id":id,"result":r}),
            Err(e) => {
                let is_not_found = e.to_string().starts_with("Method not found");
                if is_not_found {
                    json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":e.to_string()}})
                } else {
                    json!({"jsonrpc":"2.0","id":id,"error":{"code":-32603,"message":e.to_string()}})
                }
            }
        };

        writeln!(out, "{}", response)?;
        out.flush()?;
    }
    Ok(())
}

fn dispatch(method: &str, params: &Value) -> anyhow::Result<Value> {
    match method {
        "initialize" => handle_initialize(),
        "tools/list" => handle_tools_list(),
        "tools/call" => handle_tools_call(params),
        _ => Err(anyhow::anyhow!("Method not found: {}", method)),
    }
}

fn handle_initialize() -> anyhow::Result<Value> {
    Ok(json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {"tools": {}},
        "serverInfo": {"name": "s5d", "version": "2.0.0"}
    }))
}

fn handle_tools_list() -> anyhow::Result<Value> {
    Ok(json!({
        "tools": [
            {
                "name": "codebase_index",
                "description": "Build or rebuild the full codebase index for a repository",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Repository root path (default: current dir)"}
                    }
                }
            },
            {
                "name": "codebase_update",
                "description": "Incrementally update index using git diff (only re-indexes changed files)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Repository root path (default: current dir)"}
                    }
                }
            },
            {
                "name": "codebase_search",
                "description": "Search the codebase index using hybrid BM25+vector search with RRF fusion",
                "inputSchema": {
                    "type": "object",
                    "required": ["query"],
                    "properties": {
                        "query": {"type": "string", "description": "Search query"},
                        "path": {"type": "string", "description": "Repository root path (default: current dir)"},
                        "top_k": {"type": "integer", "description": "Number of results (default: 10)"}
                    }
                }
            },
            {
                "name": "codebase_symbol",
                "description": "Look up a symbol by name — returns definition location and kind",
                "inputSchema": {
                    "type": "object",
                    "required": ["name"],
                    "properties": {
                        "name": {"type": "string", "description": "Symbol name to look up"},
                        "path": {"type": "string", "description": "Repository root path (default: current dir)"}
                    }
                }
            },
            {
                "name": "s5d_analyze",
                "description": "Analyze a codebase and return Metamodel YAML (domains, capabilities, components)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Repository root path (default: current dir)"},
                        "product": {"type": "string", "description": "Product name (default: directory name)"}
                    }
                }
            },
            {
                "name": "trace_build",
                "description": "Build trace links between spec artifacts and code. Scans annotations + infers from symbols",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec"],
                    "properties": {
                        "spec": {"type": "string", "description": "Path to the spec YAML file"},
                        "path": {"type": "string", "description": "Repository root path (default: current dir)"}
                    }
                }
            },
            {
                "name": "trace_check",
                "description": "Check trace coverage for a spec — reports matched, unmatched, and coverage percentage",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec"],
                    "properties": {
                        "spec": {"type": "string", "description": "Path to the spec YAML file"},
                        "path": {"type": "string", "description": "Repository root path (default: current dir)"}
                    }
                }
            },
            {
                "name": "trace_blast_radius",
                "description": "Compute blast radius for a spec — domains touched, capabilities affected, cross-domain edges, weakest link",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec"],
                    "properties": {
                        "spec": {"type": "string", "description": "Path to the spec YAML file"},
                        "path": {"type": "string", "description": "Repository root path (default: current dir)"}
                    }
                }
            },
            {
                "name": "fpf_search",
                "description": "Search the FPF (First Principles Framework) spec using BM25 full-text search. Returns ranked sections.",
                "inputSchema": {
                    "type": "object",
                    "required": ["query"],
                    "properties": {
                        "query": {"type": "string", "description": "Search query terms"},
                        "limit": {"type": "integer", "description": "Max results (default: 5)"},
                        "full": {"type": "boolean", "description": "Return full section content (default: false)"}
                    }
                }
            },
            {
                "name": "fpf_sync",
                "description": "Download and index the latest FPF spec from GitHub. Run this before fpf_search if the index is empty.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "force": {"type": "boolean", "description": "Re-index even if up to date (default: false)"}
                    }
                }
            },
            {
                "name": "s5d_init",
                "description": "Initialize .s5d/ directory structure in the current working directory",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "s5d_new",
                "description": "Create a new spec from template",
                "inputSchema": {
                    "type": "object",
                    "required": ["id"],
                    "properties": {
                        "id": {"type": "string", "description": "Feature ID (e.g. feat.orders.tracking)"},
                        "tier": {"type": "string", "description": "Assurance tier: decision|standard|lightweight|high|note (default: standard)"},
                        "product": {"type": "string", "description": "Product name"},
                        "question": {"type": "string", "description": "Decision question (required for tier=decision)"},
                        "rationale": {"type": "string", "description": "Note rationale (required for tier=note)"}
                    }
                }
            },
            {
                "name": "s5d_validate",
                "description": "Validate a spec file against schema and business rules",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec"],
                    "properties": {
                        "spec": {"type": "string", "description": "Path to .s5d.yaml file"}
                    }
                }
            },
            {
                "name": "s5d_graph_check",
                "description": "Check graph structure — DFS cycle detection on spec relations",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec"],
                    "properties": {
                        "spec": {"type": "string", "description": "Path to .s5d.yaml file"}
                    }
                }
            },
            {
                "name": "s5d_preview",
                "description": "Dry-run import diff — shows what would be created/updated/linked",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec"],
                    "properties": {
                        "spec": {"type": "string", "description": "Path to .s5d.yaml file"}
                    }
                }
            },
            {
                "name": "s5d_approve",
                "description": "Record approval — binds spec_sha256 and diff_sha256",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec", "reviewer"],
                    "properties": {
                        "spec": {"type": "string", "description": "Path to .s5d.yaml file"},
                        "reviewer": {"type": "string", "description": "Reviewer name"}
                    }
                }
            },
            {
                "name": "s5d_run_gates",
                "description": "Execute configured gate commands for a spec",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec"],
                    "properties": {
                        "spec": {"type": "string", "description": "Path to .s5d.yaml file"}
                    }
                }
            },
            {
                "name": "s5d_import",
                "description": "Transactional import — apply spec to alias table and ledger",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec"],
                    "properties": {
                        "spec": {"type": "string", "description": "Path to .s5d.yaml file"},
                        "verified_by": {"type": "string", "description": "Who independently verified gates passed"},
                        "force": {"type": "boolean", "description": "Override methodological checks (default: false)"}
                    }
                }
            },
            {
                "name": "s5d_decide",
                "description": "Record a decision in a decision-tier spec",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec", "title", "winner", "confirmed_by", "context", "decision", "rationale", "consequences"],
                    "properties": {
                        "spec": {"type": "string", "description": "Path to .s5d.yaml file"},
                        "title": {"type": "string", "description": "Decision title"},
                        "winner": {"type": "string", "description": "Winner hypothesis ID"},
                        "confirmed_by": {"type": "string", "description": "Human who confirms the decision"},
                        "context": {"type": "string", "description": "Decision context"},
                        "decision": {"type": "string", "description": "What was decided"},
                        "rationale": {"type": "string", "description": "Why this was chosen"},
                        "consequences": {"type": "string", "description": "Expected consequences"},
                        "rejected": {"type": "string", "description": "Rejected hypothesis IDs (comma-separated)"}
                    }
                }
            },
            {
                "name": "s5d_add_hypothesis",
                "description": "Add a hypothesis to a decision-tier spec",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec", "title", "content", "scope"],
                    "properties": {
                        "spec": {"type": "string", "description": "Path to .s5d.yaml file"},
                        "title": {"type": "string", "description": "Hypothesis title"},
                        "content": {"type": "string", "description": "Hypothesis content/description"},
                        "scope": {"type": "string", "description": "Scope — where this applies"},
                        "kind": {"type": "string", "description": "Kind: system or episteme (default: system)"}
                    }
                }
            },
            {
                "name": "s5d_add_evidence",
                "description": "Add evidence to a hypothesis in a decision-tier spec",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec", "hypothesis_id", "evidence_type", "content", "verdict"],
                    "properties": {
                        "spec": {"type": "string", "description": "Path to .s5d.yaml file"},
                        "hypothesis_id": {"type": "string", "description": "Hypothesis ID to attach evidence to"},
                        "evidence_type": {"type": "string", "description": "Evidence type: internal, external, gate:test, etc."},
                        "content": {"type": "string", "description": "Evidence content"},
                        "verdict": {"type": "string", "description": "Verdict: pass, fail, refine"},
                        "formality": {"type": "integer", "description": "F-G-R Formality: rigor of evidence method (1-5)"},
                        "claim_scope": {"type": "string", "description": "F-G-R ClaimScope: what the claim covers (comma-separated)"},
                        "reliability": {"type": "number", "description": "F-G-R Reliability: probability claim is true (0.0-1.0)"}
                    }
                }
            },
            {
                "name": "s5d_reflect",
                "description": "Record reflection for a spec (OPERATE stage) — closes lifecycle with production evidence",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec", "summary", "heuristic"],
                    "properties": {
                        "spec": {"type": "string", "description": "Path to .s5d.yaml file"},
                        "summary": {"type": "string", "description": "Summary of what happened in production"},
                        "heuristic": {"type": "string", "description": "Reusable rule learned from this spec"},
                        "issues": {"type": "string", "description": "Issues encountered (comma-separated)"}
                    }
                }
            },
            {
                "name": "s5d_status",
                "description": "Show status of all specs in the current project",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "s5d_show",
                "description": "Show spec details — decision trace, hypothesis tree, or feature summary",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec"],
                    "properties": {
                        "spec": {"type": "string", "description": "Path to .s5d.yaml file"}
                    }
                }
            },
            {
                "name": "s5d_waiver",
                "description": "Record a waiver on a cross-domain link in a spec",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec", "gate", "reason", "condition", "approved_by"],
                    "properties": {
                        "spec": {"type": "string", "description": "Path to .s5d.yaml file"},
                        "gate": {"type": "string", "description": "Gate or link ID to waive"},
                        "reason": {"type": "string", "description": "Reason for waiver"},
                        "condition": {"type": "string", "description": "Condition under which waiver applies"},
                        "approved_by": {"type": "string", "description": "Who approved the waiver"}
                    }
                }
            },
            {
                "name": "s5d_rollback",
                "description": "Reverse last import for a spec — tombstones alias entries and appends rollback ledger entry",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec"],
                    "properties": {
                        "spec": {"type": "string", "description": "Path to .s5d.yaml file"}
                    }
                }
            },
            {
                "name": "s5d_drift_check",
                "description": "Compare live state vs last applied fingerprint. Checks all specs if none specified.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "spec": {"type": "string", "description": "Path to .s5d.yaml file (optional — checks all if omitted)"}
                    }
                }
            },
            {
                "name": "s5d_reconcile",
                "description": "Re-import to fix drift (desired-state restore). Reconciles all drifted specs if none specified.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "spec": {"type": "string", "description": "Path to .s5d.yaml file (optional — reconciles all drifted if omitted)"}
                    }
                }
            },
            {
                "name": "s5d_check",
                "description": "Architecture health check — coupling metrics, cycles, quality gate",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec"],
                    "properties": {
                        "spec": {"type": "string", "description": "Path to .s5d.yaml file"},
                        "threshold": {"type": "integer", "description": "Minimum health score to pass (default: 0)"}
                    }
                }
            },
            {
                "name": "s5d_metrics",
                "description": "Display per-domain coupling metrics for a spec",
                "inputSchema": {
                    "type": "object",
                    "required": ["spec"],
                    "properties": {
                        "spec": {"type": "string", "description": "Path to .s5d.yaml file"}
                    }
                }
            }
        ]
    }))
}

fn handle_tools_call(params: &Value) -> anyhow::Result<Value> {
    let name = params["name"].as_str().unwrap_or("");
    let args = &params["arguments"];

    let text = match name {
        "codebase_index" => tool_codebase_index(args)?,
        "codebase_update" => tool_codebase_update(args)?,
        "codebase_search" => tool_codebase_search(args)?,
        "codebase_symbol" => tool_codebase_symbol(args)?,
        "s5d_analyze" => tool_s5d_analyze(args)?,
        "trace_build" => tool_trace_build(args)?,
        "trace_check" => tool_trace_check(args)?,
        "trace_blast_radius" => tool_trace_blast_radius(args)?,
        "fpf_search" => tool_fpf_search(args)?,
        "fpf_sync" => tool_fpf_sync(args)?,
        "s5d_init" => tool_s5d_init(args)?,
        "s5d_new" => tool_s5d_new(args)?,
        "s5d_validate" => tool_s5d_validate(args)?,
        "s5d_graph_check" => tool_s5d_graph_check(args)?,
        "s5d_preview" => tool_s5d_preview(args)?,
        "s5d_approve" => tool_s5d_approve(args)?,
        "s5d_run_gates" => tool_s5d_run_gates(args)?,
        "s5d_import" => tool_s5d_import(args)?,
        "s5d_decide" => tool_s5d_decide(args)?,
        "s5d_add_hypothesis" => tool_s5d_add_hypothesis(args)?,
        "s5d_add_evidence" => tool_s5d_add_evidence(args)?,
        "s5d_reflect" => tool_s5d_reflect(args)?,
        "s5d_status" => tool_s5d_status(args)?,
        "s5d_show" => tool_s5d_show(args)?,
        "s5d_waiver" => tool_s5d_waiver(args)?,
        "s5d_rollback" => tool_s5d_rollback(args)?,
        "s5d_drift_check" => tool_s5d_drift_check(args)?,
        "s5d_reconcile" => tool_s5d_reconcile(args)?,
        "s5d_check" => tool_s5d_check(args)?,
        "s5d_metrics" => tool_s5d_metrics(args)?,
        _ => {
            return Ok(
                json!({"content":[{"type":"text","text":format!("Error: unknown tool '{}'", name)}],"isError":true}),
            )
        }
    };

    Ok(json!({"content":[{"type":"text","text":text}]}))
}

fn resolve_path(args: &Value) -> anyhow::Result<std::path::PathBuf> {
    let raw = args["path"].as_str().unwrap_or(".");
    let p = std::path::Path::new(raw)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(raw));
    Ok(p)
}

fn tool_codebase_index(args: &Value) -> anyhow::Result<String> {
    let path = resolve_path(args)?;
    let db_path = path.join(".s5d/codebase.db");
    let config = crate::codebase::IndexConfig {
        db_path,
        ..Default::default()
    };
    let stats = crate::codebase::index(&path, &config)?;
    Ok(format!(
        "Indexed {} files, {} chunks, {} symbols, {} embeddings",
        stats.files_indexed, stats.chunks_indexed, stats.symbols_found, stats.embeddings_generated
    ))
}

fn tool_codebase_update(args: &Value) -> anyhow::Result<String> {
    let path = resolve_path(args)?;
    let db_path = path.join(".s5d/codebase.db");
    let config = crate::codebase::IndexConfig {
        db_path,
        ..Default::default()
    };
    let stats = crate::codebase::update(&path, &config)?;
    Ok(format!(
        "Updated {} files, {} chunks, {} symbols, {} embeddings",
        stats.files_indexed, stats.chunks_indexed, stats.symbols_found, stats.embeddings_generated
    ))
}

fn tool_codebase_search(args: &Value) -> anyhow::Result<String> {
    let query = args["query"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: query"))?;
    let path = resolve_path(args)?;
    let top_k = args["top_k"].as_u64().unwrap_or(10) as usize;
    let db_path = path.join(".s5d/codebase.db");

    let index = crate::codebase::CodebaseIndex::open(&db_path)?;
    let results = crate::codebase::search::search(
        &index,
        query,
        crate::codebase::search::SearchMode::Fts,
        top_k,
        None,
    )?;

    if results.is_empty() {
        return Ok("No results found.".to_string());
    }

    let mut out = String::new();
    for (i, r) in results.iter().enumerate() {
        out.push_str(&format!(
            "[{}] {}:{}-{} (score: {:.2})\n{}\n\n",
            i + 1,
            r.file_path,
            r.start_line,
            r.end_line,
            r.score,
            r.text.trim()
        ));
    }
    Ok(out.trim_end().to_string())
}

fn tool_codebase_symbol(args: &Value) -> anyhow::Result<String> {
    let name = args["name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: name"))?;
    let path = resolve_path(args)?;
    let db_path = path.join(".s5d/codebase.db");

    let index = crate::codebase::CodebaseIndex::open(&db_path)?;
    let symbols = index.symbol_lookup(name)?;

    if symbols.is_empty() {
        return Ok(format!("Symbol '{}' not found.", name));
    }

    let mut out = String::new();
    for sym in &symbols {
        out.push_str(&format!(
            "{:<30} {}:{}\n",
            format!("{} {}", sym.kind, sym.name),
            sym.file_path,
            sym.line
        ));
    }
    Ok(out.trim_end().to_string())
}

fn tool_s5d_analyze(args: &Value) -> anyhow::Result<String> {
    let path = resolve_path(args)?;
    let product = args["product"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("product")
                .to_string()
        });
    let config = crate::AnalysisConfig {
        path: path.clone(),
        product,
        spec_id: "analysis".to_string(),
    };
    let result = crate::analyze(&config)?;
    let yaml = serde_yaml::to_string(&result.spec)?;
    Ok(yaml)
}

fn load_spec_from_arg(args: &Value) -> anyhow::Result<(std::path::PathBuf, crate::Spec)> {
    let spec_path = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let path = std::path::Path::new(spec_path);
    if !path.exists() {
        anyhow::bail!("spec not found: {}", spec_path);
    }
    let content = std::fs::read_to_string(path)?;
    let spec: crate::Spec =
        serde_yaml::from_str(&content).map_err(|e| anyhow::anyhow!("YAML parse error: {}", e))?;
    Ok((path.to_path_buf(), spec))
}

fn resolve_root_from_spec(
    spec_path: &std::path::Path,
    args: &Value,
) -> anyhow::Result<std::path::PathBuf> {
    if let Some(p) = args["path"].as_str() {
        return Ok(std::path::PathBuf::from(p)
            .canonicalize()
            .unwrap_or_else(|_| std::path::PathBuf::from(p)));
    }
    let abs = spec_path.canonicalize()?;
    let project = crate::S5dProject::find(&abs)
        .ok_or_else(|| anyhow::anyhow!("no .s5d/ found — is this spec inside an s5d project?"))?;
    Ok(project.root.clone())
}

fn tool_trace_build(args: &Value) -> anyhow::Result<String> {
    let (spec_path, spec) = load_spec_from_arg(args)?;
    let root = resolve_root_from_spec(&spec_path, args)?;
    let db_path = root.join(".s5d/codebase.db");
    if !db_path.exists() {
        anyhow::bail!("codebase index not found — run codebase_index first");
    }
    let index = crate::CodebaseIndex::open(&db_path)?;
    let links = crate::build_trace(&index, &spec, &root)?;

    let mut out = format!("Trace links built: {}\n", links.len());
    let mut by_source: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for link in &links {
        *by_source.entry(&link.source).or_insert(0) += 1;
    }
    for (source, count) in &by_source {
        out.push_str(&format!("  {}: {}\n", source, count));
    }
    Ok(out.trim_end().to_string())
}

fn tool_trace_check(args: &Value) -> anyhow::Result<String> {
    let (spec_path, spec) = load_spec_from_arg(args)?;
    let root = resolve_root_from_spec(&spec_path, args)?;
    let db_path = root.join(".s5d/codebase.db");
    if !db_path.exists() {
        anyhow::bail!("codebase index not found — run codebase_index first");
    }
    let index = crate::CodebaseIndex::open(&db_path)?;
    let report = crate::check_trace(&index, &spec)?;

    let mut out = String::new();
    out.push_str(&format!("Matched: {}\n", report.matched.len()));
    for link in &report.matched {
        out.push_str(&format!(
            "  {} {} → {}:{} [{}  {:.2}]\n",
            link.artifact_kind,
            link.artifact_id,
            link.file_path,
            link.line_start,
            link.source,
            link.confidence
        ));
    }
    if !report.unmatched_spec.is_empty() {
        out.push_str(&format!("\nUnmatched: {}\n", report.unmatched_spec.len()));
        for (kind, id) in &report.unmatched_spec {
            out.push_str(&format!("  {} {} — no code binding\n", kind, id));
        }
    }
    let total = report.matched.len() + report.unmatched_spec.len();
    if total > 0 {
        let coverage = report.matched.len() as f64 / total as f64 * 100.0;
        out.push_str(&format!(
            "\nCoverage: {:.0}% ({}/{})",
            coverage,
            report.matched.len(),
            total
        ));
    }
    Ok(out.trim_end().to_string())
}

fn tool_trace_blast_radius(args: &Value) -> anyhow::Result<String> {
    let (spec_path, spec) = load_spec_from_arg(args)?;
    let root = resolve_root_from_spec(&spec_path, args)?;
    let db_path = root.join(".s5d/codebase.db");
    if !db_path.exists() {
        anyhow::bail!("codebase index not found — run codebase_index first");
    }
    let index = crate::CodebaseIndex::open(&db_path)?;
    let br = crate::compute_blast_radius(&index, &spec)?;

    let mut out = String::new();
    out.push_str(&format!(
        "Domains touched: {}\n",
        if br.domains_touched.is_empty() {
            "none".into()
        } else {
            br.domains_touched.join(", ")
        }
    ));
    out.push_str(&format!(
        "Capabilities affected: {}\n",
        if br.capabilities_affected.is_empty() {
            "none".into()
        } else {
            br.capabilities_affected.join(", ")
        }
    ));
    out.push_str(&format!(
        "Components affected: {}\n",
        if br.components_affected.is_empty() {
            "none".into()
        } else {
            br.components_affected.join(", ")
        }
    ));
    out.push_str(&format!("Files: {}\n", br.files.len()));
    out.push_str(&format!("Symbols: {}\n", br.symbols));
    if !br.cross_domain_edges.is_empty() {
        out.push_str("\nCross-domain edges:\n");
        for (from, to, archetype) in &br.cross_domain_edges {
            out.push_str(&format!("  {} → {} [{}]\n", from, to, archetype));
        }
    }
    if let Some(ref wlnk) = br.weakest_link {
        let kind_str = match &wlnk.kind {
            crate::WeakestLinkKind::UnmatchedArtifact => "UNMATCHED — no code binding".into(),
            crate::WeakestLinkKind::LowConfidenceMatch(c) => format!("LOW CONFIDENCE — {:.2}", c),
            crate::WeakestLinkKind::CrossDomainWithoutTrace => {
                "CROSS-DOMAIN — missing trace on one side".into()
            }
        };
        out.push_str(&format!(
            "\nWeakest link: {} {} — {}",
            wlnk.artifact_kind, wlnk.artifact_id, kind_str
        ));
    }
    Ok(out.trim_end().to_string())
}

fn tool_fpf_search(args: &Value) -> anyhow::Result<String> {
    let query = args["query"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("query is required"))?;
    let limit = args["limit"].as_u64().unwrap_or(5) as usize;
    let full = args["full"].as_bool().unwrap_or(false);

    let hits = crate::fpf::search(query, limit, full)?;
    if hits.is_empty() {
        return Ok(format!("No results for: {}", query));
    }

    let mut out = String::new();
    for (i, h) in hits.iter().enumerate() {
        let breadcrumb = if h.heading_path.is_empty() {
            String::new()
        } else {
            format!(" — {}", h.heading_path)
        };
        out.push_str(&format!("### {}. {}{}\n\n{}\n\n", i + 1, h.heading, breadcrumb, h.content));
    }
    Ok(out.trim_end().to_string())
}

fn tool_fpf_sync(args: &Value) -> anyhow::Result<String> {
    let force = args["force"].as_bool().unwrap_or(false);
    let embed = args["embed"].as_bool().unwrap_or(false);
    crate::fpf::cmd_sync(force, embed)?;
    Ok("FPF index synced successfully".to_string())
}

// ── S5D lifecycle helpers ─────────────────────────────────────────────────────

fn validate_spec_path(spec_arg: &str) -> anyhow::Result<()> {
    // Prevent path traversal and arbitrary file reads
    if spec_arg.contains("..") {
        anyhow::bail!("path traversal not allowed: {}", spec_arg);
    }
    if !spec_arg.ends_with(".s5d.yaml") && !spec_arg.ends_with(".s5d.yml") {
        anyhow::bail!(
            "spec path must end with .s5d.yaml or .s5d.yml, got: {}",
            spec_arg
        );
    }
    Ok(())
}

fn load_spec_context_mcp(
    spec_arg: &str,
) -> anyhow::Result<(crate::S5dProject, std::path::PathBuf, crate::Spec, String)> {
    validate_spec_path(spec_arg)?;
    let path = std::path::Path::new(spec_arg);
    if !path.exists() {
        anyhow::bail!("file not found: {}", spec_arg);
    }
    let content = std::fs::read_to_string(path)?;
    let spec: crate::Spec =
        serde_yaml::from_str(&content).map_err(|e| anyhow::anyhow!("YAML parse error: {}", e))?;
    let abs_path = path.canonicalize()?;
    let project = crate::S5dProject::find(&abs_path)
        .ok_or_else(|| anyhow::anyhow!("no .s5d/ found — is this file inside an s5d project?"))?;
    let spec_filename = abs_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("cannot determine filename"))?
        .to_string_lossy()
        .into_owned();
    Ok((project, abs_path, spec, spec_filename))
}

fn load_spec_yaml_mcp(spec_path: &str) -> anyhow::Result<(std::path::PathBuf, crate::Spec)> {
    validate_spec_path(spec_path)?;
    let path = std::path::Path::new(spec_path);
    if !path.exists() {
        anyhow::bail!("spec not found: {}", spec_path);
    }
    let content = std::fs::read_to_string(path)?;
    let spec: crate::Spec =
        serde_yaml::from_str(&content).map_err(|e| anyhow::anyhow!("YAML parse error: {}", e))?;
    Ok((path.to_path_buf(), spec))
}

fn save_spec_yaml_mcp(path: &std::path::Path, spec: &crate::Spec) -> anyhow::Result<()> {
    let yaml = serde_yaml::to_string(spec)?;
    std::fs::write(path, yaml)?;
    Ok(())
}

fn slugify_mcp(title: &str) -> String {
    let mut out = String::new();
    for c in title.to_lowercase().chars() {
        if c.is_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

// ── s5d_init ──────────────────────────────────────────────────────────────────

fn tool_s5d_init(_args: &Value) -> anyhow::Result<String> {
    let cwd = std::env::current_dir()?;
    let (project, report) = crate::S5dProject::init(&cwd)?;
    if report.dirs_created.is_empty() && report.files_created.is_empty() {
        Ok(format!(
            "S5D already initialized at: {}",
            project.s5d_dir().display()
        ))
    } else {
        let mut out = format!(
            "S5D initialized at: {}\n",
            project.s5d_dir().display()
        );
        for d in &report.dirs_created {
            out.push_str(&format!("  dir: {}\n", d.display()));
        }
        for f in &report.files_created {
            out.push_str(&format!("  file: {}\n", f.display()));
        }
        Ok(out.trim_end().to_string())
    }
}

// ── s5d_new ───────────────────────────────────────────────────────────────────

fn tool_s5d_new(args: &Value) -> anyhow::Result<String> {
    let id = args["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: id"))?;
    crate::sanitize_id(id)?;

    let tier_str = args["tier"].as_str().unwrap_or("standard");
    let product = args["product"].as_str();
    let question = args["question"].as_str();
    let rationale = args["rationale"].as_str();

    let cwd = std::env::current_dir()?;
    let project = crate::S5dProject::find(&cwd)
        .ok_or_else(|| anyhow::anyhow!("no .s5d/ found (run s5d_init first)"))?;

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
        crate::generate_note_spec(id, product_name, title, r)
    } else if tier_str == "decision" {
        let q = question.unwrap_or(id);
        crate::generate_decision_spec(id, product_name, q)
    } else {
        let tier = match tier_str {
            "lightweight" => crate::Tier::Lightweight,
            "standard" => crate::Tier::Standard,
            "high" => crate::Tier::High,
            _ => anyhow::bail!(
                "invalid tier: {} (use lightweight|standard|high|decision|note)",
                tier_str
            ),
        };
        crate::generate_spec(id, tier, product_name)
    };

    let yaml = serde_yaml::to_string(&spec)?;
    std::fs::write(&spec_path, &yaml)?;

    let sha = crate::S5dProject::file_sha256(&spec_path)?;
    let record = crate::generate_record(&spec_filename, &sha);
    project.save_record(&spec_filename, &record)?;

    Ok(format!(
        "Created spec: {}\nRecord: {}",
        spec_path.display(),
        project
            .s5d_dir()
            .join("records")
            .join(spec_filename.replace(".s5d.yaml", ".record.yaml"))
            .display()
    ))
}

// ── s5d_validate ──────────────────────────────────────────────────────────────

fn tool_s5d_validate(args: &Value) -> anyhow::Result<String> {
    let spec_path = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let path = std::path::Path::new(spec_path);
    if !path.exists() {
        anyhow::bail!("file not found: {}", spec_path);
    }
    let content = std::fs::read_to_string(path)?;
    let spec: crate::Spec =
        serde_yaml::from_str(&content).map_err(|e| anyhow::anyhow!("YAML parse error: {}", e))?;

    let errors = crate::validate_spec(&spec);
    if errors.is_empty() {
        Ok(format!("{} is valid", spec_path))
    } else {
        let msg = errors.join("\n");
        anyhow::bail!("Validation errors:\n{}", msg)
    }
}

// ── s5d_graph_check ───────────────────────────────────────────────────────────

fn tool_s5d_graph_check(args: &Value) -> anyhow::Result<String> {
    let spec_path = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let (_project, _abs, spec, _filename) = load_spec_context_mcp(spec_path)?;

    let errors = crate::graph_check(&spec);
    if errors.is_empty() {
        Ok(format!("{} graph ok", spec_path))
    } else {
        let msg = errors.join("\n");
        anyhow::bail!("Graph errors:\n{}", msg)
    }
}

// ── s5d_preview ───────────────────────────────────────────────────────────────

fn tool_s5d_preview(args: &Value) -> anyhow::Result<String> {
    let spec_arg = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let (project, spec_path, spec, spec_filename) = load_spec_context_mcp(spec_arg)?;

    let s5d_dir = project.s5d_dir();
    let mut aliases = crate::AliasTable::load(&s5d_dir)?;
    if let Some(ref meta) = spec.meta {
        aliases.apply_renames(&spec.id, &meta.renames);
    }
    let actions = crate::compute_diff(&spec, &mut aliases);
    let diff_sha = actions.sha256();
    let counts = actions.counts();

    let mut out = format!("Preview: {}\n", spec.id);
    out.push_str(&format!(
        "  create: {}  link: {}  update: {}  delete: {}\n",
        counts.create, counts.link, counts.update, counts.delete
    ));
    out.push_str(&format!("  diff_sha256: {}\n", diff_sha));

    if !actions.create.is_empty() {
        out.push_str("\nNew artifacts:\n");
        for item in &actions.create {
            out.push_str(&format!("  + {}\n", item));
        }
    }
    if !actions.update.is_empty() {
        out.push_str("\nExisting artifacts:\n");
        for item in &actions.update {
            out.push_str(&format!("  ~ {}\n", item));
        }
    }
    if !actions.link.is_empty() {
        out.push_str("\nLinks:\n");
        for item in &actions.link {
            out.push_str(&format!("  → {}\n", item));
        }
    }

    let spec_sha = crate::S5dProject::file_sha256(&spec_path)?;
    let mut record = project
        .load_record(&spec_filename)?
        .unwrap_or_else(|| crate::generate_record(&spec_filename, &spec_sha));
    record.preview = Some(crate::PreviewResult {
        diff_sha256: diff_sha,
        actions: counts,
        log: None,
    });
    record.status = crate::SpecStatus::Previewed;
    record.status_history.push(crate::StatusEntry {
        status: crate::SpecStatus::Previewed,
        timestamp: chrono::Utc::now().to_rfc3339(),
    });
    project.save_record(&spec_filename, &record)?;

    Ok(out.trim_end().to_string())
}

// ── s5d_approve ───────────────────────────────────────────────────────────────

fn tool_s5d_approve(args: &Value) -> anyhow::Result<String> {
    let spec_arg = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let reviewer = args["reviewer"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: reviewer"))?;

    let (project, spec_path, _spec, spec_filename) = load_spec_context_mcp(spec_arg)?;

    let spec_sha = crate::S5dProject::file_sha256(&spec_path)?;
    let mut record = project
        .load_record(&spec_filename)?
        .ok_or_else(|| anyhow::anyhow!("no record found for {} — run s5d_preview first", spec_filename))?;

    if record.status != crate::SpecStatus::Previewed {
        anyhow::bail!(
            "spec must be in 'previewed' state before approval (current: {})",
            record.status
        );
    }

    let diff_sha = record
        .preview
        .as_ref()
        .map(|p| p.diff_sha256.clone())
        .ok_or_else(|| anyhow::anyhow!("no preview result on record — run s5d_preview first"))?;

    record.approvals.push(crate::Approval {
        reviewer: reviewer.into(),
        date: chrono::Utc::now().to_rfc3339(),
        spec_sha256: spec_sha,
        diff_sha256: diff_sha.clone(),
    });
    record.status = crate::SpecStatus::Approved;
    record.status_history.push(crate::StatusEntry {
        status: crate::SpecStatus::Approved,
        timestamp: chrono::Utc::now().to_rfc3339(),
    });
    project.save_record(&spec_filename, &record)?;

    Ok(format!(
        "Approved: {} (reviewer: {})\n  diff_sha256: {}",
        spec_filename, reviewer, diff_sha
    ))
}

// ── s5d_run_gates ─────────────────────────────────────────────────────────────

fn tool_s5d_run_gates(args: &Value) -> anyhow::Result<String> {
    let spec_arg = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let (project, _spec_path, spec, spec_filename) = load_spec_context_mcp(spec_arg)?;

    let config = project.load_config()?;
    let results = crate::run_gates(&spec, &config, spec_arg, &project.root, &project.s5d_dir())?;

    let passed = results.iter().filter(|r| r.status == "passed").count();
    let failed = results.iter().filter(|r| r.status == "failed").count();
    let skipped = results.iter().filter(|r| r.status == "skipped").count();

    let mut out = String::new();
    for r in &results {
        let marker = match r.status.as_str() {
            "passed" => "pass",
            "failed" => "fail",
            _ => "skip",
        };
        out.push_str(&format!("  [{}] gate:{} (attempt {})\n", marker, r.kind, r.attempt));
    }
    out.push_str(&format!(
        "passed: {}  failed: {}  skipped: {}",
        passed, failed, skipped
    ));

    let mut record = project
        .load_record(&spec_filename)?
        .ok_or_else(|| anyhow::anyhow!("no record found for {}", spec_filename))?;
    record.gate_results.extend(results);
    project.save_record(&spec_filename, &record)?;

    if failed > 0 {
        anyhow::bail!("{}", out);
    }
    Ok(out.trim_end().to_string())
}

// ── s5d_import ────────────────────────────────────────────────────────────────

fn tool_s5d_import(args: &Value) -> anyhow::Result<String> {
    let spec_arg = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let verified_by: Option<String> = args["verified_by"].as_str().map(|s| s.to_string());
    let force = args["force"].as_bool().unwrap_or(false);

    let (project, spec_path, spec, spec_filename) = load_spec_context_mcp(spec_arg)?;

    let mut record = project.load_record(&spec_filename)?.ok_or_else(|| {
        anyhow::anyhow!(
            "no record found for {} — run s5d_preview and s5d_approve first",
            spec_filename
        )
    })?;

    if record.status != crate::SpecStatus::Approved {
        anyhow::bail!(
            "spec must be approved before import (current: {})",
            record.status
        );
    }

    let import_checks = crate::check_import(&Some(record.clone()), &verified_by);
    crate::enforce_checks(&import_checks, force)?;

    if !spec.gates.is_empty() {
        let all_latest_passed = spec.gates.iter().all(|g| {
            record
                .gate_results
                .iter()
                .rev()
                .find(|r| r.kind == g.kind)
                .is_some_and(|r| r.status == "passed" || r.status == "waived")
        });
        if !all_latest_passed {
            anyhow::bail!("all declared gates must pass or be waived before import — run s5d_run_gates or s5d_waiver first");
        }
    }

    let approval = record
        .approvals
        .last()
        .ok_or_else(|| anyhow::anyhow!("no approval found on record"))?;

    let current_sha = crate::S5dProject::file_sha256(&spec_path)?;
    if current_sha != approval.spec_sha256 {
        anyhow::bail!(
            "spec file has changed since approval\n  approved: {}\n  current:  {}",
            approval.spec_sha256,
            current_sha
        );
    }

    let s5d_dir = project.s5d_dir();
    let mut aliases_check = crate::AliasTable::load(&s5d_dir)?;
    if let Some(ref meta) = spec.meta {
        aliases_check.apply_renames(&spec.id, &meta.renames);
    }
    let fresh_actions = crate::compute_diff(&spec, &mut aliases_check);
    let fresh_diff_sha = fresh_actions.sha256();

    if fresh_diff_sha != approval.diff_sha256 {
        anyhow::bail!(
            "diff has changed since approval — re-run s5d_preview and s5d_approve\n  approved: {}\n  current:  {}",
            approval.diff_sha256,
            fresh_diff_sha
        );
    }

    if verified_by.is_some() {
        record.verified_by = verified_by.clone();
        project.save_record(&spec_filename, &record)?;
    }

    let (actions, fingerprint) = crate::execute_import(&project, &spec_path, &spec, &spec_filename)?;
    let counts = actions.counts();

    Ok(format!(
        "Imported: {}\n  create: {}  link: {}  update: {}  delete: {}\n  state_fingerprint: {}",
        spec.id, counts.create, counts.link, counts.update, counts.delete, fingerprint
    ))
}

// ── s5d_decide ────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn tool_s5d_decide(args: &Value) -> anyhow::Result<String> {
    let spec_arg = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let title = args["title"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: title"))?;
    let winner = args["winner"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: winner"))?;
    let confirmed_by = args["confirmed_by"].as_str();
    let context = args["context"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: context"))?;
    let decision = args["decision"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: decision"))?;
    let rationale = args["rationale"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: rationale"))?;
    let consequences = args["consequences"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: consequences"))?;
    let rejected = args["rejected"].as_str();

    let (path, spec) = load_spec_yaml_mcp(spec_arg)?;

    // Load record — decision state lives in record, not spec
    let record_path = path.with_file_name(
        path.file_name().unwrap().to_string_lossy().replace(".s5d.yaml", ".record.yaml"),
    );
    let record: Option<crate::Record> = if record_path.exists() {
        serde_yaml::from_str(&std::fs::read_to_string(&record_path)?).ok()
    } else {
        None
    };

    let checks = crate::check_decide(&spec, &record, &confirmed_by.map(|s| s.to_string()));
    crate::enforce_checks(&checks, false)?;

    if !matches!(spec.tier, crate::Tier::Decision) {
        anyhow::bail!(
            "decide only works on decision-tier specs (this spec is {})",
            spec.tier
        );
    }

    spec.hypotheses
        .iter()
        .find(|h| h.id == winner)
        .ok_or_else(|| anyhow::anyhow!("winner hypothesis not found: {}", winner))?;

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
    let decision_record = crate::DecisionRecord {
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

    let abs_path = path.canonicalize()?;
    let project = crate::S5dProject::find(&abs_path)
        .ok_or_else(|| anyhow::anyhow!("no .s5d/ found — is this file inside an s5d project?"))?;
    let spec_filename = abs_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("cannot determine filename"))?
        .to_string_lossy()
        .into_owned();

    let spec_sha = crate::S5dProject::file_sha256(&abs_path)?;
    let mut record = project
        .load_record(&spec_filename)?
        .unwrap_or_else(|| crate::generate_record(&spec_filename, &spec_sha));
    record.decision = Some(decision_record);
    project.save_record(&spec_filename, &record)?;

    Ok(format!(
        "Decision recorded: {} (winner: {})",
        title, winner
    ))
}

// ── s5d_add_hypothesis ────────────────────────────────────────────────────────

fn tool_s5d_add_hypothesis(args: &Value) -> anyhow::Result<String> {
    let spec_path = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let title = args["title"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: title"))?;
    let content = args["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: content"))?;
    let scope = args["scope"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: scope"))?;
    let kind = args["kind"].as_str().unwrap_or("system");

    let (path, mut spec) = load_spec_yaml_mcp(spec_path)?;

    if !matches!(spec.tier, crate::Tier::Decision) {
        anyhow::bail!(
            "add_hypothesis only works on decision-tier specs (this spec is {})",
            spec.tier
        );
    }

    let hyp_id = slugify_mcp(title);

    if spec.hypotheses.iter().any(|h| h.id == hyp_id) {
        anyhow::bail!("hypothesis '{}' already exists in this spec", hyp_id);
    }

    let hyp = crate::Hypothesis {
        id: hyp_id.clone(),
        title: title.into(),
        content: content.into(),
        scope: scope.into(),
        kind: kind.into(),
        layer: "L0".into(),
        r_eff: None,
        evidence: vec![],
        depends_on: vec![],
        rationale: None,
        spec_ref: None,
    };

    spec.hypotheses.push(hyp);
    save_spec_yaml_mcp(&path, &spec)?;

    Ok(format!("Added hypothesis: {} (L0)", hyp_id))
}

// ── s5d_add_evidence ──────────────────────────────────────────────────────────

fn tool_s5d_add_evidence(args: &Value) -> anyhow::Result<String> {
    let spec_path = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let hypothesis_id = args["hypothesis_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: hypothesis_id"))?;
    let evidence_type = args["evidence_type"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: evidence_type"))?;
    let content = args["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: content"))?;
    let verdict = args["verdict"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: verdict"))?;
    let formality = args["formality"].as_u64().map(|v| v as u8);
    let claim_scope = args["claim_scope"].as_str().map(|s| s.to_string());
    let reliability = args["reliability"].as_f64();

    if let Some(f) = formality {
        if !(1..=5).contains(&f) {
            anyhow::bail!("formality must be between 1 and 5");
        }
    }
    if let Some(r) = reliability {
        if !(0.0..=1.0).contains(&r) {
            anyhow::bail!("reliability must be between 0.0 and 1.0");
        }
    }
    crate::sanitize_id(evidence_type)?;

    let (path, mut spec) = load_spec_yaml_mcp(spec_path)?;

    if !matches!(spec.tier, crate::Tier::Decision) {
        anyhow::bail!(
            "add_evidence only works on decision-tier specs (this spec is {})",
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

    let ev = crate::HypothesisEvidence {
        id: ev_id,
        evidence_type: evidence_type.into(),
        content: content.into(),
        verdict: verdict.into(),
        valid_until: Some(valid_until),
        carrier_ref: None,
        formality,
        claim_scope: claim_scope
            .map(|s| s.split(',').map(|v| v.trim().to_string()).collect())
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

    save_spec_yaml_mcp(&path, &spec)?;

    Ok(format!(
        "Added evidence to {}: {} → layer {}",
        hypothesis_id, verdict, new_layer
    ))
}

// ── s5d_reflect ───────────────────────────────────────────────────────────────

fn tool_s5d_reflect(args: &Value) -> anyhow::Result<String> {
    let spec_arg = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let summary = args["summary"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: summary"))?;
    let heuristic = args["heuristic"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: heuristic"))?;
    let issues_str = args["issues"].as_str().unwrap_or("");

    let (project, _spec_path, _spec, spec_filename) = load_spec_context_mcp(spec_arg)?;

    let mut record = project.load_record(&spec_filename)?.ok_or_else(|| {
        anyhow::anyhow!("no record found for {} — run s5d_preview first", spec_filename)
    })?;

    let issues_list: Vec<String> = issues_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let heuristics = vec![heuristic.to_string()];

    record.reflection = Some(crate::Reflection {
        summary: Some(summary.to_string()),
        worked: vec![],
        issues: issues_list,
        structured_issues: vec![],
        follow_ups: vec![],
        evidence: vec![],
        heuristics,
    });

    record.status = crate::SpecStatus::Operated;
    record.status_history.push(crate::StatusEntry {
        status: crate::SpecStatus::Operated,
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    project.save_record(&spec_filename, &record)?;

    Ok("Lifecycle closed → operated. Reflect recorded.".to_string())
}

// ── s5d_status ────────────────────────────────────────────────────────────────

fn tool_s5d_status(_args: &Value) -> anyhow::Result<String> {
    let cwd = std::env::current_dir()?;
    let project =
        crate::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;

    let specs = project.discover_specs()?;
    if specs.is_empty() {
        return Ok("No specs. Run s5d_new to create one.".to_string());
    }

    let mut out = format!(
        "S5D Specs ({})\n",
        project.root.display()
    );
    out.push_str(&format!(
        "{:<30} {:<12} {:<10} {:<10} Sync\n",
        "ID", "Version", "Tier", "Status"
    ));
    out.push_str(&format!("{}\n", "─".repeat(76)));

    for (path, spec) in &specs {
        let filename = path.file_name().unwrap().to_string_lossy();
        let record = project.load_record(&filename)?;
        let (status, sync) = match &record {
            Some(r) => (format!("{}", r.status), format!("{}", r.sync_status)),
            None => ("?".into(), "?".into()),
        };
        out.push_str(&format!(
            "{:<30} {:<12} {:<10} {:<10} {}\n",
            spec.id, spec.version, format!("{}", spec.tier), status, sync
        ));
    }

    Ok(out.trim_end().to_string())
}

// ── s5d_show ──────────────────────────────────────────────────────────────────

fn tool_s5d_show(args: &Value) -> anyhow::Result<String> {
    let spec_path = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let (_path, spec) = load_spec_yaml_mcp(spec_path)?;

    let is_decision = matches!(spec.tier, crate::Tier::Decision);
    let tier_label = format!("{}", spec.tier).to_uppercase();
    let title = spec
        .decision
        .as_ref()
        .map(|d| d.title.as_str())
        .unwrap_or(&spec.id);

    let mut out = format!("{}: {}\n", tier_label, title);
    out.push_str(&format!("{}\n", "━".repeat(60)));

    if is_decision {
        if let Some(ref problem) = spec.problem {
            out.push_str(&format!("  Signal: {}\n", problem.signal()));
        }
        if let Some(ref dec) = spec.decision {
            if !dec.context.is_empty() {
                out.push_str(&format!("  Context: {}\n", dec.context));
            }
        }

        let winner_id = spec
            .decision
            .as_ref()
            .map(|d| d.winner_id.as_str())
            .unwrap_or("");

        for hyp in &spec.hypotheses {
            let is_winner = hyp.id == winner_id;
            let is_invalid = hyp.layer == "invalid";
            let status = if is_winner {
                format!("WINNER [{}]", hyp.layer)
            } else if is_invalid {
                "ELIMINATED".into()
            } else {
                format!("[{}]", hyp.layer)
            };
            out.push_str(&format!("  {} {} — {}\n", hyp.id, hyp.title, status));
            for ev in &hyp.evidence {
                let content_short = ev.content.lines().next().unwrap_or("");
                let content_display = if content_short.len() > 72 {
                    format!("{}…", &content_short[..72])
                } else {
                    content_short.to_string()
                };
                out.push_str(&format!(
                    "    {} {}: {}  {}\n",
                    ev.evidence_type, ev.id, content_display, ev.verdict
                ));
            }
        }

        if let Some(ref dec) = spec.decision {
            out.push_str(&format!("\n  Decision: {}\n", dec.decision));
            if !dec.consequences.is_empty() {
                out.push_str(&format!("  Consequences: {}\n", dec.consequences));
            }
            if let Some(ref ts) = dec.decided_at {
                let date = ts.split('T').next().unwrap_or(ts);
                out.push_str(&format!("  Decided: {}\n", date));
            }
        }
    } else {
        out.push_str(&format!("  Tier: {}\n", spec.tier));
        out.push_str(&format!("  Product: {}\n", spec.product));
        if let Some(ref artifacts) = spec.artifacts {
            out.push_str(&format!(
                "  Artifacts: {} domains, {} capabilities, {} entities, {} components\n",
                artifacts.domains.len(),
                artifacts.capabilities.len(),
                artifacts.entities.len(),
                artifacts.components.len()
            ));
        }
        if !spec.gates.is_empty() {
            out.push_str(&format!("  Gates: {}\n", spec.gates.len()));
        }
    }

    Ok(out.trim_end().to_string())
}

// ── s5d_waiver ────────────────────────────────────────────────────────────────

fn tool_s5d_waiver(args: &Value) -> anyhow::Result<String> {
    let spec_arg = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let gate = args["gate"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: gate"))?;
    let reason = args["reason"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: reason"))?;
    let condition = args["condition"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: condition"))?;
    let approved_by = args["approved_by"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: approved_by"))?;

    let (project, _spec_path, spec, spec_filename) = load_spec_context_mcp(spec_arg)?;

    // Check that the gate kind exists in the spec
    let gate_exists = spec.gates.iter().any(|g| g.kind == gate);
    if !gate_exists {
        anyhow::bail!(
            "No gate kind '{}' declared in spec. Declared gates: {}",
            gate,
            spec.gates.iter().map(|g| g.kind.as_str()).collect::<Vec<_>>().join(", ")
        );
    }

    // Record waiver as a GateResult with status "waived" in the record file
    let mut record = project.load_record(&spec_filename)?.ok_or_else(|| {
        anyhow::anyhow!("no record found for spec — run s5d_preview and s5d_approve first")
    })?;

    record.gate_results.push(crate::GateResult {
        kind: gate.to_string(),
        status: "waived".to_string(),
        attempt: record.gate_results.iter().filter(|r| r.kind == gate).count() as u32 + 1,
        timestamp: chrono::Utc::now().to_rfc3339(),
        log: Some(format!(
            "WAIVER — reason: {}. Condition to lift: {}. Approved by: {}.",
            reason, condition, approved_by
        )),
        exit_code: None,
        evidence_path: None,
        command: None,
        duration_ms: None,
    });

    project.save_record(&spec_filename, &record)?;

    Ok(format!(
        "Gate '{}' waived — approved by: {}, reason: {}, condition to lift: {}",
        gate, approved_by, reason, condition
    ))
}

// ── s5d_rollback ──────────────────────────────────────────────────────────────

fn tool_s5d_rollback(args: &Value) -> anyhow::Result<String> {
    let spec_arg = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let (project, _spec_path, spec, spec_filename) = load_spec_context_mcp(spec_arg)?;

    let s5d_dir = project.s5d_dir();
    let mut ledger = project.load_ledger()?;

    let has_import = ledger
        .entries
        .iter()
        .any(|e| e.package_id == spec.id && e.action == "import" && e.status == "success");

    if !has_import {
        anyhow::bail!("no successful import found for {} to roll back", spec.id);
    }

    let mut aliases = crate::AliasTable::load(&s5d_dir)?;
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

    ledger.entries.push(crate::LedgerEntry {
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

    let mut record = project
        .load_record(&spec_filename)?
        .ok_or_else(|| anyhow::anyhow!("no record found for {}", spec_filename))?;
    record.status = crate::SpecStatus::Deprecated;
    record.sync_status = crate::SyncStatus::Unknown;
    record.status_history.push(crate::StatusEntry {
        status: crate::SpecStatus::Deprecated,
        timestamp: chrono::Utc::now().to_rfc3339(),
    });
    project.save_record(&spec_filename, &record)?;

    let mut index = project.load_index()?;
    index.features.retain(|e| e.id != spec.id);
    project.save_index(&index)?;

    Ok(format!("Rolled back: {}", spec.id))
}

// ── s5d_drift_check ───────────────────────────────────────────────────────────

fn tool_s5d_drift_check(args: &Value) -> anyhow::Result<String> {
    let spec_opt = args["spec"].as_str();
    let mut out = String::new();

    if let Some(spec_arg) = spec_opt {
        let (project, _spec_path, spec, spec_filename) = load_spec_context_mcp(spec_arg)?;
        let result = crate::check_drift(&project, &spec, &spec_filename)?;
        match result {
            crate::DriftResult::Synced => {
                out.push_str(&format!("{} — synced\n", spec.id));
            }
            crate::DriftResult::Drifted { expected, actual } => {
                out.push_str(&format!(
                    "{} — drifted\n  expected: {}\n  actual:   {}\n",
                    spec.id, expected, actual
                ));
            }
            crate::DriftResult::Degraded { reason } => {
                out.push_str(&format!("{} — degraded: {}\n", spec.id, reason));
            }
        }
    } else {
        let cwd = std::env::current_dir()?;
        let project =
            crate::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;
        let specs = project.discover_specs()?;

        for (path, spec) in &specs {
            let filename = path.file_name().unwrap().to_string_lossy();
            let record = project.load_record(&filename)?;
            let is_applied = record
                .as_ref()
                .is_some_and(|r| r.status == crate::SpecStatus::Applied);
            if !is_applied {
                continue;
            }
            let result = crate::check_drift(&project, spec, &filename)?;
            match result {
                crate::DriftResult::Synced => {
                    out.push_str(&format!("{} — synced\n", spec.id));
                }
                crate::DriftResult::Drifted { expected, actual } => {
                    out.push_str(&format!(
                        "{} — drifted\n  expected: {}\n  actual:   {}\n",
                        spec.id, expected, actual
                    ));
                }
                crate::DriftResult::Degraded { reason } => {
                    out.push_str(&format!("{} — degraded: {}\n", spec.id, reason));
                }
            }
        }
    }

    if out.is_empty() {
        out.push_str("No applied specs to check.");
    }
    Ok(out.trim_end().to_string())
}

// ── s5d_reconcile ─────────────────────────────────────────────────────────────

fn tool_s5d_reconcile(args: &Value) -> anyhow::Result<String> {
    let spec_opt = args["spec"].as_str();
    let mut out = String::new();

    if let Some(spec_arg) = spec_opt {
        let (project, spec_path, spec, spec_filename) = load_spec_context_mcp(spec_arg)?;
        let (actions, fingerprint) = crate::reconcile(&project, &spec, &spec_path, &spec_filename)?;
        let counts = actions.counts();
        out.push_str(&format!(
            "Reconciled: {}\n  create: {}  link: {}  update: {}  delete: {}\n  state_fingerprint: {}",
            spec.id, counts.create, counts.link, counts.update, counts.delete, fingerprint
        ));
    } else {
        let cwd = std::env::current_dir()?;
        let project =
            crate::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;
        let specs = project.discover_specs()?;
        let mut reconciled = 0usize;

        for (path, spec) in &specs {
            let filename = path.file_name().unwrap().to_string_lossy();
            let record = project.load_record(&filename)?;
            let is_applied = record
                .as_ref()
                .is_some_and(|r| r.status == crate::SpecStatus::Applied);
            if !is_applied {
                continue;
            }
            let drift = crate::check_drift(&project, spec, &filename)?;
            let needs_reconcile = matches!(
                drift,
                crate::DriftResult::Drifted { .. } | crate::DriftResult::Degraded { .. }
            );
            if !needs_reconcile {
                continue;
            }
            match crate::reconcile(&project, spec, path, &filename) {
                Ok((actions, fingerprint)) => {
                    let counts = actions.counts();
                    out.push_str(&format!(
                        "Reconciled: {}\n  create: {}  link: {}  update: {}  delete: {}\n  state_fingerprint: {}\n",
                        spec.id, counts.create, counts.link, counts.update, counts.delete, fingerprint
                    ));
                    reconciled += 1;
                }
                Err(e) => {
                    out.push_str(&format!("reconcile failed for {}: {}\n", spec.id, e));
                }
            }
        }
        out.push_str(&format!("{} spec(s) reconciled", reconciled));
    }

    Ok(out.trim_end().to_string())
}

// ── s5d_check ─────────────────────────────────────────────────────────────────

fn tool_s5d_check(args: &Value) -> anyhow::Result<String> {
    let spec_arg = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let threshold = args["threshold"].as_u64().unwrap_or(0) as u8;

    let (project, _spec_path, spec, spec_filename) = load_spec_context_mcp(spec_arg)?;

    let report = crate::compute_health_report(&spec);
    let spec_id = spec_filename.replace(".s5d.yaml", "");
    let _ = crate::save_snapshot(&project.s5d_dir(), &spec_id, &report);

    let pass = report.aggregate_score >= threshold;
    let mut out = format!(
        "{} {} health score: {}\n",
        if pass { "ok" } else { "FAIL" },
        spec_arg,
        report.aggregate_score
    );

    if !report.cycles.is_empty() {
        out.push_str("Cycles:\n");
        for scc in &report.cycles {
            out.push_str(&format!("  {} {}\n", "●", scc.join(" → ")));
        }
    }

    if !report.violations.is_empty() {
        out.push_str("Violations:\n");
        for v in &report.violations {
            out.push_str(&format!("  ● [{}] {} (−{})\n", v.kind, v.message, v.penalty));
        }
    }

    out.push_str(&format!(
        "\n{:<40} {:>4} {:>4} {:>6} {:>6}\n",
        "Domain", "Ca", "Ce", "I", "Score"
    ));
    out.push_str(&format!("{}\n", "─".repeat(64)));
    for dm in &report.domain_metrics {
        out.push_str(&format!(
            "{:<40} {:>4} {:>4} {:>6.2} {:>6}\n",
            dm.domain_id, dm.ca, dm.ce, dm.instability, dm.health_score
        ));
    }

    if !pass {
        anyhow::bail!(
            "{}Health score {} below threshold {}",
            out,
            report.aggregate_score,
            threshold
        );
    }

    Ok(out.trim_end().to_string())
}

// ── s5d_metrics ───────────────────────────────────────────────────────────────

fn tool_s5d_metrics(args: &Value) -> anyhow::Result<String> {
    let spec_arg = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let (_project, _spec_path, spec, _spec_filename) = load_spec_context_mcp(spec_arg)?;

    let report = crate::compute_health_report(&spec);

    let mut out = format!(
        "{:<40} {:>4} {:>4} {:>6} {:>6} Violations\n",
        "Domain", "Ca", "Ce", "I", "Score"
    );
    out.push_str(&format!("{}\n", "─".repeat(80)));
    for dm in &report.domain_metrics {
        let violations_str = if dm.violations.is_empty() {
            "—".into()
        } else {
            dm.violations.join(", ")
        };
        out.push_str(&format!(
            "{:<40} {:>4} {:>4} {:>6.2} {:>6} {}\n",
            dm.domain_id, dm.ca, dm.ce, dm.instability, dm.health_score, violations_str
        ));
    }
    out.push_str(&format!(
        "\nAggregate health score: {}",
        report.aggregate_score
    ));

    Ok(out.trim_end().to_string())
}
