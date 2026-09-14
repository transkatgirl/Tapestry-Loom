#![allow(non_snake_case)]

use std::{
    hash::{BuildHasherDefault, RandomState},
    path::PathBuf,
    time::{Duration, SystemTime},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use stacksafe::stacksafe;
use tapestry_weave::{
    TapestryNode, TapestryWeave,
    content::{Creator, InnerNodeContent, Model, NodeContent},
    hashers::RandomIdHasher,
    jiff::{Timestamp, Zoned},
    nanorand::{Rng, WyRand},
    universal_weave::{
        MetadataWeave, Weave,
        indexmap::{IndexMap, IndexSet},
    },
    wrappers::UniqueIdentifierRemapper,
};
use uuid::Uuid;

use crate::new_weave;

pub fn migrate_all(input: &str, created: Zoned) -> anyhow::Result<Vec<(PathBuf, TapestryWeave)>> {
    if let Ok(data) =
        serde_json::from_str::<Value>(input).and_then(serde_json::from_value::<LoomsidianData>)
    // Makes parsing untagged enums more reliable
    {
        let mut output = Vec::with_capacity(data.state.len());

        for (filename, weave) in data.state {
            output.push((filename, convert_weave(weave, created.clone())?));
        }

        Ok(output)
    } else {
        Ok(Vec::default())
    }
}

pub fn migrate(input: &str, created: Zoned) -> anyhow::Result<Option<TapestryWeave>> {
    // Hack which makes parsing untagged enums more reliable
    if let Ok(data) =
        serde_json::from_str::<Value>(input).and_then(serde_json::from_value::<LoomsidianWeave>)
    {
        Ok(Some(convert_weave(data, created)?))
    } else {
        Ok(None)
    }
}

fn convert_weave(input: LoomsidianWeave, created: Zoned) -> anyhow::Result<TapestryWeave> {
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

    let time_zone = output.metadata().created.time_zone().clone();

    let mut rng = WyRand::new();

    let mut convert_old_identifier = move |id| *mapper.map(id, || rng.generate()).get();

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
            output.insert(TapestryNode {
                id: convert_old_identifier(id),
                from: IndexSet::from_iter(node.parentId.map(&mut convert_old_identifier).filter(
                    |parent_id| {
                        if output.contains(parent_id) {
                            true
                        } else {
                            eprintln!("Warning: Node {} has missing parents", id);
                            false
                        }
                    }
                )),
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
                    aux_metadata: IndexMap::default(),
                    creator: node
                        .author
                        .map(|author| {
                            if author != "genesis" && author != "N/A" {
                                Creator::Model(Some(Model {
                                    label: author,
                                    color: None,
                                    metadata: IndexMap::default(),
                                    identifier: None,
                                    seed: None,
                                    system_fingerprint: None,
                                    finish_reason: None,
                                }))
                            } else {
                                Creator::User(None)
                            }
                        })
                        .unwrap_or(Creator::Unknown)
                },
            })
        );
    }

    Ok(output)
}

#[stacksafe]
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
