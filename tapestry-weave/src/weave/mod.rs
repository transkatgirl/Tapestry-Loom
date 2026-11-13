//! Document format implementations.

use std::{
    cmp::{Ordering, Reverse},
    hash::BuildHasherDefault,
    iter, mem,
    num::NonZeroU128,
    ops::{ControlFlow, Range},
    sync::{Arc, atomic::AtomicU64},
    time::{Duration, Instant},
};

use jiff::Zoned;
use unicode_segmentation::UnicodeSegmentation;
use universal_weave::{
    ActivePathWeave, BookmarkableWeave, DeduplicatableContents, DiscreteWeave,
    ImmutableActivePathWeave, ImmutableBookmarkableWeave, ImmutableMetadataWeave, ImmutableWeave,
    IndependentWeave as IndependentWeaveTrait, MetadataWeave, SemiIndependentWeave,
    SortableBookmarkableWeave, SortableWeave, Weave,
    hashbrown::{HashMap, HashSet},
    independent::{IndependentNode, IndependentWeave},
    indexmap::IndexSet,
    rkyv::{
        Archive,
        collections::swiss_table::{ArchivedHashMap, ArchivedHashSet, ArchivedIndexSet},
    },
    wrappers::{PatchablePathWeave, PositionAlignment, PositionedWeave},
};

use crate::{
    content::{
        ArchivedNodeContent, Author, Creator, InnerNodeContent, NodeContent, from_utf8_lossy,
    },
    metadata::{ArchivedWeaveMetadata, MetadataMap, WeaveMetadata},
    util::{Hunk, RandomIdHasher},
};

pub mod layout;
pub mod wrappers;

/// The file extension used for Tapestry Loom documents.
pub const FILE_EXTENSION: &str = "tapestry";

/// The magic bytes at the start of every Tapestry Loom document.
pub const HEADER_MAGIC_BYTES: [u8; 24] = *b"VersionedTapestryWeave__";

pub(crate) const FORMAT_VERSION: u64 = 1;

const DIFF_DEADLINE: Duration = Duration::from_millis(20); // 60% of the frame budget in a 30fps UI.
const DIFF_SLIDE_LIMIT: usize = 256;

/// A *randomly generated* node identifier used within a [`TapestryWeave`].
///
/// This identifier must be unique within the document, but may not be unique between documents.
pub type ShortId = u64;

/// A *randomly generated* node identifier used within an [`ArchivedTapestryWeave`].
///
/// This identifier must be unique within the document, but may not be unique between documents.
pub type ArchivedShortId = <ShortId as Archive>::Archived;

/// A universally unique identifier used for entities referenced in a [`TapestryWeave`].
pub type LongId = NonZeroU128;

/// A node in a [`TapestryWeave`] document.
pub type TapestryNode = IndependentNode<ShortId, NodeContent, BuildHasherDefault<RandomIdHasher>>;

/// A node in an [`ArchivedTapestryWeave`] document.
pub type ArchivedTapestryNode = <TapestryNode as Archive>::Archived;

/// The inner contents of a [`TapestryWeave`].
pub type TapestryWeaveInner =
    IndependentWeave<ShortId, NodeContent, WeaveMetadata, BuildHasherDefault<RandomIdHasher>>;

/// The inner contents of an [`ArchivedTapestryWeave`].
pub type ArchivedTapestryWeaveInner = <TapestryWeaveInner as Archive>::Archived;

/// An [`IndependentWeave`] + [`PositionedWeave`] + [`PatchablePathWeave`] wrapper which implements Tapestry Loom's document format.
///
/// All identifiers *must be* randomly generated because the underlying [`Weave`]'s hashmaps use an identity hasher.
///
/// # Wrapper State
///
/// Tracked positions are dropped when [`TapestryWeave`] is cloned or converted into its underlying [`TapestryWeaveInner`].
///
/// # DoS Resistance
///
/// This Weave implementation does not make use of DoS-resistant hashers.
#[derive(Debug, Clone)]
#[allow(clippy::type_complexity)]
#[must_use]
pub struct TapestryWeave(
    PatchablePathWeave<
        PositionedWeave<
            TapestryWeaveInner,
            ShortId,
            TapestryNode,
            NodeContent,
            BuildHasherDefault<RandomIdHasher>,
        >,
        ShortId,
        TapestryNode,
        NodeContent,
    >,
);

