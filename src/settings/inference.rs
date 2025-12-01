use std::{fmt::Display, path::PathBuf, time::Duration};

use eframe::egui::{
    ComboBox, Context, Frame, ScrollArea, Slider, SliderClamping, TextStyle, Ui, Visuals,
};
use serde::{Deserialize, Serialize};
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::indexmap::IndexMap,
    v0::{InnerNodeContent, NodeContent},
};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct InferenceSettings {}

impl InferenceSettings {
    pub(super) fn render(&mut self, ui: &mut Ui) {}
    pub async fn perform_inference(
        &self,
        request: InferenceRequest,
    ) -> Result<NodeContent, anyhow::Error> {
        todo!()
    }
}

pub struct InferenceRequest {
    pub content: Vec<u8>,
    pub model: Ulid,
    pub parameters: IndexMap<String, String>,
}

pub struct InferenceParameters {
    pub models: IndexMap<Ulid, ModelInferenceParameters>,
    pub timeout_secs: f32,
}

pub struct ModelInferenceParameters {
    pub requests_per_iteration: usize,
    pub recursion_depth: usize,
    pub parameters: IndexMap<String, String>,
}

impl InferenceParameters {
    pub fn reset(&mut self, settings: &InferenceSettings) {}
    pub fn render(&mut self, settings: &InferenceSettings) {}
}
