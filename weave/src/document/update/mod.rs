use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet, LinkedList, linked_list::CursorMut},
    ops::Range,
    vec,
};

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use similar::Instant;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::time::Instant;

use bytes::Bytes;
use ulid::Ulid;

use super::{
    Weave, WeaveView,
    content::{
        Diff, DiffContent, Modification, ModificationContent, ModificationRange, Node, NodeContent,
        NodeContents, SnippetContent, TimelineNodeLength, TimelineUpdate,
    },
};

#[allow(unused_imports)]
use super::{WeaveTimeline, content::Model};

#[cfg(test)]
mod tests;

// Note: Need to add special handling for removing text from beginning of root Node

impl Weave {
    /// Update the Weave's contents based on a UTF-8 string and a timeline index.
    ///
    /// This calculates a [`Diff`] using the output of the selected [`WeaveTimeline`] and the user input, and then applies the diff as a series of updates to the Weave. The specific algorithm used to update the Weave is subject to change.
    ///
    /// If no timelines are found, an empty timeline is created to build the diff against.
    ///
    /// If `add_diff_node` is `true`, modifications are added as [`NodeContent::Diff`] items whenever possible. If `add_diff_node` is `false`, modifications are added as updates to [`Node`] objects whenever possible.
    ///
    /// When inserting new nodes into the weave, `metadata` can be added to the inserted nodes. If `add_diff_node` is `true` and a modification to an existing node is being performed, the metadata will be combined with the node's existing metadata, if any.
    ///
    /// If `merge_tail_nodes` is `true`, modifications at the end of the timeline to nodes which do not contain an associated [`Model`] may be applied destructively. If `merge_tail_nodes` is `false`, modifications are always nondestructive (will not remove content from the Weave).
    pub fn update(
        &mut self,
        timeline: usize,
        content: String,
        metadata: Option<HashMap<String, String>>,
        diff_deadline: Instant,
        mut add_diff_node: bool,
        merge_tail_nodes: bool,
    ) {
        let mut timelines = self.get_active_timelines();

        let update = if timelines.len() > timeline {
            timelines
                .swap_remove(timeline)
                .build_update(content, metadata, diff_deadline)
        } else {
            assert!(timelines.len() == 0);
            TimelineUpdate {
                lengths: LinkedList::new(),
                total: 0,
                diff: Diff {
                    content: vec![Modification {
                        index: 0,
                        content: ModificationContent::Insertion(content.into()),
                    }],
                },
                metadata,
            }
        };

        self.reserve(update.diff.content.len(), 0);

        if !self.multiparent_nodes.is_empty() {
            add_diff_node = false;
        }
        if !self.nonconcatable_nodes.is_empty() {
            add_diff_node = true;
        }

        if add_diff_node {
            self.perform_diff_update(update, merge_tail_nodes);
        } else {
            self.perform_graph_update(update, merge_tail_nodes);
        }
    }
    fn perform_diff_update(&mut self, mut update: TimelineUpdate, merge_tail_nodes: bool) {
        /*let end = update.total;

        if update.diff.content.len() == 1 {
            let modification = update.diff.content.remove(0);

            if modification.index >= end {
                handle_modification_tail(
                    self,
                    &mut update.ranges,
                    modification,
                    update.metadata,
                    merge_tail_nodes,
                );
            } else {
                handle_singular_modification_diff_nontail(
                    self,
                    &mut update.ranges,
                    modification,
                    update.metadata,
                );
            }
        } else {
            handle_multiple_modification_diff(
                self,
                &mut update.ranges,
                update.diff,
                update.metadata,
            );
        }*/
    }
    fn perform_graph_update(&mut self, mut update: TimelineUpdate, merge_tail_nodes: bool) {
        /*

        for modification in update.diff.content {
            let end = update.total;

            if modification.index >= end {
                handle_modification_tail(
                    self,
                    &mut update.ranges,
                    modification,
                    update.metadata.clone(),
                    merge_tail_nodes,
                );
            } else {
                handle_graph_modification_nontail(
                    self,
                    &mut update.ranges,
                    modification,
                    update.metadata.clone(),
                );
            }
        }*/
    }
    // Trivial; shouldn't require unit tests
    fn remove_node_if_not_generated(&mut self, identifier: &Ulid) -> bool {
        let node = self.nodes.get(identifier).unwrap();

        if node.content.model().is_none() {
            self.remove_node(identifier);
            return true;
        }

        false
    }
    fn update_nongenerated_parent(
        &mut self,
        parent: &Ulid,
        content: Bytes,
        metadata: Option<HashMap<String, String>>,
    ) -> Option<Ulid> {
        let node = self.nodes.get_mut(parent).unwrap();

        if node.content.model().is_none() {
            if let Some(content) = NodeContent::merge(
                node.content.clone(),
                NodeContent::Snippet(SnippetContent {
                    content,
                    model: None,
                    metadata: node.content.metadata().cloned(),
                }),
            ) {
                node.content = content;
                if let Some(metadata) = metadata {
                    node.content.merge_metadata(metadata);
                }
                return Some(node.id);
            }
        }

        None
    }
    /// Add a [`Node`] (along with it's corresponding [`Model`]) to a specific byte range within a timeline.
    ///
    /// This takes a timeline index and a range of bytes within that timeline's output, and attempts to update the [`Node`] in order to insert it at that specific range. **This function will replace the child and parent nodes specified within the inserted node.**
    ///
    /// If the node's content is a [`NodeContent::Diff`], this function will always return [`None`].
    ///
    /// If the [`Weave`] is in nonconcatable mode or if `prefer_diff` is `true`, the [`NodeContent`] of the inserted node will be converted into a [`NodeContent::Diff`] and added at the end of the timeline. If the node's content is a [`NodeContent::Blank`], diff conversion will fail, causing this function to return [`None`].
    ///
    /// If the Weave is in multiparent mode, `prefer_diff` is ignored and the NodeContent is never converted.
    ///
    /// Once the Node (and Weave) has been updated, this adds the node at the specified position using [`Weave::add_node`]. If the specified range starts at the end of the timeline, the node's content will not be updated.
    #[allow(clippy::missing_panics_doc)]
    pub fn insert_at_range(
        &mut self,
        timeline: usize,
        range: Range<usize>,
        prefer_diff: bool,
        mut node: Node,
        model: Option<Model>,
        deduplicate: bool,
    ) -> Option<Ulid> {
        if let NodeContent::Diff(_) = &node.content {
            return None;
        }

        node.to = HashSet::new();
        node.from = HashSet::new();

        let mut timelines = self.get_active_timelines();

        let (timeline_content, ranges) = if timelines.len() > timeline {
            timelines.swap_remove(timeline).length_annotated_string()
        } else {
            return self.add_node(node, model, deduplicate);
        };

        if timeline_content.len() >= range.start {
            return self.add_node(node, model, deduplicate);
        }

        if !self.nonconcatable_nodes.is_empty()
            || (self.multiparent_nodes.is_empty() && prefer_diff)
        {
            if let Some(content) = node.content.into_diff(range) {
                node.content = content;
                node.from = ranges
                    .back()
                    .and_then(|annotation| annotation.node)
                    .map(|node| HashSet::from([node]))
                    .unwrap_or_default();

                return self.add_node(node, model, deduplicate);
            }

            return None;
        }

        let mut position = 0;
        let selected_ranges: Vec<(usize, &TimelineNodeLength)> = ranges
            .iter()
            .inspect(|annotation| position += annotation.len)
            .enumerate()
            .filter(|(_index, annotation)| {
                let annotation_range = Range {
                    start: position - annotation.len,
                    end: position,
                };

                range.contains(&annotation_range.start) || range.contains(&annotation_range.end)
            })
            .collect();

        let (first_range_index, first_range) = selected_ranges.first().unwrap();
        let before_first = if *first_range_index > 0 {
            Some(&ranges[first_range_index - 1])
        } else {
            None
        };

        let (starting_node, after_node) = match range.start.cmp(&first_range.range.start) {
            Ordering::Equal => (
                before_first.unwrap().node.unwrap(),
                first_range.node.unwrap(),
            ),
            Ordering::Greater => {
                let (left, right) = self
                    .split_node(&first_range.node.unwrap(), range.start)
                    .unwrap();
                (left, right)
            }
            Ordering::Less => {
                panic!() // Should never happen
            }
        };

        node.from = HashSet::from([starting_node]);
        node.to = HashSet::from([after_node]);

        self.add_node(node, model, deduplicate)
    }
}

