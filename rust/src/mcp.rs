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
        "serverInfo": {"name": "s5d", "version": env!("CARGO_PKG_VERSION")}
    }))
}

fn handle_tools_list() -> anyhow::Result<Value> {
    Ok(json!({ "tools": core_tools() }))
}

fn core_tools() -> Vec<Value> {
    vec![
        json!({
            "name": "s5d_init",
            "description": "Initialize .s5d/ directory structure and register MCP server for agent tools",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "claude": {"type": "boolean", "description": "Register for Claude Code (.mcp.json) — default if no flags"},
                    "cursor": {"type": "boolean", "description": "Register for Cursor (.cursor/mcp.json)"},
                    "codex": {"type": "boolean", "description": "Register for Codex CLI (.codex/config.toml)"},
                    "gemini": {"type": "boolean", "description": "Register for Gemini CLI (.gemini/settings.json)"},
                    "all": {"type": "boolean", "description": "Register for all supported tools"}
                }
            }
        }),
        json!({
            "name": "s5d_new",
            "description": "Create a new spec from template",
            "inputSchema": {
                "type": "object",
                "required": ["id"],
                "properties": {
                    "id": {"type": "string", "description": "Feature ID (e.g. feat.orders.tracking)"},
                    "tier": {"type": "string", "description": "Spec tier: decision|standard|lightweight|high|note (default: standard)"},
                    "product": {"type": "string", "description": "Product name"},
                    "question": {"type": "string", "description": "Decision question (required for tier=decision)"},
                    "rationale": {"type": "string", "description": "Note rationale (required for tier=note)"}
                }
            }
        }),
        json!({
            "name": "s5d_note",
            "description": "Quick note — one-shot shorthand that creates a note-tier spec from text",
            "inputSchema": {
                "type": "object",
                "required": ["text"],
                "properties": {
                    "text": {"type": "string", "description": "Note text (used as title and rationale)"},
                    "product": {"type": "string", "description": "Product name (optional)"}
                }
            }
        }),
        json!({
            "name": "s5d_validate",
            "description": "Validate a spec file against schema and business rules",
            "inputSchema": {
                "type": "object",
                "required": ["spec"],
                "properties": {
                    "spec": {"type": "string", "description": "Path to .s5d.yaml file"}
                }
            }
        }),
        json!({
            "name": "s5d_graph_check",
            "description": "Check graph structure — DFS cycle detection on spec relations",
            "inputSchema": {
                "type": "object",
                "required": ["spec"],
                "properties": {
                    "spec": {"type": "string", "description": "Path to .s5d.yaml file"}
                }
            }
        }),
        json!({
            "name": "s5d_preview",
            "description": "Dry-run import diff — shows what would be created/updated/linked",
            "inputSchema": {
                "type": "object",
                "required": ["spec"],
                "properties": {
                    "spec": {"type": "string", "description": "Path to .s5d.yaml file"}
                }
            }
        }),
        json!({
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
        }),
        json!({
            "name": "s5d_run_gates",
            "description": "Execute configured gate commands for a spec",
            "inputSchema": {
                "type": "object",
                "required": ["spec"],
                "properties": {
                    "spec": {"type": "string", "description": "Path to .s5d.yaml file"}
                }
            }
        }),
        json!({
            "name": "s5d_import",
            "description": "Transactional import — apply spec to alias table and ledger",
            "inputSchema": {
                "type": "object",
                "required": ["spec", "verified_by"],
                "properties": {
                    "spec": {"type": "string", "description": "Path to .s5d.yaml file"},
                    "verified_by": {"type": "string", "description": "Who independently verified gates passed"},
                    "force": {"type": "boolean", "description": "Override methodological checks (default: false)"}
                }
            }
        }),
        json!({
            "name": "s5d_rollback",
            "description": "Reverse last import for a spec — tombstones alias entries and appends rollback ledger entry",
            "inputSchema": {
                "type": "object",
                "required": ["spec"],
                "properties": {
                    "spec": {"type": "string", "description": "Path to .s5d.yaml file"}
                }
            }
        }),
        json!({
            "name": "s5d_decide",
            "description": "Record a decision in a decision-tier spec. Requires adversarial challenge summary.",
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
                    "rejected": {"type": "string", "description": "Rejected hypothesis IDs (comma-separated)"},
                    "challenge_summary": {"type": "string", "description": "Adversarial challenge outcome summary"},
                    "challenge_mode": {"type": "string", "description": "Challenge mode: tactical (1 probe) or standard (5 probes)"}
                }
            }
        }),
        json!({
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
                    "kind": {"type": "string", "description": "Kind: system (default) or episteme (knowledge/methodology)"},
                    "prompt": {"type": "string", "description": "FPF B.5.2:13.3 prompt — explicit question this hypothesis answers."},
                    "next_move": {"type": "string", "description": "FPF B.5.2:13.3 next downstream move.", "enum": ["deduction", "probe", "build", "defer"]}
                }
            }
        }),
        json!({
            "name": "s5d_add_evidence",
            "description": "Add evidence to a hypothesis in a decision-tier spec",
            "inputSchema": {
                "type": "object",
                "required": ["spec", "hypothesis_id", "evidence_type", "content", "verdict"],
                "properties": {
                    "spec": {"type": "string", "description": "Path to .s5d.yaml file"},
                    "hypothesis_id": {"type": "string", "description": "Hypothesis ID to attach evidence to"},
                    "evidence_type": {"type": "string", "description": "Evidence type: internal, external, gate:test, gate:review, etc. Decision/high tiers require >=1 gate:review verdict=pass before import."},
                    "content": {"type": "string", "description": "Evidence content"},
                    "verdict": {"type": "string", "description": "Verdict: pass, fail, refine"},
                    "formality": {"type": "integer", "description": "Rigor of evidence method (1-5)"},
                    "claim_scope": {"type": "string", "description": "What the claim covers (comma-separated)"},
                    "reliability": {"type": "number", "description": "Confidence that the claim is true (0.0-1.0)"},
                    "refine_kind": {"type": "string", "description": "FPF C.2:4.2 Δ-move kind. Required when verdict=refine.", "enum": ["formalise", "generalise", "specialise", "calibrate", "validate", "congrue"]},
                    "skill": {"type": "string", "description": "Provenance: which cluster skill produced this evidence (e.g. \"security-scan\")."},
                    "agent": {"type": "string", "description": "Provenance: which assess agent produced this evidence (e.g. \"security-scan-assess\")."}
                }
            }
        }),
        json!({
            "name": "s5d_status",
            "description": "Show status of all specs in the current project",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "s5d_show",
            "description": "Show spec details — decision trace, hypothesis tree, or feature summary",
            "inputSchema": {
                "type": "object",
                "required": ["spec"],
                "properties": {
                    "spec": {"type": "string", "description": "Path to .s5d.yaml file"}
                }
            }
        }),
        json!({
            "name": "s5d_trace",
            "description": "Trace a source path to S5D specs, components, capabilities, and decisions",
            "inputSchema": {
                "type": "object",
                "required": ["path"],
                "properties": {
                    "path": {"type": "string", "description": "Source path inside the project"}
                }
            }
        }),
        json!({
            "name": "s5d_drift_check",
            "description": "Compare live state vs last applied fingerprint. Checks all specs if none specified.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "spec": {"type": "string", "description": "Path to .s5d.yaml file (optional — checks all if omitted)"}
                }
            }
        }),
        json!({
            "name": "s5d_reconcile",
            "description": "Re-import to fix drift (desired-state restore). Reconciles all drifted specs if none specified.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "spec": {"type": "string", "description": "Path to .s5d.yaml file (optional — reconciles all drifted if omitted)"}
                }
            }
        }),
        json!({
            "name": "s5d_reflect",
            "description": "Record reflection for a spec (OPERATE stage) — closes lifecycle with production evidence",
            "inputSchema": {
                "type": "object",
                "required": ["spec", "summary", "heuristic"],
                "properties": {
                    "spec": {"type": "string", "description": "Path to .s5d.yaml file"},
                    "verdict": {"type": "string", "description": "Outcome verdict: confirmed, refuted, inconclusive, iterate, kill"},
                    "measurement_window": {"type": "string", "description": "Measurement window used for the verdict"},
                    "summary": {"type": "string", "description": "Summary of what happened in production"},
                    "heuristic": {"type": "string", "description": "Reusable rule learned from this spec"},
                    "issues": {"type": "string", "description": "Issues encountered (comma-separated)"},
                    "telemetry_refs": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Telemetry refs backing the verdict"
                    }
                }
            }
        }),
        json!({
            "name": "s5d_phase_list",
            "description": "List executable work states and the current active state for a spec",
            "inputSchema": {
                "type": "object",
                "required": ["spec"],
                "properties": {
                    "spec": {"type": "string", "description": "Path to .s5d.yaml file"}
                }
            }
        }),
        json!({
            "name": "s5d_phase_start",
            "description": "Mark an executable work state as active",
            "inputSchema": {
                "type": "object",
                "required": ["spec", "phase_id"],
                "properties": {
                    "spec": {"type": "string", "description": "Path to .s5d.yaml file"},
                    "phase_id": {"type": "string", "description": "Work state ID"}
                }
            }
        }),
        json!({
            "name": "s5d_phase_accept",
            "description": "Record human acceptance for an active work state",
            "inputSchema": {
                "type": "object",
                "required": ["spec", "phase_id", "reviewer"],
                "properties": {
                    "spec": {"type": "string", "description": "Path to .s5d.yaml file"},
                    "phase_id": {"type": "string", "description": "Work state ID"},
                    "reviewer": {"type": "string", "description": "Reviewer who accepts the run evidence"}
                }
            }
        }),
        json!({
            "name": "s5d_execute_loop",
            "description": "Emit a bounded Ralph task package for an active work state",
            "inputSchema": {
                "type": "object",
                "required": ["spec", "phase_id"],
                "properties": {
                    "spec": {"type": "string", "description": "Path to .s5d.yaml file"},
                    "phase_id": {"type": "string", "description": "Work state ID"},
                    "engine": {"type": "string", "description": "Execution engine name (default: ralph)"},
                    "mode": {"type": "string", "description": "Optional Ralph run mode: init, bugfix, or generic"}
                }
            }
        }),
        json!({
            "name": "s5d_route",
            "description": "Classify a request into tier, mode, and entry point",
            "inputSchema": {
                "type": "object",
                "required": ["description"],
                "properties": {
                    "description": {"type": "string", "description": "Request description to classify"}
                }
            }
        }),
        json!({
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
                    "approved_by": {"type": "string", "description": "Who approved the waiver"},
                    "expires_at": {"type": "string", "description": "FPF B.3.4:5 — RFC3339 expiry. Auto-revoked at run-gates after this date. Defaults to now+90d if omitted."}
                }
            }
        }),
    ]
}

