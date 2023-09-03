use super::{PlayerId, QueryUnwrap};
use cynic::GraphQlResponse;
use schema::schema;

// Variables

#[derive(cynic::QueryVariables, Debug)]
pub struct PlayerInfoVars {
    pub id: PlayerId,
}

// Query

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "PlayerInfoVars")]
pub struct PlayerInfo {
    #[arguments(id: $id)]
    player: Option<Player>,
}

#[derive(cynic::QueryFragment, Debug)]
struct Player {
    gamer_tag: Option<String>,
    prefix: Option<String>,
}

// Unwrapping

pub struct PlayerData {
    pub name: Option<String>,
    pub prefix: Option<String>,
}

impl QueryUnwrap<PlayerInfoVars> for PlayerInfo {
    type Unwrapped = PlayerData;

    fn unwrap_response(response: GraphQlResponse<PlayerInfo>) -> Option<PlayerData> {
        let player = response.data?.player?;
        Some(PlayerData {
            name: player.gamer_tag,
            prefix: player.prefix.filter(|pr| !pr.is_empty()),
        })
    }
}
