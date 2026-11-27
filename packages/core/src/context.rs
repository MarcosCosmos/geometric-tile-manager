use crate::error::KeyedError;
use crate::graph::Graph;
use crate::key::QueryKey;
use crate::query::Query;
use std::error::Error;

pub trait QueryContext<K>
where
    K: QueryKey
{
    type OutKey: QueryKey;
    type ErrorDetail: Error + 'static;

    /// A safe method for creating a Query: unlike direct instantiation this method validates that the key is correct and resolves it into some potentially more primitive/canonical Query.
    /// In general, this method should be preferred for all user inputs, and direct instantiation should only be used with data that has already been through validation.
    /// None of the core crate's methods are guaranteed to panic otherwise fail gracefully if called from a manually constructed Query.
    /// Note: for now this specifically allows arbitrary contexts to resolve into an inner query contextualised by the actual graph.
    /// That's because this trait is mostly for creating alias keys that
    /// Examples of how this can be used include:
    /// - e.g. assigning tags/names to node ids (without storing that in the core graph)
    /// - grid snapping, by creating a command/key that resolves to look internally for a nearby line and outputs that instead.
    fn resolve(&self, key: K) -> Result<Query<Self::OutKey>, KeyedError<K, Self::ErrorDetail>>;
}