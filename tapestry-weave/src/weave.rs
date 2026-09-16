//! Document format implementations.

use std::{
    cmp::Ordering,
    hash::BuildHasherDefault,
    iter,
    num::NonZeroU128,
    ops::Range,
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
    wrappers::PatchablePathWeave,
};

use super::{
    content::{ArchivedNodeContent, Author, Creator, InnerNodeContent, NodeContent},
    metadata::{ArchivedWeaveMetadata, MetadataMap, WeaveMetadata},
    util::{Hunk, RandomIdHasher},
};

/// The file extension used for Tapestry Loom documents.
pub const FILE_EXTENSION: &str = "tapestry";

/// The magic bytes at the start of every Tapestry Loom document.
pub const HEADER_MAGIC_BYTES: [u8; 24] = *b"VersionedTapestryWeave__";

pub(crate) const FORMAT_VERSION: u64 = 1;

const DIFF_DEADLINE: Duration = Duration::from_millis(20); // 60% of the frame budget in a 30fps UI.

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

/// An [`IndependentWeave`] + [`PatchablePathWeave`] wrapper which implements Tapestry Loom's document format.
///
/// This wrapper is guaranteed to be stateless; `TapestryWeave::from(weave.into_inner())` does not result in data loss.
///
/// All identifiers *must be* randomly generated because the underlying [`Weave`]'s hashmaps use an identity hasher.
///
/// # DoS Resistance
///
/// This Weave implementation does not make use of DoS-resistant hashers.
#[derive(Debug, Clone)]
#[must_use]
pub struct TapestryWeave(
    PatchablePathWeave<TapestryWeaveInner, ShortId, TapestryNode, NodeContent>,
);

impl Default for TapestryWeave {
    fn default() -> Self {
        Self::new()
    }
}

impl From<TapestryWeaveInner> for TapestryWeave {
    fn from(value: TapestryWeaveInner) -> Self {
        Self(PatchablePathWeave::new(value))
    }
}

impl From<TapestryWeave> for TapestryWeaveInner {
    fn from(value: TapestryWeave) -> Self {
        value.0.weave
    }
}

impl AsRef<TapestryWeaveInner> for TapestryWeave {
    fn as_ref(&self) -> &TapestryWeaveInner {
        &self.0.weave
    }
}

