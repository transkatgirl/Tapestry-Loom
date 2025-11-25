use std::sync::Arc;

use tapestry_weave::ulid::Ulid;
use tokio::runtime::Runtime;

#[derive(Debug)]
pub struct SharedState {
    identifier: Ulid,
    runtime: Arc<Runtime>,
}

impl SharedState {
    pub fn new(identifier: Ulid, runtime: Arc<Runtime>) -> Self {
        Self {
            identifier,
            runtime,
        }
    }
    pub fn reset(&mut self) {}
}
