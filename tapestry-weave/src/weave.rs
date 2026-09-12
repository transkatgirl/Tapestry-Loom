use std::{cmp::Ordering, hash::BuildHasherDefault, num::NonZeroU128};

use nanorand::WyRand;
use rkyv::collections::swiss_table::{ArchivedHashMap, ArchivedHashSet};
use universal_weave::{
    ActivePathWeave, BookmarkableWeave, DiscreteWeave, ImmutableActivePathWeave,
    ImmutableBookmarkableWeave, ImmutableMetadataWeave, ImmutableWeave,
    IndependentWeave as IndependentWeaveTrait, MetadataWeave, SemiIndependentWeave,
    SortableBookmarkableWeave, SortableWeave, Weave,
    hashbrown::{HashMap, HashSet},
    independent::{IndependentNode, IndependentWeave},
    indexmap::IndexSet,
    rkyv::{Archive, collections::swiss_table::ArchivedIndexSet},
};

use super::{
    content::{ArchivedNodeContent, NodeContent},
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

pub struct TapestryWeave {
    pub rng: WyRand,
    pub weave: TapestryWeaveInner,
}

impl Default for TapestryWeave {
    fn default() -> Self {
        Self::new()
    }
}

impl From<TapestryWeaveInner> for TapestryWeave {
    fn from(value: TapestryWeaveInner) -> Self {
        Self {
            rng: WyRand::new(),
            weave: value,
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
    pub fn new() -> Self {
        Self {
            rng: WyRand::new(),
            weave: IndependentWeave::new(WeaveMetadata::new()),
        }
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            rng: WyRand::new(),
            weave: IndependentWeave::with_capacity(capacity, WeaveMetadata::new()),
        }
    }
    pub fn with_capacity_and_metadata(capacity: usize, metadata: WeaveMetadata) -> Self {
        Self {
            rng: WyRand::new(),
            weave: IndependentWeave::with_capacity(capacity, metadata),
        }
    }
    pub fn capacity(&self) -> usize {
        self.weave.capacity()
    }
    pub fn reserve(&mut self, additional: usize) {
        self.weave.reserve(additional);
    }
    pub fn shrink_to_fit(&mut self) {
        self.weave.shrink_to_fit();
    }
    pub fn is_empty_including_metadata(&self) -> bool {
        self.weave.is_empty() && self.weave.metadata.is_empty()
    }
    pub fn set_active_dependent_semantics(&mut self, id: &ShortId, value: bool) -> bool {
        self.weave.set_active_dependent_semantics(id, value)
    }
}

// TODO: insert_node_deduplicated, split_out_token, generate_id, get_active_content, siblings, siblings_or_roots, is_mergeable_with_parent

/*

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
                            self.weave.set_node_active_status(duplicate, true);
                            has_active = true;
                            break;
                        }
                    }

                    if !has_active {
                        self.weave
                            .set_node_active_status(duplicates.first().unwrap(), true);
                    }
                }
                self.weave.remove_node(&identifier);
            }

            self.update_shape_and_active();
        }

        status
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
    }*/

impl Weave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    type Nodes = HashMap<ShortId, TapestryNode, BuildHasherDefault<RandomIdHasher>>;
    type Roots = IndexSet<ShortId, BuildHasherDefault<RandomIdHasher>>;

    fn len(&self) -> usize {
        self.weave.len()
    }
    fn is_empty(&self) -> bool {
        self.weave.is_empty()
    }
    fn nodes(&self) -> &Self::Nodes {
        self.weave.nodes()
    }
    fn roots(&self) -> &Self::Roots {
        self.weave.roots()
    }
    fn contains(&self, id: &ShortId) -> bool {
        self.weave.contains(id)
    }
    fn contains_active(&self, id: &ShortId) -> bool {
        self.weave.contains_active(id)
    }
    fn get(&self, id: &ShortId) -> Option<&TapestryNode> {
        self.weave.get(id)
    }
    fn get_parents(
        &self,
        id: &ShortId,
    ) -> Option<&IndexSet<ShortId, BuildHasherDefault<RandomIdHasher>>> {
        self.weave.get_parents(id)
    }
    fn get_children(
        &self,
        id: &ShortId,
    ) -> Option<&IndexSet<ShortId, BuildHasherDefault<RandomIdHasher>>> {
        self.weave.get_children(id)
    }
    fn get_contents(&self, id: &ShortId) -> Option<&NodeContent> {
        self.weave.get_contents(id)
    }
    fn get_ordered_identifiers(&mut self, output: &mut Vec<ShortId>) {
        self.weave.get_ordered_identifiers(output);
    }
    fn get_ordered_identifiers_from(&mut self, id: &ShortId, output: &mut Vec<ShortId>) {
        self.weave.get_ordered_identifiers_from(id, output);
    }
    fn get_active_path(&mut self, output: &mut Vec<ShortId>) {
        self.weave.get_active_path(output);
    }
    fn get_path_from(&mut self, id: &ShortId, output: &mut Vec<ShortId>) {
        self.weave.get_path_from(id, output);
    }
    fn insert(&mut self, node: TapestryNode) -> bool {
        self.weave.insert(node)
    }
    fn set_active(&mut self, id: &ShortId, value: bool) -> bool {
        self.weave.set_active(id, value)
    }
    fn remove(&mut self, id: &ShortId) -> Option<TapestryNode> {
        self.weave.remove(id)
    }
    fn remove_tracked(&mut self, id: &ShortId, on_removal: impl FnMut(TapestryNode)) -> bool {
        self.weave.remove_tracked(id, on_removal)
    }
    fn clear(&mut self) {
        self.weave.clear();
    }
}

impl MetadataWeave<ShortId, TapestryNode, NodeContent, WeaveMetadata> for TapestryWeave {
    fn metadata(&self) -> &WeaveMetadata {
        self.weave.metadata()
    }
    fn metadata_mut<O>(&mut self, callback: impl FnOnce(&mut WeaveMetadata) -> O) -> O {
        self.weave.metadata_mut(callback)
    }
}

impl BookmarkableWeave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    type Bookmarks = IndexSet<ShortId, BuildHasherDefault<RandomIdHasher>>;

    fn bookmarks(&self) -> &Self::Bookmarks {
        self.weave.bookmarks()
    }
    fn contains_bookmark(&self, id: &ShortId) -> bool {
        self.weave.contains_bookmark(id)
    }
    fn set_bookmarked(&mut self, id: &ShortId, value: bool) -> bool {
        self.weave.set_bookmarked(id, value)
    }
}

