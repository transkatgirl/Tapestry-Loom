use eframe::egui::{CollapsingHeader, TextEdit, Ui, Widget};
use log::trace;
use reqwest::{
    Client, Method, Response, Url,
    header::{CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tapestry_weave::{ulid::Ulid, universal_weave::indexmap::IndexMap, v0::InnerNodeContent};

use crate::settings::inference::{
    Endpoint, EndpointRequest, EndpointResponse, RequestTokensOrBytes, Template,
    render_config_list, render_config_map,
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
                if !(self.endpoint.ends_with("/v1") || self.endpoint.ends_with("/v1/")) {
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
            reuse_tokens: false,
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
                if !(self.endpoint.ends_with("/v1") || self.endpoint.ends_with("/v1/")) {
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

    #[serde(default)]
    pub(super) reuse_tokens: bool,
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
            render_config_map(ui, &mut self.parameters, 0.9, 1.1, true);
        });

        ui.group(|ui| {
            ui.label("Request headers:");
            render_config_map(ui, &mut self.headers, 0.9, 1.1, true);
        });

        let result = *self != old;

        ui.checkbox(
            &mut self.reuse_tokens,
            "(Opportunistically) reuse token IDs",
        );

        result
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
        vec![
            ("temperature".to_string(), "1".to_string()),
            ("max_tokens".to_string(), "10".to_string()),
            ("logprobs".to_string(), "1".to_string()),
        ]
    }
    async fn perform_request(
        &self,
        client: &Client,
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

        if body.remove("stream").is_some() {
            body.insert("stream".to_string(), Value::Bool(false));
        };

        let mut contents = Vec::with_capacity(request.content.len());

        if self.reuse_tokens {
            for segment in request.content.as_ref().clone() {
                contents.push(
                    RequestTokensOrBytes::build(segment, &tokenization_identifier)
                        .into_json_async(|bytes: Vec<u8>| async {
                            Ok(if !self.nonstandard.tokenization_endpoint.is_empty() {
                                error_for_status(
                                    client
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
                                .await?
                            } else {
                                Value::String(String::from_utf8_lossy(&bytes).to_string())
                            })
                        })
                        .await?,
                );
            }

            body.insert(
                "prompt".to_string(),
                if contents.len() == 1 {
                    contents.swap_remove(0)
                } else {
                    Value::Array(contents)
                },
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

                body.insert("prompt".to_string(), Value::Array(vec![tokenized]));
            } else {
                body.insert(
                    "prompt".to_string(),
                    Value::String(String::from_utf8_lossy(&request_bytes).to_string()),
                );
            }
        }

        trace!("{:#?}", &body);

        let response: Map<String, Value> = error_for_status(
            client
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

        let endpoint_response = parse_openai_response(response, metadata, tokenization_identifier);

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
            render_config_map(ui, &mut self.parameters, 0.9, 1.1, true);
        });

        ui.group(|ui| {
            ui.label("Prefix messages:");
            render_config_list(
                ui,
                &mut self.prefix_messages,
                Some("{\"role\": \"user\",\"content\": \"\"}"),
                Some("{\"role\": \"user\",\"content\": \"\"}"),
                2.0,
                true,
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

        ui.group(|ui| {
            ui.label("Suffix messages:");
            render_config_list(
                ui,
                &mut self.suffix_messages,
                Some("{\"role\": \"user\",\"content\": \"\"}"),
                Some("{\"role\": \"user\",\"content\": \"\"}"),
                2.0,
                true,
            );
        });

        ui.group(|ui| {
            ui.label("Request headers:");
            render_config_map(ui, &mut self.headers, 0.9, 1.1, true);
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
        vec![
            ("temperature".to_string(), "1".to_string()),
            ("max_tokens".to_string(), "10".to_string()),
            ("logprobs".to_string(), "true".to_string()),
            ("top_logprobs".to_string(), "1".to_string()),
        ]
    }
    async fn perform_request(
        &self,
        client: &Client,
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

        if !self.nonstandard.tokenization_endpoint.is_empty() {
            let tokenized: Value = error_for_status(
                client
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

            message.insert("content".to_string(), tokenized);
        } else {
            message.insert(
                "content".to_string(),
                Value::String(String::from_utf8_lossy(&request_bytes).to_string()),
            );
        }

        let mut messages =
            Vec::with_capacity(self.prefix_messages.len() + self.suffix_messages.len() + 1);

        build_json_list(&mut messages, self.prefix_messages.clone());

        messages.push(Value::Object(message));

        build_json_list(&mut messages, self.suffix_messages.clone());

        body.insert("messages".to_string(), Value::Array(messages));

        trace!("{:#?}", &body);

        let response: Map<String, Value> = error_for_status(
            client
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

        let endpoint_response = parse_openai_response(response, metadata, tokenization_identifier);

        if !endpoint_response.is_empty() {
            Ok(endpoint_response)
        } else {
            Err(anyhow::Error::msg("Response does not match API schema"))
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub(super) struct NonStandardOpenAIModifications {
    #[serde(default)]
    pub(super) tokenization_endpoint: String,

    #[serde(default)]
    pub(super) chat_message_custom_fields: Vec<(String, String)>,
}

impl NonStandardOpenAIModifications {
    fn render_settings(&mut self, ui: &mut Ui, is_chat: bool) {
        TextEdit::singleline(&mut self.tokenization_endpoint)
            .hint_text("Tapestry-Tokenize Endpoint")
            .desired_width(ui.spacing().text_edit_width * 1.5)
            .ui(ui)
            .on_hover_text("Tapestry-Tokenize Endpoint");
        if is_chat {
            ui.group(|ui| {
                ui.label("Additional input message parameters:");
                render_config_map(ui, &mut self.chat_message_custom_fields, 0.675, 0.825, true);
            });
        }
    }
    fn is_standard(&self) -> bool {
        self.tokenization_endpoint.is_empty() && self.chat_message_custom_fields.is_empty()
    }
}

fn build_json_list(list: &mut Vec<Value>, items: Vec<String>) {
    for item in items {
        if let Ok(value) = serde_json::from_str(&item) {
            list.push(value);
        } else {
            list.push(Value::String(item));
        }
    }
}

fn build_json_object(map: &mut Map<String, Value>, parameters: Vec<(String, String)>) {
    for (key, value) in parameters {
        if let Ok(value) = serde_json::from_str(&value) {
            map.insert(key, value);
        } else {
            map.insert(key, Value::String(value));
        }
    }
}

async fn error_for_status(response: Response) -> Result<Response, anyhow::Error> {
    let status = response.status();
    if status.is_client_error() || status.is_server_error() {
        Err(anyhow::Error::msg(format!(
            "HTTP {}: {}",
            status.as_u16(),
            match response.text().await {
                Ok(text) => text,
                Err(error) => error.to_string(),
            }
        )))
    } else {
        Ok(response)
    }
}

fn parse_openai_response(
    mut response: Map<String, Value>,
    metadata: Vec<(String, String)>,
    tokenization_identifier: Ulid,
) -> Vec<EndpointResponse> {
    trace!("{:#?}", &response);

    if let Some(Value::String(text)) = response.remove("text") {
        return vec![EndpointResponse {
            content: InnerNodeContent::Snippet(text.into_bytes()),
            metadata,
        }];
    }

    if let Some(Value::Array(choices)) = response.remove("choices") {
        let mut responses = Vec::with_capacity(choices.len());

        for choice in choices {
            if let Value::Object(mut choice) = choice {
                let mut tokens = Vec::new();
                let mut metadata = metadata.clone();
                let mut has_top_tokens = false;

                if let Some(Value::Object(mut logprobs)) = choice.remove("logprobs") {
                    if let Some(Value::Array(logprobs_content)) = logprobs.remove("content") {
                        tokens.reserve(logprobs_content.len());

                        for (i, logprob_item) in logprobs_content.into_iter().enumerate() {
                            if let Value::Object(mut logprob_item) = logprob_item {
                                if i == 0
                                    && let Some(Value::Array(top_logprobs)) =
                                        logprob_item.remove("top_logprobs")
                                    && top_logprobs.len() > 1
                                {
                                    let mut tokens = Vec::with_capacity(top_logprobs.len());

                                    for top_logprob in top_logprobs {
                                        if let Value::Object(top_logprob) = top_logprob {
                                            parse_openai_logprob(top_logprob, &mut tokens);
                                        }
                                    }

                                    tokens.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));

                                    responses.reserve(tokens.len());

                                    for (token, prob, token_id) in tokens {
                                        let length = token.len();

                                        responses.push(EndpointResponse {
                                            content: InnerNodeContent::Tokens(vec![(
                                                token,
                                                if let Some(token_id) = token_id {
                                                    IndexMap::from_iter([
                                                        (
                                                            "probability".to_string(),
                                                            prob.to_string(),
                                                        ),
                                                        (
                                                            "original_length".to_string(),
                                                            length.to_string(),
                                                        ),
                                                        (
                                                            "token_id".to_string(),
                                                            token_id.to_string(),
                                                        ),
                                                        (
                                                            "model_id".to_string(),
                                                            tokenization_identifier.to_string(),
                                                        ),
                                                    ])
                                                } else {
                                                    IndexMap::from_iter([
                                                        (
                                                            "probability".to_string(),
                                                            prob.to_string(),
                                                        ),
                                                        (
                                                            "original_length".to_string(),
                                                            length.to_string(),
                                                        ),
                                                    ])
                                                },
                                            )]),
                                            metadata: metadata.clone(),
                                        });
                                    }

                                    has_top_tokens = true;
                                }

                                parse_openai_logprob(logprob_item, &mut tokens);
                            }
                        }
                    } else {
                        if let Some(Value::Array(mut top_logprobs)) =
                            logprobs.remove("top_logprobs")
                            && !top_logprobs.is_empty()
                            && let Value::Object(top_logprobs) = top_logprobs.swap_remove(0)
                            && top_logprobs.len() > 1
                        {
                            let mut tokens = Vec::with_capacity(top_logprobs.len());

                            for (token, logprob) in top_logprobs {
                                if let Value::Number(logprob) = logprob
                                    && let Some(logprob) = logprob.as_f64()
                                {
                                    tokens.push((
                                        token.into_bytes(),
                                        (logprob.exp() * 10000.0).round() / 10000.0,
                                    ));
                                }
                            }

                            tokens.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));

                            responses.reserve(tokens.len());

                            for (token, prob) in tokens {
                                let length = token.len();

                                responses.push(EndpointResponse {
                                    content: InnerNodeContent::Tokens(vec![(
                                        token,
                                        IndexMap::from_iter([
                                            ("probability".to_string(), prob.to_string()),
                                            ("original_length".to_string(), length.to_string()),
                                        ]),
                                    )]),
                                    metadata: metadata.clone(),
                                });
                            }

                            has_top_tokens = true;
                        }

                        let output = &mut tokens;

                        if let Some(Value::Array(mut tokens)) = logprobs.remove("tokens")
                            && let Some(Value::Array(mut token_logprobs)) =
                                logprobs.remove("token_logprobs")
                            && tokens.len() == token_logprobs.len()
                        {
                            output.reserve(tokens.len());

                            if let Some(Value::Array(mut token_ids)) = logprobs.remove("token_ids")
                            {
                                for (token, (logprob, token_id)) in tokens
                                    .drain(..)
                                    .zip(token_logprobs.drain(..).zip(token_ids.drain(..)))
                                {
                                    if let Value::String(token) = token
                                        && let Value::Number(token_id) = token_id
                                        && let Some(token_id) = token_id.as_i128()
                                        && let Value::Number(logprob) = logprob
                                        && let Some(logprob) = logprob.as_f64()
                                    {
                                        output.push((
                                            token.into_bytes(),
                                            (logprob.exp() * 10000.0).round() / 10000.0,
                                            Some(token_id),
                                        ));
                                    }
                                }
                            } else {
                                for (token, logprob) in
                                    tokens.drain(..).zip(token_logprobs.drain(..))
                                {
                                    if let Value::String(token) = token
                                        && let Value::Number(logprob) = logprob
                                        && let Some(logprob) = logprob.as_f64()
                                    {
                                        output.push((
                                            token.into_bytes(),
                                            (logprob.exp() * 10000.0).round() / 10000.0,
                                            None,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(Value::String(finish_reason)) = choice.remove("finish_reason") {
                    metadata.push(("finish_reason".to_string(), finish_reason));
                }

                if !tokens.is_empty() {
                    if !has_top_tokens || tokens.len() > 1 {
                        responses.push(EndpointResponse {
                            content: InnerNodeContent::Tokens(
                                tokens
                                    .into_iter()
                                    .map(|(token, prob, token_id)| {
                                        let length = token.len();

                                        (
                                            token,
                                            if let Some(token_id) = token_id {
                                                IndexMap::from_iter([
                                                    ("probability".to_string(), prob.to_string()),
                                                    (
                                                        "original_length".to_string(),
                                                        length.to_string(),
                                                    ),
                                                    ("token_id".to_string(), token_id.to_string()),
                                                    (
                                                        "model_id".to_string(),
                                                        tokenization_identifier.to_string(),
                                                    ),
                                                ])
                                            } else {
                                                IndexMap::from_iter([
                                                    ("probability".to_string(), prob.to_string()),
                                                    (
                                                        "original_length".to_string(),
                                                        length.to_string(),
                                                    ),
                                                ])
                                            },
                                        )
                                    })
                                    .collect(),
                            ),
                            metadata,
                        });
                    }
                } else if let Some(Value::String(text)) = choice.remove("text") {
                    responses.push(EndpointResponse {
                        content: InnerNodeContent::Snippet(text.into_bytes()),
                        metadata,
                    });
                } else if let Some(Value::Object(mut message)) = choice.remove("message")
                    && let Some(Value::String(content)) = message.remove("content")
                {
                    if let Some(Value::String(role)) = message.remove("role")
                        && role != "assistant"
                    {
                        metadata.push(("role".to_string(), role));
                    }

                    responses.push(EndpointResponse {
                        content: InnerNodeContent::Snippet(content.into_bytes()),
                        metadata,
                    });
                }
            }
        }

        if !responses.is_empty() {
            return responses;
        }
    } else if let Some(Value::Array(outputs)) = response.remove("output") {
        for output in outputs {
            if let Value::Object(mut output) = output
                && let Some(Value::String(output_type)) = output.remove("type")
                && output_type == "message"
            {
                let mut metadata = metadata.clone();

                if let Some(Value::String(status)) = output.remove("status")
                    && status != "completed"
                {
                    metadata.push(("status".to_string(), status));
                }

                if let Some(Value::String(role)) = output.remove("role")
                    && role != "assistant"
                {
                    metadata.push(("role".to_string(), role));
                }

                let mut has_text = false;
                let mut bytes = Vec::with_capacity(512);

                if let Some(Value::Array(content)) = output.remove("content") {
                    for content_item in content {
                        if let Value::Object(mut content_item) = content_item
                            && let Some(Value::String(content_type)) = content_item.remove("type")
                            && content_type == "output_text"
                            && let Some(Value::String(text)) = content_item.remove("text")
                        {
                            bytes.extend(text.into_bytes());
                            has_text = true;
                        }
                    }
                }

                if has_text {
                    return vec![EndpointResponse {
                        content: InnerNodeContent::Snippet(bytes),
                        metadata,
                    }];
                }
            }
        }
    }

    vec![]
}

fn parse_openai_logprob(
    mut logprob: Map<String, Value>,
    output: &mut Vec<(Vec<u8>, f64, Option<i128>)>,
) {
    let token = if let Some(bytes) = logprob
        .remove("bytes")
        .and_then(|v| serde_json::from_value::<Vec<u8>>(v).ok())
        && !bytes.is_empty()
    {
        Some(bytes)
    } else if let Some(Value::String(token)) = logprob.remove("token") {
        Some(token.into_bytes())
    } else {
        None
    };

    let token_id = if let Some(Value::Number(id)) = logprob.remove("id")
        && let Some(id) = id.as_i128()
    {
        Some(id)
    } else {
        None
    };

    if let Some(token) = token
        && let Some(Value::Number(logprob)) = logprob.remove("logprob")
        && let Some(logprob) = logprob.as_f64()
    {
        output.push((token, (logprob.exp() * 10000.0).round() / 10000.0, token_id));
    }
}
