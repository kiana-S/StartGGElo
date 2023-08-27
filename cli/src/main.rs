use std::io::{self, Write};
use std::path::Path;
use futures::executor::block_on;

mod queries;
use queries::search_games::{VideogameSearch, VideogameSearchVars};


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

async fn run_query(name: &str, auth: &str) -> cynic::GraphQlResponse<VideogameSearch> {
    use cynic::http::SurfExt;
    use cynic::QueryBuilder;

    let query = VideogameSearch::build(VideogameSearchVars {
        name: String::from(name)
    });

    let response = surf::post("https://api.start.gg/gql/alpha")
        .header("Authorization", String::from("Bearer ") + auth)
        .run_graphql(query)
        .await;

    response.unwrap()
}

fn main() {
    let mut config_dir = dirs::config_dir().unwrap();
    config_dir.push("ggelo");
    let auth_key = get_auth_key(&config_dir).expect("Could not find authorization key");

    // Get search prompt
    let mut search = String::new();
    print!("Search for game: ");
    let _ = io::stdout().flush();
    io::stdin().read_line(&mut search).expect("Error reading from stdin");

    if let Some(response) = block_on(run_query(&search, &auth_key)).data {
        for maybe_game in response.videogames.unwrap().nodes.unwrap().into_iter() {
            let game = maybe_game.unwrap();
            println!("{:?} - {}", game.id.unwrap(), game.name.unwrap());
        }
    } else {
        println!("No response");
    }
}
