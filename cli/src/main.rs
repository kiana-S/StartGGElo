#![feature(iterator_try_collect)]

use std::path::Path;

mod queries;
use queries::*;

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
            read_to_string(auth_file).ok().and_then(|s| {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_owned())
                }
            })
        }
    }
}

fn main() {
    let mut config_dir = dirs::config_dir().unwrap();
    config_dir.push("ggelo");
    let auth_key = get_auth_key(&config_dir).expect("Could not find authorization key");

    if let Some(response) = run_query::<TournamentSets, _>(
        TournamentSetsVars {
            last_query: Timestamp(1),
            game_id: VideogameId(1386),
            country: None,
            state: Some("GA"),
        },
        &auth_key,
    ) {
        println!("Succeeded");
        for tournament in response {
            println!("Tournament: {}", tournament.name);
            for set in tournament.sets {
                println!(
                    "Winner: {}",
                    if set.winner {
                        set.player2.0
                    } else {
                        set.player1.0
                    }
                );
            }
        }
    } else {
        println!("Invalid GraphQL response");
    }
}
