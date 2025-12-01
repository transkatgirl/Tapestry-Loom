use std::{fmt::Display, path::PathBuf, sync::Arc, time::Duration};

use eframe::egui::{
    ComboBox, Context, Frame, ScrollArea, Slider, SliderClamping, TextStyle, Ui, Visuals,
};
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{dependent::DependentNode, indexmap::IndexMap},
    v0::{InnerNodeContent, NodeContent},
};
use tokio::{runtime::Runtime, task::JoinHandle};

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct InferenceSettings {
    accept_invalid_https: bool,
    https_only: bool,
    user_agent: String,
}

impl InferenceSettings {
    pub(super) fn render(&mut self, ui: &mut Ui) {}
    pub fn build_client(&self) -> Result<Client, reqwest::Error> {
        ClientBuilder::new()
            .connect_timeout(Duration::from_secs(15))
            .https_only(self.https_only)
            .danger_accept_invalid_certs(self.accept_invalid_https)
            .danger_accept_invalid_hostnames(self.accept_invalid_https)
            .user_agent(&self.user_agent)
            .build()
    }
}

pub struct InferenceRequest {
    pub content: Vec<u8>,
    pub model: Ulid,
    pub parameters: IndexMap<String, String>,
}

pub struct InferenceParameters {
    pub models: IndexMap<Ulid, ModelInferenceParameters>,
    pub read_timeout_secs: f32,
    pub recursion_depth: usize,
}

pub struct ModelInferenceParameters {
    pub requests: usize,
    pub parameters: IndexMap<String, String>,
}

impl InferenceParameters {
    pub fn reset(&mut self, settings: &InferenceSettings) {}
    pub fn render(&mut self, settings: &InferenceSettings) {}
    pub fn perform_request(
        &mut self,
        settings: &InferenceSettings,
        runtime: Arc<Runtime>,
        parent_node: Option<Ulid>,
    ) -> Vec<JoinHandle<Result<DependentNode<NodeContent>, anyhow::Error>>> {
        todo!()
    }
}
