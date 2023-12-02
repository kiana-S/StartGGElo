use crate::queries::*;
use sqlite::*;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

pub struct DatasetMetadata {
    pub last_sync: Timestamp,

    pub game_id: VideogameId,
    pub game_name: String,
    pub game_slug: String,
    pub country: Option<String>,
    pub state: Option<String>,

    pub set_limit: u64,
    pub decay_rate: f64,
    pub adj_decay_rate: f64,
    pub period: f64,
    pub tau: f64,
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

    let query = "PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS datasets (
    name TEXT UNIQUE NOT NULL,
    last_sync INTEGER NOT NULL,
    game_id INTEGER NOT NULL,
    game_name TEXT NOT NULL,
    game_slug TEXT NOT NULL,
    country TEXT,
    state TEXT,
    set_limit INTEGER NOT NULL,
    decay_rate REAL NOT NULL,
    adj_decay_rate REAL NOT NULL,
    period REAL NOT NULL,
    tau REAL NOT NULL
) STRICT;

CREATE TABLE IF NOT EXISTS players (
    id INTEGER PRIMARY KEY,
    discrim TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    prefix TEXT
) STRICT;

CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY,
    slug TEXT NOT NULL
) STRICT;

CREATE TABLE IF NOT EXISTS sets (
    id TEXT UNIQUE NOT NULL,
    event INTEGER NOT NULL,
    FOREIGN KEY(event) REFERENCES events
) STRICT;
";

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
                    game_slug: r_.read::<&str, _>("game_slug").to_owned(),
                    country: r_.read::<Option<&str>, _>("country").map(String::from),
                    state: r_.read::<Option<&str>, _>("state").map(String::from),
                    set_limit: r_.read::<i64, _>("set_limit") as u64,
                    decay_rate: r_.read::<f64, _>("decay_rate"),
                    adj_decay_rate: r_.read::<f64, _>("adj_decay_rate"),
                    period: r_.read::<f64, _>("period"),
                    tau: r_.read::<f64, _>("tau"),
                },
            ))
        })
        .try_collect()
}

pub fn delete_dataset(connection: &Connection, dataset: &str) -> sqlite::Result<()> {
    let query = format!(
        r#"DELETE FROM datasets WHERE name = '{0}';
        DROP TABLE "{0}_players";
        DROP TABLE "{0}_network";"#,
        dataset
    );

    connection.execute(query)
}

pub fn rename_dataset(connection: &Connection, old: &str, new: &str) -> sqlite::Result<()> {
    let query = format!(
        r#"UPDATE datasets SET name = '{1}' WHERE name = '{0}';
ALTER TABLE "{0}_players" RENAME TO "{1}_players";
ALTER TABLE "{0}_network" RENAME TO "{1}_network";
DROP INDEX "{0}_network_A";
CREATE INDEX "{1}_network_A" ON "{1}_network" (player_A);
DROP INDEX "{0}_network_B";
CREATE INDEX "{1}_network_B" ON "{1}_network" (player_B);"#,
        old, new
    );

    connection.execute(query)
}

