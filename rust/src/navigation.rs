use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{Axis, Cardinal, Sign, EAST, WEST};
use crate::graph::{Graph, TileKind};
use std::any::Any;
use std::iter::{Chain, Map, Once, Scan, TakeWhile};
use std::ops::{Deref, DerefMut, Not};
use std::sync::atomic::Ordering::SeqCst;
use uuid::Uuid;
use crate::graph;
use crate::graph::TileKind::{Surface, Window};

// todo: this file is way overcomplicated from experimentation

/// A cursor is a position on the side of a window.
/// they provide the step function for navigating along the gutter of other cursor positions parallel to them.
#[derive(Copy, Clone, Debug)]
pub struct Cursor {
    pub tile_id: Uuid,
    pub side: Cardinal,
}

impl Cursor {
    pub fn new(tile_id: Uuid, side: Cardinal) -> Self {
        Self { tile_id, side }
    }

    pub fn iter(self, graph: &Graph, direction: Sign) -> GutterIter {
        GutterIter::new(graph, self, direction)
    }
    
    pub fn face(mut self, side: Cardinal) -> Self {
        self.side = side;
        self
    }
}

#[derive(Copy, Clone, Debug)]
// A range within a parallel gutter of windows
pub struct Gutter {
    pub start_id: Uuid,
    pub end_id: Uuid,
    pub side: Cardinal,
}

impl Gutter {
    pub fn new(start_id: Uuid, end_id: Uuid, side: Cardinal) -> Self {
        Self { start_id, end_id, side }
    }
    pub fn start(&self) -> Cursor {
        Cursor::new(self.start_id, self.side)
    }


    pub fn end(&self) -> Cursor {
        Cursor::new(self.start_id, self.side)
    }

    /// note: a and b must be supplied in order along the cross axis of the given side or this fails
    pub fn is_connected(&self, graph: &Graph) -> bool {
        if graph.tiles[&self.start_id].kind == Surface {
            false
        } else {
            let nav = GutterIter::new(graph, self.start(), Positive);
            let location = nav.location();
            nav
                .take_while(|&cursor| graph.location(cursor) == location)
                .ids()
                .any(|id| id == self.end_id)
        }
    }

    pub fn are_connected(graph: &Graph, start_id: Uuid, end_id: Uuid, side: Cardinal) -> bool {
        Self { start_id, end_id, side }.is_connected(graph)
    }

    pub fn next(graph: &Graph, cursor: Cursor, direction: Sign) -> Option<Cursor> {
        let location = graph.tiles[&cursor.tile_id]
            .rect
            .get_side_location(cursor.side);
        let search_direction = Cardinal {
            axis: !cursor.side.axis,
            sign: direction,
        };
        let result = graph.intra_surface_neighbourhoods[&cursor.tile_id][search_direction]
            .iter()
            .find(|id| {
                let tile = graph.tiles[id];
                tile.kind == TileKind::Window
                    && tile.rect.get_side_location(cursor.side) == location
            });
        if let Some(&id) = result {
            Some(Cursor::new(id, cursor.side))
        } else {
            None
        }
    }
}

// /// A focus nav does it's best to stick to facilitate navigating windows in a stable and minimal fashion.
// /// Initially, it will stick to any gutter it is already in if possible for that direction.
// /// Otherwise, it will look for the neighbour in the given direction that is closest (via the center point).
// /// If that neighbour is the containing surface, it will try to move to the next surface over. If that succeeds, it will try to check for a similarly nearest window inside that surface.
// /// When moving outside of a gutter, the resulting cursor will always be on the side facing opposite the requested direction.
// ///The preserves the axis of operations and gives you the best chance to move along any corresponding gutter.
// /// todo: note: actually it really does still need memory because differences in size may be off, so generalising iter serves no purpose
// pub struct Focus;
// impl NavigationStrategy for Focus {
//     type Direction = Cardinal;
//     fn next(&mut self, graph: &Graph, cursor: Cursor, direction: Self::Direction) -> Option<Cursor> {
//         if direction.axis != cursor.side.axis
//             && let Some(cursor) = Gutter::next(graph, cursor, direction.sign)
//         {
//             Some(cursor)
//         } else {
//             let mut candidates = &graph.intra_surface_neighbourhoods[&cursor.tile_id][direction];
//             if candidates.len() == 1 && graph.tiles[&candidates[0]].kind == TileKind::Surface {
//                 let other_canidates = &graph.intra_surface_neighbourhoods[&candidates[0]][direction];
//                 if other_canidates.iter().any(|id| graph.tiles[id].kind != TileKind::Surface) {
//                     candidates = &other_canidates;
//                 } else {
//                     return Some(Cursor::new(candidates[0], !direction));
//                 }
//             }
//
//             let axis = !direction.axis;
//             let center = graph.tiles[&cursor.tile_id]
//                 .rect
//                 .get_center(axis);
//
//             let nearest = candidates
//                 .iter()
//                 .min_by(|a, b| f32::total_cmp(&f32::abs(graph.tiles[a].rect.get_center(axis) - center), &f32::abs(graph.tiles[b].rect.get_center(axis) - center)));
//             if let Some(&id) = nearest {
//                 Some(Cursor::new(id, !direction))
//             } else {
//                 None
//             }
//         }
//     }
// }

// A signed cursor is effectively iterable in the appropriate direction.
#[derive(Copy, Clone, Debug)]
pub struct GutterIter<'a>
{
    pub graph: &'a Graph,
    pub cursor: Cursor,
    pub direction: Sign,
}

impl<'a> GutterIter<'a>
{
    pub fn new(graph: &'a Graph, cursor: Cursor, direction: Sign) -> Self {
        Self { graph, cursor, direction }
    }

    pub fn flipped(mut self) -> Self {
        self.flip();
        self
    }
    pub fn flip(&mut self) -> &mut Self {
        self.direction = !self.direction;
        self
    }


    /// an iterator adaptor that yields the current cursor before moving on to others (generally it would be skipped by default)
    pub fn inclusive(self) -> Chain<Once<Cursor>, Self>
    {
        std::iter::once(self.cursor).chain(self)
    }

    pub fn location(&self) -> i32 {
        self.graph.location(self.cursor)
    }
}

impl Iterator for GutterIter<'_>
{
    type Item = Cursor;

    fn next(&mut self) -> Option<Self::Item> {
        let result = Gutter::next(self.graph, self.cursor, self.direction);
        if result.is_some() {
            self.cursor = result?;
        }
        result
    }
}

pub trait IterExt where Self : Iterator<Item=Cursor> {

    fn ids(self) -> Map<Self, fn(Cursor) -> Uuid> where Self: Sized {
        self.map(|cursor| cursor.tile_id)
    }
}

impl <T> IterExt for T where T : Iterator<Item=Cursor> {
}