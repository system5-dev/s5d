use std::path::Path;

use rusqlite::{params, Connection};

use crate::codebase::parser::{CodeChunk, ImportEdge, SymbolRecord};

pub struct CodebaseIndex {
    pub(crate) conn: Connection,
}

pub struct FtsResult {
    pub rowid: i64,
    pub rank: f64,
}

pub struct StoredChunk {
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub lang: String,
    pub text: String,
}

impl CodebaseIndex {
    pub fn open(db_path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(db_path)?;
        let idx = CodebaseIndex { conn };
        idx.init_schema()?;
        Ok(idx)
    }

    pub fn init_schema(&self) -> anyhow::Result<()> {
        self.conn.execute_batch("
            PRAGMA journal_mode=WAL;

            CREATE TABLE IF NOT EXISTS chunks (
                id            INTEGER PRIMARY KEY,
                file_path     TEXT NOT NULL,
                start_line    INTEGER,
                end_line      INTEGER,
                lang          TEXT,
                text          TEXT,
                content_hash  TEXT,
                embedding     BLOB
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
                file_path  UNINDEXED,
                lang       UNINDEXED,
                start_line UNINDEXED,
                end_line   UNINDEXED,
                text,
                content_hash UNINDEXED,
                content='chunks',
                content_rowid='id'
            );

            CREATE TRIGGER IF NOT EXISTS chunks_ai AFTER INSERT ON chunks BEGIN
                INSERT INTO chunks_fts(rowid, file_path, lang, start_line, end_line, text, content_hash)
                VALUES (new.id, new.file_path, new.lang, new.start_line, new.end_line, new.text, new.content_hash);
            END;

            CREATE TRIGGER IF NOT EXISTS chunks_ad AFTER DELETE ON chunks BEGIN
                INSERT INTO chunks_fts(chunks_fts, rowid, file_path, lang, start_line, end_line, text, content_hash)
                VALUES ('delete', old.id, old.file_path, old.lang, old.start_line, old.end_line, old.text, old.content_hash);
            END;

            CREATE TRIGGER IF NOT EXISTS chunks_au AFTER UPDATE ON chunks BEGIN
                INSERT INTO chunks_fts(chunks_fts, rowid, file_path, lang, start_line, end_line, text, content_hash)
                VALUES ('delete', old.id, old.file_path, old.lang, old.start_line, old.end_line, old.text, old.content_hash);
                INSERT INTO chunks_fts(rowid, file_path, lang, start_line, end_line, text, content_hash)
                VALUES (new.id, new.file_path, new.lang, new.start_line, new.end_line, new.text, new.content_hash);
            END;

            CREATE TABLE IF NOT EXISTS symbols (
                id        INTEGER PRIMARY KEY,
                name      TEXT NOT NULL,
                kind      TEXT,
                file_path TEXT NOT NULL,
                line      INTEGER
            );

            CREATE TABLE IF NOT EXISTS trace_links (
                id            INTEGER PRIMARY KEY,
                spec_id       TEXT NOT NULL,
                artifact_kind TEXT NOT NULL,
                artifact_id   TEXT NOT NULL,
                file_path     TEXT NOT NULL,
                symbol_name   TEXT,
                line_start    INTEGER,
                line_end      INTEGER,
                source        TEXT NOT NULL,
                confidence    REAL NOT NULL,
                updated_at    TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_trace_spec ON trace_links(spec_id);
            CREATE INDEX IF NOT EXISTS idx_trace_artifact ON trace_links(artifact_kind, artifact_id);

            CREATE TABLE IF NOT EXISTS import_edges (
                id          INTEGER PRIMARY KEY,
                source_file TEXT NOT NULL,
                imported    TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_chunks_file   ON chunks(file_path);
            CREATE INDEX IF NOT EXISTS idx_symbols_name  ON symbols(name);
            CREATE INDEX IF NOT EXISTS idx_symbols_file  ON symbols(file_path);
        ")?;
        // Migration: add new columns if missing
        let has_line_end: bool = self
            .conn
            .prepare("SELECT line_end FROM symbols LIMIT 0")
            .is_ok();
        if !has_line_end {
            self.conn.execute_batch(
                "
                ALTER TABLE symbols ADD COLUMN line_end INTEGER;
                ALTER TABLE symbols ADD COLUMN qualname TEXT;
                ALTER TABLE symbols ADD COLUMN signature TEXT;
                ALTER TABLE symbols ADD COLUMN doc_comment TEXT;
            ",
            )?;
        }
        self.conn.execute_batch(
            "
            CREATE INDEX IF NOT EXISTS idx_symbols_qualname ON symbols(qualname);
        ",
        )?;
        Ok(())
    }

    pub fn upsert_chunk(&self, chunk: &CodeChunk, embedding: Option<&[f32]>) -> anyhow::Result<()> {
        // Check if a chunk with this hash already exists for this file/range.
        let existing: Option<i64> = self.conn.query_row(
            "SELECT id FROM chunks WHERE file_path = ?1 AND start_line = ?2 AND end_line = ?3 AND content_hash = ?4",
            params![chunk.file_path, chunk.start_line, chunk.end_line, chunk.content_hash],
            |row| row.get(0),
        ).ok();

        if existing.is_some() {
            return Ok(());
        }

        let emb_blob: Option<Vec<u8>> = embedding.map(|v| {
            let mut bytes = Vec::with_capacity(v.len() * 4);
            for f in v {
                bytes.extend_from_slice(&f.to_le_bytes());
            }
            bytes
        });

        self.conn.execute(
            "INSERT INTO chunks(file_path, start_line, end_line, lang, text, content_hash, embedding)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                chunk.file_path,
                chunk.start_line,
                chunk.end_line,
                chunk.lang,
                chunk.text,
                chunk.content_hash,
                emb_blob,
            ],
        )?;
        Ok(())
    }

    pub fn upsert_symbol(&self, sym: &SymbolRecord) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO symbols(name, kind, file_path, line, line_end, qualname, signature, doc_comment) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![sym.name, sym.kind, sym.file_path, sym.line, sym.line_end, sym.qualname, sym.signature, sym.doc_comment],
        )?;
        Ok(())
    }

