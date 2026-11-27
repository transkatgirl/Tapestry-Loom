use std::time::Duration;

use anyhow::anyhow;
use eframe::egui::{
    Color32, ComboBox, RichText, Slider, SliderClamping, TextStyle, Ui, WidgetText,
};
use log::trace;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use tapestry_weave::{LongId, MetadataMap, Model, universal_weave::indexmap::IndexMap};
use ulid::Ulid;

use super::{
    openai::{
        OpenAIChatCompletionsConfig, OpenAIChatCompletionsTemplate, OpenAICompletionsConfig,
        OpenAICompletionsTemplate, OpenAIEmbeddingsConfig,
        TapestryTokenizeOpenAICompletionsTemplate,
    },
    parameters::InferenceParameters,
    shared::{EndpointRequest, EndpointResponse, InferenceCache},
};
use crate::common::{
    ui::{colored_text, label_editor, list_item_buttons},
    view::Edit,
};

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
pub struct InferenceEngineSettings {
    pub client: ClientConfig,
    pub models: IndexMap<Ulid, InferenceModel>,
    pub embedding_model: EmbeddingEndpointConfig,

    pub default_parameters: InferenceParameters,
    pub parameter_presets: Vec<ParameterPreset>,

    #[serde(skip)]
    template: EndpointTemplate,
}

impl Edit for InferenceEngineSettings {
    fn ui(&mut self, ui: &mut Ui) {
        self.client.ui(ui);

        ui.group(|ui| {
            self.template.ui(ui);

            if self.template != EndpointTemplate::None
                && ui.button("Add model").clicked()
                && let Some(endpoint) = self.template.build()
            {
                let identifier = Ulid::generate();

                self.models.insert(
                    identifier,
                    InferenceModel {
                        label: String::new(),
                        color: None,
                        endpoint,
                        identifier,
                    },
                );
            }
        });

        if self.models.is_empty() {
            ui.label("Add your first model by filling out an inference template.");
        }

        let mut action = None;

        let length = self.models.len();
        for (index, (id, model)) in self.models.iter_mut().enumerate() {
            ui.group(|ui| {
                model.ui(ui, id);

                ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.75);

                list_item_buttons(ui, "model", index, length, &mut action);
            });
        }

        if let Some(action) = action {
            action.apply_indexmap(&mut self.models, |_| Ulid::generate());
        }

        ui.separator();
        ui.heading("Embedding inference");
        self.embedding_model.ui(ui);

        ui.separator();
        ui.heading("Editor inference defaults");
        self.default_parameters.ui_inner(&self.models, ui);

        ui.separator();
        ui.heading("Editor inference presets");
        ui.add_space(ui.spacing().icon_spacing);

        let mut action = None;

        let length = self.parameter_presets.len();
        for (index, preset) in self.parameter_presets.iter_mut().enumerate() {
            ui.group(|ui| {
                label_editor(ui, &mut preset.label, "Preset label", &mut preset.color);

                ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.75);

                preset.parameters.ui_inner(&self.models, ui);

                list_item_buttons(ui, "preset", index, length, &mut action);
            });
        }

        if let Some(action) = action {
            action.apply(&mut self.parameter_presets);
        }

        if ui.button("\u{E13D}").on_hover_text("Add preset").clicked() {
            self.parameter_presets
                .push(ParameterPreset::from_parameters(
                    self.default_parameters.clone(),
                ));
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct ClientConfig {
    accept_invalid_tls: bool,
    timeout_minutes: f32,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            accept_invalid_tls: false,
            timeout_minutes: 10.0,
        }
    }
}

