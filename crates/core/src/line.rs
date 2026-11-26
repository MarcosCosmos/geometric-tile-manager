use crate::area::{Area, AreaQuery};
use crate::caret::{Caret, CaretQuery};
use crate::context::QueryContext;
use crate::error::{KeyedError, KeyedSnafu, QueryError, ValidationError};
use crate::geometry::{Cardinal, LineBounds, Sign, Sign::*};
use crate::graph::NodeId;
use crate::iter::IterativeQueryNavigation;
use crate::key::QueryKey;
use crate::node::{NodeNotFound, NodeQuery};
use crate::query::{
    ElementInformation, ElementInformation1D, ElementKind, GroupInformation, Navigation,
    NavigationMut, Query,
};
use crate::query_impl;
use markers_derive::{QueryKey, ValidationError};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use std::cmp::Ordering;
use std::fmt::Debug;
use std::ops::{Deref, Index, IndexMut};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey)]
// As in, a line between two parallel Carets.
// This line may or may not be fully "connected" (without cutting through the interior of any tiles)
pub struct Line<Node = NodeId>
where
    Node: QueryKey,
{
    negative_id: Node,
    positive_id: Node,
    pub side: Cardinal,
}

impl<Node> Line<Node>
where
    Node: QueryKey,
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
    Node: QueryKey,
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
    Node: QueryKey,
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
    Node: QueryKey,
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
    fn into_validated(self) -> Result<Self, LineProblem>;
    fn with_side(self, side: Cardinal) -> Self;
    fn with_nodes(self, negative_id: NodeId, positive_id: NodeId) -> Self;
    fn into_opposable(self) -> Option<Self>;
    fn into_opposing(self) -> Option<Self>;
    /// This method finds the line that includes at least one of the existing ends but where both ends have the same `bounds().side_location`, if any exists.
    /// It will use given sign to choose which direction to scan towards.
    fn into_straightened(self, direction: Sign) -> Option<Self>;
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
        Query::<NodeId>::try_from(*self).is_ok()
            || self
                .into_end(Negative)
                .into_iter(Positive)
                .inclusive()
                .any(|n| n.id == self.negative_id)
    }
}

// todo later: implement nested key swapping reverse engineer the input that corresponded to each error.
#[derive(Debug, Snafu, ValidationError)]
pub enum LineProblem {
    /// A Line cannot span multiple surfaces
    MultipleSurfaces,
    /// The ends of the line are arranged in the wrong order
    WrongOrder,
}

impl<'a> LineQuery for Query<'a, Line> {
    type CaretQuery = Query<'a, Caret>;
    type AreaQuery = Query<'a, Area>;
    fn into_end(self, sign: Sign) -> Self::CaretQuery {
        Query::new(self.context, Caret::new(self[sign], self.side))
    }

    fn into_normalized(mut self) -> Self {
        if self.into_end(Negative).bounds().side_location
            > self.into_end(Positive).bounds().side_location
        {
            let tmp = self.positive_id;
            self.positive_id = self.negative_id;
            self.negative_id = tmp;
        }
        self
    }

    fn into_validated(self) -> Result<Self, LineProblem> {
        let neg = self.into_end(Negative);
        let pos = self.into_end(Positive);
        if Query::<NodeId>::from(neg).into_containing_surface().deref()
            != Query::<NodeId>::from(pos).into_containing_surface().deref()
        {
            Err(LineProblem::MultipleSurfaces)
        } else if self.into_end(Negative).bounds().side_location
            > self.into_end(Positive).bounds().side_location
        {
            Err(LineProblem::WrongOrder)
        } else {
            Ok(self)
        }
    }

    fn with_side(mut self, side: Cardinal) -> Self
    where
        Self: Sized,
    {
        self.side = side;
        self
    }

    fn with_nodes(mut self, negative_id: NodeId, positive_id: NodeId) -> Self
    where
        Self: Sized,
    {
        self.negative_id = negative_id;
        self.positive_id = positive_id;
        self.into_normalized()
    }

    /// Opposable as in is facing towards an opposing line.
    /// That is, a line with the same axis-bounds, on the inverse side of the opposing tile(s).
    fn into_opposable(self) -> Option<Self> {
        let (cross_neg, _cross_pos) = self.side.perpendicular_directions();
        Some(
            self.with_nodes(
                self.into_end(Negative)
                    .into_iter(Negative)
                    .inclusive()
                    .find(|c| c.with_side(cross_neg).peek(self.side.sign).is_some())?
                    .id,
                self.into_end(Positive)
                    .into_iter(Positive)
                    .inclusive()
                    .find(|c| c.with_side(cross_neg).peek(self.side.sign).is_some())?
                    .id,
            ),
        )
    }

    fn into_opposing(self) -> Option<Self> {
        let (cross_neg, cross_pos) = self.side.perpendicular_directions();
        let opposing_neg = self
            .into_end(Negative)
            .with_side(cross_neg)
            .into_next(self.side.sign)?;
        let opposing_pos = self
            .into_end(Negative)
            .with_side(cross_pos)
            .into_next(self.side.sign)?;
        Some(Query::new(
            self.context,
            Line::new(opposing_neg.id, opposing_pos.id, !self.side),
        ))
    }

