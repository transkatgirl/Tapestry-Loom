use std::{borrow::Cow, collections::HashSet, hash::BuildHasherDefault};

use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use contracts::ensures;
use rustc_hash::FxBuildHasher;
use ulid::Ulid;
use universal_weave::{
    DeduplicatableContents, DeduplicatableWeave, DiscreteContentResult, DiscreteContents,
    DiscreteWeave, Weave,
    dependent::{DependentNode, legacy_dependent::DependentWeave},
    indexmap::{IndexMap, IndexSet},
    rkyv::{
        Archive, Deserialize, Serialize, from_bytes, rancor::Error, to_bytes, util::AlignedVec,
    },
};

use crate::{
    VersionedWeave,
    hashers::UlidHasher,
    versioning::{MixedData, VersionedBytes},
};

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct NodeContent {
    pub content: InnerNodeContent,
    pub metadata: MetadataMap,
    pub model: Option<Model>,
}

impl DiscreteContents for NodeContent {
    fn split(mut self, at: usize) -> DiscreteContentResult<Self> {
        match self.content.split(at) {
            DiscreteContentResult::Two((left, right)) => {
                self.content = left;

                let right_content = NodeContent {
                    content: right,
                    metadata: self.metadata.clone(),
                    model: self.model.clone(),
                };

                DiscreteContentResult::Two((self, right_content))
            }
            DiscreteContentResult::One(center) => {
                self.content = center;
                DiscreteContentResult::One(self)
            }
        }
    }
    fn merge(mut self, mut value: Self) -> DiscreteContentResult<Self> {
        if self.metadata != value.metadata || self.model != value.model {
            return DiscreteContentResult::Two((self, value));
        }

        match self.content.merge(value.content) {
            DiscreteContentResult::Two((left, right)) => {
                self.content = left;
                value.content = right;

                DiscreteContentResult::Two((self, value))
            }
            DiscreteContentResult::One(center) => {
                self.content = center;
                DiscreteContentResult::One(self)
            }
        }
    }
}

impl NodeContent {
    fn is_mergeable_with(&self, value: &Self) -> bool {
        if self.metadata != value.metadata || self.model != value.model {
            return false;
        }

        self.content.is_mergeable_with(&value.content)
    }
}

