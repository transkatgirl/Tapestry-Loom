use std::str::FromStr;

use anyhow::anyhow;
use eframe::egui::{CollapsingHeader, TextEdit, Ui, Widget};
use log::trace;
use reqwest::{
    Client,
    header::{CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};
use tapestry_weave::LongId;
use ulid::Ulid;

use crate::{
    common::ui::{config_list, config_map},
    inference::{
        PromptSegment,
        openai::parser::{CompletionOptions, parse_completion_response, parse_embedding_response},
        shared::{
            EndpointRequest, EndpointResponse, InferenceCache, build_json_list, build_json_object,
            error_for_status, segment_bytes, segment_text,
        },
    },
};

mod parser;

// TODO: add streaming support

const DEFAULT_COMPLETIONS_ENDPOINT: &str = "http://127.0.0.1:8080/v1/completions";
const DEFAULT_CHAT_COMPLETIONS_ENDPOINT: &str = "http://127.0.0.1:8080/v1/chat/completions";
const DEFAULT_EMBEDDINGS_ENDPOINT: &str = "http://127.0.0.1:8080/v1/embeddings";
const DEFAULT_TOKENIZATION_ENDPOINT: &str = "http://127.0.0.1:8000";

const DEFAULT_MESSAGE: &str = "{\"role\": \"user\",\"content\": \"\"}";

fn default_headers(api_key: &str) -> Vec<(String, String)> {
    let mut headers = Vec::with_capacity(4);

    if !api_key.is_empty() {
        headers.push(("Authorization".to_string(), ["Bearer ", api_key].concat()));
    }

    headers.extend([
        ("User-Agent".to_string(), "TapestryLoom".to_string()),
        (
            "HTTP-Referer".to_string(),
            "https://github.com/transkatgirl/Tapestry-Loom".to_string(),
        ),
        ("X-Title".to_string(), "Tapestry Loom".to_string()),
    ]);

    headers
}

fn normalize_endpoint(endpoint: &str, default: &str, path: &str) -> String {
    let endpoint = endpoint.trim_end_matches('/');
    let full_path = ["/v1/", path].concat();

    if endpoint.is_empty() {
        default.to_string()
    } else if endpoint.ends_with(&full_path) {
        endpoint.to_string()
    } else if endpoint.ends_with("/v1") {
        [endpoint, "/", path].concat()
    } else {
        [endpoint, &full_path].concat()
    }
}

fn model_parameters(model: &str) -> Vec<(String, String)> {
    if model.is_empty() {
        Vec::new()
    } else {
        vec![("model".to_string(), model.to_string())]
    }
}

fn template_ui(
    ui: &mut Ui,
    endpoint: &mut String,
    endpoint_hint: &str,
    model: &mut String,
    model_optional: bool,
    api_key: &mut String,
) {
    ui.horizontal_wrapped(|ui| {
        TextEdit::singleline(endpoint)
            .hint_text(endpoint_hint)
            .ui(ui)
            .on_hover_text("Endpoint URL");
        TextEdit::singleline(model)
            .hint_text(if model_optional {
                "Model (optional)"
            } else {
                "Model"
            })
            .desired_width(ui.spacing().text_edit_width / 1.5)
            .ui(ui)
            .on_hover_text("Model");
        TextEdit::singleline(api_key)
            .hint_text("API key (optional)")
            .desired_width(ui.spacing().text_edit_width / 1.5)
            .ui(ui)
            .on_hover_text("API key");
    });
}

fn config_map_group(
    ui: &mut Ui,
    label: &str,
    map: &mut Vec<(String, String)>,
    key_width: f32,
    value_width: f32,
) {
    ui.group(|ui| {
        ui.label(label);
        config_map(ui, map, key_width, value_width);
    });
}

fn message_list_group(ui: &mut Ui, label: &str, messages: &mut Vec<String>) {
    ui.group(|ui| {
        ui.label(label);
        config_list(ui, messages, DEFAULT_MESSAGE, Some(DEFAULT_MESSAGE), 2.0);
    });
}

fn build_headers(headers: &[(String, String)]) -> Result<HeaderMap, anyhow::Error> {
    let mut map = HeaderMap::with_capacity(headers.len());

    for (key, value) in headers {
        map.insert(HeaderName::from_str(key)?, HeaderValue::from_str(value)?);
    }

    Ok(map)
}

fn model_label<'a>(parameters: &'a [(String, String)], endpoint: &'a str) -> &'a str {
    parameters
        .iter()
        .find(|(key, value)| key == "model" && !value.is_empty())
        .map(|(_, value)| value.as_str())
        .unwrap_or(endpoint)
}

#[derive(Default, Debug, PartialEq)]
pub(super) struct OpenAICompletionsTemplate {
    endpoint: String,
    model: String,
    api_key: String,
}

impl OpenAICompletionsTemplate {
    pub(super) fn ui(&mut self, ui: &mut Ui) {
        template_ui(
            ui,
            &mut self.endpoint,
            DEFAULT_COMPLETIONS_ENDPOINT,
            &mut self.model,
            true,
            &mut self.api_key,
        );
    }
    pub(super) fn build(&self) -> Option<OpenAICompletionsConfig> {
        Some(OpenAICompletionsConfig {
            endpoint: normalize_endpoint(
                &self.endpoint,
                DEFAULT_COMPLETIONS_ENDPOINT,
                "completions",
            ),
            parameters: model_parameters(&self.model),
            headers: default_headers(&self.api_key),
            nonstandard: NonStandardOpenAIModifications::default(),
        })
    }
}

#[derive(Default, Debug, PartialEq)]
pub(super) struct TapestryTokenizeOpenAICompletionsTemplate {
    endpoint: String,
    model: String,
    api_key: String,

    tokenization_endpoint: String,
}

impl TapestryTokenizeOpenAICompletionsTemplate {
    pub(super) fn ui(&mut self, ui: &mut Ui) {
        template_ui(
            ui,
            &mut self.endpoint,
            DEFAULT_COMPLETIONS_ENDPOINT,
            &mut self.model,
            false,
            &mut self.api_key,
        );
        ui.horizontal_wrapped(|ui| {
            TextEdit::singleline(&mut self.tokenization_endpoint)
                .hint_text(DEFAULT_TOKENIZATION_ENDPOINT)
                .ui(ui)
                .on_hover_text("Tokenization base URL");
        });
    }
    pub(super) fn build(&self) -> Option<OpenAICompletionsConfig> {
        if self.model.is_empty() {
            return None;
        }

        Some(OpenAICompletionsConfig {
            endpoint: normalize_endpoint(
                &self.endpoint,
                DEFAULT_COMPLETIONS_ENDPOINT,
                "completions",
            ),
            parameters: model_parameters(&self.model),
            headers: default_headers(&self.api_key),
            nonstandard: NonStandardOpenAIModifications {
                tokenization_endpoint: [
                    if self.tokenization_endpoint.is_empty() {
                        DEFAULT_TOKENIZATION_ENDPOINT
                    } else {
                        &self.tokenization_endpoint
                    },
                    "/",
                    &self.model,
                ]
                .concat(),
                ..Default::default()
            },
        })
    }
}

#[derive(Default, Debug, PartialEq)]
pub(super) struct OpenAIChatCompletionsTemplate {
    endpoint: String,
    model: String,
    api_key: String,
}

impl OpenAIChatCompletionsTemplate {
    pub(super) fn ui(&mut self, ui: &mut Ui) {
        template_ui(
            ui,
            &mut self.endpoint,
            DEFAULT_CHAT_COMPLETIONS_ENDPOINT,
            &mut self.model,
            true,
            &mut self.api_key,
        );
    }
    pub(super) fn build(&self) -> Option<OpenAIChatCompletionsConfig> {
        Some(OpenAIChatCompletionsConfig {
            endpoint: normalize_endpoint(
                &self.endpoint,
                DEFAULT_CHAT_COMPLETIONS_ENDPOINT,
                "chat/completions",
            ),
            prefix_messages: Vec::new(),
            message_role: default_message_role(),
            suffix_messages: Vec::new(),
            parameters: model_parameters(&self.model),
            headers: default_headers(&self.api_key),
            nonstandard: NonStandardOpenAIModifications::default(),
        })
    }
}

fn default_parameters(endpoint: &str, logprobs: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut parameters = vec![
        ("temperature".to_string(), "1".to_string()),
        ("max_tokens".to_string(), "10".to_string()),
    ];

    // OpenRouter doesn't handle logprobs properly
    if !endpoint.contains("openrouter.ai/api/v1") {
        parameters.extend(
            logprobs
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string())),
        );
    }

    parameters
}

