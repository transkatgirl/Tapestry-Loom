use std::{
    collections::{HashMap, hash_map::Entry},
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use anyhow::Error;
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

pub fn update_message(weave: &TapestryWeave) -> Message {
    Message::Text("start".to_string()) // TODO
}

pub fn handle_message(weave: &mut TapestryWeave, input: Message) -> Message {
    input // TODO
}
