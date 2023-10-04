use cynic::{GraphQlResponse, QueryBuilder};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

pub mod search_games;
pub use search_games::*;
pub mod tournament_events;
pub use tournament_events::*;
pub mod event_sets;
pub use event_sets::*;
pub mod player_info;
pub use player_info::*;

use crate::error;
use schema::schema;

// Auth key

pub fn get_auth_token(config_dir: &Path) -> Option<String> {
    use std::env::{var, VarError};
    use std::fs::read_to_string;

    match var("AUTH_TOKEN") {
        Ok(key) => Some(key),
        Err(VarError::NotUnicode(_)) => error("Invalid authorization key", 2),
        Err(VarError::NotPresent) => {
            let mut auth_file = config_dir.to_owned();
            auth_file.push("startrnr");
            auth_file.push("auth.txt");
            read_to_string(auth_file).ok().and_then(|s| {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_owned())
                }
            })
        }
    }
}

// Types

// HACK: Unfortunately, start.gg seems to use integers for its ID type, whereas
// cynic always assumes that IDs are strings. To get around that, we define new
// scalar types that deserialize to u64.

#[derive(cynic::Scalar, Debug, Copy, Clone)]
#[cynic(graphql_type = "ID")]
pub struct VideogameId(pub u64);

#[derive(cynic::Scalar, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cynic(graphql_type = "ID")]
pub struct EventId(pub u64);

#[derive(cynic::Scalar, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cynic(graphql_type = "ID")]
pub struct EntrantId(pub u64);

#[derive(cynic::Scalar, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cynic(graphql_type = "ID")]
pub struct PlayerId(pub u64);

#[derive(cynic::Scalar, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(pub u64);

// Query machinery

pub trait QueryUnwrap<Vars>: 'static + QueryBuilder<Vars> {
    type Unwrapped;

    fn unwrap_response(response: GraphQlResponse<Self>) -> Option<Self::Unwrapped>;
}

// Generic function for running start.gg queries
pub fn run_query<Builder, Vars>(vars: Vars, auth_token: &str) -> Option<Builder::Unwrapped>
where
    Vars: Copy + Serialize,
    Builder: QueryUnwrap<Vars>,
    for<'de> Builder: Deserialize<'de>,
{
    use cynic::http::ReqwestBlockingExt;

    let mut response = reqwest::blocking::Client::new()
        .post("https://api.start.gg/gql/alpha")
        .header("Authorization", String::from("Bearer ") + auth_token)
        .run_graphql(Builder::build(vars));

    // If query fails to reach server, retry up to 10 times
    for _ in 1..10 {
        if response.is_ok() {
            break;
        }
        sleep(Duration::from_secs(2));
        response = reqwest::blocking::Client::new()
            .post("https://api.start.gg/gql/alpha")
            .header("Authorization", String::from("Bearer ") + auth_token)
            .run_graphql(Builder::build(vars));
    }

    Builder::unwrap_response(response.ok()?)
}
