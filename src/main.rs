#![feature(iterator_try_collect)]

use clap::{Parser, Subcommand};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

mod queries;
use queries::*;
mod state;
use state::*;
mod datasets;
use datasets::*;

/// ## CLI Structs

#[derive(Parser)]
#[command(name = "StartGGElo")]
#[command(author = "Kiana Sheibani <kiana.a.sheibani@gmail.com>")]
#[command(version = "0.1.0")]
#[command(about = "Elo rating calculator for start.gg tournaments", long_about = None)]
struct Cli {
    #[command(subcommand)]
    subcommand: Subcommands,
}

#[derive(Subcommand)]
enum Subcommands {
    Dataset {
        #[command(subcommand)]
        subcommand: DatasetSC,
    },
}

#[derive(Subcommand)]
enum DatasetSC {
    List,
    New { name: Option<String> },
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
    }
}

fn dataset_list() {
    let config_dir = dirs::config_dir().unwrap();

    let connection = open_datasets(&config_dir, None).unwrap();
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

    let connection = open_datasets(&config_dir, None).unwrap();
    new_dataset(&connection, &name).unwrap();
}
