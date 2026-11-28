use crate::caret::Caret;
use crate::command::transform::SegmentedArea;
use crate::context::QueryContext;
use crate::error::{KeyedError, KeyedSnafu, QueryError, ValidationError};
use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{
    AreaBounds, Axis, AxisBounds, Cardinal, Diagonal, EAST, NORTH, NORTH_EAST, NORTH_WEST, SOUTH,
    SOUTH_EAST, SOUTH_WEST, WEST,
};
use crate::graph::{Graph, NodeId};
use crate::iter::IterativeQueryNavigation;
use crate::key::{NodeKey, QueryKey, query_impl};
use crate::line::{Line, LineQuery};
use crate::node::NodeQuery;
use crate::query::ElementKind::Surface;
use crate::query::{
    ElementInformation, ElementInformation1D, ElementInformation2D, ElementKind, GraphQuery,
    GroupInformation, Query, Standardisation,
};
use geotm_macros::{Encode, QueryKey, ValidationError};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Index, IndexMut};

/// Generally dividers will be clean lines that don't intersect any tiles,
/// but the start or end may or may not result in cutting through some tiles
/// Note: it is possible for a transform container to start and end with the same pair of ids and have no dividers.
///     in this case it is a single row/column or even single tile.
#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize, QueryKey, Encode)]
#[encode("[{}..{}..{}..{}]")]
pub struct Area<Node = NodeId>
where
    Node: NodeKey,
{
    #[serde(bound(deserialize = "Node: NodeKey"))]
    north_west: Node,
    #[serde(bound(deserialize = "Node: NodeKey"))]
    north_east: Node,
    #[serde(bound(deserialize = "Node: NodeKey"))]
    south_west: Node,
    #[serde(bound(deserialize = "Node: NodeKey"))]
    south_east: Node,
}

impl<Node> From<Node> for Area<Node>
where
    Node: NodeKey,
{
    fn from(value: Node) -> Self {
        Area {
            north_west: value,
            north_east: value,
            south_west: value,
            south_east: value,
        }
    }
}

query_impl! { From<NodeId> for Area }

impl<Node> Area<Node>
where
    Node: NodeKey,
{
    pub fn new(north_west: Node, north_east: Node, south_west: Node, south_east: Node) -> Self {
        Self {
            north_west,
            north_east,
            south_west,
            south_east,
        }
    }
}

impl<Node> Index<Diagonal> for Area<Node>
where
    Node: NodeKey,
{
    type Output = Node;

    fn index(&self, index: Diagonal) -> &Self::Output {
        match index {
            Diagonal {
                horizontal: Negative,
                vertical: Negative,
            } => &self.north_west,
            Diagonal {
                horizontal: Negative,
                vertical: Positive,
            } => &self.north_east,
            Diagonal {
                horizontal: Positive,
                vertical: Negative,
            } => &self.south_west,
            Diagonal {
                horizontal: Positive,
                vertical: Positive,
            } => &self.south_east,
        }
    }
}

impl<Node> IndexMut<Diagonal> for Area<Node>
where
    Node: NodeKey,
{
    fn index_mut(&mut self, index: Diagonal) -> &mut Self::Output {
        match index {
            Diagonal {
                horizontal: Negative,
                vertical: Negative,
            } => &mut self.north_west,
            Diagonal {
                horizontal: Negative,
                vertical: Positive,
            } => &mut self.north_east,
            Diagonal {
                horizontal: Positive,
                vertical: Negative,
            } => &mut self.south_west,
            Diagonal {
                horizontal: Positive,
                vertical: Positive,
            } => &mut self.south_east,
        }
    }
}

pub trait AreaQuery: GraphQuery + ElementInformation2D {
    fn to_side(self, side: Cardinal) -> Self::Query<Line>;
    fn to_corner(self, corner: Diagonal) -> Self::Query<NodeId>;

    fn to_segmented(self, axis: Axis) -> SegmentedArea {
        SegmentedArea::new(self.to_segments(axis).collect(), axis)
    }

    fn to_segments(self, axis: Axis) -> impl Iterator<Item = Area>;
    /// Constrain an area by updating one of its sides with a line.
    /// Useful for constraining selection areas e.g. delete selections cannot include the deleted tile, and generally need to start from a given line.
    /// Note: the resulting area may become invalid. If not pre-asserted, check with into_validated!
    fn with_side(self, constraint: Line) -> Self;
}

impl ElementInformation for Query<'_, Area> {
    fn kind(&self) -> ElementKind {
        match (
            self.to(Caret::new(self.north_west, NORTH)).kind(),
            self.to(Caret::new(self.north_east, NORTH)).kind(),
            self.to(Caret::new(self.south_east, NORTH)).kind(),
            self.to(Caret::new(self.south_west, NORTH)).kind(),
        ) {
            (ElementKind::Tile, ElementKind::Tile, ElementKind::Tile, ElementKind::Tile) => {
                ElementKind::Tile
            }
            (ElementKind::Surface, ElementKind::Surface, ElementKind::Tile, ElementKind::Tile) => {
                ElementKind::Surface
            }
            _ => ElementKind::Mixed,
        }
    }
}

impl ElementInformation2D for Query<'_, Area> {
    fn bounds(&self) -> AreaBounds {
        AreaBounds::new(
            AxisBounds::new(
                self.to_side(WEST).bounds().side_location,
                self.to_side(EAST).bounds().side_location,
            ),
            AxisBounds::new(
                self.to_side(NORTH).bounds().side_location,
                self.to_side(SOUTH).bounds().side_location,
            ),
        )
    }
}

