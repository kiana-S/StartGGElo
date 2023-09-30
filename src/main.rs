#![feature(iterator_try_collect)]

use clap::{Parser, Subcommand};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::SystemTime;

mod queries;
use queries::*;
mod datasets;
use datasets::*;

/// ## CLI Structs

#[derive(Parser)]
#[command(name = "StartGGElo")]
#[command(author = "Kiana Sheibani <kiana.a.sheibani@gmail.com>")]
#[command(version = "0.1.0")]
#[command(about = "StartGGElo - Elo rating calculator for start.gg tournaments", long_about = None)]
struct Cli {
    #[command(subcommand)]
    subcommand: Subcommands,

    #[arg(short = 'A', long = "auth", value_name = "TOKEN", global = true)]
    auth_token: Option<String>,

    #[arg(short, long = "config", value_name = "DIR", global = true)]
    config_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Subcommands {
    Dataset {
        #[command(subcommand)]
        subcommand: DatasetSC,
    },
    Sync {
        #[arg(group = "datasets")]
        names: Vec<String>,
        #[arg(short, long, group = "datasets")]
        all: bool,
    },
}

#[derive(Subcommand)]
enum DatasetSC {
    List,
    New { name: Option<String> },
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
        } => dataset_new(name),
        Subcommands::Dataset {
            subcommand: DatasetSC::Delete { name },
        } => dataset_delete(name),

        Subcommands::Sync { names, all } => sync(names, all, cli.auth_token),
    }
}

fn dataset_list() {
    let config_dir = dirs::config_dir().unwrap();

    let connection = open_datasets(&config_dir).unwrap();
    let datasets = list_datasets(&connection).unwrap();

    println!("{:?}", datasets);
}

fn dataset_new(name: Option<String>) {
    let config_dir = dirs::config_dir().unwrap();

    let name = name.unwrap_or_else(|| {
        let mut line = String::new();
        print!("Name of new dataset: ");
        io::stdout().flush().expect("Could not access stdout");
        io::stdin()
            .read_line(&mut line)
            .expect("Could not read from stdin");
        line.trim().to_owned()
    });

    let connection = open_datasets(&config_dir).unwrap();
    new_dataset(&connection, &name).unwrap();
}

fn dataset_delete(name: Option<String>) {
    let config_dir = dirs::config_dir().unwrap();

    let name = name.unwrap_or_else(|| {
        let mut line = String::new();
        print!("Dataset to delete: ");
        io::stdout().flush().expect("Could not access stdout");
        io::stdin()
            .read_line(&mut line)
            .expect("Could not read from stdin");
        line.trim().to_owned()
    });

    let connection = open_datasets(&config_dir).unwrap();
    delete_dataset(&connection, &name).unwrap();
}

fn sync(names: Vec<String>, all: bool, auth_token: Option<String>) {
    let config_dir = dirs::config_dir().unwrap();

    let auth = auth_token.or_else(|| get_auth_token(&config_dir)).unwrap();

    let connection = open_datasets(&config_dir).unwrap();

    let names = if all {
        list_datasets(&connection).unwrap()
    } else if names.len() == 0 {
        new_dataset(&connection, "default").unwrap();
        vec![String::from("default")]
    } else {
        names
    };

    for name in names {
        let last_sync = get_last_sync(&connection, &name).unwrap().unwrap();

        let results = run_query::<TournamentSets, _>(
            TournamentSetsVars {
                last_query: Timestamp(last_sync),
                game_id: VideogameId(1),
                tournament: 1,
                set_page: 1,
                set_pagesize: 50,
                event_limit: 9999999,
            },
            &auth,
        )
        .unwrap();

        update_from_tournament(&connection, &name, results).unwrap();

        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        update_last_sync(&connection, &name, current_time).unwrap();
    }
}
