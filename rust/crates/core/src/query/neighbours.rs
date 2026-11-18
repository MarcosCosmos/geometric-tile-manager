use crate::geometry::Cardinal;
use crate::graph::{Graph, NodeId};
use crate::query::caret::Caret;
use crate::query::node::NodeLookup;
use crate::query::source::RawQuerySource;
use crate::query::{ElementInformation, ElementKind, NodeKey, Query, QueryKey};
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Deref;

pub trait NeighbourhoodInformation<'a, T>
where
    Self: ElementInformation<Target = T>,
{
    fn items(&self) -> &'a [NodeId];
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, QueryKey)]
#[query(N)]
pub struct NeighbourTiles<N = NodeId>(pub Caret<N>)
where
    N: NodeKey;

#[derive(Copy, Clone, Debug, Eq, PartialEq, QueryKey)]
#[query(N)]
pub struct NeighbourSurfaces<N = NodeId>(pub Caret<N>)
where
    N: NodeKey;

#[derive(Copy, Clone, Debug, Eq, PartialEq, QueryKey)]
#[query(N)]
pub enum Neighbours<N = NodeId>
where
    N: NodeKey,
{
    Tiles(NeighbourTiles<N>),
    Surfaces(NeighbourSurfaces<N>),
}

impl<N> From<NeighbourTiles<N>> for Neighbours<N>
where
    N: NodeKey,
{
    fn from(value: NeighbourTiles<N>) -> Self {
        Self::Tiles(value)
    }
}

impl<N> From<NeighbourSurfaces<N>> for Neighbours<N>
where
    N: NodeKey,
{
    fn from(value: NeighbourSurfaces<N>) -> Self {
        Self::Surfaces(value)
    }
}

impl<N> Deref for NeighbourTiles<N>
where
    N: NodeKey,
{
    type Target = Caret<N>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<N> Deref for NeighbourSurfaces<N>
where
    N: NodeKey,
{
    type Target = Caret<N>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<N> Deref for Neighbours<N>
where
    N: NodeKey,
{
    type Target = Caret<N>;

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

impl<'a> From<Query<'a, Caret>> for Query<'a, NeighbourTiles> {
    fn from(value: Query<'a, Caret>) -> Self {
        value.context.query(NeighbourTiles(*value))
    }
}

impl<'a> From<Query<'a, Caret>> for Query<'a, NeighbourSurfaces> {
    fn from(value: Query<'a, Caret>) -> Self {
        value.context.query(NeighbourSurfaces(*value))
    }
}

impl<'a> From<Query<'a, NeighbourTiles>> for Query<'a, Neighbours> {
    fn from(value: Query<'a, NeighbourTiles>) -> Self {
        value.context.query((*value).into())
    }
}

impl<'a> From<Query<'a, NeighbourSurfaces>> for Query<'a, Neighbours> {
    fn from(value: Query<'a, NeighbourSurfaces>) -> Self {
        value.context.query((*value).into())
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
