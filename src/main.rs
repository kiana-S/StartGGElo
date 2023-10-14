#![feature(iterator_try_collect)]
#![feature(extend_one)]

use clap::{Parser, Subcommand};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::exit;

mod queries;
use queries::*;
mod datasets;
use datasets::*;
mod sync;
use sync::*;

pub fn error(msg: &str, code: i32) -> ! {
    println!("\nERROR: {}", msg);
    exit(code)
}

pub fn issue(msg: &str, code: i32) -> ! {
    println!("\n{}", msg);
    exit(code)
}

/// ## CLI Structs

#[derive(Parser)]
#[command(name = "StartRNR")]
#[command(author = "Kiana Sheibani <kiana.a.sheibani@gmail.com>")]
#[command(version = "0.2.0")]
#[command(about = "StartRNR - Elo rating calculator for start.gg tournaments", long_about = None)]
struct Cli {
    #[command(subcommand)]
    subcommand: Subcommands,

    #[arg(
        short = 'A',
        long = "auth",
        value_name = "TOKEN",
        global = true,
        help = "Authentication token",
        long_help = "The authentication token for accessing start.gg.
A token can be specified using this argument, in the environment variable
AUTH_TOKEN, or in a text file '<CONFIG_DIR>/auth.txt'."
    )]
    auth_token: Option<String>,

    #[arg(
        short,
        long = "config",
        value_name = "DIR",
        global = true,
        help = "Config directory",
        long_help = "This flag overrides the default config directory.
If this directory does not exist, it will be created and a database file will
be initialized within it."
    )]
    config_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Subcommands {
    #[command(about = "Manipulate stored datasets")]
    Dataset {
        #[command(subcommand)]
        subcommand: DatasetSC,
    },
    #[command(
        about = "Sync player ratings",
        long_about = "Pull recent tournament data off of start.gg and use it to
update the network. This command will automatically keep track of the last time each
dataset was synced to ensure that each tournament is only accounted for once."
    )]
    Sync {
        #[arg(
            help = "The datasets to sync",
            long_help = "A list of datasets to sync.
If no datasets are given, then the dataset 'default' is synced. This dataset is
created if it does not already exist."
        )]
        datasets: Vec<String>,
        #[arg(short, long, help = "Sync all stored databases")]
        all: bool,
    },
}

#[derive(Subcommand)]
enum DatasetSC {
    #[command(about = "List datasets")]
    List,
    #[command(about = "Create a new dataset")]
    New { name: Option<String> },
    #[command(about = "Delete a dataset")]
    Delete { name: Option<String> },
}

fn main() {
    let cli = Cli::parse();

    match cli.subcommand {
        Subcommands::Dataset {
            subcommand: DatasetSC::List,
        } => dataset_list(),
        Subcommands::Dataset {
            subcommand: DatasetSC::New { name },
        } => dataset_new(name, cli.auth_token),
        Subcommands::Dataset {
            subcommand: DatasetSC::Delete { name },
        } => dataset_delete(name),

        Subcommands::Sync { datasets, all } => sync(datasets, all, cli.auth_token),
    }
}

fn dataset_list() {
    let config_dir = dirs::config_dir().expect("Could not determine config directory");

    let connection =
        open_datasets(&config_dir).unwrap_or_else(|_| error("Could not open datasets file", 2));
    let datasets = list_datasets(&connection).expect("Error communicating with SQLite");

    println!();
    for (name, metadata) in datasets {
        if let Some(country) = metadata.country {
            if let Some(state) = metadata.state {
                println!(
                    "{} - {} (in {}, {})",
                    name, metadata.game_name, country, state
                );
            } else {
                println!("{} - {} (in {})", name, metadata.game_name, country);
            }
        } else {
            println!("{} - {}", name, metadata.game_name);
        }
    }
}

fn read_string() -> String {
    let mut line = String::new();
    io::stdout()
        .flush()
        .unwrap_or_else(|_| error("Could not access stdout", 2));
    io::stdin()
        .read_line(&mut line)
        .unwrap_or_else(|_| error("Could not read from stdin", 2));
    line.trim().to_owned()
}

