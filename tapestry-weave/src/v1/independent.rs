//! Experimental & untested

use std::{cmp::Ordering, collections::HashSet, hash::BuildHasherDefault, num::NonZeroU128};

use jiff::Zoned;
use nanorand::WyRand;
use universal_weave::{
    ActivePathWeave, ArchivedWeave, DeduplicatableWeave, DiscreteWeave, Weave,
    independent::{ArchivedIndependentNode, IndependentNode, IndependentWeave},
    indexmap::IndexSet,
    rkyv::{
        Archive, access, access_unchecked, api::high::to_bytes_in,
        collections::swiss_table::ArchivedIndexSet, deserialize, from_bytes, from_bytes_unchecked,
        rancor::Error, rend::u64_le, ser::Writer,
    },
};

#[cfg(feature = "v0")]
use jiff::Timestamp;

#[cfg(feature = "v0")]
use ulid::Ulid;

#[cfg(feature = "v0")]
use nanorand::Rng;

#[cfg(feature = "v0")]
use crate::{
    hashers::UlidHasher, v0::TapestryWeave as OldTapestryWeave, wrappers::UniqueIdentifierRemapper,
};

use super::{
    super::{VersionedWeave, hashers::RandomIdHasher, v1::metadata::ConvertedFrom, write_header},
    content::{InnerNodeContent, NodeContent},
    dependent::TapestryWeave as DependentTapestryWeave,
    generate_unique_id, generate_unique_id_with_list,
    metadata::{ArchivedWeaveMetadata, WeaveMetadata},
};

pub(crate) const FORMAT_VERSION: u64 = 2;

pub type ShortId = u64;
pub type LongId = NonZeroU128;
pub type TapestryNode = IndependentNode<u64, NodeContent, BuildHasherDefault<RandomIdHasher>>;
pub type ArchivedTapestryNode =
    ArchivedIndependentNode<u64, NodeContent, BuildHasherDefault<RandomIdHasher>>;
pub type TapestryWeaveInner =
    IndependentWeave<u64, NodeContent, WeaveMetadata, BuildHasherDefault<RandomIdHasher>>;
pub type ArchivedTapestryWeaveInner = <TapestryWeaveInner as Archive>::Archived;

pub struct TapestryWeave {
    pub rng: WyRand,
    weave: TapestryWeaveInner,
    active: Vec<u64>,
    scratchpad: Vec<u64>,
    scratchpad_2: HashSet<u64, BuildHasherDefault<RandomIdHasher>>,
    changed: bool,
    changed_shape: bool,
}

