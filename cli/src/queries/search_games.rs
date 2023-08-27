use super::{QueryUnwrap, ID};
use cynic::GraphQlResponse;
use schema::schema;

// Query

#[derive(cynic::QueryVariables)]
pub struct VideogameSearchVars {
    pub name: String,
}

// Query

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "VideogameSearchVars")]
pub struct VideogameSearch {
    #[arguments(query: { filter: { name: $name }, page: 1, perPage: 10 })]
    pub videogames: Option<VideogameConnection>,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct VideogameConnection {
    pub nodes: Option<Vec<Option<Videogame>>>,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct Videogame {
    pub id: Option<ID>,
    pub name: Option<String>,
}

// Unwrapping

pub struct VideogameResponse {
    pub id: ID,
    pub name: String,
}

impl QueryUnwrap<VideogameSearchVars> for VideogameSearch {
    type Unwrapped = Vec<VideogameResponse>;

    fn unwrap_response(
        response: GraphQlResponse<VideogameSearch>,
    ) -> Option<Vec<VideogameResponse>> {
        Some(
            response
                .data?
                .videogames?
                .nodes?
                .into_iter()
                .map(|game| {
                    let game_ = game?;
                    Some(VideogameResponse {
                        id: game_.id?,
                        name: game_.name?,
                    })
                })
                .try_collect()?,
        )
    }
}
