use std::time::{Duration, SystemTime};

use serde::Deserialize;
use serde_json::Value;
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{
        dependent::DependentNode,
        indexmap::{IndexMap, IndexSet},
    },
    v0::{InnerNodeContent, Model, NodeContent, TapestryWeave},
};
use ws::Message;

#[derive(Deserialize, Debug)]
struct IncomingAddNode {
    id: Option<Ulid>,
    from: Option<Ulid>,
    active: bool,
    bookmarked: bool,
    content: InnerNodeContent,
    metadata: IndexMap<String, String>,
    model: Option<Model>,
}

#[derive(Deserialize, Debug)]
enum IncomingMessage {
    GetLength,
    IsChanged,
    GetNode(Ulid),
    GetNodes(Vec<Ulid>),
    GetRoots,
    GetBookmarks,
    GetActiveThread,
    AddNode(IncomingAddNode),
    SetNodeActiveStatus((Ulid, bool)),
    SetNodeBookmarkedStatus((Ulid, bool)),
    SetActiveContent((String, IndexMap<String, String>)),
    SplitNode((Ulid, usize)),
    MergeNodeWithParent(Ulid),
    IsNodeMergeableWithParent(Ulid),
    RemoveNode(Ulid),
}

pub fn handle_message(
    weave: &mut TapestryWeave,
    has_changed: bool,
    input: Message,
) -> (Option<Message>, bool) {
    match input {
        Message::Text(text) => {
            if let Ok(message) = serde_json::from_str(&text) {
                match handle_incoming_message(weave, has_changed, message) {
                    Ok((outgoing, is_changed)) => match serde_json::to_string(&outgoing) {
                        Ok(outgoing) => (Some(Message::Text(outgoing)), is_changed),
                        Err(e) => {
                            eprintln!("{e:#?}");
                            (
                                Some(Message::Text(
                                    "{\"Error\": \"Unable to serialize response\"}".to_string(),
                                )),
                                false,
                            )
                        }
                    },
                    Err(e) => {
                        eprintln!("{e:#?}");
                        (
                            Some(Message::Text(
                                "{\"Error\": \"Unable to serialize response\"}".to_string(),
                            )),
                            false,
                        )
                    }
                }
            } else {
                (
                    Some(Message::Text(
                        "{\"Error\": \"Unable to deserialize request\"}".to_string(),
                    )),
                    false,
                )
            }
        }
        Message::Binary(binary) => {
            if let Ok(message) = serde_json::from_slice(&binary) {
                match handle_incoming_message(weave, has_changed, message) {
                    Ok((outgoing, is_changed)) => match serde_json::to_vec(&outgoing) {
                        Ok(outgoing) => (Some(Message::Binary(outgoing)), is_changed),
                        Err(e) => {
                            eprintln!("{e:#?}");
                            (
                                Some(Message::Text(
                                    "{\"Error\": \"Unable to serialize response\"}".to_string(),
                                )),
                                false,
                            )
                        }
                    },
                    Err(e) => {
                        eprintln!("{e:#?}");
                        (
                            Some(Message::Text(
                                "{\"Error\": \"Unable to serialize response\"}".to_string(),
                            )),
                            false,
                        )
                    }
                }
            } else {
                (
                    Some(Message::Text(
                        "{\"Error\": \"Unable to deserialize request\"}".to_string(),
                    )),
                    false,
                )
            }
        }
        Message::Ping(payload) => (Some(Message::Pong(payload)), false),
        _ => (None, false),
    }
}

fn handle_incoming_message(
    weave: &mut TapestryWeave,
    has_changed: bool,
    message: IncomingMessage,
) -> Result<(Value, bool), serde_json::Error> {
    match message {
        IncomingMessage::GetLength => Ok((Value::Number(weave.len().into()), false)),
        IncomingMessage::IsChanged => Ok((Value::Bool(has_changed), false)),
        IncomingMessage::GetNode(id) => {
            let node = weave.get_node(&id);

            serde_json::to_value(node).map(|v| (v, false))
        }
        IncomingMessage::GetNodes(nodes) => {
            let nodes: Vec<_> = nodes.into_iter().map(|id| weave.get_node(&id)).collect();

            serde_json::to_value(nodes).map(|v| (v, false))
        }
        IncomingMessage::GetRoots => {
            let roots: Vec<_> = weave
                .get_roots()
                .filter_map(|id| weave.get_node(&id))
                .collect();

            serde_json::to_value(roots).map(|v| (v, false))
        }
        IncomingMessage::GetBookmarks => {
            let bookmarks: Vec<_> = weave
                .get_bookmarks()
                .filter_map(|id| weave.get_node(&id))
                .collect();

            serde_json::to_value(bookmarks).map(|v| (v, false))
        }
        IncomingMessage::GetActiveThread => {
            let active: Vec<_> = weave.get_active_thread().collect();

            serde_json::to_value(active).map(|v| (v, false))
        }
        IncomingMessage::AddNode(node) => {
            let node = DependentNode {
                id: node.id.unwrap_or_else(Ulid::new).0,
                from: node.from.map(|u| u.0),
                to: IndexSet::default(),
                active: node.active,
                bookmarked: node.bookmarked,
                contents: NodeContent {
                    content: node.content,
                    metadata: node.metadata,
                    model: node.model,
                },
            };

            let result = weave.add_node(node);

            Ok((Value::Bool(result), true))
        }
        IncomingMessage::SetNodeActiveStatus((id, value)) => {
            let result = weave.set_node_active_status(&id, value);

            Ok((Value::Bool(result), true))
        }
        IncomingMessage::SetNodeBookmarkedStatus((id, value)) => {
            let result = weave.set_node_bookmarked_status(&id, value);

            Ok((Value::Bool(result), true))
        }
        IncomingMessage::SetActiveContent((value, metadata)) => {
            let result = weave.set_active_content(&value, metadata, |t| match t {
                Some(t) => Ulid::from_datetime(SystemTime::UNIX_EPOCH + Duration::from_millis(t)),
                None => Ulid::new(),
            });

            Ok((Value::Bool(result), true))
        }
        IncomingMessage::SplitNode((id, at)) => {
            let result = weave.split_node(&id, at, |t| {
                Ulid::from_datetime(SystemTime::UNIX_EPOCH + Duration::from_millis(t))
            });

            serde_json::to_value(result).map(|v| (v, true))
        }
        IncomingMessage::MergeNodeWithParent(id) => {
            let result = weave.merge_with_parent(&id);

            Ok((Value::Bool(result), true))
        }
        IncomingMessage::IsNodeMergeableWithParent(id) => {
            let value = weave.is_mergeable_with_parent(&id);

            Ok((Value::Bool(value), false))
        }
        IncomingMessage::RemoveNode(id) => {
            let result = weave.remove_node(&id);

            Ok((Value::Bool(result.is_some()), true))
        }
    }
}
