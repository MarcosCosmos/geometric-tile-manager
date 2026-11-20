use crate::query::key::QueryKey;
use crate::query::node::NodeNotFound;
use snafu::Snafu;
pub(crate) use std::error::Error;

pub trait ValidationError: Error {}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[snafu(display("Invalid key {key:?}"))]
pub struct WrappedQueryError<Key, E>
where
    Key: QueryKey,
    E: Error + 'static,
{
    key: Key,
    source: E,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum QueryError<Key, V>
where
    Key: QueryKey + 'static,
    Key::NodeKey: 'static,
    V: ValidationError + 'static,
{
    #[snafu(transparent)]
    NotFound { source: NodeNotFound<Key::NodeKey> },
    #[snafu(transparent)]
    Invalid { source: V },
}
