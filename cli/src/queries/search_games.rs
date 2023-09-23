use super::{QueryUnwrap, VideogameId};
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
    #[cynic(flatten)]
    nodes: Vec<Videogame>,
}

#[derive(cynic::QueryFragment, Debug)]
struct Videogame {
    id: Option<VideogameId>,
    name: Option<String>,
}

// Unwrapping

#[derive(Debug, Clone)]
pub struct VideogameData {
    pub id: VideogameId,
    pub name: String,
}

impl<'a> QueryUnwrap<VideogameSearchVars<'a>> for VideogameSearch {
    type Unwrapped = Vec<VideogameData>;

    fn unwrap_response(response: GraphQlResponse<VideogameSearch>) -> Option<Vec<VideogameData>> {
        Some(
            response
                .data?
                .videogames?
                .nodes
                .into_iter()
                .filter_map(|game| {
                    Some(VideogameData {
                        id: game.id?,
                        name: game.name?,
                    })
                })
                .collect(),
        )
    }
}