impl ClientConfig {
    fn ui(&mut self, ui: &mut Ui) {
        let accept_invalid_tls_label = if self.accept_invalid_tls {
            RichText::new("Accept invalid TLS (dangerous)").color(ui.style().visuals.warn_fg_color)
        } else {
            RichText::new("Accept invalid TLS")
        };
        ui.checkbox(&mut self.accept_invalid_tls, accept_invalid_tls_label)
            .on_hover_text("Controls if TLS certificates are validated.\n\nYou should keep this setting disabled unless you know what you're doing, as accepting invalid certificates makes Man-In-The-Middle attacks trivial.");
        ui.add(
            Slider::new(&mut self.timeout_minutes, 1.0..=1440.0)
                .logarithmic(true)
                .clamping(SliderClamping::Never)
                .text("Request timeout")
                .suffix(" minutes"),
        ).on_hover_text("The maximum length of time to wait for a HTTP request to finish. Requests exceeding this duration will be dropped.");
    }
    pub fn build(&self) -> Result<Client, anyhow::Error> {
        Ok(ClientBuilder::new()
            .connect_timeout(Duration::from_secs(15))
            .danger_accept_invalid_certs(self.accept_invalid_tls)
            .danger_accept_invalid_hostnames(self.accept_invalid_tls)
            .timeout(Duration::try_from_secs_f32(self.timeout_minutes * 60.0)?)
            .build()?)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InferenceModel {
    label: String,
    color: Option<Color32>,
    pub endpoint: EndpointConfig,

    #[serde(default = "Ulid::generate")]
    pub identifier: Ulid,
}

impl InferenceModel {
    fn label(&self) -> &str {
        if self.label.is_empty() {
            self.endpoint.label()
        } else {
            &self.label
        }
    }
    pub fn widget_text(&self) -> WidgetText {
        colored_text(self.label(), self.color).into()
    }
    pub fn content_model(&self) -> Model {
        Model {
            label: self.label().to_string(),
            color: self.color.map(|color| color.to_hex()),
            identifier: LongId::new(self.identifier.0),
            seed: None,
            system_fingerprint: None,
            finish_reason: None,
            metadata: MetadataMap::default(),
        }
    }
    fn ui(&mut self, ui: &mut Ui, id: &Ulid) {
        label_editor(ui, &mut self.label, self.endpoint.label(), &mut self.color);

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.75);
        ui.label(["Endpoint Mode: ", self.endpoint.name()].concat());

        let old = self.endpoint.clone();

        self.endpoint.ui(ui, id);

        if self.endpoint != old {
            trace!("Updating identifier for {}", id);
            self.identifier = Ulid::generate();
        }
    }
}

#[derive(Default, Debug, PartialEq)]
enum EndpointTemplate {
    #[default]
    None,
    OpenAICompletions(OpenAICompletionsTemplate),
    OpenAIChatCompletions(OpenAIChatCompletionsTemplate),
    TapestryTokenizeOpenAICompletions(TapestryTokenizeOpenAICompletionsTemplate),
}

impl EndpointTemplate {
    fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            let combobox_label = ui.label("Template:");
            ComboBox::from_id_salt("endpoint_template_chooser")
                .selected_text(self.name())
                .show_ui(ui, |ui| {
                    let templates = [
                        Self::None,
                        Self::OpenAICompletions(OpenAICompletionsTemplate::default()),
                        Self::TapestryTokenizeOpenAICompletions(
                            TapestryTokenizeOpenAICompletionsTemplate::default(),
                        ),
                        Self::OpenAIChatCompletions(OpenAIChatCompletionsTemplate::default()),
                    ];

                    for template in templates {
                        let name = template.name();
                        ui.selectable_value(self, template, name);
                    }
                })
                .response
                .labelled_by(combobox_label.id);
        });

        match self {
            Self::None => {}
            Self::OpenAICompletions(template) => template.ui(ui),
            Self::OpenAIChatCompletions(template) => template.ui(ui),
            Self::TapestryTokenizeOpenAICompletions(template) => template.ui(ui),
        }
    }
    fn build(&mut self) -> Option<EndpointConfig> {
        let endpoint = match self {
            Self::None => None,
            Self::OpenAICompletions(template) => {
                template.build().map(EndpointConfig::OpenAICompletions)
            }
            Self::OpenAIChatCompletions(template) => {
                template.build().map(EndpointConfig::OpenAIChatCompletions)
            }
            Self::TapestryTokenizeOpenAICompletions(template) => {
                template.build().map(EndpointConfig::OpenAICompletions)
            }
        };

        if endpoint.is_some() {
            *self = Self::None;
        }

        endpoint
    }
    fn name(&self) -> &'static str {
        match self {
            Self::None => "Choose template...",
            Self::OpenAICompletions(_) => "OpenAI-style Completions (Recommended)",
            Self::OpenAIChatCompletions(_) => "OpenAI-style ChatCompletions",
            Self::TapestryTokenizeOpenAICompletions(_) => {
                "OpenAI-style Completions + Tapestry Tokenize"
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum EndpointConfig {
    OpenAICompletions(OpenAICompletionsConfig),
    OpenAIChatCompletions(OpenAIChatCompletionsConfig),
}

impl EndpointConfig {
    fn name(&self) -> &'static str {
        match self {
            Self::OpenAICompletions(_) => "OpenAI-style Completions",
            Self::OpenAIChatCompletions(_) => "OpenAI-style ChatCompletions",
        }
    }
    fn ui(&mut self, ui: &mut Ui, id: &Ulid) {
        match self {
            Self::OpenAICompletions(endpoint) => endpoint.ui(ui, id),
            Self::OpenAIChatCompletions(endpoint) => endpoint.ui(ui, id),
        }
    }
    fn label(&self) -> &str {
        match self {
            Self::OpenAICompletions(endpoint) => endpoint.label(),
            Self::OpenAIChatCompletions(endpoint) => endpoint.label(),
        }
    }
    pub fn default_parameters(&self) -> Vec<(String, String)> {
        match self {
            Self::OpenAICompletions(endpoint) => endpoint.default_parameters(),
            Self::OpenAIChatCompletions(endpoint) => endpoint.default_parameters(),
        }
    }
    pub async fn perform_request(
        &self,
        client: &Client,
        cache: &InferenceCache,
        request: EndpointRequest,
    ) -> Result<Vec<EndpointResponse>, anyhow::Error> {
        match self {
            Self::OpenAICompletions(endpoint) => {
                endpoint.perform_request(client, cache, request).await
            }
            Self::OpenAIChatCompletions(endpoint) => {
                endpoint.perform_request(client, request).await
            }
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Eq, Clone)]
pub enum EmbeddingEndpointConfig {
    #[default]
    None,
    OpenAI(OpenAIEmbeddingsConfig),
}

impl EmbeddingEndpointConfig {
    fn name(&self) -> &'static str {
        match self {
            Self::OpenAI(_) => "OpenAI Embeddings",
            Self::None => "Choose endpoint type...",
        }
    }
    fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            let combobox_label = ui.label("Endpoint type:");
            ComboBox::from_id_salt("endpoint_embedding_chooser")
                .selected_text(self.name())
                .show_ui(ui, |ui| {
                    let templates =
                        vec![Self::None, Self::OpenAI(OpenAIEmbeddingsConfig::default())];

                    for template in templates {
                        let name = template.name();
                        ui.selectable_value(self, template, name);
                    }
                })
                .response
                .labelled_by(combobox_label.id);
        });

        match self {
            Self::OpenAI(endpoint) => endpoint.ui(ui),
            Self::None => {}
        }
    }
    pub async fn perform_request(
        &self,
        client: &Client,
        cache: &InferenceCache,
        request: Vec<u8>,
    ) -> Result<Vec<f32>, anyhow::Error> {
        match self {
            Self::OpenAI(endpoint) => {
                cache
                    .embed(request, async |request| {
                        endpoint.perform_request(client, request).await
                    })
                    .await
            }
            Self::None => Err(anyhow!("No embedding endpoint selected")),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParameterPreset {
    pub label: String,
    pub color: Option<Color32>,
    pub parameters: InferenceParameters,
}

impl ParameterPreset {
    fn from_parameters(parameters: InferenceParameters) -> Self {
        Self {
            label: "New preset".to_string(),
            color: None,
            parameters,
        }
    }
}
