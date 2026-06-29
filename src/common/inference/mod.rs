use std::{sync::Arc, time::Duration};

use reqwest::{Client, ClientBuilder};
use tapestry_weave::v1::dependent::TapestryWeave;
use tokio::runtime::Runtime;
use ulid::Ulid;

pub struct InferenceEngine {
    runtime: Arc<Runtime>,
    client: Client,
}

impl InferenceEngine {
    pub fn new(runtime: Arc<Runtime>) -> Result<Self, anyhow::Error> {
        Ok(Self {
            runtime,
            client: ClientBuilder::new()
                .connect_timeout(Duration::from_secs(15))
                .timeout(Duration::from_secs(300))
                .build()?,
        })
    }
    pub fn requests(&self, document: Ulid) -> usize {
        0 // TODO
    }
    pub fn update(&mut self, document: Ulid, weave: &mut Option<TapestryWeave>) {}
    pub fn cancel(&mut self, document: Ulid) {}
}

pub struct InferenceEngineSettings {}
