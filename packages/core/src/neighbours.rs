use crate::caret::{Caret, CaretQuery};
use crate::context::QueryContext;
use crate::error::{KeyedError, KeyedSnafu};
use crate::graph::NodeId;
use crate::iter::QueryIter;
use crate::key::{QueryKey, query_impl};
use crate::node::NodeQuery;
use crate::query::{ElementInformation, ElementKind, GraphQuery, Query};
use geotm_macros::QueryKey;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use std::fmt::Debug;

pub trait NeighbourhoodQuery:
    GraphQuery + IntoIterator<Item: NodeQuery, IntoIter: DoubleEndedIterator>
{
}

trait NeighbourhoodQueryImpl<'a> {
    fn items(&self) -> &'a [NodeId];
}

pub trait NeighbourSurfacesQuery {
    fn to_normalised(self) -> Self;
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
        let target = self.to_normalised().0;
        &self.context.surface_neighbourhoods[&target.id][target.side]
    }
}

impl<'a> NeighbourhoodQueryImpl<'a> for Query<'a, Neighbours> {
    fn items(&self) -> &'a [NodeId] {
        match **self {
            Neighbours::Tiles(key) => self.to(key).items(),
            Neighbours::Surfaces(key) => self.to(key).items(),
        }
    }
}

impl NeighbourSurfacesQuery for Query<'_, NeighbourSurfaces> {
    fn to_normalised(mut self) -> Self {
        self.0.id = *Query::<Caret>::from(self).to_node().to_surface();
        self
    }
}

impl<'a, T> IntoIterator for Query<'a, T>
where
    Query<'a, T>: NeighbourhoodQueryImpl<'a>,
    T: QueryKey,
{
    type Item = Query<'a, NodeId>;
    type IntoIter = QueryIter<'a, NodeId, std::slice::Iter<'a, NodeId>>;

    fn into_iter(self) -> Self::IntoIter {
        QueryIter::new(self.context, self.items().iter())
    }
}

impl<T, Node> QueryContext<NeighbourTiles<Node>> for T
where
    T: QueryContext<Node, OutKey = NodeId>,
    Node: QueryKey,
{
    type OutKey = NeighbourTiles;
    type ErrorDetail = KeyedError<Node, T::ErrorDetail>;

    fn resolve(
        &self,
        key: NeighbourTiles<Node>,
    ) -> Result<Query<'_, Self::OutKey>, KeyedError<NeighbourTiles<Node>, Self::ErrorDetail>> {
        let caret: Caret<Node> = key.into();
        Ok(Query::new(
            self.as_ref(),
            NeighbourTiles(Caret::new(
                *self
                    .resolve(caret.id)
                    .with_context(|_| KeyedSnafu { key })?,
                caret.side,
            )),
        ))
    }
}

impl<T, Node> QueryContext<NeighbourSurfaces<Node>> for T
where
    T: QueryContext<Node, OutKey = NodeId>,
    Node: QueryKey,
{
    type OutKey = NeighbourSurfaces;
    type ErrorDetail = KeyedError<Node, T::ErrorDetail>;

    fn resolve(
        &self,
        key: NeighbourSurfaces<Node>,
    ) -> Result<Query<'_, Self::OutKey>, KeyedError<NeighbourSurfaces<Node>, Self::ErrorDetail>>
    {
        let caret: Caret<Node> = key.into();
        Ok(Query::new(
            self.as_ref(),
            NeighbourSurfaces(Caret::new(
                *self
                    .resolve(caret.id)
                    .with_context(|_| KeyedSnafu { key })?,
                caret.side,
            )),
        )
        .to_normalised())
    }
}
