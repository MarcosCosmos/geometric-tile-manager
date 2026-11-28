use crate::area::{Area, AreaQuery};
use crate::caret::Caret;
use crate::command::add_surface::AddSurface;
use crate::command::transform_area::TransformError;
use crate::command::transform_area::{Transform, TransformInformation, TransformSurroundings};
use crate::command::{Command, RawCommandExecutor};
use crate::context::QueryContext;
use crate::error::{KeyedError, KeyedSnafu, QueryError};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{AreaBounds, AxisBounds, Cardinal, EAST, NORTH, SOUTH, WEST};
use crate::graph::{Graph, NodeId};
use crate::iter::IterativeQueryNavigation;
use crate::key::QueryKey;
use crate::line::{Line, LineQuery};
use crate::neighbours::NeighbourTiles;
use crate::node::NodeQuery;
use crate::query::{
    ElementInformation, ElementKind, GraphQuery, GroupInformation, Navigation, Query,
    Standardisation,
};
use crate::query::{ElementInformation1D, ElementInformation2D};
use geotm_macros::QueryKey;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use std::fmt::{Debug, Display, Formatter, write};

// todo: expand the concept of the guide to include both a line and an axis bounds?
/// A simple selection for injecting a new tile along a line.
/// It always results in space being subtracted from each of the selection's areas.
/// It has the loosest requirements: only that there is some non-zero area in the row/column around the line.
/// Unlike other selections, it will never expand the selection line.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey, Command)]
pub struct AddTile<K = Line, B = AxisBounds>
where
    K: QueryKey,
    B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned + Display,
{
    #[serde(bound(deserialize = "K: QueryKey"))]
    pub destination: K,

    #[serde(bound(deserialize = "B: 'static"))]
    pub intended_bounds: B,
}

impl<K, B> Display for AddTile<K, B>
where
    K: QueryKey,
    B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned + Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "AddTile({},{})", self.destination, self.intended_bounds)
    }
}

impl<K, B> AddTile<K, B>
where
    K: QueryKey,
    B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + Display + DeserializeOwned,
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
    B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + Display + DeserializeOwned,
{
    fn kind(&self) -> ElementKind {
        self.to(self.destination).kind()
    }
}

impl<B> ElementInformation2D for Query<'_, AddTile<Line, B>>
where
    B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + Display + DeserializeOwned,
{
    fn bounds(&self) -> AreaBounds {
        self.to(self.destination).bounds().into()
    }
}

impl<B> TransformInformation for Query<'_, AddTile<Line, B>>
where
    B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + Display + DeserializeOwned,
{
    fn surroundings(&self) -> Option<TransformSurroundings> {
        let destination = self.to(self.destination);
        let mut maybe_neg = destination
            .to_area(|iter| iter.last())
            .map(|area| area.to_segmented(destination.side.axis));

        // we don't need to force the opposing line here
        let mut maybe_pos = destination
            .to_opposing()
            .and_then(|opposing_guide| opposing_guide.to_area(|iter| iter.last()))
            .map(|area| area.to_segmented(destination.side.axis));

        if self.destination.side.sign == Positive {
            std::mem::swap(&mut maybe_neg, &mut maybe_pos);
        }

        TransformSurroundings::new(maybe_neg, maybe_pos)
    }
}

