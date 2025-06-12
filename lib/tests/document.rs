use std::collections::{BTreeSet, HashMap, HashSet};

use ulid::Ulid;

use tapestry_weave::{
    content::{Node, NodeContent},
    document::{Weave, WeaveView},
};

fn add_blank_node(weave: &mut Weave, id: Ulid, from: &[Ulid], to: &[Ulid]) {
    assert!(
        weave.add_node(
            blank_node(id, from.iter().copied(), to.iter().copied()),
            None,
            false,
            false
        ) == Some(id)
    );
    if from.is_empty() {
        assert_eq!(
            weave
                .get_root_nodes()
                .map(|(node, _model)| (node.id, node))
                .collect::<HashMap<_, _>>()
                .remove(&id),
            Some(blank_node(id, from.iter().copied(), to.iter().copied())).as_ref()
        );
    }
    assert_eq!(
        weave.get_node(&id).0,
        Some(blank_node(id, from.iter().copied(), to.iter().copied())).as_ref()
    );
    assert_eq!(weave.get_node(&id).1, None);
    for parent in from {
        assert!(weave.get_node(parent).0.unwrap().to.contains(&id));
    }
    for child in from {
        assert!(weave.get_node(child).0.unwrap().from.contains(&id));
    }
}

fn blank_node<X, Y>(id: Ulid, from: X, to: Y) -> Node
where
    X: IntoIterator<Item = Ulid>,
    Y: IntoIterator<Item = Ulid>,
{
    Node {
        id,
        to: HashSet::from_iter(to),
        from: HashSet::from_iter(from),
        active: false,
        bookmarked: false,
        content: NodeContent::Blank,
    }
}

#[test]
fn add_node_propagation() {
    let mut weave = Weave::default();

    let root_node_identifier = Ulid::new();
    let root_node_2_identifier = Ulid::new();
    let child_node_1_identifier = Ulid::new();
    let child_node_2_identifier = Ulid::new();
    let child_node_3_identifier = Ulid::new();
    let child_node_4_identifier = Ulid::new();
    let child_node_5_identifier = Ulid::new();
    let child_node_6_identifier = Ulid::new();

    add_blank_node(&mut weave, root_node_identifier, &[], &[]);
    add_blank_node(&mut weave, root_node_2_identifier, &[], &[]);

    /*assert!(
        weave
            .add_node(
                blank_moveable_node(child_node_1_identifier, [root_node_identifier], []),
                None,
                false,
                false
            )
            .is_some()
    );
    assert!(
        weave
            .add_node(
                blank_moveable_node(child_node_2_identifier, [root_node_identifier], []),
                None,
                false,
                false
            )
            .is_some()
    );
    assert!(
        weave
            .add_node(
                blank_moveable_node(child_node_3_identifier, [child_node_2_identifier], []),
                None,
                false,
                false
            )
            .is_some()
    );
    assert!(
        weave
            .add_node(
                blank_moveable_node(child_node_4_identifier, [child_node_3_identifier], []),
                None,
                false,
                false
            )
            .is_some()
    );
    assert!(
        weave
            .add_node(
                blank_moveable_node(
                    child_node_5_identifier,
                    [child_node_3_identifier, child_node_4_identifier],
                    []
                ),
                None,
                false,
                false
            )
            .is_some()
    );
    assert!(
        weave
            .add_node(
                blank_moveable_node(child_node_6_identifier, [child_node_5_identifier], []),
                None,
                false,
                false
            )
            .is_some()
    );
    {
        assert!(
            weave.root_nodes == BTreeSet::from([root_node_identifier, root_node_2_identifier])
        );
        let root_node_1 = weave.nodes.get(&root_node_identifier).unwrap();
        let root_node_2 = weave.nodes.get(&root_node_2_identifier).unwrap();
        let child_node_1 = weave.nodes.get(&child_node_1_identifier).unwrap();
        let child_node_2 = weave.nodes.get(&child_node_2_identifier).unwrap();
        let child_node_3 = weave.nodes.get(&child_node_3_identifier).unwrap();
        let child_node_4 = weave.nodes.get(&child_node_4_identifier).unwrap();
        let child_node_5 = weave.nodes.get(&child_node_5_identifier).unwrap();
        let child_node_6 = weave.nodes.get(&child_node_6_identifier).unwrap();
        assert!(root_node_1.from.is_empty());
        assert!(
            root_node_1.to == HashSet::from([child_node_1_identifier, child_node_2_identifier])
        );
        assert!(root_node_2.from.is_empty());
        assert!(root_node_2.to.is_empty());
        assert!(child_node_1.from == HashSet::from([root_node_identifier]));
        assert!(child_node_1.to.is_empty());

        /*assert!(child_node_1.from == HashSet::from([root_node_identifier]));
        assert!(child_node_1.to.contains(&child_node_2_identifier));
        assert!(child_node_2.from == HashSet::from([child_node_1_identifier]));
        assert!(child_node_2.to == HashSet::from([child_node_3_identifier]));
        assert!(child_node_3.from == HashSet::from([child_node_2_identifier]));
        assert!(child_node_3.to.is_empty());*/

        todo!();
    }*/
}

/*#[test]
fn remove_node_propagation() {}

#[test]
fn add_node_model_propagation() {}

#[test]
fn remove_node_model_propagation() {}

#[test]
fn check_has_parent_loop() {}

#[test]
fn check_has_child_loop() {}

#[test]
fn add_node_check_loop() {}

#[test]
fn remove_node_check_loop() {}

#[test]
fn update_node_activation_propagation() {}

#[test]
fn update_node_activation_controlled_propagation() {}

#[test]
fn add_node_activation_propagation() {}

#[test]
fn remove_node_activation_propagation() {}

#[test]
fn add_node_deduplication() {}*/
