use std::{cmp::Ordering, hash::BuildHasherDefault, iter, num::NonZeroU128};

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
};

use super::{
    content::{ArchivedNodeContent, InnerNodeContent, NodeContent},
    hashers::RandomIdHasher,
    metadata::{ArchivedWeaveMetadata, WeaveMetadata},
};

pub(crate) const FORMAT_VERSION: u64 = 1;

pub type ShortId = u64;
pub type LongId = NonZeroU128;
pub type TapestryNode = IndependentNode<u64, NodeContent, BuildHasherDefault<RandomIdHasher>>;
pub type TapestryWeaveInner =
    IndependentWeave<u64, NodeContent, WeaveMetadata, BuildHasherDefault<RandomIdHasher>>;

pub type ArchivedShortId = <ShortId as Archive>::Archived;
pub type ArchivedTapestryNode = <TapestryNode as Archive>::Archived;
pub type ArchivedTapestryWeaveInner = <TapestryWeaveInner as Archive>::Archived;

/// An [`IndependentWeave`] wrapper which implements Tapestry Loom's document format.
///
/// All identifiers *must be* randomly generated because the underlying [`Weave`]'s hashmaps use an identity hasher.
///
/// # DoS Resistance
///
/// This Weave implementation does not make use of DoS-resistant hashers.
pub struct TapestryWeave(TapestryWeaveInner);

impl Default for TapestryWeave {
    fn default() -> Self {
        Self::new()
    }
}

impl From<TapestryWeaveInner> for TapestryWeave {
    fn from(value: TapestryWeaveInner) -> Self {
        Self(value)
    }
}

impl From<TapestryWeave> for TapestryWeaveInner {
    fn from(value: TapestryWeave) -> Self {
        value.0
    }
}

impl AsRef<TapestryWeaveInner> for TapestryWeave {
    fn as_ref(&self) -> &TapestryWeaveInner {
        &self.0
    }
}

