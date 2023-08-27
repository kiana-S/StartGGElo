
pub mod search_games;

use schema::schema;

/// HACK: Unfortunately, start.gg seems to use integers for its ID type, whereas
/// cynic always assumes that IDs are strings. To get around that, we define a
/// new scalar type that serializes to u64.
#[derive(cynic::Scalar, Debug)]
pub struct ID(u64);
