/* A robust completion API response parser which aims to support as many different APIs and API implementations as possible, while continuing to be somewhat resilient to malformed responses

However, it intentionally omits the following features:
- Usage tracking
- Tool calling
- Refusal messages
- Reasoning outputs
- Multimodal outputs

Based on the following:
- https://platform.openai.com/docs/api-reference/completions/object
- https://platform.openai.com/docs/api-reference/chat/object
- https://platform.openai.com/docs/api-reference/chat-streaming/streaming
- llama-cpp experimentation
- vllm experimentation
*/

use serde_json::{Map, Value};

pub struct ResponseItem {
    index: Option<usize>,
    role: Option<String>,
    finish_reason: Option<String>,
    contents: ResponseContents,
}

enum ResponseContents {
    Text(Vec<u8>),
    Tokens(Vec<Token>),
}

pub struct Token {
    pub token: LogprobToken,
    pub top_tokens: Vec<LogprobToken>,
}

impl Token {
    pub fn cleanup(&mut self, requested_top: usize) {
        if self.top_tokens.len() == requested_top + 1 {
            let index = self
                .top_tokens
                .iter()
                .enumerate()
                .find(|(_index, top_token)| {
                    top_token.id == self.token.id
                        && top_token.contents == self.token.contents
                        && top_token.logprob == self.token.logprob
                })
                .map(|(index, _)| index);

            if let Some(index) = index {
                self.top_tokens.remove(index);
                self.top_tokens.shrink_to_fit();
            }
        }
    }
}

pub struct LogprobToken {
    pub id: Option<i128>,
    pub contents: Vec<u8>,
    pub logprob: f64,
}

fn parse(mut json: Map<String, Value>) -> Vec<ResponseItem> {
    let mut items = Vec::new();

    if let Some(Value::Array(choices)) = json.remove("choices") {
        items.reserve_exact(choices.len());

        for choice in choices {
            if let Value::Object(choice) = choice
                && let Some(item) = parse_item(choice)
            {
                items.push(item);
            }
        }
    } else if let Some(Value::Array(output)) = json.remove("output") {
        todo!()
    } else if let Some(item) = parse_item(json) {
        items.push(item);
    }

    items
}

fn parse_item(mut json: Map<String, Value>) -> Option<ResponseItem> {
    let index = if let Some(Value::Number(index)) = json.remove("index")
        && let Some(index) = index.as_u64()
    {
        Some(index as usize)
    } else {
        None
    };

    let mut finish_reason = if let Some(Value::String(finish_reason)) = json.remove("finish_reason")
    {
        Some(finish_reason)
    } else {
        None
    };

    let mut role = if let Some(Value::String(role)) = json.remove("role") {
        Some(role)
    } else {
        None
    };

    let tokens = if let Some(Value::Object(mut logprobs_json)) = json.remove("logprobs") {
        if let Some(Value::Array(logprobs_list_json)) = logprobs_json.remove("content") {
            parse_openai_chatcompletion_logprobs_content(logprobs_list_json)
        } else {
            let token_ids = json
                .remove("token_ids") // vllm
                .and_then(|item| {
                    if let Value::Array(item) = item {
                        Some(item)
                    } else {
                        None
                    }
                });

            let text = if let Some(Value::String(text)) = json.get("text") {
                Some(text.as_ref())
            } else {
                None
            };

            parse_openai_completion_logprobs(logprobs_json, text, token_ids)
        }
    } else {
        None
    };

    if let Some(tokens) = tokens {
        Some(ResponseItem {
            index,
            role,
            finish_reason,
            contents: ResponseContents::Tokens(tokens),
        })
    } else if let Some(Value::String(text)) = json.remove("text") {
        Some(ResponseItem {
            index,
            role,
            finish_reason,
            contents: ResponseContents::Text(text.into_bytes()),
        })
    } else if let Some(Value::Object(mut message)) = json.remove("message")
        && let Some(Value::String(content)) = message.remove("content")
    {
        if let Some(Value::String(role_value)) = message.remove("role") {
            role = Some(role_value);
        }

        if finish_reason.is_none()
            && let Some(Value::String(finish_reason_value)) = message.remove("finish_reason")
        {
            finish_reason = Some(finish_reason_value);
        }

        Some(ResponseItem {
            index,
            role,
            finish_reason,
            contents: ResponseContents::Text(content.into_bytes()),
        })
    } else if let Some(Value::Object(mut delta)) = json.remove("delta")
        && let Some(Value::String(content)) = delta.remove("content")
    {
        if let Some(Value::String(role_value)) = delta.remove("role") {
            role = Some(role_value);
        }

        if finish_reason.is_none()
            && let Some(Value::String(finish_reason_value)) = delta.remove("finish_reason")
        {
            finish_reason = Some(finish_reason_value);
        }

        Some(ResponseItem {
            index,
            role,
            finish_reason,
            contents: ResponseContents::Text(content.into_bytes()),
        })
    } else {
        todo!()
    }
}

