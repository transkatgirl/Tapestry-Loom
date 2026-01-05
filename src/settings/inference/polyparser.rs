/* WIP!

A robust completion API response parser which aims to support as many different APIs and API implementations as possible, while continuing to be somewhat resilient to malformed responses

However, it intentionally omits the following features:
- Usage tracking
- Tool calling
- Refusal messages
- Multimodal outputs
- Reasoning outputs
- Structured outputs
- Output annotations

Based on the following:
- https://platform.openai.com/docs/api-reference/completions/object
    - TODO: Handle "echo": true
- https://platform.openai.com/docs/api-reference/chat/object
- https://platform.openai.com/docs/api-reference/chat-streaming/streaming
- https://platform.openai.com/docs/api-reference/responses/object
- https://platform.openai.com/docs/api-reference/responses-streaming/response/output_text/delta
- https://platform.openai.com/docs/api-reference/responses-streaming/response/incomplete
- https://platform.openai.com/docs/api-reference/responses-streaming/response/failed
- https://platform.openai.com/docs/api-reference/responses-streaming/response/completed
- https://platform.openai.com/docs/api-reference/embeddings
- https://platform.claude.com/docs/en/api/completions/create
- https://platform.claude.com/docs/en/api/messages/create
- https://platform.claude.com/docs/en/build-with-claude/streaming
- https://ai.google.dev/api/palm
- https://ai.google.dev/api/generate-content
- https://ai.google.dev/api/embeddings
- https://docs.ollama.com/api/generate
- https://docs.ollama.com/api/chat
- https://docs.ollama.com/api/embed
- llama-cpp experimentation
- vllm experimentation

TODO:

- do testing with llama-cpp
- do testing with koboldcpp
- do testing with vllm
- do testing with sglang
- do testing with ollama
- unit tests
*/

use log::trace;
use serde_json::{Map, Value};

#[derive(Debug)]
pub struct ResponseItem {
    pub index: Option<usize>,
    pub role: Option<String>,
    pub finish_reason: Option<String>,
    pub contents: ResponseContents,
}

