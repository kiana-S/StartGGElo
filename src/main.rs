#![feature(iterator_try_collect)]

use clap::{Parser, Subcommand};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::exit;
use std::time::SystemTime;

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
    let config_dir = dirs::config_dir().expect("Could not determine config directory");

    let connection =
        open_datasets(&config_dir).unwrap_or_else(|_| error("Could not open datasets file", 1));
    let datasets = list_datasets(&connection).expect("Error communicating with SQLite");

    println!("{:?}", datasets);
}

fn read_string() -> String {
    let mut line = String::new();
    io::stdout().flush().expect("Could not access stdout");
    io::stdin()
        .read_line(&mut line)
        .expect("Could not read from stdin");
    line.trim().to_owned()
}

fn dataset_new(name: Option<String>) {
    let config_dir = dirs::config_dir().expect("Could not determine config directory");

    let name = name.unwrap_or_else(|| {
        print!("Name of new dataset: ");
        read_string()
    });

    let connection =
        open_datasets(&config_dir).unwrap_or_else(|_| error("Could not open datasets file", 1));
    new_dataset(&connection, &name).expect("Error communicating with SQLite");
}

fn dataset_delete(name: Option<String>) {
    let config_dir = dirs::config_dir().expect("Could not determine config directory");

    let name = name.unwrap_or_else(|| {
        print!("Dataset to delete: ");
        read_string()
    });

    let connection =
        open_datasets(&config_dir).unwrap_or_else(|_| error("Could not open datasets file", 1));
    delete_dataset(&connection, &name).expect("Error communicating with SQLite");
}

fn sync(datasets: Vec<String>, all: bool, auth_token: Option<String>) {
    let config_dir = dirs::config_dir().unwrap();

    let auth = auth_token
        .or_else(|| get_auth_token(&config_dir))
        .unwrap_or_else(|| error("Access token not provided", 1));

    let connection =
        open_datasets(&config_dir).unwrap_or_else(|_| error("Could not open datasets file", 1));

    #[allow(unused_must_use)]
    let datasets = if all {
        list_datasets(&connection).unwrap()
    } else if datasets.len() == 0 {
        new_dataset(&connection, "default");
        vec![String::from("default")]
    } else {
        datasets
    };

    for dataset in datasets {
        let last_sync = get_last_sync(&connection, &dataset)
            .expect("Error communicating with SQLite")
            .unwrap_or_else(|| error(&format!("Dataset {} does not exist!", dataset), 1));

        sync_dataset(
            &connection,
            &dataset,
            last_sync,
            VideogameId(1386),
            Some("GA"),
            &auth,
        )
        .expect("Error communicating with SQLite");

        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_else(|_| error("System time is before the Unix epoch!", 2))
            .as_secs();

        update_last_sync(&connection, &dataset, current_time)
            .expect("Error communicating with SQLite");
    }
}
