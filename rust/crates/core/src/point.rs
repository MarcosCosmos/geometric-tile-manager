use crate::context::QueryContext;
use crate::error::{KeyedError, KeyedSnafu};
use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{EAST, NORTH, SOUTH, WEST};
use crate::graph::{Graph, NodeId};
use crate::key::NodeKey;
use crate::key::QueryKey;
use crate::neighbours::{NeighbourTiles, NeighbourhoodInformation};
use crate::node::{NodeNotFound, NodeQuery};
use crate::query::{ElementInformation, ElementKind, Query};
use markers_derive::NodeKey;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize, NodeKey)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl QueryContext<Point> for Graph {
    type OutKey = NodeId;
    type Error = KeyedError<Point, NodeNotFound>;

    fn try_query(&self, key: Point) -> Result<Query<Self::OutKey>, Self::Error> {
        self.bounds
            .iter()
            .filter(|&(_, &bounds)| {
                bounds[Horizontal][Negative] <= key.x
                    && bounds[Horizontal][Positive] >= key.x
                    && bounds[Vertical][Negative] <= key.y
                    && bounds[Vertical][Positive] >= key.y
            })
            .map(|(&id, _)| Query::new(self, id))
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
            .ok_or(NodeNotFound)
            .with_context(|_| KeyedSnafu { key })
    }
}
