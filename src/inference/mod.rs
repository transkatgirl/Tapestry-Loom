use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};

use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use tapestry_weave::{
    Creator, InnerNodeContent, InnerNodeToken, LongId, Model, NodeContent, ShortId,
    nanorand::WyRand,
    universal_weave::{BookmarkableWeave, Weave, wrappers::PositionAlignment},
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
    Generate {
        parent: Option<u64>,
        child: Option<u64>,
        prefix: Vec<PromptSegment>,
        suffix: Option<Vec<PromptSegment>>,
    },
    SeriateChildren(Option<u64>),
    SeriateBookmarks,
}

impl InferenceRequest {
    pub fn generate(
        weave: &mut LoggedTapestryWeave,
        parent: Option<ShortId>,
        child: Option<ShortId>,
        prefix: Vec<PromptSegment>,
        suffix: Option<Vec<PromptSegment>>,
    ) -> Self {
        Self::Generate {
            parent: parent.map(|id| {
                weave
                    .get_or_insert_position(&id, PositionAlignment::End)
                    .unwrap()
            }),
            child: child.map(|id| {
                weave
                    .get_or_insert_position(&id, PositionAlignment::Start)
                    .unwrap()
            }),
            prefix,
            suffix,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptSegment {
    Bytes(Vec<u8>),
    Tokens {
        model: LongId,
        ids: Vec<u64>,
        bytes: Vec<u8>,
    },
}

impl PromptSegment {
    pub fn from_contents<'a>(contents: impl IntoIterator<Item = &'a NodeContent>) -> Vec<Self> {
        let mut segments = Vec::new();

        for contents in contents {
            match &contents.content {
                InnerNodeContent::Snippet(snippet) => segments.push(Self::Bytes(snippet.clone())),
                InnerNodeContent::Tokens(tokens) => {
                    if let Creator::Model(Some(Model {
                        identifier: Some(model),
                        ..
                    })) = contents.creator
                    {
                        let is_usable =
                            |token: &InnerNodeToken| token.id.is_some() && !token.is_modified();

                        segments.extend(tokens.chunk_by(|a, b| is_usable(a) == is_usable(b)).map(
                            |run| {
                                let bytes =
                                    run.iter().flat_map(|token| &token.bytes).copied().collect();

                                if is_usable(&run[0]) {
                                    Self::Tokens {
                                        model,
                                        ids: run.iter().filter_map(|token| token.id).collect(),
                                        bytes,
                                    }
                                } else {
                                    Self::Bytes(bytes)
                                }
                            },
                        ));
                    } else {
                        segments.push(Self::Bytes(
                            tokens
                                .iter()
                                .flat_map(|token| &token.bytes)
                                .copied()
                                .collect(),
                        ));
                    }
                }
                InnerNodeContent::MetadataOnly => {}
            }
        }

        segments.retain(|segment| !segment.is_empty());
        segments
    }
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Bytes(bytes) => bytes.is_empty(),
            Self::Tokens { ids, .. } => ids.is_empty(),
        }
    }
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::Bytes(bytes) | Self::Tokens { bytes, .. } => bytes,
        }
    }
}

impl InferenceEngine {
    pub fn new(
        runtime: Arc<Runtime>,
        settings: Rc<RefCell<InferenceEngineSettings>>,
        toasts: Arc<Mutex<Vec<Toast>>>,
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
            InferenceRequest::Generate {
                parent,
                child,
                prefix,
                suffix,
            } => {
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
        rng: &mut WyRand,
    ) -> Vec<ShortId> {
        let updated = Vec::new();

        updated
    }
    pub fn cancel(&mut self, document: Ulid) {
        // TODO
    }
    pub fn cleanup(&self, document: Ulid, weave: &mut LoggedTapestryWeave) {
        if self.requests(document) == 0 {
            weave.clear_positions();
        }
    }
}

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct InferenceEngineSettings {}

impl Edit for InferenceEngineSettings {
    fn ui(&mut self, ui: &mut eframe::egui::Ui) {}
}
