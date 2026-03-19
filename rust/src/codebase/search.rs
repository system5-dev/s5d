use std::collections::HashMap;

use crate::codebase::storage::CodebaseIndex;

pub enum SearchMode {
    Fts,
    Hybrid,
}

pub struct SearchResult {
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub lang: String,
    pub text: String,
    pub score: f64,
    pub source: String,
}

pub fn search(
    index: &CodebaseIndex,
    query: &str,
    mode: SearchMode,
    top_k: usize,
    query_embedding: Option<&[f32]>,
) -> anyhow::Result<Vec<SearchResult>> {
    // FTS results
    let fts_hits = index.fts_search(query, top_k * 2).unwrap_or_default();

    // Map rowid → rank for FTS (1-based position in result list)
    let mut rrf_scores: HashMap<i64, f64> = HashMap::new();
    for (rank, hit) in fts_hits.iter().enumerate() {
        let rrf = 1.0 / (60.0 + (rank + 1) as f64);
        *rrf_scores.entry(hit.rowid).or_insert(0.0) += rrf;
    }

    // Vector results if Hybrid mode and embedding provided
    if matches!(mode, SearchMode::Hybrid) {
        if let Some(qvec) = query_embedding {
            let all_embs = index.get_all_embeddings().unwrap_or_default();
            let mut scored: Vec<(i64, f64)> = all_embs
                .iter()
                .map(|(rowid, vec)| (*rowid, cosine(qvec, vec)))
                .collect();
            // sort descending by cosine
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            scored.truncate(top_k * 2);

            for (rank, (rowid, _)) in scored.iter().enumerate() {
                let rrf = 1.0 / (60.0 + (rank + 1) as f64);
                *rrf_scores.entry(*rowid).or_insert(0.0) += rrf;
            }
        }
    }

    // Sort merged results by RRF score descending
    let mut merged: Vec<(i64, f64)> = rrf_scores.into_iter().collect();
    merged.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    merged.truncate(top_k);

    let source_label = match mode {
        SearchMode::Fts => "fts",
        SearchMode::Hybrid => "rrf",
    };

    let mut results = Vec::new();
    for (rowid, score) in merged {
        if let Some(chunk) = index.get_chunk_by_rowid(rowid)? {
            results.push(SearchResult {
                file_path: chunk.file_path,
                start_line: chunk.start_line,
                end_line: chunk.end_line,
                lang: chunk.lang,
                text: chunk.text,
                score,
                source: source_label.to_string(),
            });
        }
    }

    Ok(results)
}

fn cosine(a: &[f32], b: &[f32]) -> f64 {
    let len = a.len().min(b.len());
    let mut dot = 0.0f64;
    let mut na = 0.0f64;
    let mut nb = 0.0f64;
    for i in 0..len {
        let af = a[i] as f64;
        let bf = b[i] as f64;
        dot += af * bf;
        na += af * af;
        nb += bf * bf;
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}