impl Default for TapestryWeave {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl From<TapestryWeaveInner> for TapestryWeave {
    #[inline]
    fn from(value: TapestryWeaveInner) -> Self {
        Self(PatchablePathWeave::new(PositionedWeave::from_weave(value)))
    }
}

impl From<TapestryWeave> for TapestryWeaveInner {
    #[inline]
    fn from(value: TapestryWeave) -> Self {
        value.0.weave.weave
    }
}

impl AsRef<TapestryWeaveInner> for TapestryWeave {
    #[inline]
    fn as_ref(&self) -> &TapestryWeaveInner {
        &self.0.weave.weave
    }
}

impl TapestryWeave {
    /// Creates a new, empty [`TapestryWeave`].
    #[inline]
    pub fn new() -> Self {
        Self::from(IndependentWeave::new(WeaveMetadata::new()))
    }
    /// Creates a new, empty [`TapestryWeave`] with at least the specified capacity.
    ///
    /// This function over-allocates for worst-case memory usage rather than average-case.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self::from(IndependentWeave::with_capacity(
            capacity,
            WeaveMetadata::new(),
        ))
    }
    /// Creates a new, empty [`TapestryWeave`] with the specified metadata and at least the specified capacity.
    ///
    /// This function over-allocates for worst-case memory usage rather than average-case.
    #[inline]
    pub fn with_capacity_and_metadata(capacity: usize, metadata: WeaveMetadata) -> Self {
        Self::from(IndependentWeave::with_capacity(capacity, metadata))
    }
    /// Converts a [`TapestryWeave`] into the underlying [`TapestryWeaveInner`].
    ///
    /// This drops all tracked positions.
    #[inline]
    pub fn into_inner(self) -> TapestryWeaveInner {
        self.0.weave.weave
    }
    /// Returns a reference to the underlying [`TapestryWeaveInner`].
    #[inline]
    pub const fn as_inner(&self) -> &TapestryWeaveInner {
        &self.0.weave.weave
    }
    /// Returns a reference to the counter used to generate tracked positions.
    #[inline]
    pub const fn position_counter(&self) -> &Arc<AtomicU64> {
        self.0.weave.counter()
    }
    /// Replaces the counter used to generate tracked positions.
    ///
    /// Sharing the same counter between multiple [`TapestryWeave`]s keeps tracked positions unique across all of them.
    ///
    /// The counter must stay monotonic for positions to be guaranteed as unique. Thus, the counter should only be incremented and overflows must be avoided.
    ///
    /// This function always drops all tracked positions.
    #[inline]
    pub fn replace_position_counter(&mut self, counter: Arc<AtomicU64>) {
        self.0.weave.replace_counter(counter);
    }
    /// Returns a reference to the mapping between tracked positions and their corresponding node identifiers and alignments.
    #[inline]
    pub const fn positions(&self) -> &HashMap<u64, (ShortId, PositionAlignment)> {
        self.0.weave.positions()
    }
    /// Returns `true` if a position with the specified alignment is already tracked for the node with the specified identifier.
    #[must_use]
    #[inline]
    pub fn contains_position(&self, id: &ShortId, alignment: PositionAlignment) -> bool {
        self.0.weave.contains_position(id, alignment)
    }
    /// Returns the position with the specified alignment of the node with the specified identifier if it is already tracked.
    #[must_use]
    #[inline]
    pub fn get_position(&self, id: &ShortId, alignment: PositionAlignment) -> Option<u64> {
        self.0.weave.get_position(id, alignment)
    }
    /// Returns the position with the specified alignment of the node with the specified identifier, starting to track the position if it is not already tracked.
    ///
    /// Tracked positions are unique incremental identifiers which map to the node whose contents start or end at that position (see [`PositionAlignment`]), following that position across actions performed on the [`TapestryWeave`]. Once a position stops existing, it no longer maps to a node. Each node can have at most one tracked position per [`PositionAlignment`].
    ///
    /// Returns `None` if the Weave does not contain a node with the specified identifier.
    ///
    /// # Panics
    ///
    /// May panic if the position counter has gone backwards.
    #[must_use]
    #[inline]
    pub fn get_or_insert_position(
        &mut self,
        id: &ShortId,
        alignment: PositionAlignment,
    ) -> Option<u64> {
        self.0.weave.get_or_insert_position(id, alignment)
    }
    /// Returns the identifier of the position's corresponding node along with the position's alignment, if any.
    #[must_use]
    #[inline]
    pub fn resolve_position(&self, position: u64) -> Option<(ShortId, PositionAlignment)> {
        self.0.weave.resolve_position(position)
    }
    /// Stops tracking the specified position, returning the identifier of the corresponding node along with the position's alignment.
    #[inline]
    pub fn remove_position(&mut self, position: u64) -> Option<(ShortId, PositionAlignment)> {
        self.0.weave.remove_position(position)
    }
    /// Stops tracking all positions.
    #[inline]
    pub fn clear_positions(&mut self) {
        self.0.weave.clear_positions();
    }
    /// Convenience method for `self.is_empty() && self.metadata().is_empty()`
    #[inline]
    pub fn is_empty_including_metadata(&self) -> bool {
        self.0.is_empty() && self.0.metadata().is_empty()
    }
    /// Convenience method which returns `id`'s primary parent using [`Weave::get_path_from`]'s output.
    pub fn get_primary_parent(&mut self, id: &ShortId) -> Option<ShortId> {
        self.0.inner_scratchpad_mut(|weave, scratchpad| {
            let parents = weave.get_parents(id)?;
            if parents.len() < 2 {
                parents.first().copied()
            } else {
                weave.get_path_from(id, scratchpad);
                scratchpad.get(1).copied()
            }
        })
    }
    /// Convenience method which returns `id`'s primary child using [`Weave::get_active_path`], falling back to the first child if none of the node's children are on the active path.
    pub fn get_primary_child(&mut self, id: &ShortId) -> Option<ShortId> {
        self.0.inner_scratchpad_mut(|weave, scratchpad| {
            let children = weave.get_children(id)?;
            if children.len() < 2 {
                children.first().copied()
            } else {
                weave.get_active_path(scratchpad);

                let children = weave.get_children(id)?;

                scratchpad
                    .iter()
                    .position(|x| x == id)
                    .and_then(|index| index.checked_sub(1))
                    .or_else(|| scratchpad.iter().rposition(|x| children.contains(x)))
                    .map(|index| scratchpad[index])
                    .or(children.first().copied())
            }
        })
    }
    /// Convenience method which returns the siblings of the node corresponding to the identifier using the same ordering as [`Self::get_ordered_identifiers`].
    pub fn get_siblings(
        &mut self,
        id: &ShortId,
        include_roots: bool,
        include_self: bool,
    ) -> Option<Vec<ShortId>> {
        let node = self.0.get(id)?;

        let siblings = self.sibling_set(node, include_roots, include_self);
        let mut ordered = Vec::with_capacity(siblings.len());

        self.visit_ordered_siblings(node.from.len() < 2, siblings, |id| {
            ordered.push(id);
            ControlFlow::Continue(())
        });

        Some(ordered)
    }
    /// Convenience method which returns the sibling before `id` using the same ordering as [`Self::get_ordered_identifiers`].
    #[inline]
    pub fn get_previous_sibling(&mut self, id: &ShortId, include_roots: bool) -> Option<ShortId> {
        self.get_adjacent_sibling(id, include_roots, true)
    }
    /// Convenience method which returns the sibling after `id` using the same ordering as [`Self::get_ordered_identifiers`].
    #[inline]
    pub fn get_next_sibling(&mut self, id: &ShortId, include_roots: bool) -> Option<ShortId> {
        self.get_adjacent_sibling(id, include_roots, false)
    }
    fn get_adjacent_sibling(
        &mut self,
        id: &ShortId,
        include_roots: bool,
        reverse: bool,
    ) -> Option<ShortId> {
        let node = self.0.get(id)?;

        let siblings = self.sibling_set(node, include_roots, true);

        if siblings.len() < 2 {
            return None;
        }

        if reverse {
            let mut previous = None;

            self.visit_ordered_siblings(node.from.len() < 2, siblings, |sibling| {
                if sibling == *id {
                    ControlFlow::Break(())
                } else {
                    previous = Some(sibling);
                    ControlFlow::Continue(())
                }
            });

            previous
        } else {
            let mut next = None;
            let mut found = false;

            self.visit_ordered_siblings(node.from.len() < 2, siblings, |sibling| {
                if found {
                    next = Some(sibling);
                    ControlFlow::Break(())
                } else {
                    found |= sibling == *id;
                    ControlFlow::Continue(())
                }
            });

            next
        }
    }
    fn sibling_set(
        &self,
        node: &TapestryNode,
        include_roots: bool,
        include_self: bool,
    ) -> IndexSet<ShortId, BuildHasherDefault<RandomIdHasher>> {
        let filter = |id: &ShortId| {
            if node.id == *id {
                include_self
            } else {
                !node.from.contains(id) && !node.to.contains(id)
            }
        };

        if node.from.is_empty() {
            if include_roots {
                IndexSet::from_iter(self.0.roots().iter().copied().filter(filter))
            } else if include_self {
                IndexSet::from_iter([node.id])
            } else {
                IndexSet::default()
            }
        } else {
            IndexSet::from_iter(
                node.from
                    .iter()
                    .flat_map(|id| self.0.get_children(id).unwrap())
                    .copied()
                    .filter(filter),
            )
        }
    }
    fn visit_ordered_siblings(
        &mut self,
        single_parent: bool,
        siblings: IndexSet<ShortId, BuildHasherDefault<RandomIdHasher>>,
        mut visit: impl FnMut(ShortId) -> ControlFlow<()>,
    ) {
        if siblings.len() < 2
            || (single_parent
                && siblings
                    .iter()
                    .all(|id| self.0.get_parents(id).unwrap().len() < 2))
        {
            for sibling in siblings {
                if visit(sibling).is_break() {
                    return;
                }
            }

            return;
        }

        self.0.inner_scratchpad_mut(|weave, stack| {
            let mut remaining_parents =
            HashMap::<ShortId, usize, BuildHasherDefault<RandomIdHasher>>::with_capacity_and_hasher(
                siblings.len(),
                BuildHasherDefault::default(),
            );
            let mut roots = Vec::with_capacity(1);

            stack.extend(siblings.iter().copied());

            while let Some(id) = stack.pop() {
                let parents = weave.get_parents(&id).unwrap();

                if remaining_parents.insert(id, parents.len()).is_none() {
                    if parents.is_empty() {
                        roots.push(id);
                    } else {
                        stack.extend(parents.iter().copied());
                    }
                }
            }

            roots.sort_unstable_by_key(|id| Reverse(weave.roots().get_index_of(id)));

            stack.extend(roots);

            let mut unvisited = siblings.len();

            while let Some(id) = stack.pop() {
                if siblings.contains(&id) {
                    if visit(id).is_break() {
                        return;
                    }

                    unvisited -= 1;

                    if unvisited == 0 {
                        return;
                    }
                }

                for child in weave.get_children(&id).unwrap().iter().rev() {
                    if let Some(remaining) = remaining_parents.get_mut(child) {
                        *remaining -= 1;

                        if *remaining == 0 {
                            stack.push(*child);
                        }
                    }
                }
            }
        });
    }
    /// A wrapper around [`Weave::insert`] which prevents nodes with duplicate siblings from being inserted.
    #[must_use]
    pub fn insert_deduplicated(&mut self, mut node: TapestryNode) -> bool {
        node.contents.normalize();

        let siblings: Box<dyn Iterator<Item = ShortId>> = if node.from.is_empty() {
            Box::new(
                self.0
                    .roots()
                    .iter()
                    .copied()
                    .filter(|id| node.id != *id && !node.to.contains(id)),
            )
        } else {
            Box::new(
                node.from
                    .iter()
                    .filter_map(|id| self.0.get_children(id))
                    .flatten()
                    .copied()
                    .filter(|id| {
                        node.id != *id && !node.from.contains(id) && !node.to.contains(id)
                    }),
            )
        };

        if siblings
            .filter_map(|id| self.0.get_contents(&id))
            .any(|c| c.is_duplicate_of(&node.contents))
        {
            return false;
        }

        self.0.insert(node)
    }
    /// Sets the active status of a node with the specified identifier, using identical activation behavior to a tree-based Weave.
    #[inline]
    pub fn set_active_tree_semantics(&mut self, id: &ShortId, value: bool) -> bool {
        self.0.weave.weave.set_active_dependent_semantics(id, value)
    }
    /// Convenience function which calls either [`Self::sort_roots_by`] or [`Self::sort_children_by`] depending on whether or not `id` is None
    #[inline]
    pub fn sort_roots_or_children_by(
        &mut self,
        id: &Option<ShortId>,
        cmp: impl FnMut(&TapestryNode, &TapestryNode) -> Ordering,
    ) -> bool {
        if let Some(id) = id {
            self.0.sort_children_by(id, cmp)
        } else {
            self.0.sort_roots_by(cmp);
            true
        }
    }
    /// Convenience function which calls either [`Self::sort_roots_by_id`] or [`Self::sort_children_by_id`] depending on whether or not `id` is None
    #[inline]
    pub fn sort_roots_or_children_by_id(
        &mut self,
        id: &Option<ShortId>,
        cmp: impl FnMut(&ShortId, &ShortId) -> Ordering,
    ) -> bool {
        if let Some(id) = id {
            self.0.sort_children_by_id(id, cmp)
        } else {
            self.0.sort_roots_by_id(cmp);
            true
        }
    }
    /// A wrapper around [`Self::split`] which separates out the unmodified version of the token before splitting.
    ///
    /// If successful, returns a tuple of identifiers corresponding to (original_token_node, split_right_side)
    ///
    /// # Panics
    ///
    /// May panic if `generate_id` panics or returns an identifier already in the Weave.
    pub fn split_tokenized(
        &mut self,
        id: &ShortId,
        at: usize,
        mut generate_id: impl FnMut() -> ShortId,
    ) -> Option<(Option<ShortId>, ShortId)> {
        let new_id = generate_id();

        if at > 0
            && let Some(node) = self.0.get(id)
            && let InnerNodeContent::Tokens(tokens) = &node.contents.content
        {
            let total_length = tokens.iter().map(|token| token.bytes.len()).sum::<usize>();

            let mut byte_index = 0;
            let mut token_index = 0;
            let mut token_length = None;
            let mut within_unmodified_token = false;
            for (index, token) in tokens.iter().enumerate() {
                let next = byte_index + token.bytes.len();

                if next >= at {
                    token_index = index;
                    within_unmodified_token = next > at && !token.is_modified();
                    token_length = (next < total_length).then_some(token.bytes.len());
                    break;
                }

                byte_index = next;
            }

            if within_unmodified_token {
                let first_split_id = if byte_index != 0 {
                    let new_id = generate_id();

                    assert!(self.0.split(id, byte_index, new_id));

                    if self.0.contains_active(id) {
                        assert!(self.0.set_active(&new_id, true));
                    }

                    token_index -= self
                        .0
                        .get_contents(id)
                        .and_then(|contents| contents.content.token_count())
                        .unwrap();

                    new_id
                } else {
                    *id
                };
                if let Some(at) = token_length {
                    let new_id = generate_id();

                    assert!(self.0.split(&first_split_id, at, new_id));
                }

                let mut token_node = self.0.get(&first_split_id).unwrap().clone();
                token_node.active = false;
                token_node.bookmarked = false;
                if let InnerNodeContent::Tokens(tokens) = &mut token_node.contents.content {
                    tokens.drain(..token_index);
                    tokens.truncate(1);
                    tokens.shrink_to_fit();
                }

                assert!(self.0.split(&first_split_id, at - byte_index, new_id));

                let token_node_duplicate = {
                    if byte_index != 0 {
                        None
                    } else {
                        let mut siblings: Box<dyn Iterator<Item = ShortId>> =
                            if token_node.from.is_empty() {
                                Box::new(
                                    self.0
                                        .roots()
                                        .iter()
                                        .copied()
                                        .filter(|id| !token_node.to.contains(id)),
                                )
                            } else {
                                Box::new(
                                    token_node
                                        .from
                                        .iter()
                                        .filter_map(|id| self.0.get_children(id))
                                        .flatten()
                                        .copied()
                                        .filter(|id| {
                                            !token_node.from.contains(id)
                                                && !token_node.to.contains(id)
                                        }),
                                )
                            };

                        siblings.find(|id| {
                            self.0
                                .get_contents(id)
                                .is_some_and(|c| c.is_duplicate_of(&token_node.contents))
                        })
                    }
                };

                match token_node_duplicate {
                    Some(duplicate) => {
                        for token_child in &token_node.to {
                            let parents = self.0.get_parents(token_child).unwrap();

                            if !parents.contains(&duplicate) {
                                self.0.move_to(
                                    token_child,
                                    &Vec::from_iter(
                                        parents.iter().copied().chain(iter::once(duplicate)),
                                    ),
                                );
                            }
                        }

                        Some((Some(duplicate), new_id))
                    }
                    None => {
                        token_node.id = generate_id();

                        let token_node_id = token_node.id;

                        assert!(self.0.insert(token_node));

                        Some((Some(token_node_id), new_id))
                    }
                }
            } else {
                if self.0.split(id, at, new_id) {
                    Some((None, new_id))
                } else {
                    None
                }
            }
        } else {
            if self.0.split(id, at, new_id) {
                Some((None, new_id))
            } else {
                None
            }
        }
    }
    /// Splits the `index` token out of the node corresponding to the identifier `id`.
    ///
    /// The token being split out must not be empty.
    ///
    /// If successful, returns a tuple of identifiers corresponding to (token_parent, token, token_child). If the returned `token` equals the `id` argument, token_parent will be None.
    pub fn split_out_token(
        &mut self,
        id: &ShortId,
        index: usize,
        mut generate_id: impl FnMut() -> ShortId,
    ) -> Option<(Option<ShortId>, ShortId, Option<ShortId>)> {
        if let Some(node) = self.0.get(id) {
            if let InnerNodeContent::Tokens(tokens) = &node.contents.content
                && tokens.len() > index
                && !tokens[index].bytes.is_empty()
            {
                let split_index = tokens
                    .iter()
                    .take(index)
                    .map(|token| token.bytes.len())
                    .sum::<usize>();
                let token_length = tokens[index].bytes.len();
                let remaining_length = tokens
                    .iter()
                    .skip(index + 1)
                    .map(|token| token.bytes.len())
                    .sum::<usize>();

                let second_split_index = (remaining_length > 0).then_some(token_length);

                if split_index > 0 {
                    let middle_id = generate_id();

                    assert!(self.0.split(id, split_index, middle_id));

                    if self.0.contains_active(id) {
                        assert!(self.0.set_active(&middle_id, true));
                    }

                    if let Some(second_split_index) = second_split_index
                        && second_split_index > 0
                    {
                        let tail_id = generate_id();

                        assert!(self.0.split(&middle_id, second_split_index, tail_id));

                        if self.0.contains_active(&middle_id) {
                            assert!(self.0.set_active(&tail_id, true));
                        }

                        Some((Some(*id), middle_id, Some(tail_id)))
                    } else {
                        Some((Some(*id), middle_id, None))
                    }
                } else {
                    if let Some(second_split_index) = second_split_index
                        && second_split_index > 0
                    {
                        let tail_id = generate_id();

                        assert!(self.0.split(id, second_split_index, tail_id));

                        if self.0.contains_active(id) {
                            assert!(self.0.set_active(&tail_id, true));
                        }

                        Some((None, *id, Some(tail_id)))
                    } else {
                        Some((None, *id, None))
                    }
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    /// Returns `true` if [`Self::merge_with_parent`] would succeed.
    pub fn is_mergeable_with_parent(&self, id: &ShortId) -> bool {
        self.0.get(id).is_some_and(|node| {
            if node.from.len() == 1
                && let Some(parent) = self.0.get(&node.from[0])
            {
                parent.to.len() == 1 && parent.contents.is_mergeable_with(&node.contents)
            } else {
                false
            }
        })
    }
    /// Convenience function which returns the output of [`Weave::get_path_from`].
    #[inline]
    pub fn path_vec(&mut self, id: &ShortId) -> &mut Vec<ShortId> {
        self.0.path_vec(id)
    }
    /// Convenience function which returns the output of [`Weave::get_active_path`].
    #[inline]
    pub fn active_path_vec(&mut self) -> &mut Vec<ShortId> {
        self.0.active_path_vec()
    }
    /// Convenience function which returns an iterator over the content corresponding to the active path.
    #[inline]
    pub fn active_content(
        &mut self,
    ) -> impl ExactSizeIterator<Item = &NodeContent> + DoubleEndedIterator<Item = &NodeContent>
    {
        self.0.active_content()
    }
    /// Convenience function which returns an iterator over the nodes in the reversed active path.
    #[inline]
    pub fn active_nodes(
        &mut self,
    ) -> impl ExactSizeIterator<Item = &TapestryNode> + DoubleEndedIterator<Item = &TapestryNode>
    {
        self.0.active_nodes()
    }
    /// Convenience function which returns an iterator over the content corresponding to `id`'s path.
    #[inline]
    pub fn path_content(
        &mut self,
        id: &ShortId,
    ) -> impl ExactSizeIterator<Item = &NodeContent> + DoubleEndedIterator<Item = &NodeContent>
    {
        self.0.path_content(id)
    }
    /// Convenience function which returns an iterator over the content corresponding to `id`'s path, breaking if `stop` returns `true`.
    ///
    /// # Panics
    ///
    /// May panic if `stop` panics.
    #[inline]
    pub fn path_content_until<F>(
        &mut self,
        id: &ShortId,
        stop: F,
    ) -> impl Iterator<Item = &NodeContent>
    where
        F: FnMut(&ShortId) -> bool,
    {
        self.0.path_content_until(id, stop)
    }
    /// Convenience function which returns an iterator over the nodes in `id`'s reversed path.
    #[inline]
    pub fn path_nodes(
        &mut self,
        id: &ShortId,
    ) -> impl ExactSizeIterator<Item = &TapestryNode> + DoubleEndedIterator<Item = &TapestryNode>
    {
        self.0.path_nodes(id)
    }
    /// Convenience function which returns an iterator over the nodes in `id`'s reversed path, breaking if `stop` returns `true`.
    ///
    /// # Panics
    ///
    /// May panic if `stop` panics.
    #[inline]
    pub fn path_nodes_until<F>(
        &mut self,
        id: &ShortId,
        stop: F,
    ) -> impl Iterator<Item = &TapestryNode>
    where
        F: FnMut(&ShortId) -> bool,
    {
        self.0.path_nodes_until(id, stop)
    }
    /// Convenience function which returns an iterator over the text bytes corresponding to the active path.
    #[inline]
    pub fn active_text(&mut self) -> impl Iterator<Item = u8> {
        self.0
            .active_content()
            .flat_map(|content| content.content.iter_bytes())
    }
    /// Convenience function for `content::from_utf8_lossy(&self.active_text().collect::<Vec<u8>>()).to_string()`
    ///
    /// Because [`content::from_utf8_lossy`](crate::content::from_utf8_lossy) replaces every invalid byte with a 1-byte long substitution character, the converted string is always the same length as [`Self::active_text`].
    #[inline]
    pub fn active_text_string(&mut self) -> String {
        from_utf8_lossy(&self.active_text().collect::<Vec<u8>>()).into_owned()
    }
    /// Convenience function which toggles the active status of the node corresponding to the identifier `id`.
    #[inline]
    pub fn toggle_active(&mut self, id: &ShortId) -> bool {
        self.0.set_active(id, !self.0.contains_active(id))
    }
    /// Convenience function which toggles the bookmarked status of the node corresponding to the identifier `id`.
    #[inline]
    pub fn toggle_bookmarked(&mut self, id: &ShortId) -> bool {
        self.0.set_bookmarked(id, !self.0.contains_bookmark(id))
    }
    /// Removes the specified range from the active path without removing the content from the underlying Weave.
    ///
    /// If the range is empty or starts past the end of the active path, this function does nothing. If the range extends beyond the active path, its length is clamped to the active path's length.
    ///
    /// If the range starts at zero, an empty root node attributed to `author` may be inserted at the start of the active path. The identifier of this node is returned if it was inserted.
    ///
    /// # Panics
    ///
    /// May panic if `generate_id` panics or returns an identifier already in the Weave.
    pub fn split_out<F>(
        &mut self,
        range: Range<usize>,
        author: &Option<Author>,
        generate_id: F,
    ) -> Option<ShortId>
    where
        F: FnMut() -> ShortId,
    {
        if range.start == 0
            && self
                .0
                .active_content()
                .next()
                .is_some_and(|contents| !contents.content.is_empty())
        {
            let mut generated_ids = Vec::new();

            self.0
                .split_out(range, record_ids(generate_id, &mut generated_ids));

            let id = generated_ids.into_iter().find(|id| {
                self.0.get(id).is_some_and(|node| {
                    node.from.is_empty()
                        && matches!(node.contents.content, InnerNodeContent::MetadataOnly)
                })
            })?;

            self.0.get_contents_mut(&id, |contents| {
                let mut author = author.clone();
                if let Some(author) = &mut author {
                    author.normalize();
                }

                contents.creator = Creator::User(author);
            });

            Some(id)
        } else {
            self.0.split_out(range, generate_id);
            None
        }
    }
    /// Splits the active path at the specified index without deactivating the right side of the split.
    ///
    /// If `at` is zero or beyond the active path's length, this function does nothing.
    ///
    /// # Panics
    ///
    /// May panic if `generate_id` panics or returns an identifier already in the Weave.
    #[inline]
    pub fn split_at<F>(&mut self, at: usize, generate_id: F)
    where
        F: FnMut() -> ShortId,
    {
        assert!(self.0.split_at(at, generate_id), "Splitting node failed");
    }
    /// Inserts a new node into the active path at the specified index without removing existing node connections.
    ///
    /// If `prefix_all` is true, the inserted node prefixes all continuations, not just the active continuation. This may result in quadratic connection growth when repeatedly inserting at the same position.
    ///
    /// If the index extends past the end of the active path, it is clamped to the active path's length.
    ///
    /// If the index is zero, an empty root node attributed to `author` may be inserted at the start of the active path. In this case, `prefix_all` has no effect. The identifier of this node is returned if it was inserted.
    ///
    /// # Panics
    ///
    /// May panic if `generate_id` panics or returns an identifier already in the Weave.
    pub fn insert_at<F>(
        &mut self,
        at: usize,
        mut contents: NodeContent,
        prefix_all: bool,
        author: &Option<Author>,
        generate_id: F,
    ) -> Option<ShortId>
    where
        F: FnMut() -> ShortId,
    {
        contents.normalize();

        if at == 0
            && self
                .0
                .active_content()
                .next()
                .is_some_and(|contents| !contents.content.is_empty())
        {
            let mut generated_ids = Vec::new();

            self.0.insert_at(
                at,
                contents,
                prefix_all,
                record_ids(generate_id, &mut generated_ids),
            );

            let id = generated_ids.into_iter().find(|id| {
                self.0.get(id).is_some_and(|node| {
                    node.from.is_empty()
                        && matches!(node.contents.content, InnerNodeContent::MetadataOnly)
                })
            })?;

            self.0.get_contents_mut(&id, |contents| {
                let mut author = author.clone();
                if let Some(author) = &mut author {
                    author.normalize();
                }

                contents.creator = Creator::User(author);
            });

            Some(id)
        } else {
            self.0.insert_at(at, contents, prefix_all, generate_id);
            None
        }
    }
    /// Replaces the specified range of the active path without removing the old content from the underlying Weave.
    ///
    /// If `prefix_all` is true, the inserted content prefixes all continuations of the last replaced node, not just the active continuation. This may result in quadratic connection growth when repeatedly replacing the same range.
    ///
    /// If the range extends beyond the active path, its length is clamped to the active path's length.
    ///
    /// If the range is empty, this function behaves identically to [`Self::insert_at()`].
    ///
    /// # Panics
    ///
    /// May panic if `generate_id` panics or returns an identifier already in the Weave.
    #[inline]
    pub fn replace<F>(
        &mut self,
        range: Range<usize>,
        mut contents: NodeContent,
        prefix_all: bool,
        author: &Option<Author>,
        generate_id: F,
    ) -> Option<ShortId>
    where
        F: FnMut() -> ShortId,
    {
        if range.is_empty() {
            self.insert_at(range.start, contents, prefix_all, author, generate_id)
        } else {
            contents.normalize();
            self.0.replace(range, contents, prefix_all, generate_id);
            None
        }
    }
    /// Calculates a readable diff between `new` and the text bytes corresponding to the active path.
    ///
    /// Diff calculation time (but not post-processing time) is bounded, making this function generally safe to use in user interfaces.
    ///
    /// The exact diff calculation and semantic post-processing algorithms used are implementation-specific and subject to change.
    pub fn diff_active_text(&mut self, new: &[u8]) -> Vec<Hunk> {
        let deadline = Instant::now() + DIFF_DEADLINE;
        let (old, seams, tokenized) = self.active_text_structure(new.len());

        calculate_text_diff(&old, &seams, &tokenized, new, deadline)
    }
    /// Calculates a readable diff between `new` and [`Self::active_text_string`].
    ///
    /// Diff calculation time (but not post-processing time) is bounded, making this function generally safe to use in user interfaces.
    ///
    /// The exact diff calculation and semantic post-processing algorithms used are implementation-specific and subject to change.
    pub fn diff_active_text_str(&mut self, new: &str) -> Vec<Hunk> {
        let deadline = Instant::now() + DIFF_DEADLINE;
        let (old, seams, tokenized) = self.active_text_structure(new.len());

        calculate_text_diff(
            from_utf8_lossy(&old).as_bytes(),
            &seams,
            &tokenized,
            new.as_bytes(),
            deadline,
        )
    }
    fn active_text_structure(&mut self, capacity: usize) -> (Vec<u8>, Vec<Seam>, Vec<bool>) {
        let mut bytes = Vec::with_capacity(capacity);
        let mut seams = Vec::with_capacity(capacity + 1);
        let mut tokenized = Vec::with_capacity(capacity);

        seams.push(Seam::BetweenNodes);

        for content in self.0.active_content() {
            match &content.content {
                InnerNodeContent::Tokens(tokens) => {
                    for token in tokens.iter().filter(|token| !token.bytes.is_empty()) {
                        bytes.extend_from_slice(&token.bytes);
                        seams.resize(bytes.len(), Seam::WithinToken);
                        seams.push(Seam::BetweenTokens);
                    }

                    tokenized.resize(bytes.len(), true);
                }
                InnerNodeContent::Snippet(snippet) => {
                    bytes.extend_from_slice(snippet);
                    seams.resize(bytes.len() + 1, Seam::WithinSnippet);
                    tokenized.resize(bytes.len(), false);
                }
                InnerNodeContent::MetadataOnly => {}
            }

            seams[bytes.len()] = Seam::BetweenNodes;
        }

        (bytes, seams, tokenized)
    }
    /// Updates the text bytes corresponding to the active path using [`Self::diff_active_text`] followed by calls to [`Self::split_out`] and [`Self::replace`].
    ///
    /// Inserted content is attributed to `author` and replaced content is never removed from the Weave.
    ///
    /// # Panics
    ///
    /// May panic if `generate_id` panics or returns an identifier already in the Weave.
    pub fn update_active_text(
        &mut self,
        new: &[u8],
        author: &Option<Author>,
        mut generate_id: impl FnMut() -> ShortId,
    ) {
        let timestamp = Zoned::now();

        for hunk in self.diff_active_text(new).into_iter().rev() {
            if hunk.new.is_empty() {
                self.split_out(hunk.old, author, &mut generate_id);
            } else {
                self.replace(
                    hunk.old,
                    NodeContent {
                        timestamp: timestamp.clone(),
                        modified: false,
                        content: InnerNodeContent::Snippet(new[hunk.new].to_vec()),
                        metadata: MetadataMap::default(),
                        creator: Creator::User(author.clone()),
                    },
                    false,
                    author,
                    &mut generate_id,
                );
            }
        }
    }
    /// Updates the text bytes corresponding to the active path using [`Self::diff_active_text_str`] followed by calls to [`Self::split_out`] and [`Self::replace`].
    ///
    /// Inserted content is attributed to `author` and replaced content is never removed from the Weave.
    ///
    /// # Panics
    ///
    /// May panic if `generate_id` panics or returns an identifier already in the Weave.
    pub fn update_active_text_str(
        &mut self,
        new: &str,
        author: &Option<Author>,
        mut generate_id: impl FnMut() -> ShortId,
    ) {
        let timestamp = Zoned::now();

        for hunk in self.diff_active_text_str(new).into_iter().rev() {
            if hunk.new.is_empty() {
                self.split_out(hunk.old, author, &mut generate_id);
            } else {
                self.replace(
                    hunk.old,
                    NodeContent {
                        timestamp: timestamp.clone(),
                        modified: false,
                        content: InnerNodeContent::Snippet(new.as_bytes()[hunk.new].to_vec()),
                        metadata: MetadataMap::default(),
                        creator: Creator::User(author.clone()),
                    },
                    false,
                    author,
                    &mut generate_id,
                );
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Seam {
    WithinToken,
    WithinSnippet,
    BetweenTokens,
    BetweenNodes,
}

fn segment_text_bytes<'a, I>(bytes: &'a [u8], segment: impl Fn(&'a str) -> I) -> Vec<usize>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let mut boundaries = Vec::new();
    let mut cursor = 0;

    for chunk in bytes.utf8_chunks() {
        let valid = chunk.valid();
        boundaries.extend(segment(valid).map(|(index, _)| cursor + index));
        cursor += valid.len();

        let invalid_len = chunk.invalid().len();
        boundaries.extend(cursor..cursor + invalid_len);
        cursor += invalid_len;
    }

    boundaries.push(cursor);

    boundaries
}

struct Boundaries<'a, S, K> {
    bytes: &'a [u8],
    segment: S,
    keep: K,
    found: Vec<usize>,
    other: Vec<usize>,
}

impl<'a, S, I, K> Boundaries<'a, S, K>
where
    S: Fn(&'a str) -> I,
    I: Iterator<Item = (usize, &'a str)>,
    K: Fn(usize) -> bool,
{
    fn new(bytes: &'a [u8], segment: S, keep: K) -> Self {
        Self {
            bytes,
            segment,
            keep,
            found: Vec::new(),
            other: Vec::new(),
        }
    }
    fn slack(&mut self, range: &Range<usize>) -> (usize, usize) {
        (
            range.start - self.before(range.start),
            self.after(range.end) - range.end,
        )
    }
    fn before(&mut self, at: usize) -> usize {
        self.seek(at);

        self.found[self.found.partition_point(|boundary| boundary <= &at) - 1]
    }
    fn after(&mut self, at: usize) -> usize {
        self.seek(at);

        self.found[self.found.partition_point(|boundary| boundary < &at)]
    }
    fn seek(&mut self, at: usize) {
        let covers =
            |found: &[usize]| matches!(found, [start, .., end] if (*start..=*end).contains(&at));

        if covers(&self.found) {
            return;
        }

        mem::swap(&mut self.found, &mut self.other);

        if covers(&self.found) {
            return;
        }

        let line_start = |at: &usize| self.bytes[at - 1] == b'\n' && (self.keep)(*at);
        let start = (1..=at).rev().find(&line_start).unwrap_or(0);
        let end = (at + 1..=self.bytes.len())
            .find(&line_start)
            .unwrap_or(self.bytes.len());

        self.found = segment_text_bytes(&self.bytes[start..end], &self.segment);
        self.found.retain_mut(|boundary| {
            *boundary += start;

            (self.keep)(*boundary)
        });
    }
}

fn seam_affinity(text: &[u8], at: usize) -> u8 {
    if at == 0 || at == text.len() {
        return 6;
    }

    let before = (1..=at.min(4))
        .find_map(|len| str::from_utf8(&text[at - len..at]).ok())
        .and_then(|before| before.chars().next());
    let after = text[at..text.len().min(at + 4)]
        .utf8_chunks()
        .next()
        .and_then(|chunk| chunk.valid().chars().next());

    if let (Some(before), Some(after)) = (before, after) {
        if before == '\n' {
            5
        } else if after == '\n' || after == '\r' {
            4
        } else if after.is_whitespace() && !before.is_whitespace() {
            3
        } else if before.is_whitespace() && !after.is_whitespace() {
            2
        } else if !(before.is_alphanumeric() && after.is_alphanumeric()) {
            1
        } else {
            0
        }
    } else {
        0
    }
}

fn calculate_text_diff(
    old: &[u8],
    seams: &[Seam],
    tokenized: &[bool],
    new: &[u8],
    deadline: Instant,
) -> Vec<Hunk> {
    let mut hunks = Hunk::calculate_diff(old, new, Some(deadline));

    let mut old_units = Boundaries::new(
        old,
        |text| text.grapheme_indices(true),
        |at| seams[at] != Seam::WithinToken,
    );
    let mut new_units = Boundaries::new(new, |text| text.grapheme_indices(true), |_| true);
    let mut old_words = Boundaries::new(old, str::split_word_bound_indices, |_| true);
    let mut new_words = Boundaries::new(new, str::split_word_bound_indices, |_| true);

    let widest = |old: (usize, usize), new: (usize, usize)| (old.0.max(new.0), old.1.max(new.1));
    let mut unit_slack =
        |hunk: &Hunk| widest(old_units.slack(&hunk.old), new_units.slack(&hunk.new));
    let mut word_slack =
        |hunk: &Hunk| widest(old_words.slack(&hunk.old), new_words.slack(&hunk.new));

    Hunk::slide_ordered(&mut hunks, old, new, DIFF_SLIDE_LIMIT, |hunk| {
        let (left, right) = unit_slack(hunk);

        let added_nodes: usize = [hunk.old.start, hunk.old.end]
            .into_iter()
            .map(|at| match seams[at] {
                Seam::BetweenNodes if at == 0 => 2,
                Seam::BetweenNodes => 0,
                _ => 1,
            })
            .sum();

        let affinity = if hunk.old.is_empty() {
            seam_affinity(new, hunk.new.start) + seam_affinity(new, hunk.new.end)
        } else {
            seam_affinity(old, hunk.old.start) + seam_affinity(old, hunk.old.end)
        };

        (left + right, added_nodes, Reverse(affinity))
    });

    Hunk::expand_ordered(&mut hunks, old.len(), |hunk| {
        let (mut left, mut right) = unit_slack(hunk);
        let within_snippet = |at: usize| seams[at] == Seam::WithinSnippet;

        if (left == 0 && within_snippet(hunk.old.start))
            || (right == 0 && within_snippet(hunk.old.end))
        {
            let (word_left, word_right) = word_slack(hunk);

            if left == 0 {
                left = (0..word_left)
                    .take_while(|by| within_snippet(hunk.old.start - by))
                    .count();
            }

            if right == 0 {
                right = (0..word_right)
                    .take_while(|by| within_snippet(hunk.old.end + by))
                    .count();
            }
        }

        (left, right)
    });

    Hunk::merge_ordered(&mut hunks, |previous, next| {
        let size = |hunk: &Hunk| hunk.old.len().max(hunk.new.len());
        let unchanged = previous.old.end..next.old.start;

        unchanged.len() <= size(previous).min(size(next)) && !tokenized[unchanged].contains(&true)
    });

    hunks
}

impl Weave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    type Nodes = HashMap<ShortId, TapestryNode, BuildHasherDefault<RandomIdHasher>>;
    type Roots = IndexSet<ShortId, BuildHasherDefault<RandomIdHasher>>;

    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }
    #[inline]
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    #[inline]
    fn nodes(&self) -> &Self::Nodes {
        self.0.nodes()
    }
    #[inline]
    fn roots(&self) -> &Self::Roots {
        self.0.roots()
    }
    #[inline]
    fn contains(&self, id: &ShortId) -> bool {
        self.0.contains(id)
    }
    #[inline]
    fn contains_active(&self, id: &ShortId) -> bool {
        self.0.contains_active(id)
    }
    #[inline]
    fn get(&self, id: &ShortId) -> Option<&TapestryNode> {
        self.0.get(id)
    }
    #[inline]
    fn get_parents(
        &self,
        id: &ShortId,
    ) -> Option<&IndexSet<ShortId, BuildHasherDefault<RandomIdHasher>>> {
        self.0.get_parents(id)
    }
    #[inline]
    fn get_children(
        &self,
        id: &ShortId,
    ) -> Option<&IndexSet<ShortId, BuildHasherDefault<RandomIdHasher>>> {
        self.0.get_children(id)
    }
    #[inline]
    fn get_contents(&self, id: &ShortId) -> Option<&NodeContent> {
        self.0.get_contents(id)
    }
    #[inline]
    fn get_ordered_identifiers(&mut self, output: &mut Vec<ShortId>) {
        self.0.get_ordered_identifiers(output);
    }
    #[inline]
    fn get_ordered_identifiers_from(&mut self, id: &ShortId, output: &mut Vec<ShortId>) {
        self.0.get_ordered_identifiers_from(id, output);
    }
    #[inline]
    fn get_active_path(&mut self, output: &mut Vec<ShortId>) {
        self.0.get_active_path(output);
    }
    #[inline]
    fn get_path_from(&mut self, id: &ShortId, output: &mut Vec<ShortId>) {
        self.0.get_path_from(id, output);
    }
    #[inline]
    fn insert(&mut self, mut node: TapestryNode) -> bool {
        node.contents.normalize();
        self.0.insert(node)
    }
    #[inline]
    fn set_active(&mut self, id: &ShortId, value: bool) -> bool {
        self.0.set_active(id, value)
    }
    #[inline]
    fn remove(&mut self, id: &ShortId) -> Option<TapestryNode> {
        self.0.remove(id)
    }
    #[inline]
    fn remove_tracked(&mut self, id: &ShortId, on_removal: impl FnMut(TapestryNode)) -> bool {
        self.0.remove_tracked(id, on_removal)
    }
    #[inline]
    fn clear(&mut self) {
        self.0.clear();
    }
}

impl MetadataWeave<ShortId, TapestryNode, NodeContent, WeaveMetadata> for TapestryWeave {
    #[inline]
    fn metadata(&self) -> &WeaveMetadata {
        self.0.metadata()
    }
    #[inline]
    fn metadata_mut<O>(&mut self, callback: impl FnOnce(&mut WeaveMetadata) -> O) -> O {
        self.0.metadata_mut(callback)
    }
}

impl BookmarkableWeave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    type Bookmarks = IndexSet<ShortId, BuildHasherDefault<RandomIdHasher>>;

    #[inline]
    fn bookmarks(&self) -> &Self::Bookmarks {
        self.0.bookmarks()
    }
    #[inline]
    fn contains_bookmark(&self, id: &ShortId) -> bool {
        self.0.contains_bookmark(id)
    }
    #[inline]
    fn set_bookmarked(&mut self, id: &ShortId, value: bool) -> bool {
        self.0.set_bookmarked(id, value)
    }
}

impl SortableWeave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    #[inline]
    fn sort_children_by(
        &mut self,
        id: &ShortId,
        cmp: impl FnMut(&TapestryNode, &TapestryNode) -> Ordering,
    ) -> bool {
        self.0.sort_children_by(id, cmp)
    }
    #[inline]
    fn sort_children_by_id(
        &mut self,
        id: &ShortId,
        cmp: impl FnMut(&ShortId, &ShortId) -> Ordering,
    ) -> bool {
        self.0.sort_children_by_id(id, cmp)
    }
    #[inline]
    fn sort_roots_by(&mut self, cmp: impl FnMut(&TapestryNode, &TapestryNode) -> Ordering) {
        self.0.sort_roots_by(cmp);
    }
    #[inline]
    fn sort_roots_by_id(&mut self, cmp: impl FnMut(&ShortId, &ShortId) -> Ordering) {
        self.0.sort_roots_by_id(cmp);
    }
}

impl SortableBookmarkableWeave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    #[inline]
    fn sort_bookmarks_by(&mut self, cmp: impl FnMut(&TapestryNode, &TapestryNode) -> Ordering) {
        self.0.sort_bookmarks_by(cmp);
    }
    #[inline]
    fn sort_bookmarks_by_id(&mut self, cmp: impl FnMut(&ShortId, &ShortId) -> Ordering) {
        self.0.sort_bookmarks_by_id(cmp);
    }
}

impl ActivePathWeave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    type Active = HashSet<ShortId, BuildHasherDefault<RandomIdHasher>>;

    #[inline]
    fn active(&self) -> &Self::Active {
        self.0.active()
    }
    #[inline]
    fn set_active_path(&mut self, active: impl Iterator<Item = ShortId>) {
        self.0.set_active_path(active);
    }
}

impl IndependentWeaveTrait<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    #[inline]
    fn move_to(&mut self, id: &ShortId, new_parents: &[ShortId]) -> bool {
        self.0.move_to(id, new_parents)
    }
}

impl SemiIndependentWeave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    #[inline]
    fn get_contents_mut<O>(
        &mut self,
        id: &ShortId,
        callback: impl FnOnce(&mut NodeContent) -> O,
    ) -> Option<O> {
        self.0.get_contents_mut(id, |contents| {
            let output = callback(contents);
            contents.normalize();
            output
        })
    }
}

