#![allow(clippy::should_panic_without_expect)]
#![allow(clippy::too_many_lines)]

use super::*;

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
    update_node_activity(&mut nodes, &Ulid::from_parts(1, 0), false, true, None);
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
}

/*#[test]
fn get_active_timelines() {}*/

/*#[test]
fn add_model() {}*/

/*#[test]
fn add_node() {
    let mut weave = Weave::default();
    assert_eq!(
        weave.add_node(
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
                active: true,
                bookmarked: false,
                content: NodeContent::Blank,
            }
        )])
    );
    assert!(weave.models.is_empty());
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
            false,
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
                active: false,
                bookmarked: false,
                content: NodeContent::Blank,
            },
            None,
            false,
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
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
        ])
    );
    assert!(weave.models.is_empty());
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
                    active: false,
                    bookmarked: false,
                    content: NodeContent::Blank,
                }
            ),
        ])
    );
    assert!(weave.models.is_empty());
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

    println!("{:#?}", weave);
}*/

/*#[test]
fn add_node_multiparent() {}*/

/*#[test]
fn add_node_nonconcatable() {}*/

/*#[test]
fn remove_node() {}*/

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