fn handle_tools_call(params: &Value) -> anyhow::Result<Value> {
    let name = params["name"].as_str().unwrap_or("");
    let args = &params["arguments"];

    let text = match name {
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
        "s5d_phase_list" => tool_s5d_phase_list(args)?,
        "s5d_phase_start" => tool_s5d_phase_start(args)?,
        "s5d_phase_accept" => tool_s5d_phase_accept(args)?,
        "s5d_execute_loop" => tool_s5d_execute_loop(args)?,
        "s5d_status" => tool_s5d_status(args)?,
        "s5d_show" => tool_s5d_show(args)?,
        "s5d_trace" => tool_s5d_trace(args)?,
        "s5d_route" => tool_s5d_route(args)?,
        "s5d_waiver" => tool_s5d_waiver(args)?,
        "s5d_rollback" => tool_s5d_rollback(args)?,
        "s5d_drift_check" => tool_s5d_drift_check(args)?,
        "s5d_reconcile" => tool_s5d_reconcile(args)?,
        "s5d_note" => tool_s5d_note(args)?,
        _ => {
            return Ok(
                json!({"content":[{"type":"text","text":format!("Error: unknown tool '{}'", name)}],"isError":true}),
            )
        }
    };

    Ok(json!({"content":[{"type":"text","text":text}]}))
}

