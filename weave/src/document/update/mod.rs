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
                handle_modification_tail(self, &mut update.ranges, modification);
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
                handle_modification_tail(self, &mut update.ranges, modification);
            } else {
                handle_graph_modification_nontail(self, &mut update.ranges, modification);
            }
        }
    }
}

// TODO: Handle insertion merging
fn handle_modification_tail(
    weave: &mut Weave,
    ranges: &mut Vec<TimelineNodeRange>,
    modification: Modification,
) {
    let mut insertion = None;
    let mut split = (None, None);

    let last_node = ranges.last().map(|range| range.node.unwrap());

    let update_modification = ModificationRange::from(&modification);

    match modification.content {
        ModificationContent::Insertion(content) => {
            insertion = Some(
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
                        true,
                    )
                    .unwrap(),
            );
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
                } else if modification_range.contains(&timeline_range.range.end) {
                    if let Some((left, right)) = weave.split_node(
                        &identifier,
                        timeline_range.range.end - modification_range.start,
                    ) {
                        split.0 = Some(left);
                        split.1 = Some(right);
                        weave.update_node_activity(&left, false, true);
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
                                            metadata: None,
                                        }),
                                    },
                                    None,
                                    true,
                                )
                                .unwrap(),
                        );
                    }
                } else {
                    panic!() // Should never happen
                }
            }
        }
    }

    let updates = update_modification.apply_annotations(ranges);

    if let Some(index) = updates.inserted {
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
    }
}

fn handle_singular_modification_diff_nontail(
    weave: &mut Weave,
    ranges: &mut [TimelineNodeRange],
    modification: Modification,
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
                    metadata: None,
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
                    metadata: None,
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
) {
    let mut insertion = None;
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

    let starting_node = match modification_range.start.cmp(&first_range.range.start) {
        Ordering::Equal => before_first.unwrap().node.unwrap(),
        Ordering::Greater => {
            panic!()
        }
        Ordering::Less => {
            let (left, right) = weave
                .split_node(&first_range.node.unwrap(), modification_range.start)
                .unwrap();
            split = (Some(left), Some(right));
            right
        }
    };
    let ending_node = match modification_range.end.cmp(&first_range.range.end) {
        Ordering::Equal => after_last.unwrap().node.unwrap(),
        Ordering::Greater => {
            let (left, right) = weave
                .split_node(&last_range.node.unwrap(), modification_range.end)
                .unwrap();
            split = (Some(left), Some(right));
            left
        }
        Ordering::Less => {
            panic!()
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
                            to: HashSet::from([ending_node]),
                            active: true,
                            bookmarked: false,
                            content: NodeContent::Snippet(SnippetContent {
                                content,
                                model: None,
                                metadata: None,
                            }),
                        },
                        None,
                        true,
                    )
                    .unwrap(),
            );
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

            weave.move_node(&ending_node, ending_node_parents);
        }
    }

    let updates = update_modification.apply_annotations(ranges);

    if let Some(index) = updates.inserted {
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
    }
}
