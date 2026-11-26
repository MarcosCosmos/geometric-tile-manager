use crate::caret::Caret;
use crate::command::transform::SegmentedArea;
use crate::context::QueryContext;
use crate::error::{KeyedError, KeyedSnafu, QueryError, ValidationError};
use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{
    AreaBounds, AxisBounds, Cardinal, Diagonal, EAST, NORTH, NORTH_EAST, NORTH_WEST, SOUTH,
    SOUTH_EAST, SOUTH_WEST, WEST,
};
use crate::graph::NodeId;
use crate::iter::IterativeQueryNavigation;
use crate::key::{QueryKey, query_impl};
use crate::line::{Line, LineQuery};
use crate::node::NodeQuery;
use crate::query::{
    ElementInformation, ElementInformation1D, ElementInformation2D, ElementKind, GroupInformation,
    Query,
};
use markers_derive::{QueryKey, ValidationError};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt::Debug;
use std::ops::{Index, IndexMut};

/// Generally dividers will be clean lines that don't intersect any tiles,
/// but the start or end may or may not result in cutting through some tiles
/// Note: it is possible for a transform container to start and end with the same pair of ids and have no dividers.
///     in this case it is a single row/column or even single tile.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey)]
pub struct Area<Node = NodeId>
where
    Node: QueryKey,
{
    north_west: Node,
    north_east: Node,
    south_west: Node,
    south_east: Node,
}

impl<Node> From<Node> for Area<Node>
where
    Node: QueryKey,
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
    Node: QueryKey,
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
    Node: QueryKey,
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
    Node: QueryKey,
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

pub trait AreaQuery: ElementInformation2D {
    type LineQuery: LineQuery;
    type NodeQuery: NodeQuery;
    fn into_side(self, side: Cardinal) -> Self::LineQuery;
    fn into_corner(self, corner: Diagonal) -> Self::NodeQuery;

    fn into_segmented(self, start_side: Cardinal) -> SegmentedArea {
        SegmentedArea::new(self.into_segments(start_side).collect(), start_side.axis)
    }

    fn into_validated(self) -> Result<Self, AreaProblem>
    where
        Self::Target: Debug;
    fn into_segments(self, start_side: Cardinal) -> impl Iterator<Item = Area>;
    /// Constrain an area by updating one of its sides with a line.
    /// Useful for constraining selection areas e.g. delete selections cannot include the deleted tile, and generally need to start from a given line.
    /// Note: the resulting area may become invalid. If not pre-asserted, check with into_validated!
    fn with_constraint(self, constraint: Line) -> Self;
}

impl ElementInformation for Query<'_, Area> {
    fn kind(&self) -> ElementKind {
        match (
            Query::new(self.context, Caret::new(self.north_west, NORTH)).kind(),
            Query::new(self.context, Caret::new(self.north_east, NORTH)).kind(),
            Query::new(self.context, Caret::new(self.south_east, NORTH)).kind(),
            Query::new(self.context, Caret::new(self.south_west, NORTH)).kind(),
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
                self.into_side(WEST).bounds().side_location,
                self.into_side(EAST).bounds().side_location,
            ),
            AxisBounds::new(
                self.into_side(NORTH).bounds().side_location,
                self.into_side(SOUTH).bounds().side_location,
            ),
        )
    }
}

impl<'a> GroupInformation for Query<'a, Area> {
    fn is_connected(&self) -> bool {
        self.into_side(NORTH).is_connected()
            && self.into_side(EAST).is_connected()
            && self.into_side(SOUTH).is_connected()
            && self.into_side(WEST).is_connected()
    }
}

#[derive(Debug, Snafu, ValidationError)]
pub enum AreaProblem {
    #[snafu(display("An Area cannot span multiple surfaces"))]
    MultipleSurfaces,
    #[snafu(display("The following sides were in the wrong order: {bad_sides:?}"))]
    WrongOrder { bad_sides: Vec<Cardinal> },
}

impl<'a> AreaQuery for Query<'a, Area> {
    type LineQuery = Query<'a, Line>;

    type NodeQuery = Query<'a, NodeId>;

    fn into_side(self, side: Cardinal) -> Self::LineQuery {
        Query::new(
            self.context,
            match side {
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
            },
        )
    }

    fn into_corner(self, corner: Diagonal) -> Self::NodeQuery {
        Query::new(self.context, self[corner])
    }

