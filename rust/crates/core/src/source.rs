use crate::graph::{Graph, NodeId};
use crate::key::QueryKey;
use crate::query::{GraphQuery, Query};
use std::error::Error;

pub trait RawQuerySource {
    fn query<Key>(&self, key: Key) -> Query<Key>
    where
        Key: QueryKey<NodeKey = NodeId>,
        for<'a> Query<'a, Key>: GraphQuery;
}

impl RawQuerySource for Graph {
    fn query<Key>(&self, key: Key) -> Query<Key>
    where
        Key: QueryKey<NodeKey = NodeId>,
    {
        Query { context: self, key }
    }
}

pub trait QuerySource<Key>
where
    Key: QueryKey,
{
    type OutKey: QueryKey;
    type Error: Error;
    fn try_query(&self, key: Key) -> Result<Query<Self::OutKey>, Self::Error>;
}
