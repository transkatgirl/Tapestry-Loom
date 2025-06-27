// ! WIP

use std::{cmp::Ordering, collections::HashSet, time::Instant, vec};

use ulid::Ulid;

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
    /// This calculates a [`Diff`] using the output of the selected [`WeaveTimeline`] and the user input, and then applies the diff to the graph. The specific algorithm used to update the graph is subject to change.
    ///
    /// If the selected timeline is not found, an empty timeline is created to build the diff against.
    ///
    /// If `add_diff_node` is `true`, modifications are added as [`NodeContent::Diff`] items whenever possible. If `add_diff_node` is `false`, modifications are added as graph updates whenever possible.
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
    #[allow(clippy::too_many_lines)]
    fn perform_diff_update(&mut self, mut update: TimelineUpdate) {
        let (last_node, mut end) = update
            .ranges
            .last()
            .map(|range| (range.node, range.range.end))
            .unwrap_or_default();

        if update.diff.content.len() == 1 {
            let modification = update.diff.content.remove(0);

            let content = match modification.content {
                ModificationContent::Insertion(content) => {
                    if update.diff.content[0].index >= end {
                        NodeContent::Snippet(SnippetContent {
                            content,
                            model: None,
                            metadata: None,
                        })
                    } else {
                        NodeContent::Diff(DiffContent {
                            content: Diff {
                                content: vec![Modification {
                                    index: modification.index,
                                    content: ModificationContent::Insertion(content),
                                }],
                            },
                            model: None,
                            metadata: None,
                        })
                    }
                }
                ModificationContent::Deletion(length) => {
                    let modification_range = modification.range();

                    if modification_range.end >= end {
                        let selected_ranges =
                            update
                                .ranges
                                .iter()
                                .rev()
                                .enumerate()
                                .filter(|(_, node_range)| {
                                    modification_range.contains(&node_range.range.start)
                                        || modification_range.contains(&node_range.range.end)
                                });

                        for (_range_index, timeline_range) in selected_ranges {
                            if let Some(identifier) = timeline_range.node {
                                if modification_range.contains(&timeline_range.range.start)
                                    && modification_range.contains(&timeline_range.range.end)
                                {
                                    self.update_node_activity(&identifier, false, true);
                                } else if modification_range.contains(&timeline_range.range.start) {
                                    if let Some((_left, right)) = self.split_node(
                                        &identifier,
                                        timeline_range.range.start - modification_range.end,
                                    ) {
                                        self.update_node_activity(&right, false, true);
                                    } else {
                                        self.add_node(
                                            Node {
                                                id: Ulid::new(),
                                                from: HashSet::from([identifier]),
                                                to: HashSet::new(),
                                                active: true,
                                                bookmarked: false,
                                                content: NodeContent::Diff(DiffContent {
                                                    content: Diff {
                                                        content: vec![Modification {
                                                            index: modification_range.start,
                                                            content: ModificationContent::Deletion(
                                                                end - modification_range.start,
                                                            ),
                                                        }],
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
                                end = modification_range.end;
                            } else {
                                panic!() // Should never happen
                            }
                        }
                    }

                    NodeContent::Diff(DiffContent {
                        content: Diff {
                            content: vec![Modification {
                                index: modification.index,
                                content: ModificationContent::Deletion(length),
                            }],
                        },
                        model: None,
                        metadata: None,
                    })
                }
            };

            self.add_node(
                Node {
                    id: Ulid::new(),
                    from: last_node
                        .map(|node| HashSet::from([node]))
                        .unwrap_or_default(),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content,
                },
                None,
                false,
                true,
            )
            .unwrap();
            return;
        }

        self.add_node(
            Node {
                id: Ulid::new(),
                from: last_node
                    .map(|node| HashSet::from([node]))
                    .unwrap_or_default(),
                to: HashSet::new(),
                active: true,
                bookmarked: false,
                content: NodeContent::Diff(DiffContent {
                    content: update.diff,
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
    // TODO
    #[allow(clippy::too_many_lines)]
    fn perform_graph_update(&mut self, mut update: TimelineUpdate) {
        for modification in update.diff.content {
            let modification_range = modification.range();

            let selected_ranges = update.ranges.iter().enumerate().filter(|(_, node_range)| {
                modification_range.contains(&node_range.range.start)
                    || modification_range.contains(&node_range.range.end)
            });

            let (last_node, end) = update
                .ranges
                .last()
                .map(|range| (range.node, range.range.end))
                .unwrap_or_default();

            let mut new_node = None;

            match &modification.content {
                ModificationContent::Insertion(content) => {
                    if modification_range.start >= end {
                        new_node = self.add_node(
                            Node {
                                id: Ulid::new(),
                                from: last_node
                                    .map(|node| HashSet::from([node]))
                                    .unwrap_or_default(),
                                to: HashSet::new(),
                                active: true,
                                bookmarked: false,
                                content: NodeContent::Snippet(SnippetContent {
                                    content: content.clone(),
                                    model: None,
                                    metadata: None,
                                }),
                            },
                            None,
                            false,
                            true,
                        );
                    } else {
                        let selected_ranges: Vec<_> = selected_ranges.collect();

                        if let Some((first_selected_index, first_selected_range)) =
                            selected_ranges.first()
                        {
                            match first_selected_range
                                .range
                                .start
                                .cmp(&modification_range.start)
                            {
                                Ordering::Equal => {
                                    let from_node = first_selected_range.node;
                                    let to_node = &update
                                        .ranges
                                        .split_at(*first_selected_index)
                                        .1
                                        .iter()
                                        .find_map(|range| {
                                            if range.node == from_node {
                                                None
                                            } else {
                                                range.node
                                            }
                                        });

                                    new_node = self.add_node(
                                        Node {
                                            id: Ulid::new(),
                                            from: from_node
                                                .map(|node| HashSet::from([node]))
                                                .unwrap_or_default(),
                                            to: to_node
                                                .map(|node| HashSet::from([node]))
                                                .unwrap_or_default(),
                                            active: true,
                                            bookmarked: false,
                                            content: NodeContent::Snippet(SnippetContent {
                                                content: content.clone(),
                                                model: None,
                                                metadata: None,
                                            }),
                                        },
                                        None,
                                        false,
                                        true,
                                    );
                                    if let (Some(from_node), Some(to_node)) = (from_node, to_node) {
                                        self.remove_node_parent(to_node, &from_node);
                                    }
                                }
                                Ordering::Greater => {
                                    todo!()
                                }
                                Ordering::Less => {
                                    todo!()
                                }
                            }
                        }
                    }
                }
                ModificationContent::Deletion(_length) => {
                    if modification_range.end >= end {
                        for (_range_index, timeline_range) in selected_ranges {
                            if let Some(identifier) = timeline_range.node {
                                if modification_range.contains(&timeline_range.range.start)
                                    && modification_range.contains(&timeline_range.range.end)
                                {
                                    self.update_node_activity(&identifier, false, true);
                                } else if modification_range.contains(&timeline_range.range.start) {
                                    if let Some((_left, right)) = self.split_node(
                                        &identifier,
                                        timeline_range.range.start - modification_range.end,
                                    ) {
                                        self.update_node_activity(&right, false, true);
                                    } else {
                                        panic!() // Non-diff nodes should always be splitable
                                    }
                                } else if let Some((left, _right)) = self.split_node(
                                    &identifier,
                                    timeline_range.range.end - modification_range.start,
                                ) {
                                    self.update_node_activity(&left, false, true);
                                } else {
                                    panic!() // Non-diff nodes should always be splitable
                                }
                            } else {
                                panic!() // Should never happen
                            }
                        }
                    } else {
                        let selected_ranges: Vec<_> = selected_ranges.collect();

                        if let Some((first_selected_index, first_selected_range)) =
                            selected_ranges.first()
                        {
                            match first_selected_range
                                .range
                                .start
                                .cmp(&modification_range.start)
                            {
                                Ordering::Equal => {
                                    todo!()
                                }
                                Ordering::Greater => {
                                    todo!()
                                }
                                Ordering::Less => {
                                    todo!()
                                }
                            }
                        }

                        for (range_index, timeline_range) in selected_ranges {}

                        todo!()
                    }
                }
            }

            if let Some(mod_index) = modification.apply_annotations(&mut update.ranges) {
                if new_node.is_some() {
                    update.ranges[mod_index].node = new_node;
                }
            }
        }
    }
}
