use reqwest::blocking::Client;
use serde::Deserialize;

/// OpenAI-compatible embeddings client.
/// Works with LM Studio (localhost:1234), OpenAI, and any compatible server.
pub struct EmbeddingsClient {
    base_url: String,
    model: String,
    client: Client,
}

#[derive(Deserialize)]
struct EmbedData {
    embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct EmbedResponse {
    data: Vec<EmbedData>,
}

impl EmbeddingsClient {
    pub fn new(base_url: &str, model: &str) -> Self {
        EmbeddingsClient {
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let url = format!("{}/v1/embeddings", self.base_url);
        let body = serde_json::json!({
            "model": self.model,
            "input": text,
        });
        let resp = self.client.post(&url).json(&body).send()?;
        if !resp.status().is_success() {
            anyhow::bail!("Embeddings server returned {}", resp.status());
        }
        let parsed: EmbedResponse = resp.json()?;
        parsed
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| anyhow::anyhow!("Empty embedding response"))
    }

    pub fn is_available(&self) -> bool {
        let url = format!("{}/v1/models", self.base_url);
        self.client
            .get(&url)
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}