fn parse_openai_completion_logprobs(
    mut logprobs_json: Map<String, Value>,
    text: Option<&str>,
    token_ids: Option<Vec<Value>>,
) -> Option<Vec<Token>> {
    let mut top_tokens_list = Vec::new();

    if let Some(Value::Array(top_logprobs_json)) = logprobs_json.remove("top_logprobs") {
        top_tokens_list.reserve_exact(top_logprobs_json.len());

        for top_logprob_json in top_logprobs_json.into_iter() {
            let mut top_tokens = Vec::new();

            if let Value::Object(top_logprob_json) = top_logprob_json {
                top_tokens.reserve_exact(top_logprob_json.len());

                for (contents, logprob) in top_logprob_json {
                    if let Value::Number(logprob) = logprob
                        && let Some(logprob) = logprob.as_f64()
                    {
                        top_tokens.push(LogprobToken {
                            id: None,
                            contents: contents.into_bytes(),
                            logprob,
                        });
                    } else {
                        top_tokens.clear();
                        top_tokens.shrink_to_fit();
                        break;
                    }
                }
            }

            top_tokens_list.push(top_tokens);
        }
    }

    let mut token_id_list = Vec::new();

    if let Some(token_ids) = token_ids.or_else(|| {
        logprobs_json
            .remove("token_ids") // unknown origin
            .and_then(|item| {
                if let Value::Array(item) = item {
                    Some(item)
                } else {
                    None
                }
            })
    }) {
        token_id_list.reserve_exact(token_ids.len());

        for token_id in token_ids {
            if let Value::Number(id) = token_id
                && let Some(id) = id.as_i128()
            {
                token_id_list.push(id);
            } else {
                token_id_list.clear();
                token_id_list.shrink_to_fit();
                break;
            }
        }
    }

    let mut token_list = Vec::new();

    if let Some(Value::Array(token_logprobs)) = logprobs_json.remove("token_logprobs") {
        token_list.reserve_exact(token_logprobs.len());

        if let Some(Value::Array(text_offset)) = logprobs_json.remove("text_offset")
            && let Some(text) = text
            && text_offset.len() == token_logprobs.len()
        {
            let bytes = text.as_bytes();

            for index in 0..token_logprobs.len() {
                let next_text_offset = text_offset.get(index + 1);

                if let Value::Number(text_offset) = &text_offset[index]
                    && let Value::Number(logprob) = &token_logprobs[index]
                    && let Some(text_offset) = text_offset.as_u64()
                    && let Some(logprob) = logprob.as_f64()
                {
                    let contents = if let Some(next_text_offset) = next_text_offset {
                        if let Value::Number(next_text_offset) = next_text_offset
                            && let Some(next_text_offset) = next_text_offset.as_u64()
                        {
                            bytes.get(text_offset as usize..next_text_offset as usize)
                        } else {
                            None
                        }
                    } else {
                        bytes.get(text_offset as usize..)
                    };

                    if let Some(contents) = contents.map(|contents| contents.to_owned()) {
                        token_list.push(LogprobToken {
                            id: None,
                            contents,
                            logprob,
                        });
                    } else {
                        token_list.clear();
                        token_list.shrink_to_fit();
                        break;
                    }
                } else {
                    token_list.clear();
                    token_list.shrink_to_fit();
                    break;
                }
            }
        }

        if token_list.is_empty()
            && let Some(Value::Array(tokens)) = logprobs_json.remove("tokens")
            && tokens.len() == token_logprobs.len()
        {
            for (token, logprob) in tokens.into_iter().zip(token_logprobs.into_iter()) {
                if let Value::String(token) = token
                    && let Value::Number(logprob) = logprob
                    && let Some(logprob) = logprob.as_f64()
                {
                    token_list.push(LogprobToken {
                        id: None,
                        contents: token.into_bytes(),
                        logprob,
                    });
                } else {
                    return None;
                }
            }
        }
    } else {
        return None;
    }

    if token_list.len() == token_id_list.len() {
        for (token, token_id) in token_list.iter_mut().zip(token_id_list.into_iter()) {
            token.id = Some(token_id);
        }
    }

    if token_list.len() == top_tokens_list.len() {
        Some(
            token_list
                .into_iter()
                .zip(top_tokens_list)
                .map(|(token, top_tokens)| Token { token, top_tokens })
                .collect(),
        )
    } else {
        Some(
            token_list
                .into_iter()
                .map(|token| Token {
                    token,
                    top_tokens: Vec::new(),
                })
                .collect(),
        )
    }
}

