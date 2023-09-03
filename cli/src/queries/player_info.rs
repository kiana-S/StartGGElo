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

pub struct PlayerInfoResponse {
    pub name: String,
    pub prefix: Option<String>,
}

impl QueryUnwrap<PlayerInfoVars> for PlayerInfo {
    type Unwrapped = PlayerInfoResponse;

    fn unwrap_response(response: GraphQlResponse<PlayerInfo>) -> Option<PlayerInfoResponse> {
        let player = response.data?.player?;
        Some(PlayerInfoResponse {
            name: player.gamer_tag?,
            prefix: player.prefix.filter(|pr| !pr.is_empty()),
        })
    }
}
