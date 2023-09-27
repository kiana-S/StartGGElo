#![feature(iterator_try_collect)]

use clap::{Parser, Subcommand};
use std::io::{self, Write};

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
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    subcommand: Subcommands,
    #[arg(long)]
    auth_token: Option<String>,
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
}

fn main() {
    let cli = Cli::parse();

    let mut config_dir = dirs::config_dir().unwrap();
    config_dir.push("ggelo");

    let auth_token = get_auth_key(&config_dir).unwrap();

    let app_state = AppState {
        config_dir,
        auth_token,
    };

    match cli.subcommand {
        Subcommands::Dataset {
            subcommand: DatasetSC::List,
        } => dataset_list(app_state),
    }

    // let config = AppState {
    //     config_dir,
    //     auth_token,
    // };

    // let path = dataset_path(&config_dir, "test").unwrap();
    // let connection = open_dataset(&path).unwrap();

    // let set_data = SetData {
    //     teams: vec![
    //         vec![PlayerData {
    //             id: PlayerId(1),
    //             name: Some("player1".to_owned()),
    //             prefix: None,
    //         }],
    //         vec![PlayerData {
    //             id: PlayerId(2),
    //             name: Some("player2".to_owned()),
    //             prefix: None,
    //         }],
    //     ],
    //     winner: 0,
    // };

    // update_from_set(&connection, set_data.clone()).unwrap();
    // println!("{:?}", get_ratings(&connection, &set_data.teams).unwrap());
}

fn dataset_list(state: AppState) {}