impl DeduplicatableContents for NodeContent {
    fn is_duplicate_of(&self, value: &Self) -> bool {
        self == value
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub enum InnerNodeContent {
    Snippet(Vec<u8>),
    Tokens(Vec<(Vec<u8>, MetadataMap)>),
}

impl InnerNodeContent {
    fn split(self, at: usize) -> DiscreteContentResult<Self> {
        if at == 0 {
            return DiscreteContentResult::One(self);
        }

        match self {
            Self::Snippet(mut snippet) => {
                if snippet.len() <= at {
                    return DiscreteContentResult::One(Self::Snippet(snippet));
                }

                let right = snippet.split_off(at);
                snippet.shrink_to_fit();

                DiscreteContentResult::Two((Self::Snippet(snippet), Self::Snippet(right)))
            }
            Self::Tokens(tokens) => {
                if tokens.iter().map(|token| token.0.len()).sum::<usize>() <= at {
                    return DiscreteContentResult::One(Self::Tokens(tokens));
                }

                let mut content_index = 0;

                let location = tokens.iter().enumerate().find_map(|(location, token)| {
                    if content_index + token.0.len() > at {
                        return Some(location);
                    }
                    content_index += token.0.len();

                    None
                });

                if let Some(location) = location {
                    let mut left = tokens;
                    let mut right = left.split_off(location);
                    left.shrink_to_fit();

                    let mut left_token = right[0].0.clone();
                    let right_token = left_token.split_off(at - content_index);

                    if !left_token.is_empty() {
                        left_token.shrink_to_fit();
                        left.push((left_token, right[0].1.clone()));
                    }
                    right[0].0 = right_token;

                    left.last_mut().unwrap().1.shift_remove("token_id");
                    right[0].1.shift_remove("token_id");

                    DiscreteContentResult::Two((Self::Tokens(left), Self::Tokens(right)))
                } else {
                    DiscreteContentResult::One(Self::Tokens(tokens))
                }
            }
        }
    }
    fn merge(self, value: Self) -> DiscreteContentResult<Self> {
        match self {
            Self::Snippet(mut left_snippet) => match value {
                Self::Snippet(mut right_snippet) => {
                    left_snippet.append(&mut right_snippet);
                    DiscreteContentResult::One(Self::Snippet(left_snippet))
                }
                Self::Tokens(right_tokens) => DiscreteContentResult::Two((
                    Self::Snippet(left_snippet),
                    Self::Tokens(right_tokens),
                )),
            },
            Self::Tokens(mut left_tokens) => match value {
                Self::Snippet(right_snippet) => DiscreteContentResult::Two((
                    Self::Tokens(left_tokens),
                    Self::Snippet(right_snippet),
                )),
                Self::Tokens(mut right_tokens) => {
                    left_tokens.append(&mut right_tokens);
                    DiscreteContentResult::One(Self::Tokens(left_tokens))
                }
            },
        }
    }
    fn is_mergeable_with(&self, value: &Self) -> bool {
        match self {
            Self::Snippet(_) => match value {
                Self::Snippet(_) => true,
                Self::Tokens(_) => false,
            },
            Self::Tokens(_) => match value {
                Self::Snippet(_) => false,
                Self::Tokens(_) => true,
            },
        }
    }
    pub fn as_bytes(&'_ self) -> Cow<'_, Vec<u8>> {
        match self {
            Self::Snippet(snippet) => Cow::Borrowed(snippet),
            Self::Tokens(tokens) => {
                Cow::Owned(tokens.iter().flat_map(|token| token.0.clone()).collect())
            }
        }
    }
    pub fn len(&self) -> usize {
        match self {
            Self::Snippet(snippet) => snippet.len(),
            Self::Tokens(tokens) => tokens.iter().map(|token| token.0.len()).sum(),
        }
    }
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Snippet(snippet) => snippet.is_empty(),
            Self::Tokens(tokens) => tokens.iter().all(|token| token.0.is_empty()),
        }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub label: String,
    pub metadata: MetadataMap,
}

pub type TapestryNode = DependentNode<u128, NodeContent, BuildHasherDefault<UlidHasher>>;
pub type MetadataMap = IndexMap<String, String, FxBuildHasher>;

pub struct TapestryWeave {
    pub weave: DependentWeave<u128, NodeContent, MetadataMap, BuildHasherDefault<UlidHasher>>,
}

impl TapestryWeave {
    pub fn from_unversioned_bytes(bytes: &[u8]) -> Result<Self, Error> {
        Ok(Self {
            weave: from_bytes::<_, Error>(bytes)?,
        })
    }
    pub fn to_unversioned_bytes(&self) -> Result<AlignedVec, Error> {
        assert!(self.weave.validate());
        to_bytes::<Error>(&self.weave)
    }
    pub fn to_versioned_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(VersionedBytes {
            version: 0,
            data: MixedData::Output(self.to_unversioned_bytes()?),
        }
        .to_bytes())
    }
    pub fn to_versioned_weave(self) -> VersionedWeave {
        VersionedWeave::V0(self)
    }
    pub fn with_capacity(capacity: usize, metadata: MetadataMap) -> Self {
        Self {
            weave: DependentWeave::with_capacity(capacity, metadata),
        }
    }
    pub fn capacity(&self) -> usize {
        self.weave.capacity()
    }
    pub fn reserve(&mut self, additional: usize) {
        self.weave.reserve(additional);
    }
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.weave.shrink_to(min_capacity);
    }
    pub fn len(&self) -> usize {
        self.weave.len()
    }
    pub fn is_empty(&self) -> bool {
        self.weave.is_empty()
    }
    pub fn contains(&self, id: &Ulid) -> bool {
        self.weave.contains(&id.0)
    }
    pub fn get_node(&self, id: &Ulid) -> Option<&TapestryNode> {
        self.weave.get_node(&id.0)
    }
    pub fn get_roots(&self) -> impl ExactSizeIterator<Item = Ulid> {
        self.weave.roots().iter().copied().map(Ulid)
    }
    pub fn get_bookmarks(&self) -> impl ExactSizeIterator<Item = Ulid> {
        self.weave.bookmarks().iter().copied().map(Ulid)
    }
    pub fn get_active_thread(&mut self) -> impl DoubleEndedIterator<Item = &TapestryNode> {
        let active: Vec<u128> = self.weave.get_active_thread().collect();

        active.into_iter().filter_map(|id| self.weave.get_node(&id))
    }

