use crate::geometry::Cardinal;
use crate::geometry::Sign::{Negative, Positive};
use crate::graph::{Graph, NodeId};
use crate::gutter::GutterNavigation;
use std::iter::{Chain, Once};
use std::ops::{Deref, Not};

/// A caret is a position on the side of a node.
/// they provide the step function for navigating along the gutter of carets parallel to them.
#[derive(Copy, Clone, Debug)]
pub struct Caret {
    pub node_id: NodeId,
    pub side: Cardinal,
}

impl Caret {
    pub fn new(node_id: NodeId, side: Cardinal) -> Self {
        Self {
            node_id: node_id,
            side: side,
        }
    }

    pub fn get_location(&self, graph: &Graph) -> i32 {
        graph.bounds[&self.node_id].location(self.side)
    }
}



#[derive(Copy, Clone, Debug)]
// As in, a line between two parallel Carets.
// This line may or may not be fully "connected" (without cutting through the interior of any tiles)
pub struct Line {
    pub start_id: NodeId,
    pub end_id: NodeId,
    pub side: Cardinal,
}

impl Line {
    pub fn new(start_id: NodeId, end_id: NodeId, side: Cardinal) -> Self {
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

    pub fn get_location(&self, graph: &Graph) -> i32 {
        let start_location = self.start().get_location(graph);
        let end_location = self.end().get_location(graph);
        match self.side.sign {
            Positive => i32::min(start_location, end_location),
            Negative => i32::max(start_location, end_location),
        }
    }

    /// note: a and b must be supplied in order along the cross axis of the given side or this fails
    pub fn is_connected_on(&self, graph: &Graph) -> bool {
        self.start()
            .navigate_gutter(graph)
            .iter(Positive)
            .any(|n| n.node_id == self.end_id)
    }
}

pub trait Navigator: Deref<Target = Caret> {
    type Direction: Copy + Not<Output = Self::Direction>;
    fn location(&self) -> i32;

    fn peek(&self, direction: Self::Direction) -> Option<Caret>;

    fn next(&mut self, direction: Self::Direction) -> Option<Caret>;

    fn set_caret(&mut self, caret: Caret) -> &mut Self;

    fn go_to(&mut self, node_id: NodeId) -> &mut Self;

    fn face(&mut self, side: Cardinal) -> &mut Self;

    fn iter(self, direction: Self::Direction) -> NavIter<Self>
    where
        Self: Copy,
    {
        NavIter::new(self, direction)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct NavIter<T>
where
    T: Navigator + Copy,
{
    current: T,
    direction: T::Direction,
}

impl<T> NavIter<T>
where
    T: Navigator + Copy,
{
    pub fn new(navigator: T, direction: T::Direction) -> Self {
        Self {
            current: navigator,
            direction,
        }
    }
    pub fn inclusive(self) -> Chain<Once<T>, Self>
    where
        T: Sized + Copy,
    {
        std::iter::once(self.current).chain(self)
    }
}

impl<C> Iterator for NavIter<C>
where
    C: Navigator + Copy,
{
    type Item = C;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.next(self.direction).is_some() {
            Some(self.current)
        } else {
            None
        }
    }
}