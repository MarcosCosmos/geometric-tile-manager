use crate::key::QueryKey;
use crate::node::NodeNotFound;
use snafu::Snafu;
pub(crate) use std::error::Error;

pub trait ValidationError: Error {}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[snafu(display("Invalid key {key:?}"))]
pub struct KeyedError<Key, E>
where
    Key: QueryKey,
    E: Error + 'static,
{
    key: Key,
    source: E,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum QueryError<Key, T, V>
where
    Key: QueryKey,
    T: Error + 'static,
    V: ValidationError + 'static,
{
    #[snafu(transparent)]
    Transitive { source: KeyedError<Key, T> },
    #[snafu(transparent)]
    Invalid { source: V },
}
