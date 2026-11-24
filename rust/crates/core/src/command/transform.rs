use crate::area::{Area, AreaQuery};
use crate::command::{Command, RawCommandExecutor};
use crate::context::QueryContext;
use crate::error::{KeyedError, KeyedSnafu, QueryError, ValidationError};
use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{
    AreaBounds, Axis, AxisBounds, BoundsAlignment, Cardinal, NORTH_EAST, NORTH_WEST, SOUTH_EAST,
    SOUTH_WEST, Sign, WEST,
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
use markers_derive::{QueryKey, ValidationError};
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
pub trait TransformInformation: Information + ElementInformation2D {
    fn details(&self) -> Option<TransformDetails>;
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
        Query::new(self.context, self.target).kind()
    }
}

impl ElementInformation2D for Query<'_, Transform> {
    fn bounds(&self) -> AreaBounds {
        Query::new(self.context, self.target).bounds()
    }
}

impl TransformInformation for Query<'_, Transform> {
    fn details(&self) -> Option<TransformDetails> {
        let area = Query::new(self.context, self.target);
        let surface_area =
            Query::<Area>::from(area.into_corner(NORTH_WEST).into_containing_surface());
        let area = Query::new(self.context, self.target);

        let maybe_negative = area
            .into_side(Cardinal::new(self.axis, Negative))
            .into_opposing()
            .map(|start| {
                surface_area
                    .with_constraint(*start)
                    .into_segmented(start.side)
            });

        // we don't need to force the opposing line here
        let maybe_positive = area
            .into_side(Cardinal::new(self.axis, Positive))
            .into_opposing()
            .map(|start| {
                surface_area
                    .with_constraint(*start)
                    .into_segmented(start.side)
            });

        match (maybe_negative, maybe_positive) {
            (Some(negative), None) => Some(TransformDetails::Negative(negative)),
            (None, Some(positive)) => Some(TransformDetails::Positive(positive)),
            (Some(negative), Some(positive)) => {
                Some(TransformDetails::Bidirectional(negative, positive))
            }
            (None, None) => None,
        }
    }
}

impl TransformQuery for Query<'_, Transform> {
    fn into_validated(self) -> Result<Self, TransformError> {
        let target = Query::new(self.context, self.target);
        if !target.is_connected() {
            Err(TransformError::NotConnected)?
        };

        let initial_bounds = self.bounds()[self.axis];

        for sign in [Negative, Positive] {
            if self.intended_bounds[sign] != initial_bounds[sign] {
                if let Some(start) = target
                    .into_side(Cardinal::new(self.axis, sign))
                    .into_opposing()
                    && let Some(end) = start.into_iter(sign).last()
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
        let mut target = Query::new(self.context, self.target);

        let initial_bounds = self.bounds()[self.axis];
        for sign in [Negative, Positive] {
            if self.intended_bounds[sign] != initial_bounds[sign] {
                let direction = Cardinal::new(self.axis, sign);
                if let Some(updated_side) = target.into_side(direction).into_opposable() {
                    target = target.with_constraint(*updated_side);
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
    T: QueryContext<K, OutKey: QueryKey<NodeKey = NodeId>>,
    for<'a> Query<'a, T::OutKey>: Into<Query<'a, Area>>,
    for<'a> Query<'a, Transform<Area, B>>: TryInto<Query<'a, Transform>>,
    for<'a> QueryError<K, T::Error, TransformError>: From<T::Error>
        + From<<Query<'a, Transform<Area, B>> as TryInto<Query<'a, Transform>>>::Error>,
{
    type OutKey = Transform;

    type Error = KeyedError<Transform<K, B>, QueryError<K, T::Error, TransformError>>;

    fn try_query(&self, key: Transform<K, B>) -> Result<Query<Self::OutKey>, Self::Error> {
        (|| {
            let area: Area = *Into::<Query<Area>>::into(self.try_query(key.target)?);
            Ok(self
                .query(Transform::new(area, key.axis, key.intended_bounds))
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
            &Query::new(self, command.target).into_segmented(WEST),
            command.intended_bounds,
            Positive,
        );

        let details = Query::new(self, command).details().unwrap();
        if let Some(negative_area) = details.part(Negative) {
            let area = Query::new(self, negative_area.area());
            let mut new_bounds = area.bounds()[command.axis];
            new_bounds[Positive] = command.intended_bounds[Negative] - 1;
            let side_line = *area.into_side(Cardinal::new(command.axis, Positive));
            self.transform_area(negative_area, new_bounds, Negative);
        }
        if let Some(positive_area) = details.part(Negative) {
            let area = Query::new(self, positive_area.area());
            let mut new_bounds = area.bounds()[command.axis];
            new_bounds[Positive] = command.intended_bounds[Positive] + 1;
            let side_line = *area.into_side(Cardinal::new(command.axis, Negative));
            self.transform_area(positive_area, new_bounds, Positive);
        }

        let cross_neg =
            *Query::new(self, details.area()).into_side(Cardinal::new(!command.axis, Negative));

        let cross_pos =
            *Query::new(self, details.area()).into_side(Cardinal::new(!command.axis, Positive));
        self.update_connections(cross_neg);
        self.update_connections(cross_pos);
    }
}
