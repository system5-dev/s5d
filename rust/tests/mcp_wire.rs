//! Wire-level tests for the MCP server (scenario #12: MCP containment).
//!
//! The MCP surface is what agents actually speak to; until now it had zero
//! integration coverage — every guarantee was inferred from unit-level code
//! review. These tests spawn the real binary and talk line-delimited
//! JSON-RPC over stdio, exactly like an MCP client.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};
use tempfile::TempDir;

fn s5d_bin() -> &'static str {
    env!("CARGO_BIN_EXE_s5d")
}

struct McpServer {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl McpServer {
    fn spawn(cwd: &Path) -> Self {
        let mut child = Command::new(s5d_bin())
            .arg("mcp")
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            stdin: Some(stdin),
            stdout,
            next_id: 0,
        }
    }

    fn send_raw(&mut self, raw: &str) {
        let stdin = self.stdin.as_mut().unwrap();
        writeln!(stdin, "{}", raw).unwrap();
        stdin.flush().unwrap();
    }

    /// Send a request and read exactly one response line, asserting id match.
    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let req = json!({"jsonrpc":"2.0","id":self.next_id,"method":method,"params":params});
        self.send_raw(&req.to_string());
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        assert!(
            !line.is_empty(),
            "server closed stdout instead of answering {method}"
        );
        let resp: Value = serde_json::from_str(&line)
            .unwrap_or_else(|e| panic!("non-JSON response to {method}: {e}\nline: {line}"));
        assert_eq!(
            resp["id"],
            json!(self.next_id),
            "response id mismatch for {method}: {line}"
        );
        resp
    }

    fn call_tool(&mut self, name: &str, arguments: Value) -> Value {
        self.request("tools/call", json!({"name": name, "arguments": arguments}))
    }

    /// Like call_tool, but asserts the call succeeded (no JSON-RPC error,
    /// no in-band isError). Chain steps must fail AT the step, not three
    /// calls later with a misleading symptom.
    fn call_tool_ok(&mut self, name: &str, arguments: Value) -> Value {
        let resp = self.call_tool(name, arguments);
        assert!(
            resp.get("error").is_none(),
            "{name} returned a JSON-RPC error: {resp}"
        );
        assert_ne!(
            resp["result"]["isError"],
            json!(true),
            "{name} returned an in-band tool error: {resp}"
        );
        resp
    }

    /// Close stdin and wait for the process to exit.
    fn shutdown(mut self) -> std::process::ExitStatus {
        drop(self.stdin.take());
        self.child.wait().unwrap()
    }
}

/// Extract the text payload of a successful tools/call response.
fn tool_text(resp: &Value) -> &str {
    resp["result"]["content"][0]["text"].as_str().unwrap_or("")
}

#[test]
fn mcp_wire_initialize_and_tools_list_roundtrip() {
    let dir = TempDir::new().unwrap();
    let mut server = McpServer::spawn(dir.path());

    let init = server.request("initialize", json!({}));
    assert_eq!(init["result"]["protocolVersion"], "2024-11-05");
    assert!(
        init["result"]["capabilities"]["tools"].is_object(),
        "initialize must advertise tools capability: {init}"
    );

    let listed = server.request("tools/list", json!({}));
    let tools = listed["result"]["tools"].as_array().unwrap();
    assert!(
        tools.len() >= 20,
        "expected the full tool surface, got {}",
        tools.len()
    );
    for tool in tools {
        assert!(tool["name"].as_str().is_some_and(|n| n.starts_with("s5d_")));
        assert!(tool["description"].as_str().is_some_and(|d| !d.is_empty()));
        assert!(tool["inputSchema"].is_object());
    }
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    for required in ["s5d_init", "s5d_new", "s5d_import", "s5d_status"] {
        assert!(names.contains(&required), "missing tool {required}");
    }

    assert!(server.shutdown().success(), "server must exit 0 on EOF");
}

#[test]
fn mcp_wire_survives_garbage_notifications_and_unknown_methods() {
    let dir = TempDir::new().unwrap();
    let mut server = McpServer::spawn(dir.path());

    // Malformed JSON and id-less notifications must be silently skipped,
    // not crash the server or desync the response stream.
    server.send_raw("this is not json {{{");
    server.send_raw(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);

    let unknown = server.request("no/such/method", json!({}));
    assert_eq!(
        unknown["error"]["code"], -32601,
        "unknown method must map to -32601: {unknown}"
    );

    // Tool-level failure is a JSON-RPC error, and the server keeps serving.
    let failing = server.request("tools/call", json!({"name": "s5d_status", "arguments": {}}));
    assert_eq!(
        failing["error"]["code"], -32603,
        "tool failure outside a project must be -32603, not a crash: {failing}"
    );

    let unknown_tool = server.request("tools/call", json!({"name": "s5d_nope", "arguments": {}}));
    assert_eq!(
        unknown_tool["result"]["isError"], true,
        "unknown tool is an in-band tool error: {unknown_tool}"
    );

    let still_alive = server.request("initialize", json!({}));
    assert_eq!(still_alive["result"]["protocolVersion"], "2024-11-05");

    assert!(server.shutdown().success(), "server must exit 0 on EOF");
}

