use crate::error::Error;
use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{EAST, NORTH, Point, SOUTH, WEST};
use crate::graph::{Graph, NodeId};
use crate::query::area::{Area, AreaInformation};
use crate::query::caret::Caret;
use crate::query::line::{Line, LineLookup};
use crate::query::neighbours::{NeighbourSurfaces, NeighbourTiles, NeighbourhoodInformation};
use crate::query::node::NodeLookup;
use crate::query::{ElementInformation, ElementKind, NodeKey, Query, QueryKey};
use std::fmt::Debug;
use std::hash::Hash;

pub(crate) trait RawQuerySource {
    fn query<N>(&self, key: N) -> Query<N>
    where
        N: QueryKey;
}

impl RawQuerySource for Graph {
    fn query<N>(&self, key: N) -> Query<N>
    where
        N: QueryKey,
    {
        Query { context: self, key }
    }
}

pub trait QuerySource<N> {
    type Index: Sized;
    fn try_query(&self, key: N) -> anyhow::Result<Query<Self::Index>>
    where
        <Self as QuerySource<N>>::Index: QueryKey;
}

impl QuerySource<Point> for Graph {
    type Index = NodeId;

    fn try_query(&self, key: Point) -> anyhow::Result<Query<Self::Index>> {
        self.bounds
            .iter()
            .filter(|&(_, &bounds)| {
                bounds[Horizontal][Negative] <= key.x
                    && bounds[Horizontal][Positive] >= key.x
                    && bounds[Vertical][Negative] <= key.y
                    && bounds[Vertical][Positive] >= key.y
            })
            .map(|(&id, _)| self.query(id))
            .filter(|node| {
                node.kind() == ElementKind::Tile
                    || [NORTH, SOUTH, EAST, WEST].into_iter().all(|side| {
                        Query::<NeighbourTiles>::from(node.into_side(side))
                            .items()
                            .len()
                            == 0
                    })
            })
            .next()
            .ok_or(Error::NodeNotFound(key).into())
    }
}

impl<T, N> QuerySource<Caret<N>> for T
where
    T: RawQuerySource + QuerySource<N, Index = NodeId>,
    N: NodeKey,
{
    type Index = Caret;

    fn try_query(&self, key: Caret<N>) -> anyhow::Result<Query<Self::Index>> {
        Ok(self.query(Caret::new(*self.try_query(key.id)?, key.side)))
    }
}

impl<T, N> QuerySource<Line<N>> for T
where
    T: RawQuerySource + QuerySource<N, Index = NodeId>,
    N: NodeKey,
{
    type Index = Line;

    fn try_query(&self, key: Line<N>) -> anyhow::Result<Query<Self::Index>> {
        Ok(self
            .query(Line::<NodeId>::new(
                *self.try_query(key[Negative])?,
                *self.try_query(key[Positive])?,
                key.side,
            ))
            .into_validated()?)
    }
}

impl<T, N> QuerySource<Area<N>> for T
where
    T: RawQuerySource + QuerySource<N, Index = NodeId>,
    N: NodeKey,
{
    type Index = Area;

    fn try_query(&self, key: Area<N>) -> anyhow::Result<Query<Self::Index>> {
        Ok(self
            .query(Area {
                north_west: *self.try_query(key.north_west)?,
                north_east: *self.try_query(key.north_east)?,
                south_east: *self.try_query(key.south_east)?,
                south_west: *self.try_query(key.south_west)?,
            })
            .into_validated()?)
    }
}

impl<T, N> QuerySource<NeighbourTiles<N>> for T
where
    T: RawQuerySource + QuerySource<N, Index = NodeId>,
    N: NodeKey,
{
    type Index = NeighbourTiles;

    fn try_query(&self, key: NeighbourTiles<N>) -> anyhow::Result<Query<Self::Index>> {
        Ok(self.query(NeighbourTiles(Caret::new(
            *self.try_query(key.id)?,
            key.side,
        ))))
    }
}

impl<T, N> QuerySource<NeighbourSurfaces<N>> for T
where
    T: RawQuerySource + QuerySource<N, Index = NodeId>,
    N: NodeKey,
{
    type Index = NeighbourSurfaces;

    fn try_query(&self, key: NeighbourSurfaces<N>) -> anyhow::Result<Query<Self::Index>> {
        Ok(self.query(NeighbourSurfaces(Caret::new(
            *self.try_query(key.id)?,
            key.side,
        ))))
    }
}
