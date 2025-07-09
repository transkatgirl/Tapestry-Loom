#![allow(clippy::should_panic_without_expect)]
#![allow(clippy::too_many_lines)]

use super::*;

/*#[test]
fn update_node_activity() {}*/

/*#[test]
fn update_removed_child_activity() {}*/

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