impl DiscreteWeave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    #[inline]
    fn split(&mut self, id: &ShortId, at: usize, new_id: ShortId) -> bool {
        self.0.split(id, at, new_id)
    }
    #[inline]
    fn merge_with_parent(&mut self, id: &ShortId) -> Option<ShortId> {
        self.0.merge_with_parent(id)
    }
}

/// A [`TapestryWeave`] loaded using zero-copy deserialization.
#[derive(Clone)]
#[must_use]
pub struct ArchivedTapestryWeave<'a>(pub &'a ArchivedTapestryWeaveInner);

impl<'a> From<&'a ArchivedTapestryWeaveInner> for ArchivedTapestryWeave<'a> {
    #[inline]
    fn from(value: &'a ArchivedTapestryWeaveInner) -> Self {
        Self(value)
    }
}

impl<'a> From<ArchivedTapestryWeave<'a>> for &'a ArchivedTapestryWeaveInner {
    #[inline]
    fn from(value: ArchivedTapestryWeave<'a>) -> Self {
        value.0
    }
}

impl<'a> AsRef<ArchivedTapestryWeaveInner> for ArchivedTapestryWeave<'a> {
    #[inline]
    fn as_ref(&self) -> &'a ArchivedTapestryWeaveInner {
        self.0
    }
}

