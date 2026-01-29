//! Experimental & untested

// TODO: Longest common prefix deduplication
// TODO: Token ID based deduplication
// TODO: Request parameter based deduplication (especially for single-token nodes)
// TODO: Add support for temporary nodes which are not actually stored in the IndependentWeave?

use std::{
    borrow::Cow, cmp::Ordering, collections::HashSet, hash::BuildHasherDefault, num::NonZeroU128,
    sync::Arc,
};

#[cfg(feature = "v0")]
use std::str::FromStr;

//use contracts::ensures;
use foldhash::fast::RandomState;
use jiff::Zoned;
use universal_weave::{
    ArchivedWeave, DeduplicatableContents, DeduplicatableWeave, DiscreteContentResult,
    DiscreteContents, DiscreteWeave, IndependentContents, SemiIndependentWeave, Weave,
    independent::{ArchivedIndependentNode, IndependentNode, IndependentWeave},
    indexmap::{IndexMap, IndexSet},
    rkyv::{
        Archive, Deserialize, Serialize, collections::swiss_table::ArchivedIndexSet, from_bytes,
        niche::niching, rancor::Error, rend::u64_le, to_bytes, util::AlignedVec, with::NicheInto,
    },
};

#[cfg(feature = "v0")]
use ulid::Ulid;

use crate::{VersionedWeave, hashers::RandomIdHasher, to_versioned_bytes, wrappers::AsTemporal};

#[cfg(feature = "v0")]
use crate::v0::{
    InnerNodeContent as OldInnerNodeContent, Model as OldModel, NodeContent as OldNodeContent,
    TapestryWeave as OldTapestryWeave, deserialize_counterfactual_logprobs,
};

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct NodeContent {
    #[rkyv(with = AsTemporal)]
    pub timestamp: Zoned,
    pub modified: bool,

    pub content: InnerNodeContent,
    pub metadata: MetadataMap,
    pub creator: Creator,
}

impl IndependentContents for NodeContent {}

