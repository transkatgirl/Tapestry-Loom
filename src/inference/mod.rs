use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};

use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use tapestry_weave::{
    ShortId,
    universal_weave::{BookmarkableWeave, Weave},
    weave::wrappers::LoggedTapestryWeave,
};
use tokio::runtime::Runtime;
use ulid::Ulid;

mod seriate;

use crate::common::view::Edit;

pub struct InferenceEngine {
    settings: Rc<RefCell<InferenceEngineSettings>>,
    runtime: Arc<Runtime>,
    client: Client,
}

pub enum InferenceRequest {
    GenerateAfter(Option<ShortId>),
    SeriateChildren(Option<ShortId>),
    SeriateBookmarks,
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
    pub fn request(
        &mut self,
        document: Ulid,
        weave: &mut LoggedTapestryWeave,
        request: InferenceRequest,
    ) {
        match request {
            InferenceRequest::GenerateAfter(tail) => {
                let mut path = Vec::new();
                if let Some(tail) = tail {
                    weave.get_path_from(&tail, &mut path);
                }

                // TODO
            }
            InferenceRequest::SeriateChildren(node) => {
                let children: Box<dyn Iterator<Item = ShortId>> = if let Some(node) = node {
                    Box::new(weave.get_children(&node).unwrap().iter().copied())
                } else {
                    Box::new(weave.roots().iter().copied())
                };

                // TODO
            }
            InferenceRequest::SeriateBookmarks => {
                let bookmarks = weave.bookmarks();

                // TODO
            }
        }
    }
    pub fn update(
        &mut self,
        document: Ulid,
        weave: &mut Option<LoggedTapestryWeave>,
    ) -> Vec<ShortId> {
        let mut updated = Vec::new();

        updated
    }
    pub fn cancel(&mut self, document: Ulid) {}
}

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct InferenceEngineSettings {}

impl Edit for InferenceEngineSettings {
    fn ui(&mut self, ui: &mut eframe::egui::Ui) {}
}