pub fn new_dataset(
    connection: &Connection,
    dataset: &str,
    metadata: DatasetMetadata,
) -> sqlite::Result<()> {
    let query1 = r#"INSERT INTO datasets VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#;
    let query2 = format!(
        r#"CREATE TABLE "{0}_players" (
    id INTEGER PRIMARY KEY,
    last_played INTEGER NOT NULL,
    deviation REAL NOT NULL,
    volatility REAL NOT NULL,

    sets_won TEXT NOT NULL,
    sets_count_won INTEGER AS (length(sets_won) - length(replace(sets_won, ';', ''))),
    sets_lost TEXT NOT NULL,
    sets_count_lost INTEGER AS (length(sets_lost) - length(replace(sets_lost, ';', ''))),
    sets TEXT AS (sets_won || sets_lost),
    sets_count INTEGER AS (sets_count_won + sets_count_lost)
) STRICT;

CREATE TABLE "{0}_network" (
    player_A INTEGER NOT NULL,
    player_B INTEGER NOT NULL,
    advantage REAL NOT NULL,

    sets_A TEXT NOT NULL,
    sets_count_A INTEGER AS (length(sets_A) - length(replace(sets_A, ';', ''))),
    sets_B TEXT NOT NULL,
    sets_count_B INTEGER AS (length(sets_B) - length(replace(sets_B, ';', ''))),
    sets TEXT AS (sets_A || sets_B),
    sets_count INTEGER AS (sets_count_A + sets_count_B),

    UNIQUE (player_A, player_B),
    CHECK (player_A < player_B),
    FOREIGN KEY(player_A) REFERENCES "{0}_players"
        ON DELETE CASCADE,
    FOREIGN KEY(player_B) REFERENCES "{0}_players"
        ON DELETE CASCADE
) STRICT;
CREATE INDEX "{0}_network_A" ON "{0}_network" (player_A);
CREATE INDEX "{0}_network_B" ON "{0}_network" (player_B);"#,
        dataset
    );

    connection
        .prepare(query1)?
        .into_iter()
        .bind((1, dataset))?
        .bind((2, metadata.last_sync.0 as i64))?
        .bind((3, metadata.game_id.0 as i64))?
        .bind((4, &metadata.game_name[..]))?
        .bind((5, &metadata.game_slug[..]))?
        .bind((6, metadata.country.as_deref()))?
        .bind((7, metadata.state.as_deref()))?
        .bind((8, metadata.set_limit as i64))?
        .bind((9, metadata.decay_rate))?
        .bind((10, metadata.adj_decay_rate))?
        .bind((11, metadata.period))?
        .bind((12, metadata.tau))?
        .try_for_each(|x| x.map(|_| ()))?;

    connection.execute(query2)
}

pub fn get_metadata(
    connection: &Connection,
    dataset: &str,
) -> sqlite::Result<Option<DatasetMetadata>> {
    let query = "SELECT * FROM datasets WHERE name = ?";

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
                game_slug: r_.read::<&str, _>("game_slug").to_owned(),
                country: r_.read::<Option<&str>, _>("country").map(String::from),
                state: r_.read::<Option<&str>, _>("state").map(String::from),
                set_limit: r_.read::<i64, _>("set_limit") as u64,
                decay_rate: r_.read::<f64, _>("decay_rate"),
                adj_decay_rate: r_.read::<f64, _>("adj_decay_rate"),
                period: r_.read::<f64, _>("period"),
                tau: r_.read::<f64, _>("tau"),
            })
        })
        .and_then(Result::ok))
}

pub fn update_last_sync(
    connection: &Connection,
    dataset: &str,
    current_time: Timestamp,
) -> sqlite::Result<()> {
    let query = "UPDATE datasets SET last_sync = :sync WHERE name = :dataset";

    connection
        .prepare(query)?
        .into_iter()
        .bind((":sync", current_time.0 as i64))?
        .bind((":dataset", dataset))?
        .try_for_each(|x| x.map(|_| ()))
}

// Database Updating

pub fn add_event(connection: &Connection, event: EventId, slug: &str) -> sqlite::Result<()> {
    let query = "INSERT OR IGNORE INTO events (id, slug) VALUES (?, ?)";

    let mut statement = connection.prepare(&query)?;
    statement.bind((1, event.0 as i64))?;
    statement.bind((2, slug))?;
    statement.into_iter().try_for_each(|x| x.map(|_| ()))
}

pub fn add_set(connection: &Connection, set: &SetId, event: EventId) -> sqlite::Result<()> {
    let query = "INSERT OR IGNORE INTO sets (id, event) VALUES (?, ?)";

    let mut statement = connection.prepare(&query)?;
    statement.bind((1, &set.0.to_string()[..]))?;
    statement.bind((2, event.0 as i64))?;
    statement.into_iter().try_for_each(|x| x.map(|_| ()))
}

