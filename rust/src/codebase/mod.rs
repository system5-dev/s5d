pub mod annotations;
pub mod embeddings;
pub mod graph;
pub mod parser;
pub mod search;
pub mod storage;
pub mod suggest;
pub mod trace;

pub use storage::CodebaseIndex;

use std::path::{Path, PathBuf};

pub struct IndexConfig {
    pub db_path: PathBuf,
    pub embeddings_url: String,
    pub embeddings_model: String,
    pub max_chunk_chars: usize,
}

impl Default for IndexConfig {
    fn default() -> Self {
        IndexConfig {
            db_path: PathBuf::from(".s5d/codebase.db"),
            embeddings_url: "http://localhost:1234".into(),
            embeddings_model: "nomic-embed-code".into(),
            max_chunk_chars: 1000,
        }
    }
}

pub struct IndexStats {
    pub files_indexed: usize,
    pub chunks_indexed: usize,
    pub embeddings_generated: usize,
    pub symbols_found: usize,
}

/// Index all supported source files under `root` from scratch.
pub fn index(root: &Path, config: &IndexConfig) -> anyhow::Result<IndexStats> {
    let idx = CodebaseIndex::open(&config.db_path)?;
    let ollama =
        embeddings::EmbeddingsClient::new(&config.embeddings_url, &config.embeddings_model);
    let use_embeddings = ollama.is_available();

    let files = collect_source_files(root);
    let mut stats = IndexStats {
        files_indexed: 0,
        chunks_indexed: 0,
        embeddings_generated: 0,
        symbols_found: 0,
    };

    for file_path in &files {
        index_file(&idx, file_path, &ollama, use_embeddings, &mut stats)?;
    }

    Ok(stats)
}

/// Re-index only files changed since the last HEAD commit (git diff --name-only HEAD~1 HEAD).
/// Falls back to full index if git is unavailable.
pub fn update(root: &Path, config: &IndexConfig) -> anyhow::Result<IndexStats> {
    let changed = changed_files(root);

    let idx = CodebaseIndex::open(&config.db_path)?;
    let ollama =
        embeddings::EmbeddingsClient::new(&config.embeddings_url, &config.embeddings_model);
    let use_embeddings = ollama.is_available();
    let mut stats = IndexStats {
        files_indexed: 0,
        chunks_indexed: 0,
        embeddings_generated: 0,
        symbols_found: 0,
    };

    if changed.is_empty() {
        // Nothing changed or git unavailable — fall back to full index
        let files = collect_source_files(root);
        for file_path in &files {
            index_file(&idx, file_path, &ollama, use_embeddings, &mut stats)?;
        }
    } else {
        for rel in &changed {
            let abs = root.join(rel);
            if parser::detect_lang(&abs).is_none() {
                continue;
            }
            idx.delete_file(&abs.to_string_lossy())?;
            if abs.exists() {
                index_file(&idx, &abs, &ollama, use_embeddings, &mut stats)?;
            }
        }
    }

    Ok(stats)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn index_file(
    idx: &CodebaseIndex,
    file_path: &Path,
    ollama: &embeddings::EmbeddingsClient,
    use_embeddings: bool,
    stats: &mut IndexStats,
) -> anyhow::Result<()> {
    let result = match parser::parse_file(file_path) {
        Ok(r) => r,
        Err(_) => return Ok(()), // skip unparseable files
    };

    // Skip if file hash unchanged (idempotent)
    if !result.chunks.is_empty() {
        let first_hash = &result.chunks[0].content_hash;
        if let Ok(Some(stored)) = idx.get_file_hash(&result.chunks[0].file_path) {
            if &stored == first_hash && result.chunks.len() == 1 {
                return Ok(());
            }
        }
    }

    for chunk in &result.chunks {
        let embedding = if use_embeddings {
            ollama.embed(&chunk.text).ok()
        } else {
            None
        };
        let emb_ref = embedding.as_deref();
        idx.upsert_chunk(chunk, emb_ref)?;
        stats.chunks_indexed += 1;
        if emb_ref.is_some() {
            stats.embeddings_generated += 1;
        }
    }

    for sym in &result.symbols {
        idx.upsert_symbol(sym)?;
        stats.symbols_found += 1;
    }

    for imp in &result.imports {
        idx.upsert_import(imp)?;
    }

    stats.files_indexed += 1;
    Ok(())
}

fn collect_source_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_recursive(root, &mut files);
    files
}

fn collect_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(name, "target" | ".git" | "node_modules" | ".s5d" | "vendor") {
                continue;
            }
            collect_recursive(&path, out);
        } else if parser::detect_lang(&path).is_some() {
            out.push(path);
        }
    }
}

fn changed_files(root: &Path) -> Vec<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", "HEAD~1", "HEAD"])
        .current_dir(root)
        .output();
    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| PathBuf::from(l.trim()))
            .filter(|p| !p.as_os_str().is_empty())
            .collect(),
        _ => Vec::new(),
    }
}
