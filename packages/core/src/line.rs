use crate::area::{Area, AreaQuery};
use crate::caret::{Caret, CaretQuery};
use crate::context::QueryContext;
use crate::error::{KeyedError, KeyedSnafu, QueryError, ValidationError};
use crate::geometry::{Cardinal, LineBounds, Sign, Sign::*};
use crate::graph::{Graph, NodeId};
use crate::iter::{IterativeQueryNavigation, NavIter, RangeIter};
use crate::key::{NodeKey, QueryKey};
use crate::neighbours::NeighbourTiles;
use crate::node::NodeQuery;
use crate::query::{
    ElementInformation, ElementInformation1D, ElementKind, GraphQuery, GroupInformation,
    Navigation, NavigationMut, Query, Standardisation,
};
use crate::query_impl;
use geotm_macros::{Encode, QueryKey, ValidationError};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, Index, IndexMut};

// As in, a line between two parallel Carets.
// This line may or may not be fully "connected" (without cutting through the interior of any tiles)
#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize, QueryKey, Encode)]
#[encode("[{negative_id}..{positive_id}].{side}")]
pub struct Line<Node = NodeId>
where
    Node: NodeKey,
{
    #[serde(bound(deserialize = "Node: NodeKey"))]
    negative_id: Node,
    #[serde(bound(deserialize = "Node: NodeKey"))]
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

pub trait LineQuery:
    Deref<Target = Line>
    + ElementInformation1D
    + IterativeQueryNavigation
    + Navigation<Direction = Sign>
{
    fn to_end(self, sign: Sign) -> Self::Query<Caret>;

    /// Opposable as in is facing towards an opposing line.
    /// That is, a line with the same axis-bounds, on the inverse side of the opposing tile(s).
    fn to_opposable(self) -> Option<Self>;

    fn to_opposing(self) -> Option<Self>;

    /// a special case of to opposing that obtains the sentinel side of the containing surface is this line is touching it.
    fn to_sentinel(self) -> Option<Self>;

    /// This method finds the line that includes at least one of the existing ends but where both ends have the same `bounds().side_location`, if any exists.
    /// It will use given sign to choose which direction to scan towards.
    fn to_straightened(self, direction: Sign) -> Option<Self>;

    fn to_area<F>(self, f: F) -> Option<Self::Query<Area>>
    where
        F: FnOnce(NavIter<Query<Line>>) -> Option<Query<Line>>;
}

pub trait LineNavigationMut: NavigationMut + Deref<Target = Line> {
    fn set_side(&mut self, side: Cardinal);
    fn set_nodes(&mut self, negative_id: NodeId, positive_id: NodeId);
}

impl ElementInformation for Query<'_, Line> {
    fn kind(&self) -> ElementKind {
        match (self.to_end(Negative).kind(), self.to_end(Positive).kind()) {
            (ElementKind::Tile, ElementKind::Tile) => ElementKind::Tile,
            (ElementKind::Surface, ElementKind::Surface) => ElementKind::Surface,
            _ => ElementKind::Mixed,
        }
    }
}

impl ElementInformation1D for Query<'_, Line> {
    fn bounds(&self) -> LineBounds {
        let neg_bounds = self.to_end(Negative).bounds();
        let pos_bounds = self.to_end(Positive).bounds();
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
                .to_end(Negative)
                .to_iter(Positive)
                .inclusive()
                .any(|n| n.id == self.negative_id)
    }
}

// todo later: implement nested key swapping reverse engineer the input that corresponded to each error.
#[derive(Debug, Snafu, ValidationError)]
pub enum LineError {
    /// A Line cannot span multiple surfaces
    MultipleSurfaces,
    /// The ends of the line are arranged in the wrong order
    WrongOrder,
}

impl<'a> Standardisation for Query<'a, Line> {
    type Error = LineError;

    fn to_normalized(mut self) -> Self {
        if self.to_end(Negative).bounds().side_location
            > self.to_end(Positive).bounds().side_location
        {
            let tmp = self.positive_id;
            self.positive_id = self.negative_id;
            self.negative_id = tmp;
        }
        self
    }

