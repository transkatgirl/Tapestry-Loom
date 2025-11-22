use std::{
    iter,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use serde::Deserialize;
use serde_json::Value;
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
    GetNodes(Vec<Ulid>),
    GetRoots,
    GetBookmarks,
    GetActiveThread,
    AddNode(Box<DependentNode<NodeContent>>),
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
            todo!()
        }
        IncomingMessage::GetNodes(nodes) => {
            todo!()
        }
        IncomingMessage::GetRoots => {
            todo!()
        }
        IncomingMessage::GetBookmarks => {
            todo!()
        }
        IncomingMessage::GetActiveThread => {
            todo!()
        }
        IncomingMessage::AddNode(node) => {
            todo!()
        }
        IncomingMessage::SetNodeActiveStatus((id, value)) => {
            todo!()
        }
        IncomingMessage::SetNodeBookmarkedStatus((id, value)) => {
            todo!()
        }
        IncomingMessage::SetActiveContent((value, metadata)) => {
            todo!()
        }
        IncomingMessage::SplitNode((id, at)) => {
            todo!()
        }
        IncomingMessage::MergeNodeWithParent(id) => {
            todo!()
        }
        IncomingMessage::IsNodeMergeableWithParent(id) => {
            todo!()
        }
        IncomingMessage::RemoveNode(id) => {
            todo!()
        }
    }
}
