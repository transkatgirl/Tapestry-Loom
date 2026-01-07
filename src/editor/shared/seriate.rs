use elkai_rs::DistanceMatrix;
use ml_distance::distance::euclidean;

pub fn seriate(embeddings: Vec<Vec<f32>>) -> Vec<usize> {
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