impl TapestryWeave {
    /// Creates a new, empty [`TapestryWeave`].
    pub fn new() -> Self {
        Self::from(IndependentWeave::new(WeaveMetadata::new()))
    }
    /// Creates a new, empty [`TapestryWeave`] with at least the specified capacity.
    ///
    /// This function over-allocates for worst-case memory usage rather than average-case.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::from(IndependentWeave::with_capacity(
            capacity,
            WeaveMetadata::new(),
        ))
    }
    /// Creates a new, empty [`TapestryWeave`] with the specified metadata and at least the specified capacity.
    ///
    /// This function over-allocates for worst-case memory usage rather than average-case.
    pub fn with_capacity_and_metadata(capacity: usize, metadata: WeaveMetadata) -> Self {
        Self::from(IndependentWeave::with_capacity(capacity, metadata))
    }
    /// Converts a [`TapestryWeave`] into the underlying [`TapestryWeaveInner`].
    #[inline]
    pub fn into_inner(self) -> TapestryWeaveInner {
        self.0.weave
    }
    /// Returns a reference to the underlying [`TapestryWeaveInner`].
    #[inline]
    pub const fn as_inner(&self) -> &TapestryWeaveInner {
        &self.0.weave
    }
    /// Convenience method for `self.is_empty() && self.metadata().is_empty()`
    pub fn is_empty_including_metadata(&self) -> bool {
        self.0.is_empty() && self.0.metadata().is_empty()
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
        self.0.weave.set_active_dependent_semantics(id, value)
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
                            self.0.get_contents(id).is_some_and(|c| {
                                c.metadata == token_node.contents.metadata
                                    && c.content.is_duplicate_of(&token_node.contents.content)
                                    && c.creator.is_duplicate_of(&token_node.contents.creator)
                            })
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
    /// If successful, returns a tuple of identifiers corresponding to (token_parent, token, token_child).
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

                        if self.0.contains_active(id) {
                            assert!(self.0.set_active(&tail_id, true));
                        }

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
    /// Convenience function which returns an iterator over the content corresponding to the active path.
    pub fn active_content(&mut self) -> impl Iterator<Item = &NodeContent> {
        self.0.active_content()
    }
    /// Convenience function which returns an iterator over the text bytes corresponding to the active path.
    pub fn active_text(&mut self) -> impl Iterator<Item = u8> {
        self.0
            .active_content()
            .flat_map(|content| content.content.iter_bytes())
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
        mut generate_id: F,
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

            self.0.split_out(range, || {
                let id = generate_id();
                generated_ids.push(id);
                id
            });

            let id = generated_ids.into_iter().find(|id| {
                self.0.get(id).is_some_and(|node| {
                    node.from.is_empty()
                        && matches!(node.contents.content, InnerNodeContent::MetadataOnly)
                })
            })?;

            let _ = self.0.get_contents_mut(&id, |contents| {
                contents.creator = Creator::User(author.clone());
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
        contents: NodeContent,
        prefix_all: bool,
        author: &Option<Author>,
        mut generate_id: F,
    ) -> Option<ShortId>
    where
        F: FnMut() -> ShortId,
    {
        if at == 0
            && self
                .0
                .active_content()
                .next()
                .is_some_and(|contents| !contents.content.is_empty())
        {
            let mut generated_ids = Vec::new();

            self.0.insert_at(at, contents, prefix_all, || {
                let id = generate_id();
                generated_ids.push(id);
                id
            });

            let id = generated_ids.into_iter().find(|id| {
                self.0.get(id).is_some_and(|node| {
                    node.from.is_empty()
                        && matches!(node.contents.content, InnerNodeContent::MetadataOnly)
                })
            })?;

            let _ = self.0.get_contents_mut(&id, |contents| {
                contents.creator = Creator::User(author.clone());
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
    pub fn replace<F>(
        &mut self,
        range: Range<usize>,
        contents: NodeContent,
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

        let mut old = Vec::with_capacity(new.len());
        let mut editable = vec![true];

        for content in self.0.active_content() {
            match &content.content {
                InnerNodeContent::Tokens(tokens) => {
                    for token in tokens {
                        old.extend(token.bytes.iter().copied());
                        editable.resize(old.len(), false);
                        editable.push(true);
                    }
                }
                InnerNodeContent::Snippet(snippet) => {
                    old.extend(snippet.iter().copied());
                    editable.resize(old.len() + 1, true);
                }
                InnerNodeContent::MetadataOnly => {}
            }
        }

        let mut hunks = Hunk::calculate_diff(&old, new, Some(deadline));

        if hunks.is_empty() {
            return hunks;
        }

        let slack = |boundaries: &[usize], range: &Range<usize>| {
            let before = boundaries[boundaries.partition_point(|b| *b <= range.start) - 1];
            let after = boundaries[boundaries.partition_point(|b| *b < range.end)];

            (range.start - before, after - range.end)
        };

        let old_words = segment_text_bytes(&old, str::split_word_bound_indices);
        let new_words = segment_text_bytes(new, str::split_word_bound_indices);

        Hunk::expand_ordered(&mut hunks, old.len(), |hunk| {
            let (old_left, old_right) = slack(&old_words, &hunk.old);
            let (new_left, new_right) = slack(&new_words, &hunk.new);

            (old_left.max(new_left), old_right.max(new_right))
        });

        let mut boundaries = segment_text_bytes(&old, |text| text.grapheme_indices(true));

        boundaries.retain(|b| editable[*b]);

        Hunk::expand_ordered(&mut hunks, old.len(), |hunk| slack(&boundaries, &hunk.old));

        hunks
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

/// A [`TapestryWeave`] loaded using zero-copy deserialization.
#[derive(Clone)]
#[must_use]
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
