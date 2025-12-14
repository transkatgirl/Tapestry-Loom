#![allow(non_snake_case)]

use std::collections::{HashMap, HashSet};

use base64::prelude::*;
use boa_engine::{Context, JsString, Source, js_string, property::Attribute};
use chrono::{DateTime, offset};
use frontmatter::{Yaml, parse_and_find_content};
use miniz_oxide::inflate::decompress_to_vec_zlib;
use serde::{Deserialize, Serialize};
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{
        dependent::DependentNode,
        indexmap::{IndexMap, IndexSet},
    },
    v0::{InnerNodeContent, Model, NodeContent, TapestryWeave},
};

pub fn migrate(input: &str, created: DateTime<offset::Local>) -> anyhow::Result<Option<Vec<u8>>> {
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

fn convert_weave(input: String, created: DateTime<offset::Local>) -> anyhow::Result<Vec<u8>> {
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

    let mut input: LegacyWeave = serde_json::from_str(&output)?;

    input.sort();

    let mut input_nodes = Vec::with_capacity(input.nodes.len());

    for node in &input.rootNodes {
        input.build_node_list(*node, &mut input_nodes);
    }

    let mut output = TapestryWeave::with_capacity(
        input.nodes.len(),
        IndexMap::from([
            (
                "converted_from".to_string(),
                "LegacyTapestryLoom".to_string(),
            ),
            ("created".to_string(), created.to_rfc3339()),
        ]),
    );

    for node in input_nodes {
        if let Some(node) = input.nodes.get(&node).cloned() {
            output.add_node(DependentNode {
                id: node.identifier.0,
                from: node.parentNode.map(|id| id.0),
                to: IndexSet::default(),
                active: input.currentNode == Some(node.identifier),
                bookmarked: input.bookmarks.contains(&node.identifier),
                contents: NodeContent {
                    content: match node.content {
                        LegacyNodeContent::Snippet(snippet) => {
                            InnerNodeContent::Snippet(snippet.into_bytes())
                        }
                        LegacyNodeContent::Tokens(tokens) => InnerNodeContent::Tokens(
                            tokens
                                .into_iter()
                                .map(|(probability, token)| {
                                    (
                                        token.into_bytes(),
                                        IndexMap::from([(
                                            "probability".to_string(),
                                            probability.to_string(),
                                        )]),
                                    )
                                })
                                .collect(),
                        ),
                    },
                    metadata: node.parameters.unwrap_or_default(),
                    model: node
                        .model
                        .and_then(|id| input.models.get(&id).cloned())
                        .map(|model| Model {
                            label: model.label,
                            metadata: if let Some(color) = model.color {
                                IndexMap::from([("color".to_string(), color)])
                            } else {
                                IndexMap::default()
                            },
                        }),
                },
            });
        }
    }

    Ok(output.to_versioned_bytes()?)
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
    parameters: Option<IndexMap<String, String>>,
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
