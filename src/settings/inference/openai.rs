use std::iter;

use eframe::egui::Ui;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tapestry_weave::universal_weave::indexmap::IndexMap;

use crate::settings::inference::{Endpoint, EndpointRequest, EndpointResponse, Template};

#[derive(Default, Debug, Clone, PartialEq)]
pub(super) struct OpenAICompletionsTemplate {
    endpoint: String,
    model: String,
    api_key: String,
}

impl Template<OpenAICompletionsConfig> for OpenAICompletionsTemplate {
    fn render(&mut self, ui: &mut Ui) {}
    fn build(self) -> OpenAICompletionsConfig {
        todo!()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(super) struct OpenAICompletionsConfig {
    endpoint: String,
    headers: IndexMap<String, String>,
    parameters: IndexMap<String, String>,
}

impl Endpoint for OpenAICompletionsConfig {
    fn render_settings(&mut self, ui: &mut Ui) {
        todo!()
    }
    async fn perform_request(
        &self,
        client: &Client,
        request: EndpointRequest,
    ) -> Result<EndpointResponse, anyhow::Error> {
        todo!()
    }
}
