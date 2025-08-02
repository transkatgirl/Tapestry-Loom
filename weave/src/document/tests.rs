#![allow(clippy::should_panic_without_expect)]
#![allow(clippy::too_many_lines)]

use bytes::Bytes;

use crate::document::content::{ContentModel, DiffContent, SnippetContent};

use super::*;

/*
Checklist:
- [ ] ensure node connection consistency
    - [ ] deduplicate_node
    - [x] add_node
    - [ ] move_node
    - [ ] split_node
    - [ ] merge_nodes
    - [x] remove_node
- [ ] ensure active status propagation
    - [ ] deduplicate_node
    - [x] add_node
    - [ ] move_node
    - [ ] split_node
    - [ ] merge_nodes
    - [x] remove_node
- [ ] ensure node object identifier consistency
    - [ ] deduplicate_node
    - [x] add_node
    - [ ] move_node
    - [ ] split_node
    - [ ] merge_nodes
- [ ] ensure weave node identifier consistency
    - [ ] deduplicate_node
    - [ ] move_node
    - [ ] split_node
    - [ ] merge_nodes
    - [x] remove_node
- [ ] if node.from.len() == 0, add to root nodes
    - [x] add_node
    - [ ] move_node
    - [ ] split_node
    - [ ] merge_nodes
- [ ] if node.from.len() > 0, remove from root nodes
    - [x] add_node
    - [ ] move_node
    - [ ] split_node
    - [ ] merge_nodes
    - [x] remove_node
- [ ] if node.from.len() > 1, add to multiparent nodes
    - [x] add_node
    - [ ] move_node
    - [ ] split_node
    - [ ] merge_nodes
- [ ] if node.from.len() <= 1, remove from multiparent nodes
    - [ ] move_node
    - [ ] split_node
    - [ ] merge_nodes
    - [x] remove_node
- [ ] if node.content.model, add to model nodes
    - [x] add_node
    - [ ] move_node
    - [ ] split_node
    - [ ] merge_nodes
- [ ] if !node.content.model, remove from model nodes
    - [ ] move_node
    - [ ] split_node
    - [ ] merge_nodes
    - [ ] remove_node
- [ ] ensure model object identifier consistency
    - [x] add_node
    - [ ] move_node
    - [ ] split_node
    - [ ] merge_nodes
    - [ ] remove_node
- [ ] ensure weave model identifier consistency
    - [x] add_node
    - [ ] move_node
    - [ ] split_node
    - [ ] merge_nodes
    - [ ] remove_node
- [ ] if bookmarked, add to bookmarked nodes
    - [x] add_node
    - [ ] move_node
    - [ ] split_node
    - [ ] merge_nodes
- [ ] if !bookmarked, remove from bookmarked nodes
    - [ ] move_node
    - [ ] split_node
    - [ ] merge_nodes
    - [x] remove_node
- [ ] if nonconcatable, add to nonconcatable nodes
    - [x] add_node
    - [ ] move_node
    - [ ] split_node
    - [ ] merge_nodes
- [ ] if concatable, remove from nonconcatable nodes
    - [ ] move_node
    - [ ] split_node
    - [ ] merge_nodes
    - [x] remove_node
- [ ] prevent mixing of nonconcatable and multiparent nodes
    - [x] add_node
    - [ ] move_node
    - [ ] split_node
    - [ ] merge_nodes
- [ ] ensure error states do not break consistency
    - [ ] deduplicate_node
    - [x] add_node
    - [ ] move_node
    - [ ] split_node
    - [ ] merge_nodes
    - [x] remove_node
- [ ] ensure behavior lines up with documentation
    - [ ] deduplicate_node
    - [x] add_node
    - [ ] move_node
    - [ ] split_node
    - [ ] merge_nodes
    - [ ] remove_node
*/

// Need to update move_node and split_node documentation with node identifier guarantees

#[test]
#[allow(clippy::bool_assert_comparison)]
fn update_node_activity_tree() {
    let mut nodes = HashMap::from([
        (
            Ulid::from_parts(0, 0),
            Node {
                id: Ulid::from_parts(0, 0),
                from: HashSet::new(),
                to: HashSet::from([Ulid::from_parts(0, 1), Ulid::from_parts(0, 2)]),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
        ),
        (
            Ulid::from_parts(0, 1),
            Node {
                id: Ulid::from_parts(0, 1),
                from: HashSet::from([Ulid::from_parts(0, 0)]),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
        ),
        (
            Ulid::from_parts(0, 2),
            Node {
                id: Ulid::from_parts(0, 2),
                from: HashSet::from([Ulid::from_parts(0, 0)]),
                to: HashSet::from([Ulid::from_parts(0, 3)]),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
        ),
        (
            Ulid::from_parts(0, 3),
            Node {
                id: Ulid::from_parts(0, 3),
                from: HashSet::from([Ulid::from_parts(0, 2)]),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
        ),
        (
            Ulid::from_parts(1, 0),
            Node {
                id: Ulid::from_parts(1, 0),
                from: HashSet::new(),
                to: HashSet::from([Ulid::from_parts(1, 2), Ulid::from_parts(1, 3)]),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
        ),
        (
            Ulid::from_parts(1, 1),
            Node {
                id: Ulid::from_parts(1, 1),
                from: HashSet::new(),
                to: HashSet::from([Ulid::from_parts(1, 3), Ulid::from_parts(1, 4)]),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
        ),
        (
            Ulid::from_parts(1, 2),
            Node {
                id: Ulid::from_parts(1, 2),
                from: HashSet::from([Ulid::from_parts(1, 0)]),
                to: HashSet::from([
                    Ulid::from_parts(1, 5),
                    Ulid::from_parts(1, 6),
                    Ulid::from_parts(1, 7),
                ]),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
        ),
        (
            Ulid::from_parts(1, 3),
            Node {
                id: Ulid::from_parts(1, 3),
                from: HashSet::from([Ulid::from_parts(1, 0), Ulid::from_parts(1, 1)]),
                to: HashSet::from([Ulid::from_parts(1, 7), Ulid::from_parts(1, 10)]),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
        ),
        (
            Ulid::from_parts(1, 4),
            Node {
                id: Ulid::from_parts(1, 4),
                from: HashSet::from([Ulid::from_parts(1, 1)]),
                to: HashSet::from([Ulid::from_parts(1, 8), Ulid::from_parts(1, 9)]),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
        ),
        (
            Ulid::from_parts(1, 5),
            Node {
                id: Ulid::from_parts(1, 5),
                from: HashSet::from([Ulid::from_parts(1, 2)]),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
        ),
        (
            Ulid::from_parts(1, 6),
            Node {
                id: Ulid::from_parts(1, 6),
                from: HashSet::from([Ulid::from_parts(1, 2)]),
                to: HashSet::from([Ulid::from_parts(1, 10)]),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
        ),
        (
            Ulid::from_parts(1, 7),
            Node {
                id: Ulid::from_parts(1, 7),
                from: HashSet::from([Ulid::from_parts(1, 2), Ulid::from_parts(1, 3)]),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
        ),
        (
            Ulid::from_parts(1, 8),
            Node {
                id: Ulid::from_parts(1, 8),
                from: HashSet::from([Ulid::from_parts(1, 4)]),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
        ),
        (
            Ulid::from_parts(1, 9),
            Node {
                id: Ulid::from_parts(1, 9),
                from: HashSet::from([Ulid::from_parts(1, 4)]),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
        ),
        (
            Ulid::from_parts(1, 10),
            Node {
                id: Ulid::from_parts(1, 10),
                from: HashSet::from([Ulid::from_parts(1, 6), Ulid::from_parts(1, 3)]),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
        ),
    ]);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, false);
    update_node_activity(&mut nodes, &Ulid::from_parts(0, 0), true, false, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, false);
    update_node_activity(&mut nodes, &Ulid::from_parts(0, 0), true, false, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, false);
    update_node_activity(&mut nodes, &Ulid::from_parts(0, 1), true, false, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, false);
    update_node_activity(&mut nodes, &Ulid::from_parts(0, 1), false, false, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, false);
    update_node_activity(&mut nodes, &Ulid::from_parts(0, 0), true, false, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, false);
    update_node_activity(&mut nodes, &Ulid::from_parts(0, 0), false, false, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, false);
    update_node_activity(&mut nodes, &Ulid::from_parts(0, 0), true, true, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, false);
    update_node_activity(&mut nodes, &Ulid::from_parts(0, 0), true, true, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, false);
    update_node_activity(&mut nodes, &Ulid::from_parts(0, 1), true, true, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, false);
    update_node_activity(&mut nodes, &Ulid::from_parts(0, 1), false, true, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, false);
    update_node_activity(&mut nodes, &Ulid::from_parts(0, 0), false, true, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, false);
    update_node_activity(&mut nodes, &Ulid::from_parts(1, 7), true, false, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, false);
    update_node_activity(&mut nodes, &Ulid::from_parts(1, 5), true, false, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, false);
    update_node_activity(&mut nodes, &Ulid::from_parts(1, 6), true, true, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, false);
    update_node_activity(&mut nodes, &Ulid::from_parts(1, 10), true, true, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, true);
    update_node_activity(&mut nodes, &Ulid::from_parts(1, 10), false, false, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, false);
    update_node_activity(&mut nodes, &Ulid::from_parts(1, 10), true, true, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, true);
    update_node_activity(&mut nodes, &Ulid::from_parts(1, 11), false, false, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, true);
    update_node_activity(&mut nodes, &Ulid::from_parts(1, 6), true, true, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, true);
    update_node_activity(&mut nodes, &Ulid::from_parts(1, 6), false, true, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, false);
    update_node_activity(&mut nodes, &Ulid::from_parts(1, 10), true, false, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, true);
    update_node_activity(&mut nodes, &Ulid::from_parts(0, 3), true, false, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, true);
    update_node_activity(&mut nodes, &Ulid::from_parts(1, 0), false, false, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, false);
    nodes
        .get_mut(&Ulid::from_parts(1, 7))
        .unwrap()
        .to
        .insert(Ulid::from_parts(1, 10));
    nodes
        .get_mut(&Ulid::from_parts(1, 10))
        .unwrap()
        .from
        .insert(Ulid::from_parts(1, 7));
    update_node_activity(&mut nodes, &Ulid::from_parts(1, 7), true, true, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, false);
    update_node_activity(&mut nodes, &Ulid::from_parts(1, 10), true, true, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, true);
    nodes.get_mut(&Ulid::from_parts(1, 7)).unwrap().active = false;
    update_node_activity(&mut nodes, &Ulid::from_parts(1, 7), true, true, None);
    assert_eq!(nodes.len(), 15);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 2)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(0, 3)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 0)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 1)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 2)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 3)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 4)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 5)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 6)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 7)).unwrap().active, true);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 8)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 9)).unwrap().active, false);
    assert_eq!(nodes.get(&Ulid::from_parts(1, 10)).unwrap().active, true);
}

