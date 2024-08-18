#![feature(iterator_try_collect)]
#![feature(extend_one)]

use chrono::{Local, TimeZone, Utc};
use clap::{Parser, Subcommand};
use sqlite::*;
use std::{cmp::min, path::PathBuf};

mod queries;
use queries::*;
mod database;
use database::*;
mod sync;
use sync::*;
mod util;
use util::*;

/// ## CLI Structs

#[derive(Parser)]
#[command(name = "StartRNR")]
#[command(author = "Kiana Sheibani <kiana.a.sheibani@gmail.com>")]
#[command(version = "0.2.0")]
#[command(about = "StartRNR - Rating system for competitive video games based on start.gg",
          long_about = None)]
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
        long_help = "This flag overrides the default config directory."
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
update player ratings. This command will automatically keep track of the last time
each dataset was synced to ensure that each tournament is only accounted for once."
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
    #[command(about = "Access player information")]
    Player {
        #[command(subcommand)]
        subcommand: PlayerSC,
        #[arg(short, long, global = true, help = "The dataset to access")]
        dataset: Option<String>,
    },
    Ranking {
        #[command(subcommand)]
        subcommand: RankingSC,
        #[arg(short, long, global = true, help = "The dataset to access")]
        dataset: Option<String>,
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
    #[command(about = "Rename a dataset")]
    Rename {
        old: Option<String>,
        new: Option<String>,
    },
}

#[derive(Subcommand)]
enum PlayerSC {
    #[command(about = "Get info about a player")]
    Info { player: String },
    #[command(about = "Matchup data between two players")]
    Matchup { player1: String, player2: String },
}

#[derive(Subcommand)]
enum RankingSC {
    #[command(about = "Create a new ranking")]
    Create,
}

fn main() {
    let cli = Cli::parse();

    let config_dir = cli.config_dir.unwrap_or_else(|| {
        let mut dir = dirs::config_dir().expect("Could not determine config directory");
        dir.push("startrnr");
        dir
    });

    let mut data_dir = dirs::data_dir().expect("Could not determine user data directory");
    data_dir.push("startrnr");

    let connection =
        open_datasets(&data_dir).unwrap_or_else(|_| error("Could not open datasets file", 2));

    #[allow(unreachable_patterns)]
    match cli.subcommand {
        Subcommands::Dataset {
            subcommand: DatasetSC::List,
        } => dataset_list(&connection),
        Subcommands::Dataset {
            subcommand: DatasetSC::New { name },
        } => dataset_new(&connection, get_auth_token(&config_dir), name),
        Subcommands::Dataset {
            subcommand: DatasetSC::Delete { name },
        } => dataset_delete(&connection, name),
        Subcommands::Dataset {
            subcommand: DatasetSC::Rename { old, new },
        } => dataset_rename(&connection, old, new),

        Subcommands::Player {
            subcommand: PlayerSC::Info { player },
            dataset,
        } => player_info(&connection, dataset, player),
        Subcommands::Player {
            subcommand: PlayerSC::Matchup { player1, player2 },
            dataset,
        } => player_matchup(&connection, dataset, player1, player2),

        Subcommands::Sync { datasets, all } => {
            sync(&connection, get_auth_token(&config_dir), datasets, all)
        }

        Subcommands::Ranking {
            subcommand: RankingSC::Create,
            dataset,
        } => ranking_create(&connection, dataset),

        _ => eprintln!("This feature is currently unimplemented."),
    }
}

// Datasets

