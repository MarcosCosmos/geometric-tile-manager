use crate::geometry::Sign::*;
use crate::geometry::{Cardinal, Sign};
use crate::graph::{Graph, NodeId};
use crate::navigation::{Caret, Navigator};
use std::iter::Map;
use std::ops::{Deref, DerefMut};
use uuid::Uuid;

#[derive(Copy, Clone, Debug)]
pub struct GutterNav<'a> {
    inner: Caret,
    graph: &'a Graph,
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
        self.graph.bounds[&self.node_id].location(self.side)
    }

    fn peek(&self, direction: Self::Direction) -> Option<Caret> {
        let location = self.location();
        let search_direction = Cardinal {
            axis: !self.side.axis,
            sign: direction,
        };
        self.graph.intra_surface_neighbourhoods[&self.node_id][search_direction]
            .iter()
            .filter(|id| id.is_tile())
            .map(|&id| Caret::new(id, self.side))
            .find(|c| c.get_location(self.graph) == location)
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

    fn go_to(&mut self, node_id: NodeId) -> &mut Self {
        self.node_id = node_id;
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
    fn ids(self) -> Map<Self, fn(Self::Item) -> NodeId>
    where
        Self: Sized,
    {
        self.map(|c| c.node_id)
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
