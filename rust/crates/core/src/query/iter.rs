use crate::query::{Navigation, Query};
use std::iter::{Chain, Once};
use std::ops::Deref;

pub trait QueryIteration: Navigation {
    fn into_next(self, direction: Self::Direction) -> Option<Self>;

    fn into_iter(self, direction: Self::Direction) -> Iter<Self>
    where
        Self: Copy,
    {
        Iter::new(self, direction)
    }
}
impl<'a, Key> QueryIteration for Query<'a, Key>
where
    Query<'a, Key>: Navigation<Target = Key>,
{
    fn into_next(mut self, direction: Self::Direction) -> Option<Self> {
        if let Some(next) = self.peek(direction) {
            self.key = next;
            Some(self)
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Iter<T>
where
    T: Navigation + Copy,
    T::Target: Sized,
{
    current: T,
    direction: T::Direction,
}

impl<T> Iter<T>
where
    T: QueryIteration + Copy,
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

impl<T> Iterator for Iter<T>
where
    T: QueryIteration + Copy,
    <T as Deref>::Target: Sized,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.current.into_next(self.direction) {
            self.current = next;
            Some(next)
        } else {
            None
        }
    }
}
