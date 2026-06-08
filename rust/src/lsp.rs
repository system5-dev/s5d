//! Minimal synchronous LSP server for `.s5d.yaml` specs.
//!
//! Slice 1: `textDocument/publishDiagnostics` only. The diagnostics are produced
//! by the existing [`crate::validate_spec`] — the editor sees exactly what
//! `s5d validate` and the pre-commit gate see, with no second validation path.
//! Hover / completion / symbols / goto are later slices and read the same
//! parsed `Spec`, keeping the LSP a pure projection of the validator.
//!
//! Synchronous by construction: `lsp-server` is a blocking crossbeam loop, the
//! same shape as the MCP stdin loop in [`crate::mcp`]. No async runtime.

use lsp_server::{Connection, Message, Notification as ServerNotification};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    Position, PublishDiagnosticsParams, Range, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind,
};

/// Run the LSP server over stdio. Blocks until the client sends shutdown/exit.
pub fn run() -> anyhow::Result<()> {
    let (connection, io_threads) = Connection::stdio();

    let capabilities = serde_json::to_value(ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        ..Default::default()
    })?;
    connection.initialize(capabilities)?;

    main_loop(&connection)?;
    io_threads.join()?;
    Ok(())
}

fn main_loop(connection: &Connection) -> anyhow::Result<()> {
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                // Slice 1 advertises only diagnostics, but every JSON-RPC request
                // still needs a response or the client hangs waiting on it. Reply
                // MethodNotFound rather than dropping it.
                let resp = lsp_server::Response::new_err(
                    req.id,
                    -32601, // JSON-RPC MethodNotFound
                    format!("s5d lsp (slice 1) does not handle request: {}", req.method),
                );
                connection.sender.send(Message::Response(resp))?;
            }
            Message::Notification(note) => match note.method.as_str() {
                "textDocument/didOpen" => {
                    let p: DidOpenTextDocumentParams = serde_json::from_value(note.params)?;
                    publish(connection, p.text_document.uri, &p.text_document.text)?;
                }
                "textDocument/didChange" => {
                    let p: DidChangeTextDocumentParams = serde_json::from_value(note.params)?;
                    // FULL sync: the last change carries the whole document.
                    if let Some(change) = p.content_changes.into_iter().next_back() {
                        publish(connection, p.text_document.uri, &change.text)?;
                    }
                }
                _ => {}
            },
            Message::Response(_) => {}
        }
    }
    Ok(())
}

fn publish(connection: &Connection, uri: lsp_types::Uri, text: &str) -> anyhow::Result<()> {
    // Only `.s5d.yaml` specs are validated; anything else gets an empty list,
    // which clears any stale diagnostics the client may be showing.
    let diagnostics = if uri.as_str().ends_with(".s5d.yaml") {
        diagnostics_for(text)
    } else {
        Vec::new()
    };
    let params = PublishDiagnosticsParams {
        uri,
        diagnostics,
        version: None,
    };
    connection.sender.send(Message::Notification(ServerNotification::new(
        "textDocument/publishDiagnostics".to_string(),
        params,
    )))?;
    Ok(())
}

/// Compute diagnostics for one `.s5d.yaml` document. Pure and unit-testable.
///
/// Two tiers of fidelity:
///   - YAML parse errors map to a precise range via `serde_yaml::Error::location()`.
///   - Schema/metamodel errors (the `Vec<String>` from `validate_spec`) map to the
///     first line for now; the message already names the offending artifact. A
///     later slice can text-search the id to tighten the range without touching
///     `validate.rs`.
pub fn diagnostics_for(text: &str) -> Vec<Diagnostic> {
    match serde_yaml::from_str::<crate::Spec>(text) {
        Err(e) => {
            let range = e
                .location()
                .map(|loc| {
                    // serde_yaml line/column are 1-based; LSP positions are 0-based.
                    // `character` here is a Unicode-scalar offset; LSP defaults to
                    // UTF-16 code units — identical for ASCII/BMP YAML (the common
                    // case for specs). Exact non-BMP ranges need position-encoding
                    // negotiation, deferred past slice 1.
                    let line = loc.line().saturating_sub(1) as u32;
                    let col = loc.column().saturating_sub(1) as u32;
                    Range::new(Position::new(line, col), Position::new(line, col + 1))
                })
                .unwrap_or_else(|| Range::new(Position::new(0, 0), Position::new(0, 0)));
            vec![diag(range, format!("parse error: {e}"))]
        }
        Ok(spec) => {
            let first_line_len =
                text.lines().next().map(|l| l.chars().count()).unwrap_or(0) as u32;
            let range = Range::new(Position::new(0, 0), Position::new(0, first_line_len));
            crate::validate_spec(&spec)
                .into_iter()
                .map(|msg| diag(range, msg))
                .collect()
        }
    }
}

fn diag(range: Range, message: String) -> Diagnostic {
    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("s5d".to_string()),
        message,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_LIGHTWEIGHT: &str = r#"
s5d: "1.0"
id: feat-x
version: "1.0.0"
product: shop
tier: lightweight
meta:
  title: x
artifacts:
  products:
    - id: shop
      name: Shop
  features:
    - id: feat-x
      product: shop
      name: X
  capabilities:
    - id: cap.x
      domain: dom.x
      name: DoX
gates:
  - kind: schema
"#;

    #[test]
    fn valid_spec_has_no_diagnostics() {
        let diags = diagnostics_for(VALID_LIGHTWEIGHT);
        assert!(diags.is_empty(), "expected none, got: {diags:?}");
    }

    #[test]
    fn malformed_yaml_reports_a_diagnostic() {
        let diags = diagnostics_for("s5d: \"1.0\"\n  : : bad indent");
        assert!(!diags.is_empty());
        assert_eq!(diags[0].severity, Some(DiagnosticSeverity::ERROR));
    }

    #[test]
    fn schema_violation_reports_a_diagnostic() {
        // Valid YAML, invalid metamodel: lightweight requires non-empty capabilities.
        let text = r#"
s5d: "1.0"
id: feat-x
version: "1.0.0"
product: shop
tier: lightweight
meta:
  title: x
artifacts:
  products:
    - id: shop
      name: Shop
  features:
    - id: feat-x
      product: shop
      name: X
  capabilities: []
gates: []
"#;
        let diags = diagnostics_for(text);
        assert!(!diags.is_empty(), "expected a validation diagnostic");
    }
}
