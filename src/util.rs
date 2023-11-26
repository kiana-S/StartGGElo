use std::io::{self, Write};
use std::process::exit;

pub const SECS_IN_HR: u64 = 3600;
pub const SECS_IN_DAY: u64 = SECS_IN_HR * 24;
pub const SECS_IN_WEEK: u64 = SECS_IN_DAY * 7;

pub fn error(msg: &str, code: i32) -> ! {
    println!("\nERROR: {}", msg);
    exit(code)
}

pub fn issue(msg: &str, code: i32) -> ! {
    println!("\n{}", msg);
    exit(code)
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
