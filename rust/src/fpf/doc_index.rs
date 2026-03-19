//! Document indexer for markdown spec files (FPF, docs, etc.).
//! Chunks by heading, stores in SQLite FTS5, supports keyword search.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

/// A single section chunk from a markdown document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocChunk {
    pub id: i64,
    /// The heading text (e.g. "A.1.3 - Solution")
    pub heading: String,
    /// Breadcrumb of parent headings (e.g. "Part A > A.1")
    pub heading_path: String,
    pub content: String,
    pub char_offset: usize,
    pub doc_version: String,
}

/// A ranked search result.
#[derive(Debug, Clone)]
pub struct DocResult {
    pub chunk: DocChunk,
    pub score: f64,
    /// Context snippet with matched terms highlighted as **term**
    pub snippet: String,
}

/// FTS5-backed document index stored in a dedicated SQLite database.
pub struct DocIndex {
    db_path: PathBuf,
}

impl DocIndex {
    pub fn new(db_path: &Path) -> Self {
        Self {
            db_path: db_path.to_path_buf(),
        }
    }

    fn open(&self) -> Result<rusqlite::Connection> {
        let conn = rusqlite::Connection::open(&self.db_path)
            .with_context(|| format!("Opening {:?}", self.db_path))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        Ok(conn)
    }