    pub fn upsert_import(&self, edge: &ImportEdge) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO import_edges(source_file, imported) VALUES (?1, ?2)",
            params![edge.source_file, edge.imported],
        )?;
        Ok(())
    }

    pub fn delete_file(&self, file_path: &str) -> anyhow::Result<()> {
        self.conn.execute(
            "DELETE FROM chunks WHERE file_path = ?1",
            params![file_path],
        )?;
        self.conn.execute(
            "DELETE FROM symbols WHERE file_path = ?1",
            params![file_path],
        )?;
        self.conn.execute(
            "DELETE FROM import_edges WHERE source_file = ?1",
            params![file_path],
        )?;
        Ok(())
    }

    pub fn get_file_hash(&self, file_path: &str) -> anyhow::Result<Option<String>> {
        let result: Option<String> = self
            .conn
            .query_row(
                "SELECT content_hash FROM chunks WHERE file_path = ?1 LIMIT 1",
                params![file_path],
                |row| row.get(0),
            )
            .ok();
        Ok(result)
    }

    pub fn fts_search(&self, query: &str, top_k: usize) -> anyhow::Result<Vec<FtsResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT rowid, rank FROM chunks_fts WHERE chunks_fts MATCH ?1 ORDER BY rank LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![query, top_k as i64], |row| {
            Ok(FtsResult {
                rowid: row.get(0)?,
                rank: row.get(1)?,
            })
        })?;
        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn get_all_embeddings(&self) -> anyhow::Result<Vec<(i64, Vec<f32>)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, embedding FROM chunks WHERE embedding IS NOT NULL")?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            Ok((id, blob))
        })?;
        let mut out = Vec::new();
        for r in rows {
            let (id, blob) = r?;
            let floats = blob_to_f32(&blob);
            out.push((id, floats));
        }
        Ok(out)
    }

    pub fn get_chunk_by_rowid(&self, rowid: i64) -> anyhow::Result<Option<StoredChunk>> {
        let result = self.conn.query_row(
            "SELECT file_path, start_line, end_line, lang, text FROM chunks WHERE id = ?1",
            params![rowid],
            |row| {
                Ok(StoredChunk {
                    file_path: row.get(0)?,
                    start_line: row.get::<_, i64>(1)? as u32,
                    end_line: row.get::<_, i64>(2)? as u32,
                    lang: row.get(3)?,
                    text: row.get(4)?,
                })
            },
        );
        match result {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Count symbols in a specific file.
    pub fn symbols_in_file(&self, file_path: &str) -> anyhow::Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM symbols WHERE file_path = ?1",
            params![file_path],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    pub fn symbol_lookup(&self, name: &str) -> anyhow::Result<Vec<SymbolRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, kind, file_path, line, line_end, qualname, signature, doc_comment FROM symbols WHERE name = ?1",
        )?;
        let rows = stmt.query_map(params![name], |row| {
            Ok(SymbolRecord {
                name: row.get(0)?,
                kind: row.get(1)?,
                file_path: row.get(2)?,
                line: row.get::<_, i64>(3)? as u32,
                line_end: row.get::<_, Option<i64>>(4)?.map(|v| v as u32),
                qualname: row.get(5)?,
                signature: row.get(6)?,
                doc_comment: row.get(7)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn insert_trace_link(&self, link: &crate::models::TraceLink) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO trace_links(spec_id, artifact_kind, artifact_id, file_path, symbol_name, line_start, line_end, source, confidence, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![link.spec_id, link.artifact_kind, link.artifact_id, link.file_path, link.symbol_name, link.line_start, link.line_end, link.source, link.confidence, link.updated_at],
        )?;
        Ok(())
    }

    pub fn get_trace_links_for_spec(
        &self,
        spec_id: &str,
    ) -> anyhow::Result<Vec<crate::models::TraceLink>> {
        let mut stmt = self.conn.prepare(
            "SELECT spec_id, artifact_kind, artifact_id, file_path, symbol_name, line_start, line_end, source, confidence, updated_at FROM trace_links WHERE spec_id = ?1 ORDER BY artifact_kind, artifact_id",
        )?;
        let rows = stmt.query_map(params![spec_id], |row| {
            Ok(crate::models::TraceLink {
                spec_id: row.get(0)?,
                artifact_kind: row.get(1)?,
                artifact_id: row.get(2)?,
                file_path: row.get(3)?,
                symbol_name: row.get(4)?,
                line_start: row.get::<_, i64>(5)? as u32,
                line_end: row.get::<_, Option<i64>>(6)?.map(|v| v as u32),
                source: row.get(7)?,
                confidence: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn delete_trace_links_for_spec(&self, spec_id: &str) -> anyhow::Result<usize> {
        let count = self.conn.execute(
            "DELETE FROM trace_links WHERE spec_id = ?1",
            params![spec_id],
        )?;
        Ok(count)
    }

    pub fn get_import_edges(&self) -> anyhow::Result<Vec<ImportEdge>> {
        let mut stmt = self
            .conn
            .prepare("SELECT source_file, imported FROM import_edges")?;
        let rows = stmt.query_map([], |row| {
            Ok(ImportEdge {
                source_file: row.get(0)?,
                imported: row.get(1)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}

fn blob_to_f32(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect()
}
