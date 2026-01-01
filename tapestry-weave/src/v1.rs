//! Experimental & untested

use std::{borrow::Cow, cmp::Ordering, collections::HashSet, hash::BuildHasherDefault};

use contracts::ensures;
use rustc_hash::FxBuildHasher;
use ulid::Ulid;
use universal_weave::{
    ArchivedWeave, DeduplicatableContents, DeduplicatableWeave, DiscreteContentResult,
    DiscreteContents, DiscreteWeave, IndependentContents, SemiIndependentWeave, Weave,
    independent::{ArchivedIndependentNode, IndependentNode, IndependentWeave},
    indexmap::{IndexMap, IndexSet},
    rkyv::{
        Archive, Deserialize, Serialize, collections::swiss_table::ArchivedIndexSet, from_bytes,
        rancor::Error, rend::u128_le, to_bytes, util::AlignedVec,
    },
};

#[cfg(feature = "serde")]
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};

use crate::{
    VersionedWeave,
    hashers::UlidHasher,
    v0::{NodeContent as OldNodeContent, TapestryWeave as OldTapestryWeave},
    versioning::{MixedData, VersionedBytes},
};

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub struct NodeContent {
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
        if self.metadata != value.metadata || self.creator != value.creator {
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
                DiscreteContentResult::One(self)
            }
        }
    }
}

