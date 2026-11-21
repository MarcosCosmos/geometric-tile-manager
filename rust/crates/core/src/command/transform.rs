use std::error::Error;
use crate::QueryKey;
use crate::area::Area;
use crate::command::add_tile::AddTile;
use crate::command::{Command, DeleteNode, MoveLine};
use crate::error::ValidationError;
use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{Axis, Sign};
use crate::key::QueryKey;
use crate::line::{Line, LineQuery};
use crate::query::{ElementInformation, GraphQuery, Query};
use crate::source::{QuerySource, RawQuerySource};
use markers_derive::ValidationError;
use serde::Serialize;
use snafu::Snafu;
use std::ops::Deref;

#[derive(Clone, Debug, Serialize)]
pub struct SegmentedArea {
    pub segments: Vec<Area>,
    pub axis: Axis,
}

impl SegmentedArea {
    pub fn new(segments: Vec<Area>, axis: Axis) -> Self {
        Self { segments, axis }
    }

    pub fn area(&self) -> Area {
        let start = self.segments.first().unwrap();
        let end = self.segments.last().unwrap();
        match self.axis {
            Horizontal => Area {
                north_west: start.north_west,
                north_east: end.north_west,
                south_east: end.south_east,
                south_west: start.south_west,
            },
            Vertical => Area {
                north_west: start.north_west,
                north_east: start.north_east,
                south_east: end.south_east,
                south_west: end.south_west,
            },
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub enum TransformDetails {
    Negative(SegmentedArea),
    Positive(SegmentedArea),
    Bidirectional(SegmentedArea, SegmentedArea),
}

impl TransformDetails {
    pub fn part(&self, sign: Sign) -> Option<&SegmentedArea> {
        match (sign, self) {
            (Negative, TransformDetails::Negative(fragment))
            | (Positive, TransformDetails::Positive(fragment))
            | (Negative, TransformDetails::Bidirectional(fragment, _))
            | (Positive, TransformDetails::Bidirectional(_, fragment)) => Some(fragment),
            _ => None,
        }
    }

    pub fn parts(self) -> (Option<SegmentedArea>, Option<SegmentedArea>) {
        match self {
            TransformDetails::Negative(neg) => (Some(neg), None),
            TransformDetails::Positive(pos) => (None, Some(pos)),
            TransformDetails::Bidirectional(neg, pos) => (Some(neg), Some(pos)),
        }
    }

    pub fn parts_ref(&self) -> (Option<&SegmentedArea>, Option<&SegmentedArea>) {
        match self {
            TransformDetails::Negative(neg) => (Some(neg), None),
            TransformDetails::Positive(pos) => (None, Some(pos)),
            TransformDetails::Bidirectional(neg, pos) => (Some(neg), Some(pos)),
        }
    }

    pub fn area(&self) -> Area {
        match self {
            TransformDetails::Negative(neg) => neg.area(),
            TransformDetails::Positive(pos) => pos.area(),
            TransformDetails::Bidirectional(neg, pos) => {
                let start = neg.segments.first().unwrap();
                let end = pos.segments.last().unwrap();
                match neg.axis {
                    Horizontal => Area {
                        north_west: start.north_west,
                        north_east: end.north_west,
                        south_east: end.south_east,
                        south_west: start.south_west,
                    },
                    Vertical => Area {
                        north_west: start.north_west,
                        north_east: start.north_east,
                        south_east: end.south_east,
                        south_west: end.south_west,
                    },
                }
            }
        }
    }
}

// todo: when we refactor XInformation traits into structs with derived provider traits, the existing TransformDetails will become that struct for transform queries.
pub trait TransformInformation : ElementInformation {
    fn details(&self) -> TransformDetails;
}

pub trait TransformKey: Command + QueryKey + Deref<Target: QueryKey + Into<Line<<Self::Target as QueryKey>::NodeKey>>>
{
}

#[derive(Debug, Snafu, ValidationError)]
pub enum TransformProblem {
    /// The guide-line is connected end-to-end! Note: All carets within the bounds of a guide-line must lie on the line.
    NotConnected,
}
pub trait TransformQuery: GraphQuery {
    fn into_validated(self) -> Result<Self, TransformProblem>;

    /// this will vary by command, but for one thing it will generally mean that guide elements that refer to a surface will be shifted to corresponding corners
    fn into_normalized(self) -> Self;
}

/// this is the method you'd generally use when inserting tiles (unsure what strategy to employ for removals yet)
/// only works if starting from a tile, which is intended
/// todo: transform selection will be given it's own query type with the edit command as part of the arg, so it can resolve the correct shape fully.
fn into_transform_selection(mut self) -> TransformDetails {
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
                TransformDetails::Negative(inward)
            } else {
                TransformDetails::Positive(inward)
            }
        }
        Some(outward_fragment) => {
            if self.side.sign == Positive {
                TransformDetails::Bidirectional(inward, outward_fragment)
            } else {
                TransformDetails::Bidirectional(outward_fragment, inward)
            }
        }
        _ => panic!(),
    }
}