fn dataset_list(connection: &Connection) {
    let datasets = list_datasets(&connection).expect("Error communicating with SQLite");

    for (name, metadata) in datasets {
        print!(
            "· \x1b[1m\x1b[34m{}\x1b[0m
\x1b[4m\x1b]8;;https://www.start.gg/{}\x1b\\{}\x1b]8;;\x1b\\\x1b[0m ",
            name, metadata.game_slug, metadata.game_name
        );

        if let Some(country) = metadata.country {
            if let Some(state) = metadata.state {
                println!("(in {}, {})", country, state);
            } else {
                println!("(in {})", country);
            }
        } else {
            println!("(Global)");
        }

        let start = if metadata.start.0 != 1 {
            Some(
                Utc.timestamp_opt(metadata.start.0 as i64, 0)
                    .unwrap()
                    .format("%m/%d/%Y"),
            )
        } else {
            None
        };
        let end = metadata
            .end
            .map(|x| Utc.timestamp_opt(x.0 as i64, 0).unwrap().format("%m/%d/%Y"));

        match (start, end) {
            (None, None) => (),
            (Some(s), None) => println!("after {}", s),
            (None, Some(e)) => println!("until {}", e),
            (Some(s), Some(e)) => println!("{} - {}", s, e),
        }

        if metadata.last_sync == metadata.start {
            print!("\x1b[1m\x1b[91mUnsynced\x1b[0m");
        } else if Some(metadata.last_sync) == metadata.end {
            print!("\x1b[1m\x1b[92mComplete\x1b[0m");
        } else {
            print!(
                "\x1b[1mLast synced:\x1b[0m {}",
                Local
                    .timestamp_opt(metadata.last_sync.0 as i64, 0)
                    .unwrap()
                    .format("%b %e, %Y %r")
            );
        }
        if current_time().0 - metadata.last_sync.0 > SECS_IN_WEEK
            && Some(metadata.last_sync) != metadata.end
        {
            if name == "default" {
                print!(" - \x1b[33mRun 'startrnr sync' to update!\x1b[0m");
            } else {
                print!(
                    " - \x1b[33mRun 'startrnr sync {:?}' to update!\x1b[0m",
                    name
                );
            }
        }
        println!(
            "\n\x1b[1mNetwork Decay Constant:\x1b[0m {}",
            metadata.decay_const
        );
    }
}

