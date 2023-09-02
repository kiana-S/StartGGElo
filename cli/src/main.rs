#![feature(iterator_try_collect)]

mod queries;
use queries::*;

mod datasets;

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
