use crate::area::{Area, AreaQuery};
use crate::command::transform::{Transform, TransformDetails, TransformInformation};
use crate::command::transform::{TransformError, TransformQuery};
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
use crate::query::{ElementInformation, ElementKind, GroupInformation, Navigation, Query};
use crate::query::{ElementInformation1D, ElementInformation2D};
use markers_derive::QueryKey;
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
#[node(<K as QueryKey>::NodeKey)]
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
        Query::new(self.context, self.destination).kind()
    }
}

impl<B> ElementInformation2D for Query<'_, AddTile<Line, B>>
where
    B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned,
{
    fn bounds(&self) -> AreaBounds {
        Query::new(self.context, self.destination).bounds().into()
    }
}

impl<B> TransformInformation for Query<'_, AddTile<Line, B>>
where
    B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned,
{
    fn details(&self) -> Option<TransformDetails> {
        let destination = Query::new(self.context, self.destination);
        let surface_area = Query::<Area>::from(
            Query::<NodeId>::from(destination.into_end(Negative)).into_containing_surface(),
        );
        let inward = surface_area
            .with_constraint(*destination)
            .into_segmented(!destination.side);

        // we don't need to force the opposing line here
        let maybe_outward = destination.into_opposing().map(|opposing_guide| {
            surface_area
                .with_constraint(*opposing_guide)
                .into_segmented(destination.side)
        });

        // the rest of this is just normalising the above backward/forward parts to collapse intermediate Nones and arranged them in natural order
        if let Some(outward_fragment) = maybe_outward {
            if destination.side.sign == Positive {
                Some(TransformDetails::Bidirectional(inward, outward_fragment))
            } else {
                Some(TransformDetails::Bidirectional(outward_fragment, inward))
            }
        } else {
            if destination.side.sign == Positive {
                Some(TransformDetails::Negative(inward))
            } else {
                Some(TransformDetails::Positive(inward))
            }
        }
    }
}

impl TransformQuery for Query<'_, AddTile> {
    fn into_validated(self) -> Result<Self, TransformError> {
        let destination = Query::new(self.context, self.destination);
        let surface_area = Query::<Area>::from(
            Query::<NodeId>::from(destination.into_end(Negative)).into_containing_surface(),
        );
        if !destination.is_connected() {
            Err(TransformError::NotConnected)?
        }

        if self.kind() == ElementKind::Surface && destination.peek(!destination.side.sign).is_some()
        {
            Err(TransformError::IncompatibleSurroundings)?
        }

        for sign in [Negative, Positive] {
            let surface_side =
                surface_area.into_side(Cardinal::new(self.destination.side.axis, sign));
            let opp = if sign == destination.side.sign {
                Some(destination)
            } else {
                destination.into_opposing().or_else(|| {
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
                        if let Some(end) = start.into_iter(sign).last() {
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
        //     let available_bounds = Query::new(self.context, details.area()).bounds()[self.destination.side.axis];
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

    fn into_normalized(mut self) -> Self {
        let destination = Query::new(self.context, self.destination);
        if self.kind() == ElementKind::Surface
            && let Some(tile_line) = destination.peek(!destination.side.sign)
        {
            self.destination = tile_line;
        }

        if let Some(opposable) = destination.into_opposable() {
            self.destination = *opposable;
        }

        self
    }
}

impl<K, B, T> QueryContext<AddTile<K, B>> for T
where
    K: QueryKey,
    B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned,
    T: QueryContext<K>,
    for<'a> Query<'a, T::OutKey>: Into<Query<'a, Line>>,
    for<'a> Query<'a, AddTile<Line, B>>: TryInto<Query<'a, AddTile>>,
    for<'a> QueryError<K, T::Error, TransformError>:
        From<T::Error> + From<<Query<'a, AddTile<Line, B>> as TryInto<Query<'a, AddTile>>>::Error>,
{
    type OutKey = AddTile;

    type Error = KeyedError<AddTile<K, B>, QueryError<K, T::Error, TransformError>>;

    fn try_query(&self, key: AddTile<K, B>) -> Result<Query<'_, Self::OutKey>, Self::Error> {
        (|| {
            let line: Line = *Into::<Query<Line>>::into(self.try_query(key.destination)?);
            Ok(
                Query::new(self.as_ref(), AddTile::new(line, key.intended_bounds))
                    .try_into()?
                    .into_normalized()
                    .into_validated()?,
            )
        })()
        .with_context(|_| KeyedSnafu { key })
    }
}

/// todo: will probably move extras like this into a utils crate; It will be OK with the orphaning rules as long as Auto is owned in util.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct Auto;
impl<'a> TryInto<Query<'a, AddTile>> for Query<'a, AddTile<Line, Auto>> {
    type Error = TransformError;

    fn try_into(self) -> Result<Query<'a, AddTile>, Self::Error> {
        let destination = Query::new(self.context, self.destination);
        let bounds = if destination.kind() == ElementKind::Surface {
            TryInto::<Query<NodeId>>::try_into(destination)
                .unwrap()
                .bounds()[destination.side.axis]
        } else if let Some(details) = self.details() {
            let (space, segment_count): (i32, i32) =
                details.iter().fold((0, 0), |acc, segmented| {
                    (
                        acc.0
                            + segmented
                                .segments
                                .iter()
                                .map(|&area| {
                                    Query::new(self.context, area).bounds()[destination.side.axis]
                                        .extent()
                                })
                                .sum::<i32>(),
                        acc.1 + (segmented.segments.len() as i32),
                    )
                });

            let half_new_size = (space / (segment_count + 1)) / 2;
            let location = destination.bounds().side_location;
            AxisBounds::new(location - half_new_size, location + half_new_size)
        } else {
            Err(TransformError::IncompatibleSurroundings)?
        };
        Ok(Query::new(
            self.context,
            AddTile {
                destination: self.destination,
                intended_bounds: bounds,
            },
        ))
    }
}

impl RawCommandExecutor<AddTile> for Graph {
    type Response = NodeId;

    fn execute(&mut self, command: AddTile) -> Self::Response {
        let id = NodeId::new();
        let surface_id = *Query::new(self, command.destination[Negative]).into_containing_surface();
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
            .into_opposing()
            .map(|query| *query)
        {
            self.replace_connections(opposing, Line::new(id, id, command.destination.side));
        }

        self.execute(Transform::new(
            Area::from(id),
            command.destination.side.axis,
            command.intended_bounds,
        ));

        id
    }
}
