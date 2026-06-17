use std::num::NonZeroU128;

use log::trace;
use reqwest::Response;
use serde_json::{Map, Value};
use tapestry_weave::{
    jiff::Zoned,
    v1::{
        content::{Creator, Model, NodeContent, RawQuery},
        metadata::MetadataMap,
    },
};
use ulid::Ulid;

use super::{EndpointResponse, InferenceModel, polyparser};

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

pub(super) fn json_object_to_metadata_map(value: Map<String, Value>) -> MetadataMap {
    MetadataMap::from_iter(
        value
            .into_iter()
            .map(|(k, v)| (k, json_value_to_metadata_field(v))),
    )
}

pub(super) fn json_value_to_metadata_field(value: Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::String(v) => v,
        Value::Bool(v) => v.to_string(),
        Value::Number(v) => v.to_string(),
        Value::Array(v) => serde_json::to_string(&v).unwrap(),
        Value::Object(v) => serde_json::to_string(&v).unwrap(),
    }
}

pub(crate) fn ulid_to_long_identifier(value: Ulid) -> Option<NonZeroU128> {
    NonZeroU128::try_from(value.0).ok()
}

pub(crate) fn ulid_from_long_identifier(value: NonZeroU128) -> Ulid {
    Ulid(u128::from(value))
}

pub(super) async fn error_for_status(response: Response) -> Result<Response, anyhow::Error> {
    let status = response.status();
    if status.is_client_error() || status.is_server_error() {
        Err(match response.text().await {
            Ok(text) => anyhow::Error::msg(format!("HTTP {}: {}", status.as_u16(), text)),
            Err(error) => error.into(),
        })
    } else if status.is_redirection() || status.is_informational() {
        Err(anyhow::Error::msg(format!(
            "Unexpected HTTP status: {}",
            status
        )))
    } else {
        Ok(response)
    }
}

pub(super) fn parse_response(
    response: Map<String, Value>,
    metadata: Vec<(String, String)>,
    model: &InferenceModel,
    echo: bool,
    seed: Option<u32>,
    raw_query: Option<RawQuery>,
    requested_top: Option<usize>,
) -> Vec<EndpointResponse> {
    trace!("{:#?}", &response);

    let timestamp = Zoned::now();

    let items = polyparser::parse_response(response, requested_top);

    let mut outputs = Vec::with_capacity(items.len());

    for mut item in items {
        item.clear_normal();

        let mut metadata = metadata.clone();

        if let Some(role) = item.role {
            metadata.push(("role".to_string(), role));
        }

        outputs.push(EndpointResponse {
            root: echo,
            content: NodeContent {
                timestamp: timestamp.clone(),
                modified: false,
                content: item.contents,
                metadata: MetadataMap::from_iter(metadata),
                creator: Creator::Model(Some(Model {
                    label: model.label.clone(),
                    color: model.color.map(|c| c.to_hex()),
                    identifier: ulid_to_long_identifier(model.identifier),
                    seed,
                    system_fingerprint: item.fingerprint,
                    finish_reason: item.finish_reason,
                    metadata: MetadataMap::default(),
                    raw_query: raw_query.clone(),
                })),
            },
        });
    }

    outputs
}

pub(super) fn parse_embedding_response(response: Value) -> Option<Vec<f32>> {
    trace!("{:#?}", &response);

    let mut response = polyparser::parse_embedding_response(response);

    if response.len() == 1 {
        response.remove(0)
    } else {
        None
    }
}

pub(super) fn response_schema_error() -> anyhow::Error {
    anyhow::Error::msg("Response does not match API schema")
}
