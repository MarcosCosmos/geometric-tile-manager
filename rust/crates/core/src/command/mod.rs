use crate::caret::Caret;
use crate::geometry::AreaBounds;
use crate::graph::NodeId;
use crate::key::{NodeKey, QueryKey};
use crate::line::Line;
use crate::source::QuerySource;
use markers_derive::Command;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::Debug;

pub mod add_tile;
pub mod transform;
mod add_surface;

pub trait Command: Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned {}

/// guides are a todo: demo first.
/// A guide for how to execute certain commands. Can be either strict or hint;
/// If hint, the algorithm will try to resolve it into a more reasonable selection if possible.
/// Additionally, commands will fail if a strict guide is invalid, but proceed if a hint is invalid.
/// Guides are mostly lines but may later be extended to include bounding boxes for limiting a resize?
/// A good example of the importance of hinting is deleting a tile:
/// - If there is a guide selection/line that doesn't need to be expanded to be usable, it should usually be preferred, as expanding carries a risk of pancaking another tile into invisibility.
// pub enum Guide<T>()

pub trait EditCommand: Command {
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



pub trait RawCommandExecutor<Com>
where
    Com: Command {
    type Response: Serialize;
    fn execute(&mut self, command: Com) -> Self::Response;
}

pub trait CommandExecutor<Com>
where
    Com: Command {
    type Response: Serialize;
    type Error: Error;
    fn try_execute(&mut self, command: Com) -> Result<Self::Response, Self::Error>;
}

impl <Com, T> CommandExecutor<Com> for T
where Com : Command + QueryKey,
    T: QuerySource<Com, OutKey:Command> + RawCommandExecutor<T::OutKey> {
    type Response = T::Response;
    type Error = T::Error;

    fn try_execute(&mut self, command: Com) -> Result<Self::Response, Self::Error> {
        let out_com = *self.try_query(command)?;
        Ok(self.execute(out_com))
    }
}