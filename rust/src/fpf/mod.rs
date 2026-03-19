//! `s5d fpf` — FPF spec management and search.
//!
//! Syncs the First Principles Framework spec from GitHub, indexes it with
//! FTS5, and exposes keyword search + section lookup.

pub mod doc_index;
pub mod embeddings;

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use doc_index::DocIndex;

const FPF_SPEC_URL: &str = "https://raw.githubusercontent.com/ailev/FPF/main/FPF-Spec.md";
const FPF_COMMITS_URL: &str = "https://api.github.com/repos/ailev/FPF/commits/main";

fn fpf_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".s5d")
        .join("fpf")
}

fn fpf_db() -> PathBuf {
    fpf_dir().join("fpf.db")
}

fn fpf_spec() -> PathBuf {
    fpf_dir().join("spec.md")
}

fn fpf_version_file() -> PathBuf {
    fpf_dir().join("VERSION")
}

// ─── Public API (for MCP) ───────────────────────────────────────────────────

pub fn search(query: &str, limit: usize, full: bool) -> Result<Vec<SearchHit>> {
    ensure_indexed()?;
    let idx = DocIndex::new(&fpf_db());

    // Try hybrid search if embeddings are available
    if idx.has_embeddings() {
        if let Ok(hits) = hybrid_search(&idx, query, limit, full) {
            if !hits.is_empty() {
                return Ok(hits);
            }
        }
    }

    // Fallback: BM25 only
    bm25_search(&idx, query, limit, full)
}

fn bm25_search(idx: &DocIndex, query: &str, limit: usize, full: bool) -> Result<Vec<SearchHit>> {
    let words: Vec<&str> = query.split_whitespace().collect();
    let mut results = idx.search(query, limit)?;

    if results.is_empty() && words.len() > 1 {
        let or_query = words
            .iter()
            .map(|w| format!("\"{}\"", w))
            .collect::<Vec<_>>()
            .join(" OR ");
        results = idx.search(&or_query, limit)?;
    }

    Ok(results
        .into_iter()
        .map(|r| SearchHit {
            heading: r.chunk.heading,
            heading_path: r.chunk.heading_path,
            content: if full {
                r.chunk.content
            } else {
                r.snippet.replace(">>>", "**").replace("<<<", "**")
            },
            score: r.score,
        })
        .collect())
}

fn hybrid_search(idx: &DocIndex, query: &str, limit: usize, full: bool) -> Result<Vec<SearchHit>> {
    use std::collections::HashMap;

    // BM25 results
    let bm25_results = idx.search(query, limit * 2)?;
    let mut rrf_scores: HashMap<i64, f64> = HashMap::new();
    let mut chunk_map: HashMap<i64, doc_index::DocResult> = HashMap::new();

    for (rank, r) in bm25_results.into_iter().enumerate() {
        let rrf = 1.0 / (60.0 + (rank + 1) as f64);
        rrf_scores.insert(r.chunk.id, rrf);
        chunk_map.insert(r.chunk.id, r);
    }

    // Vector results
    let brain = embeddings::EmbeddingBrain::load_default();
    if let Some(brain) = brain {
        if let Some(query_vec) = brain.embed(query) {
            let stored = idx.load_embeddings().unwrap_or_default();
            let mut scored: Vec<(i64, f64)> = stored
                .iter()
                .map(|(id, blob)| {
                    let vec = embeddings::bytes_to_vectors(blob);
                    (*id, embeddings::cosine_similarity(&query_vec, &vec))
                })
                .collect();
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            scored.truncate(limit * 2);

            for (rank, (id, _)) in scored.iter().enumerate() {
                let rrf = 1.0 / (60.0 + (rank + 1) as f64);
                *rrf_scores.entry(*id).or_insert(0.0) += rrf;
            }
        }
    }

    // Merge and sort
    let mut merged: Vec<(i64, f64)> = rrf_scores.into_iter().collect();
    merged.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    merged.truncate(limit);

    // Build results — prefer chunk_map, fallback to DB lookup
    let conn_chunks = idx.all_chunks().unwrap_or_default();
    let all_chunks: HashMap<i64, String> = conn_chunks.into_iter().collect();

    let mut hits = Vec::new();
    for (id, score) in merged {
        if let Some(r) = chunk_map.get(&id) {
            hits.push(SearchHit {
                heading: r.chunk.heading.clone(),
                heading_path: r.chunk.heading_path.clone(),
                content: if full {
                    r.chunk.content.clone()
                } else {
                    r.snippet.replace(">>>", "**").replace("<<<", "**")
                },
                score,
            });
        } else if let Some(content) = all_chunks.get(&id) {
            hits.push(SearchHit {
                heading: content.lines().next().unwrap_or("").to_string(),
                heading_path: String::new(),
                content: if full { content.clone() } else { content.chars().take(200).collect() },
                score,
            });
        }
    }

    Ok(hits)
}

pub struct SearchHit {
    pub heading: String,
    pub heading_path: String,
    pub content: String,
    pub score: f64,
}

// ─── CLI handlers ───────────────────────────────────────────────────────────

