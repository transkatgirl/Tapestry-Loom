#![allow(non_snake_case)]

use std::{
    collections::{HashMap, HashSet},
    hash::BuildHasherDefault,
    num::NonZeroU128,
    sync::Arc,
};

use base64::prelude::*;
use boa_engine::{Context, JsString, Source, js_string, property::Attribute};
use frontmatter::{Yaml, parse_and_find_content};
use miniz_oxide::inflate::decompress_to_vec_zlib;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tapestry_weave::{
    VersionedWeave, getrandom,
    hashers::{RandomIdHasher, UlidHasher},
    jiff::{Timestamp, Zoned},
    ulid::Ulid,
    universal_weave::{
        dependent::DependentNode,
        indexmap::{IndexMap, IndexSet},
    },
    v1::{
        content::{Creator, InnerNodeContent, InnerNodeToken, Model, NodeContent, OriginalToken},
        metadata::MetadataMap,
    },
    wrappers::UniqueIdentifierRemapper,
};

use crate::new_weave;

pub fn migrate(input: &str, created: Zoned) -> anyhow::Result<Option<VersionedWeave>> {
    if let Ok((Some(Yaml::Hash(mut frontmatter)), _)) = parse_and_find_content(input) {
        let weave = if let Some(Yaml::String(compressed_weave)) =
            frontmatter.remove(&Yaml::String("TapestryLoomWeaveCompressed".to_string()))
        {
            Some(String::from_utf8(
                decompress_to_vec_zlib(&BASE64_STANDARD.decode(compressed_weave)?)
                    .map_err(|e| anyhow::Error::msg(format!("{}", e)))?,
            )?)
        } else if let Some(Yaml::String(decompressed_weave)) =
            frontmatter.remove(&Yaml::String("TapestryLoomWeave".to_string()))
        {
            Some(decompressed_weave)
        } else {
            None
        };

        if let Some(weave) = weave {
            return Ok(Some(convert_weave(weave, created)?));
        }
    }

    Ok(None)
}

