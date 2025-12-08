use eframe::egui::{TextEdit, Ui, Widget};
use log::trace;
use reqwest::{
    Client, Method, Response, Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tapestry_weave::{universal_weave::indexmap::IndexMap, v0::InnerNodeContent};

use crate::settings::inference::{
    Endpoint, EndpointRequest, EndpointResponse, Template, escaped_string_from_utf8,
    render_config_map,
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
                .hint_text("https://openrouter.ai/api/v1/completions")
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
                "https://openrouter.ai/api/v1/completions".to_string()
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
                .hint_text("https://openrouter.ai/api/v1/chat/completions")
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
                "https://openrouter.ai/api/v1/chat/completions".to_string()
            } else {
                if !self.endpoint.ends_with("/v1/chat/completions") {
                    self.endpoint.push_str("/v1/chat/completions");
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
        })
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

        body.insert(
            "prompt".to_string(),
            Value::String(escaped_string_from_utf8(&request.content)),
        );

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

        let endpoint_response = parse_openai_response(response, metadata);

        if !endpoint_response.is_empty() {
            Ok(endpoint_response)
        } else {
            Err(anyhow::Error::msg("Response does not match API schema"))
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(super) struct OpenAIChatCompletionsConfig {
    endpoint: String,
    parameters: Vec<(String, String)>,
    headers: Vec<(String, String)>,
}

impl Endpoint for OpenAIChatCompletionsConfig {
    fn render_settings(&mut self, ui: &mut Ui) {
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

        body.insert(
            "messages".to_string(),
            Value::Array(vec![Value::Object(Map::from_iter([
                ("role".to_string(), Value::String("assistant".to_string())),
                (
                    "content".to_string(),
                    Value::String(escaped_string_from_utf8(&request.content)),
                ),
            ]))]),
        );

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

        let endpoint_response = parse_openai_response(response, metadata);

        if !endpoint_response.is_empty() {
            Ok(endpoint_response)
        } else {
            Err(anyhow::Error::msg("Response does not match API schema"))
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

                                    for (token, prob) in tokens {
                                        let length = token.len();

                                        responses.push(EndpointResponse {
                                            content: InnerNodeContent::Tokens(vec![(
                                                token,
                                                IndexMap::from_iter([
                                                    ("probability".to_string(), prob.to_string()),
                                                    (
                                                        "original_length".to_string(),
                                                        length.to_string(),
                                                    ),
                                                ]),
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

                            for (token, logprob) in tokens.drain(..).zip(token_logprobs.drain(..)) {
                                if let Value::String(token) = token
                                    && let Value::Number(logprob) = logprob
                                    && let Some(logprob) = logprob.as_f64()
                                {
                                    output.push((
                                        token.into_bytes(),
                                        (logprob.exp() * 10000.0).round() / 10000.0,
                                    ));
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
                                    .map(|(token, prob)| {
                                        let length = token.len();

                                        (
                                            token,
                                            IndexMap::from_iter([
                                                ("probability".to_string(), prob.to_string()),
                                                ("original_length".to_string(), length.to_string()),
                                            ]),
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

fn parse_openai_logprob(mut logprob: Map<String, Value>, output: &mut Vec<(Vec<u8>, f64)>) {
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

    if let Some(token) = token
        && let Some(Value::Number(logprob)) = logprob.remove("logprob")
        && let Some(logprob) = logprob.as_f64()
    {
        output.push((token, (logprob.exp() * 10000.0).round() / 10000.0));
    }
}
