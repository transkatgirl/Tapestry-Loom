#![allow(non_snake_case)]
#![allow(clippy::upper_case_acronyms)]

use std::{
    collections::{HashMap, HashSet},
    hash::{BuildHasherDefault, RandomState},
};

use serde::{Deserialize, Serialize};
use tapestry_weave::{
    VersionedWeave,
    chrono::{DateTime, Utc},
    hashers::RandomIdHasher,
    jiff::{Zoned, fmt::rfc2822::DateTimeParser},
    nanorand::Rng,
    universal_weave::{
        dependent::DependentNode,
        indexmap::{IndexMap, IndexSet},
    },
    v1::content::{Author, Creator, InnerNodeContent, Model, NodeContent},
    wrappers::UniqueIdentifierRemapper,
};

use crate::{
    exoloom::ExoloomAuthorType::{LLM, USER},
    new_weave,
};

const PARSER: DateTimeParser = DateTimeParser::new();

pub fn migrate(input: &str, created: Zoned) -> anyhow::Result<Option<VersionedWeave>> {
    if let Ok(mut data) = serde_json::from_str::<ExoloomWeave>(input) {
        assert!(data.loomType == "Exoloom" && data.schemaVersion == 1);

        let created = if let Some(createdAt) = data.tree.createdAt {
            PARSER
                .parse_zoned(createdAt.to_rfc2822())
                .unwrap_or(created)
        } else {
            created
        };

        let mut output = new_weave(
            data.tree.nodes.len(),
            created,
            "Exoloom",
            data.version.as_deref(),
        );

        let bookmarks: HashSet<u64> = data
            .lens
            .bookmarks
            .into_iter()
            .map(|bookmark| bookmark.nodeId)
            .collect();

        let pruned: HashSet<u64> = data
            .lens
            .prunedNodes
            .into_iter()
            .map(|pruned| pruned.nodeId)
            .collect();

        assert!(
            data.tree
                .nodes
                .get(&data.tree.rootNodeId)
                .unwrap()
                .parentId
                .is_none()
        );

        let mut id_list = IndexSet::with_capacity(data.tree.nodes.len());

        build_node_list(&data.tree, data.tree.rootNodeId, &mut id_list);

        let mut mapper: UniqueIdentifierRemapper<
            u64,
            u64,
            RandomState,
            BuildHasherDefault<RandomIdHasher>,
        > = UniqueIdentifierRemapper::with_capacity(id_list.len());

        let mut rng = output.rng.clone();

        let mut convert_old_identifier = move |id| *mapper.map(id, || rng.generate()).get();

        for id in id_list {
            let node = data.tree.nodes.remove(&id).unwrap();
            let bookmarked = bookmarks.contains(&id);
            let pruned = pruned.contains(&id);

            assert!(
                output.add_node(DependentNode {
                    id: convert_old_identifier(id),
                    from: node.parentId.map(&mut convert_old_identifier),
                    to: IndexSet::default(),
                    active: false,
                    bookmarked,
                    contents: NodeContent {
                        timestamp: node
                            .createdAt
                            .and_then(|timestamp| {
                                PARSER.parse_zoned(timestamp.to_rfc2822()).ok()
                            })
                            .unwrap_or_default(),
                        modified: false,
                        content: InnerNodeContent::Snippet(node.content.into_bytes()),
                        metadata: if pruned {
                            IndexMap::from_iter([("pruned".to_string(), "true".to_string())])
                        } else {
                            IndexMap::default()
                        },
                        creator: match node.authorType {
                            LLM => {
                                Creator::Model(node.authorName.map(|label| Model {
                                    label,
                                    color: None,
                                    identifier: None,
                                    seed: None,
                                    metadata: IndexMap::default(),
                                    raw_query: None,
                                }))
                            }
                            USER => {
                                Creator::User(node.authorName.map(|label| Author {
                                    label,
                                    color: None,
                                    identifier: None,
                                    metadata: IndexMap::default(),
                                }))
                            }
                        },
                    },
                })
            );
        }

        output.metadata().title = data.tree.title;
        output.metadata().description = data.tree.description;

        Ok(Some(output.to_versioned_weave()))
    } else {
        Ok(None)
    }
}

fn build_node_list(weave: &ExoloomTree, node: u64, nodes: &mut IndexSet<u64>) {
    if nodes.contains(&node) {
        return;
    }

    let id = node;
    if let Some(node) = weave.nodes.get(&node) {
        nodes.insert(id);

        for child in node.childrenIds.iter().copied() {
            build_node_list(weave, child, nodes);
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ExoloomWeave {
    loomType: String,
    version: Option<String>,
    schemaVersion: usize,
    tree: ExoloomTree,
    lens: ExoloomLens,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ExoloomTree {
    title: Option<String>,
    description: Option<String>,
    createdAt: Option<DateTime<Utc>>,
    nodes: HashMap<u64, ExoloomNode>,
    rootNodeId: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ExoloomNode {
    content: String,
    authorType: ExoloomAuthorType,
    authorName: Option<String>,
    parentId: Option<u64>,
    childrenIds: Vec<u64>,
    createdAt: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ExoloomAuthorType {
    USER,
    LLM,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ExoloomLens {
    bookmarks: Vec<ExoloomBookmark>,
    prunedNodes: Vec<ExoloomPrunedNode>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ExoloomBookmark {
    nodeId: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ExoloomPrunedNode {
    nodeId: u64,
}
