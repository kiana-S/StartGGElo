#![feature(iterator_try_collect)]

use std::io::{self, Write};

mod queries;
use queries::*;
mod state;
use state::*;
mod datasets;
use datasets::*;

fn main() {
    let mut config_dir = dirs::config_dir().unwrap();
    config_dir.push("ggelo");

    let auth_token = get_auth_key(&config_dir).unwrap();

    // let config = AppState {
    //     config_dir,
    //     auth_token,
    // };

    let path = dataset_path(&config_dir, "test").unwrap();
    let connection = open_dataset(&path).unwrap();

    let set_data = SetData {
        teams: vec![
            vec![PlayerData {
                id: PlayerId(1),
                name: Some("player1".to_owned()),
                prefix: None,
            }],
            vec![PlayerData {
                id: PlayerId(2),
                name: Some("player2".to_owned()),
                prefix: None,
            }],
        ],
        winner: 0,
    };

    update_from_set(&connection, set_data.clone()).unwrap();
    println!("{:?}", get_ratings(&connection, &set_data.teams).unwrap());
}
