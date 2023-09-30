use crate::queries::*;
use sqlite::*;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

/// Return the path to the datasets file.
fn datasets_path(config_dir: &Path) -> io::Result<PathBuf> {
    let mut path = config_dir.to_owned();
    path.push("ggelo");

    // Create datasets path if it doesn't exist
    fs::create_dir_all(&path)?;

    path.push("datasets.sqlite");

    // Create datasets file if it doesn't exist
    OpenOptions::new().write(true).create(true).open(&path)?;

    Ok(path)
}

pub fn open_datasets(config_dir: &Path) -> sqlite::Result<Connection> {
    let path = datasets_path(config_dir).unwrap();

    let query = "
        CREATE TABLE IF NOT EXISTS datasets (
            name TEXT UNIQUE NOT NULL,
            last_sync INTEGER DEFAULT 1
        ) STRICT;";

    let connection = sqlite::open(path)?;
    connection.execute(query)?;
    Ok(connection)
}

// TODO: Sanitize dataset names

pub fn list_datasets(connection: &Connection) -> sqlite::Result<Vec<String>> {
    let query = "SELECT * FROM datasets";

    connection
        .prepare(query)?
        .into_iter()
        .map(|x| x.map(|r| r.read::<&str, _>("name").to_owned()))
        .try_collect()
}

pub fn delete_dataset(connection: &Connection, dataset: &str) -> sqlite::Result<()> {
    let query = format!(
        r#"DELETE FROM datasets WHERE name = '{0}';
        DROP TABLE "dataset_{0}";"#,
        dataset
    );

    connection.execute(query)
}

pub fn new_dataset(connection: &Connection, dataset: &str) -> sqlite::Result<()> {
    let query = format!(
        r#"INSERT INTO datasets (name) VALUES ('{0}');

        CREATE TABLE IF NOT EXISTS "dataset_{0}" (
            id INTEGER PRIMARY KEY,
            name TEXT,
            prefix TEXT,
            elo REAL NOT NULL
        ) STRICT;"#,
        dataset
    );

    connection.execute(query)
}

pub fn get_last_sync(connection: &Connection, dataset: &str) -> sqlite::Result<Option<u64>> {
    let query = "SELECT last_sync FROM datasets WHERE name = ?";

    Ok(connection
        .prepare(query)?
        .into_iter()
        .bind((1, dataset))?
        .map(|x| x.map(|r| r.read::<i64, _>("last_sync").to_owned() as u64))
        .next()
        .and_then(Result::ok))
}

pub fn update_last_sync(connection: &Connection, dataset: &str, sync: u64) -> sqlite::Result<()> {
    let query = "UPDATE datasets SET last_sync = :sync WHERE name = :dataset";

    connection
        .prepare(query)?
        .into_iter()
        .bind((":sync", sync as i64))?
        .bind((":dataset", dataset))?
        .try_for_each(|x| x.map(|_| ()))
}

// Database Updating

pub fn add_players(
    connection: &Connection,
    dataset: &str,
    teams: &Teams<PlayerData>,
) -> sqlite::Result<()> {
    let query = format!(
        r#"INSERT OR IGNORE INTO "dataset_{}" VALUES (?, ?, ?, 1500)"#,
        dataset
    );

    teams.iter().try_for_each(|team| {
        team.iter().try_for_each(|PlayerData { id, name, prefix }| {
            let mut statement = connection.prepare(&query)?;
            statement.bind((1, id.0 as i64))?;
            statement.bind((2, name.as_ref().map(|x| &x[..])))?;
            statement.bind((3, prefix.as_ref().map(|x| &x[..])))?;
            statement.into_iter().try_for_each(|x| x.map(|_| ()))
        })
    })
}

pub fn get_ratings(
    connection: &Connection,
    dataset: &str,
    teams: &Teams<PlayerData>,
) -> sqlite::Result<Teams<(PlayerId, f64)>> {
    let query = format!(r#"SELECT id, elo FROM "dataset_{}" WHERE id = ?"#, dataset);

    teams
        .iter()
        .map(|team| {
            team.iter()
                .map(|data| {
                    let mut statement = connection.prepare(&query)?;
                    statement.bind((1, data.id.0 as i64))?;
                    statement.next()?;
                    Ok((data.id, statement.read::<f64, _>("elo")?))
                })
                .try_collect()
        })
        .try_collect()
}

pub fn update_ratings(
    connection: &Connection,
    dataset: &str,
    elos: Teams<(PlayerId, f64)>,
) -> sqlite::Result<()> {
    let query = format!(
        r#"UPDATE "dataset_{}" SET elo = :elo WHERE id = :id"#,
        dataset
    );
    elos.into_iter().try_for_each(|team| {
        team.into_iter().try_for_each(|(id, elo)| {
            let mut statement = connection.prepare(&query)?;
            statement.bind((":elo", elo))?;
            statement.bind((":id", id.0 as i64))?;
            statement.into_iter().try_for_each(|x| x.map(|_| ()))
        })
    })
}