// ── S5D lifecycle helpers ─────────────────────────────────────────────────────

fn validate_spec_path(spec_arg: &str) -> anyhow::Result<()> {
    // Prevent path traversal and arbitrary file reads
    if spec_arg.contains("..") {
        anyhow::bail!("path traversal not allowed: {}", spec_arg);
    }
    if !spec_arg.ends_with(".s5d.yaml") {
        anyhow::bail!("spec path must end with .s5d.yaml, got: {}", spec_arg);
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

fn tool_s5d_init(args: &Value) -> anyhow::Result<String> {
    let cwd = std::env::current_dir()?;
    let (project, report) = crate::S5dProject::init(&cwd)?;

    let mut out = if report.dirs_created.is_empty() && report.files_created.is_empty() {
        format!(
            "S5D already initialized at: {}\n",
            project.s5d_dir().display()
        )
    } else {
        let mut s = format!("S5D initialized at: {}\n", project.s5d_dir().display());
        for d in &report.dirs_created {
            s.push_str(&format!("  dir: {}\n", d.display()));
        }
        for f in &report.files_created {
            s.push_str(&format!("  file: {}\n", f.display()));
        }
        s
    };

    // Multi-tool MCP registration
    let claude = args["claude"].as_bool().unwrap_or(false);
    let cursor = args["cursor"].as_bool().unwrap_or(false);
    let codex = args["codex"].as_bool().unwrap_or(false);
    let gemini = args["gemini"].as_bool().unwrap_or(false);
    let all = args["all"].as_bool().unwrap_or(false);
    let no_flags = !claude && !cursor && !codex && !gemini && !all;

    let binary_str = std::env::current_exe()?.to_string_lossy().into_owned();

    out.push_str("\nMCP registration:\n");

    // Claude Code → .mcp.json
    if claude || all || no_flags {
        match register_mcp_json_mcp(&cwd.join(".mcp.json"), &binary_str) {
            Ok(false) => out.push_str("  Claude (.mcp.json) — already registered\n"),
            Ok(true) => out.push_str("  Claude (.mcp.json) — registered\n"),
            Err(e) => out.push_str(&format!("  Claude: error — {}\n", e)),
        }
    }

    // Cursor → .cursor/mcp.json
    if cursor || all {
        match register_mcp_json_mcp(&cwd.join(".cursor").join("mcp.json"), &binary_str) {
            Ok(false) => out.push_str("  Cursor (.cursor/mcp.json) — already registered\n"),
            Ok(true) => out.push_str("  Cursor (.cursor/mcp.json) — registered\n"),
            Err(e) => out.push_str(&format!("  Cursor: error — {}\n", e)),
        }
    }

    // Codex CLI → .codex/config.toml
    if codex || all {
        match register_mcp_toml_mcp(&cwd.join(".codex").join("config.toml"), &binary_str) {
            Ok(false) => out.push_str("  Codex (.codex/config.toml) — already registered\n"),
            Ok(true) => out.push_str("  Codex (.codex/config.toml) — registered\n"),
            Err(e) => out.push_str(&format!("  Codex: error — {}\n", e)),
        }
    }

    // Gemini CLI → .gemini/settings.json
    if gemini || all {
        match register_mcp_json_mcp(&cwd.join(".gemini").join("settings.json"), &binary_str) {
            Ok(false) => out.push_str("  Gemini (.gemini/settings.json) — already registered\n"),
            Ok(true) => out.push_str("  Gemini (.gemini/settings.json) — registered\n"),
            Err(e) => out.push_str(&format!("  Gemini: error — {}\n", e)),
        }
    }

    Ok(out.trim_end().to_string())
}

/// Register s5d MCP server in a JSON config file (shared by MCP tool handlers).
fn register_mcp_json_mcp(path: &std::path::Path, binary: &str) -> anyhow::Result<bool> {
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

/// Register s5d MCP server in a TOML config file (Codex CLI format, shared by MCP tool handlers).
fn register_mcp_toml_mcp(path: &std::path::Path, binary: &str) -> anyhow::Result<bool> {
    use toml_edit::{value, DocumentMut, Item, Table};

    let mut doc: DocumentMut = if path.exists() {
        let raw = std::fs::read_to_string(path)?;
        raw.parse::<DocumentMut>()?
    } else {
        DocumentMut::new()
    };

    if let Some(mcp) = doc.get("mcp_servers") {
        if let Some(s5d) = mcp.get("s5d") {
            if let Some(cmd) = s5d.get("command") {
                if cmd.as_str() == Some(binary) {
                    return Ok(false);
                }
            }
        }
    }

    if doc.get("mcp_servers").is_none() {
        doc["mcp_servers"] = Item::Table(Table::new());
    }
    let mcp = doc["mcp_servers"].as_table_mut().unwrap();

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
    validate_spec_path(spec_path)?;
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
    let spec = crate::infer::materialize_spec(&spec);
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
        previewed_spec_sha256: spec_sha.clone(),
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
    let mut record = project.load_record(&spec_filename)?.ok_or_else(|| {
        anyhow::anyhow!(
            "no record found for {} — run s5d_preview first",
            spec_filename
        )
    })?;

    if record.status != crate::SpecStatus::Previewed {
        anyhow::bail!(
            "spec must be in 'previewed' state before approval (current: {})",
            record.status
        );
    }

    // Check that spec hasn't changed since preview
    if let Some(ref preview) = record.preview {
        if !preview.previewed_spec_sha256.is_empty() && preview.previewed_spec_sha256 != spec_sha {
            anyhow::bail!("spec modified since preview — re-run s5d_preview before approving");
        }
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
    let failed = results
        .iter()
        .filter(|r| r.status != "passed" && r.status != "skipped")
        .count();
    let skipped = results.iter().filter(|r| r.status == "skipped").count();

    let mut out = String::new();
    for r in &results {
        let marker = match r.status.as_str() {
            "passed" => "pass",
            "skipped" => "skip",
            _ => "fail",
        };
        out.push_str(&format!(
            "  [{}] gate:{} (attempt {})\n",
            marker, r.kind, r.attempt
        ));
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
    let verified_by = args["verified_by"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("missing required argument: verified_by"))?;
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

    let import_checks = crate::check_import(&Some(record.clone()), verified_by);
    crate::enforce_checks(&import_checks, force)?;

    let effective_gates = crate::effective_gates_for_spec(&spec);
    if !effective_gates.is_empty() {
        let now_rfc = chrono::Utc::now().to_rfc3339();
        let all_latest_passed = effective_gates.iter().all(|g| {
            let latest = record.gate_results.iter().rev().find(|r| r.kind == g.kind);
            match latest {
                Some(r) if r.status == "passed" => true,
                Some(r) if r.status == "waived" => r
                    .waiver_expires_at
                    .as_ref()
                    .is_some_and(|expires_at| expires_at.as_str() >= now_rfc.as_str()),
                _ => false,
            }
        });
        if !all_latest_passed {
            anyhow::bail!("all effective gates must pass or be waived before import — run s5d_run_gates or s5d_waiver first");
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
    let materialized = crate::infer::materialize_spec(&spec);
    let mut aliases_check = crate::AliasTable::load(&s5d_dir)?;
    if let Some(ref meta) = materialized.meta {
        aliases_check.apply_renames(&materialized.id, &meta.renames);
    }
    let fresh_actions = crate::compute_diff(&materialized, &mut aliases_check);
    let fresh_diff_sha = fresh_actions.sha256();

    if fresh_diff_sha != approval.diff_sha256 {
        anyhow::bail!(
            "diff has changed since approval — re-run s5d_preview and s5d_approve\n  approved: {}\n  current:  {}",
            approval.diff_sha256,
            fresh_diff_sha
        );
    }

    record.verified_by = Some(verified_by.to_string());
    project.save_record(&spec_filename, &record)?;

    let (actions, fingerprint) =
        crate::execute_import(&project, &spec_path, &spec, &spec_filename)?;
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

    // Resolve project and load record from correct path (.s5d/records/, not packages/)
    let abs_path = path.canonicalize()?;
    let project = crate::S5dProject::find(&abs_path)
        .ok_or_else(|| anyhow::anyhow!("no .s5d/ found — is this file inside an s5d project?"))?;
    let spec_filename = abs_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("cannot determine filename from path"))?
        .to_string_lossy()
        .into_owned();
    let record: Option<crate::Record> = project.load_record(&spec_filename)?;

    let checks = crate::check_decide(&spec, &record, &confirmed_by.map(|s| s.to_string()));
    crate::enforce_checks(&checks, false)?;

    if !matches!(spec.tier, crate::Tier::Decision) {
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

    // Enforce spec_ref on winner hypothesis (feature decomposition)
    if winner_hyp.spec_ref.is_none() {
        anyhow::bail!(
            "winner hypothesis '{}' has no spec_ref — create a feature spec first: \
             s5d_new with hypothesis linking, then re-run s5d_decide",
            winner
        );
    }

    let rejected_ids: Vec<String> = match rejected {
        Some(s) => s
            .split(',')
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .collect(),
        None => vec![],
    };

    // Parse challenge from MCP args
    let challenge = if let Some(summary) = args["challenge_summary"].as_str() {
        let mode = args["challenge_mode"]
            .as_str()
            .unwrap_or("standard")
            .to_string();
        Some(crate::Challenge {
            mode,
            passed: true,
            summary: summary.to_string(),
            checks: vec![],
        })
    } else {
        None
    };

    // Adversarial challenge gate (centralized validation)
    if let Some(msg) = crate::check_challenge(&challenge, &spec.tier, false, false) {
        anyhow::bail!("{}", msg);
    }

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
        challenge,
        decision_subject: None,
        decision_subject_granularity: None,
        evaluative_surface: None,
        belief_state: None,
        outcome_model: None,
        pareto_set: vec![],
        choice_rule: None,
    };

    let spec_sha = crate::S5dProject::file_sha256(&abs_path)?;
    let mut record = record.unwrap_or_else(|| crate::generate_record(&spec_filename, &spec_sha));
    record.decision = Some(decision_record);
    project.save_record(&spec_filename, &record)?;

    Ok(format!("Decision recorded: {} (winner: {})", title, winner))
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
    let prompt = args["prompt"].as_str();
    let next_move = args["next_move"].as_str();
    if let Some(nm) = next_move {
        let allowed = ["deduction", "probe", "build", "defer"];
        if !allowed.contains(&nm) {
            anyhow::bail!("next_move must be one of: {}", allowed.join(", "));
        }
    }

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
        prompt: prompt.map(|s| s.into()),
        next_move: next_move.map(|s| s.into()),
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
    let refine_kind = args["refine_kind"].as_str();
    let skill = args["skill"].as_str().map(|s| s.to_string());
    let agent = args["agent"].as_str().map(|s| s.to_string());

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
    if verdict == "refine" && refine_kind.is_none() {
        anyhow::bail!(
            "refine_kind is required when verdict=refine (FPF C.2:4.2). Allowed: formalise, generalise, specialise, calibrate, validate, congrue."
        );
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
            anyhow::bail!("refine_kind must be one of: {}", allowed.join(", "));
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
    let verdict = args["verdict"].as_str().map(|s| s.trim().to_lowercase());
    let measurement_window = args["measurement_window"].as_str().map(|s| s.to_string());
    let summary = args["summary"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: summary"))?;
    let heuristic = args["heuristic"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: heuristic"))?;
    let issues_str = args["issues"].as_str().unwrap_or("");

    let (project, _spec_path, _spec, spec_filename) = load_spec_context_mcp(spec_arg)?;

    let mut record = project.load_record(&spec_filename)?.ok_or_else(|| {
        anyhow::anyhow!(
            "no record found for {} — run s5d_preview first",
            spec_filename
        )
    })?;

    let issues_list: Vec<String> = issues_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let telemetry_refs: Vec<String> = args["telemetry_refs"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    let heuristics = vec![heuristic.to_string()];

    if let Some(ref outcome) = verdict {
        let valid = ["confirmed", "refuted", "inconclusive", "iterate", "kill"];
        if !valid.contains(&outcome.as_str()) {
            anyhow::bail!(
                "invalid verdict '{}' (use confirmed, refuted, inconclusive, iterate, kill)",
                outcome
            );
        }
    }

    record.reflection = Some(crate::Reflection {
        verdict,
        measurement_window,
        telemetry_refs,
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

fn workflow_required_mcp(spec: &crate::Spec) -> anyhow::Result<&crate::Workflow> {
    spec.workflow
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("spec has no workflow block"))
}

fn workflow_phase_by_id_mcp<'a>(
    workflow: &'a crate::Workflow,
    phase_id: &str,
) -> anyhow::Result<&'a crate::WorkflowPhase> {
    workflow
        .phases
        .iter()
        .find(|p| p.id == phase_id)
        .ok_or_else(|| anyhow::anyhow!("work state not found: {}", phase_id))
}

fn latest_phase_status_mcp(record: &crate::Record, phase_id: &str) -> crate::WorkflowPhaseStatus {
    record
        .phase_history
        .iter()
        .rev()
        .find(|entry| entry.phase_id == phase_id)
        .map(|entry| entry.status.clone())
        .unwrap_or(crate::WorkflowPhaseStatus::Planned)
}

fn append_phase_history_mcp(
    record: &mut crate::Record,
    phase_id: &str,
    status: crate::WorkflowPhaseStatus,
    reviewer: Option<String>,
    engine: Option<String>,
    notes: Option<String>,
) {
    record.phase_history.push(crate::WorkflowPhaseRecord {
        phase_id: phase_id.to_string(),
        status,
        timestamp: chrono::Utc::now().to_rfc3339(),
        reviewer,
        engine,
        notes,
    });
}

fn ensure_phase_execution_ready_mcp(
    spec: &crate::Spec,
    record: &crate::Record,
    phase_id: &str,
) -> anyhow::Result<()> {
    if !matches!(
        record.status,
        crate::SpecStatus::Approved | crate::SpecStatus::Applied | crate::SpecStatus::Operated
    ) {
        anyhow::bail!(
            "phase execution requires approved or later spec state (current: {})",
            record.status
        );
    }
    let workflow = workflow_required_mcp(spec)?;
    workflow_phase_by_id_mcp(workflow, phase_id)?;
    Ok(())
}

fn tool_s5d_phase_list(args: &Value) -> anyhow::Result<String> {
    let spec_arg = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let (project, _spec_path, spec, spec_filename) = load_spec_context_mcp(spec_arg)?;
    let record = project.load_record(&spec_filename)?.ok_or_else(|| {
        anyhow::anyhow!(
            "no record found for {} — run s5d_preview first",
            spec_filename
        )
    })?;
    let workflow = workflow_required_mcp(&spec)?;

    let mut out = String::new();
    out.push_str(&format!("PHASES: {}\n", spec.id));
    for phase in &workflow.phases {
        let status = latest_phase_status_mcp(&record, &phase.id);
        let marker = if record.active_phase.as_deref() == Some(phase.id.as_str()) {
            "active"
        } else {
            "idle"
        };
        out.push_str(&format!(
            "- {} [{}] ({})\n  id: {}\n  scope: {}\n",
            phase.title, status, marker, phase.id, phase.scope
        ));
    }
    Ok(out)
}

fn tool_s5d_phase_start(args: &Value) -> anyhow::Result<String> {
    let spec_arg = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let phase_id = args["phase_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: phase_id"))?;
    let (project, _spec_path, spec, spec_filename) = load_spec_context_mcp(spec_arg)?;
    let workflow = workflow_required_mcp(&spec)?;
    let phase = workflow_phase_by_id_mcp(workflow, phase_id)?;
    let mut record = project.load_record(&spec_filename)?.ok_or_else(|| {
        anyhow::anyhow!(
            "no record found for {} — run s5d_preview first",
            spec_filename
        )
    })?;
    ensure_phase_execution_ready_mcp(&spec, &record, phase_id)?;

    if let Some(ref active_phase) = record.active_phase {
        if active_phase == phase_id {
            anyhow::bail!("phase '{}' is already active", phase_id);
        }
        anyhow::bail!(
            "phase '{}' is already active — accept or clear it before starting '{}'",
            active_phase,
            phase_id
        );
    }

    record.active_phase = Some(phase_id.to_string());
    append_phase_history_mcp(
        &mut record,
        phase_id,
        crate::WorkflowPhaseStatus::Active,
        None,
        workflow
            .execution_mode
            .as_ref()
            .map(|mode| mode.engine.clone()),
        Some(format!("Started work state '{}'", phase.title)),
    );
    project.save_record(&spec_filename, &record)?;

    Ok(format!(
        "Active work state -> {}\nScope: {}",
        phase_id, phase.scope
    ))
}

fn tool_s5d_phase_accept(args: &Value) -> anyhow::Result<String> {
    let spec_arg = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let phase_id = args["phase_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: phase_id"))?;
    let reviewer = args["reviewer"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: reviewer"))?;
    let (project, _spec_path, spec, spec_filename) = load_spec_context_mcp(spec_arg)?;
    let workflow = workflow_required_mcp(&spec)?;
    workflow_phase_by_id_mcp(workflow, phase_id)?;
    let mut record = project.load_record(&spec_filename)?.ok_or_else(|| {
        anyhow::anyhow!(
            "no record found for {} — run s5d_preview first",
            spec_filename
        )
    })?;
    ensure_phase_execution_ready_mcp(&spec, &record, phase_id)?;

    if record.active_phase.as_deref() != Some(phase_id) {
        anyhow::bail!(
            "phase '{}' is not active — start it before acceptance",
            phase_id
        );
    }

    record.active_phase = None;
    append_phase_history_mcp(
        &mut record,
        phase_id,
        crate::WorkflowPhaseStatus::Accepted,
        Some(reviewer.to_string()),
        None,
        Some("Human run evidence acceptance".into()),
    );
    project.save_record(&spec_filename, &record)?;

    Ok(format!("Phase '{}' accepted by {}", phase_id, reviewer))
}

fn tool_s5d_execute_loop(args: &Value) -> anyhow::Result<String> {
    let spec_arg = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let phase_id = args["phase_id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: phase_id"))?;
    let engine = args["engine"].as_str().unwrap_or("ralph");
    let mode = args["mode"].as_str();
    let (project, _spec_path, spec, spec_filename) = load_spec_context_mcp(spec_arg)?;
    let record = project.load_record(&spec_filename)?.ok_or_else(|| {
        anyhow::anyhow!(
            "no record found for {} — run s5d_preview first",
            spec_filename
        )
    })?;
    ensure_phase_execution_ready_mcp(&spec, &record, phase_id)?;

    let workflow = workflow_required_mcp(&spec)?;
    let phase = workflow_phase_by_id_mcp(workflow, phase_id)?;

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
        anyhow::bail!("execute loop currently supports only engine=ralph");
    }

    let preset = crate::RalphPreset::resolve(mode, &phase.id)?;
    let package = crate::build_ralph_task_package(&spec, workflow, phase, preset)?;
    let task_path = project.save_task_artifact(&spec_filename, &phase.id, preset.id(), &package)?;
    let display_path = task_path
        .strip_prefix(project.root.as_path())
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| task_path.display().to_string());

    let mut result = format!("Task artifact: {}\n\n{}", display_path, package);
    if let Some(warning) = crate::ralph::check_vertical_slicing(phase) {
        result.push_str(&format!("\n⚠ Methodological warning: {}\n", warning));
    }
    Ok(result)
}

// ── s5d_status ────────────────────────────────────────────────────────────────

fn tool_s5d_status(_args: &Value) -> anyhow::Result<String> {
    let cwd = std::env::current_dir()?;
    let project = crate::S5dProject::find(&cwd).ok_or_else(|| anyhow::anyhow!("no .s5d/ found"))?;

    let specs = project.discover_specs()?;
    if specs.is_empty() {
        return Ok("No specs. Run s5d_new to create one.".to_string());
    }

    let mut out = format!("S5D Specs ({})\n", project.root.display());
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
            spec.id,
            spec.version,
            format!("{}", spec.tier),
            status,
            sync
        ));
    }

    Ok(out.trim_end().to_string())
}

// ── s5d_show ──────────────────────────────────────────────────────────────────

fn tool_s5d_show(args: &Value) -> anyhow::Result<String> {
    let spec_path = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let (path, spec) = load_spec_yaml_mcp(spec_path)?;

    // Load record for decision/approval truth
    let effective_decision = if let Ok(abs) = path.canonicalize() {
        if let Some(project) = crate::S5dProject::find(&abs) {
            let fname = abs.file_name().unwrap().to_string_lossy().into_owned();
            project
                .load_record(&fname)
                .ok()
                .flatten()
                .and_then(|r| r.decision)
        } else {
            None
        }
    } else {
        None
    }
    .or_else(|| spec.decision.clone());

    let is_decision = matches!(spec.tier, crate::Tier::Decision);
    let tier_label = format!("{}", spec.tier).to_uppercase();
    let title = effective_decision
        .as_ref()
        .map(|d| d.title.as_str())
        .unwrap_or(&spec.id);

    let mut out = format!("{}: {}\n", tier_label, title);
    out.push_str(&format!("{}\n", "━".repeat(60)));

    if is_decision {
        if let Some(ref problem) = spec.problem {
            out.push_str(&format!("  Signal: {}\n", problem.signal()));
        }
        if let Some(ref dec) = effective_decision {
            if !dec.context.is_empty() {
                out.push_str(&format!("  Context: {}\n", dec.context));
            }
        }

        let winner_id = effective_decision
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

        if let Some(ref dec) = effective_decision {
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
        let effective_gates = crate::effective_gates_for_spec(&spec);
        if !effective_gates.is_empty() {
            let source = if spec.gates.is_empty() {
                "effective, tier defaults"
            } else {
                "declared"
            };
            out.push_str(&format!(
                "  Gates: {} ({})\n",
                effective_gates.len(),
                source
            ));
        }
    }

    Ok(out.trim_end().to_string())
}

// ── s5d_trace ─────────────────────────────────────────────────────────────────

fn tool_s5d_trace(args: &Value) -> anyhow::Result<String> {
    let path = args["path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: path"))?;
    let cwd = std::env::current_dir()?;
    let project = crate::S5dProject::find(&cwd)
        .ok_or_else(|| anyhow::anyhow!("no .s5d/ found (run s5d_init first)"))?;
    let trace = crate::trace_code_path(&project, path)?;
    Ok(crate::format_code_trace(&trace))
}

// ── s5d_route ─────────────────────────────────────────────────────────────────

fn tool_s5d_route(args: &Value) -> anyhow::Result<String> {
    let description = args["description"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: description"))?;

    let result = crate::route(description);
    Ok(serde_json::to_string_pretty(&result)?)
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
    // FPF B.3.4:5 (CC-ED.5) — waivers MUST have a short-term expiry. Default 90 days.
    let expires_at = args["expires_at"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| (chrono::Utc::now() + chrono::Duration::days(90)).to_rfc3339());

    let (project, _spec_path, spec, spec_filename) = load_spec_context_mcp(spec_arg)?;

    // Check that the gate kind exists in the effective spec contract.
    let effective_gates = crate::effective_gates_for_spec(&spec);
    let gate_exists = effective_gates.iter().any(|g| g.kind == gate);
    if !gate_exists {
        anyhow::bail!(
            "No gate kind '{}' exists in the effective spec contract. Effective gates: {}",
            gate,
            effective_gates
                .iter()
                .map(|g| g.kind.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // Record waiver as a GateResult with status "waived" in the record file
    let mut record = project.load_record(&spec_filename)?.ok_or_else(|| {
        anyhow::anyhow!("no record found for spec — run s5d_preview and s5d_approve first")
    })?;

    record.gate_results.push(crate::GateResult {
        kind: gate.to_string(),
        status: "waived".to_string(),
        attempt: record
            .gate_results
            .iter()
            .filter(|r| r.kind == gate)
            .count() as u32
            + 1,
        timestamp: chrono::Utc::now().to_rfc3339(),
        log: Some(format!(
            "WAIVER — reason: {}. Condition to lift: {}. Approved by: {}. Expires: {}.",
            reason, condition, approved_by, expires_at
        )),
        exit_code: None,
        evidence_path: None,
        command: None,
        duration_ms: None,
        waiver_expires_at: Some(expires_at.clone()),
        kind_class: None,
    });

    project.save_record(&spec_filename, &record)?;

    Ok(format!(
        "Gate '{}' waived — approved by: {}, expires: {}, reason: {}, condition to lift: {}",
        gate, approved_by, expires_at, reason, condition
    ))
}

// ── s5d_rollback ──────────────────────────────────────────────────────────────

fn tool_s5d_rollback(args: &Value) -> anyhow::Result<String> {
    let spec_arg = args["spec"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: spec"))?;
    let (project, _spec_path, spec, spec_filename) = load_spec_context_mcp(spec_arg)?;
    let _lock = project.acquire_lock(&format!("rollback.{}", spec.id))?;

    let s5d_dir = project.s5d_dir();
    let mut ledger = project.load_ledger()?;

    let has_import = ledger
        .entries
        .iter()
        .any(|e| e.package_id == spec.id && e.action == "import" && e.status == "success");

    if !has_import {
        anyhow::bail!("no successful import found for {} to roll back", spec.id);
    }

    // Collect global artifact IDs still referenced by other specs
    let other_specs = project.discover_specs()?;
    let mut referenced_globals = std::collections::HashSet::new();
    for (_, other) in &other_specs {
        if other.id != spec.id {
            referenced_globals.extend(crate::import::collect_global_artifact_ids(other));
        }
    }

    let mut aliases = crate::AliasTable::load(&s5d_dir)?;
    for entry in &mut aliases.packages {
        if entry.package_id.as_deref() == Some(&spec.id) && !entry.deprecated {
            entry.deprecated = true;
        }
    }
    for entry in &mut aliases.global {
        if entry.owning_package.as_deref() == Some(&spec.id) && !entry.deprecated {
            let key = (entry.artifact_type.clone(), entry.artifact_id.clone());
            if !referenced_globals.contains(&key) {
                entry.deprecated = true;
            }
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
            crate::DriftResult::Partial { policy, note } => {
                out.push_str(&format!(
                    "{} — partial drift (tolerated by policy: {})\n  {}\n",
                    spec.id, policy, note
                ));
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
                crate::DriftResult::Partial { policy, note } => {
                    out.push_str(&format!(
                        "{} — partial drift (tolerated by policy: {})\n  {}\n",
                        spec.id, policy, note
                    ));
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

fn tool_s5d_note(args: &Value) -> anyhow::Result<String> {
    let text = args["text"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing required argument: text"))?;

    if text.is_empty() {
        anyhow::bail!("note text cannot be empty");
    }

    // Generate slug from text
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
    let product = args["product"].as_str();

    // Delegate to s5d_new internals
    let cwd = std::env::current_dir()?;
    let project = crate::S5dProject::find(&cwd)
        .ok_or_else(|| anyhow::anyhow!("no .s5d/ found (run s5d_init first)"))?;

    let product_name = product.unwrap_or("default");
    let today = chrono::Utc::now().format("%Y%m%d").to_string();
    let spec_filename = format!("{}__{}.s5d.yaml", id, today);
    let spec_path = project.s5d_dir().join("packages").join(&spec_filename);

    if spec_path.exists() {
        anyhow::bail!("spec already exists: {}", spec_path.display());
    }

    let spec = crate::generate_note_spec(&id, product_name, text, text);
    let yaml = serde_yaml::to_string(&spec)?;
    std::fs::write(&spec_path, &yaml)?;

    let sha = crate::S5dProject::file_sha256(&spec_path)?;
    let record = crate::generate_record(&spec_filename, &sha);
    project.save_record(&spec_filename, &record)?;

    let record_path = project
        .s5d_dir()
        .join("records")
        .join(spec_filename.replace(".s5d.yaml", ".record.yaml"));

    Ok(format!(
        "Note created: {}\n  spec: {}\n  record: {}",
        id,
        spec_path.display(),
        record_path.display()
    ))
}
