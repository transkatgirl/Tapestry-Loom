use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};

use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use tapestry_weave::v1::dependent::TapestryWeave;
use tokio::runtime::Runtime;
use ulid::Ulid;

mod seriate;

use crate::common::view::Edit;

pub struct InferenceEngine {
    settings: Rc<RefCell<InferenceEngineSettings>>,
    runtime: Arc<Runtime>,
    client: Client,
}

impl InferenceEngine {
    pub fn new(
        runtime: Arc<Runtime>,
        settings: Rc<RefCell<InferenceEngineSettings>>,
    ) -> Result<Self, anyhow::Error> {
        Ok(Self {
            settings,
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
    pub fn generate_children(&mut self, document: Ulid, weave: &mut TapestryWeave, id: u64) {
        // TODO
    }
    pub fn seriate_siblings(&mut self, document: Ulid, weave: &mut TapestryWeave, id: u64) {
        // TODO
    }
    pub fn update(&mut self, document: Ulid, weave: &mut Option<TapestryWeave>) {}
    pub fn cancel(&mut self, document: Ulid) {}
}

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct InferenceEngineSettings {}

impl Edit for InferenceEngineSettings {
    fn ui(&mut self, ui: &mut eframe::egui::Ui) {}
}
