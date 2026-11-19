use serde::Serialize;
use crate::graph::{Graph, NodeId};
use crate::query::{GraphQuery, Query, QueryKey};

pub(crate) trait RawQuerySource {
    fn query<Key>(&self, key: Key) -> Query<Key>
    where
        Key: QueryKey<NodeKey=NodeId>,
        for <'a> Query<'a, Key>: GraphQuery;
}

impl RawQuerySource for Graph {
    fn query<Key>(&self, key: Key) -> Query<Key>
    where
        Key: QueryKey<NodeKey=NodeId>
    {
        Query { context: self, key }
    }
}

pub trait QuerySource<Node> {
    type OutKey: Sized;
    fn try_query(&self, key: Node) -> anyhow::Result<Query<Self::OutKey>>
    where
        <Self as QuerySource<Node>>::OutKey: QueryKey;
}