impl TapestryWeave {
    /// Creates a new, empty [`TapestryWeave`].
    pub fn new() -> Self {
        Self(IndependentWeave::new(WeaveMetadata::new()))
    }
    /// Creates a new, empty [`TapestryWeave`] with at least the specified capacity.
    ///
    /// This function over-allocates for worst-case memory usage rather than average-case.
    pub fn with_capacity(capacity: usize) -> Self {
        Self(IndependentWeave::with_capacity(
            capacity,
            WeaveMetadata::new(),
        ))
    }
    /// Creates a new, empty [`TapestryWeave`] with the specified metadata and at least the specified capacity.
    ///
    /// This function over-allocates for worst-case memory usage rather than average-case.
    pub fn with_capacity_and_metadata(capacity: usize, metadata: WeaveMetadata) -> Self {
        Self(IndependentWeave::with_capacity(capacity, metadata))
    }
    /// Returns the worst-case number of nodes that the weave can hold without reallocating.
    ///
    /// May be lower than `self.len()`.
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }
    /// Reserves capacity for at least `additional` more nodes.
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }
    /// Shrinks the capacity of the weave as much as possible.
    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }
    /// Convenience method for `self.is_empty() && self.metadata().is_empty()`
    pub fn is_empty_including_metadata(&self) -> bool {
        self.0.is_empty() && self.0.metadata.is_empty()
    }
    /// Convenience method which returns the siblings of the node corresponding to the identifier.
    ///
    /// This function may return duplicate identifiers.
    pub fn get_siblings<'a>(
        &'a self,
        id: &ShortId,
        include_roots: bool,
    ) -> Option<Box<dyn Iterator<Item = ShortId> + 'a>> {
        if let Some(node) = self.0.get(id) {
            Some(if include_roots && node.from.is_empty() {
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
            })
        } else {
            None
        }
    }
    /// A wrapper around [`Weave::insert`] which prevents nodes with duplicate siblings from being inserted.
    #[must_use]
    pub fn insert_deduplicated(&mut self, node: TapestryNode) -> bool {
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
    pub fn set_active_tree_semantics(&mut self, id: &ShortId, value: bool) -> bool {
        self.0.set_active_dependent_semantics(id, value)
    }
    /// A wrapper around [`Self::split`] which separates out the unmodified version of the token before splitting.
    ///
    /// If successful, returns a tuple of identifiers corresponding to (original_token_node, split_right_side)
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
            let mut byte_index = 0;
            let mut token_length = None;
            let mut within_unmodified_token = false;
            for (index, token) in tokens.iter().enumerate() {
                let next = byte_index + token.bytes.len();

                if next >= at {
                    within_unmodified_token = next > at && !token.is_modified();
                    token_length = (index + 1 != tokens.len()).then_some(token.bytes.len());
                    break;
                }

                byte_index = next;
            }

            if within_unmodified_token {
                let first_split_id = if byte_index != 0 {
                    let new_id = generate_id();

                    assert!(self.0.split(id, byte_index, new_id));

                    new_id
                } else {
                    *id
                };
                if let Some(at) = token_length
                    && at != 0
                {
                    let new_id = generate_id();

                    assert!(self.0.split(&first_split_id, at, new_id));
                }

                let mut token_node = self.0.get(&first_split_id).unwrap().clone();
                token_node.active = false;
                token_node.bookmarked = false;

                assert!(self.0.split(&first_split_id, at - byte_index, new_id));

                let token_node_duplicate = {
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
                                        !token_node.from.contains(id) && !token_node.to.contains(id)
                                    }),
                            )
                        };

                    siblings.find(|id| {
                        self.0
                            .get_contents(id)
                            .is_some_and(|c| c.is_duplicate_of(&token_node.contents))
                    })
                };

                match token_node_duplicate {
                    Some(duplicate) => {
                        for token_child in token_node.to {
                            let parents = self.0.get_parents(&token_child).unwrap();

                            if !parents.contains(&duplicate) {
                                assert!(self.0.move_to(
                                    &token_child,
                                    &Vec::from_iter(
                                        parents.iter().copied().chain(iter::once(duplicate)),
                                    )
                                ));
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
    /// If successful, returns a tuple of identifiers corresponding to (before_token, token, after_token).
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
                    let middle_id = generate_id();

                    assert!(self.0.split(id, split_index, middle_id));

                    if let Some(second_split_index) = second_split_index
                        && second_split_index > 0
                    {
                        let tail_id = generate_id();

                        assert!(self.0.split(&middle_id, second_split_index, tail_id));

                        Some((Some(*id), middle_id, Some(tail_id)))
                    } else {
                        Some((Some(*id), middle_id, None))
                    }
                } else {
                    let chosen_parent = node
                        .from
                        .iter()
                        .copied()
                        .find(|id| self.0.contains_active(id))
                        .or_else(|| node.from.first().copied());

                    if let Some(second_split_index) = second_split_index
                        && second_split_index > 0
                    {
                        let tail_id = generate_id();

                        assert!(self.0.split(id, second_split_index, tail_id));

                        Some((chosen_parent, *id, Some(tail_id)))
                    } else {
                        Some((chosen_parent, *id, None))
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
}

impl Weave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    type Nodes = HashMap<ShortId, TapestryNode, BuildHasherDefault<RandomIdHasher>>;
    type Roots = IndexSet<ShortId, BuildHasherDefault<RandomIdHasher>>;

    fn len(&self) -> usize {
        self.0.len()
    }
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    fn nodes(&self) -> &Self::Nodes {
        self.0.nodes()
    }
    fn roots(&self) -> &Self::Roots {
        self.0.roots()
    }
    fn contains(&self, id: &ShortId) -> bool {
        self.0.contains(id)
    }
    fn contains_active(&self, id: &ShortId) -> bool {
        self.0.contains_active(id)
    }
    fn get(&self, id: &ShortId) -> Option<&TapestryNode> {
        self.0.get(id)
    }
    fn get_parents(
        &self,
        id: &ShortId,
    ) -> Option<&IndexSet<ShortId, BuildHasherDefault<RandomIdHasher>>> {
        self.0.get_parents(id)
    }
    fn get_children(
        &self,
        id: &ShortId,
    ) -> Option<&IndexSet<ShortId, BuildHasherDefault<RandomIdHasher>>> {
        self.0.get_children(id)
    }
    fn get_contents(&self, id: &ShortId) -> Option<&NodeContent> {
        self.0.get_contents(id)
    }
    fn get_ordered_identifiers(&mut self, output: &mut Vec<ShortId>) {
        self.0.get_ordered_identifiers(output);
    }
    fn get_ordered_identifiers_from(&mut self, id: &ShortId, output: &mut Vec<ShortId>) {
        self.0.get_ordered_identifiers_from(id, output);
    }
    fn get_active_path(&mut self, output: &mut Vec<ShortId>) {
        self.0.get_active_path(output);
    }
    fn get_path_from(&mut self, id: &ShortId, output: &mut Vec<ShortId>) {
        self.0.get_path_from(id, output);
    }
    fn insert(&mut self, node: TapestryNode) -> bool {
        self.0.insert(node)
    }
    fn set_active(&mut self, id: &ShortId, value: bool) -> bool {
        self.0.set_active(id, value)
    }
    fn remove(&mut self, id: &ShortId) -> Option<TapestryNode> {
        self.0.remove(id)
    }
    fn remove_tracked(&mut self, id: &ShortId, on_removal: impl FnMut(TapestryNode)) -> bool {
        self.0.remove_tracked(id, on_removal)
    }
    fn clear(&mut self) {
        self.0.clear();
    }
}

impl MetadataWeave<ShortId, TapestryNode, NodeContent, WeaveMetadata> for TapestryWeave {
    fn metadata(&self) -> &WeaveMetadata {
        self.0.metadata()
    }
    fn metadata_mut<O>(&mut self, callback: impl FnOnce(&mut WeaveMetadata) -> O) -> O {
        self.0.metadata_mut(callback)
    }
}

impl BookmarkableWeave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    type Bookmarks = IndexSet<ShortId, BuildHasherDefault<RandomIdHasher>>;

    fn bookmarks(&self) -> &Self::Bookmarks {
        self.0.bookmarks()
    }
    fn contains_bookmark(&self, id: &ShortId) -> bool {
        self.0.contains_bookmark(id)
    }
    fn set_bookmarked(&mut self, id: &ShortId, value: bool) -> bool {
        self.0.set_bookmarked(id, value)
    }
}

impl SortableWeave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    fn sort_children_by(
        &mut self,
        id: &ShortId,
        cmp: impl FnMut(&TapestryNode, &TapestryNode) -> Ordering,
    ) -> bool {
        self.0.sort_children_by(id, cmp)
    }
    fn sort_children_by_id(
        &mut self,
        id: &ShortId,
        cmp: impl FnMut(&ShortId, &ShortId) -> Ordering,
    ) -> bool {
        self.0.sort_children_by_id(id, cmp)
    }
    fn sort_roots_by(&mut self, cmp: impl FnMut(&TapestryNode, &TapestryNode) -> Ordering) {
        self.0.sort_roots_by(cmp);
    }
    fn sort_roots_by_id(&mut self, cmp: impl FnMut(&ShortId, &ShortId) -> Ordering) {
        self.0.sort_roots_by_id(cmp);
    }
}

impl SortableBookmarkableWeave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    fn sort_bookmarks_by(&mut self, cmp: impl FnMut(&TapestryNode, &TapestryNode) -> Ordering) {
        self.0.sort_bookmarks_by(cmp);
    }
    fn sort_bookmarks_by_id(&mut self, cmp: impl FnMut(&ShortId, &ShortId) -> Ordering) {
        self.0.sort_bookmarks_by_id(cmp);
    }
}