impl<'a> GroupInformation for Query<'a, Area> {
    fn is_connected(&self) -> bool {
        self.to_side(NORTH).is_connected()
            && self.to_side(EAST).is_connected()
            && self.to_side(SOUTH).is_connected()
            && self.to_side(WEST).is_connected()
    }
}

#[derive(Debug, Snafu, ValidationError)]
pub enum AreaError {
    #[snafu(display("An Area cannot span multiple surfaces"))]
    MultipleSurfaces,
    #[snafu(display("The following sides were in the wrong order: {bad_sides:?}"))]
    WrongOrder { bad_sides: Vec<Cardinal> },
}

impl<'a> AreaQuery for Query<'a, Area> {
    fn to_side(self, side: Cardinal) -> Self::Query<Line> {
        self.to(match side {
            Cardinal {
                axis: Horizontal,
                sign: Negative,
            } => Line::new(self.north_west, self.south_west, side),
            Cardinal {
                axis: Horizontal,
                sign: Positive,
            } => Line::new(self.north_east, self.south_east, side),
            Cardinal {
                axis: Vertical,
                sign: Negative,
            } => Line::new(self.north_east, self.north_west, side),
            Cardinal {
                axis: Vertical,
                sign: Positive,
            } => Line::new(self.south_east, self.south_west, side),
        })
    }

    fn to_corner(self, corner: Diagonal) -> Self::Query<NodeId> {
        self.to(self[corner])
    }

    /// Constrain an area by updating one of its sides with a line.
    /// Useful for constraining selection areas e.g. delete selections cannot include the deleted tile, and generally need to start from a given line.
    /// Note: the resulting area may become invalid. If not pre-asserted, check with into_validated!
    fn with_side(mut self, side: Line) -> Self {
        match side.side {
            Cardinal {
                axis: Horizontal,
                sign: Negative,
            } => {
                self[NORTH_WEST] = side[Negative];
                self[SOUTH_WEST] = side[Positive];
            }
            Cardinal {
                axis: Horizontal,
                sign: Positive,
            } => {
                self[NORTH_EAST] = side[Negative];
                self[SOUTH_EAST] = side[Positive];
            }
            Cardinal {
                axis: Vertical,
                sign: Negative,
            } => {
                self[NORTH_WEST] = side[Negative];
                self[NORTH_EAST] = side[Positive];
            }
            Cardinal {
                axis: Vertical,
                sign: Positive,
            } => {
                self[SOUTH_WEST] = side[Negative];
                self[SOUTH_EAST] = side[Positive];
            }
        }
        self
    }

    fn to_segments(self, axis: Axis) -> impl Iterator<Item = Self::Target> {
        debug_assert!(self.is_connected());

        let start_side = Cardinal::new(axis, Negative);
        let end_location = self.to_side(!start_side).bounds().side_location;

        let terminal_ordering = match start_side.sign {
            Negative => Ordering::Greater,
            Positive => Ordering::Less,
        };

        let mut iter = self
            .to_side(start_side)
            .to_iter(!start_side.sign)
            .inclusive()
            .take_while(move |line| {
                line.to_end(Negative)
                    .bounds()
                    .side_location
                    .cmp(&end_location)
                    != terminal_ordering
                    && line
                        .to_end(Positive)
                        .bounds()
                        .side_location
                        .cmp(&end_location)
                        != terminal_ordering
            });

        std::iter::from_fn(move || {
            if let (Some(each_start), Some(each_end)) = (iter.next(), iter.next()) {
                Some(match axis {
                    Horizontal => Area {
                        north_west: each_start[Negative],
                        north_east: each_end[Negative],
                        south_east: each_end[Positive],
                        south_west: each_start[Positive],
                    },
                    Vertical => Area {
                        north_west: each_start[Negative],
                        north_east: each_start[Positive],
                        south_west: each_end[Negative],
                        south_east: each_end[Positive],
                    },
                })
            } else {
                None
            }
        })
    }
}

impl Standardisation for Query<'_, Area> {
    type Error = AreaError;

    fn to_validated(self) -> Result<Self, AreaError> {
        if [NORTH_WEST, NORTH_EAST, SOUTH_EAST, SOUTH_WEST]
            .map(|diagonal| *self.to_corner(diagonal).to_surface())
            .iter()
            .collect::<HashSet<_>>()
            .len()
            != 1
        {
            Err(AreaError::MultipleSurfaces)
        } else {
            let bad_sides: Vec<Cardinal> = [WEST, NORTH, EAST, SOUTH]
                .into_iter()
                .filter(|&side| self.to_side(side).to_validated().is_err())
                .collect();
            if !bad_sides.is_empty() {
                Err(AreaError::WrongOrder { bad_sides })
            } else {
                Ok(self)
            }
        }
    }
}

impl<Node> QueryContext<Area<Node>> for Graph
where
    Self: QueryContext<Node, OutKey = NodeId>,
    Node: NodeKey,
{
    type OutKey = Area;

    type ErrorDetail = QueryError<Node, <Self as QueryContext<Node>>::ErrorDetail, AreaError>;

    fn resolve(
        &self,
        key: Area<Node>,
    ) -> Result<Query<'_, Self::OutKey>, KeyedError<Area<Node>, Self::ErrorDetail>> {
        (|| {
            Ok(Query::new(
                self,
                Area {
                    north_west: *self.resolve(key.north_west)?,
                    north_east: *self.resolve(key.north_east)?,
                    south_east: *self.resolve(key.south_east)?,
                    south_west: *self.resolve(key.south_west)?,
                },
            )
            .to_validated()?)
        })()
        .with_context(|_| KeyedSnafu { key })
    }
}
