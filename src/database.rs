use crate::queries::*;
use sqlite::*;
use std::path::{Path, PathBuf};

pub struct DatasetMetadata {
    pub start: Timestamp,
    pub end: Option<Timestamp>,
    pub last_sync: Timestamp,

    pub game_id: VideogameId,
    pub game_name: String,
    pub game_slug: String,
    pub country: Option<String>,
    pub state: Option<String>,

    pub decay_const: f64,
    pub var_const: f64,
}

/// Return the path to the datasets file.
fn datasets_path(dir: &Path) -> std::io::Result<PathBuf> {
    use std::fs::{self, OpenOptions};

    let mut path = dir.to_owned();

    // Create datasets path if it doesn't exist
    fs::create_dir_all(&path)?;

    path.push("datasets.sqlite");

    // Create datasets file if it doesn't exist
    OpenOptions::new().write(true).create(true).open(&path)?;

    Ok(path)
}

pub fn open_datasets(dir: &Path) -> sqlite::Result<Connection> {
    let path = datasets_path(dir).unwrap();

    let query = "
CREATE TABLE IF NOT EXISTS datasets (
    name TEXT UNIQUE NOT NULL,
    start INTEGER NOT NULL,
    end INTEGER,
    last_sync INTEGER NOT NULL,
    game_id INTEGER NOT NULL,
    game_name TEXT NOT NULL,
    game_slug TEXT NOT NULL,
    country TEXT,
    state TEXT,
    decay_rate REAL NOT NULL,
    var_const REAL NOT NULL
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
    id TEXT PRIMARY KEY,
    event INTEGER NOT NULL REFERENCES events
) STRICT, WITHOUT ROWID;
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
                    start: Timestamp(r_.read::<i64, _>("start") as u64),
                    end: r_
                        .read::<Option<i64>, _>("end")
                        .map(|x| Timestamp(x as u64)),
                    last_sync: Timestamp(r_.read::<i64, _>("last_sync") as u64),
                    game_id: VideogameId(r_.read::<i64, _>("game_id") as u64),
                    game_name: r_.read::<&str, _>("game_name").to_owned(),
                    game_slug: r_.read::<&str, _>("game_slug").to_owned(),
                    country: r_.read::<Option<&str>, _>("country").map(String::from),
                    state: r_.read::<Option<&str>, _>("state").map(String::from),
                    decay_const: r_.read::<f64, _>("decay_rate"),
                    var_const: r_.read::<f64, _>("adj_decay_rate"),
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
    if old == new {
        return Ok(());
    }

    let query = format!(
        r#"UPDATE datasets SET name = '{1}' WHERE name = '{0}';
ALTER TABLE "{0}_players" RENAME TO "{1}_players";
ALTER TABLE "{0}_network" RENAME TO "{1}_network";
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
    let query1 = r#"INSERT INTO datasets VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#;
    let query2 = format!(
        r#"CREATE TABLE "{0}_players" (
    id INTEGER PRIMARY KEY REFERENCES players,

    sets_won TEXT NOT NULL DEFAULT '',
    sets_count_won INTEGER AS (length(sets_won) - length(replace(sets_won, ';', ''))),
    sets_lost TEXT NOT NULL DEFAULT '',
    sets_count_lost INTEGER AS (length(sets_lost) - length(replace(sets_lost, ';', ''))),
    sets TEXT AS (sets_won || sets_lost),
    sets_count INTEGER AS (sets_count_won + sets_count_lost)
) STRICT;

CREATE TABLE "{0}_network" (
    player_A INTEGER NOT NULL,
    player_B INTEGER NOT NULL,
    advantage REAL NOT NULL,
    variance REAL NOT NULL,
    last_updated INTEGER NOT NULL,

    sets_A TEXT NOT NULL DEFAULT '',
    sets_count_A INTEGER AS (length(sets_A) - length(replace(sets_A, ';', ''))),
    sets_B TEXT NOT NULL DEFAULT '',
    sets_count_B INTEGER AS (length(sets_B) - length(replace(sets_B, ';', ''))),
    sets TEXT AS (sets_A || sets_B),
    sets_count INTEGER AS (sets_count_A + sets_count_B),

    PRIMARY KEY (player_A, player_B),
    CHECK (player_A < player_B),
    FOREIGN KEY(player_A) REFERENCES "{0}_players"
        ON DELETE CASCADE,
    FOREIGN KEY(player_B) REFERENCES "{0}_players"
        ON DELETE CASCADE
) STRICT;
CREATE INDEX "{0}_network_B" ON "{0}_network" (player_B);"#,
        dataset
    );

    connection
        .prepare(query1)?
        .into_iter()
        .bind((1, dataset))?
        .bind((2, metadata.start.0 as i64))?
        .bind((3, metadata.end.map(|x| x.0 as i64)))?
        .bind((4, metadata.last_sync.0 as i64))?
        .bind((5, metadata.game_id.0 as i64))?
        .bind((6, &metadata.game_name[..]))?
        .bind((7, &metadata.game_slug[..]))?
        .bind((8, metadata.country.as_deref()))?
        .bind((9, metadata.state.as_deref()))?
        .bind((10, metadata.decay_const))?
        .bind((11, metadata.var_const))?
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
                start: Timestamp(r_.read::<i64, _>("start") as u64),
                end: r_
                    .read::<Option<i64>, _>("end")
                    .map(|x| Timestamp(x as u64)),
                last_sync: Timestamp(r_.read::<i64, _>("last_sync") as u64),
                game_id: VideogameId(r_.read::<i64, _>("game_id") as u64),
                game_name: r_.read::<&str, _>("game_name").to_owned(),
                game_slug: r_.read::<&str, _>("game_slug").to_owned(),
                country: r_.read::<Option<&str>, _>("country").map(String::from),
                state: r_.read::<Option<&str>, _>("state").map(String::from),
                decay_const: r_.read::<f64, _>("decay_rate"),
                var_const: r_.read::<f64, _>("var_const"),
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
    players: &Vec<PlayerData>,
) -> sqlite::Result<()> {
    let query1 = "INSERT OR IGNORE INTO players (id, discrim, name, prefix) VALUES (?, ?, ?, ?)";
    let query2 = format!(
        r#"INSERT OR IGNORE INTO "{}_players" (id) VALUES (?)"#,
        dataset
    );

    players.iter().try_for_each(
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
            statement.into_iter().try_for_each(|x| x.map(|_| ()))
        },
    )
}