impl ActivePathWeave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    type Active = HashSet<ShortId, BuildHasherDefault<RandomIdHasher>>;

    fn active(&self) -> &Self::Active {
        self.0.active()
    }
    fn set_active_path(&mut self, active: impl Iterator<Item = ShortId>) {
        self.0.set_active_path(active);
    }
}

impl IndependentWeaveTrait<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    fn move_to(&mut self, id: &ShortId, new_parents: &[ShortId]) -> bool {
        self.0.move_to(id, new_parents)
    }
}

impl SemiIndependentWeave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    fn get_contents_mut<O>(
        &mut self,
        id: &ShortId,
        callback: impl FnOnce(&mut NodeContent) -> O,
    ) -> Option<O> {
        self.0.get_contents_mut(id, callback)
    }
}

impl DiscreteWeave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    fn split(&mut self, id: &ShortId, at: usize, new_id: ShortId) -> bool {
        self.0.split(id, at, new_id)
    }
    fn merge_with_parent(&mut self, id: &ShortId) -> Option<ShortId> {
        self.0.merge_with_parent(id)
    }
}

pub struct ArchivedTapestryWeave<'a>(pub &'a ArchivedTapestryWeaveInner);

impl<'a> From<&'a ArchivedTapestryWeaveInner> for ArchivedTapestryWeave<'a> {
    fn from(value: &'a ArchivedTapestryWeaveInner) -> Self {
        Self(value)
    }
}

