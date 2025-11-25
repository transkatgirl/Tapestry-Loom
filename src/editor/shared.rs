use std::sync::Arc;

use tokio::runtime::Runtime;

#[derive(Debug)]
pub struct SharedState {
    runtime: Arc<Runtime>,
}

impl SharedState {
    pub fn new(runtime: Arc<Runtime>) -> Self {
        Self { runtime }
    }
    pub fn reset(&mut self) {}
}
