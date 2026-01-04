// A robust completion API response parser which aims to support as many different APIs as possible

use serde_json::{Map, Value};

pub struct ResponseChoice {
    finish_reason: Option<String>,
    contents: ResponseContents,
}

enum ResponseContents {
    Textual(Vec<u8>),
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

fn parse_openai_completion_logprobs(mut logprobs_json: Map<String, Value>) -> Option<Vec<Token>> {
    todo!()
}

fn parse_openai_chatcompletion_logprob_content(
    mut logprob_json: Map<String, Value>,
) -> Option<Token> {
    let mut top_tokens = Vec::new();

    if let Some(Value::Array(top_logprobs_json)) = logprob_json.remove("top_logprobs") {
        let top_logprob_count = top_logprobs_json.len();

        top_tokens.reserve_exact(top_logprob_count);

        for top_logprob_json in top_logprobs_json.into_iter() {
            if let Value::Object(top_logprob_json) = top_logprob_json
                && let Some(top_logprob) =
                    parse_openai_chatcompletion_logprob_content_item(top_logprob_json)
            {
                top_tokens.push(top_logprob);
            }
        }

        if top_tokens.len() != top_logprob_count {
            top_tokens.clear();
            top_tokens.shrink_to_fit();
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

    let id = if let Some(Value::Number(id)) = logprob_json.remove("id")
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
        } else if let Some(Value::Number(prob)) = logprob_json.remove("prob")
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