impl Standardisation for Query<'_, AddTile> {
    type Error = TransformError;
    fn to_validated(self) -> Result<Self, TransformError> {
        let destination = self.to(self.destination);
        if !destination.is_connected() {
            Err(TransformError::NotConnected)?
        }
        if self.kind() == ElementKind::Surface && destination.peek(!destination.side.sign).is_some()
        {
            Err(TransformError::InvalidTarget)?
        }

        for sign in [Negative, Positive] {
            let start = if sign != destination.side.sign {
                Some(destination)
            } else {
                let res = destination
                    .to_opposing()
                    .or_else(|| destination.to_sentinel());

                println!(
                    "{:?}, {:?}, {:?}",
                    destination.to_opposing().map(|l| *l),
                    destination.to_sentinel().map(|l| *l),
                    res.map(|l| *l)
                );
                res
            };
            if let Some(start) = start {
                match start.kind() {
                    ElementKind::Surface => {
                        if self.intended_bounds[destination.side.sign]
                            != start.bounds().side_location
                        {
                            Err(TransformError::IncompatibleSurroundings)?
                        }
                    }
                    ElementKind::Tile => {
                        if let Some(end) = start.to_iter(!sign).inclusive().last() {
                            // debug_assert_eq!(end.side, !start.side);
                            let end_location = end.bounds().side_location;
                            match sign {
                                Negative if self.intended_bounds[sign] <= end_location => {}
                                Positive if self.intended_bounds[sign] >= end_location => {}
                                _ => Err(TransformError::InsufficientSpace)?,
                            }
                        } else {
                            Err(TransformError::IncompatibleSurroundings)?
                        }
                    }
                    _ => {
                        println!("{}", start.kind());
                        panic!(
                            "Invalid program state: a mixed line should not have arisen from internal logic."
                        )
                    }
                }
            }
        }
        Ok(self)
    }

    fn to_normalized(mut self) -> Self {
        let destination = self.to(self.destination);
        if self.kind() == ElementKind::Surface
            && let Some(tile_line) = destination.peek(!destination.side.sign)
        {
            self.destination = tile_line;
        } else if let Some(opposable) = destination.to_opposable() {
            self.destination = *opposable;
        }

        self
    }
}

impl<K> QueryContext<AddTile<K>> for Graph
where
    K: QueryKey,
    Self: QueryContext<K>,
    for<'a> Query<'a, <Self as QueryContext<K>>::OutKey>: Into<Query<'a, Line>>,
{
    type OutKey = AddTile;

    type ErrorDetail = QueryError<K, <Self as QueryContext<K>>::ErrorDetail, TransformError>;

    fn resolve(
        &self,
        key: AddTile<K>,
    ) -> Result<Query<'_, Self::OutKey>, KeyedError<AddTile<K>, Self::ErrorDetail>> {
        (|| {
            let line: Line = *Into::<Query<Line>>::into(self.resolve(key.destination)?);
            Ok(Query::new(self, AddTile::new(line, key.intended_bounds))
                .to_normalized()
                .to_validated()?)
        })()
        .with_context(|_| KeyedSnafu { key })
    }
}

impl RawCommandExecutor<AddTile> for Graph {
    type Response = NodeId;

    fn execute(&mut self, command: AddTile) -> Self::Response {
        let id = NodeId::new();
        let surface_id = *Query::new(self, command.destination[Negative]).to_surface();
        let destination = Query::new(self, command.destination);
        let original_opposing = destination
            .to_opposing()
            .or_else(|| destination.to_sentinel())
            .map(|query| *query);
        let initial_bounds = Query::new(self, command).bounds();
        // move most of this into transform's execution
        self.containing_surfaces.insert(id, surface_id);
        self.tile_neighbourhoods
            .insert(id, crate::graph::NeighbourhoodGroup::new());
        self.bounds.insert(id, initial_bounds);

        if command.destination[Negative] == surface_id
            && [NORTH, SOUTH, EAST, WEST].iter().all(|dir| {
                Query::new(self, NeighbourTiles(Caret::new(surface_id, *dir)))
                    .into_iter()
                    .next()
                    .is_none()
            })
        {
            for dir in [NORTH, SOUTH, EAST, WEST] {
                self.replace_connections(
                    Caret::new(surface_id, dir).into(),
                    Caret::new(id, dir).into(),
                )
            }
        } else {
            println!("{:?}", original_opposing);
            self.replace_connections(
                command.destination,
                Line::new(id, id, !command.destination.side),
            );
            if let Some(opposing) = original_opposing {
                self.replace_connections(opposing, Line::new(id, id, command.destination.side));
            }
        }

        println!("{}", serde_json::to_string_pretty(&self).unwrap());

        self.execute(Transform::new(
            Area::from(id),
            command.destination.side.axis,
            command.intended_bounds,
        ));

        id
    }
}
