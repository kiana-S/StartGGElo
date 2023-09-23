use crate::queries::*;
use sqlite::*;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

/// Return the path to a dataset.
pub fn dataset_path(config_dir: &Path, dataset: &str) -> PathBuf {
    // $config_dir/datasets/$dataset.sqlite
    let mut path = config_dir.to_owned();
    path.push("datasets");

    // Create datasets path if it doesn't exist
    fs::create_dir_all(&path).unwrap();

    path.push(dataset);
    path.set_extension("db");
    path
}

pub fn open_dataset(dataset: &Path) -> sqlite::Result<Connection> {
    let query = "
        CREATE TABLE IF NOT EXISTS players (
            id INTEGER PRIMARY KEY,
            name TEXT,
            prefix TEXT,
            elo REAL NOT NULL
        ) STRICT;
    ";

    File::create(dataset).map_err(|e| Error {
        code: {
            println!("{:?}", e);
            None
        },
        message: Some("unable to open database file".to_owned()),
    })?;
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

// Database Updating

pub fn add_players(connection: &Connection, teams: &Teams<PlayerData>) -> sqlite::Result<()> {
    let query = "INSERT OR IGNORE INTO players VALUES (?, ?, ?, 1500)";

    teams.iter().try_for_each(|team| {
        team.iter().try_for_each(|PlayerData { id, name, prefix }| {
            let mut statement = connection.prepare(query)?;
            statement.bind((1, id.0 as i64))?;
            statement.bind((2, name.as_ref().map(|x| &x[..])))?;
            statement.bind((3, prefix.as_ref().map(|x| &x[..])))?;
            statement.into_iter().try_for_each(|x| x.map(|_| ()))?;
            Ok(())
        })
    })
}

pub fn get_ratings(
    connection: &Connection,
    teams: &Teams<PlayerData>,
) -> sqlite::Result<Teams<(PlayerId, f64)>> {
    let query = "SELECT id, elo FROM players WHERE id = ?";

    teams
        .iter()
        .map(|team| {
            team.iter()
                .map(|data| {
                    let mut statement = connection.prepare(query)?;
                    statement.bind((1, data.id.0 as i64))?;
                    statement.next()?;
                    Ok((data.id, statement.read::<f64, _>("elo")?))
                })
                .try_collect()
        })
        .try_collect()
}

pub fn update_ratings(connection: &Connection, elos: Teams<(PlayerId, f64)>) -> sqlite::Result<()> {
    let query = "UPDATE players SET elo = :elo WHERE id = :id";
    elos.into_iter().try_for_each(|team| {
        team.into_iter().try_for_each(|(id, elo)| {
            let mut statement = connection.prepare(query)?;
            statement.bind((":elo", elo))?;
            statement.bind((":id", id.0 as i64))?;
            statement.into_iter().try_for_each(|x| x.map(|_| ()))?;
            Ok(())
        })
    })
}

pub fn update_from_set(connection: &Connection, results: SetData) -> sqlite::Result<()> {
    let players_data = results.teams;
    add_players(connection, &players_data)?;

    let mut elos = get_ratings(connection, &players_data)?;
    adjust_ratings(
        elos.iter_mut()
            .map(|v| v.iter_mut().map(|x| &mut x.1).collect())
            .collect(),
        results.winner,
    );
    update_ratings(connection, elos)
}
