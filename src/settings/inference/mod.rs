use std::{collections::HashMap, fmt::Display, rc::Rc, sync::Arc, time::Duration};

use bytes::{Bytes, BytesMut};
use eframe::egui::{
    Align, Color32, ComboBox, DragValue, Layout, RichText, Slider, SliderClamping, TextEdit,
    TextStyle, Ui, Widget, WidgetText,
    color_picker::{Alpha, color_edit_button_srgba},
};
use poll_promise::Promise;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{
        dependent::DependentNode,
        indexmap::{IndexMap, IndexSet},
    },
    v0::{InnerNodeContent, Model, NodeContent},
};
use tokio::runtime::Runtime;

use crate::settings::inference::openai::{
    OpenAIChatCompletionsConfig, OpenAIChatCompletionsTemplate, OpenAICompletionsConfig,
    OpenAICompletionsTemplate,
};

mod openai;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct InferenceSettings {
    pub client: ClientConfig,
    models: IndexMap<Ulid, InferenceModel>,
    pub default_parameters: InferenceParameters,

    #[serde(skip)]
    template: EndpointTemplate,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ClientConfig {
    accept_invalid_tls: bool,
    timeout_minutes: f32,
}

#[allow(clippy::derivable_impls)]
impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            accept_invalid_tls: false,
            timeout_minutes: 5.0,
        }
    }
}

impl ClientConfig {
    fn render(&mut self, ui: &mut Ui) {
        let accept_invalid_tls_label = if self.accept_invalid_tls {
            RichText::new("Accept invalid TLS (dangerous)").color(ui.style().visuals.warn_fg_color)
        } else {
            RichText::new("Accept invalid TLS")
        };
        ui.checkbox(&mut self.accept_invalid_tls, accept_invalid_tls_label);
        ui.add(
            Slider::new(&mut self.timeout_minutes, 1.0..=1440.0)
                .logarithmic(true)
                .clamping(SliderClamping::Never)
                .text("Request timeout")
                .suffix(" minutes"),
        );
    }
    pub fn build(&self) -> Result<Client, reqwest::Error> {
        ClientBuilder::new()
            .connect_timeout(Duration::from_secs(15))
            .danger_accept_invalid_certs(self.accept_invalid_tls)
            .danger_accept_invalid_hostnames(self.accept_invalid_tls)
            .timeout(Duration::from_secs_f32(self.timeout_minutes * 60.0))
            .build()
    }
}

impl InferenceSettings {
    pub(super) fn render(&mut self, ui: &mut Ui) {
        self.client.render(ui);
        ui.group(|ui| {
            self.template.render(ui);
            if self.template != EndpointTemplate::None
                && ui.button("Add model").clicked()
                && let Some(endpoint) = self.template.build()
            {
                self.models.insert(
                    Ulid::new(),
                    InferenceModel {
                        label: String::new(),
                        color: None,
                        endpoint,
                    },
                );
            }
        });

        let mut move_up = None;
        let mut move_down = None;
        let mut copy = None;
        let mut delete = None;

        let length = self.models.len();
        for (index, (id, model)) in &mut self.models.iter_mut().enumerate() {
            ui.group(|ui| {
                model.render(ui, id);

                ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.75);

                ui.set_max_width(ui.min_rect().width());

                ui.horizontal_wrapped(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .button("\u{E18E}")
                            .on_hover_text("Delete model")
                            .clicked()
                        {
                            delete = Some(*id);
                        }

                        if ui.button("\u{E09E}").on_hover_text("Copy model").clicked() {
                            copy = Some((*id, index));
                        }

                        if index != length.saturating_sub(1)
                            && ui
                                .button("\u{E44D}")
                                .on_hover_text("Move model down")
                                .clicked()
                        {
                            move_down = Some(*id);
                        }

                        if index != 0
                            && ui
                                .button("\u{E44E}")
                                .on_hover_text("Move model up")
                                .clicked()
                        {
                            move_up = Some(*id);
                        }
                    });
                });
            });
        }

        if let Some(index) = move_up.and_then(|id| self.models.get_index_of(&id))
            && index > 0
        {
            self.models.swap_indices(index, index - 1);
        }

        if let Some(index) = move_down.and_then(|id| self.models.get_index_of(&id))
            && index < self.models.len() - 1
        {
            self.models.swap_indices(index, index + 1);
        }

        if let Some((id, index)) = copy
            && let Some(copy) = self.models.get(&id).cloned()
        {
            self.models.insert_before(index, Ulid::new(), copy);
        }

        if let Some(delete) = delete {
            self.models.shift_remove(&delete);
        }

        ui.separator();

        ui.heading("Editor inference defaults");
        self.default_parameters.render_inner(&self.models, ui);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct InferenceModel {
    label: String,
    color: Option<Color32>,
    endpoint: EndpointConfig,
}