impl From<TapestryWeaveInner> for TapestryWeave {
    fn from(mut value: TapestryWeaveInner) -> Self {
        let mut active = Vec::with_capacity(value.capacity());
        value.get_active_thread(&mut active);

        Self {
            rng: WyRand::new(),
            active,
            scratchpad: Vec::with_capacity(value.capacity()),
            scratchpad_2: HashSet::with_capacity_and_hasher(
                value.capacity(),
                BuildHasherDefault::default(),
            ),
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
        VersionedWeave::V1Independent(self)
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            rng: WyRand::new(),
            weave: IndependentWeave::with_capacity(capacity, WeaveMetadata::new()),
            active: Vec::with_capacity(capacity),
            scratchpad: Vec::with_capacity(capacity),
            scratchpad_2: HashSet::with_capacity_and_hasher(
                capacity,
                BuildHasherDefault::default(),
            ),
            changed: false,
            changed_shape: false,
        }
    }
    pub fn with_capacity_and_metadata(capacity: usize, metadata: WeaveMetadata) -> Self {
        Self {
            rng: WyRand::new(),
            weave: IndependentWeave::with_capacity(capacity, metadata),
            active: Vec::with_capacity(capacity),
            scratchpad: Vec::with_capacity(capacity),
            scratchpad_2: HashSet::with_capacity_and_hasher(
                capacity,
                BuildHasherDefault::default(),
            ),
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
            .reserve(self.weave.capacity().saturating_sub(self.active.len()));
        self.scratchpad
            .reserve(self.weave.capacity().saturating_sub(self.scratchpad.len()));
        self.scratchpad_2.reserve(
            self.weave
                .capacity()
                .saturating_sub(self.scratchpad_2.len()),
        );
    }
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.weave.shrink_to(min_capacity);
        self.active.shrink_to(min_capacity);
        self.scratchpad.shrink_to(min_capacity);
        self.scratchpad_2.shrink_to(min_capacity);
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
        self.weave.contains_active(id)
    }
    pub fn generate_id(&mut self) -> u64 {
        generate_unique_id(&mut self.rng, &self.weave)
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
    pub fn get_active_thread(&self) -> impl DoubleEndedIterator<Item = &TapestryNode> {
        self.active.iter().filter_map(|id| self.weave.get_node(id))
    }
    pub fn get_active_thread_ids(&self) -> &Vec<u64> {
        &self.active
    }
    pub fn get_active_thread_id_set(&self) -> &HashSet<u64, BuildHasherDefault<RandomIdHasher>> {
        self.weave.active()
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
        self.scratchpad_2.clear();
        if node.active {
            self.scratchpad_2.extend(self.active.iter().copied())
        };
        let is_active = node.active;

        let status = self.weave.add_node(node);

        if status {
            let duplicates: Vec<u64> = self.weave.find_duplicates(&identifier).collect();

            if !duplicates.is_empty() {
                if is_active {
                    let mut has_active = false;

                    for duplicate in &duplicates {
                        if self.scratchpad_2.contains(duplicate) {
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
    pub fn get_active_content(&self, output: &mut Vec<u8>) {
        output.clear();

        for node in self
            .active
            .iter()
            .rev()
            .filter_map(|id| self.weave.get_node(id))
        {
            output.extend(node.contents.content.as_bytes().iter().copied());
        }
    }
    pub fn split_node(&mut self, id: &u64, at: usize) -> Option<(u64, Option<u64>, u64)> {
        let new_id = generate_unique_id(&mut self.rng, &self.weave);

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
                let first_split_id =
                    generate_unique_id_with_list(&mut self.rng, &self.weave, &[new_id]);
                let second_split_id = generate_unique_id_with_list(
                    &mut self.rng,
                    &self.weave,
                    &[new_id, first_split_id],
                );

                assert!(self.weave.split_node(id, byte_index, first_split_id));

                let mut token_node = self.weave.get_node(&first_split_id).unwrap().clone();
                token_node.id = new_id;

                //token_node.to = IndexSet::default();
                token_node.contents.content.truncate_tokens(1);

                let token_node_id = token_node.id;

                assert!(self.weave.add_node(token_node));

                assert!(
                    self.weave
                        .split_node(&token_node_id, at - byte_index, second_split_id)
                );

                self.update_shape_and_active();

                Some((*id, Some(token_node_id), second_split_id))
            } else {
                if self.weave.split_node(id, at, new_id) {
                    self.update_shape_and_active();
                    Some((*id, None, new_id))
                } else {
                    None
                }
            }
        } else {
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
    ) -> Option<(Option<u64>, u64, Option<u64>)> {
        if let Some(result) = self.split_out_token_inner(id, index) {
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
    ) -> Option<(Option<u64>, u64, Option<u64>)> {
        // before_token, token, after_token
        if let Some(node) = self.weave.get_node(id) {
            if let InnerNodeContent::Tokens(tokens) = &node.contents.content
                && tokens.len() > index
            {
                let tail_id = generate_unique_id(&mut self.rng, &self.weave);

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
                    let middle_id =
                        generate_unique_id_with_list(&mut self.rng, &self.weave, &[tail_id]);

                    assert!(self.weave.split_node(id, split_index, middle_id));

                    if let Some(second_split_index) = second_split_index
                        && second_split_index > 0
                    {
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
        if self.weave.sort_node_children_by(id, compare) {
            self.changed = true;
            self.changed_shape = true;
            true
        } else {
            false
        }
    }
    pub fn sort_bookmarks_by(
        &mut self,
        compare: impl FnMut(&TapestryNode, &TapestryNode) -> Ordering,
    ) {
        self.changed = true;
        self.changed_shape = true;
        self.weave.sort_bookmarks_by(compare)
    }
    pub fn modify_inner<T>(
        &mut self,
        callback: impl FnOnce(&mut WyRand, &mut TapestryWeaveInner, &[u64]) -> T,
    ) -> T {
        let output = callback(&mut self.rng, &mut self.weave, &self.active);
        self.update_shape_and_active();

        output
    }
}

impl TapestryWeave {
    // TODO: diff-based set_active_content, insert_node_at, remove_active_range
    /*#[ensures(self.get_active_content() == value)]
    #[allow(non_snake_case)]
    pub fn TEMPORARY_v0_like_set_active_content(
        &mut self,
        value: &[u8],
        creator: Creator,
        mut id_generator: impl FnMut() -> u64,
    ) {
        let mut modified = false;
        let mut offset: usize = 0;

        let value_len = value.len();

        let active_node = self.active.first().copied();

        let mut last_node = None;
        let mut last_index = None;

        for (index, active_identifier) in self.active.iter().copied().enumerate().rev() {
            let node = self.weave.get_node(&active_identifier).unwrap();
            let content_bytes = node.contents.content.as_bytes();

            let content_len = content_bytes.len();

            if value_len >= offset + content_len
                && value[offset..(offset + content_len)] == *content_bytes
            {
                offset += content_len;
                last_node = Some(node.id);
                last_index = Some(index);
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
                        assert!(
                            self.split_node(&target, offset - start_offset, &mut id_generator)
                                .is_some()
                        );

                        last_node = Some(target);
                        last_index = Some(index);
                    } else {
                        last_node = None;
                        last_index = None;
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
            && node.contents.creator == creator
            && node.contents.metadata.is_empty()
        {
            last_node = last_index
                .filter(|i| *i > 0)
                .and_then(|i| self.active.get(i - 1).copied());
            /*.or_else(|| {
                node.from
                    .iter()
                    .copied()
                    .find(|id| self.weave.contains_active(id))
            }); */

            let identifier = node.id;
            if let Some(node) = self.weave.remove_node(&identifier) {
                offset -= node.contents.content.len();
            }
        }

        if offset < value.len() {
            assert!(self.add_node(TapestryNode {
                id: id_generator(),
                from: if let Some(last_node) = last_node {
                    IndexSet::from_iter(iter::once(last_node))
                } else {
                    IndexSet::default()
                },
                to: IndexSet::default(),
                active: true,
                bookmarked: false,
                contents: NodeContent {
                    timestamp: Zoned::now(),
                    modified: false,
                    content: InnerNodeContent::Snippet(value[offset..].to_vec()),
                    metadata: IndexMap::default(),
                    creator,
                },
            }));

            modified = true;
        }

        if modified {
            self.update_shape_and_active();
        }
    }*/
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
                                    && !node.from.contains(sibling)
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
impl From<OldTapestryWeave> for TapestryWeave {
    fn from(mut value: OldTapestryWeave) -> Self {
        let mut output = TapestryWeave::with_capacity_and_metadata(
            value.capacity(),
            value.weave.metadata.clone().into(),
        );

        let mut identifiers = Vec::with_capacity(value.weave.len());
        value.weave.get_ordered_node_identifiers(&mut identifiers);

        let mut mapper: UniqueIdentifierRemapper<
            u128,
            u64,
            BuildHasherDefault<UlidHasher>,
            BuildHasherDefault<RandomIdHasher>,
        > = UniqueIdentifierRemapper::with_capacity(identifiers.len());

        let time_zone = output.metadata().created.time_zone().clone();

        output.modify_inner(|rng, output, _| {
            let mut convert_old_identifier = move |id| {
                *mapper
                    .map_with_initial(
                        id,
                        unsafe { std::mem::transmute::<u128, [u64; 2]>(id)[1] },
                        || rng.generate(),
                    )
                    .get()
            };

            for identifier in identifiers {
                let node = value.weave.get_node(&identifier).unwrap().clone();

                let timestamp = Timestamp::try_from(Ulid(node.id).datetime())
                    .map(|timestamp| Zoned::new(timestamp, time_zone.clone()))
                    .unwrap_or(Zoned::default());

                let mut node = TapestryNode {
                    id: convert_old_identifier(node.id),
                    from: IndexSet::from_iter(
                        node.from.into_iter().map(&mut convert_old_identifier),
                    ),
                    to: IndexSet::with_capacity_and_hasher(
                        node.to.len(),
                        BuildHasherDefault::default(),
                    ),
                    active: node.active,
                    bookmarked: node.bookmarked,
                    contents: node.contents.into(),
                };
                node.contents.timestamp = timestamp;

                assert!(output.add_node(node));
            }
        });

        output
    }
}

impl From<DependentTapestryWeave> for TapestryWeave {
    fn from(value: DependentTapestryWeave) -> Self {
        let mut weave = TapestryWeave::from(TapestryWeaveInner::from(value.weave));

        weave
            .weave
            .metadata
            .converted_from
            .push(ConvertedFrom::from_v1_dependent(Zoned::now()));

        weave
    }
}
