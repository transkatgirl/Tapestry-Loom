use std::{collections::HashSet, fmt::Display, iter, path::PathBuf, sync::Arc, time::Duration};

use eframe::egui::{
    ComboBox, Context, Frame, RichText, ScrollArea, Slider, SliderClamping, TextEdit, TextStyle,
    Ui, Visuals, Widget, WidgetText,
};
use reqwest::{Client, ClientBuilder, Request, Response};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{dependent::DependentNode, indexmap::IndexMap},
    v0::{InnerNodeContent, NodeContent},
};
use tokio::{runtime::Runtime, task::JoinHandle};

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct InferenceSettings {
    pub client: ClientConfig,
    models: IndexMap<Ulid, EndpointConfig>,

    #[serde(skip)]
    new_model: EndpointTemplate,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ClientConfig {
    accept_invalid_tls: bool,
    user_agent: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            accept_invalid_tls: false,
            user_agent: "Tapestry Loom".to_string(),
        }
    }
}

impl ClientConfig {
    fn render(&mut self, ui: &mut Ui) {
        let user_agent_label = ui.label("User Agent:");
        ui.text_edit_singleline(&mut self.user_agent)
            .labelled_by(user_agent_label.id);
        let accept_invalid_tls_label = if self.accept_invalid_tls {
            RichText::new("Accept invalid TLS (dangerous)").color(ui.style().visuals.warn_fg_color)
        } else {
            RichText::new("Accept invalid TLS")
        };
        ui.checkbox(&mut self.accept_invalid_tls, accept_invalid_tls_label);
    }
    pub fn build(&self) -> Result<Client, reqwest::Error> {
        ClientBuilder::new()
            .connect_timeout(Duration::from_secs(15))
            .danger_accept_invalid_certs(self.accept_invalid_tls)
            .danger_accept_invalid_hostnames(self.accept_invalid_tls)
            .user_agent(&self.user_agent)
            .build()
    }
}

impl InferenceSettings {
    pub(super) fn render(&mut self, ui: &mut Ui) {
        self.client.render(ui);
        /*ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                TextEdit::singleline(&mut self.new_model_scratchpad.0)
                    .hint_text("Endpoint URL")
                    .ui(ui);
                ComboBox::from_id_salt("inference_settings.new_model_scratchpad")
                    .selected_text(format!("{}", self.new_model_scratchpad.1))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.new_model_scratchpad.1,
                            EndpointType::OpenAICompletions,
                            EndpointType::OpenAICompletions.to_string(),
                        );
                    });
                TextEdit::singleline(&mut self.new_model_scratchpad.2)
                    .hint_text("Model identifier (optional)")
                    .desired_width(ui.spacing().text_edit_width / 1.5)
                    .ui(ui);
                TextEdit::singleline(&mut self.new_model_scratchpad.3)
                    .hint_text("API key (optional)")
                    .desired_width(ui.spacing().text_edit_width / 1.5)
                    .ui(ui);
                if ui.button("Add model").clicked() {
                    // TODO
                };
            });
        });*/
        for (_, model) in &mut self.models {
            ui.group(|ui| {
                model.render_settings(ui);
            });
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq)]
enum EndpointTemplate {
    #[default]
    OpenAICompletions,
}

impl Display for EndpointTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenAICompletions => f.write_str("OpenAI Completions"),
        }
    }
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
    ) -> HashSet<Ulid, JoinHandle<Result<DependentNode<NodeContent>, anyhow::Error>>> {
        todo!()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]

enum EndpointConfig {
    OpenAICompletions(OpenAICompletionsConfig),
}

impl Default for EndpointConfig {
    fn default() -> Self {
        unimplemented!()
    }
}

impl Endpoint for EndpointConfig {
    fn render_settings(&mut self, ui: &mut Ui) {
        match self {
            Self::OpenAICompletions(endpoint) => endpoint.render_settings(ui),
        }
    }
    fn perform_request(
        &self,
        client: &Client,
        request: EndpointRequest,
    ) -> impl Future<Output = Result<EndpointResponse, anyhow::Error>> {
        match self {
            Self::OpenAICompletions(endpoint) => endpoint.perform_request(client, request),
        }
    }
}

struct EndpointRequest {
    content: Vec<u8>,
    parameters: IndexMap<String, String>,
}

struct EndpointResponse {
    content: InnerNodeContent,
    metadata: IndexMap<String, String>,
}

trait Endpoint: Serialize + DeserializeOwned + Clone + Default {
    fn render_settings(&mut self, ui: &mut Ui);
    async fn perform_request(
        &self,
        client: &Client,
        request: EndpointRequest,
    ) -> Result<EndpointResponse, anyhow::Error>;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct OpenAICompletionsConfig {
    endpoint: String,
    headers: IndexMap<String, String>,
    parameters: IndexMap<String, String>,
}

impl Default for OpenAICompletionsConfig {
    fn default() -> Self {
        Self {
            endpoint: "https://api.openai.com/v1/completions".to_string(),
            headers: IndexMap::from_iter(iter::once((
                "Authorization".to_string(),
                "Bearer YOUR_API_KEY".to_string(),
            ))),
            parameters: IndexMap::from_iter(iter::once((
                "model".to_string(),
                "code-davinci-002".to_string(),
            ))),
        }
    }
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
