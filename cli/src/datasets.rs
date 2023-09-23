use sqlite::Connection;
use std::path::{Path, PathBuf};

/// Return the path to a dataset.
fn dataset_path(config_dir: &Path, dataset: &str) -> PathBuf {
    let mut path = config_dir.to_owned();
    path.push("datasets");
    path.push(dataset);
    path.set_extension("sqlite");
    path
}

/// Create a new dataset given a path.
pub fn new_dataset(dataset: &Path) -> sqlite::Result<Connection> {
    let query = "
        CREATE TABLE players (
            id INTEGER PRIMARY KEY ASC,
            name TEXT,
            elo REAL
        ) STRICT;
    ";

    let connection = sqlite::open(dataset)?;
    connection.execute(query)?;
    Ok(connection)
}

// Score calculation

/// Calculate the collective expected score for each team.
fn expected_scores(ratings: &Teams<&mut f64>) -> Vec<f64> {
    let qs: Vec<f64> = ratings
        .into_iter()
        .map(|es| 10_f64.powf(es.iter().map(|x| **x).sum::<f64>() / es.len() as f64 / 400.0))
        .collect();
    let sumq: f64 = qs.iter().sum();
    qs.into_iter().map(|q| q / sumq).collect()
}

/// Adjust the ratings of each player based on who won.
fn adjust_ratings(ratings: Teams<&mut f64>, winner: usize) {
    let exp_scores = expected_scores(&ratings);

    ratings
        .into_iter()
        .zip(exp_scores.into_iter())
        .enumerate()
        .for_each(|(i, (es, exp_sc))| {
            let len = es.len() as f64;
            let score = f64::from(winner == i);
            es.into_iter()
                .for_each(|e| *e += 40.0 * (score - exp_sc) / len);
        })
}

