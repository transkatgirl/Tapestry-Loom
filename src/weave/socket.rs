use std::{
    collections::{HashMap, hash_map::Entry},
    iter,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use rocket::{
    State, delete,
    fs::relative,
    futures::{SinkExt, StreamExt, lock::Mutex},
    get,
    http::Status,
    serde::json::Json,
    tokio::{
        fs::{File, read_dir, remove_file, try_exists},
        io::{AsyncReadExt, AsyncWriteExt},
        sync::RwLock,
    },
};
use tapestry_weave::{VersionedWeave, universal_weave::indexmap::IndexMap, v0::TapestryWeave};
use ws::Message;

enum SocketMessage {
    Refresh,
}

pub fn handle_message(
    weave: &mut TapestryWeave,
    has_changed: bool,
    input: Message,
) -> (Vec<Message>, bool) {
    (vec![input], false)
}