impl SortableWeave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    fn sort_children_by(
        &mut self,
        id: &ShortId,
        cmp: impl FnMut(&TapestryNode, &TapestryNode) -> Ordering,
    ) -> bool {
        self.weave.sort_children_by(id, cmp)
    }
    fn sort_children_by_id(
        &mut self,
        id: &ShortId,
        cmp: impl FnMut(&ShortId, &ShortId) -> Ordering,
    ) -> bool {
        self.weave.sort_children_by_id(id, cmp)
    }
    fn sort_roots_by(&mut self, cmp: impl FnMut(&TapestryNode, &TapestryNode) -> Ordering) {
        self.weave.sort_roots_by(cmp);
    }
    fn sort_roots_by_id(&mut self, cmp: impl FnMut(&ShortId, &ShortId) -> Ordering) {
        self.weave.sort_roots_by_id(cmp);
    }
}

impl SortableBookmarkableWeave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    fn sort_bookmarks_by(&mut self, cmp: impl FnMut(&TapestryNode, &TapestryNode) -> Ordering) {
        self.weave.sort_bookmarks_by(cmp);
    }
    fn sort_bookmarks_by_id(&mut self, cmp: impl FnMut(&ShortId, &ShortId) -> Ordering) {
        self.weave.sort_bookmarks_by_id(cmp);
    }
}

impl ActivePathWeave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    type Active = HashSet<ShortId, BuildHasherDefault<RandomIdHasher>>;

    fn active(&self) -> &Self::Active {
        self.weave.active()
    }
    fn set_active_path(&mut self, active: impl Iterator<Item = ShortId>) {
        self.weave.set_active_path(active);
    }
}

impl IndependentWeaveTrait<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    fn move_to(&mut self, id: &ShortId, new_parents: &[ShortId]) -> bool {
        self.weave.move_to(id, new_parents)
    }
}