fn dataset_new(connection: &Connection, auth: String, name: Option<String>) {
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
        slug: game_slug,
    } = games[index].clone();

    // Location

    print!(
        "
\x1b[1mCountry\x1b[0m
Enter the two-letter code for the country you want to track ratings in, e.g.
\"US\" for the United States. See \x1b[4m\x1b]8;;https://www.ups.com/worldshiphelp/\
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
\x1b[1mState/Province\x1b[0m
Enter the two-letter code for the US state or Canadian province you want to track
ratings in, e.g. \"CA\" for California. See \x1b[4m\x1b]8;;https://www.ups.com/worldshiphelp/\
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

    // Interval

    print!(
        "
\x1b[1mStart Date\x1b[0m
The rating system will process tournaments starting at this date. If only a year
is entered, the date will be the start of that year.

Start date (year, m/y, or m/d/y): "
    );
    let start = {
        let string = read_string();
        if string.is_empty() {
            Timestamp(1)
        } else if string.chars().all(|c| c.is_ascii_digit() || c == '/') {
            if let Some((y, m, d)) = match string.split('/').collect::<Vec<_>>()[..] {
                [] => None,
                [y] => Some((y.parse().unwrap(), 1, 1)),
                [m, y] => Some((y.parse().unwrap(), m.parse().unwrap(), 1)),
                [m, d, y] => Some((y.parse().unwrap(), m.parse().unwrap(), d.parse().unwrap())),
                _ => error("Input is not a date", 1),
            } {
                Timestamp(Utc.with_ymd_and_hms(y, m, d, 0, 1, 1).unwrap().timestamp() as u64)
            } else {
                Timestamp(1)
            }
        } else {
            error("Input is not a date", 1);
        }
    };

    print!(
        "
\x1b[1mEnd Date\x1b[0m
The rating system will stop processing tournaments when it reaches this date. If
only a year is entered, the date will be the end of that year.

End date (year, m/y, or m/d/y): "
    );
    let end = {
        let string = read_string();
        if string.is_empty() {
            None
        } else if string.chars().all(|c| c.is_ascii_digit() || c == '/') {
            if let Some((y, m, d)) = match string.split('/').collect::<Vec<_>>()[..] {
                [] => None,
                [y] => Some((y.parse().unwrap(), 12, 31)),
                [m, y] => Some((y.parse().unwrap(), m.parse().unwrap(), 30)),
                [m, d, y] => Some((y.parse().unwrap(), m.parse().unwrap(), d.parse().unwrap())),
                _ => error("Input is not a date", 1),
            } {
                Some(Timestamp(
                    Utc.with_ymd_and_hms(y, m, d, 11, 59, 59)
                        .unwrap()
                        .timestamp() as u64,
                ))
            } else {
                None
            }
        } else {
            error("Input is not a date", 1);
        }
    };

    // Advanced Options

    // Defaults
    let mut decay_const = 0.9;
    let mut var_const = (10.0 - 0.04) / SECS_IN_YEAR as f64 / 3.0;

    print!("\nConfigure advanced options? (y/n) ");
    if let Some('y') = read_string().chars().next() {
        // Decay Rate

        print!(
            "
\x1b[1mNetwork Decay Constant\x1b[0m
The network decay constant is a number between 0 and 1 that controls how
player wins and losses propagate throughout the network. If the decay
constant is 1, then it is assumed that a player's skill against one
opponent always carries over to all other opponents. If the decay
constant is 0, then all player match-ups are assumed to be independent of
each other.

Network decay constant (default 0.9): "
        );
        let decay_const_input = read_string();
        if !decay_const_input.is_empty() {
            decay_const = decay_const_input
                .parse::<f64>()
                .unwrap_or_else(|_| error("Input is not a number", 1));
            if decay_const < 0.0 || decay_const > 1.0 {
                error("Input is not between 0 and 1", 1);
            }
        }

        // Variance Constant

        print!(
            "
\x1b[1mVariance Rate\x1b[0m
This constant determines how quickly a player's variance (the uncertainty
of their rating) increases over time. See the end of \x1b[4m\x1b]8;;http:\
//www.glicko.net/glicko/glicko.pdf\x1b\\this paper\x1b]8;;\x1b\\\x1b[0m for details
on how to compute a good value, or you can leave it blank and a reasonable
default will be chosen.

Variance rate: "
        );
        let var_const_input = read_string();
        if !var_const_input.is_empty() {
            var_const = var_const_input
                .parse::<f64>()
                .unwrap_or_else(|_| error("Input is not a number", 1));
        }
    }

    // Done configuring

    new_dataset(
        connection,
        &name,
        DatasetMetadata {
            start,
            end,
            last_sync: start,
            game_id,
            game_name,
            game_slug,
            country,
            state,
            decay_const,
            var_const,
        },
    )
    .expect("Error communicating with SQLite");

    println!("\nCreated dataset {}", name);
}

fn dataset_delete(connection: &Connection, name: Option<String>) {
    let name = name.unwrap_or_else(|| {
        print!("Dataset to delete: ");
        read_string()
    });

    delete_dataset(connection, &name).unwrap_or_else(|_| error("That dataset does not exist!", 1));
}

fn dataset_rename(connection: &Connection, old: Option<String>, new: Option<String>) {
    let old = old.unwrap_or_else(|| {
        print!("Dataset to rename: ");
        read_string()
    });
    let new = new.unwrap_or_else(|| {
        print!("Rename to: ");
        read_string()
    });

    match rename_dataset(connection, &old, &new) {
        Ok(()) => (),
        Err(sqlite::Error {
            code: Some(1),
            message: _,
        }) => error(&format!("Dataset {:?} does not exist", &old), 1),
        Err(sqlite::Error {
            code: Some(19),
            message: _,
        }) => error(&format!("Dataset {:?} already exists", &new), 1),
        Err(_) => error("Unknown error occurred", 2),
    };
}

// Players

fn player_info(connection: &Connection, dataset: Option<String>, player: String) {
    let dataset = dataset.unwrap_or_else(|| String::from("default"));

    let PlayerData {
        id,
        name,
        prefix,
        discrim,
    } = get_player_from_input(connection, player)
        .unwrap_or_else(|_| error("Could not find player", 1));

    let (won, lost) = get_player_set_counts(connection, &dataset, id)
        .unwrap_or_else(|_| error("Could not find player", 1));

    if let Some(pre) = prefix {
        print!("\x1b[2m{}\x1b[22m ", pre);
    }
    println!(
        "\x1b[4m\x1b]8;;https://www.start.gg/user/{1}\x1b\\\
\x1b[1m{0}\x1b[22m\x1b]8;;\x1b\\\x1b[0m ({1})",
        name, discrim
    );
    println!("\x1b[1mID:\x1b[0m {}", id.0);

    println!(
        "\n\x1b[1mSet Count:\x1b[0m {} - {} ({:.3}%)",
        won,
        lost,
        (won as f64 / (won + lost) as f64) * 100.0
    );
}

fn player_matchup(
    connection: &Connection,
    dataset: Option<String>,
    player1: String,
    player2: String,
) {
    let dataset = dataset.unwrap_or_else(|| String::from("default"));

    let PlayerData {
        id: player1,
        name: name1,
        prefix: prefix1,
        discrim: discrim1,
    } = get_player_from_input(connection, player1)
        .unwrap_or_else(|_| error("Could not find player", 1));

    let PlayerData {
        id: player2,
        name: name2,
        prefix: prefix2,
        discrim: discrim2,
    } = get_player_from_input(connection, player2)
        .unwrap_or_else(|_| error("Could not find player", 1));

    let (hypothetical, advantage, variance) =
        get_network_data(connection, &dataset, player1, player2)
            .expect("Error communicating with SQLite")
            .map(|(adv, var)| (false, adv, var))
            .unwrap_or_else(|| {
                let metadata = get_metadata(connection, &dataset)
                    .expect("Error communicating with SQLite")
                    .unwrap_or_else(|| error("Dataset not found", 1));
                let (adv, var) = hypothetical_advantage(
                    connection,
                    &dataset,
                    player1,
                    player2,
                    metadata.decay_const,
                )
                .expect("Error communicating with SQLite");
                (true, adv, var)
            });

    let probability = 1.0 / (1.0 + f64::exp(-advantage));

    let (color, other_color) = ansi_num_color(advantage, 0.2, 2.0);

    let len1 = prefix1.as_deref().map(|s| s.len() + 1).unwrap_or(0) + name1.len();
    let len2 = prefix2.as_deref().map(|s| s.len() + 1).unwrap_or(0) + name2.len();

    // Prefix + name for each player

    if let Some(pre) = prefix1 {
        print!("\x1b[2m{}\x1b[22m ", pre);
    }
    print!(
        "\x1b[4m\x1b]8;;https://www.start.gg/user/{}\x1b\\\
\x1b[1m{}\x1b[22m\x1b]8;;\x1b\\\x1b[0m - ",
        discrim1, name1
    );
    if let Some(pre) = prefix2 {
        print!("\x1b[2m{}\x1b[22m ", pre);
    }
    println!(
        "\x1b[4m\x1b]8;;https://www.start.gg/user/{}\x1b\\\
\x1b[1m{}\x1b[22m\x1b]8;;\x1b\\\x1b[0m",
        discrim2, name2
    );

    // Probability breakdown

    println!(
        "\x1b[1m\x1b[{4}m{0:>2$}\x1b[0m - \x1b[1m\x1b[{5}m{1:<3$}\x1b[0m",
        format!("{:.1}%", (1.0 - probability) * 100.0),
        format!("{:.1}%", probability * 100.0),
        len1,
        len2,
        other_color,
        color
    );

    // Advantage + variance

    println!(
        "\n\x1b[1m{0}Advantage: \x1b[{1}m{2:+.4}\x1b[39m\n{0}Variance: {3:.4}\x1b[0m",
        if hypothetical { "Hypothetical " } else { "" },
        color,
        advantage,
        variance
    );

    if !hypothetical {
        // Set count

        let (a, b) = get_matchup_set_counts(connection, &dataset, player1, player2)
            .expect("Error communicating with SQLite");

        println!(
            "\n\x1b[1mSet Count:\x1b[0m {} - {}  ({:.3}% - {:.3}%)",
            a,
            b,
            (a as f64 / (a + b) as f64) * 100.0,
            (b as f64 / (a + b) as f64) * 100.0
        );
    }
}

// Sync

fn sync(connection: &Connection, auth: String, datasets: Vec<String>, all: bool) {
    let all_datasets = list_dataset_names(connection).unwrap();

    let datasets = if all {
        all_datasets
    } else if datasets.is_empty() {
        if all_datasets.is_empty() {
            print!("No datasets exist; create one? (y/n) ");
            if let Some('y') = read_string().chars().next() {
                dataset_new(connection, auth.clone(), Some(String::from("default")));
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

    let current_time = current_time();

    for dataset in datasets {
        let dataset_metadata = get_metadata(connection, &dataset)
            .expect("Error communicating with SQLite")
            .unwrap_or_else(|| error(&format!("Dataset {} does not exist!", dataset), 1));

        let before = dataset_metadata
            .end
            .map(|end| min(end, current_time))
            .unwrap_or(current_time);

        sync_dataset(connection, &dataset, dataset_metadata, before, &auth)
            .expect("Error communicating with SQLite");

        update_last_sync(connection, &dataset, before).expect("Error communicating with SQLite");
    }
}

fn ranking_create(connection: &Connection, dataset: Option<String>) {
    use std::collections::HashMap;

    let dataset = dataset.unwrap_or_else(|| String::from("default"));

    let metadata = get_metadata(connection, &dataset)
        .expect("Error communicating with SQLite")
        .unwrap_or_else(|| error("Dataset not found", 1));

    let exp = read_string().parse::<f64>().unwrap();

    let players = get_all_players(connection, &dataset).expect("Error communicating with SQLite");
    let num_players = players.len();
    let mut table = players
        .into_iter()
        .map(|id| (id, 1.0 / num_players as f64))
        .collect::<HashMap<_, _>>();
    table.shrink_to_fit();

    let mut diff: f64 = 1.0;
    let mut iter = 0;

    while diff > 1e-8 {
        let mut new_table = HashMap::with_capacity(table.capacity());

        for (&id, &last) in table.iter() {
            let mut points = get_edges(connection, &dataset, id)
                .expect("Error communicating with SQLite")
                .into_iter()
                .map(|(other, adv, _sets)| (other, exp.powf(adv)))
                .collect::<Vec<_>>();

            points.push((id, 1.0));

            let sum_points = points.iter().map(|(_, val)| val).sum::<f64>();

            points.into_iter().for_each(|(other, pts)| {
                let pts_ = last * pts / sum_points;
                new_table
                    .entry(other)
                    .and_modify(|v| *v += pts_)
                    .or_insert(pts_);
            })
        }

        if iter % 10 == 0 {
            diff = (table
                .iter()
                .map(|(id, &last)| (new_table[id] - last) * (new_table[id] - last))
                .sum::<f64>()
                / num_players as f64)
                .sqrt();
            println!("{}", diff);
        }

        table = new_table;
        iter += 1;
    }

    let mut list = table.into_iter().collect::<Vec<_>>();
    list.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());

    println!();
    for (id, pts) in list.into_iter().take(20) {
        let player = get_player(connection, id).unwrap();
        println!("{} - {}", player.name, pts);
    }
}
