//! Text embedding backend for the recommendation pipeline.
//!
//! The backend is an OpenAI-compatible HTTP embeddings endpoint (Ollama,
//! Text Embeddings Inference, OpenAI, etc.). Keeping it behind [`Embedder`]
//! means a local ONNX implementation can replace it without touching the
//! worker or the SQL layer. The default dimension matches the
//! `media_translations.embedding vector(768)` column.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmbedError {
  #[error("embedding endpoint is not configured (set EMBEDDING_URL)")]
  NotConfigured,
  #[error("invalid EMBEDDING_DIMENSIONS: {0}")]
  InvalidDimensions(#[from] std::num::ParseIntError),
  #[error("embedding request failed: {0}")]
  Request(#[from] reqwest::Error),
  #[error("embedding service returned {status}: {body}")]
  Service { status: u16, body: String },
  #[error("embedding response was malformed: {0}")]
  Malformed(String),
  #[error("expected {expected} dimensions, got {actual}")]
  Dimensions { expected: usize, actual: usize },
}

/// Converts a batch of texts into one vector per text, in the same order.
#[async_trait]
pub trait Embedder: Send + Sync {
  /// Vector length expected by the database column.
  fn dimensions(&self) -> usize;

  async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError>;
}

#[derive(Debug, Clone)]
pub struct HttpEmbedderConfig {
  /// Full embeddings endpoint, e.g. `http://127.0.0.1:11434/v1/embeddings`.
  pub endpoint: String,
  pub model: String,
  pub api_key: Option<String>,
  pub dimensions: usize,
}

impl HttpEmbedderConfig {
  /// `EMBEDDING_URL` is required; the rest have defaults.
  pub fn from_env() -> Result<Self, EmbedError> {
    let endpoint = std::env::var("EMBEDDING_URL").map_err(|_| EmbedError::NotConfigured)?;
    let model = std::env::var("EMBEDDING_MODEL").unwrap_or_else(|_| "bge-base-en-v1.5".to_string());
    let api_key = std::env::var("EMBEDDING_API_KEY").ok();
    let dimensions = std::env::var("EMBEDDING_DIMENSIONS")
      .unwrap_or_else(|_| "768".to_string())
      .parse()?;
    Ok(Self {
      endpoint,
      model,
      api_key,
      dimensions,
    })
  }
}

pub struct HttpEmbedder {
  client: reqwest::Client,
  config: HttpEmbedderConfig,
}

impl HttpEmbedder {
  pub fn new(config: HttpEmbedderConfig) -> Self {
    Self {
      client: reqwest::Client::new(),
      config,
    }
  }

  pub fn from_env() -> Result<Self, EmbedError> {
    Ok(Self::new(HttpEmbedderConfig::from_env()?))
  }
}

#[derive(Serialize)]
struct EmbeddingRequest<'a> {
  model: &'a str,
  input: &'a [String],
}

#[derive(Deserialize)]
struct EmbeddingResponse {
  data: Vec<EmbeddingDatum>,
}

#[derive(Deserialize)]
struct EmbeddingDatum {
  index: usize,
  embedding: Vec<f32>,
}

#[async_trait]
impl Embedder for HttpEmbedder {
  fn dimensions(&self) -> usize {
    self.config.dimensions
  }

  async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
    if texts.is_empty() {
      return Ok(Vec::new());
    }

    let mut request = self
      .client
      .post(&self.config.endpoint)
      .json(&EmbeddingRequest {
        model: &self.config.model,
        input: texts,
      });
    if let Some(key) = &self.config.api_key {
      request = request.bearer_auth(key);
    }

    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
      let body = response.text().await.unwrap_or_default();
      return Err(EmbedError::Service {
        status: status.as_u16(),
        body,
      });
    }
    let parsed: EmbeddingResponse = response.json().await?;

    let mut vectors = vec![Vec::new(); texts.len()];
    for datum in parsed.data {
      let slot = vectors
        .get_mut(datum.index)
        .ok_or_else(|| EmbedError::Malformed(format!("index {} out of range", datum.index)))?;
      if datum.embedding.len() != self.config.dimensions {
        return Err(EmbedError::Dimensions {
          expected: self.config.dimensions,
          actual: datum.embedding.len(),
        });
      }
      *slot = datum.embedding;
    }
    if vectors.iter().any(Vec::is_empty) {
      return Err(EmbedError::Malformed(format!(
        "expected {} vectors, got {}",
        texts.len(),
        vectors.iter().filter(|v| !v.is_empty()).count()
      )));
    }
    Ok(vectors)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  struct FixedEmbedder;

  #[async_trait]
  impl Embedder for FixedEmbedder {
    fn dimensions(&self) -> usize {
      3
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
      Ok(texts.iter().map(|t| vec![t.len() as f32; 3]).collect())
    }
  }

  #[tokio::test]
  async fn trait_returns_one_vector_per_input() {
    let embedder = FixedEmbedder;
    let vectors = embedder
      .embed(&["a".to_string(), "abcd".to_string()])
      .await
      .expect("embed");
    assert_eq!(vectors.len(), 2);
    assert_eq!(vectors[0].len(), embedder.dimensions());
    assert_eq!(vectors[0][0], 1.0);
    assert_eq!(vectors[1][0], 4.0);
  }
}
