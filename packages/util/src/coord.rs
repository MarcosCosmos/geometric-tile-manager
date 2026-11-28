use geotm_core::context::QueryContext;
use geotm_core::error::{KeyedError, KeyedSnafu};
use geotm_core::geometry::Axis;
use geotm_core::geometry::Axis::{Horizontal, Vertical};
use geotm_core::graph::{Graph, NodeId};
use geotm_core::key::NodeKey;
use geotm_core::key::QueryKey;
use geotm_core::node::{NodeNotFound, NodeQuery};
use geotm_core::query::{ElementInformation, ElementInformation2D, ElementKind, Query};
use geotm_macros::{NodeKey, QueryKey};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::ops::{Deref, Index, IndexMut};

#[derive(
    Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize, QueryKey, NodeKey,
)]
pub struct Coord {
    pub x: i32,
    pub y: i32,
}

impl Display for Coord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{},{}>", self.x, self.y)
    }
}

impl Index<Axis> for Coord {
    type Output = i32;

    fn index(&self, index: Axis) -> &Self::Output {
        match index {
            Horizontal => &self.x,
            Vertical => &self.y,
        }
    }
}

impl IndexMut<Axis> for Coord {
    fn index_mut(&mut self, index: Axis) -> &mut Self::Output {
        match index {
            Horizontal => &mut self.x,
            Vertical => &mut self.y,
        }
    }
}

impl QueryContext<Coord> for Graph {
    type OutKey = NodeId;
    type ErrorDetail = NodeNotFound;

    /// This method finds the node nearest to the given point, if any nodes (at all) exist.
    /// For points inside surfaces, it will prefer any tiles inside the surface over the surface itself,
    /// but it otherwise targets the nearest node in general.
    fn resolve(
        &self,
        key: Coord,
    ) -> Result<Query<Self::OutKey>, KeyedError<Coord, Self::ErrorDetail>> {
        self.iter()
            .map(|node| {
                let bounds = node.bounds();
                let distance = i32::min(
                    bounds[Horizontal].distance_to(key[Horizontal]),
                    bounds[Horizontal].distance_to(key[Horizontal]),
                );
                (node, distance, node.kind())
            })
            .min_by(|(a, dist_a, kind_a), (b, dist_b, kind_b)| {
                match ((dist_a, kind_a), (dist_b, kind_b)) {
                    ((0, ElementKind::Surface), (_, ElementKind::Tile))
                        if *b.to_surface().deref() == *a.deref() =>
                    {
                        Ordering::Greater
                    }
                    ((_, ElementKind::Tile), (0, ElementKind::Surface))
                        if *a.to_surface().deref() == *b.deref() =>
                    {
                        Ordering::Less
                    }
                    _ => dist_a.cmp(dist_b),
                }
            })
            .map(|(node, ..)| node)
            .ok_or(NodeNotFound)
            .with_context(|_| KeyedSnafu { key })
    }
}
