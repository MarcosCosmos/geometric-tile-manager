pub mod area;
pub mod caret;
pub mod iter;
pub mod key;
pub mod line;
pub mod neighbours;
pub mod node;
pub mod selection;
pub mod source;
pub mod point;

use crate::geometry::{AreaBounds, LineBounds};
use crate::graph::{Graph, NodeId};
use crate::query::key::QueryKey;
use std::fmt::Debug;
use std::ops::{Deref, Not};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ElementKind {
    Tile,
    Surface,
    Mixed,
}

// todo: add serialisation requirement to output the results once other testing has started.
pub trait GraphQuery: Deref<Target: QueryKey> + Sized {}

pub trait ElementInformation: Deref<Target: QueryKey> + Sized {
    fn kind(&self) -> ElementKind;
}



pub trait ElementInformation1D: ElementInformation {
    fn bounds(&self) -> LineBounds;
}

pub trait ElementInformation2D: ElementInformation {
    fn bounds(&self) -> AreaBounds;
}

pub trait GroupInformation: ElementInformation {
    fn is_connected(&self) -> bool;
}

pub trait GroupQuery: GroupInformation + TryInto<Self::NodeQuery> {
    type NodeQuery;
}

pub trait Navigation: Deref<Target: QueryKey> + Sized {
    type Direction: Copy + Not<Output = Self::Direction>;

    fn peek(&self, direction: Self::Direction) -> Option<Self::Target>;
}

pub trait NavigationMut: Navigation {
    fn next(&mut self, direction: Self::Direction) -> Option<&mut Self>;
    fn set_target(&mut self, target: Self::Target);
}

/// A transparent wrapper struct that facilitates que functionally for types, like Caret or Line, that have que related functionality but shouldn't be stored with a persistent reference to the graph.
/// For each type where this is rational, the Navigation trait is implemented on the specialisation of Nav<'a, T>.
#[derive(Copy, Clone, Debug)]
pub struct Query<'a, Key>
where
    Key: QueryKey,
{
    context: &'a Graph,
    key: Key,
}


impl<'a, Key> Query<'a, Key>
where
    Key: QueryKey,
{
    pub(super) fn new(context: &'a Graph, key: Key) -> Self {
        Self { context, key }
    }
}

impl<Key> Deref for Query<'_, Key>
where
    Key: QueryKey,
{
    type Target = Key;

    fn deref(&self) -> &Self::Target {
        &self.key
    }
}

impl<'a, Key> GraphQuery for Query<'a, Key> where Key: QueryKey {}

impl<'a, Key> GroupQuery for Query<'a, Key> where Self : GroupInformation, Key: QueryKey, Query<'a, NodeId>: From<Query<'a, Key>> {
    type NodeQuery = Query<'a, NodeId>;
}