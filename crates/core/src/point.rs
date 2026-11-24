use crate::context::QueryContext;
use crate::error::{KeyedError, KeyedSnafu};
use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{EAST, NORTH, SOUTH, WEST};
use crate::graph::{Graph, NodeId};
use crate::key::QueryKey;
use crate::neighbours::NeighbourTiles;
use crate::node::{NodeNotFound, NodeQuery};
use crate::query::{ElementInformation, ElementKind, Query};
use markers_derive::QueryKey;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize, QueryKey)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl QueryContext<Point> for Graph {
    type OutKey = NodeId;
    type Error = KeyedError<Point, NodeNotFound>;

    fn try_query(&self, key: Point) -> Result<Query<'_, Self::OutKey>, Self::Error> {
        self.bounds
            .iter()
            .filter(|&(_, &bounds)| {
                bounds[Horizontal][Negative] <= key.x
                    && bounds[Horizontal][Positive] >= key.x
                    && bounds[Vertical][Negative] <= key.y
                    && bounds[Vertical][Positive] >= key.y
            })
            .map(|(&id, _)| Query::new(self, id))
            .find(|node| {
                node.kind() == ElementKind::Tile
                    || [NORTH, SOUTH, EAST, WEST].into_iter().all(|side| {
                        Query::<NeighbourTiles>::from(node.into_side(side))
                            .into_iter()
                            .next()
                            .is_none()
                    })
            })
            .ok_or(NodeNotFound)
            .with_context(|_| KeyedSnafu { key })
    }
}