pub fn add_players(
    connection: &Connection,
    dataset: &str,
    teams: &Teams<PlayerData>,
    time: Timestamp,
) -> sqlite::Result<()> {
    let query1 = "INSERT OR IGNORE INTO players (id, discrim, name, prefix) VALUES (?, ?, ?, ?)";
    let query2 = format!(
        r#"INSERT OR IGNORE INTO "{}_players"
            (id, last_played, deviation, volatility, sets_won, sets_lost)
            VALUES (?, ?, 2.01, 0.06, '', '')"#,
        dataset
    );

    teams.iter().try_for_each(|team| {
        team.iter().try_for_each(
            |PlayerData {
                 id,
                 name,
                 prefix,
                 discrim,
             }| {
                let mut statement = connection.prepare(&query1)?;
                statement.bind((1, id.0 as i64))?;
                statement.bind((2, &discrim[..]))?;
                statement.bind((3, &name[..]))?;
                statement.bind((4, prefix.as_ref().map(|x| &x[..])))?;
                statement.into_iter().try_for_each(|x| x.map(|_| ()))?;

                statement = connection.prepare(&query2)?;
                statement.bind((1, id.0 as i64))?;
                statement.bind((2, time.0 as i64))?;
                statement.into_iter().try_for_each(|x| x.map(|_| ()))
            },
        )
    })
}

pub fn get_player(connection: &Connection, player: PlayerId) -> sqlite::Result<PlayerData> {
    let query = "SELECT name, prefix, discrim FROM players WHERE id = ?";

    let mut statement = connection.prepare(&query)?;
    statement.bind((1, player.0 as i64))?;
    statement.next()?;
    Ok(PlayerData {
        id: player,
        name: statement.read::<String, _>("name")?,
        prefix: statement.read::<Option<String>, _>("prefix")?,
        discrim: statement.read::<String, _>("discrim")?,
    })
}

pub fn get_player_from_discrim(
    connection: &Connection,
    discrim: &str,
) -> sqlite::Result<PlayerData> {
    let query = "SELECT id, name, prefix FROM players WHERE discrim = ?";

    let mut statement = connection.prepare(&query)?;
    statement.bind((1, discrim))?;
    statement.next()?;
    Ok(PlayerData {
        id: PlayerId(statement.read::<i64, _>("id")? as u64),
        name: statement.read::<String, _>("name")?,
        prefix: statement.read::<Option<String>, _>("prefix")?,
        discrim: discrim.to_owned(),
    })
}

pub fn match_player_name(connection: &Connection, name: &str) -> sqlite::Result<Vec<PlayerData>> {
    let query = "SELECT * FROM players WHERE name LIKE ?";

    connection
        .prepare(&query)?
        .into_iter()
        .bind((1, &format!("%{}%", name)[..]))?
        .map(|r| {
            let r_ = r?;
            Ok(PlayerData {
                id: PlayerId(r_.read::<i64, _>("id") as u64),
                name: r_.read::<&str, _>("name").to_owned(),
                prefix: r_.read::<Option<&str>, _>("prefix").map(|x| x.to_owned()),
                discrim: r_.read::<&str, _>("discrim").to_owned(),
            })
        })
        .try_collect()
}

pub fn get_player_rating_data(
    connection: &Connection,
    dataset: &str,
    player: PlayerId,
) -> sqlite::Result<(f64, f64, Timestamp)> {
    let query = format!(
        r#"SELECT deviation, volatility, last_played FROM "{}_players" WHERE id = ?"#,
        dataset
    );

    let mut statement = connection.prepare(&query)?;
    statement.bind((1, player.0 as i64))?;
    statement.next()?;
    Ok((
        statement.read::<f64, _>("deviation")?,
        statement.read::<f64, _>("volatility")?,
        Timestamp(statement.read::<i64, _>("last_played")? as u64),
    ))
}

pub fn get_player_set_counts(
    connection: &Connection,
    dataset: &str,
    player: PlayerId,
) -> sqlite::Result<(u64, u64)> {
    let query = format!(
        r#"SELECT sets_count_won, sets_count_lost FROM "{}_players" WHERE id = ?"#,
        dataset
    );

    let mut statement = connection.prepare(&query)?;
    statement.bind((1, player.0 as i64))?;
    statement.next()?;
    Ok((
        statement.read::<i64, _>("sets_count_won")? as u64,
        statement.read::<i64, _>("sets_count_lost")? as u64,
    ))
}