    fn into_validated(self) -> Result<Self, AreaProblem> {
        if [NORTH_WEST, NORTH_EAST, SOUTH_EAST, SOUTH_WEST]
            .map(|diagonal| *self.into_corner(diagonal).into_containing_surface())
            .iter()
            .collect::<HashSet<_>>()
            .len()
            != 1
        {
            Err(AreaProblem::MultipleSurfaces)
        } else {
            let bad_sides: Vec<Cardinal> = [WEST, NORTH, EAST, SOUTH]
                .into_iter()
                .filter(|&side| self.into_side(side).into_validated().is_err())
                .collect();
            if !bad_sides.is_empty() {
                Err(AreaProblem::WrongOrder { bad_sides })
            } else {
                Ok(self)
            }
        }
    }

    /// Constrain an area by updating one of its sides with a line.
    /// Useful for constraining selection areas e.g. delete selections cannot include the deleted tile, and generally need to start from a given line.
    /// Note: the resulting area may become invalid. If not pre-asserted, check with into_validated!
    fn with_constraint(mut self, constraint: Line) -> Self {
        match constraint.side {
            Cardinal {
                axis: Horizontal,
                sign: Negative,
            } => {
                self[NORTH_WEST] = constraint[Negative];
                self[SOUTH_WEST] = constraint[Positive];
            }
            Cardinal {
                axis: Horizontal,
                sign: Positive,
            } => {
                self[NORTH_EAST] = constraint[Negative];
                self[SOUTH_EAST] = constraint[Positive];
            }
            Cardinal {
                axis: Vertical,
                sign: Negative,
            } => {
                self[NORTH_WEST] = constraint[Negative];
                self[NORTH_EAST] = constraint[Positive];
            }
            Cardinal {
                axis: Vertical,
                sign: Positive,
            } => {
                self[SOUTH_WEST] = constraint[Negative];
                self[SOUTH_EAST] = constraint[Positive];
            }
        }
        self
    }

    fn into_segments(self, start_side: Cardinal) -> impl Iterator<Item = Self::Target> {
        let end_location = self.into_side(!start_side).bounds().side_location;

        let terminal_ordering = match start_side.sign {
            Negative => Ordering::Greater,
            Positive => Ordering::Less,
        };

        let mut iter = self
            .into_side(start_side)
            .into_iter(!start_side.sign)
            .include_if(|line| line.kind() == ElementKind::Tile)
            .take_while(move |line| {
                line.into_end(Negative)
                    .bounds()
                    .side_location
                    .cmp(&end_location)
                    != terminal_ordering
                    && line
                        .into_end(Positive)
                        .bounds()
                        .side_location
                        .cmp(&end_location)
                        != terminal_ordering
            });
        std::iter::from_fn(move || {
            if let (Some(each_start), Some(each_end)) = (iter.next(), iter.next()) {
                Some(match start_side {
                    Cardinal {
                        axis: Horizontal,
                        sign: Negative,
                    } => Area {
                        north_west: each_start[Negative],
                        north_east: each_end[Negative],
                        south_east: each_end[Positive],
                        south_west: each_start[Positive],
                    },
                    Cardinal {
                        axis: Horizontal,
                        sign: Positive,
                    } => Area {
                        north_west: each_end[Negative],
                        north_east: each_start[Negative],
                        south_east: each_start[Positive],
                        south_west: each_end[Positive],
                    },
                    Cardinal {
                        axis: Vertical,
                        sign: Positive,
                    } => Area {
                        north_west: each_end[Negative],
                        north_east: each_end[Positive],
                        south_west: each_start[Negative],
                        south_east: each_start[Positive],
                    },
                    Cardinal {
                        axis: Vertical,
                        sign: Negative,
                    } => Area {
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

impl<T, Node> QueryContext<Area<Node>> for T
where
    T: QueryContext<Node, OutKey = NodeId>,
    Node: QueryKey
{
    type OutKey = Area;

    type ErrorDetail = QueryError<Node, T::ErrorDetail, AreaProblem>;

    fn resolve(&self, key: Area<Node>) -> Result<Query<'_, Self::OutKey>, KeyedError<Area<Node>, Self::ErrorDetail>> {
        (|| {
            Ok(Query::new(
                self.as_ref(),
                Area {
                    north_west: *self.resolve(key.north_west)?,
                    north_east: *self.resolve(key.north_east)?,
                    south_east: *self.resolve(key.south_east)?,
                    south_west: *self.resolve(key.south_west)?,
                },
            )
            .into_validated()?)
        })()
        .with_context(|_| KeyedSnafu { key })
    }
}