fn parse_openai_chatcompletion_logprobs_content(
    logprobs_list_json: Vec<Value>,
) -> Option<Vec<Token>> {
    let mut tokens = Vec::with_capacity(logprobs_list_json.len());

    for logprob_json in logprobs_list_json {
        if let Value::Object(logprob_json) = logprob_json
            && let Some(token) = parse_openai_chatcompletion_logprob_content_item(logprob_json)
        {
            tokens.push(token);
        } else {
            return None;
        }
    }

    Some(tokens)
}

fn parse_openai_chatcompletion_logprob_content_item(
    mut logprob_json: Map<String, Value>,
) -> Option<Token> {
    let mut top_tokens = Vec::new();

    if let Some(Value::Array(top_logprobs_json)) = logprob_json.remove("top_logprobs") {
        top_tokens.reserve_exact(top_logprobs_json.len());

        for top_logprob_json in top_logprobs_json.into_iter() {
            if let Value::Object(top_logprob_json) = top_logprob_json
                && let Some(top_logprob) =
                    parse_openai_chatcompletion_logprob_content_subitem(top_logprob_json)
            {
                top_tokens.push(top_logprob);
            } else {
                top_tokens.clear();
                top_tokens.shrink_to_fit();
                break;
            }
        }
    }

    parse_openai_chatcompletion_logprob_content_subitem(logprob_json)
        .map(|token| Token { token, top_tokens })
}

fn parse_openai_chatcompletion_logprob_content_subitem(
    mut logprob_json: Map<String, Value>,
) -> Option<LogprobToken> {
    let contents = if let Some(bytes) = logprob_json
        .remove("bytes")
        .and_then(|v| serde_json::from_value::<Vec<u8>>(v).ok())
        && !bytes.is_empty()
    {
        Some(bytes)
    } else if let Some(Value::String(token)) = logprob_json.remove("token") {
        Some(token.into_bytes())
    } else {
        None
    };

    let id = if let Some(Value::Number(id)) = logprob_json.remove("id") // llama-cpp
        && let Some(id) = id.as_i128()
    {
        Some(id)
    } else {
        None
    };

    if let Some(contents) = contents {
        if let Some(Value::Number(logprob)) = logprob_json.remove("logprob")
            && let Some(logprob) = logprob.as_f64()
        {
            Some(LogprobToken {
                id,
                contents,
                logprob,
            })
        } else if let Some(Value::Number(prob)) = logprob_json.remove("prob") // llama-cpp
            && let Some(prob) = prob.as_f64()
        {
            Some(LogprobToken {
                id,
                contents,
                logprob: prob.ln(),
            })
        } else {
            None
        }
    } else {
        None
    }
}