#[allow(clippy::too_many_lines)]
fn handle_modification_tail(
    weave: &mut Weave,
    lengths_location: &mut usize,
    lengths_cursor: &mut CursorMut<'_, TimelineNodeRange>,
    modification: Modification,
    metadata: Option<HashMap<String, String>>,
    merge_tail_nodes: bool,
) {
    todo!();

    /*let mut insertion = None;
    let mut split = (None, None);

    let last_node = lengths_cursor.back().map(|range| range.node.unwrap());

    let update_modification = ModificationRange::from(&modification);

    match modification.content {
        ModificationContent::Insertion(content) => {
            insertion = Some(
                match last_node.filter(|_| merge_tail_nodes).and_then(|parent| {
                    weave.update_nongenerated_parent(&parent, content.clone(), metadata.clone())
                }) {
                    Some(updated) => updated,
                    None => weave
                        .add_node(
                            Node {
                                id: Ulid::new(),
                                from: last_node
                                    .map(|node| HashSet::from([node]))
                                    .unwrap_or_default(),
                                to: HashSet::new(),
                                active: true,
                                bookmarked: false,
                                content: NodeContent::Snippet(SnippetContent {
                                    content,
                                    model: None,
                                    metadata,
                                }),
                            },
                            None,
                            true,
                        )
                        .unwrap(),
                },
            );
        }
        ModificationContent::TokenInsertion(_) => {
            panic!() // Should never happen
        }
        ModificationContent::Deletion(length) => {
            let modification_range = modification.range();

            let selected_ranges = ranges.iter().rev().filter(|node_range| {
                modification_range.contains(&node_range.range.start)
                    || modification_range.contains(&node_range.range.end)
            });

            for timeline_range in selected_ranges {
                let identifier = timeline_range.node.unwrap();

                if modification_range.contains(&timeline_range.range.start)
                    && modification_range.contains(&timeline_range.range.end)
                {
                    weave.update_node_activity(&identifier, false, true);
                    if merge_tail_nodes {
                        weave.remove_node_if_not_generated(&identifier);
                    }
                } else if modification_range.contains(&timeline_range.range.end) {
                    if let Some((left, right)) =
                        weave.split_node(&identifier, modification_range.start)
                    {
                        split.0 = Some(left);
                        split.1 = Some(right);
                        weave.update_node_activity(&right, false, true);
                        if merge_tail_nodes && weave.remove_node_if_not_generated(&left) {
                            split.1 = None;
                        }
                    } else {
                        insertion = Some(
                            weave
                                .add_node(
                                    Node {
                                        id: Ulid::new(),
                                        from: HashSet::from([identifier]),
                                        to: HashSet::new(),
                                        active: true,
                                        bookmarked: false,
                                        content: NodeContent::Diff(DiffContent {
                                            content: Diff {
                                                content: vec![Modification {
                                                    index: modification.index,
                                                    content: ModificationContent::Deletion(length),
                                                }],
                                            },
                                            model: None,
                                            metadata,
                                        }),
                                    },
                                    None,
                                    true,
                                )
                                .unwrap(),
                        );
                        break;
                    }
                } else {
                    panic!() // Should never happen
                }
            }
        }
    }

    assert!(update_modification.apply_annotations(
        &mut lengths_cursor,
        &mut lengths_location,
        |annotation| {
            assert!(insertion.is_some());
            annotation.node = insertion;
        },
        |_, _| panic!(),
        |annotation| {
            if split.0.is_some() {
                annotation.node = split.0;
            }
        },
        |annotation| {
            if split.1.is_some() {
                annotation.node = split.1;
            }
        },
    ));*/
}

