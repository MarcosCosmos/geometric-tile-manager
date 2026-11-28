use crate::graph::Graph;
use crate::key::QueryKey;
use crate::query::{GraphQuery, Navigation, Query};
use std::iter::{Chain, Once};
use std::ops::Deref;

pub trait IterativeQueryNavigation: GraphQuery + Navigation + Ord {
    fn to_next(self, direction: Self::Direction) -> Option<Self>;

    fn to_iter(self, direction: Self::Direction) -> NavIter<Self>
    where
        Self: Copy,
    {
        NavIter::new(self, direction)
    }
}
impl<T> IterativeQueryNavigation for T
where
    T: GraphQuery + Navigation + Ord,
{
    fn to_next(mut self, direction: Self::Direction) -> Option<Self> {
        *self = self.peek(direction)?;
        Some(self)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct NavIter<T>
where
    T: IterativeQueryNavigation + Copy,
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
        self.current = self.current.to_next(self.direction)?;
        Some(self.current)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct QueryIter<'a, K, Iter>
where
    K: QueryKey,
    Iter: Iterator<Item = &'a K>,
{
    context: &'a Graph,
    inner: Iter,
}

impl<'a, K, Iter> QueryIter<'a, K, Iter>
where
    K: QueryKey,
    Iter: Iterator<Item = &'a K>,
{
    pub fn new(context: &'a Graph, inner: Iter) -> Self {
        Self { context, inner }
    }
}

impl<'a, K, Iter> Iterator for QueryIter<'a, K, Iter>
where
    K: QueryKey,
    Iter: Iterator<Item = &'a K>,
{
    type Item = Query<'a, K>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|&item| Query::new(self.context, item))
    }
}

impl<'a, K, Iter> DoubleEndedIterator for QueryIter<'a, K, Iter>
where
    K: QueryKey,
    Iter: DoubleEndedIterator<Item = &'a K>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner
            .next_back()
            .map(|&item| Query::new(self.context, item))
    }
}

/// An artificially constructed double ended iterator over the inclusive range between two locations in the graph.
/// `RangeIter` does not attempt to ensure these points are truly connected, and relies on implemented equality checks to detect them meeting in the middle.
/// So it is a logic error to construct this iter over a range that doesn't connect.
/// Also, this iterator technically evaluates one step ahead of it's return (mainly for inclusivity), but that should be inconsequential for queries.
#[derive(Copy, Clone, Debug)]
pub struct RangeIter<T>
where
    T: IterativeQueryNavigation + Copy,
    <T as Deref>::Target: Sized,
{
    current: Option<T>,
    back_current: Option<T>,
    direction: T::Direction,
}

impl<T> RangeIter<T>
where
    T: IterativeQueryNavigation + Copy,
    <T as Deref>::Target: Sized,
{
    pub fn new(start: T, end: T, direction: T::Direction) -> Self {
        Self {
            current: Some(start),
            back_current: Some(end),
            direction,
        }
    }
}

impl<T> Iterator for RangeIter<T>
where
    T: IterativeQueryNavigation + Copy,
    <T as Deref>::Target: Sized,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current?;
        if self
            .back_current
            .is_some_and(|back_current| back_current == current)
        {
            self.current = None;
            self.back_current = None;
        } else {
            self.current = current.to_next(self.direction);
        }
        Some(current)
    }
}

impl<T> DoubleEndedIterator for RangeIter<T>
where
    T: IterativeQueryNavigation + Copy,
    <T as Deref>::Target: Sized,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let back_current = self.back_current?;
        if self.current.is_some_and(|current| current == back_current) {
            self.current = None;
            self.back_current = None;
        } else {
            self.back_current = back_current.to_next(!self.direction);
        }
        Some(back_current)
    }
}
