use super::{QueryUnwrap, VideogameId, ID};
use cynic::GraphQlResponse;
use schema::schema;

// Variables

#[derive(cynic::QueryVariables)]
pub struct VideogameSearchVars<'a> {
    pub name: &'a str,
}

// Query

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "VideogameSearchVars")]
pub struct VideogameSearch {
    #[arguments(query: { filter: { name: $name }, page: 1, perPage: 10 })]
    videogames: Option<VideogameConnection>,
}

#[derive(cynic::QueryFragment, Debug)]
struct VideogameConnection {
    nodes: Option<Vec<Option<Videogame>>>,
}

#[derive(cynic::QueryFragment, Debug)]
struct Videogame {
    id: Option<ID>,
    name: Option<String>,
}

// Unwrapping

pub struct VideogameResponse {
    pub id: VideogameId,
    pub name: String,
}

impl<'a> QueryUnwrap<VideogameSearchVars<'a>> for VideogameSearch {
    type VarsUnwrapped = VideogameSearchVars<'a>;
    type Unwrapped = Vec<VideogameResponse>;

    fn wrap_vars(vars: VideogameSearchVars) -> VideogameSearchVars {
        vars
    }

    fn unwrap_response(
        response: GraphQlResponse<VideogameSearch>,
    ) -> Option<Vec<VideogameResponse>> {
        Some(
            response
                .data?
                .videogames?
                .nodes?
                .into_iter()
                .filter_map(|game| {
                    let game_ = game?;
                    Some(VideogameResponse {
                        id: VideogameId(game_.id?.0),
                        name: game_.name?,
                    })
                })
                .collect(),
        )
    }
}