fn handle_singular_modification_diff_nontail(
    weave: &mut Weave,
    ranges: &mut [TimelineNodeRange],
    modification: Modification,
    metadata: Option<HashMap<String, String>>,
) {
    let last_node = ranges.last().map(|range| range.node.unwrap());

    weave
        .add_node(
            Node {
                id: Ulid::new(),
                from: last_node
                    .map(|node| HashSet::from([node]))
                    .unwrap_or_default(),
                to: HashSet::new(),
                active: true,
                bookmarked: false,
                content: NodeContent::Diff(DiffContent {
                    content: Diff {
                        content: vec![modification],
                    },
                    model: None,
                    metadata,
                }),
            },
            None,
            true,
        )
        .unwrap();
}

fn handle_multiple_modification_diff(
    weave: &mut Weave,
    ranges: &mut [TimelineNodeRange],
    diff: Diff,
    metadata: Option<HashMap<String, String>>,
) {
    let last_node = ranges.last().map(|range| range.node.unwrap());

    weave
        .add_node(
            Node {
                id: Ulid::new(),
                from: last_node
                    .map(|node| HashSet::from([node]))
                    .unwrap_or_default(),
                to: HashSet::new(),
                active: true,
                bookmarked: false,
                content: NodeContent::Diff(DiffContent {
                    content: diff,
                    model: None,
                    metadata,
                }),
            },
            None,
            true,
        )
        .unwrap();
}