impl ImmutableWeave<ArchivedShortId, ArchivedTapestryNode, ArchivedNodeContent>
    for ArchivedTapestryWeave<'_>
{
    type Nodes = ArchivedHashMap<ArchivedShortId, ArchivedTapestryNode>;
    type Roots = ArchivedIndexSet<ArchivedShortId>;

    #[inline]
    fn len(&self) -> usize {
        self.0.len()
    }
    #[inline]
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    #[inline]
    fn nodes(&self) -> &Self::Nodes {
        self.0.nodes()
    }
    #[inline]
    fn roots(&self) -> &Self::Roots {
        self.0.roots()
    }
    #[inline]
    fn contains(&self, id: &ArchivedShortId) -> bool {
        self.0.contains(id)
    }
    #[inline]
    fn contains_active(&self, id: &ArchivedShortId) -> bool {
        self.0.contains_active(id)
    }
    #[inline]
    fn get(&self, id: &ArchivedShortId) -> Option<&ArchivedTapestryNode> {
        self.0.get(id)
    }
    #[inline]
    fn get_parents(&self, id: &ArchivedShortId) -> Option<&ArchivedIndexSet<ArchivedShortId>> {
        self.0.get_parents(id)
    }
    #[inline]
    fn get_children(&self, id: &ArchivedShortId) -> Option<&ArchivedIndexSet<ArchivedShortId>> {
        self.0.get_children(id)
    }
    #[inline]
    fn get_contents(&self, id: &ArchivedShortId) -> Option<&ArchivedNodeContent> {
        self.0.get_contents(id)
    }
    #[inline]
    fn get_ordered_identifiers(&self, output: &mut Vec<ArchivedShortId>) {
        self.0.get_ordered_identifiers(output);
    }
    #[inline]
    fn get_ordered_identifiers_from(
        &self,
        id: &ArchivedShortId,
        output: &mut Vec<ArchivedShortId>,
    ) {
        self.0.get_ordered_identifiers_from(id, output);
    }
    #[inline]
    fn get_active_path(&self, output: &mut Vec<ArchivedShortId>) {
        self.0.get_active_path(output);
    }
    #[inline]
    fn get_path_from(&self, id: &ArchivedShortId, output: &mut Vec<ArchivedShortId>) {
        self.0.get_path_from(id, output);
    }
}

