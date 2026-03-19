use std::path::Path;

use regex::Regex;
use sha2::{Digest, Sha256};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};

pub struct CodeChunk {
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub lang: String,
    pub text: String,
    pub content_hash: String,
}

pub struct SymbolRecord {
    pub name: String,
    pub kind: String,
    pub file_path: String,
    pub line: u32,
    pub line_end: Option<u32>,
    pub qualname: Option<String>,
    pub signature: Option<String>,
    pub doc_comment: Option<String>,
}

pub struct ImportEdge {
    pub source_file: String,
    pub imported: String,
}

pub struct ParseResult {
    pub chunks: Vec<CodeChunk>,
    pub symbols: Vec<SymbolRecord>,
    pub imports: Vec<ImportEdge>,
    pub lang: String,
}

pub fn detect_lang(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?;
    match ext {
        "rs" => Some("rust"),
        "py" => Some("python"),
        "js" | "mjs" | "cjs" => Some("javascript"),
        "ts" => Some("typescript"),
        "tsx" => Some("tsx"),
        "java" => Some("java"),
        "c" | "h" => Some("c"),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some("cpp"),
        "cs" => Some("csharp"),
        "kt" | "kts" => Some("generic"),
        "rb" => Some("ruby"),
        "go" => Some("go"),
        "php" => Some("generic"),
        "scala" | "sc" => Some("generic"),
        "swift" => Some("generic"),
        _ => Some("generic"),
    }
    .map(|s| s.to_string())
}

