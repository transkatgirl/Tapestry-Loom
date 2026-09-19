//! Wrappers which add functionality to a [`TapestryWeave`].

use std::{cmp::Ordering, collections::VecDeque, hash::BuildHasherDefault, ops::Range};

use jiff::Zoned;
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};
use universal_weave::{
    ActivePathWeave, BookmarkableWeave, DiscreteWeave, IndependentWeave as IndependentWeaveTrait,
    MetadataWeave, SemiIndependentWeave, SortableBookmarkableWeave, SortableWeave, Weave,
    hashbrown::{HashMap, HashSet},
    indexmap::IndexSet,
    rkyv::{Archive, Deserialize, Serialize, with::Map},
    wrappers::WeaveAction,
};

use crate::{
    content::{Author, Creator, InnerNodeContent, NodeContent},
    metadata::{MetadataMap, WeaveMetadata},
    util::{AsBinaryZoned, Hunk, RandomIdHasher},
    weave::{ShortId, TapestryNode, TapestryWeave, record_ids},
};

/// A [`TapestryWeave`] wrapper which logs actions successfully performed on the inner [`TapestryWeave`] in the order that they are performed.
///
/// See [`TapestryWeaveAction`] for the complete list of loggable actions.
#[derive(Debug, Clone)]
#[must_use]
pub struct LoggedTapestryWeave {
    /// The [`TapestryWeave`] being wrapped.
    ///
    /// Actions performed directly on the inner [`TapestryWeave`] (without using the wrapper's functions) are not logged.
    pub weave: TapestryWeave,

    /// The list of actions that were performed on the attached [`TapestryWeave`] in the order they were performed.
    pub actions: VecDeque<TapestryWeaveAction>,
}

impl Default for LoggedTapestryWeave {
    #[inline]
    fn default() -> Self {
        Self::new(TapestryWeave::default())
    }
}

impl AsRef<TapestryWeave> for LoggedTapestryWeave {
    #[inline]
    fn as_ref(&self) -> &TapestryWeave {
        &self.weave
    }
}

impl From<TapestryWeave> for LoggedTapestryWeave {
    #[inline]
    fn from(value: TapestryWeave) -> Self {
        Self::new(value)
    }
}

impl From<LoggedTapestryWeave> for TapestryWeave {
    #[inline]
    fn from(value: LoggedTapestryWeave) -> Self {
        value.weave
    }
}

impl LoggedTapestryWeave {
    /// Creates a new [`LoggedTapestryWeave`] from a [`TapestryWeave`].
    #[inline]
    pub const fn new(weave: TapestryWeave) -> Self {
        Self {
            actions: VecDeque::new(),
            weave,
        }
    }
    /// Creates a new [`LoggedTapestryWeave`] with at least the specified action capacity from a [`TapestryWeave`].
    #[inline]
    pub fn with_capacity(weave: TapestryWeave, capacity: usize) -> Self {
        Self {
            actions: VecDeque::with_capacity(capacity),
            weave,
        }
    }
    /// Creates a new [`LoggedTapestryWeave`] from a [`TapestryWeave`].
    #[inline]
    pub const fn from_weave(weave: TapestryWeave) -> Self {
        Self::new(weave)
    }
    /// Converts a [`LoggedTapestryWeave`] into its inner [`TapestryWeave`].
    #[inline]
    pub fn into_weave(self) -> TapestryWeave {
        self.weave
    }
    /// Returns a reference to the inner [`TapestryWeave`].
    #[inline]
    pub const fn as_weave(&self) -> &TapestryWeave {
        &self.weave
    }
    /// Returns a reference to the list of actions performed on the [`TapestryWeave`].
    #[inline]
    pub const fn as_actions(&self) -> &VecDeque<TapestryWeaveAction> {
        &self.actions
    }
    /// Clears the inner list of actions performed on the [`TapestryWeave`].
    #[inline]
    pub fn clear_actions(&mut self) {
        self.actions.clear();
    }
    /// Applies a [`TapestryWeaveAction`] to the [`LoggedTapestryWeave`].
    ///
    /// # Panics
    ///
    /// May panic if applying the action fails.
    pub fn apply(&mut self, action: TapestryWeaveAction) {
        self.weave.apply(action.clone());
        self.actions.push_back(action);
    }
}

