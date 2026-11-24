use crate::caret::Caret;
use crate::context::QueryContext;
use crate::error::KeyedError;
use crate::graph::NodeId;
use crate::iter::KeyIter;
use crate::key::{QueryKey, query_impl};
use crate::node::{NodeNotFound, NodeQuery};
use crate::query::{ElementInformation, ElementKind, GraphQuery, Query};
use markers_derive::QueryKey;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub trait NeighbourhoodQuery:
GraphQuery + IntoIterator<Item: NodeQuery, IntoIter: DoubleEndedIterator>
{
}

trait NeighbourhoodQueryImpl<'a> {
    fn items(&self) -> &'a [NodeId];
}

pub trait NeighbourSurfacesQuery {
    fn into_normalised(self) -> Self;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey)]
pub struct NeighbourTiles<Node = NodeId>(pub Caret<Node>)
where
    Node: QueryKey;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey)]
pub struct NeighbourSurfaces<Node = NodeId>(pub Caret<Node>)
where
    Node: QueryKey;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey)]
pub enum Neighbours<Node = NodeId>
where
    Node: QueryKey,
{
    Tiles(NeighbourTiles<Node>),
    Surfaces(NeighbourSurfaces<Node>),
}

impl<Node> From<NeighbourTiles<Node>> for Neighbours<Node>
where
    Node: QueryKey,
{
    fn from(value: NeighbourTiles<Node>) -> Self {
        Self::Tiles(value)
    }
}

impl<Node> From<NeighbourSurfaces<Node>> for Neighbours<Node>
where
    Node: QueryKey,
{
    fn from(value: NeighbourSurfaces<Node>) -> Self {
        Self::Surfaces(value)
    }
}

impl<Node> From<Caret<Node>> for NeighbourTiles<Node>
where
    Node: QueryKey,
{
    fn from(value: Caret<Node>) -> Self {
        Self(value)
    }
}

impl<Node> From<Caret<Node>> for NeighbourSurfaces<Node>
where
    Node: QueryKey,
{
    fn from(value: Caret<Node>) -> Self {
        Self(value)
    }
}

query_impl! { From<Caret> for NeighbourTiles }
query_impl! { From<Caret> for NeighbourSurfaces }
query_impl! { From<NeighbourTiles | NeighbourSurfaces> for Neighbours }

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
        match **self {
            Neighbours::Tiles(_) => ElementKind::Tile,
            Neighbours::Surfaces(_) => ElementKind::Surface,
        }
    }
}

impl<'a, T> NeighbourhoodQuery for Query<'a, T>
where
    T: QueryKey + Into<Caret>,
    Self: IntoIterator<Item = Query<'a, NodeId>, IntoIter: DoubleEndedIterator>,
{
}

impl<'a> NeighbourhoodQueryImpl<'a> for Query<'a, NeighbourTiles> {

    fn items(&self) -> &'a [NodeId] {
        &self.context.tile_neighbourhoods[&self.0.id][self.0.side]
    }
}

impl<'a> NeighbourhoodQueryImpl<'a> for Query<'a, NeighbourSurfaces> {

    fn items(&self) -> &'a [NodeId] {
        &self.context.surface_neighbourhoods[&self.0.id][self.0.side]
    }
}

impl<'a> NeighbourhoodQueryImpl<'a> for Query<'a, Neighbours> {
    fn items(&self) -> &'a [NodeId] {
        match **self {
            Neighbours::Tiles(key) => Query::new(self.context, key).items(),
            Neighbours::Surfaces(key) => Query::new(self.context, key).items(),
        }
    }
}

impl NeighbourSurfacesQuery for Query<'_, NeighbourSurfaces> {
    fn into_normalised(mut self) -> Self {
        self.0.id = *Query::<NodeId>::from(Query::<Caret>::from(self)).into_containing_surface();
        self
    }
}

impl<'a, T> IntoIterator for Query<'a, T>
where
    Query<'a, T>: NeighbourhoodQueryImpl<'a>,
    T: QueryKey,
{
    type Item = Query<'a, NodeId>;
    type IntoIter = KeyIter<
        'a,
        NodeId,
        std::slice::Iter<'a, NodeId>,
    >;

    fn into_iter(self) -> Self::IntoIter {
        KeyIter::new(self.context, self.items().into_iter())
    }
}

impl<T, Node> QueryContext<NeighbourTiles<Node>> for T
where
    T: QueryContext<Node, OutKey = NodeId, Error = KeyedError<Node, NodeNotFound>>,
    Node: QueryKey,
{
    type OutKey = NeighbourTiles;
    type Error = KeyedError<Node, NodeNotFound>;

    fn try_query(&self, key: NeighbourTiles<Node>) -> Result<Query<Self::OutKey>, Self::Error> {
        let caret: Caret<Node> = key.into();
        Ok(Query::new(self.as_ref(), NeighbourTiles(Caret::new(
            *self.try_query(caret.id)?,
            caret.side,
        ))))
    }
}

impl<T, Node> QueryContext<NeighbourSurfaces<Node>> for T
where
    T: QueryContext<Node, OutKey = NodeId, Error = KeyedError<Node, NodeNotFound>>,
    Node: QueryKey,
{
    type OutKey = NeighbourSurfaces;
    type Error = KeyedError<Node, NodeNotFound>;

    fn try_query(&self, key: NeighbourSurfaces<Node>) -> Result<Query<Self::OutKey>, Self::Error> {
        let caret: Caret<Node> = key.into();
        Ok(Query::new(self.as_ref(), NeighbourSurfaces(Caret::new(
                *self.try_query(caret.id)?,
                caret.side,
            )))
            .into_normalised())
    }
}
