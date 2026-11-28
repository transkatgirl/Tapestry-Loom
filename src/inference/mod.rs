use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use egui_notify::Toast;
use futures::future::try_join_all;
use log::{debug, error, warn};
use parking_lot::Mutex;
use reqwest::Client;
use tapestry_weave::{
    Creator, InnerNodeContent, InnerNodeToken, LongId, MetadataMap, Model, NodeContent, ShortId,
    TapestryNode,
    jiff::Zoned,
    nanorand::WyRand,
    universal_weave::{
        BookmarkableWeave, SortableBookmarkableWeave, SortableWeave, Weave, indexmap::IndexSet,
        wrappers::PositionAlignment,
    },
    weave::wrappers::LoggedTapestryWeave,
};
use tokio::{runtime::Runtime, task};
use ulid::Ulid;

mod openai;
mod parameters;
mod seriate;
mod settings;
mod shared;

pub use parameters::InferenceParameters;
pub use settings::InferenceEngineSettings;

use crate::{
    common::task::AbortOnDropHandle,
    inference::{
        settings::{ClientConfig, EmbeddingEndpointConfig},
        shared::{EndpointRequest, EndpointResponse, InferenceCache},
    },
};

pub struct InferenceEngine {
    settings: Rc<RefCell<InferenceEngineSettings>>,
    runtime: Arc<Runtime>,
    toasts: Arc<Mutex<Vec<Toast>>>,

    client: Client,
    client_config: ClientConfig,
    embedding_config: EmbeddingEndpointConfig,
    cache: InferenceCache,

    documents: HashMap<Ulid, DocumentRequests>,
}

#[derive(Default)]
struct DocumentRequests {
    generation: Vec<GenerationHandle>,
    seriation: Vec<SeriationHandle>,
}

impl DocumentRequests {
    fn len(&self) -> usize {
        self.generation.len() + self.seriation.len()
    }
}

#[derive(Clone)]
pub struct GenerationTarget {
    pub parent: Option<u64>,
    pub child: Option<u64>,
    pub prefix: Vec<PromptSegment>,
    pub suffix: Option<Vec<PromptSegment>>,
}

struct GenerationHandle {
    target: GenerationTarget,
    parameters: InferenceParameters,
    depth: usize,
    model: Model,

