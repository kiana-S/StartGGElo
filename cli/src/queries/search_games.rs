
use schema::schema;

// VARIABLES

#[derive(cynic::QueryVariables)]
pub struct VideogameSearchVars {
    name: String
}

// QUERY

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
    pub id: Option<cynic::Id>,
    pub name: Option<String>,
}
