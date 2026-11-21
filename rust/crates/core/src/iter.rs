use crate::caret::Caret;
use crate::geometry::Cardinal;
use crate::graph::{Graph, NodeId};
use crate::key::QueryKey;
use crate::query::{Navigation, Query};
use crate::source::RawQuerySource;
use std::iter::{Chain, Once};
use std::ops::Deref;

pub trait IterativeQueryNavigation: Navigation {
    fn into_next(self, direction: Self::Direction) -> Option<Self>;

    fn into_iter(self, direction: Self::Direction) -> NavIter<Self>
    where
        Self: Copy,
    {
        NavIter::new(self, direction)
    }
}
impl<'a, Key> IterativeQueryNavigation for Query<'a, Key>
where
    Query<'a, Key>: Navigation<Target = Key>,
{
    fn into_next(mut self, direction: Self::Direction) -> Option<Self> {
        self.key = self.peek(direction)?;
        Some(self)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct NavIter<T>
where
    T: Navigation + Copy,
    T::Target: Sized,
{
    current: T,
    direction: T::Direction,
}

impl<T> NavIter<T>
where
    T: IterativeQueryNavigation + Copy,
    <T as Deref>::Target: Sized,
{
    pub fn new(item: T, direction: T::Direction) -> Self {
        Self {
            current: item,
            direction,
        }
    }
    pub fn inclusive(self) -> Chain<Once<T>, Self> {
        std::iter::once(self.current).chain(self)
    }
    pub fn include_if<P>(self, predicate: P) -> impl Iterator<Item = T>
    where
        P: FnMut(&T) -> bool,
    {
        std::iter::once(self.current).filter(predicate).chain(self)
    }
}

impl<T> Iterator for NavIter<T>
where
    T: IterativeQueryNavigation + Copy,
    <T as Deref>::Target: Sized,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.current = self.current.into_next(self.direction)?;
        Some(self.current)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct KeyIter<'a, Key, Iter>
where
    Key: QueryKey<NodeKey=NodeId>,
    Iter: Iterator<Item = &'a Key>,
{
    context: &'a Graph,
    inner: Iter
}

impl<'a, Key, Iter> KeyIter<'a, Key, Iter>
where
    Key: QueryKey<NodeKey=NodeId>,
    Iter: Iterator<Item = &'a Key>,
{
    pub fn new(context: &'a Graph, inner: Iter) -> Self {
        Self {
            context,
            inner
        }
    }
}

impl<'a, Key, Iter> Iterator for KeyIter<'a, Key, Iter>
where
    Key: QueryKey<NodeKey=NodeId>,
    Iter: Iterator<Item = &'a Key>,
{
    type Item = Query<'a, Key>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|&item| self.context.query(item))
    }
}

impl<'a, Key, Iter> DoubleEndedIterator for KeyIter<'a, Key, Iter>
where
    Key: QueryKey<NodeKey=NodeId>,
    Iter: DoubleEndedIterator<Item = &'a Key>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner
            .next_back()
            .map(|&item| self.context.query(item))
    }
}