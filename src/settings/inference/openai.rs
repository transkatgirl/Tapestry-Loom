use eframe::egui::{TextEdit, Ui, Widget};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::settings::inference::{
    Endpoint, EndpointRequest, EndpointResponse, Template, render_config_map,
};

#[derive(Default, Debug, Clone, PartialEq)]
pub(super) struct OpenAICompletionsTemplate {
    endpoint: String,
    model: String,
    api_key: String,
}

impl Template<OpenAICompletionsConfig> for OpenAICompletionsTemplate {
    fn render(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            TextEdit::singleline(&mut self.endpoint)
                .hint_text("Endpoint URL")
                .ui(ui);
            TextEdit::singleline(&mut self.model)
                .hint_text("Model identifier (optional)")
                .desired_width(ui.spacing().text_edit_width / 1.5)
                .ui(ui);
            TextEdit::singleline(&mut self.api_key)
                .hint_text("API key (optional)")
                .desired_width(ui.spacing().text_edit_width / 1.5)
                .ui(ui);
        });
    }
    fn build(mut self) -> OpenAICompletionsConfig {
        OpenAICompletionsConfig {
            endpoint: if self.endpoint.is_empty() {
                "https://api.openai.com/v1/completions".to_string()
            } else {
                if !self.endpoint.ends_with("/v1/completions") {
                    self.endpoint.push_str("/v1/completions");
                }

                self.endpoint
            },
            parameters: if self.model.is_empty() {
                Vec::new()
            } else {
                vec![("model".to_string(), self.model)]
            },
            headers: if self.api_key.is_empty() {
                Vec::new()
            } else {
                vec![(
                    "Authorization".to_string(),
                    ["Bearer", &self.api_key].concat(),
                )]
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(super) struct OpenAICompletionsConfig {
    endpoint: String,
    parameters: Vec<(String, String)>,
    headers: Vec<(String, String)>,
}

impl Endpoint for OpenAICompletionsConfig {
    fn render_settings(&mut self, ui: &mut Ui) {
        TextEdit::singleline(&mut self.endpoint)
            .desired_width(ui.spacing().text_edit_width * 2.0)
            .ui(ui);

        ui.group(|ui| {
            ui.label("Request parameters:");
            render_config_map(ui, &mut self.parameters);
        });

        ui.group(|ui| {
            ui.label("Request headers:");
            render_config_map(ui, &mut self.headers);
        });
    }
    async fn perform_request(
        &self,
        client: &Client,
        request: EndpointRequest,
    ) -> Result<EndpointResponse, anyhow::Error> {
        todo!()
    }
}
