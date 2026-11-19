use crate::error::Error;
use crate::error::Error::NotASingleNode;
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{AreaBounds, Cardinal};
use crate::graph::NodeId;
use crate::query::area::{Area, AreaQuery};
use crate::query::caret::{Caret, CaretQuery};
use crate::query::line::Line;
use crate::query::source::RawQuerySource;
use crate::query::{ElementInformation, ElementInformation2D, ElementKind, Query};
use crate::query_impl;

impl From<Caret> for NodeId {
    fn from(value: Caret) -> Self {
        value.id
    }
}
impl TryFrom<Line> for NodeId {
    type Error = Error<Line>;

    fn try_from(value: Line) -> Result<Self, Self::Error> {
        if value[Negative] == value[Positive] {
            Ok(value[Negative])
        } else {
            Err(NotASingleNode(value))
        }
    }
}

impl<'a> TryFrom<Area> for NodeId {
    type Error = Error<Area>;

    fn try_from(value: Area) -> Result<Self, Self::Error> {
        if value.north_west == value.north_east
            && value.north_west == value.south_east
            && value.north_west == value.south_west
        {
            Ok(value.north_west)
        } else {
            Err(Error::NotFullyConnected(value))
        }
    }
}

query_impl! {From<Caret> | TryFrom<Line | Area> for NodeId}

pub trait NodeQuery: ElementInformation2D {
    type CaretInformation: CaretQuery;
    type AreaInformation: AreaQuery;
    fn into_side(self, side: Cardinal) -> Self::CaretInformation;
    fn into_surface_of(self) -> Self;
}

impl ElementInformation for Query<'_, NodeId> {
    fn kind(&self) -> ElementKind {
        if self.context.containing_surface.contains_key(&self.key) {
            ElementKind::Tile
        } else {
            ElementKind::Surface
        }
    }
}

impl ElementInformation2D for Query<'_, NodeId> {
    fn bounds(&self) -> AreaBounds {
        self.context.bounds[&self.key]
    }
}

impl<'a> NodeQuery for Query<'a, NodeId> {
    type CaretInformation = Query<'a, Caret>;
    type AreaInformation = Query<'a, Area>;

    fn into_side(self, side: Cardinal) -> Query<'a, Caret> {
        self.context.query(Caret::new(self.key, side))
    }

    fn into_surface_of(self) -> Query<'a, NodeId> {
        self.context.query(
            self.context
                .containing_surface
                .get(&self.key)
                .copied()
                .unwrap_or(self.key),
        )
    }
}
