// A robust completion API response parser which aims to support as many different APIs as possible

use std::ops::RangeBounds;

use serde_json::{Map, Value};

pub struct ResponseChoice {
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
    fn cleanup(&mut self, requested_top: usize) {
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

fn parse_openai_logprobs(
    mut logprobs_json: Map<String, Value>,
    requested_top: Option<usize>,
) -> Option<Vec<Token>> {
    // vllm: token_ids is in contents

    todo!()
}

fn parse_openai_completion_logprobs(
    mut logprobs_json: Map<String, Value>,
    text: Option<String>,
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
            let bytes = text.into_bytes();

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

        if let Some(Value::Array(tokens)) = logprobs_json.remove("tokens")
            && token_list.is_empty()
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

    todo!()
}

fn parse_openai_chatcompletion_logprob_content(
    mut logprob_json: Map<String, Value>,
) -> Option<Token> {
    let mut top_tokens = Vec::new();

    if let Some(Value::Array(top_logprobs_json)) = logprob_json.remove("top_logprobs") {
        top_tokens.reserve_exact(top_logprobs_json.len());

        for top_logprob_json in top_logprobs_json.into_iter() {
            if let Value::Object(top_logprob_json) = top_logprob_json
                && let Some(top_logprob) =
                    parse_openai_chatcompletion_logprob_content_item(top_logprob_json)
            {
                top_tokens.push(top_logprob);
            } else {
                top_tokens.clear();
                top_tokens.shrink_to_fit();
                break;
            }
        }
    }

    parse_openai_chatcompletion_logprob_content_item(logprob_json)
        .map(|token| Token { token, top_tokens })
}

fn parse_openai_chatcompletion_logprob_content_item(
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
