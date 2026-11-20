use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{EAST, NORTH, SOUTH, WEST};
use crate::graph::{Graph, NodeId};
use crate::query::key::NodeKey;
use crate::query::neighbours::{NeighbourTiles, NeighbourhoodInformation};
use crate::query::node::{NodeNotFound, NodeNotFoundSnafu, NodeQuery};
use crate::query::source::{QuerySource, RawQuerySource};
use crate::query::{ElementInformation, ElementKind, Query, QueryKey};
use markers_derive::NodeKey;
use serde::{Deserialize, Serialize};
use snafu::OptionExt;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize, NodeKey)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl QuerySource<Point> for Graph {
    type OutKey = NodeId;
    type Error = NodeNotFound<Point>;

    fn try_query(&self, key: Point) -> Result<Query<Self::OutKey>, Self::Error> {
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
            .with_context(|| NodeNotFoundSnafu { key })
    }
}
