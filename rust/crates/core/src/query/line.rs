use crate::error::Error;
use crate::geometry::{
    Axis::*, Cardinal, LineBounds, NORTH, NORTH_EAST, NORTH_WEST, SOUTH_EAST, SOUTH_WEST, Sign,
    Sign::*,
};
use crate::graph::{Graph, NodeId};
use crate::query::area::{Area, AreaInformation};
use crate::query::caret::{Caret, CaretLookup};
use crate::query::iter::ElementIteration;
use crate::query::node::NodeLookup;
use crate::query::selection::SelectionArea;
use crate::query::source::RawQuerySource;
use crate::query::{
    ElementInformation, ElementInformation1D, ElementInformationMut, ElementKind,
    ElementNavigation, GroupInformation, NodeKey, Query, QueryKey,
};
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Deref, Index, IndexMut};

#[derive(Copy, Clone, Debug, Eq, PartialEq, QueryKey)]
#[query(N)]
// As in, a line between two parallel Carets.
// This line may or may not be fully "connected" (without cutting through the interior of any tiles)
pub struct Line<N = NodeId>
where
    N: NodeKey,
{
    negative_id: N,
    positive_id: N,
    pub side: Cardinal,
}

impl<N> Line<N>
where
    N: NodeKey,
{
    pub fn new(negative_id: N, positive_id: N, side: Cardinal) -> Self {
        Self {
            negative_id,
            positive_id,
            side,
        }
    }
}

impl<N> Index<Sign> for Line<N>
where
    N: NodeKey,
{
    type Output = N;

    fn index(&self, index: Sign) -> &Self::Output {
        match index {
            Negative => &self.negative_id,
            Positive => &self.positive_id,
        }
    }
}

impl<N> IndexMut<Sign> for Line<N>
where
    N: NodeKey,
{
    fn index_mut(&mut self, index: Sign) -> &mut Self::Output {
        match index {
            Negative => &mut self.negative_id,
            Positive => &mut self.positive_id,
        }
    }
}

impl<N> From<Caret<N>> for Line<N>
where
    N: NodeKey,
{
    fn from(value: Caret<N>) -> Self {
        Self::new(value.id, value.id, value.side)
    }
}

pub trait LineLookup: ElementInformation1D + Deref<Target = Line> {
    type CaretInfo: CaretLookup;
    type AreaInfo: AreaInformation;
    fn into_end(self, sign: Sign) -> Self::CaretInfo;

    /// checks and gaurantees that the ends of the line are arranged in normalised (Negative < Positive) order along the axis between them.
    /// note that this can result in them being converted into a line over a single caret if one subsumes the other
    fn into_normalized(self) -> Self;
    fn into_validated(self) -> Result<Self, Error<Self::Target>>;
    fn with_side(self, side: Cardinal) -> Self;
    fn with_nodes(self, negative_id: NodeId, positive_id: NodeId) -> Self;
    fn into_scan_area(self) -> Self::AreaInfo;
    /// this is the method you'd generally use when inserting tiles (unsure what strategy to employ for removals yet)
    /// only works if starting from a tile, which is intended
    fn into_transform_selection(self) -> SelectionArea;
}

pub trait LineInformationMut: ElementInformationMut + Deref<Target = Line> {
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
    type NodeInformation = Query<'a, NodeId>;

    fn is_connected(&self) -> bool {
        self.into_end(Negative)
            .into_iter(Positive)
            .inclusive()
            .any(|n| n.id == self.negative_id)
    }
}

impl<'a> LineLookup for Query<'a, Line> {
    type CaretInfo = Query<'a, Caret>;
    type AreaInfo = Query<'a, Area>;
    fn into_end(self, sign: Sign) -> Self::CaretInfo {
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
    fn into_scan_area(self) -> Self::AreaInfo {
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

    /// this is the method you'd generally use when inserting tiles (unsure what strategy to employ for removals yet)
    /// only works if starting from a tile, which is intended
    /// todo: transform selection will be given it's own query type with the edit command as part of the arg, so it can resolve the correct shape fully.
    fn into_transform_selection(mut self) -> SelectionArea {
        self.into_validated()
            .expect("Invalid selection starting line");
        let inward = self
            .with_side(self.side)
            .into_scan_area()
            .into_segmented(!self.side);

        let maybe_outward_segments = {
            let (cross_neg, cross_pos) = self.side.perpendicular_directions();
            let initial_neg = self
                .into_end(Negative)
                .with_side(cross_neg)
                .into_next(!self.side.sign);
            let initial_pos = self
                .into_end(Negative)
                .with_side(cross_pos)
                .into_next(!self.side.sign);
            match (initial_neg, initial_pos) {
                (Some(neg), Some(pos)) => {
                    let reverse_line = self.context.query(Line::new(neg.id, pos.id, !self.side));
                    if reverse_line.is_connected() {
                        Some(reverse_line.into_scan_area().into_segmented(self.side))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        };

        // the rest of this is just normalising the above backward/forward parts to collapse intermediate Nones and arranged them in natural order
        match maybe_outward_segments {
            (None) => {
                if self.side.sign == Positive {
                    SelectionArea::Negative(inward)
                } else {
                    SelectionArea::Positive(inward)
                }
            }
            Some(outward_fragment) => {
                if self.side.sign == Positive {
                    SelectionArea::Bidirectional(inward, outward_fragment)
                } else {
                    SelectionArea::Bidirectional(outward_fragment, inward)
                }
            }
            _ => panic!(),
        }
    }
}

impl ElementNavigation for Query<'_, Line> {
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

impl<'a> From<Query<'a, Caret>> for Query<'a, Line> {
    fn from(value: Query<'a, Caret>) -> Self {
        value.context.query((*value).into())
    }
}
