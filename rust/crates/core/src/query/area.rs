use crate::error::Error;
use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{
    AreaBounds, AxisBounds, Cardinal, Diagonal, EAST, NORTH, NORTH_EAST, NORTH_WEST, SOUTH,
    SOUTH_EAST, SOUTH_WEST, WEST,
};
use crate::graph::NodeId;
use crate::query::caret::Caret;
use crate::query::iter::ElementIteration;
use crate::query::line::{Line, LineLookup};
use crate::query::node::NodeLookup;
use crate::query::selection::SegmentedContainer;
use crate::query::source::RawQuerySource;
use crate::query::{
    ElementInformation, ElementInformation1D, ElementInformation2D, ElementKind, GroupInformation,
    NodeKey, Query, QueryKey,
};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Deref, Index, IndexMut};

/// Generally dividers will be clean lines that don't intersect any tiles,
/// but the start or end may or may not result in cutting through some tiles
/// Note: it is possible for a transform container to start and end with the same pair of ids and have no dividers.
///     in this case it is a single row/column or even single tile.
#[derive(Copy, Clone, Debug, Eq, PartialEq, QueryKey)]
#[query(N)]
pub struct Area<N = NodeId>
where
    N: NodeKey,
{
    pub north_west: N,
    pub north_east: N,
    pub south_west: N,
    pub south_east: N,
}

impl<N> From<N> for Area<N>
where
    N: NodeKey,
{
    fn from(value: N) -> Self {
        Area {
            north_west: value,
            north_east: value,
            south_west: value,
            south_east: value,
        }
    }
}

impl<N> Area<N>
where
    N: NodeKey,
{
    pub fn new(north_west: N, north_east: N, south_west: N, south_east: N) -> Self {
        Self {
            north_west,
            north_east,
            south_west,
            south_east,
        }
    }
}

impl<N> Index<Diagonal> for Area<N>
where
    N: NodeKey,
{
    type Output = N;

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

impl<N> IndexMut<Diagonal> for Area<N>
where
    N: NodeKey,
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

pub trait AreaInformation: ElementInformation2D {
    type LineInformation: LineLookup;
    type NodeInformation: NodeLookup;
    fn into_side(self, side: Cardinal) -> Self::LineInformation;
    fn into_corner(self, corner: Diagonal) -> Self::NodeInformation;

    fn into_segmented(self, start_side: Cardinal) -> SegmentedContainer {
        SegmentedContainer::new(self.into_segments(start_side).collect(), start_side.axis)
    }

    fn into_validated(self) -> Result<Self, Error<Self::Target>>
    where
        Self::Target: Debug;
    fn into_segments(self, start_side: Cardinal) -> impl Iterator<Item = Area>;
}

impl ElementInformation for Query<'_, Area> {
    fn kind(&self) -> ElementKind {
        match (
            self.context
                .query(Caret::new(self.north_west, NORTH))
                .kind(),
            self.context
                .query(Caret::new(self.north_east, NORTH))
                .kind(),
            self.context
                .query(Caret::new(self.south_east, NORTH))
                .kind(),
            self.context
                .query(Caret::new(self.south_west, NORTH))
                .kind(),
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
    type NodeInformation = Query<'a, NodeId>;

    fn is_connected(&self) -> bool {
        self.into_side(NORTH).is_connected()
            && self.into_side(EAST).is_connected()
            && self.into_side(SOUTH).is_connected()
            && self.into_side(WEST).is_connected()
    }
}

impl<'a> AreaInformation for Query<'a, Area> {
    type LineInformation = Query<'a, Line>;

    type NodeInformation = Query<'a, NodeId>;

    fn into_side(self, side: Cardinal) -> Self::LineInformation {
        self.context.query(match side {
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

    fn into_corner(self, corner: Diagonal) -> Self::NodeInformation {
        self.context.query(self[corner])
    }

    fn into_validated(self) -> Result<Self, Error<Self::Target>> {
        if [NORTH_WEST, NORTH_EAST, SOUTH_EAST, SOUTH_WEST]
            .map(|diagonal| *self.into_corner(diagonal).into_surface_of())
            .iter()
            .collect::<HashSet<_>>()
            .len()
            == 1
        {
            Ok(self)
        } else {
            Err(Error::MultipleSurfaces(*self))
        }
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

    // fn into_normalized(mut self) -> Self {
    //     let corners = [
    //         self.context.que(self.north_west),
    //         self.context.que(self.north_east),
    //         self.context.que(self.south_east),
    //         self.context.que(self.south_west),
    //     ];
    //
    //     let outer_bounds = corners
    //         .iter()
    //         .map(|node| node.bounds())
    //         .reduce(|mut acc, each| {
    //             acc[Horizontal] = acc[Horizontal].span(&each[Horizontal]);
    //             acc[Vertical] = acc[Vertical].span(&each[Vertical]);
    //             acc
    //         })
    //         .unwrap();
    //
    //     let west_raw: Vec<_> = corners
    //         .iter()
    //         .filter(|node| node.bounds()[WEST] == outer_bounds[WEST])
    //         .map(|node| *node.deref())
    //         .collect();
    //     let north_raw: Vec<_> = corners
    //         .iter()
    //         .filter(|node| node.bounds()[NORTH] == outer_bounds[NORTH])
    //         .map(|node| *node.deref())
    //         .collect();
    //     let east_raw: Vec<_> = corners
    //         .iter()
    //         .filter(|node| node.bounds()[EAST] == outer_bounds[EAST])
    //         .map(|node| *node.deref())
    //         .collect();
    //     let south_raw: Vec<_> = corners
    //         .iter()
    //         .filter(|node| node.bounds()[SOUTH] == outer_bounds[SOUTH])
    //         .map(|node| *node.deref())
    //         .collect();
    //
    //     let west: HashSet<_> = west_raw.iter().copied().collect();
    //     let north: HashSet<_> = north_raw.iter().copied().collect();
    //     let east: HashSet<_> = east_raw.iter().copied().collect();
    //     let south: HashSet<_> = south_raw.iter().copied().collect();
    //
    //     let mut north_west = &north & &west;
    //     let mut north_east = &north & &east;
    //     let mut south_east = &south & &east;
    //     let mut south_west = &south & &west;
    //
    //     let north_west_essential = &north_west & &(&north_east | &south_west);
    //     let north_east_essential = &north_east & &(&north_west | &south_east);
    //     let south_east_essential = &south_east & &(&north_east | &south_west);
    //     let south_west_essential = &south_east & &(&north_west | &south_east);
    //
    //     north_west = if north_west_essential.len() > 0 {
    //         north_west_essential
    //     } else {
    //         north_west
    //     };
    //     north_east = if north_east_essential.len() > 0 {
    //         north_east_essential
    //     } else {
    //         north_east
    //     };
    //     south_east = if south_east_essential.len() > 0 {
    //         south_east_essential
    //     } else {
    //         south_east
    //     };
    //     south_west = if south_west_essential.len() > 0 {
    //         south_west_essential
    //     } else {
    //         south_west
    //     };
    //
    //     let corners =
    //
    //     self.target.north_west = north_west.into_iter().next().unwrap();
    //     self.target.north_east = north_east.into_iter().next().unwrap();
    //     self.target.south_east = south_east.into_iter().next().unwrap();
    //     self.target.south_west = south_west.into_iter().next().unwrap();
    //
    //     self
    // }
}

impl<'a> From<Query<'a, NodeId>> for Query<'a, Area> {
    fn from(value: Query<'a, NodeId>) -> Self {
        value.context.query((*value).into())
    }
}