fn convert_weave(input: String, created: Zoned) -> anyhow::Result<VersionedWeave> {
    let mut context = Context::default();

    context
        .register_global_property(
            js_string!("input_data"),
            JsString::from(input),
            Attribute::READONLY,
        )
        .unwrap();

    let output = context
        .eval(Source::from_bytes(include_bytes!("convert.js")))
        .map_err(|e| anyhow::Error::msg(format!("{}", e)))?
        .as_string()
        .ok_or(anyhow::Error::msg(
            "Incorrect return type from conversion script",
        ))?
        .to_std_string()?;

    let mut input: LegacyWeave = serde_json::from_value(serde_json::from_str::<Value>(&output)?)?;

    input.sort();

    let mut input_nodes = Vec::with_capacity(input.nodes.len());

    for node in &input.rootNodes {
        input.build_node_list(*node, &mut input_nodes);
    }

    let mut output = new_weave(input.nodes.len(), created, "LegacyTapestryLoom", None);

    let mut mapper: UniqueIdentifierRemapper<
        u128,
        u64,
        BuildHasherDefault<UlidHasher>,
        BuildHasherDefault<RandomIdHasher>,
    > = UniqueIdentifierRemapper::with_capacity(input.nodes.len());

    let mut convert_old_identifier = move |id| {
        *mapper
            .try_map_with_initial(
                id,
                unsafe { std::mem::transmute::<u128, [u64; 2]>(id)[1] },
                getrandom::u64,
            )
            .unwrap()
            .get()
    };

    let time_zone = output.metadata().created.time_zone().clone();

    let empty_counterfactual = Arc::new(Vec::new());

    for node in input_nodes {
        if let Some(node) = input.nodes.get(&node).cloned() {
            let timestamp = Timestamp::try_from(Ulid(node.identifier.0).datetime())
                .map(|timestamp| Zoned::new(timestamp, time_zone.clone()))
                .unwrap_or(Zoned::default());

            assert!(
                output.add_node(DependentNode {
                    id: convert_old_identifier(node.identifier.0),
                    from: node
                        .parentNode
                        .map(|id| convert_old_identifier(id.0))
                        .and_then(|id| if output.contains(&id) {
                            Some(id)
                        } else {
                            eprintln!("Warning: Node {} has missing parents", node.identifier);
                            None
                        }),
                    to: IndexSet::default(),
                    active: input.currentNode == Some(node.identifier),
                    bookmarked: input.bookmarks.contains(&node.identifier),
                    contents: NodeContent {
                        timestamp,
                        modified: false,
                        content: match node.content {
                            LegacyNodeContent::Snippet(snippet) =>
                                InnerNodeContent::Snippet(snippet.into_bytes()),
                            LegacyNodeContent::Tokens(tokens) => InnerNodeContent::Tokens(
                                tokens
                                    .into_iter()
                                    .map(|(probability, token)| {
                                        InnerNodeToken {
                                            bytes: token.into_bytes(),
                                            logprob: probability.ln() as f32,
                                            id: None,
                                            metadata: IndexMap::default(),
                                            entropy: None,
                                            counterfactual: empty_counterfactual.clone(),
                                            original: OriginalToken::Unmodified,
                                        }
                                    })
                                    .collect()
                            ),
                        },
                        metadata: node.parameters.unwrap_or_default(),
                        creator: node
                            .model
                            .map(
                                |id| Creator::Model(input.models.get(&id).cloned().map(|model| {
                                    Model {
                                        label: model.label,
                                        color: model.color,
                                        identifier: NonZeroU128::try_from(id.0).ok(),
                                        metadata: IndexMap::default(),
                                        seed: None,
                                        raw_query: None,
                                    }
                                }))
                            )
                            .unwrap_or(Creator::Unknown)
                    }
                })
            );
        }
    }

    Ok(output.to_versioned_weave())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LegacyWeave {
    identifier: Ulid,
    models: HashMap<Ulid, LegacyModelLabel>,
    modelNodes: HashMap<Ulid, HashSet<Ulid>>,
    nodes: HashMap<Ulid, LegacyDocumentNode>,
    rootNodes: Vec<Ulid>,
    nodeChildren: HashMap<Ulid, Vec<Ulid>>,
    currentNode: Option<Ulid>,
    bookmarks: HashSet<Ulid>,
}

impl LegacyWeave {
    fn sort(&mut self) {
        let mut roots = self
            .rootNodes
            .iter()
            .filter_map(|id| self.nodes.get(id))
            .collect();
        sort_node_list(&mut roots);
        self.rootNodes = roots.into_iter().map(|node| node.identifier).collect();

        for (_, children) in self.nodeChildren.iter_mut() {
            let mut children_nodes = children
                .iter()
                .filter_map(|id| self.nodes.get(id))
                .collect();
            sort_node_list(&mut children_nodes);
            *children = children_nodes
                .into_iter()
                .map(|node| node.identifier)
                .collect();
        }
    }
    fn build_node_list(&self, id: Ulid, nodes: &mut Vec<Ulid>) {
        nodes.push(id);

        if let Some(children) = self.nodeChildren.get(&id) {
            for child in children {
                self.build_node_list(*child, nodes);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LegacyModelLabel {
    label: String,
    color: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LegacyDocumentNode {
    identifier: Ulid,
    content: LegacyNodeContent,
    model: Option<Ulid>,
    parentNode: Option<Ulid>,
    parameters: Option<MetadataMap>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
enum LegacyNodeContent {
    Snippet(String),
    Tokens(Vec<(f64, String)>),
}

fn sort_node_list(nodes: &mut Vec<&LegacyDocumentNode>) {
    nodes.sort_unstable_by(|a, b| {
        let a_tokens = if let LegacyNodeContent::Tokens(tokens) = &a.content {
            Some(tokens)
        } else {
            None
        };
        let b_tokens = if let LegacyNodeContent::Tokens(tokens) = &b.content {
            Some(tokens)
        } else {
            None
        };

        let x = a_tokens.map(|t| t.len()).unwrap_or_default() == 1;
        let y = b_tokens.map(|t| t.len()).unwrap_or_default() == 1;

        if x && y {
            a.model.cmp(&b.model).then(
                b_tokens.unwrap()[0]
                    .partial_cmp(&a_tokens.unwrap()[0])
                    .unwrap(),
            )
        } else {
            a.model
                .cmp(&b.model)
                .then(y.cmp(&x))
                .then(a.identifier.cmp(&b.identifier))
        }
    });
}
