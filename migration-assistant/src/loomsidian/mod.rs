#![allow(non_snake_case)]

use std::{
    hash::{BuildHasherDefault, RandomState},
    path::PathBuf,
    time::{Duration, SystemTime},
};

use serde::{Deserialize, Serialize};
use tapestry_weave::{
    VersionedWeave,
    hashers::RandomIdHasher,
    jiff::{Timestamp, Zoned},
    nanorand::Rng,
    universal_weave::{
        dependent::DependentNode,
        indexmap::{IndexMap, IndexSet},
    },
    v1::content::{Creator, InnerNodeContent, Model, NodeContent},
    wrappers::UniqueIdentifierRemapper,
};
use uuid::Uuid;

use crate::new_weave;

pub fn migrate_all(input: &str, created: Zoned) -> anyhow::Result<Vec<(PathBuf, VersionedWeave)>> {
    if let Ok(data) = serde_json::from_str::<LoomsidianData>(input) {
        let mut output = Vec::with_capacity(data.state.len());

        for (filename, weave) in data.state {
            output.push((filename, convert_weave(weave, created.clone())?));
        }

        Ok(output)
    } else {
        Ok(Vec::default())
    }
}

pub fn migrate(input: &str, created: Zoned) -> anyhow::Result<Option<VersionedWeave>> {
    if let Ok(data) = serde_json::from_str::<LoomsidianWeave>(input) {
        Ok(Some(convert_weave(data, created)?))
    } else {
        Ok(None)
    }
}

fn convert_weave(input: LoomsidianWeave, created: Zoned) -> anyhow::Result<VersionedWeave> {
    let mut nodes = input.nodes.into_map();

    let mut id_list = IndexSet::with_capacity(nodes.len());

    for (id, _) in &nodes {
        build_node_list(&nodes, id, &mut id_list);
    }

    let mut output = new_weave(nodes.len(), created, "Loomsidian", None);

    let mut mapper: UniqueIdentifierRemapper<
        Uuid,
        u64,
        RandomState,
        BuildHasherDefault<RandomIdHasher>,
    > = UniqueIdentifierRemapper::with_capacity(nodes.len());

    let mut rng = output.rng.clone();

    let mut convert_old_identifier = move |id| *mapper.map(id, || rng.generate()).get();

    let time_zone = output.metadata().created.time_zone().clone();

    for id in id_list {
        let node = nodes.swap_remove(&id).unwrap();

        let timestamp = node
            .lastVisited
            .and_then(|unix_time| {
                Timestamp::try_from(SystemTime::UNIX_EPOCH + Duration::from_millis(unix_time)).ok()
            })
            .map(|timestamp| Zoned::new(timestamp, time_zone.clone()))
            .unwrap_or_default();

        assert!(
            output.add_node_direct(DependentNode {
                id: convert_old_identifier(id),
                from: node
                    .parentId
                    .map(&mut convert_old_identifier)
                    .and_then(|id| if output.contains(&id) {
                        Some(id)
                    } else {
                        eprintln!("Warning: Node {} has missing parents", id);
                        None
                    }),
                to: IndexSet::default(),
                active: input.current == id,
                bookmarked: node.bookmarked,
                contents: NodeContent {
                    timestamp,
                    modified: false,
                    content: InnerNodeContent::Snippet(
                        node.text.or(node.value).unwrap_or_default().into_bytes()
                    ),
                    metadata: IndexMap::default(),
                    creator: node
                        .author
                        .and_then(|author| {
                            if author != "genesis" && author != "N/A" {
                                Some(Creator::Model(Some(Model {
                                    label: author,
                                    color: None,
                                    metadata: IndexMap::default(),
                                    identifier: None,
                                    seed: None,
                                    raw_query: None,
                                })))
                            } else {
                                None
                            }
                        })
                        .unwrap_or(Creator::Unknown)
                },
            })
        );
    }

    Ok(output.to_versioned_weave())
}

fn build_node_list(
    weave: &IndexMap<Uuid, LoomsidianNode>,
    node: &Uuid,
    nodes: &mut IndexSet<Uuid>,
) {
    if nodes.contains(node) {
        return;
    }

    let id = *node;
    if let Some(node) = weave.get(node) {
        if let Some(parent) = node.parentId {
            build_node_list(weave, &parent, nodes);
        }

        nodes.insert(id);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LoomsidianData {
    state: IndexMap<PathBuf, LoomsidianWeave>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LoomsidianWeave {
    current: Uuid,
    nodes: LoomsidianNodes,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
enum LoomsidianNodes {
    Map(IndexMap<Uuid, LoomsidianNode>),
    List(Vec<LoomsidianListNode>),
}

impl LoomsidianNodes {
    fn into_map(self) -> IndexMap<Uuid, LoomsidianNode> {
        match self {
            Self::Map(map) => map,
            Self::List(list) => list
                .into_iter()
                .map(|node| {
                    (
                        node.id,
                        LoomsidianNode {
                            text: node.text,
                            value: node.value,
                            author: node.author,
                            parentId: node.parentId,
                            bookmarked: node.bookmarked,
                            lastVisited: node.lastVisited,
                        },
                    )
                })
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LoomsidianListNode {
    id: Uuid,
    text: Option<String>,
    value: Option<String>,
    author: Option<String>,
    parentId: Option<Uuid>,

    #[serde(default)]
    bookmarked: bool,

    lastVisited: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LoomsidianNode {
    text: Option<String>,
    value: Option<String>,
    author: Option<String>,
    parentId: Option<Uuid>,

    #[serde(default)]
    bookmarked: bool,

    lastVisited: Option<u64>,
}
