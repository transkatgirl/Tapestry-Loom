// ! WIP, not ready for use

use std::{cmp::Ordering, collections::HashSet, time::Instant, vec};

use ulid::Ulid;

use crate::document::content::{ModificationRange, TimelineNodeRange};

use super::{
    Weave, WeaveView,
    content::{
        Diff, DiffContent, Modification, ModificationContent, Node, NodeContent, SnippetContent,
        TimelineUpdate,
    },
};

#[allow(unused_imports)]
use super::WeaveTimeline;

#[cfg(test)]
mod tests;

impl Weave {
    /// Update the Weave's contents based on a UTF-8 string and a timeline index.
    ///
    /// This calculates a [`Diff`] using the output of the selected [`WeaveTimeline`] and the user input, and then applies the diff as a series of updates to the Weave. The specific algorithm used to update the Weave is subject to change.
    ///
    /// If the selected timeline is not found, an empty timeline is created to build the diff against.
    ///
    /// If `add_diff_node` is `true`, modifications are added as [`NodeContent::Diff`] items whenever possible. If `add_diff_node` is `false`, modifications are added as updates to [`Node`] objects whenever possible.
    pub fn update(
        &mut self,
        timeline: usize,
        content: String,
        diff_deadline: Instant,
        mut add_diff_node: bool,
    ) {
        let mut timelines = self.get_active_timelines();

        let update = if timelines.len() > timeline {
            timelines
                .swap_remove(timeline)
                .build_update(content, diff_deadline)
        } else {
            TimelineUpdate {
                ranges: vec![],
                diff: Diff {
                    content: vec![Modification {
                        index: 0,
                        content: ModificationContent::Insertion(content.into_bytes()),
                    }],
                },
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
            self.perform_diff_update(update);
        } else {
            self.perform_graph_update(update);
        }
    }
    fn perform_diff_update(&mut self, mut update: TimelineUpdate) {
        let end = update
            .ranges
            .last()
            .map(|range| range.range.end)
            .unwrap_or_default();

        if update.diff.content.len() == 1 {
            let modification = update.diff.content.remove(0);

            if modification.index >= end {
                handle_singular_modification_diff_tail(self, &mut update.ranges, modification);
            } else {
                handle_singular_modification_diff_nontail(self, &mut update.ranges, modification);
            }
        }

        handle_multiple_modification_diff(self, &mut update.ranges, update.diff);
    }
    fn perform_graph_update(&mut self, mut update: TimelineUpdate) {
        for modification in update.diff.content {
            let end = update
                .ranges
                .last()
                .map(|range| range.range.end)
                .unwrap_or_default();

            if modification.index >= end {
                handle_graph_modification_tail(self, &mut update.ranges, modification);
            } else {
                handle_graph_modification_nontail(self, &mut update.ranges, modification);
            }
        }
    }
}

fn handle_singular_modification_diff_tail(
    weave: &mut Weave,
    ranges: &mut [TimelineNodeRange],
    modification: Modification,
) {
    let last_node = ranges.last().map(|range| range.node).unwrap_or_default();

    match modification.content {
        ModificationContent::Insertion(content) => {
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
                        content: NodeContent::Snippet(SnippetContent {
                            content,
                            model: None,
                            metadata: None,
                        }),
                    },
                    None,
                    false,
                    true,
                )
                .unwrap();
        }
        ModificationContent::Deletion(_length) => {
            let modification_range = modification.range();
            let selected_ranges = ranges.iter().rev().enumerate().filter(|(_, node_range)| {
                modification_range.contains(&node_range.range.start)
                    || modification_range.contains(&node_range.range.end)
            });

            for (_range_index, timeline_range) in selected_ranges {
                if let Some(identifier) = timeline_range.node {
                    if modification_range.contains(&timeline_range.range.start)
                        && modification_range.contains(&timeline_range.range.end)
                    {
                        weave.update_node_activity(&identifier, false, true);
                    } else if modification_range.contains(&timeline_range.range.end) {
                        if let Some((left, _right)) = weave.split_node(
                            &identifier,
                            timeline_range.range.end - modification_range.start,
                        ) {
                            weave.update_node_activity(&left, false, true);
                        } else {
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
                                                content: vec![modification],
                                            },
                                            model: None,
                                            metadata: None,
                                        }),
                                    },
                                    None,
                                    false,
                                    true,
                                )
                                .unwrap();
                            return;
                        }
                    } else {
                        panic!() // Should never happen
                    }
                } else {
                    panic!() // Should never happen
                }
            }
        }
    }
}

fn handle_singular_modification_diff_nontail(
    weave: &mut Weave,
    ranges: &mut [TimelineNodeRange],
    modification: Modification,
) {
    let last_node = ranges.last().map(|range| range.node).unwrap_or_default();

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
                    metadata: None,
                }),
            },
            None,
            false,
            true,
        )
        .unwrap();
}

fn handle_multiple_modification_diff(
    weave: &mut Weave,
    ranges: &mut [TimelineNodeRange],
    diff: Diff,
) {
    let last_node = ranges.last().map(|range| range.node).unwrap_or_default();

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
                    metadata: None,
                }),
            },
            None,
            false,
            true,
        )
        .unwrap();
}

fn handle_graph_modification_tail(
    weave: &mut Weave,
    ranges: &mut Vec<TimelineNodeRange>,
    modification: Modification,
) {
    let mut new_node = None;

    let last_node = ranges.last().map(|range| range.node).unwrap_or_default();

    let modification_range = ModificationRange::from(&modification);

    match modification.content {
        ModificationContent::Insertion(content) => {
            new_node = weave.add_node(
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
                        metadata: None,
                    }),
                },
                None,
                false,
                true,
            );
        }
        ModificationContent::Deletion(_length) => {
            let modification_range = modification.range();

            let selected_ranges = ranges.iter().enumerate().filter(|(_, node_range)| {
                modification_range.contains(&node_range.range.start)
                    || modification_range.contains(&node_range.range.end)
            });

            for (_range_index, timeline_range) in selected_ranges {
                if let Some(identifier) = timeline_range.node {
                    if modification_range.contains(&timeline_range.range.start)
                        && modification_range.contains(&timeline_range.range.end)
                    {
                        weave.update_node_activity(&identifier, false, true);
                    } else if modification_range.contains(&timeline_range.range.end) {
                        if let Some((left, _right)) = weave.split_node(
                            &identifier,
                            timeline_range.range.end - modification_range.start,
                        ) {
                            weave.update_node_activity(&left, false, true);
                        } else {
                            panic!() // Non-diff nodes should always be splitable
                        }
                    } else {
                        panic!() // Should never happen
                    }
                } else {
                    panic!() // Should never happen
                }
            }
        }
    }

    if let Some(mod_index) = modification_range.apply_annotations(ranges) {
        if new_node.is_some() {
            ranges[mod_index].node = new_node;
        }
    }
}

fn handle_graph_modification_nontail(
    weave: &mut Weave,
    ranges: &mut Vec<TimelineNodeRange>,
    modification: Modification,
) {
    let mut new_node = None;

    let modification_range = ModificationRange::from(&modification);

    match modification.content {
        ModificationContent::Insertion(content) => {
            todo!()
        }
        ModificationContent::Deletion(length) => {
            todo!()
        }
    }

    if let Some(mod_index) = modification_range.apply_annotations(&mut ranges) {
        if new_node.is_some() {
            ranges[mod_index].node = new_node;
        }
    }
}
