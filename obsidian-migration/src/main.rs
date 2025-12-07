#![allow(non_snake_case)]

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use base64::prelude::*;
use boa_engine::{Context, JsString, Source, js_string, property::Attribute};
use clap::Parser;
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
use walkdir::WalkDir;

#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Folder to scan for weaves created by Tapestry Loom v0
    #[arg(short, long)]
    input: PathBuf,

    /// Folder to output migrated weaves
    #[arg(short, long)]
    output: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    fs::create_dir_all(&args.output)?;

    for entry in WalkDir::new(&args.input) {
        let entry = entry?;
        if entry.file_type().is_file()
            && let Some(extension) = entry.path().extension()
            && extension.to_ascii_lowercase().to_str() == Some("md")
        {
            let mut output = if let Ok(stripped_path) = entry.path().strip_prefix(&args.input) {
                args.output.clone().join(stripped_path)
            } else {
                args.output.clone().join(entry.file_name())
            };
            output.set_extension("tapestry");

            if let Some(parent) = output.parent() {
                fs::create_dir_all(&parent)?;
            }

            migrate_weave(entry.path(), &output)?;
        }
    }

    Ok(())
}

fn migrate_weave(input_path: &Path, output_path: &Path) -> anyhow::Result<()> {
    assert_ne!(input_path, output_path);

    let input = fs::read_to_string(input_path)?;

    if let Ok((Some(Yaml::Hash(mut frontmatter)), _)) = parse_and_find_content(&input) {
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
            println!("{} -> {}", input_path.display(), output_path.display());

            fs::write(output_path, convert_weave(weave)?)?;
        } else {
            println!("Skipping {}", input_path.display());
        }
    } else {
        println!("Skipping {}", input_path.display());
    }

    Ok(())
}

fn convert_weave(input: String) -> anyhow::Result<Vec<u8>> {
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

    let input: LegacyWeave = serde_json::from_str(&output)?;

    let mut input_nodes = Vec::with_capacity(input.nodes.len());

    for node in &input.rootNodes {
        input.build_node_list(*node, &mut input_nodes);
    }

    let mut output = TapestryWeave::with_capacity(
        input.nodes.len(),
        IndexMap::from([(
            "converted_from".to_string(),
            "LegacyTapestryLoom".to_string(),
        )]),
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
    rootNodes: HashSet<Ulid>,
    nodeChildren: HashMap<Ulid, HashSet<Ulid>>,
    currentNode: Option<Ulid>,
    bookmarks: HashSet<Ulid>,
}

impl LegacyWeave {
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