    /// checks and gaurantees that the ends of the line are arranged in normalised (Negative < Positive) order along the axis between them.
    /// note that this can result in them being converted into a line over a single caret if one subsumes the other
    fn to_validated(self) -> Result<Self, LineError> {
        let neg = self.to_end(Negative);
        let pos = self.to_end(Positive);
        if Query::<NodeId>::from(neg).to_surface().deref()
            != Query::<NodeId>::from(pos).to_surface().deref()
        {
            Err(LineError::MultipleSurfaces)
        } else if self.to_end(Negative).bounds().side_location
            > self.to_end(Positive).bounds().side_location
        {
            Err(LineError::WrongOrder)
        } else {
            Ok(self)
        }
    }
}

impl<'a> LineQuery for Query<'a, Line> {
    fn to_end(self, sign: Sign) -> Self::Query<Caret> {
        self.to(Caret::new(self[sign], self.side))
    }

    /// Opposable as in is facing towards an opposing line.
    /// That is, a line with the same axis-bounds, on the inverse side of the opposing tile(s).
    fn to_opposable(mut self) -> Option<Self> {
        let (cross_neg, cross_pos) = self.side.perpendicular_directions();
        self[Negative] = self
            .to_end(Negative)
            .to_iter(Negative)
            .inclusive()
            .find(|&caret| {
                self.to(Caret::new(caret.id, cross_neg))
                    .peek(self.side.sign)
                    .is_some()
            })?
            .id;
        self[Positive] = self
            .to_end(Positive)
            .to_iter(Positive)
            .inclusive()
            .find(|&caret| {
                self.to(Caret::new(caret.id, cross_pos))
                    .peek(self.side.sign)
                    .is_some()
            })?
            .id;
        Some(self)
    }

    fn to_opposing(self) -> Option<Self> {
        let (cross_neg, cross_pos) = self.side.perpendicular_directions();
        Some(
            self.to(Line::new(
                self.to(Caret::new(self[Negative], cross_neg))
                    .to_next(self.side.sign)?
                    .id,
                self.to(Caret::new(self[Positive], cross_pos))
                    .to_next(self.side.sign)?
                    .id,
                self.side,
            )),
        )
    }

    fn to_sentinel(mut self) -> Option<Self> {
        let surface = self.to_end(Negative).to_node().to_surface();
        let surface_side = surface.to_side(self.side);
        if Query::<NeighbourTiles>::from(surface_side)
            .into_iter()
            .any(|node| *node == self[Negative])
        {
            Some(surface_side.into())
        } else {
            None
        }
    }

    /// This method finds the line that includes at least one of the existing ends but where both ends have the same `bounds().side_location`, if any exists.
    /// It will use given sign to choose which direction to scan towards.
    fn to_straightened(mut self, direction: Sign) -> Option<Self> {
        let (cross_neg, cross_pos) = self.side.perpendicular_directions();
        let neg_location = self.to_end(Negative).bounds().side_location;
        let pos_location = self.to_end(Positive).bounds().side_location;
        let current_neg = self.to(Caret::new(self[Negative], cross_neg));
        let current_pos = self.to(Caret::new(self[Positive], cross_pos));
        match (direction, neg_location.cmp(&pos_location)) {
            (_, Ordering::Equal) => Some(self),
            (Negative, Ordering::Greater) | (Positive, Ordering::Less)
                if let Some(neg_target) = current_neg
                    .to_iter(direction)
                    .find(|caret| caret.bounds().side_location == pos_location) =>
            {
                self[Negative] = neg_target.id;
                Some(self)
            }
            (Negative, Ordering::Less) | (Positive, Ordering::Greater)
                if let Some(pos_target) = current_pos
                    .to_iter(direction)
                    .find(|caret| caret.bounds().side_location == neg_location) =>
            {
                self[Positive] = pos_target.id;
                Some(self)
            }
            _ => None,
        }
    }

    fn to_area<F>(self, f: F) -> Option<Self::Query<Area>>
    where
        F: FnOnce(NavIter<Query<Line>>) -> Option<Query<Line>>,
    {
        let opp = f(self.to_iter(!self.side.sign))?;
        let base_area: Query<Area> = self.to_end(Negative).to_node().into();
        Some(base_area.with_side(*self).with_side(*opp))
    }
}

