use std::fmt::{Display, Formatter};

use cynic::{GraphQlResponse, QueryBuilder};
use serde::{Deserialize, Serialize};

pub mod search_games;

use schema::schema;

// HACK: Unfortunately, start.gg seems to use integers for its ID type, whereas
// cynic always assumes that IDs are strings. To get around that, we define a
// new scalar type that serializes to u64.
#[derive(cynic::Scalar, Debug, Copy, Clone)]
pub struct ID(pub u64);

impl Display for ID {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        <u64 as Display>::fmt(&self.0, f)
    }
}

pub trait QueryUnwrap<Vars>: QueryBuilder<Vars> {
    type Unwrapped;

    fn unwrap_response(response: GraphQlResponse<Self>) -> Option<Self::Unwrapped>;
}

// Generic function for running start.gg queries
pub async fn run_query<Builder: 'static, Vars>(vars: Vars, auth: &str) -> Option<Builder::Unwrapped>
where
    Builder: QueryUnwrap<Vars>,
    Vars: Serialize,
    for<'de> Builder: Deserialize<'de>,
{
    use cynic::http::SurfExt;

    let query = Builder::build(vars);

    let response = surf::post("https://api.start.gg/gql/alpha")
        .header("Authorization", String::from("Bearer ") + auth)
        .run_graphql(query)
        .await;

    <Builder as QueryUnwrap<Vars>>::unwrap_response(response.unwrap())
}
