use crate::geometry::Sign::Positive;
use crate::geometry::{Cardinal, Sign};
use crate::graph::TileKind::Surface;
use crate::graph::{Graph, TileKind};
use crate::navigation::{Caret, Navigator};
use std::iter::Map;
use std::ops::{Deref, DerefMut};
use uuid::Uuid;

#[derive(Copy, Clone, Debug)]
// A range within a parallel gutter of windows
pub struct Gutter {
    pub start_id: Uuid,
    pub end_id: Uuid,
    pub side: Cardinal,
}

impl Gutter {
    pub fn new(start_id: Uuid, end_id: Uuid, side: Cardinal) -> Self {
        Self {
            start_id,
            end_id,
            side,
        }
    }
    pub fn start(&self) -> Caret {
        Caret::new(self.start_id, self.side)
    }

    pub fn end(&self) -> Caret {
        Caret::new(self.start_id, self.side)
    }

    /// note: a and b must be supplied in order along the cross axis of the given side or this fails
    pub fn is_connected_on(&self, graph: &Graph) -> bool {
        if graph.tiles[&self.start_id].kind == Surface {
            false
        } else {
            let nav = self.start().navigate_gutter(graph);
            let location = nav.location();
            nav.iter(Positive)
                .take_while(|&n| n.location() == location)
                .any(|n| n.tile_id == self.end_id)
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct GutterNav<'a> {
    inner: Caret,
    pub graph: &'a Graph,
}

impl<'a> GutterNav<'a> {
    pub fn new(caret: Caret, graph: &'a Graph) -> Self {
        Self {
            inner: caret,
            graph,
        }
    }
}

impl Deref for GutterNav<'_> {
    type Target = Caret;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl DerefMut for GutterNav<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Navigator for GutterNav<'_> {
    type Direction = Sign;

    fn location(&self) -> i32 {
        self.graph.tiles[&self.tile_id]
            .rect
            .get_side_location(self.side)
    }

    fn peek(&self, direction: Self::Direction) -> Option<Caret> {
        let location = self.location();
        let search_direction = Cardinal {
            axis: !self.side.axis,
            sign: direction,
        };
        self.graph.intra_surface_neighbourhoods[&self.tile_id][search_direction]
            .iter()
            .filter(|id| self.graph.tiles[id].kind == TileKind::Window)
            .map(|&id| Caret::new(id, self.side))
            .find(|c| c.location_on(self.graph) == location)
    }

    fn next(&mut self, direction: Self::Direction) -> Option<Caret> {
        let result = self.peek(direction);
        if let Some(position) = result {
            *self.deref_mut() = position;
            Some(position)
        } else {
            None
        }
    }

    fn set_caret(&mut self, caret: Caret) -> &mut Self {
        *self.deref_mut() = caret;
        self
    }

    fn go_to(&mut self, tile_id: Uuid) -> &mut Self {
        self.tile_id = tile_id;
        self
    }

    fn face(&mut self, side: Cardinal) -> &mut Self {
        self.side = side;
        self
    }
}

pub trait GutterIterExt
where
    Self: Iterator<Item: Deref<Target = Caret>>,
{
    fn ids(self) -> Map<Self, fn(Self::Item) -> Uuid>
    where
        Self: Sized,
    {
        self.map(|c| c.tile_id)
    }
}

impl<T: ?Sized> GutterIterExt for T where T: Iterator<Item: Deref<Target = Caret>> {}

pub trait GutterNavigation {
    fn navigate_gutter(self, graph: &Graph) -> GutterNav;
}

impl GutterNavigation for Caret {
    fn navigate_gutter(self, graph: &Graph) -> GutterNav {
        GutterNav::new(self, graph)
    }
}