/// An action performed on a [`TapestryWeave`] which changes its outwardly facing state.
///
/// When possible, actions map to a function of [`TapestryWeave`] (or a trait it implements), and use the same argument ordering as their corresponding function.
///
/// Some actions not logged here may change the [`TapestryWeave`]'s inner state but not its outwardly facing state.
#[derive(
    SerdeSerialize, SerdeDeserialize, Archive, Deserialize, Serialize, Debug, Clone, PartialEq,
)]
#[allow(clippy::doc_paragraphs_missing_punctuation, reason = "False positive")]
#[non_exhaustive]
#[must_use]
pub enum TapestryWeaveAction {
    /// [`Weave::insert()`] or [`TapestryWeave::insert_deduplicated()`]
    Insert(TapestryNode),
    /// [`Weave::set_active()`]
    SetActive { id: ShortId, value: bool },
    /// [`BookmarkableWeave::set_bookmarked()`]
    SetBookmarked { id: ShortId, value: bool },
    /// [`Weave::remove()`] or [`Weave::remove_tracked()`]
    Remove(ShortId),
    /// [`Weave::clear()`]
    Clear,
    /// [`MetadataWeave::metadata_mut()`]
    SetMetadata(WeaveMetadata),
    /// Caused by [`SortableWeave::sort_children_by()`], [`SortableWeave::sort_children_by_id()`], [`SortableWeave::sort_roots_by()`], [`SortableWeave::sort_roots_by_id()`], [`TapestryWeave::sort_roots_or_children_by`], and [`TapestryWeave::sort_roots_or_children_by_id`]
    SetChildOrdering {
        parent: Option<ShortId>,
        children: Vec<ShortId>,
    },
    /// Caused by [`SortableBookmarkableWeave::sort_bookmarks_by()`] and [`SortableBookmarkableWeave::sort_bookmarks_by_id()`]
    SetBookmarkOrdering(Vec<ShortId>),
    /// [`ActivePathWeave::set_active_path()`]
    SetActivePath(Vec<ShortId>),
    /// [`IndependentWeave::move_to()`](IndependentWeaveTrait::move_to)
    MoveTo {
        id: ShortId,
        new_parents: Vec<ShortId>,
    },
    /// Caused by [`SemiIndependentWeave::get_contents_mut()`]
    SetContents { id: ShortId, contents: NodeContent },
    /// [`DiscreteWeave::split()`]
    Split {
        id: ShortId,
        at: usize,
        new_id: ShortId,
    },
    /// [`DiscreteWeave::merge_with_parent()`]
    MergeWithParent(ShortId),
    /// [`TapestryWeave::set_active_tree_semantics()`]
    SetActiveTreeSemantics { id: ShortId, value: bool },
    /// [`TapestryWeave::split_tokenized()`]
    SplitTokenized {
        id: ShortId,
        at: usize,
        generated_ids: Vec<ShortId>,
    },
    /// [`TapestryWeave::split_out_token()`]
    SplitOutToken {
        id: ShortId,
        index: usize,
        generated_ids: Vec<ShortId>,
    },
    /// [`TapestryWeave::split_out()`]
    SplitOut {
        range: Range<usize>,
        author: Option<Author>,
        generated_ids: Vec<ShortId>,
        #[rkyv(with = Map<AsBinaryZoned>)]
        root_timestamp: Option<Zoned>,
    },
    /// [`TapestryWeave::split_at()`]
    SplitAt {
        at: usize,
        generated_ids: Vec<ShortId>,
    },
    /// [`TapestryWeave::insert_at()`]
    InsertAt {
        at: usize,
        contents: NodeContent,
        prefix_all: bool,
        author: Option<Author>,
        generated_ids: Vec<ShortId>,
        #[rkyv(with = Map<AsBinaryZoned>)]
        root_timestamp: Option<Zoned>,
    },
    /// [`TapestryWeave::replace()`]
    Replace {
        range: Range<usize>,
        contents: NodeContent,
        prefix_all: bool,
        author: Option<Author>,
        generated_ids: Vec<ShortId>,
        #[rkyv(with = Map<AsBinaryZoned>)]
        root_timestamp: Option<Zoned>,
    },
    /// [`TapestryWeave::update_active_text()`]
    UpdateActiveText {
        actions: Vec<TapestryWeaveTextUpdateAction>,
    },
}

/// An action performed on a [`TapestryWeave`] during the execution of [`TapestryWeave::update_active_text()`]
#[derive(
    SerdeSerialize, SerdeDeserialize, Archive, Deserialize, Serialize, Debug, Clone, PartialEq,
)]
#[allow(clippy::doc_paragraphs_missing_punctuation, reason = "False positive")]
#[allow(clippy::large_enum_variant, reason = "Inherent")]
#[non_exhaustive]
#[must_use]
pub enum TapestryWeaveTextUpdateAction {
    /// [`TapestryWeave::split_out()`]
    SplitOut {
        range: Range<usize>,
        author: Option<Author>,
        generated_ids: Vec<ShortId>,
        #[rkyv(with = Map<AsBinaryZoned>)]
        root_timestamp: Option<Zoned>,
    },
    /// [`TapestryWeave::replace()`]
    Replace {
        range: Range<usize>,
        contents: NodeContent,
        prefix_all: bool,
        author: Option<Author>,
        generated_ids: Vec<ShortId>,
        #[rkyv(with = Map<AsBinaryZoned>)]
        root_timestamp: Option<Zoned>,
    },
}

