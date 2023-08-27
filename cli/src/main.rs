#![feature(iterator_try_collect)]

use futures::executor::block_on;
use std::io::{self, Write};
use std::path::Path;

mod queries;
use queries::*;
use search_games::{VideogameSearch, VideogameSearchVars};

mod datasets;

fn get_auth_key(config_dir: &Path) -> Option<String> {
    use std::env::{var, VarError};
    use std::fs::read_to_string;

    match var("AUTH_KEY") {
        Ok(key) => Some(key),
        Err(VarError::NotUnicode(_)) => panic!("Invalid authorization key"),
        Err(VarError::NotPresent) => {
            let mut auth_file = config_dir.to_owned();
            auth_file.push("auth.txt");
            read_to_string(auth_file)
                .ok()
                .and_then(|s| s.split_whitespace().next().map(String::from))
        }
    }
}

fn main() {
    let mut config_dir = dirs::config_dir().unwrap();
    config_dir.push("ggelo");
    let auth_key = get_auth_key(&config_dir).expect("Could not find authorization key");

    // Get search prompt
    let mut search = String::new();
    print!("Search for game: ");
    let _ = io::stdout().flush();
    io::stdin()
        .read_line(&mut search)
        .expect("Error reading from stdin");

    if let Some(response) = block_on(run_query::<VideogameSearch, _>(
        VideogameSearchVars { name: search },
        &auth_key,
    )) {
        for game in response.into_iter() {
            println!("{} - {}", game.id.0, game.name);
        }
    } else {
        println!("No response");
    }
}