pub fn get_matchup_set_counts(
    connection: &Connection,
    dataset: &str,
    player1: PlayerId,
    player2: PlayerId,
) -> sqlite::Result<(u64, u64)> {
    if player1 == player2 {
        return Ok((0, 0));
    }

    let query = format!(
        r#"SELECT iif(:a > :b, sets_count_B, sets_count_A) sets_count_A, iif(:a > :b, sets_count_A, sets_count_B) sets_count_B
            FROM "{}_network" WHERE player_A = min(:a, :b) AND player_B = max(:a, :b)"#,
        dataset
    );

    let mut statement = connection.prepare(&query)?;
    statement.bind((":a", player1.0 as i64))?;
    statement.bind((":b", player2.0 as i64))?;
    statement.next()?;
    Ok((
        statement.read::<i64, _>("sets_count_A")? as u64,
        statement.read::<i64, _>("sets_count_B")? as u64,
    ))
}

pub fn set_player_data(
    connection: &Connection,
    dataset: &str,
    player: PlayerId,
    last_played: Timestamp,
    deviation: f64,
    volatility: f64,
    won: bool,
    set: &SetId,
) -> sqlite::Result<()> {
    let query = format!(
        r#"UPDATE "{}_players" SET deviation = :dev, volatility = :vol, last_played = :last,
            sets_won = iif(:won, sets_won || :set || ';', sets_won),
            sets_lost = iif(:won, sets_lost, sets_lost || :set || ';') WHERE id = :id"#,
        dataset
    );

    let mut statement = connection.prepare(&query)?;
    statement.bind((":dev", deviation))?;
    statement.bind((":vol", volatility))?;
    statement.bind((":last", last_played.0 as i64))?;
    statement.bind((":id", player.0 as i64))?;
    statement.bind((":won", if won { 1 } else { 0 }))?;
    statement.bind((":set", &set.0.to_string()[..]))?;
    statement.next()?;
    Ok(())
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
        r#"INSERT INTO "{}_network"
            (player_A, player_B, advantage, sets_A, sets_B)
            VALUES (min(:a, :b), max(:a, :b), iif(:a > :b, -:v, :v), '', '')"#,
        dataset
    );

    let mut statement = connection.prepare(query)?;
    statement.bind((":a", player1.0 as i64))?;
    statement.bind((":b", player2.0 as i64))?;
    statement.bind((":v", advantage))?;
    statement.into_iter().try_for_each(|x| x.map(|_| ()))
}

pub fn adjust_advantages(
    connection: &Connection,
    dataset: &str,
    set: SetId,
    player1: PlayerId,
    player2: PlayerId,
    winner: usize,
    adjust1: f64,
    adjust2: f64,
    decay_rate: f64,
) -> sqlite::Result<()> {
    let query1 = format!(
        r#"UPDATE "{}_network"
SET advantage = advantage + iif(:pl = player_A, -:v, :v) * :d
WHERE (player_A = :pl AND player_B != :plo)
    OR (player_B = :pl AND player_A != :plo)"#,
        dataset
    );
    let query2 = format!(
        r#"UPDATE "{}_network"
SET advantage = advantage + iif(:a > :b, -:v, :v),
    sets_A = iif(:w = (:a > :b), sets_A || :set || ';', sets_A),
    sets_B = iif(:w = (:b > :a), sets_B || :set || ';', sets_B)
WHERE player_A = min(:a, :b) AND player_B = max(:a, :b)"#,
        dataset
    );

    let mut statement = connection.prepare(&query1)?;
    statement.bind((":pl", player1.0 as i64))?;
    statement.bind((":plo", player2.0 as i64))?;
    statement.bind((":v", adjust1))?;
    statement.bind((":d", decay_rate))?;
    statement.into_iter().try_for_each(|x| x.map(|_| ()))?;

    statement = connection.prepare(&query1)?;
    statement.bind((":pl", player2.0 as i64))?;
    statement.bind((":plo", player1.0 as i64))?;
    statement.bind((":v", adjust2))?;
    statement.bind((":d", decay_rate))?;
    statement.into_iter().try_for_each(|x| x.map(|_| ()))?;

    statement = connection.prepare(&query2)?;
    statement.bind((":a", player1.0 as i64))?;
    statement.bind((":b", player2.0 as i64))?;
    statement.bind((":v", adjust2 - adjust1))?;
    statement.bind((":w", winner as i64))?;
    statement.bind((":set", &set.0.to_string()[..]))?;
    statement.into_iter().try_for_each(|x| x.map(|_| ()))
}

