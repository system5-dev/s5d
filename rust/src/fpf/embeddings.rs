//! Local embedding inference via Qwen3-Embedding-0.6B GGUF.
//!
//! Uses llama-cpp-2 with Metal GPU acceleration. Downloads model on first use.
//! Produces 1024-dimensional normalized vectors for semantic similarity search.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use log::{info, warn};

use llama_cpp_2::context::params::{LlamaContextParams, LlamaPoolingType};
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};

const MODEL_NAME: &str = "Qwen3-Embedding-0.6B-Q8_0";
const MODEL_URL: &str = "https://huggingface.co/Qwen/Qwen3-Embedding-0.6B-GGUF/resolve/main/Qwen3-Embedding-0.6B-Q8_0.gguf";

fn models_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".s5d")
        .join("models")
}

fn model_path() -> PathBuf {
    models_dir().join(format!("{MODEL_NAME}.gguf"))
}

/// Download model from HuggingFace if not present.
pub fn ensure_model() -> anyhow::Result<PathBuf> {
    let path = model_path();
    if path.exists() {
        return Ok(path);
    }

    std::fs::create_dir_all(models_dir())?;
    eprintln!("Downloading {MODEL_NAME} (~640MB)...");

    let client = reqwest::blocking::Client::builder()
        .user_agent("s5d-cli")
        .timeout(std::time::Duration::from_secs(600))
        .build()?;

    let resp = client.get(MODEL_URL).send()?;
    if !resp.status().is_success() {
        anyhow::bail!("Download failed: HTTP {}", resp.status());
    }

    let bytes = resp.bytes()?;
    std::fs::write(&path, &bytes)?;
    eprintln!("Model saved: {} ({:.0} MB)", path.display(), bytes.len() as f64 / 1e6);
    Ok(path)
}

/// A loaded GGUF embedding model for semantic similarity.
///
/// Uses LlamaPoolingType::Last (required by Qwen3-Embedding).
/// Produces fixed-dimension vectors (1024 for Qwen3-Embedding-0.6B).
pub struct EmbeddingBrain {
    ctx: Mutex<std::mem::ManuallyDrop<Box<llama_cpp_2::context::LlamaContext<'static>>>>,
    model: std::mem::ManuallyDrop<Box<LlamaModel>>,
    backend: std::mem::ManuallyDrop<Box<LlamaBackend>>,
    n_embd: usize,
    n_ctx: u32,
}

// SAFETY: LlamaContext is not Send; we enforce exclusive access via Mutex.
unsafe impl Send for EmbeddingBrain {}
unsafe impl Sync for EmbeddingBrain {}

impl Drop for EmbeddingBrain {
    fn drop(&mut self) {
        unsafe {
            std::mem::ManuallyDrop::drop(self.ctx.get_mut().unwrap_or_else(|p| p.into_inner()));
            std::mem::ManuallyDrop::drop(&mut self.model);
            std::mem::ManuallyDrop::drop(&mut self.backend);
        }
    }
}

impl EmbeddingBrain {
    /// Load an embedding model from disk. Returns `None` if loading fails.
    pub fn load(model_path: &Path) -> Option<Self> {
        if !model_path.exists() {
            warn!("embedding model not found: {}", model_path.display());
            return None;
        }

        let backend = Box::new(LlamaBackend::init().ok()?);
        let backend_ptr = backend.as_ref() as *const LlamaBackend;

        let model_params = LlamaModelParams::default().with_n_gpu_layers(99);
        let model = Box::new(unsafe {
            LlamaModel::load_from_file(&*backend_ptr, model_path, &model_params).ok()?
        });

        let n_embd = model.n_embd() as usize;
        let model_ptr = model.as_ref() as *const LlamaModel;

        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(std::num::NonZeroU32::new(512))
            .with_embeddings(true)
            .with_pooling_type(LlamaPoolingType::Last);

        let ctx = unsafe {
            let backend_static: &'static LlamaBackend = &*backend_ptr;
            Box::new((*model_ptr).new_context(backend_static, ctx_params).ok()?)
        };

        info!("embedding model loaded: {} (dim={})", model_path.display(), n_embd);

        Some(Self {
            ctx: Mutex::new(std::mem::ManuallyDrop::new(ctx)),
            model: std::mem::ManuallyDrop::new(model),
            backend: std::mem::ManuallyDrop::new(backend),
            n_embd,
            n_ctx: 512,
        })
    }

    /// Load the default Qwen3-Embedding model from ~/.s5d/models/.
    pub fn load_default() -> Option<Self> {
        Self::load(&model_path())
    }

    /// Embedding dimension (1024 for Qwen3-Embedding-0.6B).
    pub fn dimension(&self) -> usize {
        self.n_embd
    }

    /// Embed a single text. Returns a normalized f32 vector.
    pub fn embed(&self, text: &str) -> Option<Vec<f32>> {
        let mut ctx_guard = self.ctx.lock().unwrap();
        let ctx: &mut llama_cpp_2::context::LlamaContext = &mut ctx_guard;

        let max_tokens = self.n_ctx as usize - 2;
        let mut tokens = self.model.str_to_token(text, AddBos::Never).ok()?;
        tokens.truncate(max_tokens);

        // Qwen3-Embedding requires EOS token appended
        tokens.push(self.model.token_eos());

        let mut batch = LlamaBatch::new(tokens.len(), 1);
        for (i, &token) in tokens.iter().enumerate() {
            let is_last = i == tokens.len() - 1;
            if batch.add(token, i as i32, &[0], is_last).is_err() {
                return None;
            }
        }

        if ctx.decode(&mut batch).is_err() {
            warn!("embedding: decode failed");
            return None;
        }

        let emb = ctx.embeddings_seq_ith(0).ok()?;
        Some(normalize(emb))
    }

    /// Embed multiple texts sequentially.
    pub fn embed_batch(&self, texts: &[&str]) -> Option<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text)?);
        }
        Some(results)
    }
}

// ── Vector helpers ──────────────────────────────────────────────────────────

/// Cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;
    for (x, y) in a.iter().zip(b.iter()) {
        let (x, y) = (*x as f64, *y as f64);
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < 1e-10 { 0.0 } else { dot / denom }
}

/// Serialize f32 vector to bytes (little-endian).
pub fn vectors_to_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Deserialize bytes back to f32 vector.
pub fn bytes_to_vectors(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// L2-normalize a vector.
fn normalize(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm < 1e-10 { v.to_vec() } else { v.iter().map(|x| x / norm).collect() }
}