#[allow(clippy::too_many_lines)]
fn handle_graph_modification_nontail(
    weave: &mut Weave,
    ranges: &mut Vec<TimelineNodeRange>,
    modification: Modification,
    metadata: Option<HashMap<String, String>>,
) {
    todo!();

    /*let mut insertion = None;
    #[allow(unused_assignments)]
    let mut split = (None, None);

    let update_modification = ModificationRange::from(&modification);

    let modification_range = modification.range();

    let selected_ranges: Vec<(usize, &TimelineNodeRange)> = ranges
        .iter()
        .enumerate()
        .filter(|(_index, node_range)| {
            modification_range.contains(&node_range.range.start)
                || modification_range.contains(&node_range.range.end)
        })
        .collect();

    let (first_range_index, first_range) = selected_ranges.first().unwrap();
    let before_first = if *first_range_index > 0 {
        Some(&ranges[first_range_index - 1])
    } else {
        None
    };
    let (last_range_index, last_range) = selected_ranges.last().unwrap();
    let after_last = if ranges.len() > (last_range_index + 1) {
        Some(&ranges[last_range_index + 1])
    } else {
        None
    };

    let (starting_node, after_node) = match modification_range.start.cmp(&first_range.range.start) {
        Ordering::Equal => (
            before_first.unwrap().node.unwrap(),
            first_range.node.unwrap(),
        ),
        Ordering::Greater => {
            let (left, right) = weave
                .split_node(&first_range.node.unwrap(), modification_range.start)
                .unwrap();
            (left, right)
        }
        Ordering::Less => {
            panic!() // Should never happen
        }
    };
    let ending_node = match modification_range.end.cmp(&last_range.range.end) {
        Ordering::Equal => after_last.unwrap().node.unwrap(),
        Ordering::Greater => {
            panic!() // Should never happen
        }
        Ordering::Less => {
            let (_left, right) = weave
                .split_node(&last_range.node.unwrap(), modification_range.end)
                .unwrap();
            right
        }
    };

    match modification.content {
        ModificationContent::Insertion(content) => {
            insertion = Some(
                weave
                    .add_node(
                        Node {
                            id: Ulid::new(),
                            from: HashSet::from([starting_node]),
                            to: HashSet::from([after_node]),
                            active: true,
                            bookmarked: false,
                            content: NodeContent::Snippet(SnippetContent {
                                content,
                                model: None,
                                metadata,
                            }),
                        },
                        None,
                        true,
                    )
                    .unwrap(),
            );
            split = (Some(starting_node), Some(after_node));
        }
        ModificationContent::TokenInsertion(_) => {
            panic!() // Should never happen
        }
        ModificationContent::Deletion(_length) => {
            let mut ending_node_parents = weave.nodes.get(&ending_node).unwrap().from.clone();
            let selected_nodes = selected_ranges
                .iter()
                .rev()
                .filter_map(|(_index, node_range)| node_range.node);
            for selected_node in selected_nodes {
                ending_node_parents.remove(&selected_node);
            }
            ending_node_parents.insert(starting_node);

            assert!(weave.move_node(&ending_node, ending_node_parents));
            split = (Some(starting_node), Some(ending_node));
        }
    }

    let updates = update_modification.apply_annotations(ranges);

    if let Some(index) = updates.inserted_bytes {
        if insertion.is_some() {
            ranges[index].node = insertion;
        }
    }
    if let Some(index) = updates.left_split {
        if split.0.is_some() {
            ranges[index].node = split.0;
        }
    }
    if let Some(index) = updates.right_split {
        if split.1.is_some() {
            ranges[index].node = split.1;
        }
    }*/
}