fn dataset_new(name: Option<String>, auth_token: Option<String>) {
    let config_dir = dirs::config_dir().expect("Could not determine config directory");

    let auth = auth_token
        .or_else(|| get_auth_token(&config_dir))
        .unwrap_or_else(|| error("Access token not provided", 1));

    // Name

    let name = name.unwrap_or_else(|| {
        print!("Name of new dataset: ");
        read_string()
    });

    // Game

    print!("Search games: ");
    let games = run_query::<VideogameSearch, _>(
        VideogameSearchVars {
            name: &read_string(),
        },
        &auth,
    )
    .unwrap_or_else(|| error("Could not access start.gg", 1));

    if games.is_empty() {
        issue("No games found!", 0);
    }

    println!("\nSearch results:");
    for (i, game) in games.iter().enumerate() {
        println!("{} - {}", i, game.name);
    }

    print!("\nGame to track ratings for (0-{}): ", games.len() - 1);
    let index = read_string()
        .parse::<usize>()
        .unwrap_or_else(|_| error("Not an integer", 1));
    if index >= games.len() {
        error("Out of range!", 1);
    }

    let VideogameData {
        id: game_id,
        name: game_name,
    } = games[index].clone();

    // Location

    print!(
        "
\x1b[4mCountry\x1b[0m

Enter the two-letter code for the country you want to track ratings in, e.g.
\"US\" for the United States. See \x1b[1m\x1b]8;;https://www.ups.com/worldshiphelp/\
WSA/ENU/AppHelp/mergedProjects/CORE/Codes/Country_Territory_and_Currency_Codes.htm\
\x1b\\this site\x1b]8;;\x1b\\\x1b[0m for a list of these codes.
If no code is entered, then the dataset will track all players globally.

Country to track ratings for (leave empty for none): "
    );
    let country = {
        let mut string = read_string();
        if string.is_empty() {
            None
        } else if string.len() == 2 && string.chars().all(|c| c.is_ascii_alphabetic()) {
            string.make_ascii_uppercase();
            Some(string)
        } else {
            error("Input is not a two-letter code", 1);
        }
    };

    let state = if country.as_ref().is_some_and(|s| s == "US" || s == "CA") {
        print!(
            "
\x1b[4mState/Province\x1b[0m

Enter the two-letter code for the US state or Canadian province you want to track
ratings in, e.g. \"CA\" for California. See \x1b[1m\x1b]8;;https://www.ups.com/worldshiphelp/\
WSA/ENU/AppHelp/mergedProjects/CORE/Codes/State_Province_Codes.htm\x1b\\this site\
\x1b]8;;\x1b\\\x1b[0m for a list of these codes.
If no code is entered, then the dataset will track all players within the country.

State/province to track ratings for (leave empty for none): "
        );
        let mut string = read_string();
        if string.is_empty() {
            None
        } else if string.len() == 2 && string.chars().all(|c| c.is_ascii_alphabetic()) {
            string.make_ascii_uppercase();
            Some(string)
        } else {
            error("Input is not a two-letter code", 1);
        }
    } else {
        None
    };

    // Advanced Options

    // Defaults
    let mut decay_rate = 0.5;
    let mut period_days = 30.0;
    let mut tau = 0.2;

    print!("\nConfigure advanced options? (y/n) ");
    if let Some('y') = read_string().chars().next() {
        // Decay Rate

        print!(
            "
\x1b[4mNetwork Decay Rate\x1b[0m

The network decay rate is a number between 0 and 1 that controls how the
advantage network reacts to player wins and losses. If the decay rate is 1,
then it is assumed that a player's skill against one opponent always carries
over to all other opponents. If the decay rate is 0, then all player match-ups
are assumed to be independent of each other.

Network decay rate (default 0.5): "
        );
        let decay_rate_input = read_string();
        if !decay_rate_input.is_empty() {
            decay_rate = decay_rate_input
                .parse::<f64>()
                .unwrap_or_else(|_| error("Not a number", 1));
        }

        // Rating Period

        print!(
            "
\x1b[4mRating Period\x1b[0m

The rating period is an interval of time that dictates how player ratings change
during inactivity. Ideally the rating period should be somewhat long, long
enough to expect almost every player in the dataset to have played at least a
few sets.

Rating period (in days, default 30): "
        );
        let period_input = read_string();
        if !period_input.is_empty() {
            period_days = period_input
                .parse::<f64>()
                .unwrap_or_else(|_| error("Not a number", 1));
        }

        // Tau coefficient

        print!(
            "
\x1b[4mTau Constant\x1b[0m

The tau constant is an internal system constant that roughly represents how
much random chance and luck play a role in game outcomes. In games where match
results are highly predictable, and a player's skill is the sole factor for
whether they will win, the tau constant should be high (0.9 - 1.2). In games
where luck matters, and more improbable victories can occur, the tau constant
should be low (0.2 - 0.4).

The tau constant is set low by default, since skill-based competitive video
games tend to be on the more luck-heavy side.

Tau constant (default 0.2): "
        );
        let tau_input = read_string();
        if !tau_input.is_empty() {
            tau = tau_input
                .parse::<f64>()
                .unwrap_or_else(|_| error("Not a number", 1));
        }
    }

    // Done configuring

    let connection =
        open_datasets(&config_dir).unwrap_or_else(|_| error("Could not open datasets file", 2));
    new_dataset(
        &connection,
        &name,
        DatasetMetadata {
            last_sync: Timestamp(1),
            game_id,
            game_name,
            country,
            state,
            decay_rate,
            period: (3600 * 24) as f64 * period_days,
            tau,
        },
    )
    .expect("Error communicating with SQLite");

    println!("\nCreated dataset {}", name);
}

fn dataset_delete(name: Option<String>) {
    let config_dir = dirs::config_dir().expect("Could not determine config directory");

    let name = name.unwrap_or_else(|| {
        print!("Dataset to delete: ");
        read_string()
    });

    let connection =
        open_datasets(&config_dir).unwrap_or_else(|_| error("Could not open datasets file", 2));
    delete_dataset(&connection, &name).unwrap_or_else(|_| error("That dataset does not exist!", 1));
}

fn sync(datasets: Vec<String>, all: bool, auth_token: Option<String>) {
    let config_dir = dirs::config_dir().unwrap();

    let auth = auth_token
        .or_else(|| get_auth_token(&config_dir))
        .unwrap_or_else(|| error("Access token not provided", 1));

    let connection =
        open_datasets(&config_dir).unwrap_or_else(|_| error("Could not open datasets file", 2));

    let all_datasets = list_dataset_names(&connection).unwrap();

    #[allow(unused_must_use)]
    let datasets = if all {
        all_datasets
    } else if datasets.is_empty() {
        if all_datasets.is_empty() {
            print!("No datasets exist; create one? (y/n) ");
            if let Some('y') = read_string().chars().next() {
                dataset_new(Some(String::from("default")), Some(auth.clone()));
                vec![String::from("default")]
            } else {
                error("No datasets specified and no default dataset", 1)
            }
        } else if all_datasets.iter().any(|x| x == "default") {
            vec![String::from("default")]
        } else {
            error("No datasets specified and no default dataset", 1);
        }
    } else {
        datasets
    };

    for dataset in datasets {
        let dataset_config = get_metadata(&connection, &dataset)
            .expect("Error communicating with SQLite")
            .unwrap_or_else(|| error(&format!("Dataset {} does not exist!", dataset), 1));

        sync_dataset(&connection, &dataset, dataset_config, &auth)
            .unwrap_or_else(|_| error("Error communicating with SQLite", 2));

        update_last_sync(&connection, &dataset).expect("Error communicating with SQLite");
    }
}
