use std::fmt::{write, Display, Formatter};
use crate::area::{Area, AreaQuery};
use crate::command::Command;
use crate::error::ValidationError;
use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{AreaBounds, Axis, AxisBounds, Sign, NORTH_WEST, Cardinal, NORTH_EAST, SOUTH_EAST, SOUTH_WEST};
use crate::key::{NodeKey, QueryKey};
use crate::line::{Line, LineQuery};
use crate::query::{ElementInformation, ElementInformation1D, ElementInformation2D, ElementKind, GraphQuery, GroupInformation, Information, Query};
use markers_derive::{QueryKey, ValidationError};
use serde::{Deserialize, Serialize};
use snafu::{ensure, Snafu};
use std::ops::Deref;
use crate::command::add_tile::AddTile;
use crate::graph::NodeId;
use crate::iter::IterativeQueryNavigation;
use crate::node::NodeQuery;
use std::error::Error;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey, Command)]
#[node(K::NodeKey)]
pub struct Transform<K=Area, B=AxisBounds>
where
    K: QueryKey,
    B: QueryKey,
    for <'a> Query<'a, K>: Into<Query<'a, Area>>,
    for <'a> Query<'a, B>: Into<Query<'a, AxisBounds>>,

{
    pub target: K,
    pub axis: Axis,
    pub new_bounds: B,
}

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
            Horizontal => Area::new(
                start[NORTH_WEST],
                end[NORTH_EAST],
                start[SOUTH_WEST],
                end[SOUTH_EAST],
            ),
            Vertical => Area::new(
                start[NORTH_WEST],
                start[NORTH_EAST],
                end[SOUTH_WEST],
                end[SOUTH_EAST],
            ),
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
                    Horizontal => Area::new(
                        start[NORTH_WEST],
                        end[NORTH_EAST],
                        start[SOUTH_WEST],
                        end[SOUTH_EAST],
                    ),
                    Vertical => Area::new(
                        start[NORTH_WEST],
                        start[NORTH_EAST],
                        end[SOUTH_WEST],
                        end[SOUTH_EAST],
                    ),
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
pub enum TransformSpaceError {
    /// No adjacent area to scale
    NothingToScale,
    // Insufficient space for the requested size
    OutOfBounds,
}
#[derive(Debug, ValidationError)]
pub enum TransformError {
    NotConnected,
    Space {
        negative: Option<TransformSpaceError>,
        positive: Option<TransformSpaceError>,
    }
}

impl Display for TransformError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            TransformError::NotConnected => write!(f, "The area to transform is not cleanly connected! To be transformable, all sides of the area must be a straight line of parallel tiles."),
            TransformError::Space { negative, positive } => {
                if let Some(negative) = negative {
                    writeln!(f, "Could not transform the Negative side of the area: {negative};")?
                }
                if let Some(positive) = positive {
                    writeln!(f, "Could not transform the Positive side of the area: {positive};")?
                }
                Ok(())
            }
        }
    }
}

impl Error for TransformError {
}

pub trait TransformQuery: GraphQuery {
    fn into_validated(self) -> Result<Self, TransformError>;

    /// this will vary by command, but for one thing it will generally mean that guide elements that refer to a surface will be shifted to corresponding corners
    fn into_normalized(self) -> Self;
}


impl ElementInformation for Query<'_, Transform> {
    fn kind(&self) -> ElementKind {
        Query::new(self.context, self.target).kind()
    }
}

impl TransformInformation for Query<'_, Transform> {
    fn details(&self) -> TransformDetails {
        let area = Query::new(self.context, self.target);
        let surface_area = Query::<Area>::from(area.into_corner(NORTH_WEST).into_containing_surface());
        let area = Query::new(self.context, self.target);

        let maybe_negative = area.into_side(Cardinal::new(self.axis, Negative))
            .into_opposing().map(|start| surface_area.with_constraint(*start).into_segmented(start.side));


        // we don't need to force the opposing line here
        let maybe_positive = area.into_side(Cardinal::new(self.axis, Positive)).into_opposing().map(|start| {
            surface_area
                .with_constraint(*start)
                .into_segmented(start.side)
        });

        match (maybe_negative, maybe_positive) {
            (Some(negative), None) => TransformDetails::Negative(negative),
            (None, Some(positive)) => TransformDetails::Positive(positive),
            (Some(negative), Some(positive)) => TransformDetails::Bidirectional(negative, positive),
            (None, None) => panic!("Illegal transform query resulting in empty transform details. Did you forget to run validation?"),
        }
    }
}

impl TransformQuery for Query<'_, Transform> {
    fn into_validated(self) -> Result<Self, TransformError> {
        let area = Query::new(self.context, self.target);
        let initial_bounds = area.bounds()[self.axis];
        if !area.is_connected() {
            Err(TransformError::NotConnected)?
        };
        let [negative, positive] = [Negative, Positive].map(|sign| {
            if self.new_bounds[sign] != initial_bounds[sign] {
                if let Some(opp) = area.into_side(Cardinal::new(self.axis, sign)).into_opposing() {
                    if let Some(opp_end) = opp.into_iter(sign).last() {
                        debug_assert_eq!(opp_end.side, !opp.side);
                        if self.new_bounds[sign] < opp_end.bounds().side_location {
                            Some(TransformSpaceError::OutOfBounds)
                        } else {
                            None
                        }
                    } else {
                        Some(TransformSpaceError::NothingToScale)
                    }
                } else {
                    Some(TransformSpaceError::NothingToScale)
                }
            } else {
                None
            }
        });

        if negative.is_some() || positive.is_some() {
            Err(TransformError::Space { negative, positive })?;
        }

        Ok(self)
    }

    fn into_normalized(mut self) -> Self {
        if self.kind() == ElementKind::Surface
            && let Some(tile_line) = Query::new(self.context, **self).peek(!self.side.sign)
        {
            self.0 = tile_line;
        }
        self
    }
}