impl Ord for Query<'_, Line> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.bounds()
            .side_location
            .cmp(&other.bounds().side_location)
    }
}

/// For `Line` queries, navigation is among 'siblings' occupying the same row/column area:
/// - A group of 'sibling' lines must all be parallel, with the same axis bounds, and there must be perpendiclar lines at those bounds consisting entirely of collinear carets.
impl Navigation for Query<'_, Line> {
    type Direction = Sign;

    fn peek(&self, direction: Self::Direction) -> Option<Line> {
        match self.kind() {
            ElementKind::Tile => {
                let target_side = Cardinal::new(self.side.axis, direction);
                let candidate = Line {
                    side: target_side,
                    ..**self
                };
                if self.side.sign != direction && self.to(candidate).is_connected() {
                    Some(candidate)
                } else {
                    let (cross_neg, cross_pos) = self.side.perpendicular_directions();
                    let mut neg_iter = self
                        .to(Caret::new(self[Negative], cross_neg))
                        .to_iter(direction);
                    let mut pos_iter = self
                        .to(Caret::new(self[Positive], cross_pos))
                        .to_iter(direction);
                    if let (Some(next_neg), Some(next_pos)) = (neg_iter.next(), pos_iter.next()) {
                        if self.side.sign == direction && self.is_connected() {
                            let candidate = Line::new(next_neg.id, next_pos.id, !self.side);
                            assert!(self.to(candidate).is_connected());
                            Some(candidate)
                        } else {
                            let mut neg = self.to(Caret::new(next_neg.id, target_side));
                            let mut pos = self.to(Caret::new(next_pos.id, target_side));
                            let mut neg_location;
                            let mut pos_location;
                            loop {
                                let candidate = self.to(Line::new(neg.id, pos.id, target_side));
                                if !candidate.is_connected()
                                    && let (Some(next_neg), Some(next_pos)) =
                                        (neg_iter.next(), pos_iter.next())
                                {
                                    neg.id = next_neg.id;
                                    pos.id = next_pos.id;

                                    neg_location = neg.bounds().side_location;
                                    pos_location = pos.bounds().side_location;
                                } else {
                                    break Some(*candidate);
                                }

                                loop {
                                    match (direction, neg_location.cmp(&pos_location)) {
                                        (Negative, Ordering::Greater)
                                        | (Positive, Ordering::Less)
                                            if let Some(next_neg) = neg_iter.next() =>
                                        {
                                            neg.id = next_neg.id;
                                            neg_location = neg.bounds().side_location;
                                        }
                                        (Negative, Ordering::Less)
                                        | (Positive, Ordering::Greater)
                                            if let Some(next_pos) = pos_iter.next() =>
                                        {
                                            pos.id = next_pos.id;
                                            pos_location = self.bounds().side_location;
                                        }
                                        _ => break,
                                    };
                                }
                            }
                        }
                    } else {
                        None
                    }
                }
            }
            _ => None,
        }
    }
}

impl IntoIterator for Query<'_, Line> {
    type Item = <Self as GraphQuery>::Query<Caret>;
    type IntoIter = RangeIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        debug_assert!(self.is_connected());
        RangeIter::new(self.to_end(Negative), self.to_end(Positive), Positive)
    }
}

impl<Node> QueryContext<Line<Node>> for Graph
where
    Graph: QueryContext<Node, OutKey = NodeId>,
    Node: NodeKey,
{
    type OutKey = Line;

    type ErrorDetail = QueryError<Node, <Self as QueryContext<Node>>::ErrorDetail, LineError>;

    fn resolve(
        &self,
        key: Line<Node>,
    ) -> Result<Query<'_, Self::OutKey>, KeyedError<Line<Node>, Self::ErrorDetail>> {
        (|| {
            Ok(Query::new(
                self,
                Line::new(
                    *self.resolve(key[Negative])?,
                    *self.resolve(key[Positive])?,
                    key.side,
                ),
            )
            .to_normalized()
            .to_validated()?)
        })()
        .with_context(|_| KeyedSnafu { key })
    }
}
