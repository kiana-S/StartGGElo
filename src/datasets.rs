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
        DROP TABLE "{0}_players";
        DROP TABLE "{0}_network";
        DROP VIEW "{0}_view";"#,
        dataset
    );

    connection.execute(query)
}

pub fn new_dataset(
    connection: &Connection,
    dataset: &str,
    metadata: DatasetMetadata,
) -> sqlite::Result<()> {
    let query1 = r#"INSERT INTO datasets VALUES (?, ?, ?, ?, ?)"#;
    let query2 = format!(
        r#"CREATE TABLE "{0}_players" (
    id INTEGER PRIMARY KEY,
    name TEXT,
    prefix TEXT
);

CREATE TABLE "{0}_network" (
    player_A INTEGER NOT NULL,
    player_B INTEGER NOT NULL,
    advantage REAL NOT NULL,
    sets_A INTEGER NOT NULL DEFAULT 0,
    sets_B INTEGER NOT NULL DEFAULT 0,
    games_A INTEGER NOT NULL DEFAULT 0,
    games_B INTEGER NOT NULL DEFAULT 0,

    UNIQUE (player_A, player_B),
    CHECK (player_A < player_B),
    FOREIGN KEY(player_A) REFERENCES "{0}_players"
        ON DELETE CASCADE,
    FOREIGN KEY(player_B) REFERENCES "{0}_players"
        ON DELETE CASCADE
) STRICT;
CREATE INDEX "{0}_network_A"
    ON "{0}_network" (player_A);
CREATE INDEX "{0}_network_B"
    ON "{0}_network" (player_B);

CREATE VIEW "{0}_view"
    (player_A_id, player_B_id, player_A_name, player_B_name, advantage,
        sets_A, sets_B, sets, games_A, games_B, games) AS
    SELECT players_A.id, players_B.id, players_A.name, players_B.name, advantage,
        sets_A, sets_B, sets_A + sets_B, games_A, games_B, games_A + games_B FROM "{0}_network"
    INNER JOIN "{0}_players" players_A ON player_A = players_A.id
    INNER JOIN "{0}_players" players_B ON player_B = players_B.id;"#,
        dataset
    );

    connection
        .prepare(query1)?
        .into_iter()
        .bind((1, dataset))?
        .bind((2, metadata.last_sync.0 as i64))?
        .bind((3, metadata.game_id.0 as i64))?
        .bind((4, &metadata.game_name[..]))?
        .bind((5, metadata.state.as_deref()))?
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
        r#"INSERT OR IGNORE INTO "{}_players" VALUES (?, ?, ?)"#,
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
) -> sqlite::Result<Option<f64>> {
    if player1 == player2 {
        return Ok(Some(0.0));
    }

    let query = format!(
        r#"SELECT iif(:a > :b, -advantage, advantage) AS advantage FROM "{}_network"
            WHERE player_A = min(:a, :b) AND player_B = max(:a, :b)"#,
        dataset
    );

    let mut statement = connection.prepare(&query)?;
    statement.bind((":a", player1.0 as i64))?;
    statement.bind((":b", player2.0 as i64))?;
    statement.next()?;
    statement.read::<Option<f64>, _>("advantage")
}

pub fn insert_advantage(
    connection: &Connection,
    dataset: &str,
    player1: PlayerId,
    player2: PlayerId,
    advantage: f64,
) -> sqlite::Result<()> {
    let query = format!(
        r#"INSERT INTO "{}_network" (player_A, player_B, advantage)
            VALUES (min(:a, :b), max(:a, :b), iif(:a > :b, -:v, :v))"#,
        dataset
    );

    let mut statement = connection.prepare(query)?;
    statement.bind((":a", player1.0 as i64))?;
    statement.bind((":b", player2.0 as i64))?;
    statement.bind((":v", advantage))?;
    statement.into_iter().try_for_each(|x| x.map(|_| ()))
}

