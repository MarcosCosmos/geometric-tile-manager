use crate::error::Error;
use crate::geometry::{
    Axis::*, Cardinal, LineBounds, NORTH_EAST, NORTH_WEST, SOUTH_EAST, SOUTH_WEST, Sign, Sign::*,
};
use crate::graph::NodeId;
use crate::query::area::{Area, AreaQuery};
use crate::query::caret::{Caret, CaretQuery};
use crate::query::iter::QueryIteration;
use crate::query::key::{NodeKey, QueryKey};
use crate::query::node::NodeQuery;
use crate::query::selection::SelectionDetails;
use crate::query::source::{QuerySource, RawQuerySource};
use crate::query::{ElementInformation, ElementInformation1D, ElementKind, Navigation, GroupInformation, Query, NavigationMut};
use crate::query_impl;
use markers_derive::QueryKey;
use std::fmt::Debug;
use std::ops::{Deref, Index, IndexMut};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey)]
// As in, a line between two parallel Carets.
// This line may or may not be fully "connected" (without cutting through the interior of any tiles)
pub struct Line<Node = NodeId>
where
    Node: NodeKey,
{
    negative_id: Node,
    positive_id: Node,
    pub side: Cardinal,
}

impl<Node> Line<Node>
where
    Node: NodeKey,
{
    pub fn new(negative_id: Node, positive_id: Node, side: Cardinal) -> Self {
        Self {
            negative_id,
            positive_id,
            side,
        }
    }
}

impl<Node> Index<Sign> for Line<Node>
where
    Node: NodeKey,
{
    type Output = Node;

    fn index(&self, index: Sign) -> &Self::Output {
        match index {
            Negative => &self.negative_id,
            Positive => &self.positive_id,
        }
    }
}

impl<Node> IndexMut<Sign> for Line<Node>
where
    Node: NodeKey,
{
    fn index_mut(&mut self, index: Sign) -> &mut Self::Output {
        match index {
            Negative => &mut self.negative_id,
            Positive => &mut self.positive_id,
        }
    }
}

impl<Node> From<Caret<Node>> for Line<Node>
where
    Node: NodeKey,
{
    fn from(value: Caret<Node>) -> Self {
        Self::new(value.id, value.id, value.side)
    }
}

query_impl! { From<Caret> for Line }

pub trait LineQuery: ElementInformation1D + Deref<Target = Line> {
    type CaretQuery: CaretQuery;
    type AreaQuery: AreaQuery;
    fn into_end(self, sign: Sign) -> Self::CaretQuery;

    /// checks and gaurantees that the ends of the line are arranged in normalised (Negative < Positive) order along the axis between them.
    /// note that this can result in them being converted into a line over a single caret if one subsumes the other
    fn into_normalized(self) -> Self;
    fn into_validated(self) -> Result<Self, Error<Self::Target>>;
    fn with_side(self, side: Cardinal) -> Self;
    fn with_nodes(self, negative_id: NodeId, positive_id: NodeId) -> Self;
    fn into_scan_area(self) -> Self::AreaQuery;
}

pub trait LineNavigationMut: NavigationMut + Deref<Target = Line> {
    fn set_side(&mut self, side: Cardinal);
    fn set_nodes(&mut self, negative_id: NodeId, positive_id: NodeId);
}

impl ElementInformation for Query<'_, Line> {
    fn kind(&self) -> ElementKind {
        match (
            self.into_end(Negative).kind(),
            self.into_end(Positive).kind(),
        ) {
            (ElementKind::Tile, ElementKind::Tile) => ElementKind::Tile,
            (ElementKind::Surface, ElementKind::Surface) => ElementKind::Surface,
            _ => ElementKind::Mixed,
        }
    }
}

impl ElementInformation1D for Query<'_, Line> {
    fn bounds(&self) -> LineBounds {
        let neg_bounds = self.into_end(Negative).bounds();
        let pos_bounds = self.into_end(Positive).bounds();
        LineBounds {
            side: self.side,
            side_location: match self.side.sign {
                Positive => i32::min(neg_bounds.side_location, pos_bounds.side_location),
                Negative => i32::max(neg_bounds.side_location, pos_bounds.side_location),
            },
            axis_bounds: neg_bounds.axis_bounds.span(&pos_bounds.axis_bounds),
        }
    }
}

impl<'a> GroupInformation for Query<'a, Line> {
    fn is_connected(&self) -> bool {
        self.into_end(Negative)
            .into_iter(Positive)
            .inclusive()
            .any(|n| n.id == self.negative_id)
    }
}

impl<'a> LineQuery for Query<'a, Line> {
    type CaretQuery = Query<'a, Caret>;
    type AreaQuery = Query<'a, Area>;
    fn into_end(self, sign: Sign) -> Self::CaretQuery {
        self.context.query(Caret::new(self[sign], self.side))
    }

    fn into_normalized(mut self) -> Self {
        if self.into_end(Negative).bounds().side_location
            > self.into_end(Positive).bounds().side_location
        {
            std::mem::swap(&mut self.key.negative_id, &mut self.key.positive_id)
        }
        self
    }

