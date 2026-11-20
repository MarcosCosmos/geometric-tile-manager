use crate::geometry::AreaBounds;
use crate::graph::NodeId;
use crate::query::caret::Caret;
use crate::query::key::{NodeKey, QueryKey};
use crate::query::line::Line;
use markers_derive::{Command, SelectionCommand};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub trait Command: Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned {}

pub trait SelectionCommand: Command {
    type Key: QueryKey + Into<Line<<Self::Key as QueryKey>::NodeKey>>;
    fn key(self) -> Self::Key;
}

// todo: optionally allow selection commands to include an area to constraint the op to. Defaults to the surface if not specified, and the main complexity of adding it would be having to validate it's query.

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Command)]
pub struct RunQuery<Key>
where
    Key: QueryKey,
{
    pub key: Key,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, SelectionCommand)]
pub struct AddTile<Key = Line>
where
    Key: QueryKey + Into<Line<Key::NodeKey>>,
{
    pub key: Key,
}

impl<Key> AddTile<Key>
where
    Key: QueryKey + Into<Line<Key::NodeKey>>,
{
    pub fn new(key: Key) -> Self {
        Self { key }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Command)]
pub struct AddSurface {
    pub bounds: AreaBounds,
}
impl AddSurface {
    pub fn new(bounds: AreaBounds) -> Self {
        Self { bounds }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Command)]
pub struct ResizeSurface<Key = NodeId>
where
    Key: NodeKey,
{
    pub key: Key,
    pub new_bounds: AreaBounds,
}
impl<Key> ResizeSurface<Key>
where
    Key: NodeKey,
{
    pub fn new(key: Key, new_bounds: AreaBounds) -> Self {
        Self { key, new_bounds }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, SelectionCommand)]
pub struct DeleteNode<Key = Caret>
where
    Key: QueryKey + Into<Caret<Key::NodeKey>> + Into<Line<Key::NodeKey>>,
{
    pub key: Key,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, SelectionCommand)]
pub struct MoveLine<Key = Line>
where
    Key: QueryKey + Into<Line<Key::NodeKey>>,
{
    pub key: Key,
    pub new_location: i32,
}

impl<Key> MoveLine<Key>
where
    Key: QueryKey + Into<Line<Key::NodeKey>>,
{
    pub fn new(key: Key, new_location: i32) -> Self {
        Self { key, new_location }
    }
}
