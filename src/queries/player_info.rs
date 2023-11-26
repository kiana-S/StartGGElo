use super::{PlayerId, QueryUnwrap};
use cynic::GraphQlResponse;
use schema::schema;

// Variables

#[derive(cynic::QueryVariables, Debug, Copy, Clone)]
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
    id: Option<PlayerId>,
    gamer_tag: Option<String>,
    prefix: Option<String>,
    user: Option<User>,
}

#[derive(cynic::QueryFragment, Debug)]
struct User {
    discriminator: Option<String>,
}

// Unwrapping

#[derive(Debug, Clone)]
pub struct PlayerData {
    pub id: PlayerId,
    pub name: String,
    pub prefix: Option<String>,
    pub discrim: String,
}

impl QueryUnwrap<PlayerInfoVars> for PlayerInfo {
    type Unwrapped = PlayerData;

    fn unwrap_response(response: GraphQlResponse<PlayerInfo>) -> Option<PlayerData> {
        let player = response.data?.player?;
        Some(PlayerData {
            id: player.id?,
            name: player.gamer_tag?,
            prefix: player.prefix.filter(|pr| !pr.is_empty()),
            discrim: player.user?.discriminator?,
        })
    }
}