impl DiscreteContents for NodeContent {
    fn split(mut self, at: usize) -> DiscreteContentResult<Self> {
        match self.content.split(at) {
            DiscreteContentResult::Two((left, right)) => {
                self.content = left;
                self.modified = true;

                let right_content = NodeContent {
                    timestamp: self.timestamp.clone(),
                    modified: true,
                    content: right,
                    metadata: self.metadata.clone(),
                    creator: self.creator.clone(),
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
        if self.timestamp.time_zone() != value.timestamp.time_zone()
            || self.metadata != value.metadata
            || self.creator != value.creator
        {
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
                self.modified = true;
                self.timestamp = self.timestamp.max(value.timestamp);
                DiscreteContentResult::One(self)
            }
        }
    }
}

impl NodeContent {
    fn is_mergeable_with(&self, value: &Self) -> bool {
        if self.timestamp.time_zone() != value.timestamp.time_zone()
            || self.metadata != value.metadata
            || self.creator != value.creator
        {
            return false;
        }

        self.content.is_mergeable_with(&value.content)
    }
}

impl DeduplicatableContents for NodeContent {
    fn is_duplicate_of(&self, value: &Self) -> bool {
        self.modified == value.modified
            && self.content == value.content
            && self.metadata == value.metadata
            && self.creator == value.creator
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
pub enum InnerNodeContent {
    Snippet(Vec<u8>),
    Tokens(Vec<InnerNodeToken>),
    MetadataOnly,
}

impl InnerNodeContent {
    pub fn token_count(&self) -> Option<usize> {
        if let Self::Tokens(tokens) = self {
            Some(tokens.len())
        } else {
            None
        }
    }
    pub fn calculate_average_logprob(&self) -> Option<f32> {
        if let Self::Tokens(tokens) = self {
            Some(
                (tokens.iter().map(|token| token.logprob as f64).sum::<f64>() / tokens.len() as f64)
                    as f32,
            )
        } else {
            None
        }
    }
    pub fn calculate_cumulative_logprob(&self) -> Option<f32> {
        if let Self::Tokens(tokens) = self {
            Some(tokens.iter().map(|token| token.logprob as f64).sum::<f64>() as f32)
        } else {
            None
        }
    }
    pub fn calculate_confidence(&self) -> Option<(f32, usize, usize)> {
        if let Self::Tokens(tokens) = self {
            let mut confidence_sum = 0.0;
            let mut confidence_k = None;

            for token in tokens {
                if let Some((confidence, k)) = token.calculate_confidence_f64() {
                    if let Some(last_k) = confidence_k
                        && last_k != k
                    {
                        confidence_k = None;
                        break;
                    } else {
                        confidence_k = Some(k);
                    }

                    confidence_sum += confidence;
                } else {
                    confidence_k = None;
                    break;
                }
            }

            confidence_k.map(|confidence_k| {
                (
                    (confidence_sum / tokens.len() as f64) as f32,
                    confidence_k,
                    tokens.len(),
                )
            })
        } else {
            None
        }
    }
    pub fn calculate_average_entropy(&self) -> Option<f32> {
        if let Self::Tokens(tokens) = self {
            if tokens.iter().any(|token| token.entropy.is_some()) {
                Some(
                    (tokens
                        .iter()
                        .filter_map(|token| token.entropy.map(|e| e as f64))
                        .sum::<f64>()
                        / tokens.len() as f64) as f32,
                )
            } else {
                None
            }
        } else {
            None
        }
    }
    fn truncate_tokens(&mut self, count: usize) {
        if let Self::Tokens(tokens) = self {
            assert!(tokens.len() > count);
            tokens.truncate(count);
            tokens.shrink_to_fit();
        } else {
            panic!()
        }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct InnerNodeToken {
    pub bytes: Vec<u8>,
    pub logprob: f32,
    pub id: Option<u64>,
    pub metadata: MetadataMap,

    #[rkyv(with = NicheInto<niching::NaN>)]
    pub entropy: Option<f32>,
    pub counterfactual: Arc<Vec<CounterfactualToken>>,

    pub original: OriginalToken,
}

impl InnerNodeToken {
    pub fn calculate_confidence(&self) -> Option<(f32, usize)> {
        self.calculate_confidence_f64()
            .map(|(confidence, k)| (confidence as f32, k))
    }
    fn calculate_confidence_f64(&self) -> Option<(f64, usize)> {
        if !self.counterfactual.is_empty() {
            Some((
                self.counterfactual
                    .iter()
                    .map(|token| token.logprob as f64)
                    .sum::<f64>()
                    / -(self.counterfactual.len() as f64),
                self.counterfactual.len(),
            ))
        } else {
            None
        }
    }
    pub fn is_modified(&self) -> bool {
        self.original.is_modified()
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
pub enum OriginalToken {
    Unmodified,
    Known(Vec<u8>),
    Unknown,
}

impl OriginalToken {
    pub fn is_modified(&self) -> bool {
        match self {
            Self::Known(_) => true,
            Self::Unknown => true,
            Self::Unmodified => false,
        }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct CounterfactualToken {
    pub bytes: Vec<u8>,
    pub logprob: f32,
    pub id: Option<u64>,
    pub metadata: MetadataMap,
}

impl CounterfactualToken {
    pub fn round_logprob(&mut self) {
        self.logprob = (self.logprob * 100.0).round() / 100.0;
    }
}

impl CounterfactualToken {
    pub fn calculate_entropy<'a>(tokens: impl Iterator<Item = &'a CounterfactualToken>) -> f64 {
        -tokens
            .map(|token| (token.logprob as f64).exp() * (token.logprob as f64))
            .sum::<f64>()
    }
}

const EMPTY_VEC_REF: &Vec<u8> = &Vec::new();

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
                if tokens.iter().map(|token| token.bytes.len()).sum::<usize>() <= at {
                    return DiscreteContentResult::One(Self::Tokens(tokens));
                }

                let mut content_index = 0;

                let location = tokens.iter().enumerate().find_map(|(location, token)| {
                    if content_index + token.bytes.len() > at {
                        return Some(location);
                    }
                    content_index += token.bytes.len();

                    None
                });

                if let Some(location) = location {
                    let mut left = tokens;
                    let mut right = left.split_off(location);
                    left.shrink_to_fit();

                    let mut left_token = right[0].bytes.clone();
                    let right_token = left_token.split_off(at - content_index);

                    debug_assert!(!right_token.is_empty() || left_token.is_empty());

                    if !left_token.is_empty() {
                        if !right[0].original.is_modified() {
                            right[0].original = OriginalToken::Known(right[0].bytes.clone());
                        }

                        left_token.shrink_to_fit();
                        left.push(InnerNodeToken {
                            bytes: left_token,
                            id: None,
                            entropy: None,
                            logprob: right[0].logprob,
                            metadata: right[0].metadata.clone(),
                            counterfactual: right[0].counterfactual.clone(),
                            original: right[0].original.clone(),
                        });
                        right[0].id = None;
                        right[0].entropy = None;
                    }
                    right[0].bytes = right_token;

                    DiscreteContentResult::Two((Self::Tokens(left), Self::Tokens(right)))
                } else {
                    DiscreteContentResult::One(Self::Tokens(tokens))
                }
            }
            Self::MetadataOnly => DiscreteContentResult::One(Self::MetadataOnly),
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
                Self::MetadataOnly => {
                    DiscreteContentResult::Two((Self::Snippet(left_snippet), Self::MetadataOnly))
                }
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
                Self::MetadataOnly => {
                    DiscreteContentResult::Two((Self::Tokens(left_tokens), Self::MetadataOnly))
                }
            },
            Self::MetadataOnly => DiscreteContentResult::Two((Self::MetadataOnly, value)),
        }
    }
    fn is_mergeable_with(&self, value: &Self) -> bool {
        match self {
            Self::Snippet(_) => match value {
                Self::Snippet(_) => true,
                Self::Tokens(_) => false,
                Self::MetadataOnly => false,
            },
            Self::Tokens(_) => match value {
                Self::Snippet(_) => false,
                Self::Tokens(_) => true,
                Self::MetadataOnly => false,
            },
            Self::MetadataOnly => false,
        }
    }
    pub fn as_bytes(&'_ self) -> Cow<'_, Vec<u8>> {
        match self {
            Self::Snippet(snippet) => Cow::Borrowed(snippet),
            Self::Tokens(tokens) => Cow::Owned(
                tokens
                    .iter()
                    .flat_map(|token| token.bytes.clone())
                    .collect(),
            ),
            Self::MetadataOnly => Cow::Borrowed(EMPTY_VEC_REF),
        }
    }
    pub fn len(&self) -> usize {
        match self {
            Self::Snippet(snippet) => snippet.len(),
            Self::Tokens(tokens) => tokens.iter().map(|token| token.bytes.len()).sum(),
            Self::MetadataOnly => 0,
        }
    }
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Snippet(snippet) => snippet.is_empty(),
            Self::Tokens(tokens) => tokens.iter().all(|token| token.bytes.is_empty()),
            Self::MetadataOnly => true,
        }
    }
}

impl ArchivedInnerNodeContent {
    fn is_mergeable_with(&self, value: &Self) -> bool {
        match self {
            Self::Snippet(_) => match value {
                Self::Snippet(_) => true,
                Self::Tokens(_) => false,
                Self::MetadataOnly => false,
            },
            Self::Tokens(_) => match value {
                Self::Snippet(_) => false,
                Self::Tokens(_) => true,
                Self::MetadataOnly => false,
            },
            Self::MetadataOnly => false,
        }
    }
    pub fn as_bytes(&'_ self) -> Vec<u8> {
        match self {
            Self::Snippet(snippet) => snippet.to_vec(),
            Self::Tokens(tokens) => tokens
                .iter()
                .flat_map(|token| token.bytes.to_vec())
                .collect(),
            Self::MetadataOnly => Vec::new(),
        }
    }
    pub fn len(&self) -> usize {
        match self {
            Self::Snippet(snippet) => snippet.len(),
            Self::Tokens(tokens) => tokens.iter().map(|token| token.bytes.len()).sum(),
            Self::MetadataOnly => 0,
        }
    }
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Snippet(snippet) => snippet.is_empty(),
            Self::Tokens(tokens) => tokens.iter().all(|token| token.bytes.is_empty()),
            Self::MetadataOnly => true,
        }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub enum Creator {
    Model(Option<Model>),
    Human(Option<Author>),
    Unknown,
}

impl Creator {
    pub fn is_model(&self) -> bool {
        matches!(self, Self::Model(_))
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub label: String,
    pub color: Option<String>,

    #[rkyv(with = NicheInto<niching::Zero>)]
    pub identifier: Option<NonZeroU128>,
    pub seed: Option<u32>,

    pub metadata: MetadataMap,
}

pub const UNKNOWN_MODEL_LABEL: &str = "Unknown Model";

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Author {
    pub label: String,

    #[rkyv(with = NicheInto<niching::Zero>)]
    pub identifier: Option<NonZeroU128>,
}

pub type ShortId = u64;
pub type LongId = NonZeroU128;
pub type TapestryNode = IndependentNode<u64, NodeContent, BuildHasherDefault<RandomIdHasher>>;
pub type MetadataMap = IndexMap<String, String, RandomState>;
pub type ArchivedTapestryNode =
    ArchivedIndependentNode<u64, NodeContent, BuildHasherDefault<RandomIdHasher>>;
pub type TapestryWeaveInner =
    IndependentWeave<u64, NodeContent, TapestryWeaveMetadata, BuildHasherDefault<RandomIdHasher>>;

pub struct TapestryWeave {
    weave: TapestryWeaveInner,
    active: Vec<u64>,
    scratchpad: Vec<u64>,
    changed: bool,
    changed_shape: bool,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct TapestryWeaveMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    #[rkyv(with = AsTemporal)]
    pub created: Zoned,
    pub converted_from: Vec<ConvertedFrom>,

    pub metadata: MetadataMap,
}

impl TapestryWeaveMetadata {
    fn is_empty(&self) -> bool {
        self.description
            .as_ref()
            .map(|v| v.is_empty())
            .unwrap_or(true)
            && self.title.as_ref().map(|v| v.is_empty()).unwrap_or(true)
            && self.converted_from.is_empty()
            && self.metadata.is_empty()
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct ConvertedFrom {
    pub source: String,
    pub source_version: Option<String>,

    #[rkyv(with = AsTemporal)]
    pub timestamp: Zoned,
}

impl From<TapestryWeaveInner> for TapestryWeave {
    fn from(mut value: TapestryWeaveInner) -> Self {
        let mut active = Vec::with_capacity(value.capacity());
        value.get_active_thread(&mut active);

        Self {
            active,
            scratchpad: Vec::with_capacity(value.capacity()),
            weave: value,
            changed: false,
            changed_shape: false,
        }
    }
}

impl From<TapestryWeave> for TapestryWeaveInner {
    fn from(value: TapestryWeave) -> Self {
        value.weave
    }
}

impl AsRef<TapestryWeaveInner> for TapestryWeave {
    fn as_ref(&self) -> &TapestryWeaveInner {
        &self.weave
    }
}

impl TapestryWeave {
    pub fn from_unversioned_bytes(bytes: &[u8]) -> Result<Self, Error> {
        Ok(Self::from(from_bytes::<TapestryWeaveInner, Error>(bytes)?))
    }
    pub fn to_unversioned_bytes(&self) -> Result<AlignedVec, Error> {
        assert!(self.weave.validate());
        to_bytes::<Error>(&self.weave)
    }
    pub fn to_versioned_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(to_versioned_bytes(1, &self.to_unversioned_bytes()?))
    }
    pub fn to_versioned_weave(self) -> VersionedWeave {
        VersionedWeave::V1(self)
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            weave: IndependentWeave::with_capacity(
                capacity,
                TapestryWeaveMetadata {
                    title: None,
                    description: None,
                    created: Zoned::now(),
                    converted_from: Vec::new(),
                    metadata: IndexMap::default(),
                },
            ),
            active: Vec::with_capacity(capacity),
            scratchpad: Vec::with_capacity(capacity),
            changed: false,
            changed_shape: false,
        }
    }
    pub fn with_capacity_and_metadata(capacity: usize, metadata: TapestryWeaveMetadata) -> Self {
        Self {
            weave: IndependentWeave::with_capacity(capacity, metadata),
            active: Vec::with_capacity(capacity),
            scratchpad: Vec::with_capacity(capacity),
            changed: false,
            changed_shape: false,
        }
    }
    pub fn capacity(&self) -> usize {
        self.weave.capacity()
    }
    pub fn reserve(&mut self, additional: usize) {
        self.weave.reserve(additional);
        self.active
            .reserve(self.weave.capacity().saturating_sub(self.active.capacity()));
        self.scratchpad.reserve(
            self.weave
                .capacity()
                .saturating_sub(self.scratchpad.capacity()),
        );
    }
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.weave.shrink_to(min_capacity);
        self.active.shrink_to(min_capacity);
        self.scratchpad.shrink_to(min_capacity);
    }
    pub fn metadata(&mut self) -> &mut TapestryWeaveMetadata {
        &mut self.weave.metadata
    }
    pub fn len(&self) -> usize {
        self.weave.len()
    }
    pub fn is_empty(&self) -> bool {
        self.weave.is_empty()
    }
    pub fn is_empty_including_metadata(&self) -> bool {
        self.weave.is_empty() && self.weave.metadata.is_empty()
    }
    pub fn contains(&self, id: &u64) -> bool {
        self.weave.contains(id)
    }
    pub fn contains_active(&self, id: &u64) -> bool {
        self.weave.contains_active(id)
    }
    pub fn has_changed(&mut self) -> bool {
        let changed = self.changed;
        self.changed = false;

        changed
    }
    pub fn has_shape_changed(&mut self) -> bool {
        let changed = self.changed_shape;
        self.changed_shape = false;

        changed
    }
    pub fn dump_identifiers_ordered(&mut self, output: &mut Vec<u64>) {
        self.weave.get_ordered_node_identifiers(output);
    }
    pub fn dump_identifiers_ordered_rev(&mut self, output: &mut Vec<u64>) {
        self.weave
            .get_ordered_node_identifiers_reversed_children(output)
    }
    pub fn get_node(&self, id: &u64) -> Option<&TapestryNode> {
        self.weave.get_node(id)
    }
    pub fn get_node_children(
        &self,
        id: &u64,
    ) -> Option<&IndexSet<u64, BuildHasherDefault<RandomIdHasher>>> {
        self.weave.get_node(id).map(|node| &node.to)
    }
    pub fn get_node_parents(
        &self,
        id: &u64,
    ) -> Option<&IndexSet<u64, BuildHasherDefault<RandomIdHasher>>> {
        self.weave.get_node(id).map(|node| &node.from)
    }
    pub fn get_node_siblings(&self, id: &u64) -> Option<impl DoubleEndedIterator<Item = u64>> {
        self.weave.get_node(id).map(|node| {
            node.from
                .iter()
                .filter_map(|parent| self.weave.get_node(parent))
                .flat_map(|parent| {
                    parent.to.iter().copied().filter(|sibling| {
                        *sibling != node.id
                            && !node.from.contains(sibling)
                            && !node.to.contains(sibling)
                    })
                })
        })
    }
    pub fn get_node_siblings_or_roots<'s>(
        &'s self,
        id: &u64,
    ) -> Option<Box<dyn DoubleEndedIterator<Item = u64> + 's>> {
        self.weave.get_node(id).map(|node| {
            if node.from.is_empty() {
                Box::new(self.weave.roots().iter().copied().filter(|sibling| {
                    *sibling != node.id
                        && !node.from.contains(sibling)
                        && !node.to.contains(sibling)
                })) as Box<dyn DoubleEndedIterator<Item = u64>>
            } else {
                Box::new(
                    node.from
                        .iter()
                        .filter_map(|parent| self.weave.get_node(parent))
                        .flat_map(|parent| {
                            parent.to.iter().copied().filter(|sibling| {
                                *sibling != node.id
                                    && !node.from.contains(sibling)
                                    && !node.to.contains(sibling)
                            })
                        }),
                ) as Box<dyn DoubleEndedIterator<Item = u64>>
            }
        })
    }
    pub fn roots(&self) -> &IndexSet<u64, BuildHasherDefault<RandomIdHasher>> {
        self.weave.roots()
    }
    pub fn bookmarks(&self) -> &IndexSet<u64, BuildHasherDefault<RandomIdHasher>> {
        self.weave.bookmarks()
    }
    pub fn get_active_thread(&mut self) -> impl DoubleEndedIterator<Item = &TapestryNode> {
        self.active.iter().filter_map(|id| self.weave.get_node(id))
    }
    pub fn get_active_thread_ids(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = u64> + ExactSizeIterator<Item = u64> {
        self.active.iter().copied()
    }
    pub fn get_thread_from(&mut self, id: &u64) -> impl DoubleEndedIterator<Item = &TapestryNode> {
        self.weave.get_thread_from(id, &mut self.scratchpad);

        self.scratchpad
            .drain(..)
            .filter_map(|id| self.weave.get_node(&id))
    }
    pub fn get_thread_from_ids(&mut self, id: &u64) -> &Vec<u64> {
        self.weave.get_thread_from(id, &mut self.scratchpad);
        &self.scratchpad
    }
    fn update_shape_and_active(&mut self) {
        self.changed = true;
        self.changed_shape = true;
        self.weave.get_active_thread(&mut self.active)
    }
    pub fn add_node(&mut self, node: TapestryNode) -> bool {
        let identifier = node.id;
        let last_active_set: HashSet<u64, BuildHasherDefault<RandomIdHasher>> = if node.active {
            HashSet::from_iter(self.active.iter().copied())
        } else {
            HashSet::default()
        };
        let is_active = node.active;

        let status = self.weave.add_node(node);

        if status {
            let duplicates: Vec<u64> = self.weave.find_duplicates(&identifier).collect();

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

            self.update_shape_and_active();
        }

        status
    }
    pub fn add_node_direct(&mut self, node: TapestryNode) -> bool {
        if self.weave.add_node(node) {
            self.update_shape_and_active();
            true
        } else {
            false
        }
    }
    pub fn set_node_active_status(&mut self, id: &u64, value: bool, alternate: bool) -> bool {
        if self.weave.set_node_active_status(id, value, alternate) {
            self.update_shape_and_active();
            true
        } else {
            false
        }
    }
    pub fn set_node_active_status_in_place(&mut self, id: &u64, value: bool) -> bool {
        if self.weave.set_node_active_status_in_place(id, value) {
            self.update_shape_and_active();
            true
        } else {
            false
        }
    }
    pub fn set_node_bookmarked_status(&mut self, id: &u64, value: bool) -> bool {
        if self.weave.set_node_bookmarked_status(id, value) {
            self.changed = true;
            true
        } else {
            false
        }
    }
    pub fn get_active_content(&mut self) -> Vec<u8> {
        self.active
            .iter()
            .rev()
            .filter_map(|id| self.weave.get_node(id))
            .flat_map(|node| node.contents.content.as_bytes().to_vec())
            .collect()
    }
    pub fn split_node(
        &mut self,
        id: &u64,
        at: usize,
        mut id_generator: impl FnMut() -> u64,
    ) -> Option<(u64, Option<u64>, u64)> {
        if at > 0
            && let Some(node) = self.weave.get_node(id).cloned()
            && let InnerNodeContent::Tokens(tokens) = &node.contents.content
        {
            let mut byte_index = 0;
            let mut within_token = false;
            for token in tokens {
                byte_index += token.bytes.len();
                if byte_index >= at {
                    if byte_index > at {
                        within_token = true;
                    }
                    byte_index -= token.bytes.len();
                    break;
                }
            }

            if within_token {
                let first_split_id = id_generator();

                assert!(self.weave.split_node(id, byte_index, first_split_id));

                self.weave.get_contents_mut(id).unwrap().modified = true;
                self.weave
                    .get_contents_mut(&first_split_id)
                    .unwrap()
                    .modified = true;

                let mut token_node = self.weave.get_node(&first_split_id).unwrap().clone();
                token_node.id = id_generator();
                //token_node.to = IndexSet::default();
                token_node.contents.content.truncate_tokens(1);

                let token_node_id = token_node.id;

                assert!(self.weave.add_node(token_node));

                let second_split_id = id_generator();

                assert!(
                    self.weave
                        .split_node(&token_node_id, at - byte_index, second_split_id)
                );

                self.update_shape_and_active();

                Some((*id, Some(token_node_id), second_split_id))
            } else {
                let new_id = id_generator();

                if self.weave.split_node(id, at, new_id) {
                    self.weave.get_contents_mut(id).unwrap().modified = true;
                    self.weave.get_contents_mut(&new_id).unwrap().modified = true;
                    self.update_shape_and_active();
                    Some((*id, None, new_id))
                } else {
                    None
                }
            }
        } else {
            let new_id = id_generator();

            if self.weave.split_node(id, at, new_id) {
                self.weave.get_contents_mut(id).unwrap().modified = true;
                self.weave.get_contents_mut(&new_id).unwrap().modified = true;
                self.update_shape_and_active();
                Some((*id, None, new_id))
            } else {
                None
            }
        }
    }
    pub fn split_node_direct(&mut self, id: &u64, at: usize, new_id: u64) -> Option<u64> {
        if self.weave.split_node(id, at, new_id) {
            self.weave.get_contents_mut(id).unwrap().modified = true;
            self.weave.get_contents_mut(&new_id).unwrap().modified = true;
            self.update_shape_and_active();
            Some(new_id)
        } else {
            None
        }
    }
    pub fn split_out_token(
        &mut self,
        id: &u64,
        index: usize,
        id_generator: impl FnMut() -> u64,
    ) -> Option<(Option<u64>, u64, Option<u64>)> {
        if let Some(result) = self.split_out_token_inner(id, index, id_generator) {
            if result.0 == Some(*id) || result.2.is_some() {
                self.update_shape_and_active();
            }

            Some(result)
        } else {
            None
        }
    }
    fn split_out_token_inner(
        &mut self,
        id: &u64,
        index: usize,
        mut id_generator: impl FnMut() -> u64,
    ) -> Option<(Option<u64>, u64, Option<u64>)> {
        // before_token, token, after_token
        if let Some(node) = self.weave.get_node(id) {
            if let InnerNodeContent::Tokens(tokens) = &node.contents.content
                && tokens.len() > index
            {
                let chosen_parent = node
                    .from
                    .iter()
                    .copied()
                    .find(|id| self.weave.contains_active(id))
                    .or_else(|| node.from.first().copied());

                let split_index: usize = tokens
                    .iter()
                    .take(index)
                    .map(|token| token.bytes.len())
                    .sum();

                let second_split_index = if tokens.len() > index + 1 {
                    Some(
                        tokens
                            .iter()
                            .take(index + 1)
                            .map(|token| token.bytes.len())
                            .sum::<usize>()
                            - split_index,
                    )
                } else {
                    None
                };

                if split_index > 0 {
                    let middle_id = id_generator();

                    assert!(self.weave.split_node(id, split_index, middle_id));

                    self.weave.get_contents_mut(id).unwrap().modified = true;
                    self.weave.get_contents_mut(&middle_id).unwrap().modified = true;

                    if let Some(second_split_index) = second_split_index
                        && second_split_index > 0
                    {
                        let tail_id = id_generator();

                        assert!(
                            self.weave
                                .split_node(&middle_id, second_split_index, tail_id)
                        );

                        self.weave.get_contents_mut(&tail_id).unwrap().modified = true;

                        Some((Some(*id), middle_id, Some(tail_id)))
                    } else {
                        Some((Some(*id), middle_id, None))
                    }
                } else if let Some(second_split_index) = second_split_index
                    && second_split_index > 0
                {
                    let tail_id = id_generator();

                    assert!(self.weave.split_node(id, second_split_index, tail_id));

                    self.weave.get_contents_mut(&tail_id).unwrap().modified = true;

                    Some((chosen_parent, *id, Some(tail_id)))
                } else {
                    Some((chosen_parent, *id, None))
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    pub fn merge_with_parent(&mut self, id: &u64) -> bool {
        if let Some(new_id) = self.weave.merge_with_parent(id) {
            self.weave.get_contents_mut(&new_id).unwrap().modified = true;
            self.update_shape_and_active();
            true
        } else {
            false
        }
    }
    pub fn is_mergeable_with_parent(&self, id: &u64) -> bool {
        if let Some(node) = self.weave.get_node(id) {
            if node.from.len() == 1
                && let Some(parent) = node.from.first().and_then(|id| self.weave.get_node(id))
            {
                parent.to.len() == 1 && parent.contents.is_mergeable_with(&node.contents)
            } else {
                false
            }
        } else {
            false
        }
    }
    pub fn remove_node(&mut self, id: &u64) -> Option<TapestryNode> {
        if let Some(removed) = self.weave.remove_node(id) {
            self.update_shape_and_active();
            Some(removed)
        } else {
            None
        }
    }
    pub fn sort_roots_by(&mut self, compare: impl FnMut(&TapestryNode, &TapestryNode) -> Ordering) {
        self.changed = true;
        self.changed_shape = true;
        self.weave.sort_roots_by(compare)
    }
    pub fn sort_node_children_by(
        &mut self,
        id: &u64,
        compare: impl FnMut(&TapestryNode, &TapestryNode) -> Ordering,
    ) -> bool {
        self.changed = true;
        self.changed_shape = true;
        self.weave.sort_node_children_by(id, compare)
    }
    pub fn modify_inner<T>(
        &mut self,
        callback: impl FnOnce(&mut TapestryWeaveInner, &[u64]) -> T,
    ) -> T {
        let output = callback(&mut self.weave, &self.active);
        self.update_shape_and_active();

        output
    }
}

impl TapestryWeave {
    // TODO: diff-based set_active_content, insert_node_at, remove_active_range
}

pub struct ArchivedTapestryWeave {
    pub weave: <TapestryWeaveInner as Archive>::Archived,
}

impl AsRef<<TapestryWeaveInner as Archive>::Archived> for ArchivedTapestryWeave {
    fn as_ref(&self) -> &<TapestryWeaveInner as Archive>::Archived {
        &self.weave
    }
}

impl ArchivedTapestryWeave {
    pub fn len(&self) -> usize {
        self.weave.len()
    }
    pub fn is_empty(&self) -> bool {
        self.weave.is_empty()
    }
    pub fn contains(&self, id: &u64_le) -> bool {
        self.weave.contains(id)
    }
    pub fn contains_active(&self, id: &u64_le) -> bool {
        self.weave.contains_active(id)
    }
    pub fn get_node(&self, id: &u64_le) -> Option<&ArchivedTapestryNode> {
        self.weave.get_node(id)
    }
    pub fn get_node_children(&self, id: &u64_le) -> Option<&ArchivedIndexSet<u64_le>> {
        self.weave.get_node(id).map(|node| &node.to)
    }
    pub fn get_node_parents(&self, id: &u64_le) -> Option<&ArchivedIndexSet<u64_le>> {
        self.weave.get_node(id).map(|node| &node.from)
    }
    pub fn get_node_siblings(&self, id: &u64_le) -> Option<impl Iterator<Item = u64_le>> {
        self.weave.get_node(id).map(|node| {
            node.from
                .iter()
                .filter_map(|parent| self.weave.get_node(parent))
                .flat_map(|parent| {
                    parent.to.iter().copied().filter(|sibling| {
                        *sibling != node.id
                            && !node.from.contains(sibling)
                            && !node.to.contains(sibling)
                    })
                })
        })
    }
    pub fn get_node_siblings_or_roots<'s>(
        &'s self,
        id: &u64_le,
    ) -> Option<Box<dyn Iterator<Item = u64_le> + 's>> {
        self.weave.get_node(id).map(|node| {
            if node.from.is_empty() {
                Box::new(self.weave.roots().iter().copied().filter(|sibling| {
                    *sibling != node.id
                        && !node.from.contains(sibling)
                        && !node.to.contains(sibling)
                })) as Box<dyn Iterator<Item = u64_le>>
            } else {
                Box::new(
                    node.from
                        .iter()
                        .filter_map(|parent| self.weave.get_node(parent))
                        .flat_map(|parent| {
                            parent.to.iter().copied().filter(|sibling| {
                                *sibling != node.id
                                    && !node.from.contains(sibling)
                                    && !node.to.contains(sibling)
                            })
                        }),
                ) as Box<dyn Iterator<Item = u64_le>>
            }
        })
    }
    pub fn get_roots(&self) -> impl ExactSizeIterator<Item = u64_le> {
        self.weave.roots().iter().copied()
    }
    pub fn get_bookmarks(&self) -> impl ExactSizeIterator<Item = u64_le> {
        self.weave.bookmarks().iter().copied()
    }
    pub fn get_active_thread(&mut self) -> impl DoubleEndedIterator<Item = &ArchivedTapestryNode> {
        let mut scratchpad = Vec::with_capacity(self.weave.len());

        self.weave.get_active_thread(&mut scratchpad);

        scratchpad
            .into_iter()
            .filter_map(|id| self.weave.get_node(&id))
    }
    pub fn get_thread_from(
        &mut self,
        id: &u64_le,
    ) -> impl DoubleEndedIterator<Item = &ArchivedTapestryNode> {
        let mut scratchpad = Vec::with_capacity(self.weave.len());

        self.weave.get_thread_from(id, &mut scratchpad);

        scratchpad
            .into_iter()
            .filter_map(|id| self.weave.get_node(&id))
    }
    pub fn get_active_content(&self) -> Vec<u8> {
        let mut scratchpad = Vec::with_capacity(self.weave.len());

        self.weave.get_active_thread(&mut scratchpad);

        scratchpad
            .into_iter()
            .rev()
            .filter_map(|id| self.weave.get_node(&id))
            .flat_map(|node| node.contents.content.as_bytes())
            .collect()
    }
    pub fn is_mergeable_with_parent(&self, id: &u64_le) -> bool {
        if let Some(node) = self.weave.get_node(id) {
            if node.from.len() == 1
                && let Some(parent) = node
                    .from
                    .get_index(0)
                    .and_then(|id| self.weave.get_node(id))
            {
                parent.to.len() == 1
                    && parent
                        .contents
                        .content
                        .is_mergeable_with(&node.contents.content)
            } else {
                false
            }
        } else {
            false
        }
    }
}

#[cfg(feature = "v0")]
impl From<OldInnerNodeContent> for InnerNodeContent {
    fn from(value: OldInnerNodeContent) -> Self {
        match value {
            OldInnerNodeContent::Snippet(snippet) => Self::Snippet(snippet),
            OldInnerNodeContent::Tokens(tokens) => Self::Tokens(
                tokens
                    .into_iter()
                    .map(|(token, mut metadata)| {
                        metadata.shift_remove("model_id");
                        metadata.shift_remove("confidence");
                        metadata.shift_remove("confidence_k");

                        let mut modified = metadata
                            .shift_remove("original_length")
                            .and_then(|value| value.parse::<usize>().ok())
                            .map(|original_length| original_length != token.len())
                            .unwrap_or(false);

                        if let Some(value) = metadata.shift_remove("modified")
                            && value == "true"
                        {
                            modified = true;
                        }

                        InnerNodeToken {
                            bytes: token,
                            logprob: metadata
                                .shift_remove("probability")
                                .and_then(|value| value.parse::<f32>().ok())
                                .unwrap_or(f32::NAN)
                                .ln(),
                            id: if !modified {
                                metadata
                                    .shift_remove("token_id")
                                    .and_then(|value| value.parse::<u64>().ok())
                            } else {
                                None
                            },
                            entropy: None,
                            counterfactual: Arc::new(
                                metadata
                                    .shift_remove("counterfactual")
                                    .and_then(|value| {
                                        deserialize_counterfactual_logprobs(&value).map(
                                            |counterfactual| {
                                                counterfactual
                                                    .into_iter()
                                                    .map(|(token, mut metadata)| {
                                                        metadata.shift_remove("model_id");
                                                        metadata.shift_remove("confidence");
                                                        metadata.shift_remove("confidence_k");
                                                        metadata.shift_remove("original_length");
                                                        metadata.shift_remove("modified");

                                                        CounterfactualToken {
                                                            bytes: token,
                                                            logprob: metadata
                                                                .shift_remove("probability")
                                                                .and_then(|value| {
                                                                    value.parse::<f32>().ok()
                                                                })
                                                                .unwrap_or(f32::NAN)
                                                                .ln(),
                                                            id: metadata
                                                                .shift_remove("token_id")
                                                                .and_then(|value| {
                                                                    value.parse::<u64>().ok()
                                                                }),
                                                            metadata,
                                                        }
                                                    })
                                                    .collect()
                                            },
                                        )
                                    })
                                    .unwrap_or_default(),
                            ),
                            metadata,
                            original: if modified {
                                OriginalToken::Unknown
                            } else {
                                OriginalToken::Unmodified
                            },
                        }
                    })
                    .collect(),
            ),
        }
    }
}

#[cfg(feature = "v0")]
impl From<OldModel> for Creator {
    fn from(mut value: OldModel) -> Self {
        if value.label.to_lowercase() == "unknown model"
            || value.label.to_lowercase() == "unknown"
            || value.label.to_lowercase() == "n/a"
            || value.label.is_empty()
        {
            value.label = UNKNOWN_MODEL_LABEL.to_string();
        }

        Self::Model(
            if value.label == UNKNOWN_MODEL_LABEL && value.metadata.is_empty() {
                None
            } else {
                Some(Model {
                    label: value.label,
                    color: value.metadata.shift_remove("color"),
                    identifier: None,
                    seed: None,
                    metadata: value.metadata,
                })
            },
        )
    }
}

#[cfg(feature = "v0")]
impl From<OldNodeContent> for NodeContent {
    fn from(mut value: OldNodeContent) -> Self {
        value.metadata.shift_remove("confidence");
        value.metadata.shift_remove("confidence_k");
        value.metadata.shift_remove("confidence_n");

        let mut creator = value.model.map(Creator::from).unwrap_or(Creator::Unknown);

        if let Creator::Model(Some(model)) = &mut creator
            && let OldInnerNodeContent::Tokens(tokens) = &mut value.content
        {
            let mut model_id = None;

            for (_, metadata) in tokens {
                if let Some(value) = metadata.shift_remove("model_id") {
                    if let Some(existing_id) = &model_id
                        && *existing_id != value
                    {
                        model_id = None;
                        break;
                    } else {
                        model_id = Some(value);
                    }
                }
            }

            if let Some(model_id) = model_id.and_then(|id| id.parse::<NonZeroU128>().ok()) {
                model.identifier = Some(model_id);
            }
        }

        let content = InnerNodeContent::from(value.content);

        let mut modified = if let InnerNodeContent::Tokens(tokens) = &content {
            tokens.iter().any(|token| token.original.is_modified())
        } else {
            false
        };

        if let Some(value) = value.metadata.shift_remove("modified")
            && value.to_lowercase() == "true"
        {
            modified = true;
        }

        Self {
            timestamp: Zoned::default(),
            modified,
            metadata: value.metadata,
            creator,
            content,
        }
    }
}

#[cfg(feature = "v0")]
impl From<MetadataMap> for TapestryWeaveMetadata {
    fn from(mut value: MetadataMap) -> Self {
        let conversion_timestamp = value
            .shift_remove("converted")
            .and_then(|value| Zoned::from_str(&value).ok());
        let source = value.shift_remove("converted_from");
        let source_version = value.shift_remove("converted_from_version");

        let mut converted_from = Vec::with_capacity(2);

        if source.is_some() || conversion_timestamp.is_some() {
            converted_from.push(ConvertedFrom {
                source: source.unwrap_or_else(|| "Unknown".to_string()),
                source_version,
                timestamp: conversion_timestamp.unwrap_or_default(),
            });
        }

        converted_from.push(ConvertedFrom {
            source: "TapestryLoomBeta".to_string(),
            source_version: Some("0".to_string()),
            timestamp: Zoned::now(),
        });

        TapestryWeaveMetadata {
            title: value.shift_remove("title"),
            description: value
                .shift_remove("description")
                .or_else(|| value.shift_remove("notes")),
            created: value
                .shift_remove("created")
                .and_then(|value| Zoned::from_str(&value).ok())
                .unwrap_or_default(),
            converted_from,
            metadata: value,
        }
    }
}

#[cfg(feature = "v0")]
fn convert_old_identifier(value: u128) -> u64 {
    unsafe { std::mem::transmute::<u128, [u64; 2]>(value)[1] }
}

#[cfg(feature = "v0")]
impl From<OldTapestryWeave> for TapestryWeave {
    fn from(mut value: OldTapestryWeave) -> Self {
        let mut output = TapestryWeave::with_capacity_and_metadata(
            value.capacity(),
            value.weave.metadata.clone().into(),
        );

        let mut identifiers = Vec::with_capacity(value.weave.len());
        value.weave.get_ordered_node_identifiers(&mut identifiers);

        for identifier in identifiers {
            let node = value.weave.get_node(&identifier).unwrap().clone();

            let timestamp = Zoned::try_from(Ulid(node.id).datetime()).unwrap_or(Zoned::default());

            let mut node = TapestryNode {
                id: convert_old_identifier(node.id),
                from: IndexSet::from_iter(node.from.into_iter().map(convert_old_identifier)),
                to: IndexSet::from_iter(node.to.into_iter().map(convert_old_identifier)),
                active: node.active,
                bookmarked: node.bookmarked,
                contents: node.contents.into(),
            };

            node.contents.timestamp = timestamp;

            assert!(output.add_node(node));
        }

        output
    }
}
