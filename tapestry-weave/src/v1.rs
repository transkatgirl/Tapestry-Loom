//! Experimental & untested

use std::{borrow::Cow, cmp::Ordering, collections::HashSet, hash::BuildHasherDefault};

use chrono::{FixedOffset, NaiveDateTime};
use contracts::ensures;
use rustc_hash::FxBuildHasher;
use universal_weave::{
    ArchivedWeave, DeduplicatableContents, DeduplicatableWeave, DiscreteContentResult,
    DiscreteContents, DiscreteWeave, IndependentContents, SemiIndependentWeave, Weave,
    independent::{ArchivedIndependentNode, IndependentNode, IndependentWeave},
    indexmap::{IndexMap, IndexSet},
    rkyv::{
        Archive, Deserialize, Serialize, collections::swiss_table::ArchivedIndexSet, from_bytes,
        rancor::Error, rend::u64_le, to_bytes, util::AlignedVec,
    },
};

use crate::{
    VersionedWeave,
    hashers::RandomIdHasher,
    v0::{NodeContent as OldNodeContent, TapestryWeave as OldTapestryWeave},
    versioning::{MixedData, VersionedBytes},
};

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct NodeContent {
    pub timestamp: (NaiveDateTime, FixedOffset),
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
                    timestamp: self.timestamp,
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
        if self.timestamp.1 != value.timestamp.1
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
                self.timestamp.0 = self.timestamp.0.max(value.timestamp.0);
                DiscreteContentResult::One(self)
            }
        }
    }
}

impl NodeContent {
    fn is_mergeable_with(&self, value: &Self) -> bool {
        if self.timestamp.1 != value.timestamp.1
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
    Link(String),
}

impl InnerNodeContent {
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
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct InnerNodeToken {
    bytes: Vec<u8>,
    logprob: Option<f32>,
    metadata: MetadataMap,
    counterfactual: Vec<CounterfactualToken>,
    original: Option<Vec<u8>>,
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
                    / self.counterfactual.len() as f64,
                self.counterfactual.len(),
            ))
        } else {
            None
        }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct CounterfactualToken {
    bytes: Vec<u8>,
    logprob: f32,
    metadata: MetadataMap,
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

                    if !left_token.is_empty() {
                        left_token.shrink_to_fit();
                        left.push(InnerNodeToken {
                            bytes: left_token,
                            logprob: right[0].logprob,
                            metadata: right[0].metadata.clone(),
                            counterfactual: right[0].counterfactual.clone(),
                            original: Some(right[0].bytes.clone()),
                        });
                        right[0].original = Some(right[0].bytes.clone());
                    }
                    right[0].bytes = right_token;

                    DiscreteContentResult::Two((Self::Tokens(left), Self::Tokens(right)))
                } else {
                    DiscreteContentResult::One(Self::Tokens(tokens))
                }
            }
            Self::Link(link) => DiscreteContentResult::One(Self::Link(link)),
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
                Self::Link(right_link) => DiscreteContentResult::Two((
                    Self::Snippet(left_snippet),
                    Self::Link(right_link),
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
                Self::Link(right_link) => {
                    DiscreteContentResult::Two((Self::Tokens(left_tokens), Self::Link(right_link)))
                }
            },
            Self::Link(link) => DiscreteContentResult::Two((Self::Link(link), value)),
        }
    }
    fn is_mergeable_with(&self, value: &Self) -> bool {
        match self {
            Self::Snippet(_) => match value {
                Self::Snippet(_) => true,
                Self::Tokens(_) => false,
                Self::Link(_) => false,
            },
            Self::Tokens(_) => match value {
                Self::Snippet(_) => false,
                Self::Tokens(_) => true,
                Self::Link(_) => false,
            },
            Self::Link(_) => false,
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
            Self::Link(_) => Cow::Owned(Vec::new()),
        }
    }
    pub fn len(&self) -> usize {
        match self {
            Self::Snippet(snippet) => snippet.len(),
            Self::Tokens(tokens) => tokens.iter().map(|token| token.bytes.len()).sum(),
            Self::Link(_) => 0,
        }
    }
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Snippet(snippet) => snippet.is_empty(),
            Self::Tokens(tokens) => tokens.iter().all(|token| token.bytes.is_empty()),
            Self::Link(_) => true,
        }
    }
}

