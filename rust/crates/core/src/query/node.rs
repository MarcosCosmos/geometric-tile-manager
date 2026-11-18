use crate::error::Error;
use crate::error::Error::NotASingleNode;
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{AreaBounds, Cardinal};
use crate::graph::NodeId;
use crate::query::area::{Area, AreaInformation};
use crate::query::caret::{Caret, CaretLookup};
use crate::query::line::Line;
use crate::query::source::RawQuerySource;
use crate::query::{ElementInformation, ElementInformation2D, ElementKind, Query};

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

pub trait NodeLookup: ElementInformation2D {
    type CaretInformation: CaretLookup;
    type AreaInformation: AreaInformation;
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

impl<'a> NodeLookup for Query<'a, NodeId> {
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

impl<'a> From<Query<'a, Caret>> for Query<'a, NodeId> {
    fn from(value: Query<'a, Caret>) -> Self {
        value.context.query(value.id)
    }
}

impl<'a> TryFrom<Query<'a, Line>> for Query<'a, NodeId> {
    type Error = Error<Line>;

    fn try_from(value: Query<'a, Line>) -> Result<Self, Self::Error> {
        (*value).try_into().map(|id| value.context.query(id))
    }
}

impl<'a> TryFrom<Query<'a, Area>> for Query<'a, NodeId> {
    type Error = Error<Area>;

    fn try_from(value: Query<'a, Area>) -> Result<Self, Self::Error> {
        (*value).try_into().map(|id| value.context.query(id))
    }
}
