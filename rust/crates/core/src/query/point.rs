use crate::query::{ElementInformation, ElementKind, Query, QueryKey};
use crate::query::key::NodeKey;
use serde::{Deserialize, Serialize};
use markers_derive::NodeKey;
use crate::error::Error;
use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::{EAST, NORTH, SOUTH, WEST};
use crate::geometry::Sign::{Negative, Positive};
use crate::graph::{Graph, NodeId};
use crate::query::neighbours::{NeighbourTiles, NeighbourhoodInformation};
use crate::query::node::NodeQuery;
use crate::query::source::{QuerySource, RawQuerySource};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize, NodeKey)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl QuerySource<Point> for Graph {
    type OutKey = NodeId;

    fn try_query(&self, key: Point) -> anyhow::Result<Query<Self::OutKey>> {
        self.bounds
            .iter()
            .filter(|&(_, &bounds)| {
                bounds[Horizontal][Negative] <= key.x
                    && bounds[Horizontal][Positive] >= key.x
                    && bounds[Vertical][Negative] <= key.y
                    && bounds[Vertical][Positive] >= key.y
            })
            .map(|(&id, _)| self.query(id))
            .filter(|node| {
                node.kind() == ElementKind::Tile
                    || [NORTH, SOUTH, EAST, WEST].into_iter().all(|side| {
                    Query::<NeighbourTiles>::from(node.into_side(side))
                        .items()
                        .len()
                        == 0
                })
            })
            .next()
            .ok_or(Error::NodeNotFound(key).into())
    }
}