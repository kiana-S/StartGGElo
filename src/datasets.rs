use crate::error;
use crate::queries::*;
use sqlite::*;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub struct DatasetMetadata {
    pub last_sync: Timestamp,

    pub game_id: VideogameId,
    pub game_name: String,
    pub state: Option<String>,
}

/// Return the path to the datasets file.
fn datasets_path(config_dir: &Path) -> io::Result<PathBuf> {
    let mut path = config_dir.to_owned();
    path.push("startrnr");

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
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS datasets (
            name TEXT UNIQUE NOT NULL,
            last_sync INTEGER NOT NULL,
            game_id INTEGER NOT NULL,
            game_name TEXT NOT NULL,
            state TEXT
        ) STRICT;";

    let connection = sqlite::open(path)?;
    connection.execute(query)?;
    Ok(connection)
}

// TODO: Sanitize dataset names

pub fn list_dataset_names(connection: &Connection) -> sqlite::Result<Vec<String>> {
    let query = "SELECT name FROM datasets";

    connection
        .prepare(query)?
        .into_iter()
        .map(|r| r.map(|x| x.read::<&str, _>("name").to_owned()))
        .try_collect()
}

pub fn list_datasets(connection: &Connection) -> sqlite::Result<Vec<(String, DatasetMetadata)>> {
    let query = "SELECT * FROM datasets";

    connection
        .prepare(query)?
        .into_iter()
        .map(|r| {
            let r_ = r?;
            Ok((
                r_.read::<&str, _>("name").to_owned(),
                DatasetMetadata {
                    last_sync: Timestamp(r_.read::<i64, _>("last_sync") as u64),
                    game_id: VideogameId(r_.read::<i64, _>("game_id") as u64),
                    game_name: r_.read::<&str, _>("game_name").to_owned(),
                    state: r_.read::<Option<&str>, _>("state").map(String::from),
                },
            ))
        })
        .try_collect()
}

pub fn delete_dataset(connection: &Connection, dataset: &str) -> sqlite::Result<()> {
    let query = format!(
        r#"DELETE FROM datasets WHERE name = '{0}';
        DROP TABLE "dataset_{0}_players";
        DROP TABLE "dataset_{0}_network";"#,
        dataset
    );

    connection.execute(query)
}

pub fn new_dataset(
    connection: &Connection,
    dataset: &str,
    config: DatasetMetadata,
) -> sqlite::Result<()> {
    let query1 = r#"INSERT INTO datasets (name, game_id, game_name, state)
                        VALUES (?, ?, ?, ?)"#;
    let query2 = format!(
        r#"
        CREATE TABLE "dataset_{0}_players" (
            id INTEGER PRIMARY KEY,
            name TEXT,
            prefix TEXT
        );
        CREATE TABLE "dataset_{0}_network" (
            player_A INTEGER NOT NULL,
            player_B INTEGER NOT NULL,
            advantage REAL NOT NULL,
            sets_A INTEGER NOT NULL,
            sets_B INTEGER NOT NULL,
            games_A INTEGER NOT NULL,
            games_B INTEGER NOT NULL,

            UNIQUE (player_A, player_B),
            CHECK (player_A < player_B),
            FOREIGN KEY(player_A, player_B) REFERENCES "dataset_{0}_players"
                ON DELETE CASCADE
        ) STRICT;"#,
        dataset
    );

    connection
        .prepare(query1)?
        .into_iter()
        .bind((1, dataset))?
        .bind((2, config.game_id.0 as i64))?
        .bind((3, &config.game_name[..]))?
        .bind((4, config.state.as_deref()))?
        .try_for_each(|x| x.map(|_| ()))?;

    connection.execute(query2)
}

pub fn get_metadata(
    connection: &Connection,
    dataset: &str,
) -> sqlite::Result<Option<DatasetMetadata>> {
    let query = "SELECT last_sync, game_id, game_name, state FROM datasets WHERE name = ?";

    Ok(connection
        .prepare(query)?
        .into_iter()
        .bind((1, dataset))?
        .next()
        .map(|r| {
            let r_ = r?;
            Ok(DatasetMetadata {
                last_sync: Timestamp(r_.read::<i64, _>("last_sync") as u64),
                game_id: VideogameId(r_.read::<i64, _>("game_id") as u64),
                game_name: r_.read::<&str, _>("game_name").to_owned(),
                state: r_.read::<Option<&str>, _>("state").map(String::from),
            })
        })
        .and_then(Result::ok))
}

