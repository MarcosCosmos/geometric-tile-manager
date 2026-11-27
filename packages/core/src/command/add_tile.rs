use crate::area::{Area, AreaQuery};
use crate::command::transform_area::{TransformArea, TransformInformation, TransformSurroundings};
use crate::command::transform_area::{TransformError};
use crate::command::{Command, RawCommandExecutor};
use crate::context::QueryContext;
use crate::error::{KeyedError, KeyedSnafu, QueryError};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{AreaBounds, AxisBounds, Cardinal};
use crate::graph::{Graph, NodeId};
use crate::iter::IterativeQueryNavigation;
use crate::key::QueryKey;
use crate::line::{Line, LineQuery};
use crate::node::NodeQuery;
use crate::query::{ElementInformation, ElementKind, GraphQuery, GroupInformation, Navigation, Query, Standardisation};
use crate::query::{ElementInformation1D, ElementInformation2D};
use geotm_macros::QueryKey;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use std::fmt::Debug;

// todo: expand the concept of the guide to include both a line and an axis bounds?
/// A simple selection for injecting a new tile along a line.
/// It always results in space being subtracted from each of the selection's areas.
/// It has the loosest requirements: only that there is some non-zero area in the row/column around the line.
/// Unlike other selections, it will never expand the selection line.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey, Command)]
pub struct AddTile<K = Line, B = AxisBounds>
where
    K: QueryKey,
    B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned,
{
    pub destination: K,
    pub intended_bounds: B,
}

impl<K, B> AddTile<K, B>
where
    K: QueryKey,
    B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned,
{
    pub fn new(destination: K, bounds: B) -> Self {
        Self {
            destination,
            intended_bounds: bounds,
        }
    }
}

impl<B> ElementInformation for Query<'_, AddTile<Line, B>>
where
    B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned,
{
    fn kind(&self) -> ElementKind {
        self.to(self.destination).kind()
    }
}

impl<B> ElementInformation2D for Query<'_, AddTile<Line, B>>
where
    B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned,
{
    fn bounds(&self) -> AreaBounds {
        self.to(self.destination).bounds().into()
    }
}

impl<B> TransformInformation for Query<'_, AddTile<Line, B>>
where
    B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned,
{
    fn surroundings(&self) -> Option<TransformSurroundings> {
        let destination = self.to(self.destination);
        let surface_area =
            Query::<Area>::from(Query::<NodeId>::from(destination.to_end(Negative)).to_surface());
        let inward = surface_area
            .with_side(*destination)
            .to_segmented(!destination.side);

        // we don't need to force the opposing line here
        let maybe_outward = destination.to_opposing().map(|opposing_guide| {
            surface_area
                .with_side(*opposing_guide)
                .to_segmented(destination.side)
        });

        // the rest of this is just normalising the above backward/forward parts to collapse intermediate Nones and arranged them in natural order
        if let Some(outward_fragment) = maybe_outward {
            if destination.side.sign == Positive {
                Some(TransformSurroundings::Bidirectional(
                    inward,
                    outward_fragment,
                ))
            } else {
                Some(TransformSurroundings::Bidirectional(
                    outward_fragment,
                    inward,
                ))
            }
        } else {
            if destination.side.sign == Positive {
                Some(TransformSurroundings::Negative(inward))
            } else {
                Some(TransformSurroundings::Positive(inward))
            }
        }
    }
}

