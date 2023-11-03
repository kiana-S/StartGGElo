use schema::schema;

// Types

// HACK: Unfortunately, start.gg seems to use integers for its ID type, whereas
// cynic always assumes that IDs are strings. To get around that, we define new
// scalar types that deserialize to u64.

#[derive(cynic::Scalar, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cynic(graphql_type = "ID")]
#[repr(transparent)]
pub struct VideogameId(pub u64);

#[derive(cynic::Scalar, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cynic(graphql_type = "ID")]
#[repr(transparent)]
pub struct EventId(pub u64);

#[derive(cynic::Scalar, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cynic(graphql_type = "ID")]
#[repr(transparent)]
pub struct EntrantId(pub u64);

#[derive(cynic::Scalar, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cynic(graphql_type = "ID")]
#[repr(transparent)]
pub struct PlayerId(pub u64);

#[derive(cynic::Scalar, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cynic(graphql_type = "ID")]
#[repr(transparent)]
pub struct SetId(pub u64);

#[derive(cynic::Scalar, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Timestamp(pub u64);
