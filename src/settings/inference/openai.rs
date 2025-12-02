use eframe::egui::{TextEdit, Ui, Widget};
use log::debug;
use reqwest::{
    Client, Method, Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tapestry_weave::{universal_weave::indexmap::IndexMap, v0::InnerNodeContent};

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
        ]
    }
    async fn perform_request(
        &self,
        client: &Client,
        request: EndpointRequest,
    ) -> Result<EndpointResponse, anyhow::Error> {
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

        body.insert(
            "prompt".to_string(),
            Value::String(String::from_utf8_lossy(&request.content).to_string()),
        );

        let mut response: Map<String, Value> = client
            .request(Method::POST, Url::parse(&self.endpoint)?)
            .headers(headers)
            .json(&Value::Object(body))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let metadata = request.parameters.as_ref().clone();

        //debug!("{:#?}", response);

        if let Some(Value::String(text)) = response.remove("text") {
            return Ok(EndpointResponse {
                content: vec![InnerNodeContent::Snippet(text.into_bytes())],
                metadata,
            });
        }

        if let Some(Value::Array(choices)) = response.remove("choices") {
            let mut contents = Vec::with_capacity(choices.len());

            for choice in choices {
                if let Value::Object(mut choice) = choice {
                    let mut tokens = Vec::new();

                    if let Some(Value::Object(mut logprobs)) = choice.remove("logprobs") {
                        debug!("{}", serde_json::to_string_pretty(&logprobs).unwrap());

                        if let Some(Value::Array(logprobs_content)) = logprobs.remove("content") {
                            tokens.reserve(logprobs_content.len());

                            for (i, logprob_item) in logprobs_content.into_iter().enumerate() {
                                if let Value::Object(mut logprob_item) = logprob_item {
                                    let token = if let Some(bytes) = logprob_item
                                        .remove("bytes")
                                        .and_then(|v| serde_json::from_value::<Vec<u8>>(v).ok())
                                    {
                                        Some(bytes)
                                    } else if let Some(Value::String(token)) =
                                        logprob_item.remove("token")
                                    {
                                        Some(token.into_bytes())
                                    } else {
                                        None
                                    };

                                    if i == 0
                                        && let Some(Value::Array(top_logprobs)) =
                                            logprob_item.remove("top_logprobs")
                                        && top_logprobs.len() > 1
                                    {
                                        let mut tokens = Vec::with_capacity(top_logprobs.len());

                                        for top_logprob in top_logprobs {
                                            if let Value::Object(mut top_logprob) = top_logprob {
                                                let token = if let Some(bytes) =
                                                    top_logprob.remove("bytes").and_then(|v| {
                                                        serde_json::from_value::<Vec<u8>>(v).ok()
                                                    }) {
                                                    Some(bytes)
                                                } else if let Some(Value::String(token)) =
                                                    top_logprob.remove("token")
                                                {
                                                    Some(token.into_bytes())
                                                } else {
                                                    None
                                                };

                                                if let Some(token) = token
                                                    && let Some(Value::Number(logprob)) =
                                                        logprob_item.remove("logprob")
                                                    && let Some(logprob) = logprob.as_f64()
                                                {
                                                    tokens.push((
                                                        token,
                                                        (logprob.exp() * 10000.0).round() / 10000.0,
                                                    ));
                                                }
                                            }
                                        }

                                        tokens.sort_unstable_by(|a, b| a.1.total_cmp(&b.1));

                                        contents.reserve(tokens.len());

                                        contents.push(InnerNodeContent::Tokens(
                                            tokens
                                                .into_iter()
                                                .map(|(token, prob)| {
                                                    (
                                                        token,
                                                        IndexMap::from_iter([(
                                                            "probability".to_string(),
                                                            prob.to_string(),
                                                        )]),
                                                    )
                                                })
                                                .collect(),
                                        ));
                                    }

                                    if let Some(token) = token
                                        && let Some(Value::Number(logprob)) =
                                            logprob_item.remove("logprob")
                                        && let Some(logprob) = logprob.as_f64()
                                    {
                                        tokens.push((
                                            token,
                                            (logprob.exp() * 10000.0).round() / 10000.0,
                                        ));
                                    }
                                }
                            }
                        } else {
                            // TODO
                        }
                    }

                    if !tokens.is_empty() {
                        contents.push(InnerNodeContent::Tokens(
                            tokens
                                .into_iter()
                                .map(|(token, prob)| {
                                    (
                                        token,
                                        IndexMap::from_iter([(
                                            "probability".to_string(),
                                            prob.to_string(),
                                        )]),
                                    )
                                })
                                .collect(),
                        ));
                    } else if let Some(Value::String(text)) = choice.remove("text") {
                        contents.push(InnerNodeContent::Snippet(text.into_bytes()));
                    }
                }
            }

            if !contents.is_empty() {
                return Ok(EndpointResponse {
                    content: contents,
                    metadata,
                });
            }
        }

        Err(anyhow::Error::msg("Unimplemented"))
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

fn parse_openai_response(
    response: Map<String, Value>,
    metadata: &mut Vec<(String, String)>,
) -> Vec<InnerNodeContent> {
    todo!()
}