pub fn update_last_sync(connection: &Connection, dataset: &str) -> sqlite::Result<()> {
    let query = "UPDATE datasets SET last_sync = :sync WHERE name = :dataset";

    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_else(|_| error("System time is before the Unix epoch (1970)!", 2))
        .as_secs();

    connection
        .prepare(query)?
        .into_iter()
        .bind((":sync", current_time as i64))?
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
        r#"INSERT OR IGNORE INTO "dataset_{}_players" VALUES (?, ?, ?)"#,
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

pub fn get_advantage(
    connection: &Connection,
    dataset: &str,
    player1: PlayerId,
    player2: PlayerId,
) -> sqlite::Result<f64> {
    if player1 == player2 {
        return Ok(0.0);
    }

    let query = format!(
        r#"SELECT iif(:a > :b, -advantage, advantage) FROM "dataset_{}_network"
            WHERE player_A = min(:a, :b) AND player_B = max(:a, :b)"#,
        dataset
    );

    let mut statement = connection.prepare(&query)?;
    statement.bind((":a", player1.0 as i64))?;
    statement.bind((":b", player2.0 as i64))?;
    statement.next()?;
    statement.read::<f64, _>("advantage")
}

pub fn adjust_advantage(
    connection: &Connection,
    dataset: &str,
    player1: PlayerId,
    player2: PlayerId,
    adjust: f64,
) -> sqlite::Result<()> {
    let query = format!(
        r#"UPDATE "dataset_{}_network"
            SET advantage = advantage + iif(:a > :b, -:v, :v)
            WHERE player_A = min(:a, :b) AND player_B = max(:a, :b)"#,
        dataset
    );

    let mut statement = connection.prepare(&query)?;
    statement.bind((":a", player1.0 as i64))?;
    statement.bind((":b", player2.0 as i64))?;
    statement.bind((":v", adjust))?;
    statement.into_iter().try_for_each(|x| x.map(|_| ()))
}

pub fn adjust_advantages(
    connection: &Connection,
    dataset: &str,
    player: PlayerId,
    adjust: f64,
) -> sqlite::Result<()> {
    let query = format!(
        r#"UPDATE "dataset_{}_network"
            SET advantage = advantage + iif(:pl = player_A, -:v, :v)
            WHERE player_A = :pl OR player_B = :pl"#,
        dataset
    );

    let mut statement = connection.prepare(&query)?;
    statement.bind((":pl", player.0 as i64))?;
    statement.bind((":v", adjust))?;
    statement.into_iter().try_for_each(|x| x.map(|_| ()))
}

pub fn get_edges(
    connection: &Connection,
    dataset: &str,
    player: PlayerId,
) -> sqlite::Result<Vec<(PlayerId, f64)>> {
    let query = format!(
        r#"SELECT iif(:pl = player_B, player_A, player_B) AS id, iif(:pl = player_B, -advantage, advantage) AS advantage
            FROM "dataset_{}_network"
            WHERE player_A = :pl OR player_B = :pl"#,
        dataset
    );

    connection
        .prepare(&query)?
        .into_iter()
        .bind((":pl", player.0 as i64))?
        .map(|r| {
            let r_ = r?;
            Ok((
                PlayerId(r_.read::<i64, _>("id") as u64),
                r_.read::<f64, _>("advantage"),
            ))
        })
        .try_collect()
}

pub fn get_path_advantage(
    connection: &Connection,
    dataset: &str,
    players: &[PlayerId],
) -> sqlite::Result<f64> {
    players.windows(2).try_fold(0.0, |acc, [a, b]| {
        Ok(acc + get_advantage(connection, dataset, *a, *b)?)
    })
}

pub fn hypothetical_advantage(
    connection: &Connection,
    dataset: &str,
    player1: PlayerId,
    player2: PlayerId,
) -> sqlite::Result<f64> {
    if player1 == player2 {
        return Ok(0.0);
    }

    let mut paths: Vec<Vec<(Vec<PlayerId>, f64)>> = vec![vec![(vec![player1], 0.0)]];

    for _ in 2..=6 {
        let new_paths = paths.last().unwrap().into_iter().cloned().try_fold(
            Vec::new(),
            |mut acc, (path, adv)| {
                acc.extend(
                    get_edges(connection, dataset, *path.last().unwrap())?
                        .into_iter()
                        .map(|(x, next_adv)| {
                            let mut path = path.clone();
                            path.extend_one(x);
                            (path, adv + next_adv)
                        }),
                );
                Ok(acc)
            },
        )?;
        paths.extend_one(new_paths);
    }

    let mut shortest_len = 0;

    Ok(paths[1..]
        .into_iter()
        .enumerate()
        .map(|(i, ps)| {
            let num_ps = ps.len();
            if num_ps == 0 {
                return 0.0;
            }
            if shortest_len == 0 {
                shortest_len = i + 1;
            }
            ps.into_iter()
                .filter_map(|(path, adv)| {
                    if *path.last().unwrap() == player2 {
                        Some(adv)
                    } else {
                        None
                    }
                })
                .sum::<f64>()
                / num_ps as f64
                * (0.5_f64.powi((i - shortest_len) as i32))
        })
        .sum())
}