impl From<WeaveAction<ShortId, TapestryNode, NodeContent, WeaveMetadata>> for TapestryWeaveAction {
    fn from(value: WeaveAction<ShortId, TapestryNode, NodeContent, WeaveMetadata>) -> Self {
        match value {
            WeaveAction::Insert(node) => Self::Insert(node),
            WeaveAction::SetActive { id, value } => Self::SetActive { id, value },
            WeaveAction::SetBookmarked { id, value } => Self::SetBookmarked { id, value },
            WeaveAction::Remove(id) => Self::Remove(id),
            WeaveAction::Clear => Self::Clear,
            WeaveAction::SetMetadata(metadata) => Self::SetMetadata(metadata),
            WeaveAction::SetChildOrdering { parent, children } => {
                Self::SetChildOrdering { parent, children }
            }
            WeaveAction::SetBookmarkOrdering(ids) => Self::SetBookmarkOrdering(ids),
            WeaveAction::SetActivePath(active) => Self::SetActivePath(active),
            WeaveAction::MoveTo { id, new_parents } => Self::MoveTo { id, new_parents },
            WeaveAction::SetContents { id, contents } => Self::SetContents { id, contents },
            WeaveAction::Split { id, at, new_id } => Self::Split { id, at, new_id },
            WeaveAction::MergeWithParent(id) => Self::MergeWithParent(id),
            _ => unimplemented!(),
        }
    }
}

impl TryFrom<TapestryWeaveAction>
    for WeaveAction<ShortId, TapestryNode, NodeContent, WeaveMetadata>
{
    type Error = TapestryWeaveAction;

    fn try_from(value: TapestryWeaveAction) -> Result<Self, Self::Error> {
        match value {
            TapestryWeaveAction::Insert(node) => Ok(Self::Insert(node)),
            TapestryWeaveAction::SetActive { id, value } => Ok(Self::SetActive { id, value }),
            TapestryWeaveAction::SetBookmarked { id, value } => {
                Ok(Self::SetBookmarked { id, value })
            }
            TapestryWeaveAction::Remove(id) => Ok(Self::Remove(id)),
            TapestryWeaveAction::Clear => Ok(Self::Clear),
            TapestryWeaveAction::SetMetadata(metadata) => Ok(Self::SetMetadata(metadata)),
            TapestryWeaveAction::SetChildOrdering { parent, children } => {
                Ok(Self::SetChildOrdering { parent, children })
            }
            TapestryWeaveAction::SetBookmarkOrdering(ids) => Ok(Self::SetBookmarkOrdering(ids)),
            TapestryWeaveAction::SetActivePath(active) => Ok(Self::SetActivePath(active)),
            TapestryWeaveAction::MoveTo { id, new_parents } => Ok(Self::MoveTo { id, new_parents }),
            TapestryWeaveAction::SetContents { id, contents } => {
                Ok(Self::SetContents { id, contents })
            }
            TapestryWeaveAction::Split { id, at, new_id } => Ok(Self::Split { id, at, new_id }),
            TapestryWeaveAction::MergeWithParent(id) => Ok(Self::MergeWithParent(id)),
            action => Err(action),
        }
    }
}