    task: AbortOnDropHandle<Result<Vec<EndpointResponse>, anyhow::Error>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeriationTarget {
    Children(Option<u64>),
    Bookmarks,
}

struct SeriationHandle {
    target: SeriationTarget,
    task: AbortOnDropHandle<Result<Vec<ShortId>, anyhow::Error>>,
}

pub enum InferenceRequest {
    Generate(GenerationTarget),
    Seriate(SeriationTarget),
}

impl InferenceRequest {
    pub fn generate(
        weave: &mut LoggedTapestryWeave,
        parent: Option<ShortId>,
        child: Option<ShortId>,
        prefix: Vec<PromptSegment>,
        suffix: Option<Vec<PromptSegment>>,
    ) -> Self {
        Self::Generate(GenerationTarget {
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
        })
    }
    pub fn seriate_children(weave: &mut LoggedTapestryWeave, parent: Option<ShortId>) -> Self {
        Self::Seriate(SeriationTarget::Children(parent.map(|id| {
            weave
                .get_or_insert_position(&id, PositionAlignment::End)
                .unwrap()
        })))
    }
    pub fn seriate_bookmarks() -> Self {
        Self::Seriate(SeriationTarget::Bookmarks)
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
        let (client, client_config, embedding_config) = {
            let settings = settings.borrow();

            (
                settings.client.build()?,
                settings.client.clone(),
                settings.embedding_model.clone(),
            )
        };

        Ok(Self {
            runtime,
            settings,
            toasts,
            client,
            client_config,
            embedding_config,
            cache: InferenceCache::default(),
            documents: HashMap::with_capacity(8),
        })
    }
    pub fn requests(&self, document: Ulid) -> usize {
        self.documents
            .get(&document)
            .map_or(0, DocumentRequests::len)
    }
    pub fn request(
        &mut self,
        document: Ulid,
        weave: &mut LoggedTapestryWeave,
        parameters: &InferenceParameters,
        request: InferenceRequest,
    ) {
        self.sync_settings();

        match request {
            InferenceRequest::Generate(target) => {
                if parameters.models.is_empty() {
                    warn!("Inference failed: No models loaded");
                    self.toasts
                        .lock()
                        .push(Toast::error("Inference failed: No models loaded"));
                    return;
                }

                self.generate(document, target, parameters, parameters.recursion_depth);
            }
            InferenceRequest::Seriate(target) => {
                let items: Vec<ShortId> = match &target {
                    SeriationTarget::Children(Some(parent)) => {
                        if let Some(node) = weave.get_position(parent, PositionAlignment::End) {
                            weave.get_children(&node).unwrap()
                        } else {
                            return;
                        }
                    }
                    SeriationTarget::Children(None) => weave.roots(),
                    SeriationTarget::Bookmarks => weave.bookmarks(),
                }
                .iter()
                .copied()
                .filter(|id| {
                    weave
                        .get_contents(id)
                        .is_some_and(|contents| contents.content.token_count() != Some(1))
                })
                .collect();

                if items.len() < 3 {
                    return;
                }

                self.seriate(
                    document,
                    target,
                    items
                        .into_iter()
                        .map(|id| {
                            (
                                id,
                                weave
                                    .path_content(&id)
                                    .flat_map(|contents| contents.content.iter_bytes())
                                    .collect(),
                            )
                        })
                        .collect(),
                );
            }
        }
    }
    pub fn update(
        &mut self,
        document: Ulid,
        weave: &mut LoggedTapestryWeave,
        rng: &mut WyRand,
    ) -> Vec<ShortId> {
        let mut updated = Vec::new();

        if let Some(requests) = self.documents.get_mut(&document) {
            for handle in requests
                .seriation
                .extract_if(.., |handle| handle.task.is_finished())
            {
                if let Some(order) = resolve_task(&self.runtime, &self.toasts, handle.task) {
                    let order: HashMap<ShortId, usize> = order
                        .into_iter()
                        .enumerate()
                        .map(|(index, id)| (id, index))
                        .collect();
                    let cmp = |a: &ShortId, b: &ShortId| order.get(a).cmp(&order.get(b));

                    match handle.target {
                        SeriationTarget::Children(Some(position)) => {
                            if let Some((parent, _)) = weave.resolve_position(position) {
                                weave.sort_children_by_id(&parent, cmp);
                            } else {
                                debug!("Discarded seration response due to unresolvable parent");
                            }
                        }
                        SeriationTarget::Children(None) => weave.sort_roots_by_id(cmp),
                        SeriationTarget::Bookmarks => weave.sort_bookmarks_by_id(cmp),
                    }
                }
            }

            let generation: Vec<_> = requests
                .generation
                .extract_if(.., |handle| handle.task.is_finished())
                .collect();

            for handle in generation {
                if let Some(responses) = resolve_task(&self.runtime, &self.toasts, handle.task) {
                    let parent = match handle.target.parent {
                        Some(position) => match weave.resolve_position(position) {
                            Some((id, _)) => Some(id),
                            None => {
                                debug!("Discarded generation response due to unresolvable parent");
                                continue;
                            }
                        },
                        None => None,
                    };
                    let child = match handle.target.child {
                        Some(position) => match weave.resolve_position(position) {
                            Some((id, _)) => Some(id),
                            None => {
                                debug!("Discarded generation response due to unresolvable child");
                                continue;
                            }
                        },
                        None => None,
                    };

                    for response in responses {
                        let id = tapestry_weave::generate_id(rng, weave);

                        if weave.insert_deduplicated(TapestryNode {
                            id,
                            from: IndexSet::from_iter(parent.filter(|_| !response.root)),
                            to: IndexSet::from_iter(child),
                            active: false,
                            bookmarked: false,
                            contents: NodeContent {
                                timestamp: Zoned::now(),
                                modified: false,
                                content: response.content,
                                metadata: MetadataMap::from_iter(response.metadata),
                                creator: Creator::Model(Some(Model {
                                    seed: response.seed,
                                    system_fingerprint: response.system_fingerprint,
                                    finish_reason: response.finish_reason,
                                    ..handle.model.clone()
                                })),
                            },
                        }) {
                            updated.push(id);

                            if handle.depth > 0
                                && !response.root
                                && handle.target.child.is_none()
                                && handle.target.suffix.is_none()
                            {
                                self.generate(
                                    document,
                                    GenerationTarget {
                                        parent: weave
                                            .get_or_insert_position(&id, PositionAlignment::End),
                                        child: None,
                                        prefix: handle
                                            .target
                                            .prefix
                                            .iter()
                                            .cloned()
                                            .chain(PromptSegment::from_contents(
                                                weave.get_contents(&id),
                                            ))
                                            .collect(),
                                        suffix: None,
                                    },
                                    &handle.parameters,
                                    handle.depth - 1,
                                );
                            }
                        } else {
                            debug!("Failed to insert generated node");
                        }
                    }
                }
            }

            if self.requests(document) == 0 {
                self.documents.remove(&document);
            }
        }

        updated
    }
    pub fn cancel(&mut self, document: Ulid) {
        self.documents.remove(&document);
    }
    pub fn cleanup(&self, document: Ulid, weave: &mut LoggedTapestryWeave) {
        if self.requests(document) == 0 {
            weave.clear_positions();
        }
    }
    fn sync_settings(&mut self) {
        let mut settings = self.settings.borrow_mut();

        if self.client_config != settings.client {
            match settings.client.build() {
                Ok(client) => {
                    self.client = client;
                    self.client_config = settings.client.clone()
                }
                Err(error) => {
                    settings.client = self.client_config.clone();

                    error!("HTTP client initialization failed: {error:#?}");
                    self.toasts
                        .lock()
                        .push(Toast::error("HTTP client initialization failed"));
                }
            }
        }

        if self.embedding_config != settings.embedding_model {
            self.cache.clear_embeddings();
            self.embedding_config = settings.embedding_model.clone();
        }

        let models: HashSet<Ulid> = settings
            .models
            .values()
            .map(|model| model.identifier)
            .collect();

        self.cache
            .retain_models(|identifier| models.contains(identifier));
    }
}

fn resolve_task<T>(
    runtime: &Runtime,
    toasts: &Mutex<Vec<Toast>>,
    task: AbortOnDropHandle<Result<T, anyhow::Error>>,
) -> Option<T> {
    match runtime.block_on(task) {
        Ok(Ok(output)) => Some(output),
        Ok(Err(error)) => {
            warn!("Inference failed: {error:#?}");
            toasts
                .lock()
                .push(Toast::error(format!("Inference failed: {error:#}")));
            None
        }
        Err(error) => {
            if !error.is_cancelled() {
                error!("Background task failed: {error:?}");
                warn!("Inference failed: Background task failed");
                toasts
                    .lock()
                    .push(Toast::error("Inference failed: Background task failed"));
            }
            None
        }
    }
}
