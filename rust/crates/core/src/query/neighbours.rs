use crate::geometry::Cardinal;
use crate::graph::{Graph, NodeId};
use crate::query::caret::Caret;
use crate::query::key::NodeKey;
use crate::query::node::{NodeNotFound, NodeQuery};
use crate::query::source::{QuerySource, RawQuerySource};
use crate::query::{ElementInformation, ElementKind, Query, QueryKey};
use crate::query_impl;
use markers_derive::QueryKey;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::ops::Deref;

pub trait NeighbourhoodInformation<'a, T>
where
    Self: ElementInformation<Target = T>,
{
    fn items(&self) -> &'a [NodeId];
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey)]
pub struct NeighbourTiles<Node = NodeId>(pub Caret<Node>)
where
    Node: NodeKey;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey)]
pub struct NeighbourSurfaces<Node = NodeId>(pub Caret<Node>)
where
    Node: NodeKey;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey)]
pub enum Neighbours<Node = NodeId>
where
    Node: NodeKey,
{
    Tiles(NeighbourTiles<Node>),
    Surfaces(NeighbourSurfaces<Node>),
}

impl<Node> From<NeighbourTiles<Node>> for Neighbours<Node>
where
    Node: NodeKey,
{
    fn from(value: NeighbourTiles<Node>) -> Self {
        Self::Tiles(value)
    }
}

impl<Node> From<NeighbourSurfaces<Node>> for Neighbours<Node>
where
    Node: NodeKey,
{
    fn from(value: NeighbourSurfaces<Node>) -> Self {
        Self::Surfaces(value)
    }
}

impl<Node> From<Caret<Node>> for NeighbourTiles<Node>
where
    Node: NodeKey,
{
    fn from(value: Caret<Node>) -> Self {
        Self(value)
    }
}

impl<Node> From<Caret<Node>> for NeighbourSurfaces<Node>
where
    Node: NodeKey,
{
    fn from(value: Caret<Node>) -> Self {
        Self(value)
    }
}

query_impl! { From<Caret> for NeighbourTiles }
query_impl! { From<Caret> for NeighbourSurfaces }
query_impl! { From<NeighbourTiles | NeighbourSurfaces> for Neighbours }

impl<Node> Deref for NeighbourTiles<Node>
where
    Node: NodeKey,
{
    type Target = Caret<Node>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<Node> Deref for NeighbourSurfaces<Node>
where
    Node: NodeKey,
{
    type Target = Caret<Node>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<Node> Deref for Neighbours<Node>
where
    Node: NodeKey,
{
    type Target = Caret<Node>;

    fn deref(&self) -> &Self::Target {
        match self {
            Neighbours::Tiles(tile) => tile.deref(),
            Neighbours::Surfaces(surface) => surface.deref(),
        }
    }
}

impl ElementInformation for Query<'_, NeighbourTiles> {
    fn kind(&self) -> ElementKind {
        ElementKind::Tile
    }
}

impl ElementInformation for Query<'_, NeighbourSurfaces> {
    fn kind(&self) -> ElementKind {
        ElementKind::Surface
    }
}

impl ElementInformation for Query<'_, Neighbours> {
    fn kind(&self) -> ElementKind {
        match self.key {
            Neighbours::Tiles(_) => ElementKind::Tile,
            Neighbours::Surfaces(_) => ElementKind::Surface,
        }
    }
}

impl<'a> NeighbourhoodInformation<'a, NeighbourTiles> for Query<'a, NeighbourTiles> {
    fn items(&self) -> &'a [NodeId] {
        &self.context.tile_neighbourhoods[&self.id][self.side]
    }
}

impl<'a> NeighbourhoodInformation<'a, NeighbourSurfaces> for Query<'a, NeighbourSurfaces> {
    fn items(&self) -> &'a [NodeId] {
        &self.context.surface_neighbourhoods[&self.id][self.side]
    }
}

impl<'a> NeighbourhoodInformation<'a, Neighbours> for Query<'a, Neighbours> {
    fn items(&self) -> &'a [NodeId] {
        let target = self.context.query(self.id).into_surface_of();
        &self.context.surface_neighbourhoods[&target][self.side]
    }
}

#[derive(Copy, Clone, Debug)]
pub struct NeighbourCaretIter<'a, Iter>
where
    Iter: DoubleEndedIterator<Item = &'a NodeId>,
{
    context: &'a Graph,
    inner: Iter,
    out_direction: Cardinal,
}

impl<'a, Iter> NeighbourCaretIter<'a, Iter>
where
    Iter: DoubleEndedIterator<Item = &'a NodeId>,
{
    pub fn new(context: &'a Graph, inner: Iter, out_direction: Cardinal) -> Self {
        Self {
            context,
            inner,
            out_direction,
        }
    }
}

impl<'a, Iter> Iterator for NeighbourCaretIter<'a, Iter>
where
    Iter: DoubleEndedIterator<Item = &'a NodeId>,
{
    type Item = Query<'a, Caret>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|&item| self.context.query(Caret::new(item, self.out_direction)))
    }
}

impl<'a, Iter> DoubleEndedIterator for NeighbourCaretIter<'a, Iter>
where
    Iter: DoubleEndedIterator<Item = &'a NodeId>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner
            .next_back()
            .map(|&item| self.context.query(Caret::new(item, self.out_direction)))
    }
}

impl<'a, T> IntoIterator for Query<'a, T>
where
    T: QueryKey + Deref<Target = Caret>,
    Self: Into<&'a [NodeId]>,
{
    type Item = Query<'a, Caret>;
    type IntoIter = NeighbourCaretIter<'a, std::slice::Iter<'a, NodeId>>;

    fn into_iter(self) -> Self::IntoIter {
        let side: Cardinal = !(**self).side;
        NeighbourCaretIter::<'a, _>::new(self.context, self.into().into_iter(), side)
    }
}

impl<T, Node> QuerySource<NeighbourTiles<Node>> for T
where
    T: RawQuerySource + QuerySource<Node, OutKey = NodeId, Error = NodeNotFound<Node>>,
    Node: NodeKey + 'static,
{
    type OutKey = NeighbourTiles;
    type Error = NodeNotFound<Node>;

    fn try_query(&self, key: NeighbourTiles<Node>) -> Result<Query<Self::OutKey>, Self::Error> {
        Ok(self.query(NeighbourTiles(Caret::new(
            *self.try_query(key.id)?,
            key.side,
        ))))
    }
}

impl<T, Node> QuerySource<NeighbourSurfaces<Node>> for T
where
    T: RawQuerySource + QuerySource<Node, OutKey = NodeId, Error = NodeNotFound<Node>>,
    Node: NodeKey + 'static,
{
    type OutKey = NeighbourSurfaces;
    type Error = NodeNotFound<Node>;

    fn try_query(&self, key: NeighbourSurfaces<Node>) -> Result<Query<Self::OutKey>, Self::Error> {
        Ok(self.query(NeighbourSurfaces(Caret::new(
            *self.try_query(key.id)?,
            key.side,
        ))))
    }
}