pub fn get_all_players(connection: &Connection, dataset: &str) -> sqlite::Result<Vec<PlayerId>> {
    let query = format!(r#"SELECT id FROM "{}_players""#, dataset,);

    connection
        .prepare(&query)?
        .into_iter()
        .map(|r| {
            let r_ = r?;
            Ok(PlayerId(r_.read::<i64, _>("id") as u64))
        })
        .try_collect()
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

pub fn set_player_set_counts(
    connection: &Connection,
    dataset: &str,
    player: PlayerId,
    won: bool,
    set: &SetId,
) -> sqlite::Result<()> {
    let query = format!(
        r#"UPDATE "{}_players" SET
sets_won = iif(:won, sets_won || :set || ';', sets_won),
sets_lost = iif(:won, sets_lost, sets_lost || :set || ';') WHERE id = :id"#,
        dataset
    );

    let mut statement = connection.prepare(&query)?;
    statement.bind((":id", player.0 as i64))?;
    statement.bind((":won", if won { 1 } else { 0 }))?;
    statement.bind((":set", &set.0.to_string()[..]))?;
    statement.next()?;
    Ok(())
}

pub fn get_network_data(
    connection: &Connection,
    dataset: &str,
    player1: PlayerId,
    player2: PlayerId,
) -> sqlite::Result<Option<(f64, f64)>> {
    if player1 == player2 {
        return Ok(Some((0.0, 0.0)));
    }

    let query = format!(
        r#"SELECT iif(:a > :b, -advantage, advantage) AS advantage, variance FROM "{}_network"
            WHERE player_A = min(:a, :b) AND player_B = max(:a, :b)"#,
        dataset
    );

    let mut statement = connection.prepare(&query)?;
    statement.bind((":a", player1.0 as i64))?;
    statement.bind((":b", player2.0 as i64))?;
    statement.next()?;
    Ok(statement
        .read::<Option<f64>, _>("advantage")?
        .zip(statement.read::<Option<f64>, _>("variance")?))
}

pub fn insert_network_data(
    connection: &Connection,
    dataset: &str,
    player1: PlayerId,
    player2: PlayerId,
    advantage: f64,
    variance: f64,
    time: Timestamp,
) -> sqlite::Result<()> {
    let query = format!(
        r#"INSERT INTO "{}_network"
            (player_A, player_B, advantage, variance, last_updated)
            VALUES (min(:a, :b), max(:a, :b), iif(:a > :b, -:v, :v), :d, :t)"#,
        dataset
    );

    let mut statement = connection.prepare(query)?;
    statement.bind((":a", player1.0 as i64))?;
    statement.bind((":b", player2.0 as i64))?;
    statement.bind((":v", advantage))?;
    statement.bind((":d", variance))?;
    statement.bind((":t", time.0 as i64))?;
    statement.into_iter().try_for_each(|x| x.map(|_| ()))
}

pub fn adjust_for_time(
    connection: &Connection,
    dataset: &str,
    player: PlayerId,
    var_const: f64,
    time: Timestamp,
) -> sqlite::Result<()> {
    let query = format!(
        r#"UPDATE "{0}_network" SET
variance = min(variance + :c * (:t - last_updated), 5.0),
last_updated = :t
WHERE player_A = :i OR player_B = :i"#,
        dataset
    );

    let mut statement = connection.prepare(query)?;
    statement.bind((":i", player.0 as i64))?;
    statement.bind((":c", var_const))?;
    statement.bind((":t", time.0 as i64))?;
    statement.into_iter().try_for_each(|x| x.map(|_| ()))
}

pub fn glicko_adjust(
    connection: &Connection,
    dataset: &str,
    set: &SetId,
    player1: PlayerId,
    player2: PlayerId,
    advantage: f64,
    variance: f64,
    winner: usize,
    decay_rate: f64,
) -> sqlite::Result<()> {
    let score = if winner != 0 { 1.0 } else { 0.0 };

    let exp_val = 1.0 / (1.0 + (-advantage).exp());

    let like_var = 1.0 / exp_val / (1.0 - exp_val);
    let var_new = 1.0 / (1.0 / variance + 1.0 / like_var);
    let adjust = score - exp_val;

    let query1 = format!(
        r#"UPDATE "{}_network" SET
variance = 1.0 / (1.0 / variance + :d / :lv),
advantage = advantage + :d * iif(:pl = player_A, -:adj, :adj)
            / (1.0 / variance + :d / :lv)
WHERE (player_A = :pl AND player_B != :plo)
    OR (player_B = :pl AND player_A != :plo)"#,
        dataset
    );
    let query2 = format!(
        r#"UPDATE "{}_network" SET
variance = :var,
advantage = advantage + iif(:a > :b, -:adj, :adj) * :var,
sets_A = iif(:w = (:a > :b), sets_A || :set || ';', sets_A),
sets_B = iif(:w = (:b > :a), sets_B || :set || ';', sets_B)
WHERE player_A = min(:a, :b) AND player_B = max(:a, :b)"#,
        dataset
    );

    let mut statement = connection.prepare(&query1)?;
    statement.bind((":pl", player1.0 as i64))?;
    statement.bind((":plo", player2.0 as i64))?;
    statement.bind((":adj", -0.5 * adjust))?;
    statement.bind((":d", decay_rate))?;
    statement.bind((":lv", like_var))?;
    statement.into_iter().try_for_each(|x| x.map(|_| ()))?;

    statement = connection.prepare(&query1)?;
    statement.bind((":pl", player2.0 as i64))?;
    statement.bind((":plo", player1.0 as i64))?;
    statement.bind((":adj", 0.5 * adjust))?;
    statement.bind((":d", decay_rate))?;
    statement.bind((":lv", like_var))?;
    statement.into_iter().try_for_each(|x| x.map(|_| ()))?;

    statement = connection.prepare(&query2)?;
    statement.bind((":a", player1.0 as i64))?;
    statement.bind((":b", player2.0 as i64))?;
    statement.bind((":adj", adjust))?;
    statement.bind((":var", var_new))?;
    statement.bind((":w", winner as i64))?;
    statement.bind((":set", &set.0.to_string()[..]))?;
    statement.into_iter().try_for_each(|x| x.map(|_| ()))
}

pub fn get_edges(
    connection: &Connection,
    dataset: &str,
    player: PlayerId,
) -> sqlite::Result<Vec<(PlayerId, f64, f64)>> {
    let query = format!(
        r#"SELECT
    iif(:pl = player_B, player_A, player_B) AS id,
    iif(:pl = player_B, -advantage, advantage) AS advantage, variance
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
                r_.read::<f64, _>("variance"),
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
    decay_rate: f64,
) -> sqlite::Result<(f64, f64)> {
    use std::collections::{HashSet, VecDeque};

    // Check trivial cases
    if player1 == player2 {
        return Ok((0.0, 0.0));
    } else if decay_rate < 0.05 || either_isolated(connection, dataset, player1, player2)? {
        return Ok((0.0, 5.0));
    }

    let mut visited: HashSet<PlayerId> = HashSet::new();
    let mut queue: VecDeque<(PlayerId, Vec<(f64, f64, f64)>)> =
        VecDeque::from([(player1, Vec::from([(0.0, 0.0, 1.0 / decay_rate)]))]);

    let mut final_paths = Vec::new();

    while !queue.is_empty() && final_paths.len() < 100 {
        let (visiting, paths) = queue.pop_front().unwrap();

        let connections = get_edges(connection, dataset, visiting)?;

        for (id, adv, var) in connections
            .into_iter()
            .filter(|(id, _, _)| !visited.contains(id))
        {
            let rf = if id == player2 {
                &mut final_paths
            } else if let Some(r) = queue.iter_mut().find(|(id_, _)| id == *id_) {
                &mut r.1
            } else {
                queue.push_back((id, Vec::new()));
                &mut queue.back_mut().unwrap().1
            };

            if rf.len() < 100 {
                let iter = paths
                    .iter()
                    .map(|(av, vr, dec)| (av + adv, vr + var, dec * decay_rate));

                rf.extend(iter);
                rf.truncate(100);
            }
        }

        visited.insert(visiting);
    }

    if final_paths.len() == 0 {
        // No paths found
        Ok((0.0, 5.0))
    } else {
        let sum_decay: f64 = final_paths.iter().map(|(_, _, dec)| dec).sum();
        let (final_adv, final_var) = final_paths
            .into_iter()
            .fold((0.0, 0.0), |(av, vr), (adv, var, dec)| {
                (av + adv * dec, vr + (var + adv * adv) * dec)
            });
        let mut final_adv = final_adv / sum_decay;
        let mut final_var = final_var / sum_decay - final_adv * final_adv;
        if final_var > 5.0 {
            final_adv = final_adv * (5.0 / final_var).sqrt();
            final_var = 5.0;
        }
        Ok((final_adv, final_var))
    }
}

pub fn initialize_edge(
    connection: &Connection,
    dataset: &str,
    player1: PlayerId,
    player2: PlayerId,
    decay_rate: f64,
    time: Timestamp,
) -> sqlite::Result<(f64, f64)> {
    let (adv, var) = hypothetical_advantage(connection, dataset, player1, player2, decay_rate)?;
    insert_network_data(connection, dataset, player1, player2, adv, var, time)?;
    Ok((adv, var))
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
    var_const
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
            start: Timestamp(1),
            end: None,
            last_sync: Timestamp(1),
            game_id: VideogameId(0),
            game_name: String::from("Test Game"),
            game_slug: String::from("test"),
            country: None,
            state: None,
            decay_const: 0.5,
            var_const: 0.00000001,
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
}
