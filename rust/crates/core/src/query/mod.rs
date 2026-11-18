pub mod area;
pub mod caret;
pub mod focus;
pub mod iter;
pub mod line;
pub mod neighbours;
pub mod node;
pub mod selection;
pub mod source;

pub use querykey_derive::QueryKey;

use crate::geometry::{AreaBounds, LineBounds};
use crate::graph::Graph;
use crate::query::area::AreaInformation;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Deref, Not};
use uuid::Uuid;

pub trait QueryKey: Sized + Copy + Clone + Debug + Eq {
    type NodeKey: NodeKey;
}

pub use nodekey_derive::NodeKey;

pub trait NodeKey: QueryKey {}

impl QueryKey for Uuid {
    type NodeKey = Self;
}
impl NodeKey for Uuid {}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ElementKind {
    Tile,
    Surface,
    Mixed,
}

pub trait ElementInformation: Deref<Target: QueryKey> + Sized {
    fn kind(&self) -> ElementKind;
}

pub trait ElementInformationMut: ElementInformation {
    fn set_target(&mut self, target: Self::Target);
}

pub trait ElementInformation1D: ElementInformation {
    fn bounds(&self) -> LineBounds;
}

pub trait ElementInformation2D: ElementInformation {
    fn bounds(&self) -> AreaBounds;
}

pub trait GroupInformation: ElementInformation + TryInto<Self::NodeInformation> {
    type NodeInformation;
    fn is_connected(&self) -> bool;
}

pub trait ElementNavigation: ElementInformation {
    type Direction: Copy + Not<Output = Self::Direction>;

    fn peek(&self, direction: Self::Direction) -> Option<Self::Target>;
}

pub trait ElementNavigationMut: ElementNavigation {
    fn next(&mut self, direction: Self::Direction) -> Option<&mut Self>;
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