impl NodeContent {
    fn is_mergeable_with(&self, value: &Self) -> bool {
        if self.metadata != value.metadata || self.creator != value.creator {
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
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub enum InnerNodeContent {
    Snippet(Vec<u8>),
    Tokens(Vec<InnerNodeToken>),
    Link(String),
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub struct InnerNodeToken {
    bytes: Vec<u8>,
    metadata: MetadataMap,
    counterfactual: Vec<CounterfactualToken>,
    original: Option<Vec<u8>>,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub struct CounterfactualToken {
    bytes: Vec<u8>,
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
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub enum Creator {
    Model(Option<Model>),
    Human(Option<Author>),
    Unknown,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub struct Model {
    pub label: String,
    pub identifier: Option<u128>,
    pub metadata: MetadataMap,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(SerdeSerialize, SerdeDeserialize))]
pub struct Author {
    pub label: String,
    pub identifier: Option<u128>,
}

pub type TapestryNode = IndependentNode<u128, NodeContent, BuildHasherDefault<UlidHasher>>;
pub type MetadataMap = IndexMap<String, String, FxBuildHasher>;
pub type ArchivedTapestryNode =
    ArchivedIndependentNode<u128, NodeContent, BuildHasherDefault<UlidHasher>>;
pub type TapestryWeaveInner =
    IndependentWeave<u128, NodeContent, MetadataMap, BuildHasherDefault<UlidHasher>>;

pub struct TapestryWeave {
    weave: TapestryWeaveInner,
    active: Vec<u128>,
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
    pub fn contains(&self, id: &Ulid) -> bool {
        self.weave.contains(&id.0)
    }
    pub fn contains_u128(&self, id: &u128) -> bool {
        self.weave.contains(id)
    }
    pub fn contains_active(&self, id: &Ulid) -> bool {
        self.weave.contains_active(&id.0)
    }
    pub fn contains_active_u128(&self, id: &u128) -> bool {
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
    pub fn get_node(&self, id: &Ulid) -> Option<&TapestryNode> {
        self.weave.get_node(&id.0)
    }
    pub fn get_node_u128(&self, id: &u128) -> Option<&TapestryNode> {
        self.weave.get_node(id)
    }
    pub fn get_node_children(
        &self,
        id: &u128,
    ) -> Option<&IndexSet<u128, BuildHasherDefault<UlidHasher>>> {
        self.weave.get_node(id).map(|node| &node.to)
    }
    pub fn get_node_parents(
        &self,
        id: &u128,
    ) -> Option<&IndexSet<u128, BuildHasherDefault<UlidHasher>>> {
        self.weave.get_node(id).map(|node| &node.from)
    }
    pub fn get_node_siblings(&self, id: &u128) -> Option<impl DoubleEndedIterator<Item = u128>> {
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
        id: &u128,
    ) -> Option<Box<dyn DoubleEndedIterator<Item = u128> + 's>> {
        self.weave.get_node(id).map(|node| {
            if node.from.is_empty() {
                Box::new(self.weave.roots().iter().copied().filter(|sibling| {
                    *sibling != node.id
                        && !node.from.contains(sibling)
                        && !node.to.contains(sibling)
                })) as Box<dyn DoubleEndedIterator<Item = u128>>
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
                ) as Box<dyn DoubleEndedIterator<Item = u128>>
            }
        })
    }
    pub fn get_roots(&self) -> impl ExactSizeIterator<Item = Ulid> {
        self.weave.roots().iter().copied().map(Ulid)
    }
    pub fn get_roots_u128(&self) -> impl ExactSizeIterator<Item = u128> {
        self.weave.roots().iter().copied()
    }
    pub fn get_roots_u128_direct(&self) -> &IndexSet<u128, BuildHasherDefault<UlidHasher>> {
        self.weave.roots()
    }
    pub fn get_bookmarks(&self) -> impl ExactSizeIterator<Item = Ulid> {
        self.weave.bookmarks().iter().copied().map(Ulid)
    }
    pub fn get_bookmarks_u128(&self) -> impl ExactSizeIterator<Item = u128> {
        self.weave.bookmarks().iter().copied()
    }
    pub fn get_bookmarks_u128_direct(&self) -> &IndexSet<u128, BuildHasherDefault<UlidHasher>> {
        self.weave.bookmarks()
    }
    pub fn get_active_thread(&mut self) -> impl DoubleEndedIterator<Item = &TapestryNode> {
        self.active.iter().filter_map(|id| self.weave.get_node(id))
    }
    pub fn get_active_thread_u128(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = u128> + ExactSizeIterator<Item = u128> {
        self.active.iter().copied()
    }
    pub fn get_thread_from(&mut self, id: &Ulid) -> impl DoubleEndedIterator<Item = &TapestryNode> {
        let thread: Vec<u128> = self.weave.get_thread_from(&id.0).collect();

        thread.into_iter().filter_map(|id| self.weave.get_node(&id))
    }
    pub fn get_thread_from_u128(
        &mut self,
        id: &u128,
    ) -> impl DoubleEndedIterator<Item = u128> + ExactSizeIterator<Item = u128> {
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
        let last_active_set: HashSet<u128, BuildHasherDefault<UlidHasher>> = if node.active {
            HashSet::from_iter(self.active.iter().copied())
        } else {
            HashSet::default()
        };
        let is_active = node.active;

        let status = self.weave.add_node(node);

        if status {
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

            self.update_shape_and_active();
        }

        status
    }
    pub fn set_node_active_status(&mut self, id: &Ulid, value: bool, alternate: bool) -> bool {
        self.set_node_active_status_u128(&id.0, value, alternate)
    }
    pub fn set_node_active_status_u128(&mut self, id: &u128, value: bool, alternate: bool) -> bool {
        if self.weave.set_node_active_status(id, value, alternate) {
            self.update_shape_and_active();
            true
        } else {
            false
        }
    }
    pub fn set_node_active_status_in_place(&mut self, id: &Ulid, value: bool) -> bool {
        self.set_node_active_status_in_place_u128(&id.0, value)
    }
    pub fn set_node_active_status_in_place_u128(&mut self, id: &u128, value: bool) -> bool {
        if self.weave.set_node_active_status_in_place(id, value) {
            self.update_shape_and_active();
            true
        } else {
            false
        }
    }
    pub fn set_node_bookmarked_status(&mut self, id: &Ulid, value: bool) -> bool {
        self.set_node_bookmarked_status_u128(&id.0, value)
    }
    pub fn set_node_bookmarked_status_u128(&mut self, id: &u128, value: bool) -> bool {
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
    pub fn split_node(&mut self, id: &Ulid, at: usize, duplicate: bool) -> Option<(Ulid, Ulid)> {
        if duplicate {
            if let Some(mut node) = self.weave.get_node(&id.0).cloned() {
                let from = Ulid::from_datetime(id.datetime());
                let to = Ulid::from_datetime(id.datetime());

                node.id = from.0;
                node.bookmarked = false;
                self.weave.add_node(node);

                if self.weave.split_node(&from.0, at, to.0) {
                    self.weave.get_contents_mut(&from.0).unwrap().modified = true;
                    self.weave.get_contents_mut(&to.0).unwrap().modified = true;
                    self.update_shape_and_active();
                    Some((from, to))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            let new_id = Ulid::from_datetime(id.datetime());

            if self.weave.split_node(&id.0, at, new_id.0) {
                self.weave.get_contents_mut(&id.0).unwrap().modified = true;
                self.weave.get_contents_mut(&new_id.0).unwrap().modified = true;
                self.update_shape_and_active();
                Some((*id, new_id))
            } else {
                None
            }
        }
    }
    pub fn split_node_direct(&mut self, id: &u128, at: usize, new_id: u128) -> Option<u128> {
        if self.weave.split_node(id, at, new_id) {
            self.weave.get_contents_mut(id).unwrap().modified = true;
            self.weave.get_contents_mut(&new_id).unwrap().modified = true;
            self.update_shape_and_active();
            Some(new_id)
        } else {
            None
        }
    }
    pub fn merge_with_parent(&mut self, id: &Ulid) -> bool {
        self.merge_with_parent_u128(&id.0)
    }
    pub fn merge_with_parent_u128(&mut self, id: &u128) -> bool {
        if let Some(new_id) = self.weave.merge_with_parent(id) {
            self.weave.get_contents_mut(&new_id).unwrap().modified = true;
            self.update_shape_and_active();
            true
        } else {
            false
        }
    }
    pub fn is_mergeable_with_parent(&self, id: &Ulid) -> bool {
        self.is_mergeable_with_parent_u128(&id.0)
    }
    pub fn is_mergeable_with_parent_u128(&self, id: &u128) -> bool {
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
    pub fn remove_node(&mut self, id: &Ulid) -> Option<TapestryNode> {
        self.remove_node_u128(&id.0)
    }
    pub fn remove_node_u128(&mut self, id: &u128) -> Option<TapestryNode> {
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
        id: &u128,
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
    pub fn contains(&self, id: &u128_le) -> bool {
        self.weave.contains(id)
    }
    pub fn contains_active(&self, id: &u128_le) -> bool {
        self.weave.contains_active(id)
    }
    pub fn get_node(&self, id: &u128_le) -> Option<&ArchivedTapestryNode> {
        self.weave.get_node(id)
    }
    pub fn get_node_children(&self, id: &u128_le) -> Option<&ArchivedIndexSet<u128_le>> {
        self.weave.get_node(id).map(|node| &node.to)
    }
    pub fn get_node_parents(&self, id: &u128_le) -> Option<&ArchivedIndexSet<u128_le>> {
        self.weave.get_node(id).map(|node| &node.from)
    }
    pub fn get_node_siblings(&self, id: &u128_le) -> Option<impl Iterator<Item = u128_le>> {
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
        id: &u128_le,
    ) -> Option<Box<dyn Iterator<Item = u128_le> + 's>> {
        self.weave.get_node(id).map(|node| {
            if node.from.is_empty() {
                Box::new(self.weave.roots().iter().copied().filter(|sibling| {
                    *sibling != node.id
                        && !node.from.contains(sibling)
                        && !node.to.contains(sibling)
                })) as Box<dyn Iterator<Item = u128_le>>
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
                ) as Box<dyn Iterator<Item = u128_le>>
            }
        })
    }
    pub fn get_roots(&self) -> impl ExactSizeIterator<Item = u128_le> {
        self.weave.roots().iter().copied()
    }
    pub fn get_bookmarks(&self) -> impl ExactSizeIterator<Item = u128_le> {
        self.weave.bookmarks().iter().copied()
    }
    pub fn get_active_thread(&mut self) -> impl DoubleEndedIterator<Item = &ArchivedTapestryNode> {
        self.weave
            .get_active_thread()
            .filter_map(|id| self.weave.get_node(&id))
    }
    pub fn get_thread_from(
        &mut self,
        id: &u128_le,
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
    pub fn is_mergeable_with_parent_u128(&self, id: &u128_le) -> bool {
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
        let mut output =
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

        output
    }
}
