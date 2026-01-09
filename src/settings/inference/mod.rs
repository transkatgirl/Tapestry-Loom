use std::{
    collections::{HashMap, hash_map::Entry},
    fmt::Display,
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use eframe::egui::{
    Align, Color32, ComboBox, DragValue, Layout, RichText, Slider, SliderClamping, TextEdit,
    TextStyle, Ui, Widget, WidgetText,
    color_picker::{Alpha, color_edit_button_srgba},
};
use linked_hash_map::LinkedHashMap;
use log::trace;
use poll_promise::Promise;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tapestry_weave::{
    ulid::Ulid,
    universal_weave::{
        dependent::DependentNode,
        indexmap::{IndexMap, IndexSet},
    },
    v0::{InnerNodeContent, Model, NodeContent, TapestryNode},
};
use tokio::{runtime::Runtime, sync::Mutex};

use crate::settings::inference::openai::{
    OpenAIChatCompletionsConfig, OpenAIChatCompletionsTemplate, OpenAICompletionsConfig,
    OpenAICompletionsTemplate, OpenAIEmbeddingsConfig, TapestryTokenizeOpenAICompletionsTemplate,
};

mod openai;
mod polyparser;
mod shared;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct InferenceSettings {
    pub client: ClientConfig,
    models: IndexMap<Ulid, InferenceModel>,

    #[serde(default)]
    embedding_model: EmbeddingEndpointConfig,

    pub default_parameters: InferenceParameters,

    #[serde(default)]
    parameter_presets: Vec<(String, Option<Color32>, InferenceParameters)>,

    #[serde(skip)]
    template: EndpointTemplate,

    #[serde(skip)]
    clear_embedding_cache: bool,
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
            timeout_minutes: 10.0,
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
    pub fn build(&self) -> Result<InferenceClient, anyhow::Error> {
        Ok(InferenceClient {
            client: ClientBuilder::new()
                .connect_timeout(Duration::from_secs(15))
                .danger_accept_invalid_certs(self.accept_invalid_tls)
                .danger_accept_invalid_hostnames(self.accept_invalid_tls)
                .timeout(Duration::from_secs_f32(self.timeout_minutes * 60.0))
                .build()?,
        })
    }
}

#[derive(Clone)]
pub struct InferenceClient {
    client: Client,
}

#[derive(Clone)]
pub struct InferenceCache {
    embeddings: Arc<Mutex<EmbeddingCache>>,
    tokens: Arc<Mutex<TokenizationCache>>,
}

const EMBEDDING_CACHE_MAX_SIZE: usize = 512;
const TOKENIZATION_CACHE_MAX_SIZE: usize = 16384;

impl Default for InferenceCache {
    fn default() -> Self {
        Self {
            embeddings: Arc::new(Mutex::new(LinkedHashMap::with_capacity(
                EMBEDDING_CACHE_MAX_SIZE,
            ))),
            tokens: Arc::new(Mutex::new(HashMap::with_capacity(16))),
        }
    }
}

type EmbeddingCache = LinkedHashMap<Vec<u8>, Vec<f32>>;

type TokenizationCache = HashMap<Ulid, Arc<Mutex<LinkedHashMap<Vec<u8>, Vec<u64>>>>>;

impl InferenceSettings {
    pub(super) fn render(&mut self, ui: &mut Ui) {
        self.client.render(ui);
        ui.group(|ui| {
            self.template.render(ui);
            if self.template != EndpointTemplate::None
                && ui.button("Add model").clicked()
                && let Some(endpoint) = self.template.build()
            {
                let identifier = Ulid::new();

                self.models.insert(
                    identifier,
                    InferenceModel {
                        label: String::new(),
                        color: None,
                        endpoint,
                        tokenization_identifier: identifier,
                    },
                );
            }
        });
        if self.models.is_empty() {
            ui.label("Add your first model by filling out an inference template.");
        }

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
        ui.heading("Embedding inference");
        if self.embedding_model.render_settings(ui) {
            self.clear_embedding_cache = true;
        };

        ui.separator();
        ui.heading("Editor inference defaults");

        self.default_parameters.render_inner(&self.models, ui);

        ui.separator();
        ui.heading("Editor inference presets");
        ui.add_space(ui.spacing().icon_spacing);

        let mut move_up = None;
        let mut move_down = None;
        let mut copy = None;
        let mut delete = None;

        let length = self.parameter_presets.len();

        for (index, (label, color, parameters)) in self.parameter_presets.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.horizontal_wrapped(|ui| {
                    let textedit_label = ui.label("Label:");

                    TextEdit::singleline(label)
                        .hint_text("Preset label")
                        .ui(ui)
                        .labelled_by(textedit_label.id);

                    if let Some(label_color) = color {
                        color_edit_button_srgba(ui, label_color, Alpha::Opaque)
                            .on_hover_text("Label color");
                        if ui
                            .button("\u{E148}")
                            .on_hover_text("Remove label color")
                            .clicked()
                        {
                            *color = None;
                        }
                    } else if ui
                        .button("\u{E1DD}")
                        .on_hover_text("Add label color")
                        .clicked()
                    {
                        *color = Some(ui.style().visuals.hyperlink_color);
                    }
                });

                ui.add_space(ui.text_style_height(&TextStyle::Body) * 0.75);

                parameters.render_inner(&self.models, ui);

                ui.set_max_width(ui.min_rect().width());

                ui.horizontal_wrapped(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .button("\u{E18E}")
                            .on_hover_text("Delete preset")
                            .clicked()
                        {
                            delete = Some(index);
                        }

                        if ui.button("\u{E09E}").on_hover_text("Copy preset").clicked() {
                            copy = Some(index);
                        }

                        if index != length.saturating_sub(1)
                            && ui
                                .button("\u{E44D}")
                                .on_hover_text("Move preset down")
                                .clicked()
                        {
                            move_down = Some(index);
                        }

                        if index != 0
                            && ui
                                .button("\u{E44E}")
                                .on_hover_text("Move preset up")
                                .clicked()
                        {
                            move_up = Some(index);
                        }
                    });
                });
            });
        }

        if let Some(index) = move_up
            && index > 0
        {
            self.parameter_presets.swap(index, index - 1);
        }

        if let Some(index) = move_down
            && index < self.parameter_presets.len() - 1
        {
            self.parameter_presets.swap(index, index + 1);
        }

        if let Some(index) = copy {
            self.parameter_presets
                .insert(index, self.parameter_presets[index].clone());
        }

        if let Some(delete) = delete {
            self.parameter_presets.remove(delete);
        }

        if ui.button("\u{E13D}").on_hover_text("Add preset").clicked() {
            self.parameter_presets.push((
                "New preset".to_string(),
                None,
                self.default_parameters.clone(),
            ));
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct InferenceModel {
    label: String,
    color: Option<Color32>,
    endpoint: EndpointConfig,

    #[serde(default = "Ulid::new")]
    tokenization_identifier: Ulid,
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
                color_edit_button_srgba(ui, color, Alpha::Opaque).on_hover_text("Label color");
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

        if self.endpoint.render_settings(ui, id) {
            trace!("Updating tokenization identifier for {}", id);
            self.tokenization_identifier = Ulid::new();
        };
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
            )
            .on_hover_text("Request count");
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
        render_config_map(ui, &mut self.parameters, 0.55, 0.45);
    }
}