impl InferenceModel {
    fn label(&self) -> &str {
        if self.label.is_empty() {
            self.endpoint.label()
        } else {
            &self.label
        }
    }
    fn widget_text(&self) -> WidgetText {
        if let Some(color) = self.color {
            WidgetText::RichText(Arc::new(RichText::new(self.label()).color(color)))
        } else {
            WidgetText::Text(self.label().to_string())
        }
    }
    fn content_model(&self) -> Model {
        Model {
            label: self.label().to_string(),
            metadata: if let Some(color) = self.color {
                IndexMap::from_iter([("color".to_string(), color.to_hex())])
            } else {
                IndexMap::default()
            },
        }
    }
    fn render(&mut self, ui: &mut Ui, id: &Ulid) {
        ui.horizontal_wrapped(|ui| {
            let textedit_label = ui.label("Label:");

            TextEdit::singleline(&mut self.label)
                .hint_text(self.endpoint.label())
                .ui(ui)
                .labelled_by(textedit_label.id);

            if let Some(color) = &mut self.color {
                color_edit_button_srgba(ui, color, Alpha::Opaque);
                if ui
                    .button("\u{E148}")
                    .on_hover_text("Remove label color")
                    .clicked()
                {
                    self.color = None;
                }
            } else if ui
                .button("\u{E1DD}")
                .on_hover_text("Add label color")
                .clicked()
            {
                self.color = Some(ui.style().visuals.hyperlink_color);
            }
        });

        ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.75);
        ui.label(["Endpoint Mode: ", &self.endpoint.to_string()].concat());

