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
