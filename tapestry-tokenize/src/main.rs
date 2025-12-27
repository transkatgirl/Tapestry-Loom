use std::{collections::HashMap, env, fs, path::PathBuf, sync::Arc};

use log::{info, warn};
use rocket::{State, get, http::Status, post, serde::json::Json, tokio::task::block_in_place};
use serde::{Deserialize, Serialize};
use tokenizers::Tokenizer;

#[derive(Serialize, Deserialize, Default)]
struct ModelConfig {
    #[serde(default)]
    models: Vec<Model>,
}

#[derive(Serialize, Deserialize, Default)]
struct Model {
    label: String,
    file: PathBuf,
}

#[derive(Default)]
struct SharedState {
    tokenizers: HashMap<String, (Tokenizer, Arc<[u8]>)>,
}

#[rocket::main]
#[allow(clippy::result_large_err)]
async fn main() -> Result<(), anyhow::Error> {
    unsafe {
        env::set_var("RAYON_RS_NUM_THREADS", "1");
    }

    if !fs::exists("Rocket.toml")? {
        fs::write("Rocket.toml", include_bytes!("default-rocket.toml"))?;
    }

    if !fs::exists("models.toml")? {
        fs::write("models.toml", include_bytes!("default-models.toml"))?;
    }

    let model_config: ModelConfig = toml::from_slice(&fs::read("models.toml")?)?;

    let mut tokenizers = HashMap::with_capacity(model_config.models.len());

    for model in model_config.models {
        println!("Loading {}", model.file.display());
        let contents = fs::read(model.file)?;

        tokenizers.insert(
            model.label,
            (
                Tokenizer::from_bytes(&contents).map_err(anyhow::Error::from_boxed)?,
                contents.into(),
            ),
        );
    }

    let shared = SharedState { tokenizers };

    let _rocket = rocket::build()
        .manage(shared)
        .mount(
            "/",
            rocket::routes![tokenize, detokenize, tokenizer, tokenize_root],
        )
        .launch()
        .await?;

    Ok(())
}

#[post("/<model>", data = "<data>")]
async fn tokenize_root(
    state: &State<SharedState>,
    model: &str,
    data: Vec<u8>,
) -> Result<Json<Vec<u32>>, Status> {
    tokenize(state, model, data).await
}

#[post("/<model>/tokenize", data = "<data>")]
async fn tokenize(
    state: &State<SharedState>,
    model: &str,
    data: Vec<u8>,
) -> Result<Json<Vec<u32>>, Status> {
    if let Some((tokenizer, _)) = state.tokenizers.get(model) {
        info!("Tokenizing {} bytes using {:?}", data.len(), model);

        block_in_place(|| {
            let input = if let Ok(input) = str::from_utf8(&data) {
                input
            } else {
                warn!(
                    "Request body contains characters not supported by the current tokenization backend"
                );
                &String::from_utf8_lossy(&data)
            };

            let encoding = tokenizer
                .encode_fast(input, false)
                .map_err(|_| Status::InternalServerError)?;

            Ok(Json(encoding.get_ids().to_vec()))
        })
    } else {
        warn!("Unable to find model {:?}", model);

        Err(Status::NotFound)
    }
}

#[get("/<model>/tokenizer.json")]
async fn tokenizer(state: &State<SharedState>, model: &str) -> Result<Arc<[u8]>, Status> {
    if let Some((_, data)) = state.tokenizers.get(model) {
        info!("Sending tokenizer file for {:?}", model);

        Ok(data.clone())
    } else {
        warn!("Unable to find model {:?}", model);

        Err(Status::NotFound)
    }
}

#[post("/<model>/detokenize", data = "<data>")]
async fn detokenize(
    state: &State<SharedState>,
    model: &str,
    data: Json<Vec<u32>>,
) -> Result<Vec<u8>, Status> {
    if let Some((tokenizer, _)) = state.tokenizers.get(model) {
        info!("Detokenizing {} tokens using {:?}", data.0.len(), model);

        block_in_place(|| {
            Ok(tokenizer
                .decode(&data.0, true)
                .map_err(|_| Status::InternalServerError)?
                .into_bytes())
        })
    } else {
        warn!("Unable to find model {:?}", model);

        Err(Status::NotFound)
    }
}