        self.endpoint.render_settings(ui, id);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InferenceParameters {
    pub recursion_depth: usize,
    pub models: Vec<ModelInferenceParameters>,

    #[serde(skip)]
    new_model: Ulid,
}

impl Default for InferenceParameters {
    fn default() -> Self {
        Self {
            recursion_depth: 0,
            models: Vec::new(),
            new_model: Ulid(0),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelInferenceParameters {
    pub model: Ulid,
    pub requests: usize,
    pub parameters: Vec<(String, String)>,
}

impl ModelInferenceParameters {
    fn render(
        &mut self,
        ui: &mut Ui,
        models: &IndexMap<Ulid, InferenceModel>,
        buttons: impl FnOnce(&mut Ui),
    ) {
        let selected = if let Some(model) = models.get(&self.model) {
            model.widget_text()
        } else {
            WidgetText::Text("Invalid model".to_string())
        };

        ui.horizontal_wrapped(|ui| {
            ui.add(
                DragValue::new(&mut self.requests)
                    .suffix("x")
                    .range(1..=usize::MAX),
            );
            ComboBox::from_id_salt(ui.next_auto_id())
                .selected_text(selected)
                .width(ui.spacing().text_edit_width * 0.6)
                .show_ui(ui, |ui| {
                    for (id, model) in models {
                        ui.selectable_value(&mut self.model, *id, model.widget_text());
                    }
                });
            buttons(ui);
        });

        ui.label("Request parameters:");
        render_config_map_small(ui, &mut self.parameters, 0.55, 0.45);
    }
}

impl InferenceParameters {
    pub fn reset(&mut self, settings: &InferenceSettings) {
        *self = settings.default_parameters.clone();
    }
    pub fn render(&mut self, settings: &InferenceSettings, ui: &mut Ui) {
        self.render_inner(&settings.models, ui);
    }
    fn render_inner(&mut self, models: &IndexMap<Ulid, InferenceModel>, ui: &mut Ui) {
        ui.add(
            Slider::new(&mut self.recursion_depth, 0..=3)
                .clamping(SliderClamping::Never)
                .text("Recursion")
                .suffix(" layers"),
        );

        let mut move_up = None;
        let mut move_down = None;
        let mut copy = None;
        let mut delete = None;

        let length = self.models.len();
        for (index, model) in &mut self.models.iter_mut().enumerate() {
            ui.group(|ui| {
                model.render(ui, models, |ui| {
                    if index != 0
                        && ui
                            .button("\u{E44E}")
                            .on_hover_text("Move model up")
                            .clicked()
                    {
                        move_up = Some(index);
                    }

                    if index != length.saturating_sub(1)
                        && ui
                            .button("\u{E44D}")
                            .on_hover_text("Move model down")
                            .clicked()
                    {
                        move_down = Some(index);
                    }

                    if ui.button("\u{E09E}").on_hover_text("Copy model").clicked() {
                        copy = Some(index);
                    }

                    if ui
                        .button("\u{E18E}")
                        .on_hover_text("Delete model")
                        .clicked()
                    {
                        delete = Some(index);
                    }
                });
            });
        }

        if let Some(index) = move_up
            && index > 0
        {
            self.models.swap(index, index - 1);
        }

        if let Some(index) = move_down
            && index < self.models.len() - 1
        {
            self.models.swap(index, index + 1);
        }

        if let Some(index) = copy
            && let Some(copy) = self.models.get(index).cloned()
        {
            self.models.insert(index, copy);
        }

        if let Some(delete) = delete {
            self.models.remove(delete);
        }

        let selected = if let Some(model) = models.get(&self.new_model) {
            model.widget_text()
        } else if self.new_model == Ulid(0) {
            WidgetText::Text("Choose model...".to_string())
        } else {
            WidgetText::Text("Invalid model".to_string())
        };

        ComboBox::from_id_salt(ui.next_auto_id())
            .selected_text(selected)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.new_model, Ulid(0), "Choose model...");

                for (id, model) in models {
                    ui.selectable_value(&mut self.new_model, *id, model.widget_text());
                }
            });

        if self.new_model != Ulid(0)
            && let Some(model) = models.get(&self.new_model)
        {
            self.models.push(ModelInferenceParameters {
                model: self.new_model,
                requests: 5,
                parameters: model.endpoint.default_parameters(),
            });
            self.new_model = Ulid(0);
        }
    }
    pub fn create_request(
        &self,
        settings: &InferenceSettings,
        runtime: &Runtime,
        client: &Client,
        parent: Option<Ulid>,
        content: Vec<u8>,
        output: &mut HashMap<Ulid, InferenceHandle>,
    ) {
        self.create_request_inner(
            Rc::new(settings.models.clone()),
            runtime,
            client,
            parent,
            Bytes::from(content),
            output,
        );
    }
    fn create_request_inner(
        &self,
        models: Rc<IndexMap<Ulid, InferenceModel>>,
        runtime: &Runtime,
        client: &Client,
        parent_node: Option<Ulid>,
        content: Bytes,
        output: &mut HashMap<Ulid, InferenceHandle>,
    ) {
        let parameters = Rc::new(self.clone());
        let _guard = runtime.enter();

        for model in &self.models {
            if let Some(inference_model) = models.get(&model.model) {
                let content_model = inference_model.content_model();
                let request = EndpointRequest {
                    content: content.clone(),
                    parameters: Arc::new(model.parameters.clone()),
                };
                let endpoint = Arc::new(inference_model.endpoint.clone());

                for _ in 0..model.requests {
                    let content_model = content_model.clone();
                    let request = request.clone();
                    let endpoint = endpoint.clone();
                    let client = client.clone();
                    output.insert(
                        Ulid::new(),
                        InferenceHandle {
                            parent: parent_node,
                            parent_content: content.clone(),
                            models: models.clone(),
                            parameters: parameters.clone(),
                            handle: Promise::spawn_async(async move {
                                let responses =
                                    endpoint.as_ref().perform_request(&client, request).await?;

                                responses
                                    .into_iter()
                                    .map(|response| {
                                        Ok(NodeContent {
                                            content: response.content,
                                            metadata: IndexMap::from_iter(response.metadata),
                                            model: Some(content_model.clone()),
                                        })
                                    })
                                    .collect()
                            }),
                        },
                    );
                }
            }
        }
    }
    pub fn get_responses(
        runtime: &Runtime,
        client: Option<&Client>,
        input: &mut HashMap<Ulid, InferenceHandle>,
        output: &mut Vec<Result<DependentNode<NodeContent>, anyhow::Error>>,
    ) {
        let keys: Vec<Ulid> = input.keys().cloned().collect();

        for key in keys {
            let mut is_ready = false;

            if let Some(value) = input.get(&key)
                && value.handle.ready().is_some()
            {
                is_ready = true;
            }

            if is_ready && let Some(value) = input.remove(&key) {
                let result = value.handle.block_and_take();

                let identifiers = if let Ok(content) = &result {
                    (0..content.len())
                        .map(|_| Ulid::from_datetime(key.datetime()))
                        .collect()
                } else {
                    vec![]
                };

                if value.parameters.recursion_depth > 0
                    && let Ok(content) = &result
                    && let Some(client) = client
                {
                    let mut parameters = value.parameters.as_ref().clone();
                    parameters.recursion_depth -= 1;

                    for (i, item) in content.iter().enumerate() {
                        let mut parent_content = BytesMut::from(value.parent_content.clone());
                        parent_content.extend_from_slice(&item.content.as_bytes());

                        parameters.create_request_inner(
                            value.models.clone(),
                            runtime,
                            client,
                            Some(identifiers[i]),
                            parent_content.into(),
                            input,
                        );
                    }
                }

                match result {
                    Ok(contents) => {
                        for (i, content) in contents.into_iter().enumerate() {
                            output.push(Ok(DependentNode {
                                id: identifiers[i].0,
                                from: value.parent.map(|id| id.0),
                                to: IndexSet::default(),
                                active: false,
                                bookmarked: false,
                                contents: content,
                            }));
                        }
                    }
                    Err(error) => output.push(Err(error)),
                }
            }
        }
    }
}

pub struct InferenceHandle {
    parent: Option<Ulid>,
    parent_content: Bytes,
    models: Rc<IndexMap<Ulid, InferenceModel>>,
    parameters: Rc<InferenceParameters>,
    handle: Promise<Result<Vec<NodeContent>, anyhow::Error>>,
}

#[derive(Default, Debug, PartialEq)]
enum EndpointTemplate {
    #[default]
    None,
    OpenAICompletions(OpenAICompletionsTemplate),
    OpenAIChatCompletions(OpenAIChatCompletionsTemplate),
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
                        Self::OpenAIChatCompletions(OpenAIChatCompletionsTemplate::default()),
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
            Self::OpenAIChatCompletions(template) => template.render(ui),
        }
    }
    fn build(&mut self) -> Option<EndpointConfig> {
        match self {
            Self::None => None,
            Self::OpenAICompletions(template) => {
                if let Some(endpoint) = template.clone().build() {
                    *self = EndpointTemplate::None;

                    Some(EndpointConfig::OpenAICompletions(endpoint))
                } else {
                    None
                }
            }
            Self::OpenAIChatCompletions(template) => {
                if let Some(endpoint) = template.clone().build() {
                    *self = EndpointTemplate::None;

                    Some(EndpointConfig::OpenAIChatCompletions(endpoint))
                } else {
                    None
                }
            }
        }
    }
}