    /// Create schema if not present.
    pub fn init(&self) -> Result<()> {
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = self.open()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS doc_chunks (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                heading       TEXT    NOT NULL,
                heading_path  TEXT    NOT NULL DEFAULT '',
                content       TEXT    NOT NULL,
                char_offset   INTEGER NOT NULL DEFAULT 0,
                doc_version   TEXT    NOT NULL DEFAULT ''
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS doc_fts USING fts5(
                heading,
                heading_path,
                content,
                content=doc_chunks,
                content_rowid=id,
                tokenize='unicode61'
            );
            CREATE TABLE IF NOT EXISTS doc_meta (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            -- Embedding storage for hybrid search
            CREATE TABLE IF NOT EXISTS doc_embeddings (
                chunk_id  INTEGER PRIMARY KEY REFERENCES doc_chunks(id),
                vector    BLOB NOT NULL
            );",
        )?;
        Ok(())
    }

    /// Index a markdown document chunked by headings.
    /// Replaces any previously indexed content.
    pub fn index(&self, content: &str, version: &str) -> Result<usize> {
        let chunks = chunk_markdown(content);
        let n = chunks.len();
        let mut conn = self.open()?;
        let tx = conn.transaction()?;

        tx.execute_batch(
            "DELETE FROM doc_fts;
             DELETE FROM doc_chunks;",
        )?;

        for chunk in &chunks {
            let id: i64 = tx.query_row(
                "INSERT INTO doc_chunks (heading, heading_path, content, char_offset, doc_version)
                 VALUES (?1, ?2, ?3, ?4, ?5) RETURNING id",
                rusqlite::params![
                    chunk.heading,
                    chunk.heading_path,
                    chunk.content,
                    chunk.char_offset as i64,
                    version,
                ],
                |row| row.get(0),
            )?;
            tx.execute(
                "INSERT INTO doc_fts(rowid, heading, heading_path, content)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![id, chunk.heading, chunk.heading_path, chunk.content],
            )?;
        }

        tx.execute(
            "INSERT OR REPLACE INTO doc_meta VALUES ('doc_version', ?1)",
            rusqlite::params![version],
        )?;

        tx.commit()?;
        Ok(n)
    }

    /// BM25 full-text search. Returns up to `limit` results.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<DocResult>> {
        let conn = self.open()?;
        let fts_query = to_fts_query(query);

        let mut stmt = conn.prepare(
            "SELECT
                 c.id, c.heading, c.heading_path, c.content,
                 c.char_offset, c.doc_version,
                 bm25(doc_fts) AS score,
                 snippet(doc_fts, 2, '>>>', '<<<', '...', 48) AS snip
             FROM doc_fts
             JOIN doc_chunks c ON doc_fts.rowid = c.id
             WHERE doc_fts MATCH ?1
             ORDER BY score
             LIMIT ?2",
        )?;

        let results = stmt
            .query_map(rusqlite::params![fts_query, limit as i64], |row| {
                let chunk = DocChunk {
                    id: row.get(0)?,
                    heading: row.get(1)?,
                    heading_path: row.get(2)?,
                    content: row.get(3)?,
                    char_offset: row.get::<_, i64>(4)? as usize,
                    doc_version: row.get(5)?,
                };
                let score: f64 = row.get::<_, f64>(6)?.abs(); // bm25 is negative
                let snippet: String = row.get(7)?;
                Ok(DocResult {
                    chunk,
                    score,
                    snippet,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Exact section lookup by partial heading match (case-insensitive).
    pub fn get_section(&self, heading_query: &str) -> Result<Option<DocChunk>> {
        let conn = self.open()?;
        let pattern = format!("%{}%", heading_query.to_lowercase());
        conn.query_row(
            "SELECT id, heading, heading_path, content, char_offset, doc_version
             FROM doc_chunks
             WHERE LOWER(heading) LIKE ?1
             ORDER BY LENGTH(heading) ASC
             LIMIT 1",
            rusqlite::params![pattern],
            |row| {
                Ok(DocChunk {
                    id: row.get(0)?,
                    heading: row.get(1)?,
                    heading_path: row.get(2)?,
                    content: row.get(3)?,
                    char_offset: row.get::<_, i64>(4)? as usize,
                    doc_version: row.get(5)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    /// Stored version string (empty if not indexed).
    pub fn stored_version(&self) -> Option<String> {
        let conn = self.open().ok()?;
        conn.query_row(
            "SELECT value FROM doc_meta WHERE key = 'doc_version'",
            [],
            |row| row.get(0),
        )
        .ok()
    }

    pub fn chunk_count(&self) -> usize {
        let Ok(conn) = self.open() else { return 0 };
        conn.query_row("SELECT COUNT(*) FROM doc_chunks", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap_or(0) as usize
    }

    /// Store embeddings for chunks (by chunk id).
    pub fn store_embeddings(&self, pairs: &[(i64, Vec<u8>)]) -> Result<()> {
        let mut conn = self.open()?;
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM doc_embeddings", [])?;
        for (chunk_id, blob) in pairs {
            tx.execute(
                "INSERT INTO doc_embeddings (chunk_id, vector) VALUES (?1, ?2)",
                rusqlite::params![chunk_id, blob],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Load all embeddings: returns (chunk_id, vector_bytes).
    pub fn load_embeddings(&self) -> Result<Vec<(i64, Vec<u8>)>> {
        let conn = self.open()?;
        let mut stmt = conn.prepare("SELECT chunk_id, vector FROM doc_embeddings")?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// Check if embeddings are indexed.
    pub fn has_embeddings(&self) -> bool {
        let Ok(conn) = self.open() else { return false };
        conn.query_row("SELECT COUNT(*) FROM doc_embeddings", [], |r| r.get::<_, i64>(0))
            .unwrap_or(0)
            > 0
    }

    /// Get all chunks (id + content) for embedding.
    pub fn all_chunks(&self) -> Result<Vec<(i64, String)>> {
        let conn = self.open()?;
        let mut stmt = conn.prepare("SELECT id, heading || '\n' || content FROM doc_chunks")?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }
}

// ─── Markdown chunker ────────────────────────────────────────────────────────

/// Chunk markdown content by headings.
/// Each chunk: heading text + body until the next same/higher-level heading.
pub fn chunk_markdown(content: &str) -> Vec<DocChunk> {
    let mut chunks: Vec<DocChunk> = Vec::new();
    // Stack of (level, text) for ancestors
    let mut parent_stack: Vec<(usize, String)> = Vec::new();

    let mut cur_heading = String::new();
    let mut cur_path = String::new();
    let mut cur_body = String::new();
    let mut cur_offset: usize = 0;
    let mut offset: usize = 0;

    for line in content.lines() {
        let line_offset = offset;
        offset += line.len() + 1;

        if let Some(level) = heading_level(line) {
            // Flush previous chunk
            if !cur_heading.is_empty() {
                let body = cur_body.trim().to_string();
                if !body.is_empty() {
                    chunks.push(DocChunk {
                        id: 0,
                        heading: cur_heading.clone(),
                        heading_path: cur_path.clone(),
                        content: body,
                        char_offset: cur_offset,
                        doc_version: String::new(),
                    });
                }
            }

            // Trim parent stack to ancestors only (strictly shallower)
            parent_stack.retain(|(l, _)| *l < level);

            // Build path from current parent stack (before pushing new heading)
            let path = parent_stack
                .iter()
                .map(|(_, t)| t.as_str())
                .collect::<Vec<_>>()
                .join(" > ");

            let heading_text = line[level + 1..].trim().to_string();

            // Push current heading onto stack for descendants
            parent_stack.push((level, heading_text.clone()));

            cur_heading = heading_text;
            cur_path = path;
            cur_body = String::new();
            cur_offset = line_offset;
        } else {
            cur_body.push_str(line);
            cur_body.push('\n');
        }
    }

    // Flush last chunk
    if !cur_heading.is_empty() {
        let body = cur_body.trim().to_string();
        if !body.is_empty() {
            chunks.push(DocChunk {
                id: 0,
                heading: cur_heading,
                heading_path: cur_path,
                content: body,
                char_offset: cur_offset,
                doc_version: String::new(),
            });
        }
    }

    chunks
}

fn heading_level(line: &str) -> Option<usize> {
    if !line.starts_with('#') {
        return None;
    }
    let level = line.chars().take_while(|&c| c == '#').count();
    if level > 0 && line.as_bytes().get(level) == Some(&b' ') {
        Some(level)
    } else {
        None
    }
}

/// Build a safe FTS5 query: wrap each word in double quotes, join with AND.
fn to_fts_query(query: &str) -> String {
    let parts: Vec<String> = query
        .split_whitespace()
        .filter(|w| !w.is_empty())
        .map(|w| format!("\"{}\"", w.replace('"', "''")))
        .collect();

    if parts.is_empty() {
        return String::new();
    }

    // Try AND first; if too restrictive the caller can re-query with OR
    parts.join(" AND ")
}