#[test]
fn add_node() {
    let mut weave = Weave::default();
    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(0, 0),
                from: HashSet::new(),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            true,
        ),
        Some(Ulid::from_parts(0, 0))
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([(
            Ulid::from_parts(0, 0),
            Node {
                id: Ulid::from_parts(0, 0),
                from: HashSet::new(),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            }
        )])
    );
    assert!(weave.models.is_empty());
    assert!(weave.model_nodes.is_empty());
    assert!(weave.bookmarked_nodes.is_empty());
    assert!(weave.multiparent_nodes.is_empty());
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(weave.root_nodes, HashSet::from([Ulid::from_parts(0, 0)]));

    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(0, 1),
                from: HashSet::from([Ulid::from_parts(0, 0)]),
                to: HashSet::new(),
                active: true,
                bookmarked: true,
                content: NodeContent::Blank,
            },
            None,
            true,
        ),
        Some(Ulid::from_parts(0, 1))
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(0, 1)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 1),
                Node {
                    id: Ulid::from_parts(0, 1),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: true,
                    content: NodeContent::Blank,
                }
            )
        ])
    );
    assert!(weave.models.is_empty());
    assert!(weave.model_nodes.is_empty());
    assert!(weave.multiparent_nodes.is_empty());
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(weave.root_nodes, HashSet::from([Ulid::from_parts(0, 0)]));
    assert_eq!(
        weave.bookmarked_nodes,
        HashSet::from([Ulid::from_parts(0, 1)])
    );

    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(0, 2),
                from: HashSet::new(),
                to: HashSet::new(),
                active: true,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            true,
        ),
        Some(Ulid::from_parts(0, 2))
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(0, 1)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 1),
                Node {
                    id: Ulid::from_parts(0, 1),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: true,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 2),
                Node {
                    id: Ulid::from_parts(0, 2),
                    from: HashSet::new(),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
        ])
    );
    assert!(weave.models.is_empty());
    assert!(weave.model_nodes.is_empty());
    assert!(weave.multiparent_nodes.is_empty());
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(
        weave.root_nodes,
        HashSet::from([Ulid::from_parts(0, 0), Ulid::from_parts(0, 2)])
    );
    assert_eq!(
        weave.bookmarked_nodes,
        HashSet::from([Ulid::from_parts(0, 1)])
    );
    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(0, 2),
                from: HashSet::new(),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            true,
        ),
        None
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(0, 1)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 1),
                Node {
                    id: Ulid::from_parts(0, 1),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: true,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 2),
                Node {
                    id: Ulid::from_parts(0, 2),
                    from: HashSet::new(),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
        ])
    );
    assert!(weave.models.is_empty());
    assert!(weave.model_nodes.is_empty());
    assert!(weave.multiparent_nodes.is_empty());
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(
        weave.root_nodes,
        HashSet::from([Ulid::from_parts(0, 0), Ulid::from_parts(0, 2)])
    );
    assert_eq!(
        weave.bookmarked_nodes,
        HashSet::from([Ulid::from_parts(0, 1)])
    );
    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(0, 3),
                from: HashSet::from([Ulid::from_parts(0, 0)]),
                to: HashSet::from([Ulid::from_parts(0, 2)]),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            true,
        ),
        Some(Ulid::from_parts(0, 3))
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(0, 1), Ulid::from_parts(0, 3)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 1),
                Node {
                    id: Ulid::from_parts(0, 1),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: true,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 2),
                Node {
                    id: Ulid::from_parts(0, 2),
                    from: HashSet::from([Ulid::from_parts(0, 3)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 3),
                Node {
                    id: Ulid::from_parts(0, 3),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 2)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
        ])
    );
    assert!(weave.models.is_empty());
    assert!(weave.model_nodes.is_empty());
    assert!(weave.multiparent_nodes.is_empty());
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(weave.root_nodes, HashSet::from([Ulid::from_parts(0, 0)]));
    assert_eq!(
        weave.bookmarked_nodes,
        HashSet::from([Ulid::from_parts(0, 1)])
    );
    weave.nodes.get_mut(&Ulid::from_parts(0, 1)).unwrap().active = true;

    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(0, 4),
                from: HashSet::from([Ulid::from_parts(0, 0)]),
                to: HashSet::from([Ulid::from_parts(0, 2)]),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            true,
        ),
        Some(Ulid::from_parts(0, 4))
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::from([
                        Ulid::from_parts(0, 1),
                        Ulid::from_parts(0, 3),
                        Ulid::from_parts(0, 4)
                    ]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 1),
                Node {
                    id: Ulid::from_parts(0, 1),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: true,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 2),
                Node {
                    id: Ulid::from_parts(0, 2),
                    from: HashSet::from([Ulid::from_parts(0, 3), Ulid::from_parts(0, 4)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 3),
                Node {
                    id: Ulid::from_parts(0, 3),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 2)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 4),
                Node {
                    id: Ulid::from_parts(0, 4),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 2)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
        ])
    );
    assert!(weave.models.is_empty());
    assert!(weave.model_nodes.is_empty());
    assert_eq!(
        weave.multiparent_nodes,
        HashSet::from([Ulid::from_parts(0, 2)])
    );
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(weave.root_nodes, HashSet::from([Ulid::from_parts(0, 0)]));
    assert_eq!(
        weave.bookmarked_nodes,
        HashSet::from([Ulid::from_parts(0, 1)])
    );

    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(0, 5),
                from: HashSet::from([Ulid::from_parts(0, 0), Ulid::from_parts(0, 1)]),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            true,
        ),
        Some(Ulid::from_parts(0, 5))
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::from([
                        Ulid::from_parts(0, 1),
                        Ulid::from_parts(0, 3),
                        Ulid::from_parts(0, 4),
                        Ulid::from_parts(0, 5)
                    ]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 1),
                Node {
                    id: Ulid::from_parts(0, 1),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 5)]),
                    active: true,
                    bookmarked: true,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 2),
                Node {
                    id: Ulid::from_parts(0, 2),
                    from: HashSet::from([Ulid::from_parts(0, 3), Ulid::from_parts(0, 4)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 3),
                Node {
                    id: Ulid::from_parts(0, 3),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 2)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 4),
                Node {
                    id: Ulid::from_parts(0, 4),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 2)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 5),
                Node {
                    id: Ulid::from_parts(0, 5),
                    from: HashSet::from([Ulid::from_parts(0, 0), Ulid::from_parts(0, 1)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
        ])
    );
    assert!(weave.models.is_empty());
    assert!(weave.model_nodes.is_empty());
    assert_eq!(
        weave.multiparent_nodes,
        HashSet::from([Ulid::from_parts(0, 2), Ulid::from_parts(0, 5)])
    );
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(weave.root_nodes, HashSet::from([Ulid::from_parts(0, 0)]));
    assert_eq!(
        weave.bookmarked_nodes,
        HashSet::from([Ulid::from_parts(0, 1)])
    );

    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(0, 6),
                from: HashSet::new(),
                to: HashSet::new(),
                active: true,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            true,
        ),
        Some(Ulid::from_parts(0, 6))
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::from([
                        Ulid::from_parts(0, 1),
                        Ulid::from_parts(0, 3),
                        Ulid::from_parts(0, 4),
                        Ulid::from_parts(0, 5)
                    ]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 1),
                Node {
                    id: Ulid::from_parts(0, 1),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 5)]),
                    active: true,
                    bookmarked: true,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 2),
                Node {
                    id: Ulid::from_parts(0, 2),
                    from: HashSet::from([Ulid::from_parts(0, 3), Ulid::from_parts(0, 4)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 3),
                Node {
                    id: Ulid::from_parts(0, 3),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 2)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 4),
                Node {
                    id: Ulid::from_parts(0, 4),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 2)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 5),
                Node {
                    id: Ulid::from_parts(0, 5),
                    from: HashSet::from([Ulid::from_parts(0, 0), Ulid::from_parts(0, 1)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 6),
                Node {
                    id: Ulid::from_parts(0, 6),
                    from: HashSet::new(),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
        ])
    );
    assert!(weave.models.is_empty());
    assert!(weave.model_nodes.is_empty());
    assert_eq!(
        weave.multiparent_nodes,
        HashSet::from([Ulid::from_parts(0, 2), Ulid::from_parts(0, 5)])
    );
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(
        weave.root_nodes,
        HashSet::from([Ulid::from_parts(0, 0), Ulid::from_parts(0, 6)])
    );
    assert_eq!(
        weave.bookmarked_nodes,
        HashSet::from([Ulid::from_parts(0, 1)])
    );

    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(0, 7),
                from: HashSet::from([Ulid::from_parts(0, 5)]),
                to: HashSet::from([Ulid::from_parts(0, 6)]),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            false,
        ),
        Some(Ulid::from_parts(0, 7))
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::from([
                        Ulid::from_parts(0, 1),
                        Ulid::from_parts(0, 3),
                        Ulid::from_parts(0, 4),
                        Ulid::from_parts(0, 5)
                    ]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 1),
                Node {
                    id: Ulid::from_parts(0, 1),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 5)]),
                    active: true,
                    bookmarked: true,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 2),
                Node {
                    id: Ulid::from_parts(0, 2),
                    from: HashSet::from([Ulid::from_parts(0, 3), Ulid::from_parts(0, 4)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 3),
                Node {
                    id: Ulid::from_parts(0, 3),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 2)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 4),
                Node {
                    id: Ulid::from_parts(0, 4),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 2)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 5),
                Node {
                    id: Ulid::from_parts(0, 5),
                    from: HashSet::from([Ulid::from_parts(0, 0), Ulid::from_parts(0, 1)]),
                    to: HashSet::from([Ulid::from_parts(0, 7)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 6),
                Node {
                    id: Ulid::from_parts(0, 6),
                    from: HashSet::from([Ulid::from_parts(0, 7)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 7),
                Node {
                    id: Ulid::from_parts(0, 7),
                    from: HashSet::from([Ulid::from_parts(0, 5)]),
                    to: HashSet::from([Ulid::from_parts(0, 6)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
        ])
    );
    assert!(weave.models.is_empty());
    assert!(weave.model_nodes.is_empty());
    assert_eq!(
        weave.multiparent_nodes,
        HashSet::from([Ulid::from_parts(0, 2), Ulid::from_parts(0, 5)])
    );
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(weave.root_nodes, HashSet::from([Ulid::from_parts(0, 0)]));
    assert_eq!(
        weave.bookmarked_nodes,
        HashSet::from([Ulid::from_parts(0, 1)])
    );

    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(0, 8),
                from: HashSet::from([Ulid::from_parts(0, 0)]),
                to: HashSet::new(),
                active: true,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            true,
        ),
        Some(Ulid::from_parts(0, 8))
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::from([
                        Ulid::from_parts(0, 1),
                        Ulid::from_parts(0, 3),
                        Ulid::from_parts(0, 4),
                        Ulid::from_parts(0, 5),
                        Ulid::from_parts(0, 8)
                    ]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 1),
                Node {
                    id: Ulid::from_parts(0, 1),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 5)]),
                    active: false,
                    bookmarked: true,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 2),
                Node {
                    id: Ulid::from_parts(0, 2),
                    from: HashSet::from([Ulid::from_parts(0, 3), Ulid::from_parts(0, 4)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 3),
                Node {
                    id: Ulid::from_parts(0, 3),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 2)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 4),
                Node {
                    id: Ulid::from_parts(0, 4),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 2)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 5),
                Node {
                    id: Ulid::from_parts(0, 5),
                    from: HashSet::from([Ulid::from_parts(0, 0), Ulid::from_parts(0, 1)]),
                    to: HashSet::from([Ulid::from_parts(0, 7)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 6),
                Node {
                    id: Ulid::from_parts(0, 6),
                    from: HashSet::from([Ulid::from_parts(0, 7)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 7),
                Node {
                    id: Ulid::from_parts(0, 7),
                    from: HashSet::from([Ulid::from_parts(0, 5)]),
                    to: HashSet::from([Ulid::from_parts(0, 6)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 8),
                Node {
                    id: Ulid::from_parts(0, 8),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
        ])
    );
    assert!(weave.models.is_empty());
    assert!(weave.model_nodes.is_empty());
    assert_eq!(
        weave.multiparent_nodes,
        HashSet::from([Ulid::from_parts(0, 2), Ulid::from_parts(0, 5)])
    );
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(weave.root_nodes, HashSet::from([Ulid::from_parts(0, 0)]));
    assert_eq!(
        weave.bookmarked_nodes,
        HashSet::from([Ulid::from_parts(0, 1)])
    );

    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(0, 9),
                from: HashSet::from([Ulid::from_parts(0, 0)]),
                to: HashSet::new(),
                active: true,
                bookmarked: false,
                content: NodeContent::Diff(DiffContent::default()),
            },
            None,
            false,
        ),
        None
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::from([
                        Ulid::from_parts(0, 1),
                        Ulid::from_parts(0, 3),
                        Ulid::from_parts(0, 4),
                        Ulid::from_parts(0, 5),
                        Ulid::from_parts(0, 8)
                    ]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 1),
                Node {
                    id: Ulid::from_parts(0, 1),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 5)]),
                    active: false,
                    bookmarked: true,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 2),
                Node {
                    id: Ulid::from_parts(0, 2),
                    from: HashSet::from([Ulid::from_parts(0, 3), Ulid::from_parts(0, 4)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 3),
                Node {
                    id: Ulid::from_parts(0, 3),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 2)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 4),
                Node {
                    id: Ulid::from_parts(0, 4),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 2)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 5),
                Node {
                    id: Ulid::from_parts(0, 5),
                    from: HashSet::from([Ulid::from_parts(0, 0), Ulid::from_parts(0, 1)]),
                    to: HashSet::from([Ulid::from_parts(0, 7)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 6),
                Node {
                    id: Ulid::from_parts(0, 6),
                    from: HashSet::from([Ulid::from_parts(0, 7)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 7),
                Node {
                    id: Ulid::from_parts(0, 7),
                    from: HashSet::from([Ulid::from_parts(0, 5)]),
                    to: HashSet::from([Ulid::from_parts(0, 6)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 8),
                Node {
                    id: Ulid::from_parts(0, 8),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
        ])
    );
    assert!(weave.models.is_empty());
    assert!(weave.model_nodes.is_empty());
    assert_eq!(
        weave.multiparent_nodes,
        HashSet::from([Ulid::from_parts(0, 2), Ulid::from_parts(0, 5)])
    );
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(weave.root_nodes, HashSet::from([Ulid::from_parts(0, 0)]));
    assert_eq!(
        weave.bookmarked_nodes,
        HashSet::from([Ulid::from_parts(0, 1)])
    );

    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(0, 9),
                from: HashSet::new(),
                to: HashSet::new(),
                active: true,
                bookmarked: false,
                content: NodeContent::Snippet(SnippetContent {
                    content: Bytes::new(),
                    model: Some(ContentModel {
                        id: Ulid::from_parts(0, 0),
                        parameters: Vec::new(),
                    }),
                    metadata: None,
                })
            },
            Some(Model {
                id: Ulid::from_parts(0, 0),
                label: String::new(),
                metadata: HashMap::new()
            }),
            false,
        ),
        Some(Ulid::from_parts(0, 9))
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::from([
                        Ulid::from_parts(0, 1),
                        Ulid::from_parts(0, 3),
                        Ulid::from_parts(0, 4),
                        Ulid::from_parts(0, 5),
                        Ulid::from_parts(0, 8)
                    ]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 1),
                Node {
                    id: Ulid::from_parts(0, 1),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 5)]),
                    active: false,
                    bookmarked: true,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 2),
                Node {
                    id: Ulid::from_parts(0, 2),
                    from: HashSet::from([Ulid::from_parts(0, 3), Ulid::from_parts(0, 4)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 3),
                Node {
                    id: Ulid::from_parts(0, 3),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 2)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 4),
                Node {
                    id: Ulid::from_parts(0, 4),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 2)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 5),
                Node {
                    id: Ulid::from_parts(0, 5),
                    from: HashSet::from([Ulid::from_parts(0, 0), Ulid::from_parts(0, 1)]),
                    to: HashSet::from([Ulid::from_parts(0, 7)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 6),
                Node {
                    id: Ulid::from_parts(0, 6),
                    from: HashSet::from([Ulid::from_parts(0, 7)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 7),
                Node {
                    id: Ulid::from_parts(0, 7),
                    from: HashSet::from([Ulid::from_parts(0, 5)]),
                    to: HashSet::from([Ulid::from_parts(0, 6)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 8),
                Node {
                    id: Ulid::from_parts(0, 8),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 9),
                Node {
                    id: Ulid::from_parts(0, 9),
                    from: HashSet::new(),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Snippet(SnippetContent {
                        content: Bytes::new(),
                        model: Some(ContentModel {
                            id: Ulid::from_parts(0, 0),
                            parameters: Vec::new(),
                        }),
                        metadata: None,
                    })
                }
            )
        ])
    );
    assert_eq!(
        weave.models,
        HashMap::from([(
            Ulid::from_parts(0, 0),
            Model {
                id: Ulid::from_parts(0, 0),
                label: String::new(),
                metadata: HashMap::new()
            }
        )])
    );
    assert_eq!(
        weave.model_nodes,
        HashMap::from([(
            Ulid::from_parts(0, 0),
            HashSet::from([Ulid::from_parts(0, 9)])
        )])
    );
    assert_eq!(
        weave.multiparent_nodes,
        HashSet::from([Ulid::from_parts(0, 2), Ulid::from_parts(0, 5)])
    );
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(
        weave.root_nodes,
        HashSet::from([Ulid::from_parts(0, 0), Ulid::from_parts(0, 9)])
    );
    assert_eq!(
        weave.bookmarked_nodes,
        HashSet::from([Ulid::from_parts(0, 1)])
    );

    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(0, 10),
                from: HashSet::new(),
                to: HashSet::new(),
                active: true,
                bookmarked: false,
                content: NodeContent::Snippet(SnippetContent {
                    content: Bytes::new(),
                    model: Some(ContentModel {
                        id: Ulid::from_parts(0, 0),
                        parameters: Vec::new(),
                    }),
                    metadata: None,
                })
            },
            Some(Model {
                id: Ulid::from_parts(0, 0),
                label: "Test Model".to_string(),
                metadata: HashMap::new()
            }),
            false,
        ),
        Some(Ulid::from_parts(0, 10))
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::from([
                        Ulid::from_parts(0, 1),
                        Ulid::from_parts(0, 3),
                        Ulid::from_parts(0, 4),
                        Ulid::from_parts(0, 5),
                        Ulid::from_parts(0, 8)
                    ]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 1),
                Node {
                    id: Ulid::from_parts(0, 1),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 5)]),
                    active: false,
                    bookmarked: true,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 2),
                Node {
                    id: Ulid::from_parts(0, 2),
                    from: HashSet::from([Ulid::from_parts(0, 3), Ulid::from_parts(0, 4)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 3),
                Node {
                    id: Ulid::from_parts(0, 3),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 2)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 4),
                Node {
                    id: Ulid::from_parts(0, 4),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 2)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 5),
                Node {
                    id: Ulid::from_parts(0, 5),
                    from: HashSet::from([Ulid::from_parts(0, 0), Ulid::from_parts(0, 1)]),
                    to: HashSet::from([Ulid::from_parts(0, 7)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 6),
                Node {
                    id: Ulid::from_parts(0, 6),
                    from: HashSet::from([Ulid::from_parts(0, 7)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 7),
                Node {
                    id: Ulid::from_parts(0, 7),
                    from: HashSet::from([Ulid::from_parts(0, 5)]),
                    to: HashSet::from([Ulid::from_parts(0, 6)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 8),
                Node {
                    id: Ulid::from_parts(0, 8),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 9),
                Node {
                    id: Ulid::from_parts(0, 9),
                    from: HashSet::new(),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Snippet(SnippetContent {
                        content: Bytes::new(),
                        model: Some(ContentModel {
                            id: Ulid::from_parts(0, 0),
                            parameters: Vec::new(),
                        }),
                        metadata: None,
                    })
                }
            ),
            (
                Ulid::from_parts(0, 10),
                Node {
                    id: Ulid::from_parts(0, 10),
                    from: HashSet::new(),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Snippet(SnippetContent {
                        content: Bytes::new(),
                        model: Some(ContentModel {
                            id: Ulid::from_parts(0, 0),
                            parameters: Vec::new(),
                        }),
                        metadata: None,
                    })
                }
            )
        ])
    );
    assert_eq!(
        weave.models,
        HashMap::from([(
            Ulid::from_parts(0, 0),
            Model {
                id: Ulid::from_parts(0, 0),
                label: "Test Model".to_string(),
                metadata: HashMap::new()
            }
        )])
    );
    assert_eq!(
        weave.model_nodes,
        HashMap::from([(
            Ulid::from_parts(0, 0),
            HashSet::from([Ulid::from_parts(0, 9), Ulid::from_parts(0, 10)])
        )])
    );
    assert_eq!(
        weave.multiparent_nodes,
        HashSet::from([Ulid::from_parts(0, 2), Ulid::from_parts(0, 5)])
    );
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(
        weave.root_nodes,
        HashSet::from([
            Ulid::from_parts(0, 0),
            Ulid::from_parts(0, 9),
            Ulid::from_parts(0, 10)
        ])
    );
    assert_eq!(
        weave.bookmarked_nodes,
        HashSet::from([Ulid::from_parts(0, 1)])
    );

    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(0, 11),
                from: HashSet::new(),
                to: HashSet::new(),
                active: true,
                bookmarked: false,
                content: NodeContent::Snippet(SnippetContent {
                    content: Bytes::new(),
                    model: Some(ContentModel {
                        id: Ulid::from_parts(0, 0),
                        parameters: Vec::new(),
                    }),
                    metadata: None,
                })
            },
            None,
            false,
        ),
        Some(Ulid::from_parts(0, 11))
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::from([
                        Ulid::from_parts(0, 1),
                        Ulid::from_parts(0, 3),
                        Ulid::from_parts(0, 4),
                        Ulid::from_parts(0, 5),
                        Ulid::from_parts(0, 8)
                    ]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 1),
                Node {
                    id: Ulid::from_parts(0, 1),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 5)]),
                    active: false,
                    bookmarked: true,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 2),
                Node {
                    id: Ulid::from_parts(0, 2),
                    from: HashSet::from([Ulid::from_parts(0, 3), Ulid::from_parts(0, 4)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 3),
                Node {
                    id: Ulid::from_parts(0, 3),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 2)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 4),
                Node {
                    id: Ulid::from_parts(0, 4),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 2)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 5),
                Node {
                    id: Ulid::from_parts(0, 5),
                    from: HashSet::from([Ulid::from_parts(0, 0), Ulid::from_parts(0, 1)]),
                    to: HashSet::from([Ulid::from_parts(0, 7)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 6),
                Node {
                    id: Ulid::from_parts(0, 6),
                    from: HashSet::from([Ulid::from_parts(0, 7)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 7),
                Node {
                    id: Ulid::from_parts(0, 7),
                    from: HashSet::from([Ulid::from_parts(0, 5)]),
                    to: HashSet::from([Ulid::from_parts(0, 6)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 8),
                Node {
                    id: Ulid::from_parts(0, 8),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 9),
                Node {
                    id: Ulid::from_parts(0, 9),
                    from: HashSet::new(),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Snippet(SnippetContent {
                        content: Bytes::new(),
                        model: Some(ContentModel {
                            id: Ulid::from_parts(0, 0),
                            parameters: Vec::new(),
                        }),
                        metadata: None,
                    })
                }
            ),
            (
                Ulid::from_parts(0, 10),
                Node {
                    id: Ulid::from_parts(0, 10),
                    from: HashSet::new(),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Snippet(SnippetContent {
                        content: Bytes::new(),
                        model: Some(ContentModel {
                            id: Ulid::from_parts(0, 0),
                            parameters: Vec::new(),
                        }),
                        metadata: None,
                    })
                }
            ),
            (
                Ulid::from_parts(0, 11),
                Node {
                    id: Ulid::from_parts(0, 11),
                    from: HashSet::new(),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Snippet(SnippetContent {
                        content: Bytes::new(),
                        model: Some(ContentModel {
                            id: Ulid::from_parts(0, 0),
                            parameters: Vec::new(),
                        }),
                        metadata: None,
                    })
                }
            )
        ])
    );
    assert_eq!(
        weave.models,
        HashMap::from([(
            Ulid::from_parts(0, 0),
            Model {
                id: Ulid::from_parts(0, 0),
                label: "Test Model".to_string(),
                metadata: HashMap::new()
            }
        )])
    );
    assert_eq!(
        weave.model_nodes,
        HashMap::from([(
            Ulid::from_parts(0, 0),
            HashSet::from([
                Ulid::from_parts(0, 9),
                Ulid::from_parts(0, 10),
                Ulid::from_parts(0, 11)
            ])
        )])
    );
    assert_eq!(
        weave.multiparent_nodes,
        HashSet::from([Ulid::from_parts(0, 2), Ulid::from_parts(0, 5)])
    );
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(
        weave.root_nodes,
        HashSet::from([
            Ulid::from_parts(0, 0),
            Ulid::from_parts(0, 9),
            Ulid::from_parts(0, 10),
            Ulid::from_parts(0, 11)
        ])
    );
    assert_eq!(
        weave.bookmarked_nodes,
        HashSet::from([Ulid::from_parts(0, 1)])
    );
}

#[test]
#[should_panic]
fn add_node_invalid_parent() {
    let mut weave = Weave::default();
    let _ = weave.add_node(
        Node {
            id: Ulid::from_parts(0, 0),
            from: HashSet::from([Ulid::from_parts(0, 1)]),
            to: HashSet::new(),
            active: true,
            bookmarked: false,
            content: NodeContent::Blank,
        },
        None,
        true,
    );
}

#[test]
#[should_panic]
fn add_node_invalid_child() {
    let mut weave = Weave::default();
    let _ = weave.add_node(
        Node {
            id: Ulid::from_parts(0, 0),
            from: HashSet::new(),
            to: HashSet::from([Ulid::from_parts(0, 1)]),
            active: true,
            bookmarked: false,
            content: NodeContent::Blank,
        },
        None,
        true,
    );
}

#[test]
#[should_panic]
fn add_node_unspecified_model() {
    let mut weave = Weave::default();
    let _ = weave.add_node(
        Node {
            id: Ulid::from_parts(0, 0),
            from: HashSet::new(),
            to: HashSet::new(),
            active: true,
            bookmarked: false,
            content: NodeContent::Snippet(SnippetContent {
                content: Bytes::new(),
                model: Some(ContentModel {
                    id: Ulid::from_parts(0, 0),
                    parameters: Vec::new(),
                }),
                metadata: None,
            }),
        },
        None,
        true,
    );
}

#[test]
#[should_panic]
fn add_node_invalid_model() {
    let mut weave = Weave::default();
    let _ = weave.add_node(
        Node {
            id: Ulid::from_parts(0, 0),
            from: HashSet::new(),
            to: HashSet::new(),
            active: true,
            bookmarked: false,
            content: NodeContent::Snippet(SnippetContent {
                content: Bytes::new(),
                model: Some(ContentModel {
                    id: Ulid::from_parts(0, 0),
                    parameters: Vec::new(),
                }),
                metadata: None,
            }),
        },
        Some(Model {
            id: Ulid::from_parts(0, 1),
            label: String::new(),
            metadata: HashMap::new(),
        }),
        true,
    );
}

#[test]
fn add_node_nonconcatable() {
    let mut weave = Weave::default();
    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(1, 0),
                from: HashSet::new(),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Diff(DiffContent::default()),
            },
            None,
            true,
        ),
        Some(Ulid::from_parts(1, 0))
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([(
            Ulid::from_parts(1, 0),
            Node {
                id: Ulid::from_parts(1, 0),
                from: HashSet::new(),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Diff(DiffContent::default()),
            }
        )])
    );
    assert!(weave.models.is_empty());
    assert!(weave.model_nodes.is_empty());
    assert!(weave.bookmarked_nodes.is_empty());
    assert!(weave.multiparent_nodes.is_empty());
    assert_eq!(
        weave.nonconcatable_nodes,
        HashSet::from([Ulid::from_parts(1, 0)])
    );
    assert_eq!(weave.root_nodes, HashSet::from([Ulid::from_parts(1, 0)]));

    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(0, 0),
                from: HashSet::new(),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            true,
        ),
        Some(Ulid::from_parts(0, 0))
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(1, 0),
                Node {
                    id: Ulid::from_parts(1, 0),
                    from: HashSet::new(),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Diff(DiffContent::default()),
                }
            )
        ])
    );
    assert!(weave.models.is_empty());
    assert!(weave.model_nodes.is_empty());
    assert!(weave.bookmarked_nodes.is_empty());
    assert!(weave.multiparent_nodes.is_empty());
    assert_eq!(
        weave.nonconcatable_nodes,
        HashSet::from([Ulid::from_parts(1, 0)])
    );
    assert_eq!(
        weave.root_nodes,
        HashSet::from([Ulid::from_parts(0, 0), Ulid::from_parts(1, 0)])
    );

    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(0, 1),
                from: HashSet::new(),
                to: HashSet::from([Ulid::from_parts(0, 0)]),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            true,
        ),
        None
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(1, 0),
                Node {
                    id: Ulid::from_parts(1, 0),
                    from: HashSet::new(),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Diff(DiffContent::default()),
                }
            )
        ])
    );
    assert!(weave.models.is_empty());
    assert!(weave.model_nodes.is_empty());
    assert!(weave.bookmarked_nodes.is_empty());
    assert!(weave.multiparent_nodes.is_empty());
    assert_eq!(
        weave.nonconcatable_nodes,
        HashSet::from([Ulid::from_parts(1, 0)])
    );
    assert_eq!(
        weave.root_nodes,
        HashSet::from([Ulid::from_parts(0, 0), Ulid::from_parts(1, 0)])
    );

    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(0, 1),
                from: HashSet::from([Ulid::from_parts(0, 0)]),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            true,
        ),
        Some(Ulid::from_parts(0, 1))
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(0, 1)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 1),
                Node {
                    id: Ulid::from_parts(0, 1),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(1, 0),
                Node {
                    id: Ulid::from_parts(1, 0),
                    from: HashSet::new(),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Diff(DiffContent::default()),
                }
            )
        ])
    );
    assert!(weave.models.is_empty());
    assert!(weave.model_nodes.is_empty());
    assert!(weave.bookmarked_nodes.is_empty());
    assert!(weave.multiparent_nodes.is_empty());
    assert_eq!(
        weave.nonconcatable_nodes,
        HashSet::from([Ulid::from_parts(1, 0)])
    );
    assert_eq!(
        weave.root_nodes,
        HashSet::from([Ulid::from_parts(0, 0), Ulid::from_parts(1, 0)])
    );

    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(0, 2),
                from: HashSet::from([Ulid::from_parts(0, 0)]),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            true,
        ),
        Some(Ulid::from_parts(0, 2))
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(0, 1), Ulid::from_parts(0, 2)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 1),
                Node {
                    id: Ulid::from_parts(0, 1),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 2),
                Node {
                    id: Ulid::from_parts(0, 2),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(1, 0),
                Node {
                    id: Ulid::from_parts(1, 0),
                    from: HashSet::new(),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Diff(DiffContent::default()),
                }
            )
        ])
    );
    assert!(weave.models.is_empty());
    assert!(weave.model_nodes.is_empty());
    assert!(weave.bookmarked_nodes.is_empty());
    assert!(weave.multiparent_nodes.is_empty());
    assert_eq!(
        weave.nonconcatable_nodes,
        HashSet::from([Ulid::from_parts(1, 0)])
    );
    assert_eq!(
        weave.root_nodes,
        HashSet::from([Ulid::from_parts(0, 0), Ulid::from_parts(1, 0)])
    );

    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(0, 3),
                from: HashSet::from([Ulid::from_parts(0, 1), Ulid::from_parts(0, 2)]),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            true,
        ),
        None
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(0, 1), Ulid::from_parts(0, 2)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 1),
                Node {
                    id: Ulid::from_parts(0, 1),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 2),
                Node {
                    id: Ulid::from_parts(0, 2),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(1, 0),
                Node {
                    id: Ulid::from_parts(1, 0),
                    from: HashSet::new(),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Diff(DiffContent::default()),
                }
            )
        ])
    );
    assert!(weave.models.is_empty());
    assert!(weave.model_nodes.is_empty());
    assert!(weave.bookmarked_nodes.is_empty());
    assert!(weave.multiparent_nodes.is_empty());
    assert_eq!(
        weave.nonconcatable_nodes,
        HashSet::from([Ulid::from_parts(1, 0)])
    );
    assert_eq!(
        weave.root_nodes,
        HashSet::from([Ulid::from_parts(0, 0), Ulid::from_parts(1, 0)])
    );

    assert_eq!(
        weave.add_node(
            Node {
                id: Ulid::from_parts(0, 3),
                from: HashSet::from([Ulid::from_parts(0, 2)]),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            true,
        ),
        Some(Ulid::from_parts(0, 3))
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(0, 1), Ulid::from_parts(0, 2)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 1),
                Node {
                    id: Ulid::from_parts(0, 1),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 2),
                Node {
                    id: Ulid::from_parts(0, 2),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 3)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(0, 3),
                Node {
                    id: Ulid::from_parts(0, 3),
                    from: HashSet::from([Ulid::from_parts(0, 2)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
            (
                Ulid::from_parts(1, 0),
                Node {
                    id: Ulid::from_parts(1, 0),
                    from: HashSet::new(),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Diff(DiffContent::default()),
                }
            )
        ])
    );
    assert!(weave.models.is_empty());
    assert!(weave.model_nodes.is_empty());
    assert!(weave.bookmarked_nodes.is_empty());
    assert!(weave.multiparent_nodes.is_empty());
    assert_eq!(
        weave.nonconcatable_nodes,
        HashSet::from([Ulid::from_parts(1, 0)])
    );
    assert_eq!(
        weave.root_nodes,
        HashSet::from([Ulid::from_parts(0, 0), Ulid::from_parts(1, 0)])
    );
}