impl TapestryWeave {
    /// Applies a [`TapestryWeaveAction`] to the [`TapestryWeave`].
    ///
    /// # Panics
    ///
    /// May panic if applying the action fails.
    pub fn apply(&mut self, action: TapestryWeaveAction) {
        fn replay_ids(generated_ids: Vec<ShortId>) -> impl FnMut() -> ShortId {
            let mut ids = generated_ids.into_iter();
            move || ids.next().unwrap()
        }

        let apply_root_timestamp =
            |weave: &mut Self, root: Option<ShortId>, timestamp: Option<Zoned>| {
                if let Some(root) = root
                    && let Some(timestamp) = timestamp
                {
                    assert!(
                        weave
                            .get_contents_mut(&root, |contents| contents.timestamp = timestamp)
                            .is_some(),
                        "Failed to apply Weave action"
                    );
                }
            };

        match action {
            TapestryWeaveAction::Insert(node) => {
                assert!(self.insert(node), "Failed to apply Weave action");
            }
            TapestryWeaveAction::SetActive { id, value } => {
                assert!(self.set_active(&id, value), "Failed to apply Weave action");
            }
            TapestryWeaveAction::SetBookmarked { id, value } => {
                assert!(
                    self.set_bookmarked(&id, value),
                    "Failed to apply Weave action"
                );
            }
            TapestryWeaveAction::Remove(id) => {
                assert!(self.remove(&id).is_some(), "Failed to apply Weave action");
            }
            TapestryWeaveAction::Clear => self.clear(),
            TapestryWeaveAction::SetMetadata(metadata) => {
                self.metadata_mut(|m| *m = metadata);
            }
            TapestryWeaveAction::SetChildOrdering { parent, children } => {
                let mut id_mapping: HashMap<ShortId, usize, BuildHasherDefault<RandomIdHasher>> =
                    HashMap::with_capacity_and_hasher(
                        children.len(),
                        BuildHasherDefault::default(),
                    );
                id_mapping.extend(
                    children
                        .into_iter()
                        .enumerate()
                        .map(|(index, id)| (id, index)),
                );

                match parent {
                    Some(id) => {
                        assert!(
                            self.sort_children_by_id(&id, |a, b| {
                                id_mapping[a].cmp(&id_mapping[b])
                            }),
                            "Failed to apply Weave action"
                        );
                    }
                    None => {
                        self.sort_roots_by_id(|a, b| id_mapping[a].cmp(&id_mapping[b]));
                    }
                }
            }
            TapestryWeaveAction::SetBookmarkOrdering(ids) => {
                let mut id_mapping: HashMap<ShortId, usize, BuildHasherDefault<RandomIdHasher>> =
                    HashMap::with_capacity_and_hasher(ids.len(), BuildHasherDefault::default());
                id_mapping.extend(ids.into_iter().enumerate().map(|(index, id)| (id, index)));

                self.sort_bookmarks_by_id(|a, b| id_mapping[a].cmp(&id_mapping[b]));
            }
            TapestryWeaveAction::SetActivePath(active) => {
                self.set_active_path(active.into_iter());
            }
            TapestryWeaveAction::MoveTo { id, new_parents } => assert!(
                self.move_to(&id, &new_parents),
                "Failed to apply Weave action"
            ),
            TapestryWeaveAction::SetContents { id, contents } => {
                assert!(
                    self.get_contents_mut(&id, |c| *c = contents).is_some(),
                    "Failed to apply Weave action"
                );
            }
            TapestryWeaveAction::Split { id, at, new_id } => {
                assert!(self.split(&id, at, new_id), "Failed to apply Weave action");
            }
            TapestryWeaveAction::MergeWithParent(id) => assert!(
                self.merge_with_parent(&id).is_some(),
                "Failed to apply Weave action"
            ),
            TapestryWeaveAction::SetActiveTreeSemantics { id, value } => {
                assert!(
                    self.set_active_tree_semantics(&id, value),
                    "Failed to apply Weave action"
                );
            }
            TapestryWeaveAction::SplitTokenized {
                id,
                at,
                generated_ids,
            } => {
                assert!(
                    self.split_tokenized(&id, at, replay_ids(generated_ids))
                        .is_some(),
                    "Failed to apply Weave action"
                );
            }
            TapestryWeaveAction::SplitOutToken {
                id,
                index,
                generated_ids,
            } => {
                assert!(
                    self.split_out_token(&id, index, replay_ids(generated_ids))
                        .is_some(),
                    "Failed to apply Weave action"
                );
            }
            TapestryWeaveAction::SplitOut {
                range,
                author,
                generated_ids,
                root_timestamp,
            } => {
                let root = self.split_out(range, &author, replay_ids(generated_ids));

                apply_root_timestamp(self, root, root_timestamp);
            }
            TapestryWeaveAction::SplitAt { at, generated_ids } => {
                self.split_at(at, replay_ids(generated_ids));
            }
            TapestryWeaveAction::InsertAt {
                at,
                contents,
                prefix_all,
                author,
                generated_ids,
                root_timestamp,
            } => {
                let root =
                    self.insert_at(at, contents, prefix_all, &author, replay_ids(generated_ids));

                apply_root_timestamp(self, root, root_timestamp);
            }
            TapestryWeaveAction::Replace {
                range,
                contents,
                prefix_all,
                author,
                generated_ids,
                root_timestamp,
            } => {
                let root = self.replace(
                    range,
                    contents,
                    prefix_all,
                    &author,
                    replay_ids(generated_ids),
                );

                apply_root_timestamp(self, root, root_timestamp);
            }
            TapestryWeaveAction::UpdateActiveText { actions } => {
                for action in actions {
                    match action {
                        TapestryWeaveTextUpdateAction::SplitOut {
                            range,
                            author,
                            generated_ids,
                            root_timestamp,
                        } => {
                            let root = self.split_out(range, &author, replay_ids(generated_ids));

                            apply_root_timestamp(self, root, root_timestamp);
                        }
                        TapestryWeaveTextUpdateAction::Replace {
                            range,
                            contents,
                            prefix_all,
                            author,
                            generated_ids,
                            root_timestamp,
                        } => {
                            let root = self.replace(
                                range,
                                contents,
                                prefix_all,
                                &author,
                                replay_ids(generated_ids),
                            );

                            apply_root_timestamp(self, root, root_timestamp);
                        }
                    }
                }
            }
        }
    }
}