fn build_body(
    endpoint_parameters: &[(String, String)],
    request: &EndpointRequest,
    top_keys: &[&str],
) -> (Map<String, Value>, CompletionOptions) {
    let mut body = Map::with_capacity(2 + request.parameters.len() + endpoint_parameters.len());

    build_json_object(&mut body, endpoint_parameters);
    build_json_object(&mut body, &request.parameters);

    if let Some(stream) = body.get_mut("stream") {
        *stream = Value::Bool(false);
    }

    let options = CompletionOptions {
        parameters: request.parameters.clone(),
        seed: body.get("seed").and_then(Value::as_u64),
        echo: body.get("echo").and_then(Value::as_bool).unwrap_or(false),
        single_token: body.get("max_tokens").and_then(Value::as_u64) == Some(1),
        requested_top: top_keys
            .iter()
            .find_map(|key| body.get(*key)?.as_u64())
            .map(|top| top as usize),
    };

    (body, options)
}

async fn post_json<T: DeserializeOwned>(
    client: &Client,
    endpoint: &str,
    headers: HeaderMap,
    body: Map<String, Value>,
) -> Result<T, anyhow::Error> {
    trace!("{:#?}", body);
    Ok(error_for_status(
        client
            .post(endpoint)
            .headers(headers)
            .json(&body)
            .send()
            .await?,
    )
    .await?
    .json()
    .await?)
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(super) struct OpenAICompletionsConfig {
    endpoint: String,
    parameters: Vec<(String, String)>,
    headers: Vec<(String, String)>,

    #[serde(default)]
    nonstandard: NonStandardOpenAIModifications,
}

impl OpenAICompletionsConfig {
    pub(super) fn ui(&mut self, ui: &mut Ui, id: &Ulid) {
        TextEdit::singleline(&mut self.endpoint)
            .hint_text("Endpoint URL")
            .desired_width(ui.spacing().text_edit_width * 2.0)
            .ui(ui)
            .on_hover_text("Endpoint URL");

        self.nonstandard.ui(ui, id, false);

        config_map_group(ui, "Request parameters:", &mut self.parameters, 0.9, 1.1);

        config_map_group(ui, "Request headers:", &mut self.headers, 0.9, 1.1);
    }
    pub(super) fn label(&self) -> &str {
        model_label(&self.parameters, &self.endpoint)
    }
    pub(super) fn default_parameters(&self) -> Vec<(String, String)> {
        default_parameters(&self.endpoint, &[("logprobs", "20")])
    }
    async fn tokenize(
        &self,
        client: &Client,
        headers: &HeaderMap,
        bytes: Vec<u8>,
    ) -> Result<Vec<u64>, anyhow::Error> {
        Ok(error_for_status(
            client
                .post(&self.nonstandard.tokenization_endpoint)
                .headers(headers.clone())
                .header(CONTENT_TYPE, "application/octet-stream")
                .body(bytes)
                .send()
                .await?,
        )
        .await?
        .json()
        .await?)
    }
    pub(super) async fn perform_request(
        &self,
        client: &Client,
        cache: &InferenceCache,
        request: EndpointRequest,
    ) -> Result<Vec<EndpointResponse>, anyhow::Error> {
        let headers = build_headers(&self.headers)?;

        let (mut body, options) = build_body(&self.parameters, &request, &["logprobs"]);

        let model_identifier = LongId::new(request.tokenization_identifier.0);

        let prefix_tokens = if self.nonstandard.tokenization_endpoint.is_empty() {
            Vec::new()
        } else if self.nonstandard.reuse_tokens {
            let mut tokens = Vec::new();

            for segment in request.prefix.iter() {
                match segment {
                    PromptSegment::Tokens { model, ids, .. }
                        if Some(model) == model_identifier.as_ref() =>
                    {
                        tokens.extend(ids);
                    }
                    segment => tokens.extend(
                        cache
                            .tokenize(request.tokenization_identifier, segment.bytes(), |bytes| {
                                self.tokenize(client, &headers, bytes)
                            })
                            .await?,
                    ),
                }
            }

            tokens
        } else {
            self.tokenize(client, &headers, segment_bytes(&request.prefix))
                .await?
        };

        let suffix_tokens = if !self.nonstandard.tokenization_endpoint.is_empty()
            && self.nonstandard.tokenize_suffix
            && let Some(suffix) = &request.suffix
        {
            if self.nonstandard.reuse_tokens {
                let mut tokens = Vec::new();

                for segment in suffix.iter() {
                    match segment {
                        PromptSegment::Tokens { model, ids, .. }
                            if Some(model) == model_identifier.as_ref() =>
                        {
                            tokens.extend(ids);
                        }
                        segment => tokens.extend(
                            cache
                                .tokenize(
                                    request.tokenization_identifier,
                                    segment.bytes(),
                                    |bytes| self.tokenize(client, &headers, bytes),
                                )
                                .await?,
                        ),
                    }
                }

                tokens
            } else {
                self.tokenize(client, &headers, segment_bytes(suffix))
                    .await?
            }
        } else {
            Vec::new()
        };

        body.insert(
            "prompt".to_string(),
            if prefix_tokens.is_empty() {
                Value::String(segment_text(&request.prefix))
            } else {
                Value::from(prefix_tokens)
            },
        );

        if let Some(suffix) = &request.suffix {
            let suffix = segment_text(suffix);

            if !suffix.is_empty() || !suffix_tokens.is_empty() {
                body.insert(
                    "suffix".to_string(),
                    if suffix_tokens.is_empty() {
                        Value::String(suffix)
                    } else {
                        Value::from(suffix_tokens)
                    },
                );
            }
        }

        parse_completion_response(
            post_json(client, &self.endpoint, headers, body).await?,
            options,
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(super) struct OpenAIChatCompletionsConfig {
    endpoint: String,

    #[serde(default)]
    prefix_messages: Vec<String>,

    #[serde(default = "default_message_role")]
    message_role: String,

    #[serde(default)]
    suffix_messages: Vec<String>,

    parameters: Vec<(String, String)>,
    headers: Vec<(String, String)>,

    #[serde(default)]
    nonstandard: NonStandardOpenAIModifications,
}

fn default_message_role() -> String {
    "assistant".to_string()
}

impl OpenAIChatCompletionsConfig {
    pub(super) fn ui(&mut self, ui: &mut Ui, id: &Ulid) {
        TextEdit::singleline(&mut self.endpoint)
            .hint_text("Endpoint URL")
            .desired_width(ui.spacing().text_edit_width * 2.0)
            .ui(ui)
            .on_hover_text("Endpoint URL");

        self.nonstandard.ui(ui, id, true);

        config_map_group(ui, "Request parameters:", &mut self.parameters, 0.9, 1.1);

        message_list_group(ui, "Prefix messages:", &mut self.prefix_messages);

        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                let label = ui.label("Message role:").id;
                TextEdit::singleline(&mut self.message_role)
                    .hint_text("assistant")
                    .clip_text(false)
                    .ui(ui)
                    .labelled_by(label);
            });
        });

        if !self.suffix_messages.is_empty() {
            message_list_group(ui, "Suffix messages:", &mut self.suffix_messages);
        }

        config_map_group(ui, "Request headers:", &mut self.headers, 0.9, 1.1);
    }
    pub(super) fn label(&self) -> &str {
        model_label(&self.parameters, &self.endpoint)
    }
    pub(super) fn default_parameters(&self) -> Vec<(String, String)> {
        default_parameters(
            &self.endpoint,
            &[("logprobs", "true"), ("top_logprobs", "20")],
        )
    }
    pub(super) async fn perform_request(
        &self,
        client: &Client,
        request: EndpointRequest,
    ) -> Result<Vec<EndpointResponse>, anyhow::Error> {
        if request.suffix.is_some() {
            return Err(anyhow!("Endpoint does not support FIM"));
        }

        let headers = build_headers(&self.headers)?;

        let (mut body, mut options) =
            build_body(&self.parameters, &request, &["top_logprobs", "logprobs"]);

        options.echo = false;

        let mut message = Map::with_capacity(self.nonstandard.chat_message_custom_fields.len() + 2);

        build_json_object(&mut message, &self.nonstandard.chat_message_custom_fields);

        message.insert("role".to_string(), Value::String(self.message_role.clone()));
        message.insert(
            "content".to_string(),
            Value::String(segment_text(&request.prefix)),
        );

        let mut messages =
            Vec::with_capacity(self.prefix_messages.len() + self.suffix_messages.len() + 1);

        build_json_list(&mut messages, &self.prefix_messages);
        messages.push(Value::Object(message));
        build_json_list(&mut messages, &self.suffix_messages);

        body.insert("messages".to_string(), Value::Array(messages));

        parse_completion_response(
            post_json(client, &self.endpoint, headers, body).await?,
            options,
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(default)]
pub(super) struct NonStandardOpenAIModifications {
    tokenization_endpoint: String,
    reuse_tokens: bool,
    #[serde(default)]
    tokenize_suffix: bool,

    chat_message_custom_fields: Vec<(String, String)>,
}

impl Default for NonStandardOpenAIModifications {
    fn default() -> Self {
        Self {
            tokenization_endpoint: String::new(),
            reuse_tokens: true,
            tokenize_suffix: false,
            chat_message_custom_fields: Vec::new(),
        }
    }
}

impl NonStandardOpenAIModifications {
    fn is_standard(&self) -> bool {
        self.tokenization_endpoint.is_empty() && self.chat_message_custom_fields.is_empty()
    }
    fn ui(&mut self, ui: &mut Ui, id: &Ulid, is_chat: bool) {
        CollapsingHeader::new("Non-standard API modifications")
            .id_salt(id)
            .default_open(!self.is_standard())
            .show(ui, |ui| {
                if is_chat {
                    config_map_group(
                        ui,
                        "Additional input message parameters:",
                        &mut self.chat_message_custom_fields,
                        0.675,
                        0.825,
                    );
                } else {
                    TextEdit::singleline(&mut self.tokenization_endpoint)
                        .hint_text("Tapestry-Tokenize Endpoint")
                        .desired_width(ui.spacing().text_edit_width * 1.5)
                        .ui(ui)
                        .on_hover_text("Tapestry-Tokenize Endpoint");

                    if !self.tokenization_endpoint.is_empty() {
                        ui.checkbox(
                            &mut self.reuse_tokens,
                            "(Opportunistically) reuse output token IDs",
                        ).on_hover_text("Reuses token IDs from the model's output whenever possible rather than retokenizing the input.\n\nThis may improve output quality, especially when nodes are being generated by one model rather than an ensemble of models.");

                        ui.checkbox(
                            &mut self.tokenize_suffix,
                            "Tokenize suffix",
                        );
                    }
                }
            });
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(super) struct OpenAIEmbeddingsConfig {
    endpoint: String,
    parameters: Vec<(String, String)>,
    headers: Vec<(String, String)>,

    #[serde(default)]
    prefix: String,
}

impl Default for OpenAIEmbeddingsConfig {
    fn default() -> Self {
        Self {
            endpoint: DEFAULT_EMBEDDINGS_ENDPOINT.to_string(),
            parameters: vec![
                ("model".to_string(), "embeddinggemma-2-BF16".to_string()),
                ("encoding_format".to_string(), "base64".to_string()),
            ],
            headers: default_headers(""),
            prefix: String::new(),
        }
    }
}

impl OpenAIEmbeddingsConfig {
    pub(super) fn ui(&mut self, ui: &mut Ui) {
        TextEdit::singleline(&mut self.endpoint)
            .hint_text("Endpoint URL")
            .desired_width(ui.spacing().text_edit_width * 2.0)
            .ui(ui)
            .on_hover_text("Endpoint URL");

        config_map_group(ui, "Request parameters:", &mut self.parameters, 0.9, 1.1);

        config_map_group(ui, "Request headers:", &mut self.headers, 0.9, 1.1);

        ui.group(|ui| {
            let label = ui.label("Input prefix:").id;

            TextEdit::singleline(&mut self.prefix)
                .desired_width(ui.spacing().text_edit_width * 2.0)
                .ui(ui)
                .labelled_by(label);
        });
    }
    pub(super) async fn perform_request(
        &self,
        client: &Client,
        request: &[u8],
    ) -> Result<Vec<f32>, anyhow::Error> {
        let headers = build_headers(&self.headers)?;

        let mut body = Map::with_capacity(1 + self.parameters.len());

        build_json_object(&mut body, &self.parameters);

        body.insert(
            "input".to_string(),
            Value::String([self.prefix.as_str(), &String::from_utf8_lossy(request)].concat()),
        );

        parse_embedding_response(post_json(client, &self.endpoint, headers, body).await?)
    }
}