impl Display for EndpointTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str("Choose template..."),
            Self::OpenAICompletions(_) => f.write_str("OpenAI-style Completions"),
            Self::OpenAIChatCompletions(_) => f.write_str("OpenAI-style ChatCompletions"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]

enum EndpointConfig {
    OpenAICompletions(OpenAICompletionsConfig),
    OpenAIChatCompletions(OpenAIChatCompletionsConfig),
}

impl Display for EndpointConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenAICompletions(_) => f.write_str("OpenAI-style Completions"),
            Self::OpenAIChatCompletions(_) => f.write_str("OpenAI-style ChatCompletions"),
        }
    }
}

impl Endpoint for EndpointConfig {
    fn render_settings(&mut self, ui: &mut Ui, id: &Ulid) {
        match self {
            Self::OpenAICompletions(endpoint) => endpoint.render_settings(ui, id),
            Self::OpenAIChatCompletions(endpoint) => endpoint.render_settings(ui, id),
        }
    }
    fn label(&self) -> &str {
        match self {
            Self::OpenAICompletions(endpoint) => endpoint.label(),
            Self::OpenAIChatCompletions(endpoint) => endpoint.label(),
        }
    }
    fn default_parameters(&self) -> Vec<(String, String)> {
        match self {
            Self::OpenAICompletions(endpoint) => endpoint.default_parameters(),
            Self::OpenAIChatCompletions(endpoint) => endpoint.default_parameters(),
        }
    }
    async fn perform_request(
        &self,
        client: &Client,
        request: EndpointRequest,
    ) -> Result<Vec<EndpointResponse>, anyhow::Error> {
        match self {
            Self::OpenAICompletions(endpoint) => endpoint.perform_request(client, request).await,
            Self::OpenAIChatCompletions(endpoint) => {
                endpoint.perform_request(client, request).await
            }
        }
    }
}

