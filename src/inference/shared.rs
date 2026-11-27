use std::{collections::HashMap, hash::Hash, sync::Arc};

use anyhow::anyhow;
use linked_hash_map::LinkedHashMap;
use log::trace;
use parking_lot::Mutex;
use reqwest::Response;
use tapestry_weave::InnerNodeContent;
use tokio::sync::Mutex as AsyncMutex;
use ulid::Ulid;

use super::PromptSegment;

const EMBEDDING_CACHE_MAX_SIZE: usize = 512;
const TOKENIZATION_CACHE_MAX_SIZE: usize = 16384;

type EmbeddingCache = LinkedHashMap<Vec<u8>, Vec<f32>>;
type TokenizationCache = LinkedHashMap<Vec<u8>, Vec<u64>>;

#[derive(Clone, Default)]
pub struct InferenceCache {
    embeddings: Arc<Mutex<EmbeddingCache>>,
    tokens: Arc<Mutex<HashMap<Ulid, Arc<AsyncMutex<TokenizationCache>>>>>,
}

impl InferenceCache {
    pub fn clear_embeddings(&mut self) {
        self.embeddings = Arc::default();
    }
    pub fn retain_models(&self, mut keep: impl FnMut(&Ulid) -> bool) {
        self.tokens.lock().retain(|identifier, _| keep(identifier));
    }
    pub async fn embed(
        &self,
        bytes: Vec<u8>,
        embed: impl AsyncFnOnce(&[u8]) -> Result<Vec<f32>, anyhow::Error>,
    ) -> Result<Vec<f32>, anyhow::Error> {
        if let Some(embedding) = self.embeddings.lock().get_refresh(&bytes) {
            return Ok(embedding.clone());
        }

        let embedding = embed(&bytes).await?;

        insert_bounded(
            &mut self.embeddings.lock(),
            bytes,
            embedding.clone(),
            EMBEDDING_CACHE_MAX_SIZE,
        );

        Ok(embedding)
    }
    pub async fn tokenize(
        &self,
        identifier: Ulid,
        bytes: &[u8],
        tokenize: impl AsyncFnOnce(Vec<u8>) -> Result<Vec<u64>, anyhow::Error>,
    ) -> Result<Vec<u64>, anyhow::Error> {
        let model_cache = self.tokens.lock().entry(identifier).or_default().clone();

        let bytes_printable = String::from_utf8_lossy(bytes);

        let mut model_cache = model_cache.lock().await;

        if let Some(tokens) = model_cache.get_refresh(bytes) {
            trace!("Using cached tokenization of {:?}", bytes_printable);
            return Ok(tokens.clone());
        }

        trace!("Tokenizing {:?}", bytes_printable);

        let tokens = tokenize(bytes.to_owned()).await?;

        trace!("{:?} = {:?}", bytes_printable, tokens);

        insert_bounded(
            &mut model_cache,
            bytes.to_owned(),
            tokens.clone(),
            TOKENIZATION_CACHE_MAX_SIZE,
        );

        Ok(tokens)
    }
}

fn insert_bounded<K: Hash + Eq, V>(
    cache: &mut LinkedHashMap<K, V>,
    key: K,
    value: V,
    max_size: usize,
) {
    cache.insert(key, value);

    while cache.len() > max_size {
        cache.pop_front();
    }
}

#[derive(Debug, Clone)]
pub struct EndpointRequest {
    pub prefix: Vec<PromptSegment>,
    pub suffix: Option<Vec<PromptSegment>>,
    pub parameters: Vec<(String, String)>,
    pub model_identifier: Ulid,
}

#[derive(Debug)]
pub struct EndpointResponse {
    pub root: bool,
    pub content: InnerNodeContent,
    pub metadata: Vec<(String, String)>,
    pub seed: Option<u64>,
    pub system_fingerprint: Option<String>,
    pub finish_reason: Option<String>,
}

pub fn segment_bytes(segments: &[PromptSegment]) -> Vec<u8> {
    segments
        .iter()
        .flat_map(|segment| segment.bytes())
        .copied()
        .collect()
}

pub fn segment_text(segments: &[PromptSegment]) -> String {
    String::from_utf8_lossy(&segment_bytes(segments)).into_owned()
}

pub async fn error_for_status(response: Response) -> Result<Response, anyhow::Error> {
    let status = response.status();

    if status.is_success() {
        Ok(response)
    } else {
        Err(anyhow!(
            "HTTP {}: {}",
            status.as_u16(),
            response.text().await?
        ))
    }
}
