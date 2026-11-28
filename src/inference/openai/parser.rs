use anyhow::anyhow;
use base64::{Engine, prelude::BASE64_STANDARD};
use log::trace;
use serde_json::{Map, Value};
use tapestry_weave::{
    CounterfactualToken, InnerNodeContent, InnerNodeToken, universal_weave::indexmap::IndexMap,
};

use crate::inference::EndpointResponse;

#[derive(Debug)]
pub struct CompletionOptions {
    pub parameters: Vec<(String, String)>,
    pub seed: Option<u64>,
    pub echo: bool,
    pub single_token: bool,
    pub requested_top: Option<usize>,
}

pub fn parse_completion_response(
    response: Value,
    options: CompletionOptions,
) -> Result<Vec<EndpointResponse>, anyhow::Error> {
    if let Value::Object(mut response) = response {
        if let Some(Value::Array(choices)) = response.remove("choices") {
            let system_fingerprint = if let Some(Value::String(system_fingerprint)) =
                response.remove("system_fingerprint")
                && !system_fingerprint.is_empty()
            {
                Some(system_fingerprint)
            } else {
                None
            };

            let mut items = Vec::with_capacity(choices.len());

            for choice in choices {
                if let Value::Object(value) = choice
                    && let Some(item) = parse_item(value)
                {
                    items.push(item);
                }
            }

            if !items.is_empty() {
                items.sort_by_key(|item| item.index);

                let base_metadata: Vec<_> = options
                    .parameters
                    .iter()
                    .filter(|&(key, _)| key != "seed")
                    .cloned()
                    .collect();

                return Ok(items
                    .into_iter()
                    .map(|mut item| {
                        let mut metadata = base_metadata.clone();

                        if let Some(role) = item.role
                            && role != "assistant"
                        {
                            metadata.push(("role".to_string(), role));
                        }

                        if options.single_token && !options.echo {
                            item.contents.truncate_tokens(1);
                        }

                        if let Some(requested_top) = options.requested_top {
                            item.contents.sort_counterfactual();
                            item.contents.truncate_counterfactual(requested_top);
                        }

                        EndpointResponse {
                            root: options.echo,
                            content: item.contents,
                            metadata,
                            seed: options.seed,
                            system_fingerprint: system_fingerprint.clone(),
                            finish_reason: item.finish_reason,
                        }
                    })
                    .collect());
            }
        } else if let Some(error) = response.remove("error") {
            return Err(anyhow!("API Error: {}", error));
        }
    }

    Err(anyhow!("Response does not match API schema"))
}

#[derive(Debug)]
pub struct ResponseItem {
    pub index: Option<u64>,
    pub role: Option<String>,
    pub finish_reason: Option<String>,
    pub contents: InnerNodeContent,
}

fn parse_item(mut json: Map<String, Value>) -> Option<ResponseItem> {
    let index = if let Some(Value::Number(index)) = json.remove("index")
        && let Some(index) = index.as_u64()
    {
        Some(index)
    } else {
        None
    };

    let finish_reason = if let Some(Value::String(finish_reason)) = json.remove("finish_reason") {
        Some(finish_reason)
    } else {
        None
    };

    let (text, role) = if let Some(Value::Object(mut message)) = json.remove("message") {
        (
            if let Some(content) = message.remove("content") {
                if content.is_null() {
                    Some(String::new())
                } else if let Value::String(content) = content {
                    Some(content)
                } else {
                    None
                }
            } else {
                Some(String::new())
            },
            if let Some(Value::String(role)) = message.remove("role") {
                Some(role)
            } else {
                None
            },
        )
    } else if let Some(Value::String(text)) = json.remove("text") {
        (Some(text), None)
    } else {
        (None, None)
    };

    Some(ResponseItem {
        index,
        role,
        finish_reason,
        contents: if let Some(Value::Object(logprobs)) = json.remove("logprobs")
            && let Some(parsed_logprobs) = parse_item_logprobs(logprobs, &mut json)
            && !parsed_logprobs.is_empty()
        {
            InnerNodeContent::Tokens(parsed_logprobs)
        } else {
            InnerNodeContent::Snippet(text?.into_bytes())
        },
    })
}

