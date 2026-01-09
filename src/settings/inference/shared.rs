use log::trace;
use reqwest::Response;
use serde_json::{Map, Value};
use tapestry_weave::{
    ulid::Ulid,
    v0::{InnerNodeContent, MetadataMap},
};

use super::{EndpointResponse, polyparser};

pub(super) fn build_json_list(list: &mut Vec<Value>, items: Vec<String>) {
    for item in items {
        if let Ok(value) = serde_json::from_str(&item) {
            list.push(value);
        } else {
            list.push(Value::String(item));
        }
    }
}

pub(super) fn build_json_object(map: &mut Map<String, Value>, parameters: Vec<(String, String)>) {
    for (key, value) in parameters {
        if let Ok(value) = serde_json::from_str(&value) {
            map.insert(key, value);
        } else {
            map.insert(key, Value::String(value));
        }
    }
}

pub(super) async fn error_for_status(response: Response) -> Result<Response, anyhow::Error> {
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

pub(super) fn parse_response(
    response: Map<String, Value>,
    metadata: Vec<(String, String)>,
    tokenization_identifier: Ulid,
    echo: bool,
    single_token: bool,
    requested_top: Option<usize>,
) -> Vec<EndpointResponse> {
    trace!("{:#?}", &response);

    let items = polyparser::parse_response(response, requested_top);

    let mut outputs = Vec::with_capacity(items.len());

    for mut item in items {
        item.clear_normal();

        let mut metadata = metadata.clone();

        let mut metadata_capacity = 0;

        if item.role.is_some() {
            metadata_capacity += 1;
        }

        if item.finish_reason.is_some() {
            metadata_capacity += 1;
        }

        if metadata_capacity > 0 {
            metadata.reserve_exact(metadata_capacity);
        }

        if let Some(role) = item.role {
            metadata.push(("role".to_string(), role));
        }

        if let Some(finish_reason) = item.finish_reason {
            metadata.push(("finish_reason".to_string(), finish_reason));
        }

        match item.contents {
            polyparser::ResponseContents::Text(text) => outputs.push(EndpointResponse {
                root: echo,
                content: InnerNodeContent::Snippet(text),
                metadata,
            }),
            polyparser::ResponseContents::Tokens(tokens) => {
                if single_token && let Some(token) = tokens.first().cloned() {
                    let mut base_token_metadata = Vec::new();

                    if token.top_tokens.len() >= 10 {
                        let mut confidence = 0.0;

                        let top_token_count = token.top_tokens.len();

                        for top_token in &token.top_tokens {
                            confidence += top_token.logprob;
                        }

                        confidence /= top_token_count as f64;

                        base_token_metadata.extend([
                            (
                                "confidence".to_string(),
                                ((confidence * -100.0).round() / 100.0).to_string(),
                            ),
                            ("confidence_k".to_string(), top_token_count.to_string()),
                        ]);
                    }

                    outputs.extend(token.top_tokens.into_iter().map(|top_token| {
                        let mut token_metadata_capacity = 2 + base_token_metadata.len();

                        if top_token.id.is_some() {
                            token_metadata_capacity += 2;
                        }

                        let mut token_metadata = MetadataMap::default();
                        token_metadata.reserve_exact(token_metadata_capacity);

                        token_metadata.extend([
                            (
                                "probability".to_string(),
                                ((top_token.logprob.exp() * 10000.0).round() / 10000.0).to_string(),
                            ),
                            (
                                "original_length".to_string(),
                                top_token.contents.len().to_string(),
                            ),
                        ]);

                        token_metadata.extend(base_token_metadata.clone());

                        if let Some(token_id) = top_token.id {
                            token_metadata.extend([
                                ("token_id".to_string(), token_id.to_string()),
                                ("model_id".to_string(), tokenization_identifier.to_string()),
                            ]);
                        }

                        EndpointResponse {
                            root: false,
                            content: InnerNodeContent::Tokens(vec![(
                                top_token.contents,
                                token_metadata,
                            )]),
                            metadata: metadata.clone(),
                        }
                    }));
                }

                let tokens = tokens
                    .into_iter()
                    .map(|token| {
                        let mut token_metadata_capacity = 2;

                        if token.token.id.is_some() {
                            token_metadata_capacity += 2;
                        }

                        if token.top_tokens.len() >= 10 {
                            token_metadata_capacity += 2;
                        }

                        let mut token_metadata = MetadataMap::default();
                        token_metadata.reserve_exact(token_metadata_capacity);

                        token_metadata.extend([
                            (
                                "probability".to_string(),
                                ((token.token.logprob.exp() * 10000.0).round() / 10000.0)
                                    .to_string(),
                            ),
                            (
                                "original_length".to_string(),
                                token.token.contents.len().to_string(),
                            ),
                        ]);

                        if token.top_tokens.len() >= 10 {
                            let mut confidence = 0.0;

                            let top_token_count = token.top_tokens.len();

                            for top_token in token.top_tokens {
                                confidence += top_token.logprob;
                            }

                            confidence /= top_token_count as f64;

                            token_metadata.extend([
                                (
                                    "confidence".to_string(),
                                    ((confidence * -100.0).round() / 100.0).to_string(),
                                ),
                                ("confidence_k".to_string(), top_token_count.to_string()),
                            ]);
                        }

                        if let Some(token_id) = token.token.id {
                            token_metadata.extend([
                                ("token_id".to_string(), token_id.to_string()),
                                ("model_id".to_string(), tokenization_identifier.to_string()),
                            ]);
                        }

                        (token.token.contents, token_metadata)
                    })
                    .collect();

                outputs.push(EndpointResponse {
                    root: echo,
                    content: InnerNodeContent::Tokens(tokens),
                    metadata,
                });
            }
            polyparser::ResponseContents::Empty => outputs.push(EndpointResponse {
                root: echo,
                content: InnerNodeContent::Snippet(Vec::new()),
                metadata,
            }),
        };
    }

    outputs
}

pub(super) fn convert_embedding_response(mut response: Vec<Option<Vec<f32>>>) -> Option<Vec<f32>> {
    if response.len() == 1 {
        response.remove(0)
    } else {
        None
    }
}