#[allow(clippy::too_many_arguments)]
impl InferenceParameters {
    pub fn reset(&mut self, settings: &InferenceSettings) {
        *self = settings.default_parameters.clone();
    }
    pub fn switch_preset(&mut self, settings: &InferenceSettings, preset: usize) {
        if let Some((_, _, preset)) = settings.parameter_presets.get(preset.saturating_sub(1)) {
            *self = preset.clone();
        }
    }
    pub fn render(&mut self, settings: &InferenceSettings, ui: &mut Ui) {
        if !settings.parameter_presets.is_empty() {
            ui.group(|ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("\u{E148}").clicked() {
                        *self = settings.default_parameters.clone();
                    }

                    for (label, color, preset) in &settings.parameter_presets {
                        if ui
                            .button(if let Some(color) = color {
                                RichText::new(label).color(*color)
                            } else {
                                RichText::new(label)
                            })
                            .clicked()
                        {
                            *self = preset.clone();
                        }
                    }
                });
            });
            ui.add_space(ui.spacing().icon_spacing);
        }

        self.render_inner(&settings.models, ui);
    }
    fn render_inner(&mut self, models: &IndexMap<Ulid, InferenceModel>, ui: &mut Ui) {
        ui.add(
            Slider::new(&mut self.recursion_depth, 0..=3)
                .clamping(SliderClamping::Never)
                .text("Recursion")
                .suffix(" layers"),
        ).on_hover_text("The recursion depth used for generating nodes. If this is > 0, nodes will be recursively generated up to the set number of layers.");

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
        client: &InferenceClient,
        cache: &InferenceCache,
        parent: Option<Ulid>,
        content: Vec<TokensOrBytes>,
        output: &mut HashMap<Ulid, InferenceHandle>,
    ) {
        self.create_request_inner(
            Rc::new(settings.models.clone()),
            runtime,
            client,
            cache,
            parent,
            Arc::new(content),
            output,
        );
    }
    fn create_request_inner(
        &self,
        models: Rc<IndexMap<Ulid, InferenceModel>>,
        runtime: &Runtime,
        client: &InferenceClient,
        cache: &InferenceCache,
        parent_node: Option<Ulid>,
        content: Arc<Vec<TokensOrBytes>>,
        output: &mut HashMap<Ulid, InferenceHandle>,
    ) {
        let parameters = Rc::new(self.clone());
        let _guard = runtime.enter();

        for model in &self.models {
            if let Some(inference_model) = models.get(&model.model) {
                let content_model = inference_model.content_model();
                let request = EndpointRequest {
                    content: content.clone(),
                    suffix: None,
                    parameters: Arc::new(model.parameters.clone()),
                };
                let endpoint = Arc::new(inference_model.endpoint.clone());
                let tokenization_identifier = inference_model.tokenization_identifier;

                for _ in 0..model.requests {
                    let content_model = content_model.clone();
                    let request = request.clone();
                    let endpoint = endpoint.clone();
                    let client = client.clone();
                    let cache = cache.clone();
                    output.insert(
                        Ulid::new(),
                        InferenceHandle {
                            parent: parent_node,
                            parent_content: content.clone(),
                            models: models.clone(),
                            parameters: parameters.clone(),
                            handle: Promise::spawn_async(async move {
                                let responses = endpoint
                                    .as_ref()
                                    .perform_request(
                                        &client,
                                        &cache,
                                        request,
                                        tokenization_identifier,
                                    )
                                    .await?;

                                responses
                                    .into_iter()
                                    .map(|response| {
                                        Ok((
                                            NodeContent {
                                                content: response.content,
                                                metadata: IndexMap::from_iter(response.metadata),
                                                model: Some(content_model.clone()),
                                            },
                                            response.root,
                                        ))
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
        client: Option<&InferenceClient>,
        cache: &InferenceCache,
        input: &mut HashMap<Ulid, InferenceHandle>,
        output: &mut Vec<Result<TapestryNode, anyhow::Error>>,
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
                        let mut parent_content = value.parent_content.as_ref().clone();
                        parent_content.push(item.0.content.clone().into());

                        parameters.create_request_inner(
                            value.models.clone(),
                            runtime,
                            client,
                            cache,
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
                                from: if !content.1 {
                                    value.parent.map(|id| id.0)
                                } else {
                                    None
                                },
                                to: IndexSet::default(),
                                active: false,
                                bookmarked: false,
                                contents: content.0,
                            }));
                        }
                    }
                    Err(error) => output.push(Err(error)),
                }
            }
        }
    }
}

impl InferenceSettings {
    pub fn create_embedding_request(
        &mut self,
        runtime: &Runtime,
        client: &InferenceClient,
        cache: &InferenceCache,
        request: (Option<Ulid>, Vec<(Ulid, Vec<u8>)>),
        output: &mut HashMap<Option<Ulid>, EmbeddingInferenceHandle>,
    ) {
        let clear_embedding_cache = self.clear_embedding_cache;
        if self.clear_embedding_cache {
            self.clear_embedding_cache = false;
        }

        let _guard = runtime.enter();

        let endpoint = Arc::new(self.embedding_model.clone());

        output.insert(
            request.0,
            EmbeddingInferenceHandle {
                handle: request
                    .1
                    .into_iter()
                    .map(|request| {
                        let endpoint = endpoint.clone();
                        let client = client.clone();
                        let cache = cache.clone();
                        let request_content = request.1;
                        (
                            request.0,
                            Promise::spawn_async(async move {
                                if clear_embedding_cache {
                                    cache.embeddings.lock().await.clear();
                                }

                                endpoint
                                    .as_ref()
                                    .perform_request(&client, &cache, request_content)
                                    .await
                            }),
                        )
                    })
                    .collect(),
            },
        );
    }
    pub fn get_embedding_responses(
        input: &mut HashMap<Option<Ulid>, EmbeddingInferenceHandle>,
        output: &mut Vec<Result<EmbeddingResponse, anyhow::Error>>,
    ) {
        let keys: Vec<Option<Ulid>> = input.keys().cloned().collect();

        'outer: for key in keys {
            let mut is_ready = false;

            if let Some(value) = input.get(&key)
                && value.handle.iter().all(|handle| handle.1.ready().is_some())
            {
                is_ready = true;
            }

            if is_ready && let Some(value) = input.remove(&key) {
                let mut embeddings = Vec::with_capacity(value.handle.len());

                for (id, handle) in value.handle {
                    let result = handle.block_and_take();

                    match result {
                        Ok(contents) => {
                            embeddings.push((id, contents));
                        }
                        Err(error) => {
                            output.push(Err(error));
                            continue 'outer;
                        }
                    }
                }

                output.push(Ok(EmbeddingResponse {
                    id: key,
                    embeddings,
                }));
            }
        }
    }
}

