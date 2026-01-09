use std::fmt::Display;

use eframe::egui::{CollapsingHeader, TextEdit, Ui, Widget};
use log::trace;
use reqwest::{
    Method, Url,
    header::{CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use tapestry_weave::ulid::Ulid;

use super::{
    EmbeddingEndpoint, Endpoint, EndpointRequest, EndpointResponse, InferenceCache,
    InferenceClient, RequestTokensOrBytes, Template, render_config_list, render_config_map,
    shared::{
        build_json_list, build_json_object, error_for_status, parse_embedding_response,
        parse_response,
    },
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
                .hint_text("http://127.0.0.1:8080/v1/completions")
                .ui(ui)
                .on_hover_text("Endpoint URL");
            TextEdit::singleline(&mut self.model)
                .hint_text("Model (optional)")
                .desired_width(ui.spacing().text_edit_width / 1.5)
                .ui(ui)
                .on_hover_text("Model");
            TextEdit::singleline(&mut self.api_key)
                .hint_text("API key (optional)")
                .desired_width(ui.spacing().text_edit_width / 1.5)
                .ui(ui)
                .on_hover_text("API key");
        });
    }
    fn build(mut self) -> Option<OpenAICompletionsConfig> {
        Some(OpenAICompletionsConfig {
            endpoint: if self.endpoint.is_empty() {
                "http://127.0.0.1:8080/v1/completions".to_string()
            } else {
                if !(self.endpoint.ends_with("/v1")
                    || self.endpoint.ends_with("/v1/")
                    || self.endpoint.ends_with("/v1/completions"))
                {
                    if self.endpoint.ends_with("/") {
                        self.endpoint.push_str("v1");
                    } else {
                        self.endpoint.push_str("/v1");
                    }
                }

                if !self.endpoint.ends_with("/completions") {
                    if self.endpoint.ends_with("/") {
                        self.endpoint.push_str("completions");
                    } else {
                        self.endpoint.push_str("/completions");
                    }
                }

                self.endpoint
            },
            parameters: if self.model.is_empty() {
                Vec::new()
            } else {
                vec![("model".to_string(), self.model)]
            },
            headers: if self.api_key.is_empty() {
                vec![
                    ("User-Agent".to_string(), "TapestryLoom".to_string()),
                    (
                        "HTTP-Referer".to_string(),
                        "https://github.com/transkatgirl/Tapestry-Loom".to_string(),
                    ),
                    ("X-Title".to_string(), "Tapestry Loom".to_string()),
                ]
            } else {
                vec![
                    (
                        "Authorization".to_string(),
                        ["Bearer ", &self.api_key].concat(),
                    ),
                    ("User-Agent".to_string(), "TapestryLoom".to_string()),
                    (
                        "HTTP-Referer".to_string(),
                        "https://github.com/transkatgirl/Tapestry-Loom".to_string(),
                    ),
                    ("X-Title".to_string(), "Tapestry Loom".to_string()),
                ]
            },
            nonstandard: NonStandardOpenAIModifications::default(),
        })
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub(super) struct TapestryTokenizeOpenAICompletionsTemplate {
    endpoint: String,
    model: String,
    api_key: String,

    tokenization_endpoint: String,
}

impl Template<OpenAICompletionsConfig> for TapestryTokenizeOpenAICompletionsTemplate {
    fn render(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            TextEdit::singleline(&mut self.endpoint)
                .hint_text("http://127.0.0.1:8080/v1/completions")
                .ui(ui)
                .on_hover_text("Endpoint URL");
            TextEdit::singleline(&mut self.model)
                .hint_text("Model")
                .desired_width(ui.spacing().text_edit_width / 1.5)
                .ui(ui)
                .on_hover_text("Model");
            TextEdit::singleline(&mut self.api_key)
                .hint_text("API key (optional)")
                .desired_width(ui.spacing().text_edit_width / 1.5)
                .ui(ui)
                .on_hover_text("API key");
        });
        ui.horizontal_wrapped(|ui| {
            TextEdit::singleline(&mut self.tokenization_endpoint)
                .hint_text("http://127.0.0.1:8000")
                .ui(ui)
                .on_hover_text("Tokenization base URL");
        });
    }
    fn build(mut self) -> Option<OpenAICompletionsConfig> {
        if self.model.is_empty() {
            return None;
        }

        Some(OpenAICompletionsConfig {
            nonstandard: NonStandardOpenAIModifications {
                tokenization_endpoint: [
                    if self.tokenization_endpoint.is_empty() {
                        "http://127.0.0.1:8000"
                    } else {
                        &self.tokenization_endpoint
                    },
                    "/",
                    &self.model,
                ]
                .concat(),
                reuse_tokens: true,
                ..Default::default()
            },
            endpoint: if self.endpoint.is_empty() {
                "http://127.0.0.1:8080/v1/completions".to_string()
            } else {
                if !(self.endpoint.ends_with("/v1")
                    || self.endpoint.ends_with("/v1/")
                    || self.endpoint.ends_with("/v1/completions"))
                {
                    if self.endpoint.ends_with("/") {
                        self.endpoint.push_str("v1");
                    } else {
                        self.endpoint.push_str("/v1");
                    }
                }

                if !self.endpoint.ends_with("/completions") {
                    if self.endpoint.ends_with("/") {
                        self.endpoint.push_str("completions");
                    } else {
                        self.endpoint.push_str("/completions");
                    }
                }

                self.endpoint
            },
            parameters: vec![("model".to_string(), self.model)],
            headers: if self.api_key.is_empty() {
                vec![
                    ("User-Agent".to_string(), "TapestryLoom".to_string()),
                    (
                        "HTTP-Referer".to_string(),
                        "https://github.com/transkatgirl/Tapestry-Loom".to_string(),
                    ),
                    ("X-Title".to_string(), "Tapestry Loom".to_string()),
                ]
            } else {
                vec![
                    (
                        "Authorization".to_string(),
                        ["Bearer ", &self.api_key].concat(),
                    ),
                    ("User-Agent".to_string(), "TapestryLoom".to_string()),
                    (
                        "HTTP-Referer".to_string(),
                        "https://github.com/transkatgirl/Tapestry-Loom".to_string(),
                    ),
                    ("X-Title".to_string(), "Tapestry Loom".to_string()),
                ]
            },
        })
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub(super) struct OpenAIChatCompletionsTemplate {
    endpoint: String,
    model: String,
    api_key: String,
}

impl Template<OpenAIChatCompletionsConfig> for OpenAIChatCompletionsTemplate {
    fn render(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            TextEdit::singleline(&mut self.endpoint)
                .hint_text("http://127.0.0.1:8080/v1/chat/completions")
                .ui(ui)
                .on_hover_text("Endpoint URL");
            TextEdit::singleline(&mut self.model)
                .hint_text("Model (optional)")
                .desired_width(ui.spacing().text_edit_width / 1.5)
                .ui(ui)
                .on_hover_text("Model");
            TextEdit::singleline(&mut self.api_key)
                .hint_text("API key (optional)")
                .desired_width(ui.spacing().text_edit_width / 1.5)
                .ui(ui)
                .on_hover_text("API key");
        });
    }
    fn build(mut self) -> Option<OpenAIChatCompletionsConfig> {
        Some(OpenAIChatCompletionsConfig {
            endpoint: if self.endpoint.is_empty() {
                "http://127.0.0.1:8080/v1/chat/completions".to_string()
            } else {
                if !(self.endpoint.ends_with("/v1")
                    || self.endpoint.ends_with("/v1/")
                    || self.endpoint.ends_with("/v1/chat/completions"))
                {
                    if self.endpoint.ends_with("/") {
                        self.endpoint.push_str("v1");
                    } else {
                        self.endpoint.push_str("/v1");
                    }
                }

                if !self.endpoint.ends_with("/chat/completions") {
                    if self.endpoint.ends_with("/") {
                        self.endpoint.push_str("chat/completions");
                    } else {
                        self.endpoint.push_str("/chat/completions");
                    }
                }

                self.endpoint
            },
            prefix_messages: Vec::new(),
            message_role: "assistant".to_string(),
            suffix_messages: Vec::new(),
            parameters: if self.model.is_empty() {
                Vec::new()
            } else {
                vec![("model".to_string(), self.model)]
            },
            headers: if self.api_key.is_empty() {
                vec![
                    ("User-Agent".to_string(), "TapestryLoom".to_string()),
                    (
                        "HTTP-Referer".to_string(),
                        "https://github.com/transkatgirl/Tapestry-Loom".to_string(),
                    ),
                    ("X-Title".to_string(), "Tapestry Loom".to_string()),
                ]
            } else {
                vec![
                    (
                        "Authorization".to_string(),
                        ["Bearer ", &self.api_key].concat(),
                    ),
                    ("User-Agent".to_string(), "TapestryLoom".to_string()),
                    (
                        "HTTP-Referer".to_string(),
                        "https://github.com/transkatgirl/Tapestry-Loom".to_string(),
                    ),
                    ("X-Title".to_string(), "Tapestry Loom".to_string()),
                ]
            },
            nonstandard: NonStandardOpenAIModifications::default(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(super) struct OpenAICompletionsConfig {
    pub(super) endpoint: String,
    pub(super) parameters: Vec<(String, String)>,
    pub(super) headers: Vec<(String, String)>,

    #[serde(default)]
    pub(super) nonstandard: NonStandardOpenAIModifications,
}

impl Endpoint for OpenAICompletionsConfig {
    fn render_settings(&mut self, ui: &mut Ui, id: &Ulid) -> bool {
        let old = self.clone();

        TextEdit::singleline(&mut self.endpoint)
            .hint_text("Endpoint URL")
            .desired_width(ui.spacing().text_edit_width * 2.0)
            .ui(ui)
            .on_hover_text("Endpoint URL");

        CollapsingHeader::new("Non-standard API modifications")
            .id_salt(id)
            .default_open(!self.nonstandard.is_standard())
            .show(ui, |ui| {
                self.nonstandard.render_settings(ui, false);
            });

        ui.group(|ui| {
            ui.label("Request parameters:");
            render_config_map(ui, &mut self.parameters, 0.9, 1.1);
        });

        ui.group(|ui| {
            ui.label("Request headers:");
            render_config_map(ui, &mut self.headers, 0.9, 1.1);
        });

        *self != old
    }
    fn label(&self) -> &str {
        for (key, value) in &self.parameters {
            if key == "model" && !value.is_empty() {
                return value;
            }
        }

        &self.endpoint
    }
    fn default_parameters(&self) -> Vec<(String, String)> {
        if self.endpoint.contains("openrouter.ai/api/v1") {
            // OpenRouter doesn't handle logprobs properly
            vec![
                ("temperature".to_string(), "1".to_string()),
                ("max_tokens".to_string(), "10".to_string()),
            ]
        } else {
            vec![
                ("temperature".to_string(), "1".to_string()),
                ("max_tokens".to_string(), "10".to_string()),
                ("logprobs".to_string(), "20".to_string()),
            ]
        }
    }
    async fn perform_request(
        &self,
        client: &InferenceClient,
        cache: &InferenceCache,
        request: EndpointRequest,
        tokenization_identifier: Ulid,
    ) -> Result<Vec<EndpointResponse>, anyhow::Error> {
        let mut headers = HeaderMap::with_capacity(self.headers.len());

        for (key, value) in &self.headers {
            headers.insert(
                HeaderName::from_bytes(key.as_bytes())?,
                HeaderValue::from_str(value)?,
            );
        }

        let mut body = Map::with_capacity(1 + request.parameters.len() + self.parameters.len());

        build_json_object(&mut body, self.parameters.clone());
        build_json_object(&mut body, request.parameters.as_ref().clone());

        let echo = body.get("echo").and_then(|t| t.as_bool()).unwrap_or(false);

        let single_token = body
            .get("max_tokens")
            .and_then(|t| t.as_u64())
            .map(|t| t == 1)
            .unwrap_or(false);

        let requested_top = body
            .get("logprobs")
            .and_then(|t| t.as_u64())
            .map(|t| t as usize);

        if body.remove("stream").is_some() {
            body.insert("stream".to_string(), Value::Bool(false));
        };

        if self.nonstandard.reuse_tokens && !self.nonstandard.tokenization_endpoint.is_empty() {
            let mut token_futures = Vec::with_capacity(request.content.len());

            for segment in request.content.as_ref().clone() {
                token_futures.push(
                    RequestTokensOrBytes::build(segment, &tokenization_identifier)
                        .cached_into_tokens_async(
                            tokenization_identifier,
                            &cache.tokens,
                            |bytes: Vec<u8>| async {
                                Ok(error_for_status(
                                    client
                                        .client
                                        .request(
                                            Method::POST,
                                            Url::parse(&self.nonstandard.tokenization_endpoint)?,
                                        )
                                        .headers(headers.clone())
                                        .header(CONTENT_TYPE, "application/octet-stream")
                                        .body(bytes)
                                        .send()
                                        .await?,
                                )
                                .await?
                                .json()
                                .await?)
                            },
                        ),
                );
            }

            let mut token_segments = Vec::with_capacity(token_futures.len());

            for item in token_futures {
                token_segments.push(item.await?);
            }

            body.insert(
                "prompt".to_string(),
                Value::Array(
                    token_segments
                        .into_iter()
                        .flatten()
                        .map(|t| Value::Number(Number::from_u128(t.into()).unwrap()))
                        .collect(),
                ),
            );
        } else {
            let request_bytes: Vec<u8> = request
                .content
                .as_ref()
                .clone()
                .into_iter()
                .flat_map(|t| t.into_bytes())
                .collect();

            if !self.nonstandard.tokenization_endpoint.is_empty() {
                let tokenized: Value = error_for_status(
                    client
                        .client
                        .request(
                            Method::POST,
                            Url::parse(&self.nonstandard.tokenization_endpoint)?,
                        )
                        .headers(headers.clone())
                        .header(CONTENT_TYPE, "application/octet-stream")
                        .body(request_bytes)
                        .send()
                        .await?,
                )
                .await?
                .json()
                .await?;

                body.insert("prompt".to_string(), tokenized);
            } else {
                body.insert(
                    "prompt".to_string(),
                    Value::String(String::from_utf8_lossy(&request_bytes).to_string()),
                );
            }
        }

        trace!("{:#?}", &body);

        if let Some(prompt) = body.get_mut("prompt")
            && let Value::Array(prompt_list) = prompt
            && prompt_list.is_empty()
        {
            *prompt = Value::String(String::new());
        }

        let response: Map<String, Value> = error_for_status(
            client
                .client
                .request(Method::POST, Url::parse(&self.endpoint)?)
                .headers(headers)
                .json(&Value::Object(body))
                .send()
                .await?,
        )
        .await?
        .json()
        .await?;

        let metadata = request.parameters.as_ref().clone();

        let endpoint_response = parse_response(
            response,
            metadata,
            tokenization_identifier,
            echo,
            single_token,
            requested_top,
        );

        if !endpoint_response.is_empty() {
            Ok(endpoint_response)
        } else {
            Err(anyhow::Error::msg("Response does not match API schema"))
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(super) struct OpenAIChatCompletionsConfig {
    pub(super) endpoint: String,

    #[serde(default)]
    pub(super) prefix_messages: Vec<String>,

    #[serde(default = "default_message_role")]
    pub(super) message_role: String,

    #[serde(default)]
    pub(super) suffix_messages: Vec<String>,

    pub(super) parameters: Vec<(String, String)>,
    pub(super) headers: Vec<(String, String)>,

    #[serde(default)]
    pub(super) nonstandard: NonStandardOpenAIModifications,
}

fn default_message_role() -> String {
    "assistant".to_string()
}

impl Endpoint for OpenAIChatCompletionsConfig {
    fn render_settings(&mut self, ui: &mut Ui, id: &Ulid) -> bool {
        let old = self.clone();

        TextEdit::singleline(&mut self.endpoint)
            .hint_text("Endpoint URL")
            .desired_width(ui.spacing().text_edit_width * 2.0)
            .ui(ui)
            .on_hover_text("Endpoint URL");

        CollapsingHeader::new("Non-standard API modifications")
            .id_salt(id)
            .default_open(!self.nonstandard.is_standard())
            .show(ui, |ui| {
                self.nonstandard.render_settings(ui, true);
            });

        ui.group(|ui| {
            ui.label("Request parameters:");
            render_config_map(ui, &mut self.parameters, 0.9, 1.1);
        });

        ui.group(|ui| {
            ui.label("Prefix messages:");
            render_config_list(
                ui,
                &mut self.prefix_messages,
                Some("{\"role\": \"user\",\"content\": \"\"}"),
                Some("{\"role\": \"user\",\"content\": \"\"}"),
                2.0,
            );
        });

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
            ui.group(|ui| {
                ui.label("Suffix messages:");
                render_config_list(
                    ui,
                    &mut self.suffix_messages,
                    Some("{\"role\": \"user\",\"content\": \"\"}"),
                    Some("{\"role\": \"user\",\"content\": \"\"}"),
                    2.0,
                );
            });
        }

        ui.group(|ui| {
            ui.label("Request headers:");
            render_config_map(ui, &mut self.headers, 0.9, 1.1);
        });

        *self != old
    }
    fn label(&self) -> &str {
        for (key, value) in &self.parameters {
            if key == "model" && !value.is_empty() {
                return value;
            }
        }

        &self.endpoint
    }
    fn default_parameters(&self) -> Vec<(String, String)> {
        if self.endpoint.contains("openrouter.ai/api/v1") {
            // OpenRouter doesn't handle logprobs properly
            vec![
                ("temperature".to_string(), "1".to_string()),
                ("max_tokens".to_string(), "10".to_string()),
            ]
        } else {
            vec![
                ("temperature".to_string(), "1".to_string()),
                ("max_tokens".to_string(), "10".to_string()),
                ("logprobs".to_string(), "true".to_string()),
                ("top_logprobs".to_string(), "20".to_string()),
            ]
        }
    }
    async fn perform_request(
        &self,
        client: &InferenceClient,
        _cache: &InferenceCache,
        request: EndpointRequest,
        tokenization_identifier: Ulid,
    ) -> Result<Vec<EndpointResponse>, anyhow::Error> {
        let mut headers = HeaderMap::with_capacity(self.headers.len());

        for (key, value) in &self.headers {
            headers.insert(
                HeaderName::from_bytes(key.as_bytes())?,
                HeaderValue::from_str(value)?,
            );
        }

        let mut body = Map::with_capacity(1 + request.parameters.len() + self.parameters.len());

        build_json_object(&mut body, self.parameters.clone());
        build_json_object(&mut body, request.parameters.as_ref().clone());

        let single_token = body
            .get("max_tokens")
            .and_then(|t| t.as_u64())
            .map(|t| t == 1)
            .unwrap_or(false);

        let requested_top = body
            .get("logprobs")
            .and_then(|t| t.as_u64())
            .map(|t| t as usize);

        if body.remove("stream").is_some() {
            body.insert("stream".to_string(), Value::Bool(false));
        };

        let mut message = Map::with_capacity(self.nonstandard.chat_message_custom_fields.len() + 2);

        build_json_object(
            &mut message,
            self.nonstandard.chat_message_custom_fields.clone(),
        );

        message.insert("role".to_string(), Value::String(self.message_role.clone()));

        let request_bytes: Vec<u8> = request
            .content
            .as_ref()
            .clone()
            .into_iter()
            .flat_map(|t| t.into_bytes())
            .collect();

        message.insert(
            "content".to_string(),
            Value::String(String::from_utf8_lossy(&request_bytes).to_string()),
        );

        let mut messages =
            Vec::with_capacity(self.prefix_messages.len() + self.suffix_messages.len() + 1);

        build_json_list(&mut messages, self.prefix_messages.clone());

        /*if !(request_bytes.is_empty()
            && !self.prefix_messages.is_empty()
            && self.suffix_messages.is_empty())
        {
            messages.push(Value::Object(message));
        }*/

        messages.push(Value::Object(message));

        build_json_list(&mut messages, self.suffix_messages.clone());

        body.insert("messages".to_string(), Value::Array(messages));

        trace!("{:#?}", &body);

        let response: Map<String, Value> = error_for_status(
            client
                .client
                .request(Method::POST, Url::parse(&self.endpoint)?)
                .headers(headers)
                .json(&Value::Object(body))
                .send()
                .await?,
        )
        .await?
        .json()
        .await?;

        let metadata = request.parameters.as_ref().clone();

        let endpoint_response = parse_response(
            response,
            metadata,
            tokenization_identifier,
            false,
            single_token,
            requested_top,
        );

        if !endpoint_response.is_empty() {
            Ok(endpoint_response)
        } else {
            Err(anyhow::Error::msg("Response does not match API schema"))
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(super) struct NonStandardOpenAIModifications {
    #[serde(default)]
    pub(super) tokenization_endpoint: String,

    #[serde(default = "default_reuse_tokens")]
    pub(super) reuse_tokens: bool,

    #[serde(default)]
    pub(super) chat_message_custom_fields: Vec<(String, String)>,
}

impl Default for NonStandardOpenAIModifications {
    fn default() -> Self {
        Self {
            tokenization_endpoint: String::new(),
            reuse_tokens: true,
            chat_message_custom_fields: Vec::new(),
        }
    }
}

fn default_reuse_tokens() -> bool {
    true
}

impl NonStandardOpenAIModifications {
    fn render_settings(&mut self, ui: &mut Ui, is_chat: bool) {
        if is_chat {
            ui.group(|ui| {
                ui.label("Additional input message parameters:");
                render_config_map(ui, &mut self.chat_message_custom_fields, 0.675, 0.825);
            });
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
            }
        }
    }
    #[allow(clippy::nonminimal_bool)]
    fn is_standard(&self) -> bool {
        !(self.reuse_tokens && !self.tokenization_endpoint.is_empty())
            && self.tokenization_endpoint.is_empty()
            && self.chat_message_custom_fields.is_empty()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub(super) struct OpenAIEmbeddingsConfig {
    pub(super) endpoint: String,
    pub(super) parameters: Vec<(String, String)>,
    pub(super) headers: Vec<(String, String)>,
}

impl Default for OpenAIEmbeddingsConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:8080/v1/embeddings".to_string(),
            parameters: vec![
                ("model".to_string(), "embeddinggemma-300M-F32".to_string()),
                ("encoding_format".to_string(), "base64".to_string()),
            ],
            headers: vec![
                ("User-Agent".to_string(), "TapestryLoom".to_string()),
                (
                    "HTTP-Referer".to_string(),
                    "https://github.com/transkatgirl/Tapestry-Loom".to_string(),
                ),
                ("X-Title".to_string(), "Tapestry Loom".to_string()),
            ],
        }
    }
}

impl Display for OpenAIEmbeddingsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OpenAI Embeddings (Recommended)")
    }
}

impl EmbeddingEndpoint for OpenAIEmbeddingsConfig {
    fn render_settings(&mut self, ui: &mut Ui) -> bool {
        let old = self.clone();

        TextEdit::singleline(&mut self.endpoint)
            .hint_text("Endpoint URL")
            .desired_width(ui.spacing().text_edit_width * 2.0)
            .ui(ui)
            .on_hover_text("Endpoint URL");

        ui.group(|ui| {
            ui.label("Request parameters:");
            render_config_map(ui, &mut self.parameters, 0.9, 1.1);
        });

        ui.group(|ui| {
            ui.label("Request headers:");
            render_config_map(ui, &mut self.headers, 0.9, 1.1);
        });

        *self != old
    }
    async fn perform_request(
        &self,
        client: &InferenceClient,
        cache: &InferenceCache,
        request: Vec<u8>,
    ) -> Result<Vec<f32>, anyhow::Error> {
        if let Some(embedding) = cache.embeddings.lock().await.get(&request) {
            return Ok(embedding.clone());
        };

        let mut headers = HeaderMap::with_capacity(self.headers.len());

        for (key, value) in &self.headers {
            headers.insert(
                HeaderName::from_bytes(key.as_bytes())?,
                HeaderValue::from_str(value)?,
            );
        }

        let mut body = Map::with_capacity(1 + self.parameters.len());

        build_json_object(&mut body, self.parameters.clone());

        body.insert(
            "input".to_string(),
            Value::String(String::from_utf8_lossy(&request).to_string()),
        );

        trace!("{:#?}", &body);

        let response: Value = error_for_status(
            client
                .client
                .request(Method::POST, Url::parse(&self.endpoint)?)
                .headers(headers)
                .json(&Value::Object(body))
                .send()
                .await?,
        )
        .await?
        .json()
        .await?;

        match parse_embedding_response(response) {
            Some(embedding) => {
                cache
                    .embeddings
                    .lock()
                    .await
                    .insert(request, embedding.clone());
                Ok(embedding)
            }
            None => Err(anyhow::Error::msg("Response does not match API schema")),
        }
    }
}
