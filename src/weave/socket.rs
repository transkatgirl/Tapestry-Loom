use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
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
    content: MessageNodeContent,
    metadata: IndexMap<String, String>,
    model: Option<Model>,
}

impl From<IncomingAddNode> for DependentNode<NodeContent> {
    fn from(value: IncomingAddNode) -> Self {
        DependentNode {
            #[allow(clippy::unwrap_or_default)]
            id: value.id.unwrap_or_else(Ulid::new).0,
            from: value.from.map(|u| u.0),
            to: IndexSet::default(),
            active: value.active,
            bookmarked: value.bookmarked,
            contents: NodeContent {
                content: value.content.into(),
                metadata: value.metadata,
                model: value.model,
            },
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
enum MessageNodeContent {
    Snippet(String),
    Tokens(Vec<MessageNodeContentToken>),
}

#[derive(Deserialize, Serialize, Debug)]
struct MessageNodeContentToken {
    label: String,
    metadata: IndexMap<String, String>,
}

impl From<InnerNodeContent> for MessageNodeContent {
    fn from(value: InnerNodeContent) -> Self {
        match value {
            InnerNodeContent::Snippet(snippet) => {
                Self::Snippet(String::from_utf8_lossy(&snippet).to_string())
            }
            InnerNodeContent::Tokens(tokens) => Self::Tokens(
                tokens
                    .into_iter()
                    .map(|t| MessageNodeContentToken {
                        label: String::from_utf8_lossy(&t.0).to_string(),
                        metadata: t.1,
                    })
                    .collect(),
            ),
        }
    }
}

impl From<MessageNodeContent> for InnerNodeContent {
    fn from(value: MessageNodeContent) -> Self {
        match value {
            MessageNodeContent::Snippet(snippet) => Self::Snippet(snippet.into_bytes()),
            MessageNodeContent::Tokens(tokens) => Self::Tokens(
                tokens
                    .into_iter()
                    .map(|t| (t.label.into_bytes(), t.metadata))
                    .collect(),
            ),
        }
    }
}

#[derive(Serialize, Debug)]
struct OutgoingNode {
    id: Ulid,
    from: Option<Ulid>,
    to: IndexSet<Ulid>,
    active: bool,
    bookmarked: bool,
    content: MessageNodeContent,
    metadata: IndexMap<String, String>,
    model: Option<Model>,
}

impl From<DependentNode<NodeContent>> for OutgoingNode {
    fn from(value: DependentNode<NodeContent>) -> Self {
        Self {
            id: Ulid(value.id),
            from: value.from.map(Ulid),
            to: IndexSet::from_iter(value.to.into_iter().map(Ulid)),
            active: value.active,
            bookmarked: value.bookmarked,
            content: value.contents.content.into(),
            metadata: value.contents.metadata,
            model: value.contents.model,
        }
    }
}

#[derive(Deserialize, Debug)]
enum IncomingMessage {
    GetNodeCount,
    IsChanged,
    GetMetadata,
    SetMetadata(IndexMap<String, String>),
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

#[derive(Serialize, Debug)]
enum OutgoingMessage {
    GetNodeCount(usize),
    IsChanged(bool),
    GetMetadata(IndexMap<String, String>),
    SetMetadata,
    GetNode(Box<Option<OutgoingNode>>),
    GetNodes(Vec<OutgoingNode>),
    GetRoots(Vec<OutgoingNode>),
    GetBookmarks(Vec<OutgoingNode>),
    GetActiveThread(Vec<OutgoingNode>),
    AddNode(Option<Ulid>),
    SetNodeActiveStatus(bool),
    SetNodeBookmarkedStatus(bool),
    SetActiveContent(bool),
    SplitNode(Option<Ulid>),
    MergeNodeWithParent(bool),
    IsNodeMergeableWithParent(bool),
    RemoveNode(bool),
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
) -> Result<(OutgoingMessage, bool), serde_json::Error> {
    match message {
        IncomingMessage::GetNodeCount => Ok((OutgoingMessage::GetNodeCount(weave.len()), false)),
        IncomingMessage::IsChanged => Ok((OutgoingMessage::IsChanged(has_changed), false)),
        IncomingMessage::GetMetadata => Ok((
            OutgoingMessage::GetMetadata(weave.weave.metadata.clone()),
            false,
        )),
        IncomingMessage::SetMetadata(metadata) => {
            weave.weave.metadata = metadata;
            Ok((OutgoingMessage::SetMetadata, true))
        }
        IncomingMessage::GetNode(id) => {
            let node = weave.get_node(&id).cloned().map(OutgoingNode::from);

            Ok((OutgoingMessage::GetNode(Box::new(node)), false))
        }
        IncomingMessage::GetNodes(nodes) => {
            let nodes: Vec<OutgoingNode> = nodes
                .into_iter()
                .filter_map(|id| weave.get_node(&id).cloned().map(OutgoingNode::from))
                .collect();

            Ok((OutgoingMessage::GetNodes(nodes), false))
        }
        IncomingMessage::GetRoots => {
            let roots: Vec<OutgoingNode> = weave
                .get_roots()
                .filter_map(|id| weave.get_node(&id).cloned().map(OutgoingNode::from))
                .collect();

            Ok((OutgoingMessage::GetRoots(roots), false))
        }
        IncomingMessage::GetBookmarks => {
            let bookmarks: Vec<OutgoingNode> = weave
                .get_bookmarks()
                .filter_map(|id| weave.get_node(&id).cloned().map(OutgoingNode::from))
                .collect();

            Ok((OutgoingMessage::GetBookmarks(bookmarks), false))
        }
        IncomingMessage::GetActiveThread => {
            let active: Vec<OutgoingNode> = weave
                .get_active_thread()
                .map(|node| OutgoingNode::from(node.clone()))
                .collect();

            Ok((OutgoingMessage::GetActiveThread(active), false))
        }
        IncomingMessage::AddNode(node) => {
            let node: DependentNode<NodeContent> = node.into();
            let identifier = node.id;
            let result = weave.add_node(node);

            Ok((
                OutgoingMessage::AddNode(if result { Some(Ulid(identifier)) } else { None }),
                true,
            ))
        }
        IncomingMessage::SetNodeActiveStatus((id, value)) => {
            let result = weave.set_node_active_status(&id, value);

            Ok((OutgoingMessage::SetNodeActiveStatus(result), true))
        }
        IncomingMessage::SetNodeBookmarkedStatus((id, value)) => {
            let result = weave.set_node_bookmarked_status(&id, value);

            Ok((OutgoingMessage::SetNodeBookmarkedStatus(result), true))
        }
        IncomingMessage::SetActiveContent((value, metadata)) => {
            let result = weave.set_active_content(&value, metadata, |t| match t {
                Some(t) => Ulid::from_datetime(SystemTime::UNIX_EPOCH + Duration::from_millis(t)),
                None => Ulid::new(),
            });

            Ok((OutgoingMessage::SetActiveContent(result), true))
        }
        IncomingMessage::SplitNode((id, at)) => {
            let result = weave.split_node(&id, at, |t| {
                Ulid::from_datetime(SystemTime::UNIX_EPOCH + Duration::from_millis(t))
            });

            Ok((OutgoingMessage::SplitNode(result), true))
        }
        IncomingMessage::MergeNodeWithParent(id) => {
            let result = weave.merge_with_parent(&id);

            Ok((OutgoingMessage::MergeNodeWithParent(result), true))
        }
        IncomingMessage::IsNodeMergeableWithParent(id) => {
            let value = weave.is_mergeable_with_parent(&id);

            Ok((OutgoingMessage::IsNodeMergeableWithParent(value), false))
        }
        IncomingMessage::RemoveNode(id) => {
            let result = weave.remove_node(&id);

            Ok((OutgoingMessage::RemoveNode(result.is_some()), true))
        }
    }
}
