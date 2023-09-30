#![feature(iterator_try_collect)]

use clap::{Parser, Subcommand};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::SystemTime;

mod queries;
use queries::*;
mod datasets;
use datasets::*;
mod sync;
use sync::*;

/// ## CLI Structs

#[derive(Parser)]
#[command(name = "StartGGElo")]
#[command(author = "Kiana Sheibani <kiana.a.sheibani@gmail.com>")]
#[command(version = "0.1.0")]
#[command(about = "StartGGElo - Elo rating calculator for start.gg tournaments", long_about = None)]
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
        long_help = "This option overrides the default config directory.
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
        long_about = "Pull recent tournament data off of start.gg and use it to update each player's
stored ratings. This command will automatically keep track of the last time each
dataset was synced."
    )]
    Sync {
        #[arg(
            group = "datasets",
            help = "The datasets to sync",
            long_help = "A list of datasets to sync.
If no datasets are given, then the dataset 'default' is synced. This dataset is
created if it does not already exist."
        )]
        datasets: Vec<String>,
        #[arg(short, long, group = "datasets", help = "Sync all stored databases")]
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
        } => dataset_new(name),
        Subcommands::Dataset {
            subcommand: DatasetSC::Delete { name },
        } => dataset_delete(name),

        Subcommands::Sync { datasets, all } => sync(datasets, all, cli.auth_token),
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

fn sync(datasets: Vec<String>, all: bool, auth_token: Option<String>) {
    let config_dir = dirs::config_dir().unwrap();

    let auth = auth_token.or_else(|| get_auth_token(&config_dir)).unwrap();

    let connection = open_datasets(&config_dir).unwrap();

    let datasets = if all {
        list_datasets(&connection).unwrap()
    } else if datasets.len() == 0 {
        new_dataset(&connection, "default").unwrap();
        vec![String::from("default")]
    } else {
        datasets
    };

    for dataset in datasets {
        let last_sync = get_last_sync(&connection, &dataset).unwrap().unwrap();
    }
}
