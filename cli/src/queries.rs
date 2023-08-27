use cynic::QueryBuilder;
use serde::{Serialize, Deserialize};

pub mod search_games;

use schema::schema;

/// HACK: Unfortunately, start.gg seems to use integers for its ID type, whereas
/// cynic always assumes that IDs are strings. To get around that, we define a
/// new scalar type that serializes to u64.
#[derive(cynic::Scalar, Debug)]
pub struct ID(u64);


pub async fn run_query<Builder: 'static, Vars>(vars: Vars, auth: &str) -> cynic::GraphQlResponse<Builder>
    where Builder: QueryBuilder<Vars>,
          Vars: Serialize,
          for<'de> Builder: Deserialize<'de>

{
    use cynic::http::SurfExt;

    let query = Builder::build(vars);

    let response = surf::post("https://api.start.gg/gql/alpha")
        .header("Authorization", String::from("Bearer ") + auth)
        .run_graphql(query)
        .await;

    response.unwrap()
}
