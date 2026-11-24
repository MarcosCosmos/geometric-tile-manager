use crate::graph::Graph;
use crate::key::QueryKey;
use crate::query::{GraphQuery, Navigation, Query};
use std::iter::{Chain, Once};
use std::ops::Deref;

pub trait IterativeQueryNavigation: GraphQuery + Navigation + Ord {
    fn into_next(self, direction: Self::Direction) -> Option<Self>;

    fn into_iter(self, direction: Self::Direction) -> NavIter<Self>
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
    fn into_next(mut self, direction: Self::Direction) -> Option<Self> {
        *self = self.peek(direction)?;
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
pub struct KeyIter<'a, K, Iter>
where
    K: QueryKey,
    Iter: Iterator<Item = &'a K>,
{
    context: &'a Graph,
    inner: Iter,
}

impl<'a, K, Iter> KeyIter<'a, K, Iter>
where
    K: QueryKey,
    Iter: Iterator<Item = &'a K>,
{
    pub fn new(context: &'a Graph, inner: Iter) -> Self {
        Self { context, inner }
    }
}

impl<'a, K, Iter> Iterator for KeyIter<'a, K, Iter>
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

impl<'a, K, Iter> DoubleEndedIterator for KeyIter<'a, K, Iter>
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
