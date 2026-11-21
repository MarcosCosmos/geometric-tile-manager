use crate::area::{Area, AreaQuery};
use crate::command::transform::{TransformDetails, TransformInformation};
use crate::command::transform::{TransformKey, TransformProblem, TransformQuery};
use crate::command::{Command, CommandExecutor, RawCommandExecutor};
use crate::error::{KeyedError, KeyedSnafu, QueryError};
use crate::geometry::Sign::{Negative, Positive};
use crate::graph::{Graph, NodeId};
use crate::key::QueryKey;
use crate::line::{Line, LineQuery};
use crate::node::NodeQuery;
use crate::query::{ElementInformation, ElementKind, GroupInformation, Navigation, Query};
use crate::source::{QuerySource, RawQuerySource};
use markers_derive::{QueryKey, TransformKey};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use std::ops::Deref;

// todo: expand the concept of the guide to include both a line and an axis bounds?
/// A simple selection for injecting a new tile along a line.
/// It always results in space being subtracted from each of the selection's areas.
/// It has the loosest requirements: only that there is some non-zero area in the row/column around the line.
/// Unlike other selections, it will never expand the selection line.
#[derive(
    Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey, TransformKey, Command,
)]
#[node(<Key as QueryKey>::NodeKey)]
pub struct AddTile<Key = Line>(pub Key)
where
    Key: QueryKey + Into<Line<<Key as QueryKey>::NodeKey>>;

impl<Key> Deref for AddTile<Key>
where
    Key: QueryKey + Into<Line<<Key as QueryKey>::NodeKey>>,
{
    type Target = Key;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ElementInformation for Query<'_, AddTile> {
    fn kind(&self) -> ElementKind {
        self.context.query(***self).kind()
    }
}

impl TransformInformation for Query<'_, AddTile> {
    fn details(&self) -> TransformDetails {
        let guide_line = self.context.query(***self);
        let surface_area = Query::<Area>::from(
            Query::<NodeId>::from(guide_line.into_end(Negative)).into_surface_of(),
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
        if self.context.query(**self).is_connected() {
            Ok(self)
        } else {
            Err(TransformProblem::NotConnected)
        }
    }

    fn into_normalized(mut self) -> Self {
        if self.kind() == ElementKind::Surface && let Some(tile_line) = self.context.query(**self).peek(!self.side.sign) {
            self.key.0 = tile_line;
        }
        self
    }
}

impl<Key, T> QuerySource<AddTile<Key>> for T
where
    Key: QueryKey + Into<Line<<Key as QueryKey>::NodeKey>>,
    T: RawQuerySource + QuerySource<Key, Error: 'static>,
    for<'a> Query<'a, T::OutKey>: Into<Query<'a, Line>>,
    QueryError<Key, T::Error, TransformProblem>: From<T::Error>,
{
    type OutKey = AddTile;

    type Error = KeyedError<AddTile<Key>, QueryError<Key, T::Error, TransformProblem>>;

    fn try_query(&self, key: AddTile<Key>) -> Result<Query<Self::OutKey>, Self::Error> {
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
    type Response = ();

    fn execute(&mut self, command: AddTile) -> Self::Response {
        // impl method in graph that internally calls this query to get the transform details? maybe, or impl here.
    }
}