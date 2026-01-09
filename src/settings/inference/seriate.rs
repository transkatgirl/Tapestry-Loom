use std::collections::HashMap;

use elkai_rs::DistanceMatrix;
use ml_distance::distance::euclidean;
use tapestry_weave::ulid::Ulid;

pub fn seriate(embeddings: Vec<(Ulid, Vec<f32>)>) -> Vec<Ulid> {
    if embeddings.len() < 3 {
        return embeddings.into_iter().map(|(id, _)| id).collect();
    }

    let mut index_map = HashMap::with_capacity(embeddings.len());
    let mut embedding_list = Vec::with_capacity(embeddings.len());

    for (index, (id, embedding)) in embeddings.into_iter().enumerate() {
        index_map.insert(index, id);
        embedding_list.push(embedding);
    }

    seriate_inner(embedding_list)
        .into_iter()
        .filter_map(|index| index_map.remove(&index))
        .collect()
}

fn seriate_inner(embeddings: Vec<Vec<f32>>) -> Vec<usize> {
    let distances = embeddings
        .iter()
        .map(|row| {
            embeddings
                .iter()
                //.map(|column| cosine(column, row) + 1.0)
                .map(|column| euclidean(column, row))
                .collect::<Vec<f64>>()
        })
        .collect::<Vec<Vec<f64>>>();

    drop(embeddings);

    DistanceMatrix::new(distances).solve(10)
}
