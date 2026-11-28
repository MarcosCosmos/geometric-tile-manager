use crate::area::{Area, AreaQuery};
use crate::caret::{Caret, CaretQuery};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{AreaBounds, Cardinal, NORTH_EAST, NORTH_WEST, SOUTH_EAST, SOUTH_WEST, WEST};
use crate::graph::NodeId;
use crate::key::QueryKey;
use crate::line::{Line, LineQuery};
use crate::neighbours::NeighbourTiles;
use crate::query::ElementKind::Surface;
use crate::query::{ElementInformation, ElementInformation2D, ElementKind, GraphQuery, Query};
use crate::query_impl;
use snafu::{Snafu, ensure};
use std::fmt::Debug;
use std::ops::Deref;

impl From<Caret> for NodeId {
    fn from(value: Caret) -> Self {
        value.id
    }
}
impl TryFrom<Line> for NodeId {
    type Error = TooManyNodesError<Line>;

    fn try_from(value: Line) -> Result<Self, Self::Error> {
        ensure!(
            value[Negative] == value[Positive],
            TooManyNodesSnafu { key: value }
        );
        Ok(value[Negative])
    }
}

impl TryFrom<Area> for NodeId {
    type Error = TooManyNodesError<Area>;

    fn try_from(value: Area) -> Result<Self, Self::Error> {
        ensure!(
            value[NORTH_WEST] == value[NORTH_EAST]
                && value[NORTH_WEST] == value[SOUTH_EAST]
                && value[NORTH_WEST] == value[SOUTH_WEST],
            TooManyNodesSnafu { key: value }
        );
        Ok(value[NORTH_WEST])
    }
}

query_impl! {From<Caret> | TryFrom<Line | Area> for NodeId}

pub trait NodeQuery: GraphQuery + ElementInformation2D + Deref<Target = NodeId> {
    fn to_side(self, side: Cardinal) -> Self::Query<Caret> {
        self.to(Caret::new(*self, side))
    }
    fn to_surface(self) -> Self;

    /// If and only if this is a non-empty surface return the area,
    /// made up of tile corners, that encloses all of the interior tiles  
    fn to_interior(self) -> Option<Self::Query<Area>>;
}

impl ElementInformation for Query<'_, NodeId> {
    fn kind(&self) -> ElementKind {
        if self.context.containing_surfaces.contains_key(self) {
            ElementKind::Tile
        } else {
            ElementKind::Surface
        }
    }
}

impl ElementInformation2D for Query<'_, NodeId> {
    fn bounds(&self) -> AreaBounds {
        self.context.bounds[self]
    }
}

impl<'a> NodeQuery for Query<'a, NodeId> {
    fn to_surface(self) -> Query<'a, NodeId> {
        self.to(self
            .context
            .containing_surfaces
            .get(&*self)
            .copied()
            .unwrap_or(*self))
    }

    fn to_interior(self) -> Option<Self::Query<Area>> {
        if self.kind() == Surface {
            let mut neighbours = self.to(NeighbourTiles(*self.to_side(WEST))).into_iter();
            let neg = neighbours.next()?;
            let pos = neighbours.next_back().unwrap_or(neg);
            let mut neighbours = self.to(NeighbourTiles(*self.to_side(WEST))).into_iter();
            let neg = neighbours.next()?;
            let pos = neighbours.next_back().unwrap_or(neg);
            self.to(Line::new(*neg, *pos, WEST))
                .to_area(|iter| iter.last())
        } else {
            None
        }
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(in super)))]
#[snafu(display(
    "Ambiguous attempt to convert a key that spans multiple nodes into a single node: {key:?}"
))]
pub struct TooManyNodesError<K>
where
    K: QueryKey,
{
    key: K,
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[snafu(display("No matching node could be found"))]
pub struct NodeNotFound;