impl ResponseItem {
    pub fn clear_normal(&mut self) {
        if let Some(role) = &self.role
            && role == "assistant"
        {
            self.role = None;
        }

        if let Some(finish_reason) = &self.finish_reason
            && (finish_reason == "stop"
                || finish_reason == "stop_sequence"
                || finish_reason == "end_turn"
                || finish_reason == "completed")
        {
            self.finish_reason = None;
        }
    }
    pub fn remove_selected_from_top(&mut self, requested_top: usize) {
        if let ResponseContents::Tokens(tokens) = &mut self.contents {
            for token in tokens {
                token.remove_selected_from_top(requested_top);
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ResponseContents {
    Text(Vec<u8>),
    Tokens(Vec<Token>),
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token: LogprobToken,
    pub top_tokens: Vec<LogprobToken>,
}

impl Token {
    pub fn remove_selected_from_top(&mut self, requested_top: usize) {
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

#[derive(Debug, Clone, PartialEq)]
pub struct LogprobToken {
    pub id: Option<i128>,
    pub contents: Vec<u8>,
    pub logprob: f64,
}

/*pub fn parse_embedding_response(mut json: Map<String, Value>) -> Vec<Option<Vec<f32>>> {
    if let Some(embedding) = json.remove("embedding") {
        vec![if let Value::Object(mut embedding) = embedding {
            embedding
                .remove("values")
                .and_then(|v| serde_json::from_value::<Vec<f32>>(v).ok())
        } else {
            serde_json::from_value::<Vec<f32>>(embedding).ok()
        }]
    } else if let Some(Value::Array(embeddings)) = json.remove("embeddings") {
        embeddings
            .into_iter()
            .map(|embedding| {
                if let Value::Object(mut embedding) = embedding {
                    embedding
                        .remove("values")
                        .and_then(|v| serde_json::from_value::<Vec<f32>>(v).ok())
                } else {
                    serde_json::from_value::<Vec<f32>>(embedding).ok()
                }
            })
            .collect()
    } else if let Some(Value::Array(data)) = json.remove("data") {
        let mut items = Vec::with_capacity(data.len());

        for item in data {
            if let Value::Object(item) = item {
                items.extend(parse_embedding_response(item));
            } else {
                items.push(None);
            }
        }

        items
    } else {
        vec![]
    }
}*/

pub fn parse_response(mut json: Map<String, Value>) -> Vec<ResponseItem> {
    let mut items = Vec::with_capacity(1);

    if let Some(Value::String(output_object)) = json.get("object") {
        match output_object.as_ref() {
            "text_completion" | "chat.completion" | "chat.completion.chunk" => {
                if let Some(Value::Array(choices)) = json.remove("choices") {
                    items.reserve_exact(choices.len());

                    for choice in choices {
                        if let Value::Object(choice) = choice
                            && let Some(item) = parse_item(choice)
                        {
                            items.push(item);
                        }
                    }
                }
            }
            "response" => {
                if let Some(Value::Array(output)) = json.remove("output") {
                    let mut item_sum = ResponseItem {
                        index: None,
                        role: None,
                        finish_reason: None,
                        contents: ResponseContents::Empty,
                    };

                    for output in output {
                        if let Value::Object(output) = output
                            && let Some(Value::String(output_type)) = output.get("type")
                            && output_type == "message"
                            && let Some(Value::Object(content)) = output.get("content")
                            && let Some(Value::String(content_type)) = content.get("type")
                            && content_type == "output_text"
                            && let Some(item) = parse_item(output)
                        {
                            if item.index.is_some() {
                                break;
                            }

                            if let Some(role) = item.role {
                                if let Some(sum_role) = &item_sum.role
                                    && *sum_role != role
                                {
                                    break;
                                } else {
                                    item_sum.role = Some(role);
                                }
                            }

                            match item.contents {
                                ResponseContents::Tokens(tokens) => match &mut item_sum.contents {
                                    ResponseContents::Tokens(sum_tokens) => {
                                        sum_tokens.extend(tokens);
                                    }
                                    ResponseContents::Text(_) => {
                                        break;
                                    }
                                    ResponseContents::Empty => {}
                                },
                                ResponseContents::Text(text) => match &mut item_sum.contents {
                                    ResponseContents::Tokens(_) => {
                                        break;
                                    }
                                    ResponseContents::Text(sum_text) => {
                                        sum_text.extend(text);
                                    }
                                    ResponseContents::Empty => {}
                                },
                                ResponseContents::Empty => {}
                            }

                            if let Some(finish_reason) = item.finish_reason {
                                item_sum.finish_reason = Some(finish_reason);
                                break;
                            }
                        }
                    }

                    if let Some(Value::String(status)) = json.remove("status") {
                        if status == "in_progress" {
                            item_sum.finish_reason = None;
                        } else if status == "incomplete" {
                            if let Some(Value::Object(mut incomplete_details)) =
                                json.remove("incomplete_details")
                                && let Some(Value::String(reason)) =
                                    incomplete_details.remove("reason")
                            {
                                item_sum.finish_reason = Some(reason);
                            } else {
                                item_sum.finish_reason = Some("incomplete".to_string())
                            }
                        } else if status == "failed" {
                            item_sum.finish_reason = Some("failed".to_string());
                        }
                    }
                    items.push(item_sum);
                }
            }
            _ => {
                if let Some(item) = parse_item(json) {
                    items.push(item);
                }
            }
        }
    } else if let Some(Value::String(output_type)) = json.get("type") {
        match output_type.as_ref() {
            "response.output_text.delta" => {
                if let Some(Value::Object(response)) = json.remove("response") {
                    return parse_response(response);
                }
            }
            "response.completed" => {
                items.push(ResponseItem {
                    index: None,
                    role: None,
                    finish_reason: Some("completed".to_string()),
                    contents: ResponseContents::Empty,
                });
            }
            "response.failed" => {
                items.push(ResponseItem {
                    index: None,
                    role: None,
                    finish_reason: Some("failed".to_string()),
                    contents: ResponseContents::Empty,
                });
            }
            "response.incomplete" => {
                items.push(ResponseItem {
                    index: None,
                    role: if let Some(Value::Object(mut response)) = json.remove("response")
                        && let Some(Value::Object(mut incomplete_details)) =
                            response.remove("incomplete_details")
                        && let Some(Value::String(reason)) = incomplete_details.remove("reason")
                    {
                        Some(reason)
                    } else {
                        Some("incomplete".to_string())
                    },
                    finish_reason: None,
                    contents: ResponseContents::Empty,
                });
            }
            "message_start" => {
                if let Some(Value::Object(mut message)) = json.remove("message")
                    && let Some(Value::String(role)) = message.remove("role")
                {
                    items.push(ResponseItem {
                        index: None,
                        role: Some(role),
                        finish_reason: None,
                        contents: ResponseContents::Empty,
                    });
                }
            }
            "message_delta" => {
                if let Some(Value::Object(mut delta)) = json.remove("delta")
                    && let Some(Value::String(stop_reason)) = delta.remove("stop_reason")
                {
                    items.push(ResponseItem {
                        index: None,
                        role: None,
                        finish_reason: Some(stop_reason),
                        contents: ResponseContents::Empty,
                    });
                }
            }
            "content_block_delta" => {
                if let Some(Value::Object(mut delta)) = json.remove("delta")
                    && let Some(Value::String(delta_type)) = delta.remove("type")
                    && delta_type == "text_delta"
                    && let Some(Value::String(text)) = delta.remove("text")
                {
                    items.push(ResponseItem {
                        index: None,
                        role: None,
                        finish_reason: None,
                        contents: ResponseContents::Text(text.into_bytes()),
                    });
                }
            }
            _ => {
                if let Some(item) = parse_item(json) {
                    items.push(item);
                }
            }
        }
    } else if let Some(Value::Array(choices)) = json.remove("choices") {
        items.reserve_exact(choices.len());

        for choice in choices {
            if let Value::Object(choice) = choice
                && let Some(item) = parse_item(choice)
            {
                items.push(item);
            }
        }
    } else if let Some(Value::Array(candidates)) = json.remove("candidates") {
        items.reserve_exact(candidates.len());

        for candidate in candidates {
            if let Value::Object(candidate) = candidate
                && let Some(item) = parse_item(candidate)
            {
                items.push(item);
            }
        }
    } else if let Some(item) = parse_item(json) {
        items.push(item);
    }

    trace!("{:#?}", &items);

    items
}

fn parse_item(mut json: Map<String, Value>) -> Option<ResponseItem> {
    if let Some(Value::String(output_type)) = json.get("type")
        && !(output_type == "message"
            || output_type == "response.output_text.delta"
            || output_type == "completion")
    {
        return None;
    }

    let index = if let Some(Value::Number(index)) = json.remove("index")
        && let Some(index) = index.as_u64()
    {
        Some(index as usize)
    } else if let Some(Value::Number(output_index)) = json.remove("output_index")
        && let Some(output_index) = output_index.as_u64()
    {
        Some(output_index as usize)
    } else {
        None
    };

    let mut finish_reason = if let Some(Value::String(finish_reason)) = json.remove("finish_reason")
    {
        Some(finish_reason)
    } else if let Some(Value::String(stop_reason)) = json.remove("stop_reason") {
        Some(stop_reason)
    } else if let Some(Value::String(finish_reason)) = json.remove("finishReason") {
        Some(finish_reason)
    } else if let Some(Value::String(done_reason)) = json.remove("done_reason") {
        Some(done_reason)
    } else {
        None
    };

    if let Some(Value::String(status)) = json.remove("status") {
        if status == "in_progress" {
            finish_reason = None;
        } else if status == "incomplete" {
            if let Some(Value::Object(mut incomplete_details)) = json.remove("incomplete_details")
                && let Some(Value::String(reason)) = incomplete_details.remove("reason")
            {
                finish_reason = Some(reason);
            } else {
                finish_reason = Some("incomplete".to_string())
            }
        } else if status == "failed" {
            finish_reason = Some("failed".to_string());
        }
    }

    if let Some(Value::Bool(done)) = json.remove("done") {
        if done {
            if finish_reason.is_none() {
                finish_reason = Some("completed".to_string())
            }
        } else if finish_reason.is_some() {
            finish_reason = None;
        }
    }

    let mut role = if let Some(Value::String(role)) = json.remove("role") {
        Some(role)
    } else if let Some(Value::String(author)) = json.remove("author") {
        Some(author)
    } else {
        None
    };

    let tokens = if let Some(logprobs_json) = json.remove("logprobs") {
        if let Value::Object(mut logprobs_json) = logprobs_json {
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
        } else if let Value::Array(logprobs_list_json) = logprobs_json {
            parse_openai_chatcompletion_logprobs_content(logprobs_list_json)
        } else {
            None
        }
    } else if let Some(Value::Object(logprobs_json)) = json.remove("logprobsResult") {
        parse_gemma_logprobs(logprobs_json)
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
    } else if let Some(Value::String(output)) = json.remove("output") {
        Some(ResponseItem {
            index,
            role,
            finish_reason,
            contents: ResponseContents::Text(output.into_bytes()),
        })
    } else if let Some(Value::String(completion)) = json.remove("completion") {
        Some(ResponseItem {
            index,
            role,
            finish_reason,
            contents: ResponseContents::Text(completion.into_bytes()),
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
    } else if let Some(delta) = json.remove("delta") {
        match delta {
            Value::Object(mut delta) => {
                if let Some(Value::String(role_value)) = delta.remove("role") {
                    role = Some(role_value);
                }

                if finish_reason.is_none()
                    && let Some(Value::String(finish_reason_value)) = delta.remove("finish_reason")
                {
                    finish_reason = Some(finish_reason_value);
                }

                if let Some(Value::String(content)) = delta.remove("content") {
                    Some(ResponseItem {
                        index,
                        role,
                        finish_reason,
                        contents: ResponseContents::Text(content.into_bytes()),
                    })
                } else {
                    None
                }
            }
            Value::String(delta) => Some(ResponseItem {
                index,
                role,
                finish_reason,
                contents: ResponseContents::Text(delta.into_bytes()),
            }),
            _ => None,
        }
    } else if let Some(content) = json.remove("content") {
        if let Value::Object(mut content) = content {
            if let Some(Value::String(role_value)) = content.remove("role") {
                role = Some(role_value);
            }

            if let Some(Value::String(content_type)) = content.get("type") {
                if content_type == "text" {
                    if let Some(Value::String(text)) = content.remove("text") {
                        Some(ResponseItem {
                            index,
                            role,
                            finish_reason,
                            contents: ResponseContents::Text(text.into_bytes()),
                        })
                    } else {
                        None
                    }
                } else if content_type == "output_text"
                    && let Some(Value::String(text)) = content.remove("text")
                {
                    let tokens = if let Some(Value::Array(logprobs_list_json)) =
                        content.remove("logprobs")
                    {
                        parse_openai_chatcompletion_logprobs_content(logprobs_list_json)
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
                    } else {
                        Some(ResponseItem {
                            index,
                            role,
                            finish_reason,
                            contents: ResponseContents::Text(text.into_bytes()),
                        })
                    }
                } else {
                    None
                }
            } else if let Some(Value::Array(parts)) = content.remove("parts") {
                let text: Vec<u8> = parts
                    .into_iter()
                    .flat_map(|part| {
                        if let Value::Object(mut part) = part {
                            if let Some(Value::Bool(thought)) = part.get("thought")
                                && *thought
                            {
                                Vec::new()
                            } else if let Some(Value::String(text)) = part.remove("text") {
                                text.into_bytes()
                            } else {
                                Vec::new()
                            }
                        } else {
                            Vec::new()
                        }
                    })
                    .collect();

                Some(ResponseItem {
                    index,
                    role,
                    finish_reason,
                    contents: ResponseContents::Text(text),
                })
            } else {
                None
            }
        } else if let Value::String(content) = content {
            Some(ResponseItem {
                index,
                role,
                finish_reason,
                contents: ResponseContents::Text(content.into_bytes()),
            })
        } else {
            None
        }
    } else if let Some(Value::String(response)) = json.remove("response") {
        Some(ResponseItem {
            index,
            role,
            finish_reason,
            contents: ResponseContents::Text(response.into_bytes()),
        })
    } else {
        None
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

fn parse_gemma_logprobs(mut logprobs_json: Map<String, Value>) -> Option<Vec<Token>> {
    if let Some(Value::Array(chosen_candidates)) = logprobs_json.remove("chosenCandidates") {
        let mut tokens = Vec::with_capacity(chosen_candidates.len());

        if let Some(Value::Array(top_candidates)) = logprobs_json.remove("topCandidates")
            && top_candidates.len() == chosen_candidates.len()
        {
            for (chosen_candidate, top_candidates) in
                chosen_candidates.into_iter().zip(top_candidates)
            {
                let mut top_tokens = Vec::new();

                if let Value::Object(mut top_candidates) = top_candidates
                    && let Some(Value::Array(candidates)) = top_candidates.remove("candidates")
                {
                    top_tokens.reserve_exact(candidates.len());

                    for candidate in candidates {
                        if let Value::Object(candidate) = candidate
                            && let Some(top_logprob) = parse_gemma_logprob_candidate(candidate)
                        {
                            top_tokens.push(top_logprob);
                        } else {
                            top_tokens.clear();
                            top_tokens.shrink_to_fit();
                            break;
                        }
                    }
                }

                if let Value::Object(chosen_candidate) = chosen_candidate
                    && let Some(token) = parse_gemma_logprob_candidate(chosen_candidate)
                {
                    tokens.push(Token { token, top_tokens });
                } else {
                    return None;
                }
            }
        } else {
            for chosen_candidate in chosen_candidates {
                if let Value::Object(chosen_candidate) = chosen_candidate
                    && let Some(token) = parse_gemma_logprob_candidate(chosen_candidate)
                {
                    tokens.push(Token {
                        token,
                        top_tokens: Vec::new(),
                    });
                } else {
                    return None;
                }
            }
        }

        Some(tokens)
    } else {
        None
    }
}

fn parse_gemma_logprob_candidate(mut logprob_json: Map<String, Value>) -> Option<LogprobToken> {
    if let Some(Value::String(token)) = logprob_json.remove("token")
        && let Some(Value::Number(logprob)) = logprob_json.remove("logProbability")
        && let Some(logprob) = logprob.as_f64()
    {
        Some(LogprobToken {
            id: if let Some(Value::Number(token_id)) = logprob_json.remove("tokenId")
                && let Some(token_id) = token_id.as_i128()
            {
                Some(token_id)
            } else {
                None
            },
            contents: token.into_bytes(),
            logprob,
        })
    } else {
        None
    }
}