// llama.cpp returns chatcompletion logprobs format for completions too
fn parse_item_logprobs(
    mut logprobs: Map<String, Value>,
    json: &mut Map<String, Value>,
) -> Option<Vec<InnerNodeToken>> {
    // vllm specific
    let token_ids = json
        .remove("token_ids")
        .and_then(|v| serde_json::from_value::<Vec<u64>>(v).ok())
        // together and fireworks specific
        .or_else(|| {
            logprobs
                .remove("token_ids")
                .and_then(|v| serde_json::from_value::<Vec<u64>>(v).ok())
        });

    // vllm specific
    let prompt_token_ids = json
        .remove("prompt_token_ids")
        .and_then(|v| serde_json::from_value::<Vec<u64>>(v).ok());

    let tokens = if let Some(Value::Array(logprobs)) = logprobs.remove("content") {
        let parse_item = |mut item: Map<String, Value>| {
            Some(CounterfactualToken {
                bytes: if let Some(bytes) = item
                    .remove("bytes")
                    .and_then(|v| serde_json::from_value::<Option<Vec<u8>>>(v).ok())
                {
                    Some(bytes.unwrap_or_default())
                } else if let Some(Value::String(token)) = item.remove("token") {
                    Some(token.into_bytes())
                } else {
                    None
                }?,
                logprob: if let Some(Value::Number(logprob)) = item.remove("logprob")
                    && let Some(logprob) = logprob.as_f64()
                {
                    normalize_logprob(logprob as f32)
                } else if let Some(Value::Number(prob)) = item.remove("prob") // llama.cpp specific
                        && let Some(prob) = prob.as_f64()
                {
                    normalize_logprob(prob.ln() as f32)
                } else {
                    None
                },
                id: if let Some(Value::Number(id)) = item.remove("id") // llama.cpp specific
					&& let Some(id) = id.as_u64()
                {
                    Some(id)
                } else {
                    None
                },
            })
        };

        let logprob_count = logprobs.len();

        let logprobs: Vec<InnerNodeToken> = logprobs
            .into_iter()
            .filter_map(|logprob| {
                if let Value::Object(mut logprob) = logprob {
                    let counterfactual =
                        if let Some(Value::Array(top_logprobs)) = logprob.remove("top_logprobs") {
                            top_logprobs
                        } else if let Some(Value::Array(top_probs)) = logprob.remove("top_probs") {
                            // llama.cpp specific
                            top_probs
                        } else {
                            Vec::new()
                        };

                    Some(InnerNodeToken::from_counterfactual_pair(
                        parse_item(logprob)?,
                        counterfactual
                            .into_iter()
                            .filter_map(|logprob| {
                                if let Value::Object(logprob) = logprob {
                                    parse_item(logprob)
                                } else {
                                    None
                                }
                            })
                            .collect(),
                    ))
                } else {
                    None
                }
            })
            .collect();

        if logprobs.len() == logprob_count {
            Some(logprobs)
        } else {
            None
        }
    } else {
        None
    };

    let mut tokens = if let Some(tokens) = tokens {
        Some(tokens)
    } else if let Some(tokens) = logprobs
        .remove("tokens")
        .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok())
    {
        let parse_token = |token: String| {
            if let Some(escaped) = token.strip_prefix("bytes:")
                && let Some(bytes) = unescape_bytes(escaped)
            {
                bytes
            } else {
                token.into_bytes()
            }
        };

        let mut tokens: Vec<InnerNodeToken> = tokens
            .into_iter()
            .map(|token| {
                InnerNodeToken::from_counterfactual_pair(
                    CounterfactualToken {
                        bytes: parse_token(token),
                        logprob: None,
                        id: None,
                    },
                    Vec::new(),
                )
            })
            .collect();

        if let Some(token_logprobs) = logprobs
            .remove("token_logprobs")
            .and_then(|v| serde_json::from_value::<Vec<Option<f32>>>(v).ok())
            && token_logprobs.len() == tokens.len()
        {
            for (logprob, token) in token_logprobs.into_iter().zip(tokens.iter_mut()) {
                token.logprob = logprob.and_then(normalize_logprob);
            }
        }

        if let Some(top_logprobs) = logprobs.remove("top_logprobs").and_then(|v| {
            serde_json::from_value::<Vec<Option<IndexMap<String, Option<f32>>>>>(v).ok()
        }) && top_logprobs.len() == tokens.len()
        {
            for (top_logprobs, token) in top_logprobs.into_iter().zip(tokens.iter_mut()) {
                if let Some(top_logprobs) = top_logprobs {
                    token.counterfactual.extend(top_logprobs.into_iter().map(
                        |(token, logprob)| CounterfactualToken {
                            bytes: parse_token(token),
                            logprob: logprob.and_then(normalize_logprob),
                            id: None,
                        },
                    ));
                }
            }
        }

        Some(tokens)
    } else {
        None
    }?;

    // vllm specific
    if let Some(token_ids) = token_ids {
        if token_ids.len() == tokens.len() {
            for (id, token) in token_ids.into_iter().zip(tokens.iter_mut()) {
                token.id = Some(id);
            }
        } else if let Some(prompt_token_ids) = prompt_token_ids
            && prompt_token_ids.len() + token_ids.len() == tokens.len()
        {
            for (id, token) in prompt_token_ids
                .into_iter()
                .chain(token_ids)
                .zip(tokens.iter_mut())
            {
                token.id = Some(id);
            }
        }
    }

    Some(tokens)
}

fn unescape_bytes(escaped: &str) -> Option<Vec<u8>> {
    if escaped.is_empty() || !escaped.len().is_multiple_of(4) {
        return None;
    }

    let chunks = escaped.as_bytes().as_chunks::<4>().0;
    let mut bytes = Vec::with_capacity(chunks.len());

    for chunk in chunks {
        if chunk[0] != b'\\' || chunk[1] != b'x' {
            return None;
        }

        let high = char::from(chunk[2]).to_digit(16)?;
        let low = char::from(chunk[3]).to_digit(16)?;

        bytes.push((high * 16 + low) as u8);
    }

    Some(bytes)
}

fn normalize_logprob(logprob: f32) -> Option<f32> {
    // vllm and openai use -9999.0 for zero probabilities
    if logprob.is_finite() && logprob > -9999.0 {
        Some(logprob)
    } else {
        None
    }
}

pub fn parse_embedding_response(response: Value) -> Result<Vec<f32>, anyhow::Error> {
    trace!("{:#?}", response);

    if let Value::Object(mut response) = response
        && let Some(Value::Array(mut data)) = response.remove("data")
        && data.len() == 1
        && let Value::Object(mut data) = data.swap_remove(0)
        && let Some(embedding) = data.remove("embedding")
    {
        if let Value::String(embedding) = embedding {
            if let Ok(embedding) = BASE64_STANDARD.decode(embedding)
                && !embedding.is_empty()
                && embedding.len().is_multiple_of(4)
            {
                return Ok(embedding
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|chunk| f32::from_le_bytes(*chunk))
                    .collect());
            }
        } else if let Ok(embedding) = serde_json::from_value::<Vec<f32>>(embedding)
            && !embedding.is_empty()
        {
            return Ok(embedding);
        }
    }

    Err(anyhow!("Response does not match API schema"))
}
