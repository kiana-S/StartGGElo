use schema::schema;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Error, Formatter};

// HACK: Unfortunately, start.gg seems to use integers and strings
// interchangeably for its ID types (... for some reason), whereas cynic always
// assumes that IDs are strings. To get around that, we define new scalar types
// that deserialize properly.

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrInt {
    String(String),
    Int(u64),
}

impl Display for StringOrInt {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        match self {
            StringOrInt::String(x) => x.fmt(fmt),
            StringOrInt::Int(x) => x.fmt(fmt),
        }
    }
}

// Scalar Types

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

#[derive(cynic::Scalar, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cynic(graphql_type = "ID")]
#[repr(transparent)]
pub struct SetId(pub StringOrInt);

#[derive(cynic::Scalar, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Timestamp(pub u64);
