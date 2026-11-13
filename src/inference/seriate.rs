use std::collections::HashMap;

use elkai_rs::DistanceMatrix;
use ml_distance::similarity::cosine;

pub fn seriate<T>(embeddings: Vec<(T, Vec<f32>)>) -> Vec<T> {
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
    let mut distances = vec![vec![0; embeddings.len() + 1]; embeddings.len() + 1];

    for (row, row_embedding) in embeddings.iter().enumerate() {
        for (column, column_embedding) in embeddings.iter().enumerate().skip(row + 1) {
            let distance = cosine_distance(row_embedding, column_embedding);

            distances[row][column] = distance;
            distances[column][row] = distance;
        }
    }

    let path_boundary = embeddings.len();

    drop(embeddings);

    let mut solved = DistanceMatrix::new(distances).solve(10);

    if let Some(position) = solved.iter().position(|index| *index == path_boundary) {
        solved.rotate_left(position);
        solved.remove(0);
    }

    solved
}

fn cosine_distance(a: &[f32], b: &[f32]) -> u32 {
    /// must stay small enough to prevent the solver's internal calculations from overflowing i32
    const DISTANCE_SCALE: f64 = 1_000_000.0;

    let distance = 1.0 - cosine(a, b);

    if distance.is_nan() {
        return DISTANCE_SCALE as u32;
    }

    (distance.clamp(0.0, 2.0) * DISTANCE_SCALE).round() as u32
}
