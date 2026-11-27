use crate::context::QueryContext;
use crate::error::{KeyedError, KeyedSnafu};
use crate::geometry::{Cardinal, LineBounds, Sign};
use crate::graph::{Graph, NodeId};
use crate::key::QueryKey;
use crate::line::Line;
use crate::neighbours::{NeighbourSurfaces, NeighbourTiles, Neighbours};
use crate::node::{NodeQuery, TooManyNodesError};
use crate::query::{ElementInformation, ElementInformation1D, ElementKind, GraphQuery, Navigation, NavigationMut, Query};
use crate::query_impl;
use geotm_macros::QueryKey;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::iter::Map;
use std::ops::Deref;

/// A caret is a position on the side of a node.
/// they provide the step function for navigating along the gutter of carets parallel to them.

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey)]
pub struct Caret<Node = NodeId>
where
    Node: QueryKey,
{
    pub id: Node,
    pub side: Cardinal,
}

impl<Node> Caret<Node>
where
    Node: QueryKey,
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
    T: QueryKey + Deref<Target = Caret>,
{
    fn from(value: T) -> Self {
        *value
    }
}

impl<Node> From<NeighbourTiles<Node>> for Caret<Node>
where
    Node: QueryKey,
{
    fn from(value: NeighbourTiles<Node>) -> Self {
        value.0
    }
}

impl<Node> From<NeighbourSurfaces<Node>> for Caret<Node>
where
    Node: QueryKey,
{
    fn from(value: NeighbourSurfaces<Node>) -> Self {
        value.0
    }
}

impl<Node> From<Neighbours<Node>> for Caret<Node>
where
    Node: QueryKey,
{
    fn from(value: Neighbours<Node>) -> Self {
        match value {
            Neighbours::Tiles(NeighbourTiles(caret)) => caret,
            Neighbours::Surfaces(NeighbourSurfaces(caret)) => caret,
        }
    }
}

query_impl! { From<NeighbourTiles | NeighbourSurfaces | Neighbours> | TryFrom<Line> for Caret }

pub trait CaretQuery: Deref<Target = Caret> {
    type NodeQuery: NodeQuery;
    fn to_node(self) -> Self::NodeQuery;
}

pub trait CaretNavigationMut: NavigationMut + Deref<Target = Caret> {
    fn set_node(&mut self, node_id: NodeId);

    fn set_side(&mut self, side: Cardinal);
}

impl ElementInformation for Query<'_, Caret> {
    fn kind(&self) -> ElementKind {
        self.to(self.id).kind()
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

    fn to_node(self) -> Self::NodeQuery {
        self.to(self.id)
    }
}

impl Ord for Query<'_, Caret> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.bounds().axis_bounds.cmp(&other.bounds().axis_bounds)
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
        self.to(NeighbourTiles(Caret::new(self.id, search_direction)))
            .into_iter()
            .map(|node| self.to(Caret::new(*node, self.side)))
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

impl<Node> QueryContext<Caret<Node>> for Graph
where
    Self: QueryContext<Node, OutKey = NodeId>,
    Node: QueryKey,
{
    type OutKey = Caret;
    type ErrorDetail = KeyedError<Node, <Self as QueryContext<Node>>::ErrorDetail>;

    fn resolve(
        &self,
        key: Caret<Node>,
    ) -> Result<Query<'_, Self::OutKey>, KeyedError<Caret<Node>, Self::ErrorDetail>> {
        Ok(Query::new(
            self,
            Caret::new(
                *self.resolve(key.id).with_context(|_| KeyedSnafu { key })?,
                key.side,
            ),
        ))
    }
}
