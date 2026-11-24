use crate::area::{Area, AreaQuery};
use crate::caret::{Caret, CaretQuery};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{AreaBounds, Cardinal, NORTH_EAST, NORTH_WEST, SOUTH_EAST, SOUTH_WEST};
use crate::graph::NodeId;
use crate::key::QueryKey;
use crate::line::Line;
use crate::query::{ElementInformation, ElementInformation2D, ElementKind, Query};
use crate::query_impl;
use snafu::{Snafu, ensure};
use std::fmt::Debug;

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

pub trait NodeQuery: ElementInformation2D {
    type CaretInformation: CaretQuery;
    type AreaInformation: AreaQuery;
    fn into_side(self, side: Cardinal) -> Self::CaretInformation;
    fn into_containing_surface(self) -> Self;
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
    type CaretInformation = Query<'a, Caret>;
    type AreaInformation = Query<'a, Area>;

    fn into_side(self, side: Cardinal) -> Query<'a, Caret> {
        Query::new(self.context, Caret::new(*self, side))
    }

    fn into_containing_surface(self) -> Query<'a, NodeId> {
        Query::new(
            self.context,
            self.context
                .containing_surfaces
                .get(&*self)
                .copied()
                .unwrap_or(*self),
        )
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
