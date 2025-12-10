use std::{collections::HashMap, env, fs, path::PathBuf, sync::Arc};

use rocket::{Data, State, data::ByteUnit, http::Status, post, serde::json::Json};
use serde::{Deserialize, Serialize};
use tokenizers::Tokenizer;

#[derive(Serialize, Deserialize, Default)]
struct ModelConfig {
    models: Vec<Model>,
}

#[derive(Serialize, Deserialize, Default)]
struct Model {
    label: String,
    file: PathBuf,
}

#[derive(Default)]
struct SharedState {
    tokenizers: HashMap<String, Tokenizer>,
}

#[rocket::main]
#[allow(clippy::result_large_err)]
async fn main() -> Result<(), anyhow::Error> {
    unsafe {
        env::set_var("RAYON_RS_NUM_THREADS", "1");
    }

    let model_config: ModelConfig = toml::from_slice(&fs::read("models.toml")?)?;

    let mut tokenizers = HashMap::with_capacity(model_config.models.len());

    for model in model_config.models {
        tokenizers.insert(
            model.label,
            Tokenizer::from_file(&model.file).map_err(anyhow::Error::from_boxed)?,
        );
    }

    let shared = Arc::new(SharedState { tokenizers });

    let _rocket = rocket::build()
        .manage(shared.clone())
        .mount("/", rocket::routes![tokenize, detokenize])
        .launch()
        .await?;

    Ok(())
}

#[post("/<model>/tokenize", data = "<data>")]
async fn tokenize(
    state: &State<Arc<SharedState>>,
    model: &str,
    data: Data<'_>,
) -> Result<Json<Vec<u32>>, Status> {
    if let Some(tokenizer) = state.tokenizers.get(model) {
        let data = data
            .open(ByteUnit::Megabyte(4))
            .into_bytes()
            .await
            .map_err(|_| Status::BadRequest)?;

        let input = String::from_utf8_lossy(&data);

        let encoding = tokenizer
            .encode_fast(input, false)
            .map_err(|_| Status::InternalServerError)?;

        Ok(Json(encoding.get_ids().to_vec()))
    } else {
        Err(Status::NotFound)
    }
}

#[post("/<model>/detokenize", data = "<data>")]
async fn detokenize(
    state: &State<Arc<SharedState>>,
    model: &str,
    data: Json<Vec<u32>>,
) -> Result<Vec<u8>, Status> {
    if let Some(tokenizer) = state.tokenizers.get(model) {
        Ok(tokenizer
            .decode(&data.0, true)
            .map_err(|_| Status::InternalServerError)?
            .into_bytes())
    } else {
        Err(Status::NotFound)
    }
}