#[derive(Debug, Clone)]
struct EndpointRequest {
    content: Bytes,
    parameters: Arc<Vec<(String, String)>>,
}

struct EndpointResponse {
    content: InnerNodeContent,
    metadata: Vec<(String, String)>,
}

trait Endpoint: Serialize + DeserializeOwned + Clone {
    fn render_settings(&mut self, ui: &mut Ui, id: &Ulid);
    fn label(&self) -> &str;
    fn default_parameters(&self) -> Vec<(String, String)>;
    async fn perform_request(
        &self,
        client: &Client,
        request: EndpointRequest,
    ) -> Result<Vec<EndpointResponse>, anyhow::Error>;
}

trait Template<T>: Default + Clone
where
    T: Endpoint,
{
    fn render(&mut self, ui: &mut Ui);
    fn build(self) -> Option<T>;
}

fn render_config_map(
    ui: &mut Ui,
    value: &mut Vec<(String, String)>,
    key_width: f32,
    value_width: f32,
) {
    let mut remove = None;

    let key_width = ui.spacing().text_edit_width * key_width;
    let value_width = ui.spacing().text_edit_width * value_width;

    for (index, (key, value)) in value.iter_mut().enumerate() {
        ui.horizontal_wrapped(|ui| {
            TextEdit::singleline(key)
                .hint_text("key")
                .desired_width(key_width)
                .ui(ui);
            TextEdit::singleline(value)
                .hint_text("value")
                .desired_width(value_width)
                .ui(ui);
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

fn render_config_map_small(
    ui: &mut Ui,
    value: &mut Vec<(String, String)>,
    key_width: f32,
    value_width: f32,
) {
    let mut remove = None;

    let key_width = ui.spacing().text_edit_width * key_width;
    let value_width = ui.spacing().text_edit_width * value_width;

    for (index, (key, value)) in value.iter_mut().enumerate() {
        ui.horizontal_wrapped(|ui| {
            TextEdit::singleline(key)
                .hint_text("key")
                .desired_width(key_width)
                .clip_text(false)
                .ui(ui);
            TextEdit::singleline(value)
                .hint_text("value")
                .desired_width(value_width)
                .clip_text(false)
                .ui(ui);
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

pub fn escaped_string_from_utf8(bytes: &[u8]) -> String {
    let mut string = String::with_capacity(bytes.len());

    for chunk in bytes.utf8_chunks() {
        string.push_str(chunk.valid());

        for invalid in chunk.invalid() {
            string.push_str(&format!("\\x{invalid:X}"));
        }
    }

    string
}
