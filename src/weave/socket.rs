use std::{
    iter,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use serde::Deserialize;
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{dependent::DependentNode, indexmap::IndexMap},
    v0::{NodeContent, TapestryWeave},
};
use ws::Message;

#[derive(Deserialize, Debug)]
enum IncomingMessage {
    GetLength,
    IsChanged,
    GetNode(Ulid),
    GetRoots,
    GetBookmarks,
    GetActiveThread,
    AddNode(Box<DependentNode<NodeContent>>),
    SetNodeActiveStatus((Ulid, bool)),
    SetNodeBookmarkedStatus((Ulid, bool)),
    SetActiveContent((Ulid, IndexMap<String, String>)),
    SplitNode((Ulid, usize)),
    MergeNodeWithParent(Ulid),
    IsNodeMergeableWithParent(Ulid),
    RemoveNode(Ulid),
}

pub fn handle_message(
    weave: &mut TapestryWeave,
    has_changed: bool,
    input: Message,
) -> (Vec<Message>, bool) {
    (vec![input], false)
}