impl
    ImmutableMetadataWeave<
        ArchivedShortId,
        ArchivedTapestryNode,
        ArchivedNodeContent,
        ArchivedWeaveMetadata,
    > for ArchivedTapestryWeave<'_>
{
    #[inline]
    fn metadata(&self) -> &ArchivedWeaveMetadata {
        self.0.metadata()
    }
}

impl ImmutableBookmarkableWeave<ArchivedShortId, ArchivedTapestryNode, ArchivedNodeContent>
    for ArchivedTapestryWeave<'_>
{
    type Bookmarks = ArchivedIndexSet<ArchivedShortId>;

    #[inline]
    fn bookmarks(&self) -> &Self::Bookmarks {
        self.0.bookmarks()
    }
    #[inline]
    fn contains_bookmark(&self, id: &ArchivedShortId) -> bool {
        self.0.contains_bookmark(id)
    }
}

impl ImmutableActivePathWeave<ArchivedShortId, ArchivedTapestryNode, ArchivedNodeContent>
    for ArchivedTapestryWeave<'_>
{
    type Active = ArchivedHashSet<ArchivedShortId>;

    #[inline]
    fn active(&self) -> &Self::Active {
        self.0.active()
    }
}

fn record_ids<'a>(
    mut generate_id: impl FnMut() -> ShortId + 'a,
    generated_ids: &'a mut Vec<ShortId>,
) -> impl FnMut() -> ShortId + 'a {
    move || {
        let id = generate_id();
        generated_ids.push(id);
        id
    }
}
