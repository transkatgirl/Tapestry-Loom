use std::sync::Arc;

use egui_notify::Toasts;
use tapestry_weave::{ulid::Ulid, v0::TapestryWeave};
use tokio::runtime::Runtime;

use crate::settings::Settings;

#[derive(Debug)]
pub struct SharedState {
    pub identifier: Ulid,
    pub runtime: Arc<Runtime>,
}

impl SharedState {
    pub fn new(identifier: Ulid, runtime: Arc<Runtime>) -> Self {
        Self {
            identifier,
            runtime,
        }
    }
    pub fn update(&mut self, weave: &mut TapestryWeave, settings: &Settings, toasts: &mut Toasts) {}
    pub fn reset(&mut self) {}
    pub fn run_inference(
        &mut self,
        weave: &mut TapestryWeave,
        parent: Option<Ulid>,
        settings: &Settings,
    ) {
    }
}
