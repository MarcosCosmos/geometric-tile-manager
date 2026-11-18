use crate::geometry::{AreaBounds, Axis};
use crate::query::caret::Caret;
use crate::query::line::Line;
use crate::query::{NodeKey, QueryKey};
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

pub enum Command {
    Query,
    Edit,
}
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Edit<CaretKey, LineKey>
where
    CaretKey: QueryKey + Into<Caret<CaretKey::NodeKey>>,
    LineKey: QueryKey<NodeKey = CaretKey::NodeKey> + Into<Line<LineKey::NodeKey>>,
{
    AddTile(LineKey),
    AddSurface(AreaBounds),
    DeleteNode(CaretKey),
    MoveLine(LineKey, Axis, i32),
}

pub struct QueryCommand<Key>
where
    Key: QueryKey,
{
    target: Key,
}

pub struct AddTile<N, N>
where
    N: Into<Line<N>>,
    N: Copy + Clone + Debug + Eq + Hash,
{
    pub target: N,
    phantom_data: PhantomData<N>,
}
impl<N, N> AddTile<N, N>
where
    N: Into<Line<N>>,
    N: Copy + Clone + Debug + Eq + Hash,
{
    pub fn new(target: N) -> Self {
        Self {
            target,
            phantom_data: Default::default(),
        }
    }
}

pub struct AddSurface {
    pub target: AreaBounds,
}

impl AddSurface {
    pub fn new(target: AreaBounds) -> Self {
        Self { target }
    }
}

pub struct DeleteNode<N>
where
    N: Copy + Clone + Debug + Eq + Hash,
{
    pub target: N,
}
impl<N> DeleteNode<N>
where
    N: Copy + Clone + Debug + Eq + Hash,
{
    pub fn new(target: N) -> Self {
        Self { target }
    }
}
