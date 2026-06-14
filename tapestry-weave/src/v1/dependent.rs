//! Experimental & untested

use std::{cmp::Ordering, collections::HashSet, hash::BuildHasherDefault, num::NonZeroU128};

use universal_weave::{
    ArchivedWeave, DeduplicatableWeave, DiscreteWeave, Weave,
    dependent::{ArchivedDependentNode, DependentNode, DependentWeave},
    indexmap::IndexSet,
    rkyv::{
        Archive, access, access_unchecked, api::high::to_bytes_in,
        collections::swiss_table::ArchivedIndexSet, deserialize, from_bytes, from_bytes_unchecked,
        option::ArchivedOption, rancor::Error, rend::u64_le, ser::Writer,
    },
};

#[cfg(feature = "v0")]
use jiff::Zoned;

#[cfg(feature = "v0")]
use ulid::Ulid;

#[cfg(feature = "v0")]
use crate::v0::TapestryWeave as OldTapestryWeave;

use super::{
    super::{VersionedWeave, hashers::RandomIdHasher, write_header},
    content::{InnerNodeContent, NodeContent},
    metadata::{ArchivedWeaveMetadata, WeaveMetadata},
};

pub(crate) const FORMAT_VERSION: u64 = 1;

pub type ShortId = u64;
pub type LongId = NonZeroU128;
pub type TapestryNode = DependentNode<u64, NodeContent, BuildHasherDefault<RandomIdHasher>>;
pub type ArchivedTapestryNode =
    ArchivedDependentNode<u64, NodeContent, BuildHasherDefault<RandomIdHasher>>;
pub type TapestryWeaveInner =
    DependentWeave<u64, NodeContent, WeaveMetadata, BuildHasherDefault<RandomIdHasher>>;
pub type ArchivedTapestryWeaveInner = <TapestryWeaveInner as Archive>::Archived;

pub struct TapestryWeave {
    pub(super) weave: TapestryWeaveInner,
    active: Vec<u64>,
    active_set: HashSet<u64, BuildHasherDefault<RandomIdHasher>>,
    scratchpad: Vec<u64>,
    changed: bool,
    changed_shape: bool,
}