impl LoggedTapestryWeave {
    fn timestamp_empty_root(&mut self, root: Option<ShortId>) -> Option<Zoned> {
        let root = root?;
        let timestamp = Zoned::now();

        self.weave
            .get_contents_mut(&root, |contents| {
                contents.timestamp = timestamp.clone();
            })
            .unwrap();

        Some(timestamp)
    }
    /// Convenience method for `self.is_empty() && self.metadata().is_empty()`
    #[inline]
    pub fn is_empty_including_metadata(&self) -> bool {
        self.weave.is_empty_including_metadata()
    }
    /// Convenience method which returns the siblings of the node corresponding to the identifier.
    ///
    /// This function may return duplicate identifiers.
    #[inline]
    pub fn get_siblings<'a>(
        &'a self,
        id: &ShortId,
        include_roots: bool,
    ) -> Option<Box<dyn Iterator<Item = ShortId> + 'a>> {
        self.weave.get_siblings(id, include_roots)
    }
    /// A wrapper around [`Weave::insert`] which prevents nodes with duplicate siblings from being inserted.
    #[must_use]
    pub fn insert_deduplicated(&mut self, node: TapestryNode) -> bool {
        if self.weave.insert_deduplicated(node.clone()) {
            self.actions.push_back(TapestryWeaveAction::Insert(node));
            true
        } else {
            false
        }
    }
    /// Sets the active status of a node with the specified identifier, using identical activation behavior to a tree-based Weave.
    pub fn set_active_tree_semantics(&mut self, id: &ShortId, value: bool) -> bool {
        if self.weave.set_active_tree_semantics(id, value) {
            self.actions
                .push_back(TapestryWeaveAction::SetActiveTreeSemantics { id: *id, value });
            true
        } else {
            false
        }
    }
    /// Convenience function which calls either [`Self::sort_roots_by`] or [`Self::sort_children_by`] depending on whether or not `id` is None
    #[inline]
    pub fn sort_roots_or_children_by(
        &mut self,
        id: &Option<ShortId>,
        cmp: impl FnMut(&TapestryNode, &TapestryNode) -> Ordering,
    ) -> bool {
        if let Some(id) = id {
            self.sort_children_by(id, cmp)
        } else {
            self.sort_roots_by(cmp);
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
            self.sort_children_by_id(id, cmp)
        } else {
            self.sort_roots_by_id(cmp);
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
        generate_id: impl FnMut() -> ShortId,
    ) -> Option<(Option<ShortId>, ShortId)> {
        let mut generated_ids = Vec::new();

        let output =
            self.weave
                .split_tokenized(id, at, record_ids(generate_id, &mut generated_ids));

        if output.is_some() {
            self.actions.push_back(TapestryWeaveAction::SplitTokenized {
                id: *id,
                at,
                generated_ids,
            });
        }

        output
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
        generate_id: impl FnMut() -> ShortId,
    ) -> Option<(Option<ShortId>, ShortId, Option<ShortId>)> {
        let mut generated_ids = Vec::new();

        let output =
            self.weave
                .split_out_token(id, index, record_ids(generate_id, &mut generated_ids));

        if output.is_some() {
            self.actions.push_back(TapestryWeaveAction::SplitOutToken {
                id: *id,
                index,
                generated_ids,
            });
        }

        output
    }
    /// Returns `true` if [`Self::merge_with_parent`] would succeed.
    #[inline]
    pub fn is_mergeable_with_parent(&self, id: &ShortId) -> bool {
        self.weave.is_mergeable_with_parent(id)
    }
    /// Convenience function which returns an iterator over the content corresponding to the active path.
    #[inline]
    pub fn active_content(&mut self) -> impl Iterator<Item = &NodeContent> {
        self.weave.active_content()
    }
    /// Convenience function which returns an iterator over the text bytes corresponding to the active path.
    #[inline]
    pub fn active_text(&mut self) -> impl Iterator<Item = u8> {
        self.weave.active_text()
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
        let mut generated_ids = Vec::new();

        let root = self.weave.split_out(
            range.clone(),
            author,
            record_ids(generate_id, &mut generated_ids),
        );
        let root_timestamp = self.timestamp_empty_root(root);

        self.actions.push_back(TapestryWeaveAction::SplitOut {
            range,
            author: author.clone(),
            generated_ids,
            root_timestamp,
        });

        root
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
        let mut generated_ids = Vec::new();

        self.weave
            .split_at(at, record_ids(generate_id, &mut generated_ids));

        self.actions
            .push_back(TapestryWeaveAction::SplitAt { at, generated_ids });
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
        generate_id: F,
    ) -> Option<ShortId>
    where
        F: FnMut() -> ShortId,
    {
        let mut generated_ids = Vec::new();

        let root = self.weave.insert_at(
            at,
            contents.clone(),
            prefix_all,
            author,
            record_ids(generate_id, &mut generated_ids),
        );
        let root_timestamp = self.timestamp_empty_root(root);

        self.actions.push_back(TapestryWeaveAction::InsertAt {
            at,
            contents,
            prefix_all,
            author: author.clone(),
            generated_ids,
            root_timestamp,
        });

        root
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
        let mut generated_ids = Vec::new();

        let root = self.weave.replace(
            range.clone(),
            contents.clone(),
            prefix_all,
            author,
            record_ids(generate_id, &mut generated_ids),
        );
        let root_timestamp = self.timestamp_empty_root(root);

        self.actions.push_back(TapestryWeaveAction::Replace {
            range,
            contents,
            prefix_all,
            author: author.clone(),
            generated_ids,
            root_timestamp,
        });

        root
    }
    /// Calculates a readable diff between `new` and the text bytes corresponding to the active path.
    ///
    /// Diff calculation time (but not post-processing time) is bounded, making this function generally safe to use in user interfaces.
    ///
    /// The exact diff calculation and semantic post-processing algorithms used are implementation-specific and subject to change.
    #[inline]
    pub fn diff_active_text(&mut self, new: &[u8]) -> Vec<Hunk> {
        self.weave.diff_active_text(new)
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
        let hunks = self.diff_active_text(new);

        if hunks.is_empty() {
            return;
        }

        let mut actions = Vec::with_capacity(hunks.len());

        for hunk in hunks.into_iter().rev() {
            let mut generated_ids = Vec::new();

            if hunk.new.is_empty() {
                let root = self.weave.split_out(
                    hunk.old.clone(),
                    author,
                    record_ids(&mut generate_id, &mut generated_ids),
                );
                let root_timestamp = self.timestamp_empty_root(root);

                actions.push(TapestryWeaveTextUpdateAction::SplitOut {
                    range: hunk.old,
                    author: author.clone(),
                    generated_ids,
                    root_timestamp,
                });
            } else {
                let contents = NodeContent {
                    timestamp: timestamp.clone(),
                    modified: false,
                    content: InnerNodeContent::Snippet(new[hunk.new].to_vec()),
                    metadata: MetadataMap::default(),
                    creator: Creator::User(author.clone()),
                };

                let root = self.weave.replace(
                    hunk.old.clone(),
                    contents.clone(),
                    false,
                    author,
                    record_ids(&mut generate_id, &mut generated_ids),
                );
                let root_timestamp = self.timestamp_empty_root(root);

                actions.push(TapestryWeaveTextUpdateAction::Replace {
                    range: hunk.old,
                    contents,
                    prefix_all: false,
                    author: author.clone(),
                    generated_ids,
                    root_timestamp,
                });
            }
        }

        self.actions
            .push_back(TapestryWeaveAction::UpdateActiveText { actions });
    }
}

