use std::{fmt::Display, path::PathBuf, time::Duration};

use eframe::egui::{
    ComboBox, Context, Frame, ScrollArea, Slider, SliderClamping, TextStyle, Ui, Visuals,
};
use serde::{Deserialize, Serialize};
use tapestry_weave::{ulid::Ulid, universal_weave::indexmap::IndexMap, v0::InnerNodeContent};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct InferenceSettings {}

impl InferenceSettings {
    pub(super) fn render(&mut self, ui: &mut Ui) {}
    pub async fn perform_inference(
        &self,
        request: InferenceRequest,
    ) -> Result<InferenceResponse, anyhow::Error> {
        todo!()
    }
}

pub struct InferenceRequest {
    model: Ulid,
    parameters: IndexMap<String, String>,
    content: Vec<u8>,
}

pub struct InferenceResponse {
    content: InnerNodeContent,
    metadata: IndexMap<String, String>,
}
