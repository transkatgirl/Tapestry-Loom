use std::{collections::HashSet, fmt::Display, iter, path::PathBuf, sync::Arc, time::Duration};

use eframe::egui::{
    ComboBox, Context, Frame, RichText, ScrollArea, Slider, SliderClamping, TextEdit, TextStyle,
    Ui, Visuals, Widget, WidgetText,
};
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{dependent::DependentNode, indexmap::IndexMap},
    v0::{InnerNodeContent, NodeContent},
};
use tokio::{runtime::Runtime, task::JoinHandle};

use crate::settings::inference::openai::{OpenAICompletionsConfig, OpenAICompletionsTemplate};

mod openai;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct InferenceSettings {
    pub client: ClientConfig,
    models: IndexMap<Ulid, EndpointConfig>,

    #[serde(skip)]
    template: EndpointTemplate,
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
        ui.group(|ui| {
            self.template.render(ui);
            if ui.button("Add model").clicked()
                && let Some(model) = self.template.build()
            {
                self.models.insert(Ulid::new(), model);
            }
        });
        for (_, model) in &mut self.models {
            ui.group(|ui| {
                model.render_settings(ui);
            });
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
    pub parameters: Vec<(String, String)>,
}

impl Default for ModelInferenceParameters {
    fn default() -> Self {
        Self {
            requests: 10,
            parameters: vec![("temperature".to_string(), "1".to_string())],
        }
    }
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

#[derive(Default, Debug, PartialEq)]
enum EndpointTemplate {
    #[default]
    None,
    OpenAICompletions(OpenAICompletionsTemplate),
}

impl EndpointTemplate {
    fn render(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            let combobox_label = ui.label("Template:");
            ComboBox::from_id_salt("endpoint_template_chooser")
                .selected_text(format!("{}", self))
                .show_ui(ui, |ui| {
                    let templates = vec![
                        Self::None,
                        Self::OpenAICompletions(OpenAICompletionsTemplate::default()),
                    ];

                    for template in templates {
                        let template_label = template.to_string();
                        ui.selectable_value(self, template, template_label);
                    }
                })
                .response
                .labelled_by(combobox_label.id);
        });

        match self {
            Self::None => {}
            Self::OpenAICompletions(template) => template.render(ui),
        }
    }
    fn build(&mut self) -> Option<EndpointConfig> {
        match self {
            Self::None => None,
            Self::OpenAICompletions(template) => {
                let endpoint = template.clone().build();
                *self = EndpointTemplate::None;

                Some(EndpointConfig::OpenAICompletions(endpoint))
            }
        }
    }
}

impl Display for EndpointTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str("Choose Template..."),
            Self::OpenAICompletions(_) => f.write_str("OpenAI Completions"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]

enum EndpointConfig {
    OpenAICompletions(OpenAICompletionsConfig),
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
    parameters: Vec<(String, String)>,
}

struct EndpointResponse {
    content: InnerNodeContent,
    metadata: Vec<(String, String)>,
}

trait Endpoint: Serialize + DeserializeOwned + Clone {
    fn render_settings(&mut self, ui: &mut Ui);
    async fn perform_request(
        &self,
        client: &Client,
        request: EndpointRequest,
    ) -> Result<EndpointResponse, anyhow::Error>;
}

trait Template<T>: Default + Clone
where
    T: Endpoint,
{
    fn render(&mut self, ui: &mut Ui);
    fn build(self) -> T;
}

fn render_config_map(ui: &mut Ui, value: &mut Vec<(String, String)>) {
    let mut remove = None;

    for (index, (key, value)) in value.iter_mut().enumerate() {
        ui.horizontal_wrapped(|ui| {
            TextEdit::singleline(key).hint_text("key").ui(ui);
            TextEdit::singleline(value).hint_text("value").ui(ui);
            if ui.button("\u{E28F}").on_hover_text("Remove item").clicked() {
                remove = Some(index);
            }
        });
    }

    if let Some(remove) = remove {
        value.remove(remove);
    }

    if ui.button("\u{E13D}").on_hover_text("Add item").clicked() {
        value.push((String::new(), String::new()));
    }
}