pub fn adjust_advantage(
    connection: &Connection,
    dataset: &str,
    player1: PlayerId,
    player2: PlayerId,
    adjust: f64,
) -> sqlite::Result<()> {
    let query = format!(
        r#"UPDATE "{}_network"
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
        r#"UPDATE "{}_network"
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
            FROM "{}_network"
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

pub fn is_isolated(
    connection: &Connection,
    dataset: &str,
    player: PlayerId,
) -> sqlite::Result<bool> {
    let query = format!(
        r#"SELECT EXISTS(SELECT 1 FROM "{}_network" WHERE player_A = :pl OR player_B = :pl)"#,
        dataset
    );

    match connection
        .prepare(&query)?
        .into_iter()
        .bind((":pl", player.0 as i64))?
        .next()
    {
        None => Ok(false),
        Some(r) => r.map(|_| true),
    }
}

pub fn hypothetical_advantage(
    connection: &Connection,
    dataset: &str,
    player1: PlayerId,
    player2: PlayerId,
) -> sqlite::Result<f64> {
    // Check trivial cases
    if player1 == player2
        || is_isolated(connection, dataset, player1)?
        || is_isolated(connection, dataset, player2)?
    {
        return Ok(0.0);
    }

    let mut paths: Vec<Vec<(Vec<PlayerId>, f64)>> = vec![vec![(vec![player1], 0.0)]];

    for _ in 2..=7 {
        let new_paths = paths.last().unwrap().iter().cloned().try_fold(
            Vec::new(),
            |mut acc, (path, adv)| {
                acc.extend(
                    get_edges(connection, dataset, *path.last().unwrap())?
                        .into_iter()
                        .filter_map(|(x, next_adv)| {
                            if path.contains(&x) {
                                None
                            } else {
                                let mut path = path.clone();
                                path.extend_one(x);
                                Some((path, adv + next_adv))
                            }
                        }),
                );
                Ok(acc)
            },
        )?;
        paths.extend_one(new_paths);
    }

    Ok(paths
        .into_iter()
        .skip(1)
        .map(|ps| {
            let ps_correct = ps
                .iter()
                .filter_map(|(path, adv)| {
                    if *path.last().unwrap() == player2 {
                        Some(adv)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            let num_ps = ps_correct.len();
            if num_ps == 0 {
                return None;
            }
            Some(ps_correct.into_iter().sum::<f64>() / num_ps as f64)
        })
        .skip_while(|x| x.is_none())
        .enumerate()
        .fold((0.0, 0.0), |(total, last), (i, adv)| {
            let adv_ = adv.unwrap_or(last);
            (total + (0.5_f64.powi((i + 1) as i32) * adv_), adv_)
        })
        .0)
}

pub fn initialize_edge(
    connection: &Connection,
    dataset: &str,
    player1: PlayerId,
    player2: PlayerId,
) -> sqlite::Result<f64> {
    let adv = hypothetical_advantage(connection, dataset, player1, player2)?;
    insert_advantage(connection, dataset, player1, player2, adv)?;
    Ok(adv)
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    // Mock a database file in transient memory
    fn mock_datasets() -> sqlite::Result<Connection> {
        let query = "
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS datasets (
                name TEXT UNIQUE NOT NULL,
                last_sync INTEGER NOT NULL,
                game_id INTEGER NOT NULL,
                game_name TEXT NOT NULL,
                state TEXT
            ) STRICT;";

        let connection = sqlite::open(":memory:")?;
        connection.execute(query)?;
        Ok(connection)
    }

    // Functions to generate test data

    fn metadata() -> DatasetMetadata {
        DatasetMetadata {
            last_sync: Timestamp(1),
            game_id: VideogameId(0),
            game_name: String::from("Test Game"),
            state: None,
        }
    }

    fn players(num: u64) -> Vec<PlayerData> {
        (1..=num)
            .map(|i| PlayerData {
                id: PlayerId(i),
                name: Some(format!("{}", i)),
                prefix: None,
            })
            .collect()
    }

    #[test]
    fn sqlite_sanity_check() -> sqlite::Result<()> {
        let test_value: i64 = 2;

        let connection = sqlite::open(":memory:")?;
        connection.execute(
            r#"CREATE TABLE test (a INTEGER);
            INSERT INTO test VALUES (1);
            INSERT INTO test VALUES (2)"#,
        )?;

        let mut statement = connection.prepare("SELECT * FROM test WHERE a = ?")?;
        statement.bind((1, test_value))?;
        statement.next()?;
        assert_eq!(statement.read::<i64, _>("a")?, test_value);
        Ok(())
    }

    #[test]
    fn test_players() -> sqlite::Result<()> {
        let connection = mock_datasets()?;
        new_dataset(&connection, "test", metadata())?;

        add_players(&connection, "test", &vec![players(2)])?;

        let mut statement =
            connection.prepare("SELECT * FROM dataset_test_players WHERE id = 1")?;
        statement.next()?;
        assert_eq!(statement.read::<i64, _>("id")?, 1);
        assert_eq!(statement.read::<String, _>("name")?, "1");
        assert_eq!(statement.read::<Option<String>, _>("prefix")?, None);

        Ok(())
    }

    #[test]
    fn edge_insert_get() -> sqlite::Result<()> {
        let connection = mock_datasets()?;
        new_dataset(&connection, "test", metadata())?;
        add_players(&connection, "test", &vec![players(2)])?;

        insert_advantage(&connection, "test", PlayerId(2), PlayerId(1), 1.0)?;

        assert_eq!(
            get_advantage(&connection, "test", PlayerId(1), PlayerId(2))?,
            Some(-1.0)
        );
        assert_eq!(
            get_advantage(&connection, "test", PlayerId(2), PlayerId(1))?,
            Some(1.0)
        );

        Ok(())
    }

    #[test]
    fn player_all_edges() -> sqlite::Result<()> {
        let connection = mock_datasets()?;
        new_dataset(&connection, "test", metadata())?;
        add_players(&connection, "test", &vec![players(3)])?;

        insert_advantage(&connection, "test", PlayerId(2), PlayerId(1), 1.0)?;
        insert_advantage(&connection, "test", PlayerId(1), PlayerId(3), 5.0)?;

        assert_eq!(
            get_edges(&connection, "test", PlayerId(1))?,
            [(PlayerId(2), -1.0), (PlayerId(3), 5.0)]
        );
        assert_eq!(
            get_edges(&connection, "test", PlayerId(2))?,
            [(PlayerId(1), 1.0)]
        );
        assert_eq!(
            get_edges(&connection, "test", PlayerId(3))?,
            [(PlayerId(1), -5.0)]
        );
        Ok(())
    }

    #[test]
    fn hypoth_adv_trivial() -> sqlite::Result<()> {
        let num_players = 3;

        let connection = mock_datasets()?;
        new_dataset(&connection, "test", metadata())?;
        add_players(&connection, "test", &vec![players(num_players)])?;

        for i in 1..=num_players {
            for j in 1..=num_players {
                assert_eq!(
                    hypothetical_advantage(&connection, "test", PlayerId(i), PlayerId(j))?,
                    0.0
                );
            }
        }

        Ok(())
    }

    #[test]
    fn hypoth_adv1() -> sqlite::Result<()> {
        let connection = mock_datasets()?;
        new_dataset(&connection, "test", metadata())?;
        add_players(&connection, "test", &vec![players(2)])?;

        insert_advantage(&connection, "test", PlayerId(1), PlayerId(2), 1.0)?;

        assert!(
            (hypothetical_advantage(&connection, "test", PlayerId(1), PlayerId(2))? - 1.0) < 0.1
        );

        Ok(())
    }
}
