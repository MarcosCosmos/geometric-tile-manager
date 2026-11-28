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
    GroupInformation, Information, Query, Standardisation,
};
use geotm_macros::{QueryKey, ValidationError};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use std::fmt::{Debug, Display, Formatter};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey, Command)]
pub struct Transform<K = Area, B = AxisBounds>
where
    K: QueryKey,
    B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + Display + DeserializeOwned,
{
    #[serde(bound(deserialize = "K: QueryKey"))]
    pub target: K,
    pub axis: Axis,
    #[serde(bound(deserialize = "B: DeserializeOwned"))]
    pub intended_bounds: B,
}

impl<K, B> Transform<K, B>
where
    K: QueryKey,
    B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + Display + DeserializeOwned,
{
    pub fn new(target: K, axis: Axis, intended_bounds: B) -> Self {
        Self {
            target,
            axis,
            intended_bounds,
        }
    }
}

impl<K, B> Display for Transform<K, B>
where
    K: QueryKey,
    B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + Display + DeserializeOwned,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TransformArea({},{},{})",
            self.target, self.axis, self.intended_bounds
        )
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
    pub fn new(neg: Option<SegmentedArea>, pos: Option<SegmentedArea>) -> Option<Self> {
        match (neg, pos) {
            (Some(negative), None) => Some(Self::Negative(negative)),
            (None, Some(positive)) => Some(Self::Positive(positive)),
            (Some(negative), Some(positive)) => Some(Self::Bidirectional(negative, positive)),
            (None, None) => None,
        }
    }
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
    /// Selected a surface when it was non-empty.
    InvalidTarget,
    /// Unable to transform the surrounding area to satisfy the requested bounds
    IncompatibleSurroundings,
    // There is insufficient space available for the requested size
    InsufficientSpace,
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
        let initial_bounds = target.bounds()[self.axis];

        let [maybe_neg, maybe_pos] = [Negative, Positive].map(|sign| {
            if self.intended_bounds[sign] == initial_bounds[sign] {
                None
            } else {
                target
                    .to_side(Cardinal::new(self.axis, sign))
                    .to_opposing()
                    .map(|start| {
                        start
                            .to_area(|iter| iter.last())
                            .unwrap()
                            .to_segmented(self.axis)
                    })
            }
        });

        TransformSurroundings::new(maybe_neg, maybe_pos)
    }
}

impl Standardisation for Query<'_, Transform> {
    type Error = TransformError;

    fn to_normalized(mut self) -> Self {
        let mut target = self.to(self.target);

        let initial_bounds = self.bounds()[self.axis];
        for sign in [Negative, Positive] {
            if self.intended_bounds[sign] != initial_bounds[sign] {
                let direction = Cardinal::new(self.axis, sign);
                // todo: this can probably be improved by outscanning in a more complex way but for now it's not that important.
                if let Some(updated_side) = target.to_side(direction).to_opposable() {
                    target = target.with_side(*updated_side);
                }
            }
        }

        self.target = *target;

        self
    }

    fn to_validated(self) -> Result<Self, TransformError> {
        let target = self.to(self.target);
        if !target.is_connected() {
            Err(TransformError::NotConnected)?
        };

        let initial_bounds = self.bounds()[self.axis];

        for sign in [Negative, Positive] {
            if self.intended_bounds[sign] != initial_bounds[sign] {
                if let Some(start) = target.to_side(Cardinal::new(self.axis, sign)).to_opposing()
                    && let Some(end) = start.to_iter(sign).last()
                {
                    debug_assert_eq!(end.side, !start.side);
                    let end_location = end.bounds().side_location;
                    match sign {
                        Negative if self.intended_bounds[sign] >= end_location => {}
                        Positive if self.intended_bounds[sign] <= end_location => {}
                        _ => Err(TransformError::InsufficientSpace)?,
                    }
                } else {
                    Err(TransformError::IncompatibleSurroundings)?
                }
            }
        }

        Ok(self)
    }
}

impl<K> QueryContext<Transform<K>> for Graph
where
    K: QueryKey,
    Self: QueryContext<K>,
    for<'a> Query<'a, <Self as QueryContext<K>>::OutKey>: Into<Query<'a, Area>>,
{
    type OutKey = Transform;

    type ErrorDetail = QueryError<K, <Self as QueryContext<K>>::ErrorDetail, TransformError>;

    fn resolve(
        &self,
        key: Transform<K>,
    ) -> Result<Query<'_, Self::OutKey>, KeyedError<Transform<K>, Self::ErrorDetail>> {
        (|| {
            let area: Area = *Into::<Query<Area>>::into(self.resolve(key.target)?);
            Ok(
                Query::new(self, Transform::new(area, key.axis, key.intended_bounds))
                    .to_normalized()
                    .to_validated()?,
            )
        })()
        .with_context(|_| KeyedSnafu { key })
    }
}

impl RawCommandExecutor<Transform> for Graph {
    type Response = ();

    fn execute(&mut self, command: Transform) -> Self::Response {
        if let Ok(node) = NodeId::try_from(command.target) {
            self.transform_node(node, command.axis, command.intended_bounds, Positive);
        } else {
            self.transform_area(
                &Query::new(self, command.target).to_segmented(command.axis),
                command.intended_bounds,
                Positive,
            );
        }

        let surrounding_area = if let Some(surroundings) = Query::new(self, command).surroundings()
        {
            if let Some(negative_area) = surroundings.part(Negative) {
                let area = Query::new(self, negative_area.area());
                let mut new_bounds = area.bounds()[command.axis];
                new_bounds[Positive] = command.intended_bounds[Negative] - 1;
                let _side_line = *area.to_side(Cardinal::new(command.axis, Positive));
                self.transform_area(negative_area, new_bounds, Negative);
            }
            if let Some(positive_area) = surroundings.part(Negative) {
                let area = Query::new(self, positive_area.area());
                let mut new_bounds = area.bounds()[command.axis];
                new_bounds[Positive] = command.intended_bounds[Positive] + 1;
                let _side_line = *area.to_side(Cardinal::new(command.axis, Negative));
                self.transform_area(positive_area, new_bounds, Positive);
            }
            surroundings.area()
        } else {
            command.target
        };

        let cross_neg =
            *Query::new(self, surrounding_area).to_side(Cardinal::new(!command.axis, Negative));

        let cross_pos =
            *Query::new(self, surrounding_area).to_side(Cardinal::new(!command.axis, Positive));
        self.update_connections(cross_neg);
        self.update_connections(cross_pos);
    }
}