impl Weave<ShortId, TapestryNode, NodeContent> for LoggedTapestryWeave {
    type Nodes = HashMap<ShortId, TapestryNode, BuildHasherDefault<RandomIdHasher>>;
    type Roots = IndexSet<ShortId, BuildHasherDefault<RandomIdHasher>>;

    #[inline]
    fn len(&self) -> usize {
        self.weave.len()
    }
    #[inline]
    fn is_empty(&self) -> bool {
        self.weave.is_empty()
    }
    #[inline]
    fn nodes(&self) -> &Self::Nodes {
        self.weave.nodes()
    }
    #[inline]
    fn roots(&self) -> &Self::Roots {
        self.weave.roots()
    }
    #[inline]
    fn contains(&self, id: &ShortId) -> bool {
        self.weave.contains(id)
    }
    #[inline]
    fn contains_active(&self, id: &ShortId) -> bool {
        self.weave.contains_active(id)
    }
    #[inline]
    fn get(&self, id: &ShortId) -> Option<&TapestryNode> {
        self.weave.get(id)
    }
    #[inline]
    fn get_parents(
        &self,
        id: &ShortId,
    ) -> Option<&IndexSet<ShortId, BuildHasherDefault<RandomIdHasher>>> {
        self.weave.get_parents(id)
    }
    #[inline]
    fn get_children(
        &self,
        id: &ShortId,
    ) -> Option<&IndexSet<ShortId, BuildHasherDefault<RandomIdHasher>>> {
        self.weave.get_children(id)
    }
    #[inline]
    fn get_contents(&self, id: &ShortId) -> Option<&NodeContent> {
        self.weave.get_contents(id)
    }
    #[inline]
    fn get_ordered_identifiers(&mut self, output: &mut Vec<ShortId>) {
        self.weave.get_ordered_identifiers(output);
    }
    #[inline]
    fn get_ordered_identifiers_from(&mut self, id: &ShortId, output: &mut Vec<ShortId>) {
        self.weave.get_ordered_identifiers_from(id, output);
    }
    #[inline]
    fn get_active_path(&mut self, output: &mut Vec<ShortId>) {
        self.weave.get_active_path(output);
    }
    #[inline]
    fn get_path_from(&mut self, id: &ShortId, output: &mut Vec<ShortId>) {
        self.weave.get_path_from(id, output);
    }
    fn insert(&mut self, node: TapestryNode) -> bool {
        if self.weave.insert(node.clone()) {
            self.actions.push_back(TapestryWeaveAction::Insert(node));
            true
        } else {
            false
        }
    }
    fn set_active(&mut self, id: &ShortId, value: bool) -> bool {
        if self.weave.set_active(id, value) {
            self.actions
                .push_back(TapestryWeaveAction::SetActive { id: *id, value });
            true
        } else {
            false
        }
    }
    fn remove(&mut self, id: &ShortId) -> Option<TapestryNode> {
        if let Some(removed) = self.weave.remove(id) {
            self.actions.push_back(TapestryWeaveAction::Remove(*id));
            Some(removed)
        } else {
            None
        }
    }
    fn remove_tracked(&mut self, id: &ShortId, on_removal: impl FnMut(TapestryNode)) -> bool {
        if self.weave.remove_tracked(id, on_removal) {
            self.actions.push_back(TapestryWeaveAction::Remove(*id));
            true
        } else {
            false
        }
    }
    fn clear(&mut self) {
        self.weave.clear();
        self.actions.push_back(TapestryWeaveAction::Clear);
    }
}