pub fn get_edges(
    connection: &Connection,
    dataset: &str,
    player: PlayerId,
) -> sqlite::Result<Vec<(PlayerId, f64, u64)>> {
    let query = format!(
        r#"SELECT
    iif(:pl = player_B, player_A, player_B) AS id,
    iif(:pl = player_B, -advantage, advantage) AS advantage, sets_count
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
                r_.read::<i64, _>("sets_count") as u64,
            ))
        })
        .try_collect()
}

pub fn either_isolated(
    connection: &Connection,
    dataset: &str,
    player1: PlayerId,
    player2: PlayerId,
) -> sqlite::Result<bool> {
    let query = format!(
        r#"SELECT EXISTS(SELECT 1 FROM "{}_network"
            WHERE player_A = :a OR player_B = :a OR player_A = :b OR player_B = :b)"#,
        dataset
    );

    match connection
        .prepare(&query)?
        .into_iter()
        .bind((":a", player1.0 as i64))?
        .bind((":b", player2.0 as i64))?
        .next()
    {
        None => Ok(true),
        Some(r) => r.map(|_| false),
    }
}

pub fn hypothetical_advantage(
    connection: &Connection,
    dataset: &str,
    player1: PlayerId,
    player2: PlayerId,
    set_limit: u64,
    decay_rate: f64,
    adj_decay_rate: f64,
) -> sqlite::Result<f64> {
    use std::collections::{HashMap, HashSet};

    // Check trivial cases
    if player1 == player2 || either_isolated(connection, dataset, player1, player2)? {
        return Ok(0.0);
    }

    let mut visited: HashSet<PlayerId> = HashSet::new();
    let mut queue: HashMap<PlayerId, (f64, f64)> = HashMap::from([(player1, (0.0, 1.0))]);

    while !queue.is_empty() {
        let visiting = *queue
            .iter()
            .max_by(|a, b| a.1 .1.partial_cmp(&b.1 .1).unwrap())
            .unwrap()
            .0;
        let (adv_v, decay_v) = queue.remove(&visiting).unwrap();
        let connections = get_edges(connection, dataset, visiting)?;

        for (id, adv, sets) in connections
            .into_iter()
            .filter(|(id, _, _)| !visited.contains(id))
        {
            let advantage = adv_v + adv;

            if id == player2 {
                return Ok(advantage * decay_v);
            }

            let decay = decay_v
                * if sets >= set_limit {
                    decay_rate
                } else {
                    adj_decay_rate
                };

            if queue
                .get(&id)
                .map(|(_, decay_old)| *decay_old < decay)
                .unwrap_or(true)
            {
                queue.insert(id, (advantage, decay));
            }
        }

        visited.insert(visiting);
    }

    // No path found
    Ok(0.0)
}

pub fn initialize_edge(
    connection: &Connection,
    dataset: &str,
    player1: PlayerId,
    player2: PlayerId,
    set_limit: u64,
    decay_rate: f64,
    adj_decay_rate: f64,
) -> sqlite::Result<f64> {
    let adv = hypothetical_advantage(
        connection,
        dataset,
        player1,
        player2,
        set_limit,
        decay_rate,
        adj_decay_rate,
    )?;
    insert_advantage(connection, dataset, player1, player2, adv)?;
    Ok(adv)
}

// Tests

#[cfg(test)]
pub mod tests {
    use super::*;

