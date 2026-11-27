use crate::key::QueryKey;
use snafu::Snafu;
use std::error::Error;
use std::panic::panic_any;

pub trait ValidationError: Error {}

impl ValidationError for ! {}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[snafu(display("Invalid key {key:?}"))]
pub struct KeyedError<K, E>
where
    K: QueryKey,
    E: Error + 'static,
{
    key: K,
    source: E,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum QueryError<K, T, V>
where
    K: QueryKey,
    T: Error + 'static,
    V: ValidationError + 'static,
{
    #[snafu(transparent)]
    Transitive { source: KeyedError<K, T> },
    #[snafu(transparent)]
    Invalid { source: V },
}