use crate::area::{Area, AreaQuery};
use crate::command::transform::{TransformDetails, TransformInformation};
use crate::command::transform::{TransformProblem, TransformQuery};
use crate::command::{Command, RawCommandExecutor};
use crate::context::QueryContext;
use crate::error::{KeyedError, KeyedSnafu, QueryError};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{AreaBounds, AxisBounds, Cardinal};
use crate::graph::{Graph, NodeId};
use crate::key::QueryKey;
use crate::line::{Line, LineQuery};
use crate::node::NodeQuery;
use crate::query::{ElementInformation, ElementKind, GroupInformation, Navigation, Query};
use crate::query::{ElementInformation1D, ElementInformation2D};
use markers_derive::{QueryKey};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use std::ops::Deref;

// todo: expand the concept of the guide to include both a line and an axis bounds?
/// A simple selection for injecting a new tile along a line.
/// It always results in space being subtracted from each of the selection's areas.
/// It has the loosest requirements: only that there is some non-zero area in the row/column around the line.
/// Unlike other selections, it will never expand the selection line.
#[derive(
    Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey, Command,
)]
#[node(<K as QueryKey>::NodeKey)]
pub struct AddTile<K = Line>(pub K)
where
    K: QueryKey + Into<Line<<K as QueryKey>::NodeKey>>;

impl<K> Deref for AddTile<K>
where
    K: QueryKey + Into<Line<<K as QueryKey>::NodeKey>>,
{
    type Target = K;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ElementInformation for Query<'_, AddTile> {
    fn kind(&self) -> ElementKind {
        Query::new(self.context, ***self).kind()
    }
}

impl TransformInformation for Query<'_, AddTile> {
    fn details(&self) -> TransformDetails {
        let guide_line = Query::new(self.context, ***self);
        let surface_area = Query::<Area>::from(
            Query::<NodeId>::from(guide_line.into_end(Negative)).into_containing_surface(),
        );
        let inward = surface_area
            .with_constraint(*guide_line)
            .into_segmented(!self.side);

        // we don't need to force the opposing line here
        let maybe_outward = guide_line.into_opposing().map(|opposing_guide| {
            surface_area
                .with_constraint(*opposing_guide)
                .into_segmented(self.side)
        });

        // the rest of this is just normalising the above backward/forward parts to collapse intermediate Nones and arranged them in natural order
        if let Some(outward_fragment) = maybe_outward {
            if self.side.sign == Positive {
                TransformDetails::Bidirectional(inward, outward_fragment)
            } else {
                TransformDetails::Bidirectional(outward_fragment, inward)
            }
        } else {
            if self.side.sign == Positive {
                TransformDetails::Negative(inward)
            } else {
                TransformDetails::Positive(inward)
            }
        }
    }
}

impl TransformQuery for Query<'_, AddTile> {
    fn into_validated(self) -> Result<Self, TransformProblem> {
        if Query::new(self.context, **self).is_connected() {
            Ok(self)
        } else {
            Err(TransformProblem::NotConnected)
        }
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

impl<K, T> QueryContext<AddTile<K>> for T
where
    K: QueryKey + Into<Line<<K as QueryKey>::NodeKey>>,
    T: QueryContext<K>,
    for<'a> Query<'a, T::OutKey>: Into<Query<'a, Line>>,
    QueryError<K, T::Error, TransformProblem>: From<T::Error>,
{
    type OutKey = AddTile;

    type Error = KeyedError<AddTile<K>, QueryError<K, T::Error, TransformProblem>>;

    fn try_query(&self, key: AddTile<K>) -> Result<Query<Self::OutKey>, Self::Error> {
        (|| {
            Ok(self
                .query(AddTile(*self.try_query(key.0)?.into()))
                .into_normalized()
                .into_validated()?)
        })()
        .with_context(|_| KeyedSnafu { key })
    }
}

impl RawCommandExecutor<AddTile> for Graph {
    type Response = NodeId;

    fn execute(&mut self, command: AddTile) -> Self::Response {
        let id = NodeId::new();


        let details = Query::new(self, command).details();
        let (mut space, segment_count): (i32, i32) =
            details.iter().fold((0, 0), |acc, segmented| {
                (
                    acc.0
                        + segmented
                            .segments
                            .iter()
                            .map(|&area| Query::new(self, area).bounds()[command.side.axis].len())
                            .sum::<i32>(),
                    acc.1 + (segmented.segments.len() as i32),
                )
            });

        let half_new_size = (space / (segment_count + 1)) / 2;
        let added_bounds = {
            let reference_bounds = Query::new(self, *command).bounds();
            let mut result = AreaBounds::default();
            result[!command.side.axis] = reference_bounds.axis_bounds;
            result[command.side.axis] = AxisBounds::new(
                reference_bounds.side_location - half_new_size,
                reference_bounds.side_location + half_new_size,
            );
            result
        };

        let id = NodeId::new();
        let surface_id = *Query::new(self, command[Negative]).into_containing_surface();
        self.containing_surfaces.insert(id, surface_id);
        self.bounds.insert(id, added_bounds);
        self.tile_neighbourhoods
            .insert(id.into(), crate::graph::NeighbourhoodGroup::new());

        let added_bounds = added_bounds[command.side.axis];

        if let Some(negative_area) = details.part(Negative) {
            let area = Query::new(self, negative_area.area());
            let mut new_bounds = area.bounds()[command.side.axis];
            new_bounds[Positive] = added_bounds[Negative] - 1;
            let side_line = *area.into_side(Cardinal::new(command.side.axis, Positive));
            self.transform_area(negative_area, new_bounds, Negative);
            self.update_connections(side_line, id, id);
        }
        if let Some(positive_area) = details.part(Negative) {
            let area = Query::new(self, positive_area.area());
            let mut new_bounds = area.bounds()[command.side.axis];
            new_bounds[Positive] = added_bounds[Positive] + 1;
            let side_line = *area.into_side(Cardinal::new(command.side.axis, Negative));
            self.transform_area(positive_area, new_bounds, Positive);
            self.update_connections(side_line, id, id);
        }

        let cross_neg = *Query::new(self, details.area())
            .into_side(Cardinal::new(!command.side.axis, Negative));

        let cross_pos = *Query::new(self, details.area())
            .into_side(Cardinal::new(!command.side.axis, Positive));
        self.repair_connections(cross_neg);
        self.repair_connections(cross_pos);

        id
    }
}