impl ArchivedInnerNodeContent {
    fn is_mergeable_with(&self, value: &Self) -> bool {
        match self {
            Self::Snippet(_) => match value {
                Self::Snippet(_) => true,
                Self::Tokens(_) => false,
                Self::Link(_) => false,
            },
            Self::Tokens(_) => match value {
                Self::Snippet(_) => false,
                Self::Tokens(_) => true,
                Self::Link(_) => false,
            },
            Self::Link(_) => false,
        }
    }
    pub fn as_bytes(&'_ self) -> Vec<u8> {
        match self {
            Self::Snippet(snippet) => snippet.to_vec(),
            Self::Tokens(tokens) => tokens
                .iter()
                .flat_map(|token| token.bytes.to_vec())
                .collect(),
            Self::Link(_) => Vec::new(),
        }
    }
    pub fn len(&self) -> usize {
        match self {
            Self::Snippet(snippet) => snippet.len(),
            Self::Tokens(tokens) => tokens.iter().map(|token| token.bytes.len()).sum(),
            Self::Link(_) => 0,
        }
    }
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Snippet(snippet) => snippet.is_empty(),
            Self::Tokens(tokens) => tokens.iter().all(|token| token.bytes.is_empty()),
            Self::Link(_) => true,
        }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub enum Creator {
    Model(Option<Model>),
    Human(Option<Author>),
    Unknown,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub label: String,
    pub identifier: Option<u64>,
    pub metadata: MetadataMap,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Author {
    pub label: String,
    pub identifier: Option<u64>,
}

pub type TapestryNode = IndependentNode<u64, NodeContent, BuildHasherDefault<RandomIdHasher>>;
pub type MetadataMap = IndexMap<String, String, FxBuildHasher>;
pub type ArchivedTapestryNode =
    ArchivedIndependentNode<u64, NodeContent, BuildHasherDefault<RandomIdHasher>>;
pub type TapestryWeaveInner =
    IndependentWeave<u64, NodeContent, MetadataMap, BuildHasherDefault<RandomIdHasher>>;

pub struct TapestryWeave {
    weave: TapestryWeaveInner,
    active: Vec<u64>,
    changed: bool,
    changed_shape: bool,
}

impl From<TapestryWeaveInner> for TapestryWeave {
    fn from(mut value: TapestryWeaveInner) -> Self {
        let mut active = Vec::with_capacity(value.capacity());
        active.extend(value.get_active_thread());

        Self {
            active,
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
        Ok(VersionedBytes {
            version: 1,
            data: MixedData::Output(self.to_unversioned_bytes()?),
        }
        .to_bytes())
    }
    /*pub fn to_versioned_weave(self) -> VersionedWeave {
        VersionedWeave::V1(self)
    }*/
    pub fn with_capacity(capacity: usize, metadata: MetadataMap) -> Self {
        Self {
            weave: IndependentWeave::with_capacity(capacity, metadata),
            active: Vec::with_capacity(capacity),
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
    }
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.weave.shrink_to(min_capacity);
        self.active.shrink_to(min_capacity);
    }
    pub fn metadata(&mut self) -> &mut MetadataMap {
        &mut self.weave.metadata
    }
    pub fn len(&self) -> usize {
        self.weave.len()
    }
    pub fn is_empty(&self) -> bool {
        self.weave.is_empty()
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
        let thread: Vec<u64> = self.weave.get_thread_from(id).collect();

        thread.into_iter().filter_map(|id| self.weave.get_node(&id))
    }
    pub fn get_thread_from_ids(
        &mut self,
        id: &u64,
    ) -> impl DoubleEndedIterator<Item = u64> + ExactSizeIterator<Item = u64> {
        self.weave.get_thread_from(id)
    }
    fn update_shape_and_active(&mut self) {
        self.changed = true;
        self.changed_shape = true;
        self.active.clear();
        self.active.extend(self.weave.get_active_thread());
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
    pub fn set_node_bookmarked_status_u64(&mut self, id: &u64, value: bool) -> bool {
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
        duplicate: bool,
        mut id_generator: impl FnMut() -> u64,
    ) -> Option<(u64, u64)> {
        if duplicate {
            if let Some(mut node) = self.weave.get_node(id).cloned() {
                let from = id_generator();
                let to = id_generator();

                node.id = from;
                node.bookmarked = false;
                self.weave.add_node(node);

                if self.weave.split_node(&from, at, to) {
                    self.weave.get_contents_mut(&from).unwrap().modified = true;
                    self.weave.get_contents_mut(&to).unwrap().modified = true;
                    self.update_shape_and_active();
                    Some((from, to))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            let new_id = id_generator();

            if self.weave.split_node(id, at, new_id) {
                self.weave.get_contents_mut(id).unwrap().modified = true;
                self.weave.get_contents_mut(&new_id).unwrap().modified = true;
                self.update_shape_and_active();
                Some((*id, new_id))
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
    pub fn merge_with_parent_u64(&mut self, id: &u64) -> bool {
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
}

impl TapestryWeave {
    // TODO: (diff-based) set_active_content, insert_node_at, dump_identifiers_ordered
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
        self.weave
            .get_active_thread()
            .filter_map(|id| self.weave.get_node(&id))
    }
    pub fn get_thread_from(
        &mut self,
        id: &u64_le,
    ) -> impl DoubleEndedIterator<Item = &ArchivedTapestryNode> {
        self.weave
            .get_thread_from(id)
            .filter_map(|id| self.weave.get_node(&id))
    }
    pub fn get_active_content(&self) -> Vec<u8> {
        self.weave
            .get_active_thread()
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

impl From<OldNodeContent> for NodeContent {
    fn from(value: OldNodeContent) -> Self {
        todo!()
    }
}

impl From<OldTapestryWeave> for TapestryWeave {
    fn from(value: OldTapestryWeave) -> Self {
        /*let mut output =
            TapestryWeave::with_capacity(value.capacity(), value.weave.metadata.clone());

        for identifier in value.weave.get_ordered_node_identifiers() {
            let node = value.weave.get_node(&identifier).unwrap().clone();

            assert!(output.add_node(IndependentNode {
                id: node.id,
                from: IndexSet::from_iter(node.from.into_iter()),
                to: node.to,
                active: node.active,
                bookmarked: node.bookmarked,
                contents: node.contents.into(),
            }));
        }

        output*/

        todo!()
    }
}