impl SemiIndependentWeave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    fn get_contents_mut<O>(
        &mut self,
        id: &ShortId,
        callback: impl FnOnce(&mut NodeContent) -> O,
    ) -> Option<O> {
        self.weave.get_contents_mut(id, callback)
    }
}

impl DiscreteWeave<ShortId, TapestryNode, NodeContent> for TapestryWeave {
    fn split(&mut self, id: &ShortId, at: usize, new_id: ShortId) -> bool {
        self.weave.split(id, at, new_id)
    }
    fn merge_with_parent(&mut self, id: &ShortId) -> Option<ShortId> {
        self.weave.merge_with_parent(id)
    }
}

pub struct ArchivedTapestryWeave<'a> {
    pub inner: &'a ArchivedTapestryWeaveInner,
}

impl<'a> From<&'a ArchivedTapestryWeaveInner> for ArchivedTapestryWeave<'a> {
    fn from(value: &'a ArchivedTapestryWeaveInner) -> Self {
        Self { inner: value }
    }
}

impl<'a> From<ArchivedTapestryWeave<'a>> for &'a ArchivedTapestryWeaveInner {
    fn from(value: ArchivedTapestryWeave<'a>) -> Self {
        value.inner
    }
}

impl<'a> AsRef<ArchivedTapestryWeaveInner> for ArchivedTapestryWeave<'a> {
    fn as_ref(&self) -> &'a ArchivedTapestryWeaveInner {
        self.inner
    }
}

impl ImmutableWeave<ArchivedShortId, ArchivedTapestryNode, ArchivedNodeContent>
    for ArchivedTapestryWeave<'_>
{
    type Nodes = ArchivedHashMap<ArchivedShortId, ArchivedTapestryNode>;
    type Roots = ArchivedIndexSet<ArchivedShortId>;

    fn len(&self) -> usize {
        self.inner.len()
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn nodes(&self) -> &Self::Nodes {
        self.inner.nodes()
    }
    fn roots(&self) -> &Self::Roots {
        self.inner.roots()
    }
    fn contains(&self, id: &ArchivedShortId) -> bool {
        self.inner.contains(id)
    }
    fn contains_active(&self, id: &ArchivedShortId) -> bool {
        self.inner.contains_active(id)
    }
    fn get(&self, id: &ArchivedShortId) -> Option<&ArchivedTapestryNode> {
        self.inner.get(id)
    }
    fn get_parents(&self, id: &ArchivedShortId) -> Option<&ArchivedIndexSet<ArchivedShortId>> {
        self.inner.get_parents(id)
    }
    fn get_children(&self, id: &ArchivedShortId) -> Option<&ArchivedIndexSet<ArchivedShortId>> {
        self.inner.get_children(id)
    }
    fn get_contents(&self, id: &ArchivedShortId) -> Option<&ArchivedNodeContent> {
        self.inner.get_contents(id)
    }
    fn get_ordered_identifiers(&self, output: &mut Vec<ArchivedShortId>) {
        self.inner.get_ordered_identifiers(output);
    }
    fn get_ordered_identifiers_from(
        &self,
        id: &ArchivedShortId,
        output: &mut Vec<ArchivedShortId>,
    ) {
        self.inner.get_ordered_identifiers_from(id, output);
    }
    fn get_active_path(&self, output: &mut Vec<ArchivedShortId>) {
        self.inner.get_active_path(output);
    }
    fn get_path_from(&self, id: &ArchivedShortId, output: &mut Vec<ArchivedShortId>) {
        self.inner.get_path_from(id, output);
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
        self.inner.metadata()
    }
}

impl ImmutableBookmarkableWeave<ArchivedShortId, ArchivedTapestryNode, ArchivedNodeContent>
    for ArchivedTapestryWeave<'_>
{
    type Bookmarks = ArchivedIndexSet<ArchivedShortId>;

    fn bookmarks(&self) -> &Self::Bookmarks {
        self.inner.bookmarks()
    }
    fn contains_bookmark(&self, id: &ArchivedShortId) -> bool {
        self.inner.contains_bookmark(id)
    }
}

impl ImmutableActivePathWeave<ArchivedShortId, ArchivedTapestryNode, ArchivedNodeContent>
    for ArchivedTapestryWeave<'_>
{
    type Active = ArchivedHashSet<ArchivedShortId>;

    fn active(&self) -> &Self::Active {
        self.inner.active()
    }
}
