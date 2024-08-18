use sqlite::*;
use std::io::{self, Write};
use std::process::exit;

use crate::database::*;
use crate::queries::{PlayerData, PlayerId, Timestamp};

pub const SECS_IN_HR: u64 = 3600;
pub const SECS_IN_DAY: u64 = SECS_IN_HR * 24;
pub const SECS_IN_WEEK: u64 = SECS_IN_DAY * 7;
pub const SECS_IN_YEAR: u64 = SECS_IN_DAY * 365 + SECS_IN_HR * 6;

pub fn error(msg: &str, code: i32) -> ! {
    eprintln!("\nERROR: {}", msg);
    exit(code)
}

pub fn issue(msg: &str, code: i32) -> ! {
    eprintln!("\n{}", msg);
    exit(code)
}

pub fn current_time() -> Timestamp {
    use std::time::SystemTime;

    Timestamp(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_else(|_| error("System time is before the Unix epoch (1970)!", 2))
            .as_secs(),
    )
}

pub fn read_string() -> String {
    let mut line = String::new();
    io::stdout()
        .flush()
        .unwrap_or_else(|_| error("Could not access stdout", 2));
    io::stdin()
        .read_line(&mut line)
        .unwrap_or_else(|_| error("Could not read from stdin", 2));
    line.trim().to_owned()
}

pub fn ansi_num_color(num: f64, threshold1: f64, threshold2: f64) -> (&'static str, &'static str) {
    let sign = num > 0.0;
    let num_abs = num.abs();
    let severity = if num_abs < threshold1 {
        0
    } else if num_abs < threshold2 {
        1
    } else {
        2
    };

    match (sign, severity) {
        (false, 1) => ("31", "32"),
        (true, 1) => ("32", "31"),
        (false, 2) => ("91", "92"),
        (true, 2) => ("92", "91"),
        _ => ("39", "39"),
    }
}

// Player Input

pub enum PlayerInput {
    Id(PlayerId),
    Discrim(String),
    Name(String),
}

pub fn parse_player_input(input: String) -> PlayerInput {
    if let Ok(id) = input.parse::<u64>() {
        PlayerInput::Id(PlayerId(id))
    } else if input.chars().all(|c| c.is_ascii_hexdigit()) {
        PlayerInput::Discrim(input)
    } else {
        PlayerInput::Name(input)
    }
}

pub fn get_player_from_input(connection: &Connection, input: String) -> sqlite::Result<PlayerData> {
    match parse_player_input(input) {
        PlayerInput::Id(id) => get_player(connection, id),
        PlayerInput::Discrim(discrim) => get_player_from_discrim(connection, &discrim),
        PlayerInput::Name(name) => {
            let players = match_player_name(connection, &name)?;

            if players.is_empty() {
                error(&format!("Player {:?} not found", name), 1);
            } else if players.len() == 1 {
                Ok(players.into_iter().next().unwrap())
            } else {
                println!("\nInput {:?} matches more than one player:\n", name);

                for (i, player) in players.iter().enumerate() {
                    print!("{} - ", i);
                    if let Some(pre) = player.prefix.clone() {
                        print!("\x1b[2m{}\x1b[22m ", pre);
                    }
                    println!(
                        "\x1b[4m\x1b]8;;https://www.start.gg/user/{1}\x1b\\\
\x1b[1m{0}\x1b[22m\x1b]8;;\x1b\\\x1b[0m ({1})",
                        player.name, player.discrim
                    )
                }

                print!("\nPlayer (0-{}): ", players.len() - 1);
                let index = read_string()
                    .parse::<usize>()
                    .unwrap_or_else(|_| error("Not an integer", 1));
                if index >= players.len() {
                    error("Out of range!", 1);
                }

                Ok(players[index].clone())
            }
        }
    }
}