pub fn cmd_sync(force: bool, embed: bool) -> Result<()> {
    std::fs::create_dir_all(fpf_dir())?;

    print!("Checking upstream FPF... ");
    let client = reqwest::blocking::Client::builder()
        .user_agent("s5d-cli")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let commit_json: serde_json::Value = client
        .get(FPF_COMMITS_URL)
        .send()
        .context("Failed to reach GitHub API")?
        .json()
        .context("Failed to parse GitHub API response")?;

    let latest_sha = commit_json["sha"]
        .as_str()
        .context("GitHub response missing 'sha' field")?
        .to_string();

    let current_sha = std::fs::read_to_string(fpf_version_file())
        .unwrap_or_default()
        .trim()
        .to_string();

    if !force && current_sha == latest_sha {
        println!("up to date.");
        let idx = DocIndex::new(&fpf_db());
        println!("  SHA:    {}", &latest_sha[..7]);
        println!("  Chunks: {}", idx.chunk_count());
        return Ok(());
    }

    if current_sha.is_empty() {
        println!("first sync...");
    } else {
        println!(
            "updating {} → {}",
            &current_sha[..7.min(current_sha.len())],
            &latest_sha[..7]
        );
    }

    print!("  Downloading FPF-Spec.md... ");
    let spec = client
        .get(FPF_SPEC_URL)
        .send()
        .context("Failed to download FPF spec")?
        .text()
        .context("Failed to read FPF spec response")?;
    println!("{} KB", spec.len() / 1024);

    std::fs::write(fpf_spec(), &spec)?;

    print!("  Indexing... ");
    let idx = DocIndex::new(&fpf_db());
    idx.init()?;
    let n = idx.index(&spec, &latest_sha)?;
    println!("{} chunks", n);

    std::fs::write(fpf_version_file(), &latest_sha)?;

    println!("Done. {} indexed ({} chunks).", &latest_sha[..7], n);

    if !embed {
        println!("  Embedding skipped (use --embed to enable hybrid search)");
        return Ok(());
    }

    // Embed chunks if model is available
    print!("  Embedding... ");
    match embeddings::ensure_model() {
        Ok(model_path) => {
            if let Some(brain) = embeddings::EmbeddingBrain::load(&model_path) {
                let chunks = idx.all_chunks()?;
                let texts: Vec<&str> = chunks.iter().map(|(_, t)| t.as_str()).collect();
                if let Some(vectors) = brain.embed_batch(&texts) {
                    let pairs: Vec<(i64, Vec<u8>)> = chunks
                        .iter()
                        .zip(vectors.iter())
                        .map(|((id, _), vec)| (*id, embeddings::vectors_to_bytes(vec)))
                        .collect();
                    idx.store_embeddings(&pairs)?;
                    println!(
                        "{} vectors stored ({}d, hybrid search enabled)",
                        pairs.len(),
                        brain.dimension()
                    );
                } else {
                    println!("embedding failed, BM25 only");
                }
            } else {
                println!("model load failed, BM25 only");
            }
        }
        Err(e) => println!("skipped ({}), BM25 only", e),
    }

    Ok(())
}

pub fn cmd_search(query_words: &[String], limit: usize, full: bool) -> Result<()> {
    if query_words.is_empty() {
        bail!("Query required. Usage: s5d fpf search <terms...>");
    }

    let query = query_words.join(" ");
    let hits = search(&query, limit, full)?;

    if hits.is_empty() {
        println!("No results for: {}", query);
        return Ok(());
    }

    for (i, h) in hits.iter().enumerate() {
        let breadcrumb = if h.heading_path.is_empty() {
            String::new()
        } else {
            format!(" — {}", h.heading_path)
        };
        println!("\n### {}. {}{}", i + 1, h.heading, breadcrumb);
        println!();
        println!("{}", h.content);
    }
    println!();
    Ok(())
}

pub fn cmd_section(heading_words: &[String]) -> Result<()> {
    if heading_words.is_empty() {
        bail!("Heading required. Usage: s5d fpf section <heading...>");
    }
    ensure_indexed()?;

    let query = heading_words.join(" ");
    let idx = DocIndex::new(&fpf_db());

    match idx.get_section(&query)? {
        Some(chunk) => {
            let path = if chunk.heading_path.is_empty() {
                String::new()
            } else {
                format!(" ({})", chunk.heading_path)
            };
            println!("## {}{}\n", chunk.heading, path);
            println!("{}", chunk.content);
        }
        None => println!("Section not found: {}", query),
    }
    Ok(())
}

pub fn cmd_status() -> Result<()> {
    let version = std::fs::read_to_string(fpf_version_file())
        .map(|s| {
            let s = s.trim().to_string();
            if s.len() >= 7 {
                s[..7].to_string()
            } else {
                s
            }
        })
        .unwrap_or_else(|_| "not synced".into());

    let spec_size = std::fs::metadata(fpf_spec())
        .map(|m| format!("{} KB", m.len() / 1024))
        .unwrap_or_else(|_| "not downloaded".into());

    let idx = DocIndex::new(&fpf_db());
    let chunks = if fpf_db().exists() {
        idx.chunk_count()
    } else {
        0
    };

    println!("FPF Index");
    println!("  Version : {}", version);
    println!("  Chunks  : {}", chunks);
    println!("  Spec    : {}", spec_size);
    println!("  DB      : {}", fpf_db().display());
    Ok(())
}

fn ensure_indexed() -> Result<()> {
    if !fpf_db().exists() || DocIndex::new(&fpf_db()).chunk_count() == 0 {
        bail!("FPF index not found. Run: s5d fpf sync");
    }
    Ok(())
}
