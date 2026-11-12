use crate::geometry::Cardinal;
use crate::graph::Graph;
use std::iter::{Chain, Once};
use std::ops::{Deref, DerefMut, Not};
use uuid::Uuid;

/// A caret is a position on the side of a window.
/// they provide the step function for navigating along the gutter of carets parallel to them.
#[derive(Copy, Clone, Debug)]
pub struct Caret {
    pub tile_id: Uuid,
    pub side: Cardinal,
}

impl Caret {
    pub fn new(tile_id: Uuid, side: Cardinal) -> Self {
        Self {
            tile_id,
            side: side,
        }
    }

    pub fn location_on(&self, graph: &Graph) -> i32 {
        graph.tiles[&self.tile_id].rect.get_side_location(self.side)
    }
}

pub trait Navigator: Deref<Target = Caret> {
    type Direction: Copy + Not<Output = Self::Direction>;
    fn location(&self) -> i32;

    fn peek(&self, direction: Self::Direction) -> Option<Caret>;

    fn next(&mut self, direction: Self::Direction) -> Option<Caret>;

    fn set_caret(&mut self, caret: Caret) -> &mut Self;

    fn go_to(&mut self, tile_id: Uuid) -> &mut Self;

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