impl MetadataWeave<ShortId, TapestryNode, NodeContent, WeaveMetadata> for LoggedTapestryWeave {
    #[inline]
    fn metadata(&self) -> &WeaveMetadata {
        self.weave.metadata()
    }
    fn metadata_mut<O>(&mut self, callback: impl FnOnce(&mut WeaveMetadata) -> O) -> O {
        self.weave.metadata_mut(|metadata| {
            let output = callback(metadata);

            self.actions
                .push_back(TapestryWeaveAction::SetMetadata(metadata.clone()));

            output
        })
    }
}

impl BookmarkableWeave<ShortId, TapestryNode, NodeContent> for LoggedTapestryWeave {
    type Bookmarks = IndexSet<ShortId, BuildHasherDefault<RandomIdHasher>>;

    #[inline]
    fn bookmarks(&self) -> &Self::Bookmarks {
        self.weave.bookmarks()
    }
    #[inline]
    fn contains_bookmark(&self, id: &ShortId) -> bool {
        self.weave.contains_bookmark(id)
    }
    fn set_bookmarked(&mut self, id: &ShortId, value: bool) -> bool {
        if self.weave.set_bookmarked(id, value) {
            self.actions
                .push_back(TapestryWeaveAction::SetBookmarked { id: *id, value });
            true
        } else {
            false
        }
    }
}