impl<'a> From<ArchivedTapestryWeave<'a>> for &'a ArchivedTapestryWeaveInner {
    fn from(value: ArchivedTapestryWeave<'a>) -> Self {
        value.0
    }
}

impl<'a> AsRef<ArchivedTapestryWeaveInner> for ArchivedTapestryWeave<'a> {
    fn as_ref(&self) -> &'a ArchivedTapestryWeaveInner {
        self.0
    }
}

impl ImmutableWeave<ArchivedShortId, ArchivedTapestryNode, ArchivedNodeContent>
    for ArchivedTapestryWeave<'_>
{
    type Nodes = ArchivedHashMap<ArchivedShortId, ArchivedTapestryNode>;
    type Roots = ArchivedIndexSet<ArchivedShortId>;

    fn len(&self) -> usize {
        self.0.len()
    }
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    fn nodes(&self) -> &Self::Nodes {
        self.0.nodes()
    }
    fn roots(&self) -> &Self::Roots {
        self.0.roots()
    }
    fn contains(&self, id: &ArchivedShortId) -> bool {
        self.0.contains(id)
    }
    fn contains_active(&self, id: &ArchivedShortId) -> bool {
        self.0.contains_active(id)
    }
    fn get(&self, id: &ArchivedShortId) -> Option<&ArchivedTapestryNode> {
        self.0.get(id)
    }
    fn get_parents(&self, id: &ArchivedShortId) -> Option<&ArchivedIndexSet<ArchivedShortId>> {
        self.0.get_parents(id)
    }
    fn get_children(&self, id: &ArchivedShortId) -> Option<&ArchivedIndexSet<ArchivedShortId>> {
        self.0.get_children(id)
    }
    fn get_contents(&self, id: &ArchivedShortId) -> Option<&ArchivedNodeContent> {
        self.0.get_contents(id)
    }
    fn get_ordered_identifiers(&self, output: &mut Vec<ArchivedShortId>) {
        self.0.get_ordered_identifiers(output);
    }
    fn get_ordered_identifiers_from(
        &self,
        id: &ArchivedShortId,
        output: &mut Vec<ArchivedShortId>,
    ) {
        self.0.get_ordered_identifiers_from(id, output);
    }
    fn get_active_path(&self, output: &mut Vec<ArchivedShortId>) {
        self.0.get_active_path(output);
    }
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
    fn metadata(&self) -> &ArchivedWeaveMetadata {
        self.0.metadata()
    }
}

impl ImmutableBookmarkableWeave<ArchivedShortId, ArchivedTapestryNode, ArchivedNodeContent>
    for ArchivedTapestryWeave<'_>
{
    type Bookmarks = ArchivedIndexSet<ArchivedShortId>;

    fn bookmarks(&self) -> &Self::Bookmarks {
        self.0.bookmarks()
    }
    fn contains_bookmark(&self, id: &ArchivedShortId) -> bool {
        self.0.contains_bookmark(id)
    }
}

impl ImmutableActivePathWeave<ArchivedShortId, ArchivedTapestryNode, ArchivedNodeContent>
    for ArchivedTapestryWeave<'_>
{
    type Active = ArchivedHashSet<ArchivedShortId>;

    fn active(&self) -> &Self::Active {
        self.0.active()
    }
}
