use crate::error::Error;
use crate::error::Error::NotASingleNode;
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{Cardinal, LineBounds, Sign};
use crate::graph::NodeId;
use crate::query::line::{Line, LineLookup};
use crate::query::neighbours::{NeighbourSurfaces, NeighbourTiles, NeighbourhoodInformation};
use crate::query::node::NodeLookup;
use crate::query::source::RawQuerySource;
use crate::query::{
    ElementInformation, ElementInformation1D, ElementInformationMut, ElementKind,
    ElementNavigation, NodeKey, Query, QueryKey,
};
use std::fmt::Debug;
use std::hash::Hash;
use std::iter::Map;
use std::ops::Deref;

/// A caret is a position on the side of a node.
/// they provide the step function for navigating along the gutter of carets parallel to them.
#[derive(Copy, Clone, Debug, Eq, PartialEq, QueryKey)]
#[query(N)]
pub struct Caret<N = NodeId>
where
    N: NodeKey,
{
    pub id: N,
    pub side: Cardinal,
}

impl<N> Caret<N>
where
    N: NodeKey,
{
    pub fn new(id: N, side: Cardinal) -> Self {
        Self { id, side }
    }
}

impl TryFrom<Line> for Caret {
    type Error = Error<Line>;

    fn try_from(value: Line) -> Result<Self, Self::Error> {
        if value[Negative] == value[Positive] {
            Ok(Caret::new(value[Negative], value.side))
        } else {
            Err(Error::NotFullyConnected(value))
        }
    }
}

pub trait CaretLookup: ElementInformation1D + Deref<Target = Caret> {
    type NodeInformation: NodeLookup;
    fn with_node(self, id: NodeId) -> Self;

    fn with_side(self, side: Cardinal) -> Self;
}

pub trait CaretInformationMut: ElementInformationMut + Deref<Target = Caret> {
    fn set_node(&mut self, node_id: NodeId);

    fn set_side(&mut self, side: Cardinal);
}

impl ElementInformation for Query<'_, Caret> {
    fn kind(&self) -> ElementKind {
        self.context.query(self.id).kind()
    }
}

impl ElementInformation1D for Query<'_, Caret> {
    fn bounds(&self) -> LineBounds {
        let bounds = self.context.bounds[&self.id];
        LineBounds {
            side: self.side,
            side_location: bounds[self.side.axis][self.side.sign],
            axis_bounds: bounds[!self.side.axis],
        }
    }
}

impl<'a> CaretLookup for Query<'a, Caret> {
    type NodeInformation = Query<'a, NodeId>;
    fn with_node(mut self, id: NodeId) -> Self
    where
        Self: Sized,
    {
        self.key.id = id;
        self
    }

    fn with_side(mut self, side: Cardinal) -> Self
    where
        Self: Sized,
    {
        self.key.side = side;
        self
    }
}

impl ElementNavigation for Query<'_, Caret> {
    type Direction = Sign;

    fn peek(&self, direction: Sign) -> Option<Caret> {
        let location = self.bounds().side_location;
        let search_direction = Cardinal {
            axis: !self.side.axis,
            sign: direction,
        };
        self.context
            .query(NeighbourTiles(*self.with_side(search_direction)))
            .items()
            .iter()
            .map(|&id| self.context.query(Caret::new(id, self.side)))
            .take_while(|c| c.kind() == ElementKind::Tile)
            .find(|&c| c.bounds().side_location == location)
            .map(|c| *c)
    }
}

impl<'a> TryFrom<Query<'a, Line>> for Query<'a, Caret> {
    type Error = Error<Line>;

    fn try_from(value: Query<'a, Line>) -> Result<Self, Self::Error> {
        if value[Negative] == value[Positive] {
            Ok(value.into_end(Negative))
        } else {
            Err(NotASingleNode(*value))
        }
    }
}

pub trait CaretIterExt
where
    Self: Iterator<Item: Deref<Target = Caret>>,
{
    fn ids(self) -> Map<Self, fn(Self::Item) -> NodeId>
    where
        Self: Sized,
    {
        self.map(|c| c.id)
    }
}

impl<T: ?Sized> CaretIterExt for T where T: Iterator<Item: Deref<Target = Caret>> {}

impl<'a> From<Query<'a, NeighbourTiles>> for Query<'a, Caret> {
    fn from(value: Query<'a, NeighbourTiles>) -> Self {
        value.context.query(**value)
    }
}

impl<'a> From<Query<'a, NeighbourSurfaces>> for Query<'a, Caret> {
    fn from(value: Query<'a, NeighbourSurfaces>) -> Self {
        value.context.query(**value)
    }
}