#[test]
fn remove_node() {
    let mut weave = Weave::default();
    weave
        .add_node(
            Node {
                id: Ulid::from_parts(0, 0),
                from: HashSet::new(),
                to: HashSet::new(),
                active: true,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            false,
        )
        .unwrap();
    weave
        .add_node(
            Node {
                id: Ulid::from_parts(0, 1),
                from: HashSet::from([Ulid::from_parts(0, 0)]),
                to: HashSet::new(),
                active: true,
                bookmarked: true,
                content: NodeContent::Blank,
            },
            None,
            false,
        )
        .unwrap();
    weave
        .add_node(
            Node {
                id: Ulid::from_parts(0, 2),
                from: HashSet::from([Ulid::from_parts(0, 0)]),
                to: HashSet::new(),
                active: true,
                bookmarked: true,
                content: NodeContent::Blank,
            },
            None,
            false,
        )
        .unwrap();
    weave
        .add_node(
            Node {
                id: Ulid::from_parts(0, 3),
                from: HashSet::from([Ulid::from_parts(0, 2)]),
                to: HashSet::new(),
                active: true,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            false,
        )
        .unwrap();
    weave
        .add_node(
            Node {
                id: Ulid::from_parts(1, 0),
                from: HashSet::new(),
                to: HashSet::new(),
                active: true,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            false,
        )
        .unwrap();
    weave
        .add_node(
            Node {
                id: Ulid::from_parts(1, 1),
                from: HashSet::new(),
                to: HashSet::new(),
                active: true,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            false,
        )
        .unwrap();
    weave
        .add_node(
            Node {
                id: Ulid::from_parts(1, 2),
                from: HashSet::from([Ulid::from_parts(1, 0)]),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            false,
        )
        .unwrap();
    weave
        .add_node(
            Node {
                id: Ulid::from_parts(1, 3),
                from: HashSet::from([Ulid::from_parts(1, 0), Ulid::from_parts(1, 1)]),
                to: HashSet::new(),
                active: true,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            false,
        )
        .unwrap();
    weave
        .add_node(
            Node {
                id: Ulid::from_parts(1, 4),
                from: HashSet::from([Ulid::from_parts(1, 1)]),
                to: HashSet::new(),
                active: true,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            false,
        )
        .unwrap();
    weave
        .add_node(
            Node {
                id: Ulid::from_parts(1, 5),
                from: HashSet::from([Ulid::from_parts(1, 2)]),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            false,
        )
        .unwrap();
    weave
        .add_node(
            Node {
                id: Ulid::from_parts(1, 6),
                from: HashSet::from([Ulid::from_parts(1, 2)]),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            false,
        )
        .unwrap();
    weave
        .add_node(
            Node {
                id: Ulid::from_parts(1, 7),
                from: HashSet::from([Ulid::from_parts(1, 2), Ulid::from_parts(1, 3)]),
                to: HashSet::new(),
                active: true,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            false,
        )
        .unwrap();
    weave
        .add_node(
            Node {
                id: Ulid::from_parts(1, 8),
                from: HashSet::from([Ulid::from_parts(1, 4)]),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            false,
        )
        .unwrap();
    weave
        .add_node(
            Node {
                id: Ulid::from_parts(1, 9),
                from: HashSet::from([Ulid::from_parts(1, 4)]),
                to: HashSet::new(),
                active: true,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            false,
        )
        .unwrap();
    weave
        .add_node(
            Node {
                id: Ulid::from_parts(1, 10),
                from: HashSet::from([
                    Ulid::from_parts(1, 6),
                    Ulid::from_parts(1, 7),
                    Ulid::from_parts(1, 3),
                ]),
                to: HashSet::new(),
                active: true,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            false,
        )
        .unwrap();
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(0, 1), Ulid::from_parts(0, 2)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(0, 1),
                Node {
                    id: Ulid::from_parts(0, 1),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: true,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(0, 2),
                Node {
                    id: Ulid::from_parts(0, 2),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::from([Ulid::from_parts(0, 3)]),
                    active: true,
                    bookmarked: true,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(0, 3),
                Node {
                    id: Ulid::from_parts(0, 3),
                    from: HashSet::from([Ulid::from_parts(0, 2)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 0),
                Node {
                    id: Ulid::from_parts(1, 0),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(1, 2), Ulid::from_parts(1, 3)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 1),
                Node {
                    id: Ulid::from_parts(1, 1),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(1, 3), Ulid::from_parts(1, 4)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 2),
                Node {
                    id: Ulid::from_parts(1, 2),
                    from: HashSet::from([Ulid::from_parts(1, 0)]),
                    to: HashSet::from([
                        Ulid::from_parts(1, 5),
                        Ulid::from_parts(1, 6),
                        Ulid::from_parts(1, 7),
                    ]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 3),
                Node {
                    id: Ulid::from_parts(1, 3),
                    from: HashSet::from([Ulid::from_parts(1, 0), Ulid::from_parts(1, 1)]),
                    to: HashSet::from([Ulid::from_parts(1, 7), Ulid::from_parts(1, 10)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 4),
                Node {
                    id: Ulid::from_parts(1, 4),
                    from: HashSet::from([Ulid::from_parts(1, 1)]),
                    to: HashSet::from([Ulid::from_parts(1, 8), Ulid::from_parts(1, 9)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 5),
                Node {
                    id: Ulid::from_parts(1, 5),
                    from: HashSet::from([Ulid::from_parts(1, 2)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 6),
                Node {
                    id: Ulid::from_parts(1, 6),
                    from: HashSet::from([Ulid::from_parts(1, 2)]),
                    to: HashSet::from([Ulid::from_parts(1, 10)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 7),
                Node {
                    id: Ulid::from_parts(1, 7),
                    from: HashSet::from([Ulid::from_parts(1, 2), Ulid::from_parts(1, 3)]),
                    to: HashSet::from([Ulid::from_parts(1, 10)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 8),
                Node {
                    id: Ulid::from_parts(1, 8),
                    from: HashSet::from([Ulid::from_parts(1, 4)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 9),
                Node {
                    id: Ulid::from_parts(1, 9),
                    from: HashSet::from([Ulid::from_parts(1, 4)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 10),
                Node {
                    id: Ulid::from_parts(1, 10),
                    from: HashSet::from([
                        Ulid::from_parts(1, 6),
                        Ulid::from_parts(1, 7),
                        Ulid::from_parts(1, 3),
                    ]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
        ])
    );
    //assert!(weave.models.is_empty());
    //assert!(weave.model_nodes.is_empty());
    assert_eq!(
        weave.bookmarked_nodes,
        HashSet::from([Ulid::from_parts(0, 1), Ulid::from_parts(0, 2)])
    );
    assert_eq!(
        weave.multiparent_nodes,
        HashSet::from([
            Ulid::from_parts(1, 3),
            Ulid::from_parts(1, 7),
            Ulid::from_parts(1, 10)
        ])
    );
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(
        weave.root_nodes,
        HashSet::from([
            Ulid::from_parts(1, 0),
            Ulid::from_parts(1, 1),
            Ulid::from_parts(0, 0)
        ])
    );

    assert_eq!(
        weave.remove_node(&Ulid::from_parts(0, 2)),
        Some(Node {
            id: Ulid::from_parts(0, 2),
            from: HashSet::from([Ulid::from_parts(0, 0)]),
            to: HashSet::from([Ulid::from_parts(0, 3)]),
            active: true,
            bookmarked: true,
            content: NodeContent::Blank,
        })
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(0, 0),
                Node {
                    id: Ulid::from_parts(0, 0),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(0, 1)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(0, 1),
                Node {
                    id: Ulid::from_parts(0, 1),
                    from: HashSet::from([Ulid::from_parts(0, 0)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: true,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 0),
                Node {
                    id: Ulid::from_parts(1, 0),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(1, 2), Ulid::from_parts(1, 3)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 1),
                Node {
                    id: Ulid::from_parts(1, 1),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(1, 3), Ulid::from_parts(1, 4)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 2),
                Node {
                    id: Ulid::from_parts(1, 2),
                    from: HashSet::from([Ulid::from_parts(1, 0)]),
                    to: HashSet::from([
                        Ulid::from_parts(1, 5),
                        Ulid::from_parts(1, 6),
                        Ulid::from_parts(1, 7),
                    ]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 3),
                Node {
                    id: Ulid::from_parts(1, 3),
                    from: HashSet::from([Ulid::from_parts(1, 0), Ulid::from_parts(1, 1)]),
                    to: HashSet::from([Ulid::from_parts(1, 7), Ulid::from_parts(1, 10)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 4),
                Node {
                    id: Ulid::from_parts(1, 4),
                    from: HashSet::from([Ulid::from_parts(1, 1)]),
                    to: HashSet::from([Ulid::from_parts(1, 8), Ulid::from_parts(1, 9)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 5),
                Node {
                    id: Ulid::from_parts(1, 5),
                    from: HashSet::from([Ulid::from_parts(1, 2)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 6),
                Node {
                    id: Ulid::from_parts(1, 6),
                    from: HashSet::from([Ulid::from_parts(1, 2)]),
                    to: HashSet::from([Ulid::from_parts(1, 10)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 7),
                Node {
                    id: Ulid::from_parts(1, 7),
                    from: HashSet::from([Ulid::from_parts(1, 2), Ulid::from_parts(1, 3)]),
                    to: HashSet::from([Ulid::from_parts(1, 10)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 8),
                Node {
                    id: Ulid::from_parts(1, 8),
                    from: HashSet::from([Ulid::from_parts(1, 4)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 9),
                Node {
                    id: Ulid::from_parts(1, 9),
                    from: HashSet::from([Ulid::from_parts(1, 4)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 10),
                Node {
                    id: Ulid::from_parts(1, 10),
                    from: HashSet::from([
                        Ulid::from_parts(1, 6),
                        Ulid::from_parts(1, 7),
                        Ulid::from_parts(1, 3),
                    ]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
        ])
    );
    //assert!(weave.models.is_empty());
    //assert!(weave.model_nodes.is_empty());
    assert_eq!(
        weave.bookmarked_nodes,
        HashSet::from([Ulid::from_parts(0, 1)])
    );
    assert_eq!(
        weave.multiparent_nodes,
        HashSet::from([
            Ulid::from_parts(1, 3),
            Ulid::from_parts(1, 7),
            Ulid::from_parts(1, 10)
        ])
    );
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(
        weave.root_nodes,
        HashSet::from([
            Ulid::from_parts(1, 0),
            Ulid::from_parts(1, 1),
            Ulid::from_parts(0, 0)
        ])
    );

    assert_eq!(
        weave.remove_node(&Ulid::from_parts(0, 0)),
        Some(Node {
            id: Ulid::from_parts(0, 0),
            from: HashSet::new(),
            to: HashSet::from([Ulid::from_parts(0, 1)]),
            active: true,
            bookmarked: false,
            content: NodeContent::Blank,
        })
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(1, 0),
                Node {
                    id: Ulid::from_parts(1, 0),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(1, 2), Ulid::from_parts(1, 3)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 1),
                Node {
                    id: Ulid::from_parts(1, 1),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(1, 3), Ulid::from_parts(1, 4)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 2),
                Node {
                    id: Ulid::from_parts(1, 2),
                    from: HashSet::from([Ulid::from_parts(1, 0)]),
                    to: HashSet::from([
                        Ulid::from_parts(1, 5),
                        Ulid::from_parts(1, 6),
                        Ulid::from_parts(1, 7),
                    ]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 3),
                Node {
                    id: Ulid::from_parts(1, 3),
                    from: HashSet::from([Ulid::from_parts(1, 0), Ulid::from_parts(1, 1)]),
                    to: HashSet::from([Ulid::from_parts(1, 7), Ulid::from_parts(1, 10)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 4),
                Node {
                    id: Ulid::from_parts(1, 4),
                    from: HashSet::from([Ulid::from_parts(1, 1)]),
                    to: HashSet::from([Ulid::from_parts(1, 8), Ulid::from_parts(1, 9)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 5),
                Node {
                    id: Ulid::from_parts(1, 5),
                    from: HashSet::from([Ulid::from_parts(1, 2)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 6),
                Node {
                    id: Ulid::from_parts(1, 6),
                    from: HashSet::from([Ulid::from_parts(1, 2)]),
                    to: HashSet::from([Ulid::from_parts(1, 10)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 7),
                Node {
                    id: Ulid::from_parts(1, 7),
                    from: HashSet::from([Ulid::from_parts(1, 2), Ulid::from_parts(1, 3)]),
                    to: HashSet::from([Ulid::from_parts(1, 10)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 8),
                Node {
                    id: Ulid::from_parts(1, 8),
                    from: HashSet::from([Ulid::from_parts(1, 4)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 9),
                Node {
                    id: Ulid::from_parts(1, 9),
                    from: HashSet::from([Ulid::from_parts(1, 4)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 10),
                Node {
                    id: Ulid::from_parts(1, 10),
                    from: HashSet::from([
                        Ulid::from_parts(1, 6),
                        Ulid::from_parts(1, 7),
                        Ulid::from_parts(1, 3),
                    ]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
        ])
    );
    //assert!(weave.models.is_empty());
    //assert!(weave.model_nodes.is_empty());
    assert!(weave.bookmarked_nodes.is_empty());
    assert_eq!(
        weave.multiparent_nodes,
        HashSet::from([
            Ulid::from_parts(1, 3),
            Ulid::from_parts(1, 7),
            Ulid::from_parts(1, 10)
        ])
    );
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(
        weave.root_nodes,
        HashSet::from([Ulid::from_parts(1, 0), Ulid::from_parts(1, 1)])
    );

    assert_eq!(
        weave.remove_node(&Ulid::from_parts(1, 7)),
        Some(Node {
            id: Ulid::from_parts(1, 7),
            from: HashSet::from([Ulid::from_parts(1, 2), Ulid::from_parts(1, 3)]),
            to: HashSet::from([Ulid::from_parts(1, 10)]),
            active: true,
            bookmarked: false,
            content: NodeContent::Blank,
        })
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(1, 0),
                Node {
                    id: Ulid::from_parts(1, 0),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(1, 2), Ulid::from_parts(1, 3)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 1),
                Node {
                    id: Ulid::from_parts(1, 1),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(1, 3), Ulid::from_parts(1, 4)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 2),
                Node {
                    id: Ulid::from_parts(1, 2),
                    from: HashSet::from([Ulid::from_parts(1, 0)]),
                    to: HashSet::from([Ulid::from_parts(1, 5), Ulid::from_parts(1, 6),]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 3),
                Node {
                    id: Ulid::from_parts(1, 3),
                    from: HashSet::from([Ulid::from_parts(1, 0), Ulid::from_parts(1, 1)]),
                    to: HashSet::from([Ulid::from_parts(1, 10)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 4),
                Node {
                    id: Ulid::from_parts(1, 4),
                    from: HashSet::from([Ulid::from_parts(1, 1)]),
                    to: HashSet::from([Ulid::from_parts(1, 8), Ulid::from_parts(1, 9)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 5),
                Node {
                    id: Ulid::from_parts(1, 5),
                    from: HashSet::from([Ulid::from_parts(1, 2)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 6),
                Node {
                    id: Ulid::from_parts(1, 6),
                    from: HashSet::from([Ulid::from_parts(1, 2)]),
                    to: HashSet::from([Ulid::from_parts(1, 10)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 8),
                Node {
                    id: Ulid::from_parts(1, 8),
                    from: HashSet::from([Ulid::from_parts(1, 4)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 9),
                Node {
                    id: Ulid::from_parts(1, 9),
                    from: HashSet::from([Ulid::from_parts(1, 4)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 10),
                Node {
                    id: Ulid::from_parts(1, 10),
                    from: HashSet::from([Ulid::from_parts(1, 6), Ulid::from_parts(1, 3),]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
        ])
    );
    //assert!(weave.models.is_empty());
    //assert!(weave.model_nodes.is_empty());
    assert!(weave.bookmarked_nodes.is_empty());
    assert_eq!(
        weave.multiparent_nodes,
        HashSet::from([Ulid::from_parts(1, 3), Ulid::from_parts(1, 10)])
    );
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(
        weave.root_nodes,
        HashSet::from([Ulid::from_parts(1, 0), Ulid::from_parts(1, 1)])
    );

    assert_eq!(
        weave.remove_node(&Ulid::from_parts(1, 3)),
        Some(Node {
            id: Ulid::from_parts(1, 3),
            from: HashSet::from([Ulid::from_parts(1, 0), Ulid::from_parts(1, 1)]),
            to: HashSet::from([Ulid::from_parts(1, 10)]),
            active: true,
            bookmarked: false,
            content: NodeContent::Blank,
        })
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(1, 0),
                Node {
                    id: Ulid::from_parts(1, 0),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(1, 2)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 1),
                Node {
                    id: Ulid::from_parts(1, 1),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(1, 4)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 2),
                Node {
                    id: Ulid::from_parts(1, 2),
                    from: HashSet::from([Ulid::from_parts(1, 0)]),
                    to: HashSet::from([Ulid::from_parts(1, 5), Ulid::from_parts(1, 6)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 4),
                Node {
                    id: Ulid::from_parts(1, 4),
                    from: HashSet::from([Ulid::from_parts(1, 1)]),
                    to: HashSet::from([Ulid::from_parts(1, 8), Ulid::from_parts(1, 9)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 5),
                Node {
                    id: Ulid::from_parts(1, 5),
                    from: HashSet::from([Ulid::from_parts(1, 2)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 6),
                Node {
                    id: Ulid::from_parts(1, 6),
                    from: HashSet::from([Ulid::from_parts(1, 2)]),
                    to: HashSet::from([Ulid::from_parts(1, 10)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 8),
                Node {
                    id: Ulid::from_parts(1, 8),
                    from: HashSet::from([Ulid::from_parts(1, 4)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 9),
                Node {
                    id: Ulid::from_parts(1, 9),
                    from: HashSet::from([Ulid::from_parts(1, 4)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 10),
                Node {
                    id: Ulid::from_parts(1, 10),
                    from: HashSet::from([Ulid::from_parts(1, 6)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
        ])
    );
    //assert!(weave.models.is_empty());
    //assert!(weave.model_nodes.is_empty());
    assert!(weave.bookmarked_nodes.is_empty());
    assert!(weave.multiparent_nodes.is_empty());
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(
        weave.root_nodes,
        HashSet::from([Ulid::from_parts(1, 0), Ulid::from_parts(1, 1)])
    );

    assert_eq!(
        weave.remove_node(&Ulid::from_parts(1, 5)),
        Some(Node {
            id: Ulid::from_parts(1, 5),
            from: HashSet::from([Ulid::from_parts(1, 2)]),
            to: HashSet::new(),
            active: false,
            bookmarked: false,
            content: NodeContent::Blank,
        })
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(1, 0),
                Node {
                    id: Ulid::from_parts(1, 0),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(1, 2)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 1),
                Node {
                    id: Ulid::from_parts(1, 1),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(1, 4)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 2),
                Node {
                    id: Ulid::from_parts(1, 2),
                    from: HashSet::from([Ulid::from_parts(1, 0)]),
                    to: HashSet::from([Ulid::from_parts(1, 6)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 4),
                Node {
                    id: Ulid::from_parts(1, 4),
                    from: HashSet::from([Ulid::from_parts(1, 1)]),
                    to: HashSet::from([Ulid::from_parts(1, 8), Ulid::from_parts(1, 9)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 6),
                Node {
                    id: Ulid::from_parts(1, 6),
                    from: HashSet::from([Ulid::from_parts(1, 2)]),
                    to: HashSet::from([Ulid::from_parts(1, 10)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 8),
                Node {
                    id: Ulid::from_parts(1, 8),
                    from: HashSet::from([Ulid::from_parts(1, 4)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 9),
                Node {
                    id: Ulid::from_parts(1, 9),
                    from: HashSet::from([Ulid::from_parts(1, 4)]),
                    to: HashSet::new(),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 10),
                Node {
                    id: Ulid::from_parts(1, 10),
                    from: HashSet::from([Ulid::from_parts(1, 6)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
        ])
    );
    //assert!(weave.models.is_empty());
    //assert!(weave.model_nodes.is_empty());
    assert!(weave.bookmarked_nodes.is_empty());
    assert!(weave.multiparent_nodes.is_empty());
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(
        weave.root_nodes,
        HashSet::from([Ulid::from_parts(1, 0), Ulid::from_parts(1, 1)])
    );

    assert_eq!(
        weave.remove_node(&Ulid::from_parts(1, 9)),
        Some(Node {
            id: Ulid::from_parts(1, 9),
            from: HashSet::from([Ulid::from_parts(1, 4)]),
            to: HashSet::new(),
            active: true,
            bookmarked: false,
            content: NodeContent::Blank,
        })
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(1, 0),
                Node {
                    id: Ulid::from_parts(1, 0),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(1, 2)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 1),
                Node {
                    id: Ulid::from_parts(1, 1),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(1, 4)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 2),
                Node {
                    id: Ulid::from_parts(1, 2),
                    from: HashSet::from([Ulid::from_parts(1, 0)]),
                    to: HashSet::from([Ulid::from_parts(1, 6)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 4),
                Node {
                    id: Ulid::from_parts(1, 4),
                    from: HashSet::from([Ulid::from_parts(1, 1)]),
                    to: HashSet::from([Ulid::from_parts(1, 8)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 6),
                Node {
                    id: Ulid::from_parts(1, 6),
                    from: HashSet::from([Ulid::from_parts(1, 2)]),
                    to: HashSet::from([Ulid::from_parts(1, 10)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 8),
                Node {
                    id: Ulid::from_parts(1, 8),
                    from: HashSet::from([Ulid::from_parts(1, 4)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 10),
                Node {
                    id: Ulid::from_parts(1, 10),
                    from: HashSet::from([Ulid::from_parts(1, 6)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
        ])
    );
    //assert!(weave.models.is_empty());
    //assert!(weave.model_nodes.is_empty());
    assert!(weave.bookmarked_nodes.is_empty());
    assert!(weave.multiparent_nodes.is_empty());
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(
        weave.root_nodes,
        HashSet::from([Ulid::from_parts(1, 0), Ulid::from_parts(1, 1)])
    );

    assert_eq!(weave.remove_node(&Ulid::from_parts(1, 16)), None);
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(1, 0),
                Node {
                    id: Ulid::from_parts(1, 0),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(1, 2)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 1),
                Node {
                    id: Ulid::from_parts(1, 1),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(1, 4)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 2),
                Node {
                    id: Ulid::from_parts(1, 2),
                    from: HashSet::from([Ulid::from_parts(1, 0)]),
                    to: HashSet::from([Ulid::from_parts(1, 6)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 4),
                Node {
                    id: Ulid::from_parts(1, 4),
                    from: HashSet::from([Ulid::from_parts(1, 1)]),
                    to: HashSet::from([Ulid::from_parts(1, 8)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 6),
                Node {
                    id: Ulid::from_parts(1, 6),
                    from: HashSet::from([Ulid::from_parts(1, 2)]),
                    to: HashSet::from([Ulid::from_parts(1, 10)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 8),
                Node {
                    id: Ulid::from_parts(1, 8),
                    from: HashSet::from([Ulid::from_parts(1, 4)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 10),
                Node {
                    id: Ulid::from_parts(1, 10),
                    from: HashSet::from([Ulid::from_parts(1, 6)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
        ])
    );
    //assert!(weave.models.is_empty());
    //assert!(weave.model_nodes.is_empty());
    assert!(weave.bookmarked_nodes.is_empty());
    assert!(weave.multiparent_nodes.is_empty());
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(
        weave.root_nodes,
        HashSet::from([Ulid::from_parts(1, 0), Ulid::from_parts(1, 1)])
    );

    weave
        .add_node(
            Node {
                id: Ulid::from_parts(1, 3),
                from: HashSet::from([Ulid::from_parts(1, 0)]),
                to: HashSet::new(),
                active: false,
                bookmarked: false,
                content: NodeContent::Diff(DiffContent::default()),
            },
            None,
            false,
        )
        .unwrap();
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(1, 0),
                Node {
                    id: Ulid::from_parts(1, 0),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(1, 2), Ulid::from_parts(1, 3)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 1),
                Node {
                    id: Ulid::from_parts(1, 1),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(1, 4)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 2),
                Node {
                    id: Ulid::from_parts(1, 2),
                    from: HashSet::from([Ulid::from_parts(1, 0)]),
                    to: HashSet::from([Ulid::from_parts(1, 6)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 3),
                Node {
                    id: Ulid::from_parts(1, 3),
                    from: HashSet::from([Ulid::from_parts(1, 0)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Diff(DiffContent::default()),
                },
            ),
            (
                Ulid::from_parts(1, 4),
                Node {
                    id: Ulid::from_parts(1, 4),
                    from: HashSet::from([Ulid::from_parts(1, 1)]),
                    to: HashSet::from([Ulid::from_parts(1, 8)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 6),
                Node {
                    id: Ulid::from_parts(1, 6),
                    from: HashSet::from([Ulid::from_parts(1, 2)]),
                    to: HashSet::from([Ulid::from_parts(1, 10)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 8),
                Node {
                    id: Ulid::from_parts(1, 8),
                    from: HashSet::from([Ulid::from_parts(1, 4)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 10),
                Node {
                    id: Ulid::from_parts(1, 10),
                    from: HashSet::from([Ulid::from_parts(1, 6)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
        ])
    );
    //assert!(weave.models.is_empty());
    //assert!(weave.model_nodes.is_empty());
    assert!(weave.bookmarked_nodes.is_empty());
    assert!(weave.multiparent_nodes.is_empty());
    assert_eq!(
        weave.nonconcatable_nodes,
        HashSet::from([Ulid::from_parts(1, 3)])
    );
    assert_eq!(
        weave.root_nodes,
        HashSet::from([Ulid::from_parts(1, 0), Ulid::from_parts(1, 1)])
    );

    assert_eq!(
        weave.remove_node(&Ulid::from_parts(1, 3)),
        Some(Node {
            id: Ulid::from_parts(1, 3),
            from: HashSet::from([Ulid::from_parts(1, 0)]),
            to: HashSet::new(),
            active: false,
            bookmarked: false,
            content: NodeContent::Diff(DiffContent::default()),
        })
    );
    assert_eq!(
        weave.nodes,
        HashMap::from([
            (
                Ulid::from_parts(1, 0),
                Node {
                    id: Ulid::from_parts(1, 0),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(1, 2)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 1),
                Node {
                    id: Ulid::from_parts(1, 1),
                    from: HashSet::new(),
                    to: HashSet::from([Ulid::from_parts(1, 4)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 2),
                Node {
                    id: Ulid::from_parts(1, 2),
                    from: HashSet::from([Ulid::from_parts(1, 0)]),
                    to: HashSet::from([Ulid::from_parts(1, 6)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 4),
                Node {
                    id: Ulid::from_parts(1, 4),
                    from: HashSet::from([Ulid::from_parts(1, 1)]),
                    to: HashSet::from([Ulid::from_parts(1, 8)]),
                    active: true,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 6),
                Node {
                    id: Ulid::from_parts(1, 6),
                    from: HashSet::from([Ulid::from_parts(1, 2)]),
                    to: HashSet::from([Ulid::from_parts(1, 10)]),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 8),
                Node {
                    id: Ulid::from_parts(1, 8),
                    from: HashSet::from([Ulid::from_parts(1, 4)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
            (
                Ulid::from_parts(1, 10),
                Node {
                    id: Ulid::from_parts(1, 10),
                    from: HashSet::from([Ulid::from_parts(1, 6)]),
                    to: HashSet::new(),
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                },
            ),
        ])
    );
    //assert!(weave.models.is_empty());
    //assert!(weave.model_nodes.is_empty());
    assert!(weave.bookmarked_nodes.is_empty());
    assert!(weave.multiparent_nodes.is_empty());
    assert!(weave.nonconcatable_nodes.is_empty());
    assert_eq!(
        weave.root_nodes,
        HashSet::from([Ulid::from_parts(1, 0), Ulid::from_parts(1, 1)])
    );
}

#[test]
#[should_panic]
fn remove_inconsistent_node() {
    let mut weave = Weave::default();
    weave
        .add_node(
            Node {
                id: Ulid::from_parts(0, 0),
                from: HashSet::new(),
                to: HashSet::new(),
                active: true,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            true,
        )
        .unwrap();
    weave.nodes.get_mut(&Ulid::from_parts(0, 0)).unwrap().id = Ulid::from_parts(0, 1);
    weave.remove_node(&Ulid::from_parts(0, 0));
}

/*#[test]
fn deduplicate_node() {}*/

/*#[test]
fn move_node() {}*/

/*#[test]
fn move_node_multiparent() {}*/

/*#[test]
fn move_node_nonconcatable() {}*/

/*#[test]
fn split_node() {}*/

/*#[test]
fn merge_node() {}*/

/*#[test]
fn get_active_timelines() {}*/