    fn into_validated(self) -> Result<Self, Error<Self::Target>> {
        let neg = self.into_end(Negative);
        let pos = self.into_end(Positive);
        if Query::<NodeId>::from(neg).into_surface_of().deref()
            != Query::<NodeId>::from(pos).into_surface_of().deref()
        {
            Err(Error::MultipleSurfaces(*self))
        } else if neg.bounds().side_location != pos.bounds().side_location {
            Err(Error::NotFullyConnected(*self))
        } else {
            Ok(self.into_normalized())
        }
    }

    fn with_side(mut self, side: Cardinal) -> Self
    where
        Self: Sized,
    {
        self.key.side = side;
        self
    }

    fn with_nodes(mut self, negative_id: NodeId, positive_id: NodeId) -> Self
    where
        Self: Sized,
    {
        self.key.negative_id = negative_id;
        self.key.positive_id = positive_id;
        self.into_normalized()
    }

    /// A scan area is an area that represents the area that would be produced between this line and some far off line,
    /// following gutters perpendicular to this line. i.e. the scan stops when those two gutters end.
    /// Scan areas are key building blocks in determining selections for graph edits.
    /// Before the scan is performed, the scan area is approximated by the absolute limits of the area (i.e. the far side of the containing surface).
    fn into_scan_area(self) -> Self::AreaQuery {
        let surface_id = *Query::<NodeId>::from(self.into_end(Negative)).into_surface_of();
        let mut result = Area::new(surface_id, surface_id, surface_id, surface_id);
        match self.side {
            Cardinal {
                axis: Horizontal,
                sign: Negative,
            } => {
                result[NORTH_WEST] = self[Negative];
                result[SOUTH_WEST] = self[Positive];
            }
            Cardinal {
                axis: Horizontal,
                sign: Positive,
            } => {
                result[NORTH_EAST] = self[Negative];
                result[SOUTH_EAST] = self[Positive];
            }
            Cardinal {
                axis: Vertical,
                sign: Negative,
            } => {
                result[NORTH_WEST] = self[Negative];
                result[NORTH_EAST] = self[Positive];
            }
            Cardinal {
                axis: Vertical,
                sign: Positive,
            } => {
                result[SOUTH_WEST] = self[Negative];
                result[SOUTH_EAST] = self[Positive];
            }
        }
        self.context.query(result)
    }

    
}

impl Navigation for Query<'_, Line> {
    type Direction = Sign;

    fn peek(&self, direction: Self::Direction) -> Option<Line> {
        let target_side = Cardinal::new(self.side.axis, direction);
        let mut candidate = *self.with_side(target_side);
        if self.kind() == ElementKind::Tile
            && self.side.sign != direction
            && self.context.query(candidate).is_connected()
        {
            Some(candidate)
        } else {
            let (cross_neg, cross_pos) = self.side.perpendicular_directions();
            let mut current_neg = self.context.query(Caret::new(self.negative_id, cross_neg));
            let mut current_pos = self.context.query(Caret::new(self.positive_id, cross_pos));
            let mut neg_iter = current_neg.into_iter(direction);
            let mut pos_iter = current_pos.into_iter(direction);
            if let (Some(next_neg), Some(next_pos)) = (neg_iter.next(), pos_iter.next()) {
                if self.kind() == ElementKind::Tile
                    && self.side.sign == direction
                    && self.is_connected()
                {
                    let candidate = Line::new(next_neg.id, next_pos.id, !self.side);
                    assert!(self.context.query(candidate).is_connected());
                    Some(candidate)
                } else {
                    current_neg = next_neg;
                    current_pos = next_neg;
                    loop {
                        let mut neg_location =
                            current_neg.with_side(target_side).bounds().side_location;
                        let mut pos_location =
                            current_pos.with_side(target_side).bounds().side_location;
                        while neg_location != pos_location {
                            if neg_location < pos_location {
                                if let Some(next_neg) = neg_iter.next() {
                                    current_neg = next_neg;
                                    neg_location =
                                        current_neg.with_side(target_side).bounds().side_location;
                                } else {
                                    continue;
                                }
                            } else if neg_location > pos_location {
                                if let Some(next_pos) = pos_iter.next() {
                                    current_pos = next_pos;
                                    pos_location =
                                        current_pos.with_side(target_side).bounds().side_location;
                                } else {
                                    continue;
                                }
                            }
                        }

                        candidate = Line::new(current_neg.id, current_pos.id, target_side);
                        if !self.context.query(candidate).is_connected()
                            && let (Some(next_neg), Some(next_pos)) =
                                (neg_iter.next(), pos_iter.next())
                        {
                            current_neg = next_neg;
                            current_pos = next_pos;
                        } else {
                            break Some(candidate);
                        }
                    }
                }
            } else {
                None
            }
        }
    }
}

impl<T, Node> QuerySource<Line<Node>> for T
where
    T: RawQuerySource + QuerySource<Node, OutKey = NodeId>,
    Node: NodeKey,
{
    type OutKey = Line<NodeId>;

    fn try_query(&self, key: Line<Node>) -> anyhow::Result<Query<Self::OutKey>> {
        Ok(self
            .query(Line::new(
                *self.try_query(key[Negative])?,
                *self.try_query(key[Positive])?,
                key.side,
            ))
            .into_validated()?)
    }
}