pub fn parse_file(path: &Path) -> anyhow::Result<ParseResult> {
    let lang_name = match detect_lang(path) {
        Some(l) => l,
        None => {
            return Ok(ParseResult {
                chunks: vec![],
                symbols: vec![],
                imports: vec![],
                lang: "unknown".into(),
            })
        }
    };

    let src = std::fs::read(path)?;

    if lang_name == "generic" {
        return Ok(parse_generic(path, &src));
    }

    let file_path = path.to_string_lossy().into_owned();

    let (ts_lang, fn_query_src, struct_query_src, import_query_src) = match lang_name.as_str() {
        "rust" => (
            tree_sitter_rust::LANGUAGE.into(),
            "(function_item name: (identifier) @name)",
            "(struct_item name: (type_identifier) @name) (trait_item name: (type_identifier) @name) (enum_item name: (type_identifier) @name) (type_item name: (type_identifier) @name) (const_item name: (identifier) @name)",
            "(use_declaration) @import",
        ),
        "python" => (
            tree_sitter_python::LANGUAGE.into(),
            "(function_definition name: (identifier) @name)",
            "(class_definition name: (identifier) @name)",
            "(import_statement) @import (import_from_statement) @import",
        ),
        "javascript" => (
            tree_sitter_javascript::LANGUAGE.into(),
            "(function_declaration name: (identifier) @name)",
            "(class_declaration name: (identifier) @name)",
            "(import_statement) @import",
        ),
        "typescript" => (
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            "(function_declaration name: (identifier) @name) (method_definition name: (property_identifier) @name) (function_signature name: (identifier) @name)",
            "(class_declaration name: (type_identifier) @name) (interface_declaration name: (type_identifier) @name) (type_alias_declaration name: (type_identifier) @name) (enum_declaration name: (identifier) @name)",
            "(import_statement) @import",
        ),
        "tsx" => (
            tree_sitter_typescript::LANGUAGE_TSX.into(),
            "(function_declaration name: (identifier) @name) (method_definition name: (property_identifier) @name) (function_signature name: (identifier) @name)",
            "(class_declaration name: (type_identifier) @name) (interface_declaration name: (type_identifier) @name) (type_alias_declaration name: (type_identifier) @name) (enum_declaration name: (identifier) @name)",
            "(import_statement) @import",
        ),
        "java" => (
            tree_sitter_java::LANGUAGE.into(),
            "(method_declaration name: (identifier) @name)",
            "(class_declaration name: (identifier) @name) (interface_declaration name: (identifier) @name) (enum_declaration name: (identifier) @name)",
            "(import_declaration) @import",
        ),
        "c" => (
            tree_sitter_c::LANGUAGE.into(),
            "(function_definition declarator: (function_declarator declarator: (identifier) @name))",
            "(struct_specifier name: (type_identifier) @name) (type_definition declarator: (type_identifier) @name)",
            "(preproc_include) @import",
        ),
        "cpp" => (
            tree_sitter_cpp::LANGUAGE.into(),
            "(function_definition declarator: (function_declarator declarator: (identifier) @name))",
            "(class_specifier name: (type_identifier) @name) (struct_specifier name: (type_identifier) @name)",
            "(preproc_include) @import",
        ),
        "csharp" => (
            tree_sitter_c_sharp::LANGUAGE.into(),
            "(method_declaration name: (identifier) @name)",
            "(class_declaration name: (identifier) @name) (interface_declaration name: (identifier) @name)",
            "(using_directive) @import",
        ),
        "ruby" => (
            tree_sitter_ruby::LANGUAGE.into(),
            "(method (identifier) @name)",
            "(class (constant) @name) (module (constant) @name)",
            r#"(call method: (identifier) @method (#eq? @method "require")) @import"#,
        ),
        "go" => (
            tree_sitter_go::LANGUAGE.into(),
            "(function_declaration name: (identifier) @name)",
            "(type_spec name: (type_identifier) @name)",
            "(import_spec) @import",
        ),
        _ => return Ok(parse_generic(path, &src)),
    };

    let mut parser = Parser::new();
    parser.set_language(&ts_lang)?;

    let tree = parser
        .parse(&src, None)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to parse {:?}", path))?;
    let root = tree.root_node();

    // ── symbols ──────────────────────────────────────────────────────────────

    let mut symbols: Vec<SymbolRecord> = Vec::new();

    let collect_symbols =
        |query_src: &str, kind: &str, syms: &mut Vec<SymbolRecord>| -> anyhow::Result<()> {
            let query = Query::new(&ts_lang, query_src)?;
            let cap_idx = query.capture_index_for_name("name");
            let mut cursor = QueryCursor::new();
            let mut matches = cursor.matches(&query, root, src.as_slice());
            while let Some(m) = matches.next() {
                for cap in m.captures {
                    if cap_idx.is_none_or(|i| cap.index == i) {
                        let name = cap.node.utf8_text(&src).unwrap_or("").to_string();
                        let line = cap.node.start_position().row as u32 + 1;
                        let line_end = cap.node.parent().map(|p| p.end_position().row as u32 + 1);
                        syms.push(SymbolRecord {
                            name,
                            kind: kind.into(),
                            file_path: file_path.clone(),
                            line,
                            line_end,
                            qualname: None,
                            signature: None,
                            doc_comment: None,
                        });
                    }
                }
            }
            Ok(())
        };

    // fn symbols
    if let Err(e) = collect_symbols(fn_query_src, "fn", &mut symbols) {
        let _ = e;
    }

    // struct/class symbols
    let struct_kind = match lang_name.as_str() {
        "rust" => {
            let patterns = [
                ("(struct_item name: (type_identifier) @name)", "struct"),
                ("(trait_item name: (type_identifier) @name)", "trait"),
                ("(enum_item name: (type_identifier) @name)", "enum"),
                ("(type_item name: (type_identifier) @name)", "type"),
                ("(const_item name: (identifier) @name)", "const"),
            ];
            for (q, k) in &patterns {
                let _ = collect_symbols(q, k, &mut symbols);
            }
            None
        }
        _ => Some("class"),
    };
    if let Some(kind) = struct_kind {
        let _ = collect_symbols(struct_query_src, kind, &mut symbols);
    }

    // ── imports ───────────────────────────────────────────────────────────────

    let mut imports: Vec<ImportEdge> = Vec::new();
    {
        let query = Query::new(&ts_lang, import_query_src);
        if let Ok(query) = query {
            let mut cursor = QueryCursor::new();
            let mut matches = cursor.matches(&query, root, src.as_slice());
            while let Some(m) = matches.next() {
                for cap in m.captures {
                    let text = cap.node.utf8_text(&src).unwrap_or("").trim().to_string();
                    if !text.is_empty() {
                        imports.push(ImportEdge {
                            source_file: file_path.clone(),
                            imported: text,
                        });
                    }
                }
            }
        }
    }

    // ── chunking ──────────────────────────────────────────────────────────────

    let max_chunk = 1000usize;
    let mut chunks: Vec<CodeChunk> = Vec::new();

    let child_count = root.child_count();
    for i in 0..child_count {
        let node = root.child(i as u32).unwrap();
        let node_type = node.kind();
        let is_def = matches!(
            node_type,
            "function_item"
                | "function_definition"
                | "function_declaration"
                | "struct_item"
                | "enum_item"
                | "trait_item"
                | "impl_item"
                | "class_definition"
                | "class_declaration"
                | "method_definition"
                | "method_declaration"
                | "class_specifier"
                | "struct_specifier"
                | "interface_declaration"
                | "enum_declaration"
                | "type_alias_declaration"
        );
        if !is_def {
            continue;
        }
        let byte_range = node.byte_range();
        let text = match std::str::from_utf8(&src[byte_range.clone()]) {
            Ok(t) => t.to_string(),
            Err(_) => continue,
        };

        if text.len() <= max_chunk {
            chunks.push(make_chunk(
                &file_path,
                &lang_name,
                node.start_position().row as u32 + 1,
                node.end_position().row as u32 + 1,
                &text,
            ));
        } else {
            let mut current = String::new();
            let mut start_line = node.start_position().row as u32 + 1;
            let mut last_line = start_line;
            let child_count2 = node.child_count();
            let walk2 = node.walk();
            for j in 0..child_count2 {
                let child = node.child(j as u32).unwrap();
                let child_bytes = &src[child.byte_range()];
                let child_text = match std::str::from_utf8(child_bytes) {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let child_start = child.start_position().row as u32 + 1;
                let child_end = child.end_position().row as u32 + 1;
                if !current.is_empty() && current.len() + child_text.len() > max_chunk {
                    chunks.push(make_chunk(
                        &file_path, &lang_name, start_line, last_line, &current,
                    ));
                    current = child_text.to_string();
                    start_line = child_start;
                    last_line = child_end;
                } else {
                    current.push_str(child_text);
                    last_line = child_end;
                }
                let _ = walk2;
                let _ = j;
            }
            if !current.is_empty() {
                chunks.push(make_chunk(
                    &file_path, &lang_name, start_line, last_line, &current,
                ));
            }
        }
    }

    if chunks.is_empty() {
        let text = String::from_utf8_lossy(&src).into_owned();
        let lines: Vec<&str> = text.lines().collect();
        let mut i = 0usize;
        while i < lines.len() {
            let mut buf = String::new();
            let start = i as u32 + 1;
            while i < lines.len() && buf.len() + lines[i].len() < max_chunk {
                buf.push_str(lines[i]);
                buf.push('\n');
                i += 1;
            }
            if buf.is_empty() {
                buf = lines[i].to_string();
                i += 1;
            }
            let end = i as u32;
            chunks.push(make_chunk(&file_path, &lang_name, start, end, &buf));
        }
    }

    Ok(ParseResult {
        chunks,
        symbols,
        imports,
        lang: lang_name,
    })
}

fn parse_generic(path: &Path, src: &[u8]) -> ParseResult {
    let text = String::from_utf8_lossy(src);
    let file_path = path.to_string_lossy().into_owned();

    let fn_patterns = [
        r"(?m)^\s*(?:pub\s+)?(?:async\s+)?(?:def|func|function|fn|sub|procedure)\s+(\w+)\s*[\(\{]",
        r"(?m)^\s*(?:public|private|protected|static|final|abstract|override)(?:\s+\w+)*\s+\w+\s+(\w+)\s*\(",
    ];
    let class_patterns = [
        r"(?m)^\s*(?:pub\s+)?(?:abstract\s+)?(?:class|interface|trait|struct|enum|object)\s+(\w+)",
    ];

    let mut symbols = vec![];
    for pattern in &fn_patterns {
        if let Ok(re) = Regex::new(pattern) {
            for cap in re.captures_iter(&text) {
                if let Some(name) = cap.get(1) {
                    let line =
                        text[..name.start()].chars().filter(|&c| c == '\n').count() as u32 + 1;
                    symbols.push(SymbolRecord {
                        name: name.as_str().to_string(),
                        kind: "fn".into(),
                        file_path: file_path.clone(),
                        line,
                        line_end: None,
                        qualname: None,
                        signature: None,
                        doc_comment: None,
                    });
                }
            }
        }
    }
    for pattern in &class_patterns {
        if let Ok(re) = Regex::new(pattern) {
            for cap in re.captures_iter(&text) {
                if let Some(name) = cap.get(1) {
                    let line =
                        text[..name.start()].chars().filter(|&c| c == '\n').count() as u32 + 1;
                    symbols.push(SymbolRecord {
                        name: name.as_str().to_string(),
                        kind: "class".into(),
                        file_path: file_path.clone(),
                        line,
                        line_end: None,
                        qualname: None,
                        signature: None,
                        doc_comment: None,
                    });
                }
            }
        }
    }

    let lines: Vec<&str> = text.lines().collect();
    let mut chunks = vec![];
    let max_chunk = 1000usize;
    let mut i = 0;
    while i < lines.len() {
        let mut buf = String::new();
        let start = i as u32 + 1;
        while i < lines.len() && buf.len() + lines[i].len() < max_chunk {
            buf.push_str(lines[i]);
            buf.push('\n');
            i += 1;
        }
        if buf.is_empty() {
            buf = lines[i].to_string();
            i += 1;
        }
        let end = i as u32;
        chunks.push(make_chunk(&file_path, "generic", start, end, &buf));
    }

    ParseResult {
        chunks,
        symbols,
        imports: vec![],
        lang: "generic".into(),
    }
}

fn make_chunk(
    file_path: &str,
    lang: &str,
    start_line: u32,
    end_line: u32,
    text: &str,
) -> CodeChunk {
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    let hash = format!("{:x}", h.finalize());
    CodeChunk {
        file_path: file_path.to_string(),
        start_line,
        end_line,
        lang: lang.to_string(),
        text: text.to_string(),
        content_hash: hash,
    }
}