#[test]
fn mcp_wire_full_lifecycle_with_import_guard_parity() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();

    let mut server = McpServer::spawn(root);
    server.request("initialize", json!({}));

    server.call_tool_ok("s5d_init", json!({}));

    let created = server.call_tool_ok(
        "s5d_new",
        json!({"id": "feat.demo.wire", "tier": "standard", "product": "demo"}),
    );
    assert!(
        tool_text(&created).contains("feat.demo.wire"),
        "s5d_new: {created}"
    );
    let spec_rel = std::fs::read_dir(root.join(".s5d/packages"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .find(|n| n.ends_with(".s5d.yaml"))
        .map(|n| format!(".s5d/packages/{n}"))
        .expect("spec file created");

    server.call_tool_ok("s5d_preview", json!({"spec": spec_rel}));
    server.call_tool_ok(
        "s5d_approve",
        json!({"spec": spec_rel, "reviewer": "roman"}),
    );
    server.call_tool_ok("s5d_run_gates", json!({"spec": spec_rel}));

    // The scaffold still carries TODO-set-source-paths/ — the MCP import
    // path must refuse it just like the CLI does (guard parity).
    let denied = server.call_tool(
        "s5d_import",
        json!({"spec": spec_rel, "verified_by": "diana"}),
    );
    assert!(
        denied["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("scaffold placeholder paths"),
        "MCP import must refuse the placeholder scaffold: {denied}"
    );

    // Materialize real paths the way a user would, re-run the chain, import.
    let spec_path = root.join(&spec_rel);
    let spec_src = std::fs::read_to_string(&spec_path).unwrap();
    std::fs::write(
        &spec_path,
        spec_src.replace("TODO-set-source-paths/", "src/"),
    )
    .unwrap();

    server.call_tool_ok("s5d_preview", json!({"spec": spec_rel}));
    server.call_tool_ok(
        "s5d_approve",
        json!({"spec": spec_rel, "reviewer": "roman"}),
    );
    server.call_tool_ok("s5d_run_gates", json!({"spec": spec_rel}));
    let imported = server.call_tool_ok(
        "s5d_import",
        json!({"spec": spec_rel, "verified_by": "diana"}),
    );
    assert!(
        tool_text(&imported).contains("Imported"),
        "import after materializing paths must succeed: {imported}"
    );

    let status = server.call_tool_ok("s5d_status", json!({}));
    assert!(
        tool_text(&status).contains("feat.demo.wire"),
        "status must show the imported spec: {status}"
    );

    assert!(server.shutdown().success(), "server must exit 0 on EOF");
}

#[test]
fn mcp_wire_shape_review_stories_parity() {
    // CLI/MCP parity for the shape-native surface
    // (decision.s5d.bmad-native-runtime): kernel shaping with emit, layered
    // review scaffold, story phases — all over real stdio JSON-RPC.
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    let mut server = McpServer::spawn(root);
    server.request("initialize", json!({}));
    server.call_tool_ok("s5d_init", json!({}));

    // Not-ready kernel reports readiness errors without failing the call.
    let partial = server.call_tool_ok(
        "s5d_shape",
        json!({"kernel": {"why": "slow checkout drops conversions"}}),
    );
    let text = tool_text(&partial);
    assert!(text.contains("Route:"), "s5d_shape route block: {partial}");
    assert!(
        text.contains("intent_kernel.success_signal"),
        "readiness error must be named: {partial}"
    );

    // Ready kernel + emit_spec creates a spec with the kernel embedded.
    let emitted = server.call_tool_ok(
        "s5d_shape",
        json!({
            "kernel": {
                "why": "slow checkout drops conversions",
                "success_signal": "p95 checkout under 3s",
                "constraints": ["no new PII collection"]
            },
            "emit_spec": "feat.fastpay",
            "product": "shop"
        }),
    );
    assert!(
        tool_text(&emitted).contains("Created spec with embedded kernel"),
        "emit: {emitted}"
    );
    let spec_rel = std::fs::read_dir(root.join(".s5d/packages"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .find(|n| n.starts_with("feat.fastpay"))
        .map(|n| format!(".s5d/packages/{n}"))
        .expect("emitted spec exists");
    let spec_yaml = std::fs::read_to_string(root.join(&spec_rel)).unwrap();
    assert!(
        spec_yaml.contains("intent_kernel") && spec_yaml.contains("p95 checkout under 3s"),
        "kernel embedded in spec file:\n{spec_yaml}"
    );

    // Adversarial review scaffold lands in the evidence dir.
    let review = server.call_tool_ok("s5d_review_adversarial", json!({"spec": spec_rel}));
    assert!(
        tool_text(&review).contains("Binding:"),
        "review binding: {review}"
    );
    assert!(root
        .join(".s5d/evidence/feat.fastpay/adversarial-review-1.md")
        .exists());

    // Story phases append and validate.
    let stories = server.call_tool_ok(
        "s5d_plan_stories",
        json!({
            "spec": spec_rel,
            "stories": [
                {"id": "story-wallet", "title": "Wallet support", "scope": "Apple/Google Pay path", "acceptance": ["wallet button renders on supported devices"]}
            ]
        }),
    );
    assert!(
        tool_text(&stories).contains("story-wallet"),
        "stories: {stories}"
    );
    let spec_yaml = std::fs::read_to_string(root.join(&spec_rel)).unwrap();
    assert!(spec_yaml.contains("story-wallet"));

    assert!(server.shutdown().success(), "server must exit 0 on EOF");
}
