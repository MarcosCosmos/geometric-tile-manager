use crate::area::Area;
use crate::command::Command;
use crate::error::ValidationError;
use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{Axis, Sign};
use crate::key::QueryKey;
use crate::line::Line;
use crate::query::{ElementInformation, GraphQuery, Information};
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

    pub fn iter(&self) -> TransformDetailIter<&SegmentedArea> {
        self.into_iter()
    }
}

impl IntoIterator for TransformDetails {
    type Item = SegmentedArea;
    type IntoIter = TransformDetailIter<SegmentedArea>;

    fn into_iter(self) -> Self::IntoIter {
        TransformDetailIter(self.parts())
    }
}
impl<'a> IntoIterator for &'a TransformDetails {
    type Item = &'a SegmentedArea;
    type IntoIter = TransformDetailIter<&'a SegmentedArea>;

    fn into_iter(self) -> Self::IntoIter {
        TransformDetailIter(self.parts_ref())
    }
}

pub struct TransformDetailIter<T>((Option<T>, Option<T>));

impl<T> Iterator for TransformDetailIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.0.take().or_else(|| self.0.1.take())
    }
}

// todo: when we refactor XInformation traits into structs with derived provider traits, the existing TransformDetails will become that struct for transform queries.
pub trait TransformInformation: Information {
    fn details(&self) -> TransformDetails;
}

// pub trait TransformKey:
//     Command + QueryKey + Deref<Target: QueryKey + Into<Line<<Self::Target as QueryKey>::NodeKey>>>
// {
// }

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