    // Mock a database file in transient memory
    pub fn mock_datasets() -> sqlite::Result<Connection> {
        let query = "PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS datasets (
    name TEXT UNIQUE NOT NULL,
    last_sync INTEGER NOT NULL,
    game_id INTEGER NOT NULL,
    game_name TEXT NOT NULL,
    game_slug TEXT NOT NULL,
    country TEXT,
    state TEXT,
    set_limit INTEGER NOT NULL,
    decay_rate REAL NOT NULL,
    adj_decay_rate REAL NOT NULL,
    period REAL NOT NULL,
    tau REAL NOT NULL
) STRICT;

CREATE TABLE IF NOT EXISTS players (
    id INTEGER PRIMARY KEY,
    discrim TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    prefix TEXT
) STRICT;

CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY,
    slug TEXT NOT NULL
) STRICT;

CREATE TABLE IF NOT EXISTS sets (
    id TEXT UNIQUE NOT NULL,
    event INTEGER NOT NULL,
    FOREIGN KEY(event) REFERENCES events
) STRICT;
";

        let connection = sqlite::open(":memory:")?;
        connection.execute(query)?;
        Ok(connection)
    }

    // Functions to generate placeholder data

    pub fn metadata() -> DatasetMetadata {
        DatasetMetadata {
            last_sync: Timestamp(1),
            game_id: VideogameId(0),
            game_name: String::from("Test Game"),
            game_slug: String::from("test"),
            country: None,
            state: None,
            set_limit: 0,
            decay_rate: 0.5,
            adj_decay_rate: 0.5,
            period: (3600 * 24 * 30) as f64,
            tau: 0.2,
        }
    }

    pub fn players(num: u64) -> Vec<PlayerData> {
        (1..=num)
            .map(|i| PlayerData {
                id: PlayerId(i),
                name: format!("{}", i),
                prefix: None,
                discrim: String::from("a"),
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

        add_players(&connection, "test", &vec![players(2)], Timestamp(0))?;

        let mut statement = connection.prepare("SELECT * FROM players WHERE id = 1")?;
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
        add_players(&connection, "test", &vec![players(2)], Timestamp(0))?;

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
        add_players(&connection, "test", &vec![players(3)], Timestamp(0))?;

        insert_advantage(&connection, "test", PlayerId(2), PlayerId(1), 1.0)?;
        insert_advantage(&connection, "test", PlayerId(1), PlayerId(3), 5.0)?;

        assert_eq!(
            get_edges(&connection, "test", PlayerId(1))?,
            [(PlayerId(2), -1.0, 0), (PlayerId(3), 5.0, 0)]
        );
        assert_eq!(
            get_edges(&connection, "test", PlayerId(2))?,
            [(PlayerId(1), 1.0, 0)]
        );
        assert_eq!(
            get_edges(&connection, "test", PlayerId(3))?,
            [(PlayerId(1), -5.0, 0)]
        );
        Ok(())
    }

    #[test]
    fn hypoth_adv_trivial() -> sqlite::Result<()> {
        let num_players = 3;

        let connection = mock_datasets()?;
        new_dataset(&connection, "test", metadata())?;
        add_players(
            &connection,
            "test",
            &vec![players(num_players)],
            Timestamp(0),
        )?;

        let metadata = metadata();
        for i in 1..=num_players {
            for j in 1..=num_players {
                assert_eq!(
                    hypothetical_advantage(
                        &connection,
                        "test",
                        PlayerId(i),
                        PlayerId(j),
                        metadata.set_limit,
                        metadata.decay_rate,
                        metadata.adj_decay_rate
                    )?,
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
        add_players(&connection, "test", &vec![players(2)], Timestamp(0))?;

        insert_advantage(&connection, "test", PlayerId(1), PlayerId(2), 1.0)?;

        let metadata = metadata();
        assert_eq!(
            hypothetical_advantage(
                &connection,
                "test",
                PlayerId(1),
                PlayerId(2),
                metadata.set_limit,
                metadata.decay_rate,
                metadata.adj_decay_rate
            )?,
            1.0
        );

        Ok(())
    }
}