impl From<TapestryWeaveInner> for TapestryWeave {
    fn from(mut value: TapestryWeaveInner) -> Self {
        let mut active = Vec::with_capacity(value.capacity());
        value.get_active_thread(&mut active);

        let mut active_set =
            HashSet::with_capacity_and_hasher(value.capacity(), BuildHasherDefault::default());
        active_set.extend(active.iter().copied());

        Self {
            active,
            active_set,
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
    pub unsafe fn from_unversioned_bytes_unchecked(bytes: &[u8]) -> Result<Self, Error> {
        Ok(Self::from(unsafe {
            from_bytes_unchecked::<TapestryWeaveInner, Error>(bytes)?
        }))
    }
    pub fn from_archived(value: &ArchivedTapestryWeave) -> Result<Self, Error> {
        Ok(Self::from(deserialize::<TapestryWeaveInner, Error>(
            value.weave,
        )?))
    }
    pub fn write_unversioned_bytes<W: Writer<Error>>(&self, writer: W) -> Result<W, Error> {
        assert!(self.weave.validate());
        to_bytes_in::<W, Error>(&self.weave, writer)
    }
    pub fn write_versioned_bytes<W: Writer<Error>>(&self, mut writer: W) -> Result<W, Error> {
        assert!(self.weave.validate());
        write_header(&mut writer, FORMAT_VERSION)?;
        to_bytes_in::<W, Error>(&self.weave, writer)
    }
    pub fn to_versioned_weave(self) -> VersionedWeave {
        VersionedWeave::V1Dependent(self)
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            weave: DependentWeave::with_capacity(capacity, WeaveMetadata::new()),
            active: Vec::with_capacity(capacity),
            active_set: HashSet::with_capacity_and_hasher(capacity, BuildHasherDefault::default()),
            scratchpad: Vec::with_capacity(capacity),
            changed: false,
            changed_shape: false,
        }
    }
    pub fn with_capacity_and_metadata(capacity: usize, metadata: WeaveMetadata) -> Self {
        Self {
            weave: DependentWeave::with_capacity(capacity, metadata),
            active: Vec::with_capacity(capacity),
            active_set: HashSet::with_capacity_and_hasher(capacity, BuildHasherDefault::default()),
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
        self.active_set.reserve(
            self.weave
                .capacity()
                .saturating_sub(self.active_set.capacity()),
        );
        self.scratchpad.reserve(
            self.weave
                .capacity()
                .saturating_sub(self.scratchpad.capacity()),
        );
    }
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.weave.shrink_to(min_capacity);
        self.active.shrink_to(min_capacity);
        self.active_set.shrink_to(min_capacity);
        self.scratchpad.shrink_to(min_capacity);
    }
    pub fn metadata(&mut self) -> &mut WeaveMetadata {
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
        self.active_set.contains(id)
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
    pub fn get_node_parent(&self, id: &u64) -> Option<&Option<u64>> {
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
                            && node.from != Some(*sibling)
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
            if node.from.is_none() {
                Box::new(
                    self.weave
                        .roots()
                        .iter()
                        .copied()
                        .filter(|sibling| *sibling != node.id && !node.to.contains(sibling)),
                ) as Box<dyn DoubleEndedIterator<Item = u64>>
            } else {
                Box::new(
                    node.from
                        .iter()
                        .filter_map(|parent| self.weave.get_node(parent))
                        .flat_map(|parent| {
                            parent.to.iter().copied().filter(|sibling| {
                                *sibling != node.id
                                    && node.from != Some(*sibling)
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
    pub fn get_active_thread_ids(&mut self) -> &Vec<u64> {
        &self.active
    }
    pub fn get_active_thread_id_set(
        &mut self,
    ) -> &HashSet<u64, BuildHasherDefault<RandomIdHasher>> {
        &self.active_set
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
        self.weave.get_active_thread(&mut self.active);
        self.active_set.clear();
        self.active_set.extend(self.active.iter().copied());
    }
    pub fn add_node(&mut self, node: TapestryNode) -> bool {
        let identifier = node.id;
        let last_active_set: HashSet<u64, BuildHasherDefault<RandomIdHasher>> =
            self.active_set.clone();
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
            .flat_map(|node| node.contents.content.as_bytes().into_owned())
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
                    self.update_shape_and_active();
                    Some((*id, None, new_id))
                } else {
                    None
                }
            }
        } else {
            let new_id = id_generator();

            if self.weave.split_node(id, at, new_id) {
                self.update_shape_and_active();
                Some((*id, None, new_id))
            } else {
                None
            }
        }
    }
    pub fn split_node_direct(&mut self, id: &u64, at: usize, new_id: u64) -> bool {
        if self.weave.split_node(id, at, new_id) {
            self.update_shape_and_active();
            true
        } else {
            false
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
                    .or(node.from);

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

                    if let Some(second_split_index) = second_split_index
                        && second_split_index > 0
                    {
                        let tail_id = id_generator();

                        assert!(
                            self.weave
                                .split_node(&middle_id, second_split_index, tail_id)
                        );

                        Some((Some(*id), middle_id, Some(tail_id)))
                    } else {
                        Some((Some(*id), middle_id, None))
                    }
                } else if let Some(second_split_index) = second_split_index
                    && second_split_index > 0
                {
                    let tail_id = id_generator();

                    assert!(self.weave.split_node(id, second_split_index, tail_id));

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
    pub fn merge_with_parent(&mut self, id: &u64) -> Option<u64> {
        if let Some(new_id) = self.weave.merge_with_parent(id) {
            self.update_shape_and_active();
            Some(new_id)
        } else {
            None
        }
    }
    pub fn is_mergeable_with_parent(&self, id: &u64) -> bool {
        if let Some(node) = self.weave.get_node(id) {
            if let Some(parent) = node.from.as_ref().and_then(|id| self.weave.get_node(id)) {
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
        if self.weave.sort_node_children_by(id, compare) {
            self.changed = true;
            self.changed_shape = true;
            true
        } else {
            false
        }
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

pub struct ArchivedTapestryWeave<'a> {
    pub weave: &'a ArchivedTapestryWeaveInner,
}

impl AsRef<ArchivedTapestryWeaveInner> for ArchivedTapestryWeave<'_> {
    fn as_ref(&self) -> &ArchivedTapestryWeaveInner {
        self.weave
    }
}

impl<'a> ArchivedTapestryWeave<'a> {
    pub fn from_unversioned_bytes(bytes: &'a [u8]) -> Result<Self, Error> {
        Ok(Self {
            weave: access::<ArchivedTapestryWeaveInner, Error>(bytes)?,
        })
    }
    pub unsafe fn from_unversioned_bytes_unchecked(bytes: &'a [u8]) -> Self {
        Self {
            weave: unsafe { access_unchecked::<ArchivedTapestryWeaveInner>(bytes) },
        }
    }
    pub fn metadata(&self) -> &ArchivedWeaveMetadata {
        &self.weave.metadata
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
    pub fn contains(&self, id: &u64_le) -> bool {
        self.weave.contains(id)
    }
    pub fn contains_active(&self, id: &u64_le) -> bool {
        self.weave.contains_active(id)
    }
    pub fn dump_identifiers_ordered(&mut self, output: &mut Vec<u64_le>) {
        self.weave.get_ordered_node_identifiers(output);
    }
    pub fn dump_identifiers_ordered_rev(&mut self, output: &mut Vec<u64_le>) {
        self.weave
            .get_ordered_node_identifiers_reversed_children(output)
    }
    pub fn get_node(&self, id: &u64_le) -> Option<&ArchivedTapestryNode> {
        self.weave.get_node(id)
    }
    pub fn get_node_children(&self, id: &u64_le) -> Option<&ArchivedIndexSet<u64_le>> {
        self.weave.get_node(id).map(|node| &node.to)
    }
    pub fn get_node_parent(&self, id: &u64_le) -> Option<&ArchivedOption<u64_le>> {
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
                            && node.from != Some(*sibling)
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
            if node.from.is_none() {
                Box::new(
                    self.weave
                        .roots()
                        .iter()
                        .copied()
                        .filter(|sibling| *sibling != node.id && !node.to.contains(sibling)),
                ) as Box<dyn Iterator<Item = u64_le>>
            } else {
                Box::new(
                    node.from
                        .iter()
                        .filter_map(|parent| self.weave.get_node(parent))
                        .flat_map(|parent| {
                            parent.to.iter().copied().filter(|sibling| {
                                *sibling != node.id
                                    && node.from != Some(*sibling)
                                    && !node.to.contains(sibling)
                            })
                        }),
                ) as Box<dyn Iterator<Item = u64_le>>
            }
        })
    }
    pub fn roots(&self) -> &ArchivedIndexSet<u64_le> {
        self.weave.roots()
    }
    pub fn bookmarks(&self) -> &ArchivedIndexSet<u64_le> {
        self.weave.bookmarks()
    }
    pub fn get_active_thread(&self) -> impl DoubleEndedIterator<Item = &ArchivedTapestryNode> {
        let mut scratchpad = Vec::with_capacity(self.weave.len());

        self.weave.get_active_thread(&mut scratchpad);

        scratchpad
            .into_iter()
            .filter_map(|id| self.weave.get_node(&id))
    }
    pub fn get_active_thread_ids(&self) -> Vec<u64_le> {
        let mut scratchpad = Vec::with_capacity(self.weave.len());

        self.weave.get_active_thread(&mut scratchpad);

        scratchpad
    }
    pub fn get_thread_from(
        &self,
        id: &u64_le,
    ) -> impl DoubleEndedIterator<Item = &ArchivedTapestryNode> {
        let mut scratchpad = Vec::with_capacity(self.weave.len());

        self.weave.get_thread_from(id, &mut scratchpad);

        scratchpad
            .into_iter()
            .filter_map(|id| self.weave.get_node(&id))
    }
    pub fn get_thread_from_ids(&self, id: &u64_le) -> Vec<u64_le> {
        let mut scratchpad = Vec::with_capacity(self.weave.len());

        self.weave.get_thread_from(id, &mut scratchpad);

        scratchpad
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
                from: node.from.map(convert_old_identifier),
                to: IndexSet::from_iter(
                    node.to
                        .into_iter()
                        .map(convert_old_identifier)
                        .filter(|id| output.contains(id)),
                ),
                active: node.active,
                bookmarked: node.bookmarked,
                contents: node.contents.into(),
            };
            node.contents.timestamp = timestamp;

            assert!(output.add_node_direct(node));
        }

        output
    }
}