    /// This method finds the line that includes at least one of the existing ends but where both ends have the same `bounds().side_location`, if any exists.
    /// It will use given sign to choose which direction to scan towards.
    fn into_straightened(mut self, direction: Sign) -> Option<Self> {
        let (cross_neg, cross_pos) = self.side.perpendicular_directions();
        let current_neg = Query::new(self.context, Caret::new(self.negative_id, cross_neg));
        let current_pos = Query::new(self.context, Caret::new(self.positive_id, cross_pos));
        let neg_location = current_neg.with_side(self.side).bounds().side_location;
        let pos_location = current_pos.with_side(self.side).bounds().side_location;
        match (direction, neg_location.cmp(&pos_location)) {
            (_, Ordering::Equal) => Some(self),
            (Negative, Ordering::Greater) | (Positive, Ordering::Less)
                if let Some(neg_target) = current_neg
                    .into_iter(direction)
                    .find(|caret| caret.bounds().side_location == pos_location) =>
            {
                self[Negative] = neg_target.id;
                Some(self)
            }
            (Negative, Ordering::Less) | (Positive, Ordering::Greater)
                if let Some(pos_target) = current_pos
                    .into_iter(direction)
                    .find(|caret| caret.bounds().side_location == neg_location) =>
            {
                self[Positive] = pos_target.id;
                Some(self)
            }
            _ => None,
        }
    }
}

impl Ord for Query<'_, Line> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.bounds()
            .side_location
            .cmp(&other.bounds().side_location)
    }
}

impl Navigation for Query<'_, Line> {
    type Direction = Sign;

    fn peek(&self, direction: Self::Direction) -> Option<Line> {
        let target_side = Cardinal::new(self.side.axis, direction);
        let mut candidate = *self.with_side(target_side);
        if self.kind() == ElementKind::Tile
            && self.side.sign != direction
            && Query::new(self.context, candidate).is_connected()
        {
            Some(candidate)
        } else {
            let (cross_neg, cross_pos) = self.side.perpendicular_directions();
            let mut current_neg = Query::new(self.context, Caret::new(self.negative_id, cross_neg));
            let mut current_pos = Query::new(self.context, Caret::new(self.positive_id, cross_pos));
            let mut neg_iter = current_neg.into_iter(direction);
            let mut pos_iter = current_pos.into_iter(direction);
            if let (Some(next_neg), Some(next_pos)) = (neg_iter.next(), pos_iter.next()) {
                if self.kind() == ElementKind::Tile
                    && self.side.sign == direction
                    && self.is_connected()
                {
                    let candidate = Line::new(next_neg.id, next_pos.id, !self.side);
                    assert!(Query::new(self.context, candidate).is_connected());
                    Some(candidate)
                } else if self.kind() == ElementKind::Surface {
                    assert_ne!(self.side.sign, direction); // we have to be going in the reverse direction if we got some, otherwise we'd be going into the void!
                    let candidate = Line::new(next_neg.id, next_pos.id, self.side);
                    assert!(Query::new(self.context, candidate).is_connected());
                    Some(candidate)
                } else {
                    current_neg = next_neg;
                    current_pos = next_neg;
                    let mut neg_location =
                        current_neg.with_side(target_side).bounds().side_location;
                    let mut pos_location =
                        current_pos.with_side(target_side).bounds().side_location;
                    loop {
                        match (direction, neg_location.cmp(&pos_location)) {
                            (_, Ordering::Equal) => {
                                let candidate = Query::new(self.context, Line::new(current_neg.id, current_pos.id, target_side));
                                if !candidate.is_connected()
                                    && let (Some(next_neg), Some(next_pos)) =
                                    (neg_iter.next(), pos_iter.next())
                                {
                                    current_neg = next_neg;
                                    current_pos = next_pos;
                                } else {
                                    break Some(*candidate);
                                }
                            },
                            (Negative, Ordering::Greater) | (Positive, Ordering::Less)
                            if let Some(next_neg) = neg_iter.next() =>
                            {
                                current_neg = next_neg;
                                neg_location = next_neg.with_side(target_side).bounds().side_location;
                            }
                            (Negative, Ordering::Less) | (Positive, Ordering::Greater)
                            if let Some(next_pos) = pos_iter.next() => {
                                current_pos = next_pos;
                                pos_location =
                                    current_pos.with_side(target_side).bounds().side_location;
                            }
                            _ => break None,
                        };
                    }
                }
            } else {
                None
            }
        }
    }
}

impl<T, Node> QueryContext<Line<Node>> for T
where
    T: QueryContext<Node, OutKey = NodeId>,
    Node: QueryKey,
{
    type OutKey = Line;

    type ErrorDetail = QueryError<Node, T::ErrorDetail, LineProblem>;

    fn resolve(&self, key: Line<Node>) -> Result<Query<'_, Self::OutKey>, KeyedError<Line<Node>, Self::ErrorDetail>> {
        (|| {
            Ok(Query::new(
                self.as_ref(),
                Line::new(
                    *self.resolve(key[Negative])?,
                    *self.resolve(key[Positive])?,
                    key.side,
                ),
            )
            .into_normalized()
            .into_validated()?)
        })()
        .with_context(|_| KeyedSnafu { key })
    }
}