pub struct InferenceHandle {
    parent: Option<Ulid>,
    parent_content: Arc<Vec<TokensOrBytes>>,
    models: Rc<IndexMap<Ulid, InferenceModel>>,
    parameters: Rc<InferenceParameters>,
    handle: Promise<Result<Vec<(NodeContent, bool)>, anyhow::Error>>,
}

#[allow(clippy::type_complexity)]
pub struct EmbeddingInferenceHandle {
    handle: Vec<(Ulid, Promise<Result<Vec<f32>, anyhow::Error>>)>,
}

pub struct EmbeddingResponse {
    pub id: Option<Ulid>,
    pub embeddings: Vec<(Ulid, Vec<f32>)>,
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
    fn render(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            let combobox_label = ui.label("Template:");
            ComboBox::from_id_salt("endpoint_template_chooser")
                .selected_text(format!("{}", self))
                .show_ui(ui, |ui| {
                    let templates = vec![
                        Self::None,
                        Self::OpenAICompletions(OpenAICompletionsTemplate::default()),
                        Self::TapestryTokenizeOpenAICompletions(
                            TapestryTokenizeOpenAICompletionsTemplate::default(),
                        ),
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
            Self::TapestryTokenizeOpenAICompletions(template) => template.render(ui),
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
            Self::TapestryTokenizeOpenAICompletions(template) => {
                if let Some(endpoint) = template.clone().build() {
                    *self = EndpointTemplate::None;

                    Some(EndpointConfig::OpenAICompletions(endpoint))
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
            Self::OpenAICompletions(_) => f.write_str("OpenAI-style Completions (Recommended)"),
            Self::OpenAIChatCompletions(_) => f.write_str("OpenAI-style ChatCompletions"),
            Self::TapestryTokenizeOpenAICompletions(_) => {
                f.write_str("OpenAI-style Completions (Recommended) + Tapestry Tokenize")
            }
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
    fn render_settings(&mut self, ui: &mut Ui, id: &Ulid) -> bool {
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
        client: &InferenceClient,
        cache: &InferenceCache,
        request: EndpointRequest,
        tokenization_identifier: Ulid,
    ) -> Result<Vec<EndpointResponse>, anyhow::Error> {
        match self {
            Self::OpenAICompletions(endpoint) => {
                endpoint
                    .perform_request(client, cache, request, tokenization_identifier)
                    .await
            }
            Self::OpenAIChatCompletions(endpoint) => {
                endpoint
                    .perform_request(client, cache, request, tokenization_identifier)
                    .await
            }
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Eq, Clone)]
enum EmbeddingEndpointConfig {
    #[default]
    None,
    OpenAI(OpenAIEmbeddingsConfig),
}

impl Display for EmbeddingEndpointConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenAI(endpoint) => endpoint.fmt(f),
            Self::None => f.write_str("Choose endpoint type..."),
        }
    }
}

impl EmbeddingEndpoint for EmbeddingEndpointConfig {
    fn render_settings(&mut self, ui: &mut Ui) -> bool {
        let mut result = false;

        ui.horizontal_wrapped(|ui| {
            let combobox_label = ui.label("Endpoint type:");
            if ComboBox::from_id_salt("endpoint_embedding_chooser")
                .selected_text(format!("{}", self))
                .show_ui(ui, |ui| {
                    let templates =
                        vec![Self::None, Self::OpenAI(OpenAIEmbeddingsConfig::default())];

                    for template in templates {
                        let template_label = template.to_string();
                        if ui
                            .selectable_value(self, template, template_label)
                            .changed()
                        {
                            result = true;
                        };
                    }
                })
                .response
                .labelled_by(combobox_label.id)
                .changed()
            {
                result = true;
            };
        });

        if match self {
            Self::OpenAI(endpoint) => endpoint.render_settings(ui),
            Self::None => false,
        } {
            result = true;
        }

        result
    }
    async fn perform_request(
        &self,
        client: &InferenceClient,
        cache: &InferenceCache,
        requests: Vec<u8>,
    ) -> Result<Vec<f32>, anyhow::Error> {
        match self {
            Self::OpenAI(endpoint) => endpoint.perform_request(client, cache, requests).await,
            Self::None => Err(anyhow::Error::msg("No embedding endpoint selected")),
        }
    }
}

#[derive(Debug, Clone)]
struct EndpointRequest {
    content: Arc<Vec<TokensOrBytes>>,
    suffix: Option<Arc<Vec<TokensOrBytes>>>,
    parameters: Arc<Vec<(String, String)>>,
}

struct EndpointResponse {
    root: bool,
    content: InnerNodeContent,
    metadata: Vec<(String, String)>,
}

trait Endpoint: Serialize + DeserializeOwned + Clone {
    fn render_settings(&mut self, ui: &mut Ui, id: &Ulid) -> bool;
    fn label(&self) -> &str;
    fn default_parameters(&self) -> Vec<(String, String)>;
    async fn perform_request(
        &self,
        client: &InferenceClient,
        cache: &InferenceCache,
        request: EndpointRequest,
        tokenization_identifier: Ulid,
    ) -> Result<Vec<EndpointResponse>, anyhow::Error>;
}

trait EmbeddingEndpoint: Serialize + DeserializeOwned + Clone + Display {
    fn render_settings(&mut self, ui: &mut Ui) -> bool;
    async fn perform_request(
        &self,
        client: &InferenceClient,
        cache: &InferenceCache,
        request: Vec<u8>,
    ) -> Result<Vec<f32>, anyhow::Error>;
}

trait Template<T>: Default + Clone
where
    T: Endpoint,
{
    fn render(&mut self, ui: &mut Ui);
    fn build(self) -> Option<T>;
}

pub fn render_config_map(
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

pub fn render_config_list(
    ui: &mut Ui,
    value: &mut Vec<String>,
    hint_text: Option<&str>,
    new_item_text: Option<&str>,
    item_width: f32,
) {
    let mut remove = None;

    let item_width = ui.spacing().text_edit_width * item_width;

    for (index, item) in value.iter_mut().enumerate() {
        ui.horizontal_wrapped(|ui| {
            TextEdit::singleline(item)
                .hint_text(hint_text.unwrap_or("item"))
                .desired_width(item_width)
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
        value.push(new_item_text.map(|s| s.to_string()).unwrap_or_default());
    }
}

#[derive(Debug, Clone)]
pub enum TokensOrBytes {
    TokensAndBytes(Vec<(Vec<u8>, u64, Ulid)>),
    Bytes(Vec<u8>),
}

impl TokensOrBytes {
    fn into_bytes(self) -> Vec<u8> {
        match self {
            TokensOrBytes::Bytes(bytes) => bytes,
            TokensOrBytes::TokensAndBytes(token_pairs) => token_pairs
                .into_iter()
                .flat_map(|(t, _, _)| t.into_iter())
                .collect(),
        }
    }
}

enum RequestTokensOrBytes {
    Tokens(Vec<u64>),
    Bytes(Vec<u8>),
}

impl RequestTokensOrBytes {
    fn build(input: TokensOrBytes, model_id: &Ulid) -> Self {
        match input {
            TokensOrBytes::Bytes(bytes) => Self::Bytes(bytes),
            TokensOrBytes::TokensAndBytes(token_pairs) => {
                let input_len = token_pairs.len();
                let mut bytes =
                    Vec::with_capacity(token_pairs.iter().map(|(t, _, _)| t.len()).sum());
                let mut token_ids = Vec::with_capacity(token_pairs.len());

                for (token, token_id, token_model_id) in token_pairs {
                    bytes.extend(token);
                    if &token_model_id == model_id {
                        token_ids.push(token_id);
                    }
                }

                if token_ids.len() == input_len {
                    Self::Tokens(token_ids)
                } else {
                    Self::Bytes(bytes)
                }
            }
        }
    }
    /*fn into_json(
        self,
        mut byte_handler: impl FnMut(Vec<u8>) -> serde_json::Value,
    ) -> serde_json::Value {
        match self {
            Self::Bytes(bytes) => byte_handler(bytes),
            Self::Tokens(tokens) => serde_json::Value::Array(
                tokens
                    .into_iter()
                    .map(|t| serde_json::Value::Number(serde_json::Number::from_i128(t).unwrap()))
                    .collect(),
            ),
        }
    }*/
    /*async fn into_json_async(
        self,
        byte_handler: impl AsyncFnOnce(Vec<u8>) -> Result<serde_json::Value, anyhow::Error>,
    ) -> Result<serde_json::Value, anyhow::Error> {
        match self {
            Self::Bytes(bytes) => byte_handler(bytes).await,
            Self::Tokens(tokens) => Ok(serde_json::Value::Array(
                tokens
                    .into_iter()
                    .map(|t| serde_json::Value::Number(serde_json::Number::from_i128(t).unwrap()))
                    .collect(),
            )),
        }
    }
    async fn into_tokens_async(
        self,
        byte_handler: impl AsyncFnOnce(Vec<u8>) -> Result<Vec<i128>, anyhow::Error>,
    ) -> Result<Vec<i128>, anyhow::Error> {
        match self {
            Self::Bytes(bytes) => byte_handler(bytes).await,
            Self::Tokens(tokens) => Ok(tokens),
        }
    }*/
    async fn cached_into_tokens_async(
        self,
        identifier: Ulid,
        cache: &Mutex<TokenizationCache>,
        byte_handler: impl AsyncFnOnce(Vec<u8>) -> Result<Vec<u64>, anyhow::Error>,
    ) -> Result<Vec<u64>, anyhow::Error> {
        match self {
            Self::Bytes(bytes) => {
                let mut model_cache = match cache.lock().await.entry(identifier) {
                    Entry::Occupied(occupied) => {
                        if let Some(tokens) = occupied.get().lock().await.get(&bytes) {
                            trace!(
                                "Using cached tokenization of {:?}",
                                String::from_utf8_lossy(&bytes)
                            );
                            return Ok(tokens.clone());
                        } else {
                            occupied.get().clone()
                        }
                    }
                    Entry::Vacant(vacant) => {
                        let occupied = vacant.insert_entry(Arc::new(Mutex::new(
                            LinkedHashMap::with_capacity(TOKENIZATION_CACHE_MAX_SIZE),
                        )));
                        occupied.get().clone()
                    }
                }
                .lock_owned()
                .await;

                trace!("Tokenizing {:?}", String::from_utf8_lossy(&bytes));

                let tokens = byte_handler(bytes.clone()).await?;

                trace!("{:?} = {:?}", String::from_utf8_lossy(&bytes), tokens);

                //let mut model_cache = model_cache.lock_owned().await;

                model_cache.insert(bytes, tokens.clone());

                while model_cache.len() >= TOKENIZATION_CACHE_MAX_SIZE - 1 {
                    model_cache.pop_front();
                }

                Ok(tokens)
            }
            Self::Tokens(tokens) => Ok(tokens),
        }
    }
}