impl Standardisation for Query<'_, AddTile> {
    type Error = TransformError;
    fn to_validated(self) -> Result<Self, TransformError> {
        let destination = self.to(self.destination);
        if !destination.is_connected() {
            Err(TransformError::NotConnected)?
        }
        let surface_area =
            Query::<Area>::from(Query::<NodeId>::from(destination.to_end(Negative)).to_surface());

        if self.kind() == ElementKind::Surface && destination.peek(!destination.side.sign).is_some()
        {
            Err(TransformError::IncompatibleSurroundings)?
        }

        for sign in [Negative, Positive] {
            let surface_side =
                surface_area.to_line(Cardinal::new(self.destination.side.axis, sign));
            let opp = if sign == destination.side.sign {
                Some(destination)
            } else {
                destination.to_opposing().or_else(|| {
                    if surface_side
                        .peek(!sign)
                        .is_some_and(|neighbour| neighbour == self.destination)
                    {
                        Some(surface_side)
                    } else {
                        None
                    }
                })
            };
            if let Some(start) = opp {
                match start.kind() {
                    ElementKind::Surface => {
                        if self.intended_bounds[sign] != surface_side.bounds().side_location {
                            Err(TransformError::IncompatibleSurroundings)?
                        }
                    }
                    ElementKind::Tile => {
                        if let Some(end) = start.to_iter(sign).last() {
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
                    _ => {
                        panic!("Invalid program state: a mixed line cannot possibly be connected.")
                    }
                }
            }
        }

        // if let Some(details) = self.details() {
        //     let available_bounds = self.to(details.area()).bounds()[self.destination.side.axis];
        //     let alignment = available_bounds.alignment(&self.intended_bounds);
        //     match alignment {
        //         BoundsAlignment::Contains | BoundsAlignment::Parallel if destination.kind() == ElementKind::Tile => {},
        //         BoundsAlignment::Parallel if destination.kind() == ElementKind::Surface &&  => ,
        //         _ => Err(TransformError::InsufficientSpace)?,
        //     }
        // } else {
        //     Err(TransformError::IncompatibleSurroundings)?
        // }

        Ok(self)
    }

    fn to_normalized(mut self) -> Self {
        let destination = self.to(self.destination);
        if self.kind() == ElementKind::Surface
            && let Some(tile_line) = destination.peek(!destination.side.sign)
        {
            self.destination = tile_line;
        }

        if let Some(opposable) = destination.to_opposable() {
            self.destination = *opposable;
        }

        self
    }
}

impl<K> QueryContext<AddTile<K>> for Graph
where
    K: QueryKey,
    Self: QueryContext<K>,
    for<'a> Query<'a, <Self as QueryContext<K>>::OutKey>: Into<Query<'a, Line>>
{
    type OutKey = AddTile;

    type ErrorDetail = QueryError<K, <Self as QueryContext<K>>::ErrorDetail, TransformError>;

    fn resolve(
        &self,
        key: AddTile<K>,
    ) -> Result<Query<'_, Self::OutKey>, KeyedError<AddTile<K>, Self::ErrorDetail>> {
        (|| {
            let line: Line = *Into::<Query<Line>>::into(self.resolve(key.destination)?);
            Ok(
                Query::new(self, AddTile::new(line, key.intended_bounds))
                    .to_normalized()
                    .to_validated()?,
            )
        })()
        .with_context(|_| KeyedSnafu { key })
    }
}

impl RawCommandExecutor<AddTile> for Graph {
    type Response = NodeId;

    fn execute(&mut self, command: AddTile) -> Self::Response {
        let id = NodeId::new();
        let surface_id = *Query::new(self, command.destination[Negative]).to_surface();
        let initial_bounds = Query::new(self, command).bounds();
        // move most of this into transform's execution
        self.tile_neighbourhoods
            .insert(id, crate::graph::NeighbourhoodGroup::new());
        self.containing_surfaces.insert(id, surface_id);
        self.bounds.insert(id, initial_bounds);

        self.replace_connections(
            command.destination,
            Line::new(id, id, !command.destination.side),
        );
        if let Some(opposing) = Query::new(self, command.destination)
            .to_opposing()
            .map(|query| *query)
        {
            self.replace_connections(opposing, Line::new(id, id, command.destination.side));
        }

        self.execute(TransformArea::new(
            Area::from(id),
            command.destination.side.axis,
            command.intended_bounds,
        ));

        id
    }
}
