use crate::context::{QueryContext, RawQueryContext};
use crate::error::KeyedError;
use crate::geometry::{Cardinal, LineBounds, Sign};
use crate::graph::NodeId;
use crate::key::{NodeKey, QueryKey};
use crate::line::Line;
use crate::neighbours::{NeighbourTiles, NeighbourhoodInformation};
use crate::node::{NodeNotFound, NodeQuery, TooManyNodesError};
use crate::query::{
    ElementInformation, ElementInformation1D, ElementKind, Navigation, NavigationMut, Query,
};
use crate::query_impl;
use markers_derive::QueryKey;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::iter::Map;
use std::ops::Deref;

/// A caret is a position on the side of a node.
/// they provide the step function for navigating along the gutter of carets parallel to them.

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey)]
pub struct Caret<Node = NodeId>
where
    Node: NodeKey,
{
    pub id: Node,
    pub side: Cardinal,
}

impl<Node> Caret<Node>
where
    Node: NodeKey,
{
    pub fn new(id: Node, side: Cardinal) -> Self {
        Self { id, side }
    }
}

impl TryFrom<Line> for Caret {
    type Error = TooManyNodesError<Line>;
    fn try_from(value: Line) -> Result<Self, Self::Error> {
        Ok(Caret::new(NodeId::try_from(value)?, value.side))
    }
}
impl<T> From<T> for Caret
where
    T: QueryKey<NodeKey = NodeId> + Deref<Target = Caret>,
{
    fn from(value: T) -> Self {
        *value
    }
}

query_impl! { <T> From<T> for Caret where T: QueryKey<NodeKey=NodeId> + Deref<Target=Caret> }

query_impl! { TryFrom<Line> for Caret }

pub trait CaretQuery: Deref<Target = Caret> {
    type NodeQuery: NodeQuery;
    fn with_node(self, id: NodeId) -> Self;

    fn with_side(self, side: Cardinal) -> Self;
}

pub trait CaretNavigationMut: NavigationMut + Deref<Target = Caret> {
    fn set_node(&mut self, node_id: NodeId);

    fn set_side(&mut self, side: Cardinal);
}

impl ElementInformation for Query<'_, Caret> {
    fn kind(&self) -> ElementKind {
        Query::new(self.context, self.id).kind()
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

impl<'a> CaretQuery for Query<'a, Caret> {
    type NodeQuery = Query<'a, NodeId>;
    fn with_node(mut self, id: NodeId) -> Self
    where
        Self: Sized,
    {
        self.id = id;
        self
    }

    fn with_side(mut self, side: Cardinal) -> Self
    where
        Self: Sized,
    {
        self.side = side;
        self
    }
}

impl Navigation for Query<'_, Caret> {
    type Direction = Sign;

    fn peek(&self, direction: Sign) -> Option<Caret> {
        let location = self.bounds().side_location;
        let search_direction = Cardinal {
            axis: !self.side.axis,
            sign: direction,
        };
        Query::<NeighbourTiles>::from(self.with_side(search_direction))
            .items()
            .iter()
            .map(|&id| Query::new(self.context, Caret::new(id, self.side)))
            .take_while(|c| c.kind() == ElementKind::Tile)
            .find(|&c| c.bounds().side_location == location)
            .map(|c| *c)
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

impl<T, Node> QueryContext<Caret<Node>> for T
where
    T: RawQueryContext
        + QueryContext<Node, OutKey = NodeId, Error = KeyedError<Node, NodeNotFound>>,
    Node: NodeKey,
{
    type OutKey = Caret;
    type Error = KeyedError<Node, NodeNotFound>;

    fn try_query(&self, key: Caret<Node>) -> Result<Query<Self::OutKey>, Self::Error> {
        Ok(self.query(Caret::new(*self.try_query(key.id)?, key.side)))
    }
}