    pub fn add_node(&mut self, node: TapestryNode) -> bool {
        let identifier = node.id;
        let last_active_set: HashSet<u128> = if node.active {
            HashSet::from_iter(self.weave.get_active_thread())
        } else {
            HashSet::default()
        };
        let is_active = node.active;

        let status = self.weave.add_node(node);

        let duplicates: Vec<u128> = self.weave.find_duplicates(&identifier).collect();

        if !duplicates.is_empty() {
            if is_active {
                let mut has_active = false;

                for duplicate in &duplicates {
                    if last_active_set.contains(duplicate) {
                        self.weave.set_node_active_status_in_place(duplicate, true);
                        has_active = true;
                        break;
                    }
                }

                if !has_active {
                    self.weave
                        .set_node_active_status_in_place(duplicates.first().unwrap(), true);
                }
            }
            self.weave.remove_node(&identifier);
        }

        status
    }
    pub fn set_node_active_status(&mut self, id: &Ulid, value: bool) -> bool {
        self.weave.set_node_active_status(&id.0, value, false)
    }
    pub fn set_node_bookmarked_status(&mut self, id: &Ulid, value: bool) -> bool {
        self.weave.set_node_bookmarked_status(&id.0, value)
    }
    #[ensures(self.get_active_content() == value)]
    pub fn set_active_content<F>(
        &mut self,
        value: &[u8],
        metadata: MetadataMap,
        mut id_generator: F,
    ) -> bool
    where
        F: FnMut(Option<u64>) -> Ulid,
    {
        let mut modified = false;
        let mut offset: usize = 0;

        let value_len = value.len();

        let active_thread: Vec<u128> = self.weave.get_active_thread().rev().collect();
        let active_node = active_thread.iter().copied().last();

        let mut last_node = None;

        for active_identifier in active_thread {
            let node = self.weave.get_node(&active_identifier).unwrap();
            let content_bytes = node.contents.content.as_bytes();

            let content_len = content_bytes.len();

            if value_len >= offset + content_len
                && value[offset..(offset + content_len)] == *content_bytes
            {
                offset += content_len;
                last_node = Some(node.id);
            } else {
                let start_offset = offset;

                while offset < value_len
                    && offset < content_len + start_offset
                    && value[offset] == content_bytes[offset - start_offset]
                {
                    offset += 1;
                }

                let target = node.id;

                if offset > start_offset {
                    if offset > 0 {
                        let split_identifier = id_generator(Some(Ulid(target).timestamp_ms()));

                        assert!(self.weave.split_node(
                            &target,
                            offset - start_offset,
                            split_identifier.0
                        ));

                        last_node = Some(target);
                    } else {
                        last_node = None;
                    }
                }

                modified = true;

                break;
            }
        }

        if let Some(last_node) = last_node {
            self.weave.set_node_active_status(&last_node, true, false);
        } else if let Some(active_node) = active_node {
            self.weave
                .set_node_active_status(&active_node, false, false);
        }

        if let Some(node) = last_node.and_then(|id| self.weave.get_node(&id))
            && node.to.len() <= 1
            && node
                .to
                .iter()
                .filter_map(|id| self.weave.get_node(id))
                .all(|child| child.to.is_empty())
            && !node.bookmarked
            && node.contents.model.is_none()
            && node.contents.metadata == metadata
        {
            last_node = node.from;

            let identifier = node.id;
            if let Some(node) = self.weave.remove_node(&identifier) {
                offset -= node.contents.content.len();
            }
        }

        if offset < value.len() {
            assert!(self.add_node(DependentNode {
                id: id_generator(None).0,
                from: last_node,
                to: IndexSet::default(),
                active: true,
                bookmarked: false,
                contents: NodeContent {
                    content: InnerNodeContent::Snippet(value[offset..].to_vec()),
                    metadata: metadata.clone(),
                    model: None,
                },
            }));

            modified = true;
        }

        modified
    }
    pub fn get_active_content(&mut self) -> Vec<u8> {
        let active_thread: Vec<u128> = self.weave.get_active_thread().rev().collect();

        active_thread
            .into_iter()
            .filter_map(|id| self.weave.get_node(&id))
            .flat_map(|node| node.contents.content.as_bytes().to_vec())
            .collect()
    }
    pub fn split_node<F>(&mut self, id: &Ulid, at: usize, id_generator: F) -> Option<Ulid>
    where
        F: FnOnce(u64) -> Ulid,
    {
        let new_id = id_generator(id.timestamp_ms());

        if self.weave.split_node(&id.0, at, new_id.0) {
            Some(new_id)
        } else {
            None
        }
    }
    pub fn merge_with_parent(&mut self, id: &Ulid) -> bool {
        self.weave.merge_with_parent(&id.0).is_some()
    }
    pub fn is_mergeable_with_parent(&self, id: &Ulid) -> bool {
        if let Some(node) = self.weave.get_node(&id.0) {
            if let Some(parent) = node.from.and_then(|id| self.weave.get_node(&id)) {
                parent.to.len() == 1 && parent.contents.is_mergeable_with(&node.contents)
            } else {
                false
            }
        } else {
            false
        }
    }
    pub fn remove_node(&mut self, id: &Ulid) -> Option<TapestryNode> {
        self.weave.remove_node(&id.0)
    }
}

pub fn serialize_counterfactual_logprobs(logprobs: Vec<(Vec<u8>, MetadataMap)>) -> String {
    let logprobs: Vec<_> = logprobs
        .into_iter()
        .map(|(data, metadata)| (BASE64_URL_SAFE_NO_PAD.encode(data), metadata))
        .collect();

    serde_json::to_string(&logprobs).unwrap()
}

pub fn deserialize_counterfactual_logprobs(logprobs: &str) -> Option<Vec<(Vec<u8>, MetadataMap)>> {
    serde_json::from_str::<Vec<(String, MetadataMap)>>(logprobs)
        .ok()
        .map(|logprobs| {
            logprobs
                .into_iter()
                .filter_map(|(data, metadata)| {
                    BASE64_URL_SAFE_NO_PAD
                        .decode(data)
                        .ok()
                        .map(|data| (data, metadata))
                })
                .collect()
        })
}
