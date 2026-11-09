use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{Cardinal, EAST, Sign, WEST};
use crate::graph::{Graph, TileKind};
use std::any::Any;
use std::ops::{Deref, DerefMut, Not};
use std::sync::atomic::Ordering::SeqCst;
use uuid::Uuid;

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

    pub fn nav<'a, T>(self, graph: &'a Graph) -> T where T: From<Nav<'a>> {
        Nav {
            graph,
            cursor: self,
        }.into()
    }
}


/// a nav is basically just a cursor that can be moved directionally
#[derive(Copy, Clone, Debug)]
pub struct Nav<'a> {
    pub graph: &'a Graph,
    pub cursor: Cursor,
}

impl<'a> Nav<'a> {
    pub fn new(graph: &'a Graph, tile_id: Uuid, side: Cardinal) -> Self {
        Cursor { tile_id, side }.nav(graph)
    }
}

pub trait Navigator {
    type Direction: Copy + Not<Output = Self::Direction>;
    type Item;

    fn next(&mut self, direction: Self::Direction) -> Option<Self::Item>;
    fn new<'a>(graph: &'a Graph, tile_id: Uuid, side: Cardinal) -> Self
    where
        Self: From<Nav<'a>>,
    {
        Nav::new(graph, tile_id, side).into()
    }

    fn iter(self, direction: Self::Direction) -> NavIter<Self>
    where
        Self: Sized,
    {
        NavIter { inner: self, direction }
    }
}

pub trait RawNavigator where Self : Navigator {
    fn next_raw(graph: &Graph, tile_id: Uuid, side: Cardinal, direction: Self::Direction) -> Option<Self::Item>;

    fn peek(nav: &Nav, direction: Self::Direction) -> Option<Self::Item> {
        Self::next_raw(nav.graph, nav.cursor.tile_id, nav.cursor.side, direction)
    }
}

// A signed cursor is effectively iterable in the appropriate direction.
#[derive(Copy, Clone, Debug)]
pub struct NavIter<I>
where
    I: Navigator,
{
    pub inner: I,
    pub direction: I::Direction,
}

impl<'a, I> NavIter<I>
where
    I: Navigator
{
    pub fn new(graph: &'a Graph, tile_id: Uuid, side: Cardinal, direction: I::Direction) -> Self
     where I: From<Nav<'a>> {
        Self { inner: I::new(graph, tile_id, side), direction }
    }

    pub fn flipped(mut self) -> Self {
        self.flip();
        self
    }
    pub fn flip(&mut self) -> &mut Self {
        self.direction = !self.direction;
        self
    }
}

impl<I> Iterator for NavIter<I>
where
    I: Navigator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next(self.direction)
    }
}

fn peek_by<F>(graph: &Graph, tile_id: Uuid, side: Cardinal, direction: Cardinal, finder: F) -> Option<Cursor>
    where F: Fn(&[Uuid]) -> Option<&Uuid>
{
    if direction.axis != side.axis
        && let Some(id) = GutterNav::next_raw(graph, tile_id, side, direction.sign)
    {
        Some(Cursor::new(id, side))
    } else {
        let candidate = finder(
            &graph.intra_surface_neighbourhoods[&tile_id][direction],
        )
        .or_else(|| {
            finder(
                &graph.inter_surface_neighbourhoods[&tile_id][direction],
            )
        });
        if let Some(&id) = candidate {
            Some(Cursor::new(id, !direction))
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct GutterNav<'a>(Nav<'a>);

impl<'a> Deref for GutterNav<'a> {
    type Target = Nav<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GutterNav<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl <'a> From<Nav<'a>> for GutterNav<'a> {
    fn from(value: Nav<'a>) -> Self {
        Self(value)
    }
}

impl RawNavigator for GutterNav<'_> {
    fn next_raw(graph: &Graph, tile_id: Uuid, side: Cardinal, direction: Sign) -> Option<Uuid> {
        let location = graph.tiles[&tile_id]
            .rect
            .get_side_location(side);
        let search_direction = Cardinal {
            axis: !side.axis,
            sign: direction,
        };
        graph.intra_surface_neighbourhoods[&tile_id][search_direction]
            .iter()
            .find(|id| {
                let tile = graph.tiles[id];
                tile.kind == TileKind::Window
                    && tile.rect.get_side_location(side) == location
            })
            .copied()
    }
}

impl Navigator for GutterNav<'_> {
    type Direction = Sign;
    type Item = Uuid;
    fn next(&mut self, direction: Sign) -> Option<Uuid> {
        let result = Self::peek(self, direction);
        if let Some(id) = result {
            self.cursor.tile_id = id;
        }
        result
    }
}

