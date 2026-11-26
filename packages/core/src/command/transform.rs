use crate::area::{Area, AreaQuery};
use crate::command::{Command, RawCommandExecutor};
use crate::context::QueryContext;
use crate::error::{KeyedError, KeyedSnafu, QueryError, ValidationError};
use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{
    AreaBounds, Axis, AxisBounds, Cardinal, NORTH_EAST, NORTH_WEST, SOUTH_EAST, SOUTH_WEST, Sign,
    WEST,
};
use crate::graph::{Graph, NodeId};
use crate::iter::IterativeQueryNavigation;
use crate::key::QueryKey;
use crate::line::LineQuery;
use crate::node::NodeQuery;
use crate::query::{
    ElementInformation, ElementInformation1D, ElementInformation2D, ElementKind, GraphQuery,
    GroupInformation, Information, Query,
};
use geotm_macros::{QueryKey, ValidationError};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use std::fmt::Debug;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey, Command)]
#[node(K::NodeKey)]
pub struct Transform<K = Area, B = AxisBounds>
where
    K: QueryKey,
    B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned,
{
    pub target: K,
    pub axis: Axis,
    pub intended_bounds: B,
}

impl<K, B> Transform<K, B>
where
    K: QueryKey,
    B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned,
{
    pub fn new(target: K, axis: Axis, intended_bounds: B) -> Self {
        Self {
            target,
            axis,
            intended_bounds,
        }
    }
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
pub enum TransformSurroundings {
    Negative(SegmentedArea),
    Positive(SegmentedArea),
    Bidirectional(SegmentedArea, SegmentedArea),
}

impl TransformSurroundings {
    pub fn part(&self, sign: Sign) -> Option<&SegmentedArea> {
        match (sign, self) {
            (Negative, TransformSurroundings::Negative(fragment))
            | (Positive, TransformSurroundings::Positive(fragment))
            | (Negative, TransformSurroundings::Bidirectional(fragment, _))
            | (Positive, TransformSurroundings::Bidirectional(_, fragment)) => Some(fragment),
            _ => None,
        }
    }

    pub fn parts(self) -> (Option<SegmentedArea>, Option<SegmentedArea>) {
        match self {
            TransformSurroundings::Negative(neg) => (Some(neg), None),
            TransformSurroundings::Positive(pos) => (None, Some(pos)),
            TransformSurroundings::Bidirectional(neg, pos) => (Some(neg), Some(pos)),
        }
    }

    pub fn parts_ref(&self) -> (Option<&SegmentedArea>, Option<&SegmentedArea>) {
        match self {
            TransformSurroundings::Negative(neg) => (Some(neg), None),
            TransformSurroundings::Positive(pos) => (None, Some(pos)),
            TransformSurroundings::Bidirectional(neg, pos) => (Some(neg), Some(pos)),
        }
    }

    pub fn area(&self) -> Area {
        match self {
            TransformSurroundings::Negative(neg) => neg.area(),
            TransformSurroundings::Positive(pos) => pos.area(),
            TransformSurroundings::Bidirectional(neg, pos) => {
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

impl IntoIterator for TransformSurroundings {
    type Item = SegmentedArea;
    type IntoIter = TransformDetailIter<SegmentedArea>;

    fn into_iter(self) -> Self::IntoIter {
        TransformDetailIter(self.parts())
    }
}
impl<'a> IntoIterator for &'a TransformSurroundings {
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
pub trait TransformInformation: Information + ElementInformation2D {
    fn surroundings(&self) -> Option<TransformSurroundings>;
}

#[derive(Debug, Snafu, ValidationError)]
pub enum TransformError {
    /// The area to transform is not cleanly connected! To be transformable, all sides of the area must be a straight line of parallel tiles.
    NotConnected,
    /// The requested bounds is malformed (the negative end lies after the positive end).
    InvertedBounds,
    /// Unable to transform the surrounding area to satisfy the requested bounds
    IncompatibleSurroundings,
    // There is insufficient space available for the requested size
    InsufficientSpace,
}

pub trait TransformQuery: GraphQuery + TransformInformation {
    fn into_validated(self) -> Result<Self, TransformError>;

    /// this will vary by command, but for one thing it will generally mean that guide elements that refer to a surface will be shifted to corresponding corners
    fn into_normalized(self) -> Self;
}

impl ElementInformation for Query<'_, Transform> {
    fn kind(&self) -> ElementKind {
        self.to(self.target).kind()
    }
}

impl ElementInformation2D for Query<'_, Transform> {
    fn bounds(&self) -> AreaBounds {
        self.to(self.target).bounds()
    }
}

impl TransformInformation for Query<'_, Transform> {
    fn surroundings(&self) -> Option<TransformSurroundings> {
        let target = self.to(self.target);
        let surface_area = Query::<Area>::from(target.to_corner(NORTH_WEST).to_surface());

        let initial_bounds = target.bounds()[self.axis];

        let maybe_negative = if self.intended_bounds[Negative] == initial_bounds[Negative] {
            None
        } else {
            target
                .to_line(Cardinal::new(self.axis, Negative))
                .to_opposing()
                .map(|start| surface_area.with_side(*start).to_segmented(start.side))
        };

        // we don't need to force the opposing line here
        let maybe_positive = if self.intended_bounds[Positive] != initial_bounds[Positive] {
            None
        } else {
            target
                .to_line(Cardinal::new(self.axis, Positive))
                .to_opposing()
                .map(|start| surface_area.with_side(*start).to_segmented(start.side))
        };

        match (maybe_negative, maybe_positive) {
            (Some(negative), None) => Some(TransformSurroundings::Negative(negative)),
            (None, Some(positive)) => Some(TransformSurroundings::Positive(positive)),
            (Some(negative), Some(positive)) => {
                Some(TransformSurroundings::Bidirectional(negative, positive))
            }
            (None, None) => None,
        }
    }
}

impl TransformQuery for Query<'_, Transform> {
    fn into_validated(self) -> Result<Self, TransformError> {
        let target = self.to(self.target);
        if !target.is_connected() {
            Err(TransformError::NotConnected)?
        };

        let initial_bounds = self.bounds()[self.axis];

        for sign in [Negative, Positive] {
            if self.intended_bounds[sign] != initial_bounds[sign] {
                if let Some(start) = target.to_line(Cardinal::new(self.axis, sign)).to_opposing()
                    && let Some(end) = start.to_iter(sign).last()
                {
                    debug_assert_eq!(end.side, !start.side);
                    let end_location = end.bounds().side_location;
                    match sign {
                        Negative if self.intended_bounds[sign] >= end_location => {}
                        Positive if self.intended_bounds[sign] <= end_location => {}
                        _ => Err(TransformError::IncompatibleSurroundings)?,
                    }
                } else {
                    Err(TransformError::IncompatibleSurroundings)?
                }
            }
        }

        Ok(self)
    }

    fn into_normalized(mut self) -> Self {
        let mut target = self.to(self.target);

        let initial_bounds = self.bounds()[self.axis];
        for sign in [Negative, Positive] {
            if self.intended_bounds[sign] != initial_bounds[sign] {
                let direction = Cardinal::new(self.axis, sign);
                // todo: this can probably be improved by outscanning in a more complex way but for now it's not that important.
                if let Some(updated_side) = target.to_line(direction).to_opposable() {
                    target = target.with_side(*updated_side);
                }
            }
        }

        self.target = *target;

        self
    }
}

impl<K, B, T> QueryContext<Transform<K, B>> for T
where
    K: QueryKey,
    B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned,
    T: QueryContext<K, OutKey: QueryKey>,
    for<'a> Query<'a, T::OutKey>: Into<Query<'a, Area>>,
    for<'a> Query<'a, Transform<Area, B>>: TryInto<Query<'a, Transform>, Error = TransformError>,
{
    type OutKey = Transform;

    type ErrorDetail = QueryError<K, T::ErrorDetail, TransformError>;

    fn resolve(
        &self,
        key: Transform<K, B>,
    ) -> Result<Query<'_, Self::OutKey>, KeyedError<Transform<K, B>, Self::ErrorDetail>> {
        (|| {
            let area: Area = *Into::<Query<Area>>::into(self.resolve(key.target)?);
            Ok(Query::new(
                self.as_ref(),
                Transform::new(area, key.axis, key.intended_bounds),
            )
            .try_into()?
            .into_normalized()
            .into_validated()?)
        })()
        .with_context(|_| KeyedSnafu { key })
    }
}

impl RawCommandExecutor<Transform> for Graph {
    type Response = ();

    fn execute(&mut self, command: Transform) -> Self::Response {
        if let Ok(node) = NodeId::try_from(command.target) {
            self.transform_node(node, command.axis, command.intended_bounds, Positive);
            return;
        }

        self.transform_area(
            &Query::new(self, command.target).to_segmented(WEST),
            command.intended_bounds,
            Positive,
        );

        let surroundings = Query::new(self, command).surroundings().unwrap();
        if let Some(negative_area) = surroundings.part(Negative) {
            let area = Query::new(self, negative_area.area());
            let mut new_bounds = area.bounds()[command.axis];
            new_bounds[Positive] = command.intended_bounds[Negative] - 1;
            let _side_line = *area.to_line(Cardinal::new(command.axis, Positive));
            self.transform_area(negative_area, new_bounds, Negative);
        }
        if let Some(positive_area) = surroundings.part(Negative) {
            let area = Query::new(self, positive_area.area());
            let mut new_bounds = area.bounds()[command.axis];
            new_bounds[Positive] = command.intended_bounds[Positive] + 1;
            let _side_line = *area.to_line(Cardinal::new(command.axis, Negative));
            self.transform_area(positive_area, new_bounds, Positive);
        }

        let surrounding_area = surroundings.area();

        let cross_neg =
            *Query::new(self, surrounding_area).to_line(Cardinal::new(!command.axis, Negative));

        let cross_pos =
            *Query::new(self, surrounding_area).to_line(Cardinal::new(!command.axis, Positive));
        self.update_connections(cross_neg);
        self.update_connections(cross_pos);
    }
}