impl SortableWeave<ShortId, TapestryNode, NodeContent> for LoggedTapestryWeave {
    fn sort_children_by(
        &mut self,
        id: &ShortId,
        cmp: impl FnMut(&TapestryNode, &TapestryNode) -> Ordering,
    ) -> bool {
        if self.weave.sort_children_by(id, cmp) {
            self.actions
                .push_back(TapestryWeaveAction::SetChildOrdering {
                    parent: Some(*id),
                    children: self
                        .weave
                        .get_children(id)
                        .unwrap()
                        .iter()
                        .copied()
                        .collect(),
                });
            true
        } else {
            false
        }
    }
    fn sort_children_by_id(
        &mut self,
        id: &ShortId,
        cmp: impl FnMut(&ShortId, &ShortId) -> Ordering,
    ) -> bool {
        if self.weave.sort_children_by_id(id, cmp) {
            self.actions
                .push_back(TapestryWeaveAction::SetChildOrdering {
                    parent: Some(*id),
                    children: self
                        .weave
                        .get_children(id)
                        .unwrap()
                        .iter()
                        .copied()
                        .collect(),
                });
            true
        } else {
            false
        }
    }
    fn sort_roots_by(&mut self, cmp: impl FnMut(&TapestryNode, &TapestryNode) -> Ordering) {
        self.weave.sort_roots_by(cmp);
        self.actions
            .push_back(TapestryWeaveAction::SetChildOrdering {
                parent: None,
                children: self.weave.roots().iter().copied().collect(),
            });
    }
    fn sort_roots_by_id(&mut self, cmp: impl FnMut(&ShortId, &ShortId) -> Ordering) {
        self.weave.sort_roots_by_id(cmp);
        self.actions
            .push_back(TapestryWeaveAction::SetChildOrdering {
                parent: None,
                children: self.weave.roots().iter().copied().collect(),
            });
    }
}

impl SortableBookmarkableWeave<ShortId, TapestryNode, NodeContent> for LoggedTapestryWeave {
    fn sort_bookmarks_by(&mut self, cmp: impl FnMut(&TapestryNode, &TapestryNode) -> Ordering) {
        self.weave.sort_bookmarks_by(cmp);
        self.actions
            .push_back(TapestryWeaveAction::SetBookmarkOrdering(
                self.weave.bookmarks().iter().copied().collect(),
            ));
    }
    fn sort_bookmarks_by_id(&mut self, cmp: impl FnMut(&ShortId, &ShortId) -> Ordering) {
        self.weave.sort_bookmarks_by_id(cmp);
        self.actions
            .push_back(TapestryWeaveAction::SetBookmarkOrdering(
                self.weave.bookmarks().iter().copied().collect(),
            ));
    }
}

impl ActivePathWeave<ShortId, TapestryNode, NodeContent> for LoggedTapestryWeave {
    type Active = HashSet<ShortId, BuildHasherDefault<RandomIdHasher>>;

    #[inline]
    fn active(&self) -> &Self::Active {
        self.weave.active()
    }
    fn set_active_path(&mut self, active: impl Iterator<Item = ShortId>) {
        let active: Vec<ShortId> = active.collect();

        self.weave.set_active_path(active.iter().copied());
        self.actions
            .push_back(TapestryWeaveAction::SetActivePath(active));
    }
}

impl IndependentWeaveTrait<ShortId, TapestryNode, NodeContent> for LoggedTapestryWeave {
    fn move_to(&mut self, id: &ShortId, new_parents: &[ShortId]) -> bool {
        if self.weave.move_to(id, new_parents) {
            self.actions.push_back(TapestryWeaveAction::MoveTo {
                id: *id,
                new_parents: new_parents.to_vec(),
            });
            true
        } else {
            false
        }
    }
}

impl SemiIndependentWeave<ShortId, TapestryNode, NodeContent> for LoggedTapestryWeave {
    fn get_contents_mut<O>(
        &mut self,
        id: &ShortId,
        callback: impl FnOnce(&mut NodeContent) -> O,
    ) -> Option<O> {
        self.weave
            .get_contents_mut(id, |contents| (callback(contents), contents.clone()))
            .map(|(output, contents)| {
                self.actions
                    .push_back(TapestryWeaveAction::SetContents { id: *id, contents });

                output
            })
    }
}

impl DiscreteWeave<ShortId, TapestryNode, NodeContent> for LoggedTapestryWeave {
    fn split(&mut self, id: &ShortId, at: usize, new_id: ShortId) -> bool {
        if self.weave.split(id, at, new_id) {
            self.actions.push_back(TapestryWeaveAction::Split {
                id: *id,
                at,
                new_id,
            });
            true
        } else {
            false
        }
    }
    fn merge_with_parent(&mut self, id: &ShortId) -> Option<ShortId> {
        match self.weave.merge_with_parent(id) {
            Some(new_id) => {
                self.actions
                    .push_back(TapestryWeaveAction::MergeWithParent(*id));
                Some(new_id)
            }
            None => None,
        }
    }
}