#[derive(Copy, Clone, Debug)]
/// Navigates to the first of its neighbours in a direction (if it can't stay in the gutter)
pub struct FirstNav<'a>(Nav<'a>);

impl<'a> Deref for FirstNav<'a> {
    type Target = Nav<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FirstNav<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl <'a> From<Nav<'a>> for FirstNav<'a> {
    fn from(value: Nav<'a>) -> Self {
        Self(value)
    }
}

impl Navigator for FirstNav<'_> {
    type Direction = Cardinal;
    type Item = Cursor;
    fn next(&mut self, direction: Cardinal) -> Option<Cursor> {
        let result = Self::peek(self, direction);
        if let Some(cursor) = result {
            self.cursor = cursor;
        }
        result
    }
}

impl FirstNav<'_> {
    pub fn peek_raw(graph: &Graph, tile_id: Uuid, side: Cardinal, direction: Cardinal) -> Option<Cursor> {
        peek_by(graph, tile_id, side, direction, <[Uuid]>::first)
    }

    pub fn peek(nav: &Nav, direction: Cardinal) -> Option<Cursor> {
        Self::peek_raw(nav.graph, nav.cursor.tile_id, nav.cursor.side, direction)
    }
}

#[derive(Copy, Clone, Debug)]
/// Navigates to the last of its neighbours in a direction (if it can't stay in the gutter)
pub struct LastNav<'a>(Nav<'a>);

impl<'a> Deref for LastNav<'a> {
    type Target = Nav<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LastNav<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl <'a> From<Nav<'a>> for LastNav<'a> {
    fn from(value: Nav<'a>) -> Self {
        Self(value)
    }
}

impl Navigator for LastNav<'_> {
    type Direction = Cardinal;
    type Item = Cursor;
    fn next(&mut self, direction: Cardinal) -> Option<Cursor> {
        let result = Self::peek(self, direction);
        if let Some(cursor) = result {
            self.cursor = cursor;
        }
        result
    }
}

impl RawNavigator for LastNav<'_> {
    fn next_raw(graph: &Graph, tile_id: Uuid, side: Cardinal, direction: Cardinal) -> Option<Cursor> {
        peek_by(graph, tile_id, side, direction, <[Uuid]>::last)
    }
}

#[derive(Copy, Clone, Debug)]
/// Navigates to the nearest of its neighbours in a direction (if it can't stay in the gutter)
pub struct NearestNav<'a>(Nav<'a>);

impl<'a> Deref for NearestNav<'a> {
    type Target = Nav<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for NearestNav<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl <'a> From<Nav<'a>> for NearestNav<'a> {
    fn from(value: Nav<'a>) -> Self {
        Self(value)
    }
}

impl Navigator for NearestNav<'_> {
    type Direction = Cardinal;
    type Item = Cursor;
    fn next(&mut self, direction: Cardinal) -> Option<Cursor> {
        let result = Self::peek(self, direction);
        if let Some(cursor) = result {
            self.cursor = cursor;
        }
        result
    }
}


impl RawNavigator for NearestNav<'_> {
    fn next_raw(graph: &Graph, tile_id: Uuid, side: Cardinal, direction: Cardinal) -> Option<Cursor> {
        let axis = !direction.axis;
        let center = graph.tiles[&tile_id]
            .rect
            .get_center(axis);
        peek_by(graph, tile_id, side, direction, |candidates| {
            candidates
                .iter()
                .min_by_key(|id| i32::abs(graph.tiles[id].rect.get_center(axis) - center))
        })
    }
}