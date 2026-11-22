use crate::caret::Caret;
use crate::context::QueryContext;
use crate::geometry::AreaBounds;
use crate::graph::NodeId;
use crate::key::{NodeKey, QueryKey};
use crate::line::Line;
use markers_derive::Command;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::Debug;

mod add_surface;
pub mod add_tile;
pub mod resize_surface;
pub mod transform;

pub trait Command: Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned {}

/// guides are a todo: demo first.
/// A guide for how to execute certain commands. Can be either strict or hint;
/// If hint, the algorithm will try to resolve it into a more reasonable selection if possible.
/// Additionally, commands will fail if a strict guide is invalid, but proceed if a hint is invalid.
/// Guides are mostly lines but may later be extended to include bounding boxes for limiting a resize?
/// A good example of the importance of hinting is deleting a tile:
/// - If there is a guide selection/line that doesn't need to be expanded to be usable, it should usually be preferred, as expanding carries a risk of pancaking another tile into invisibility.
// pub enum Guide<T>()

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, SelectionCommand)]
pub struct DeleteNode<K = Caret>
where
    K: QueryKey + Into<Caret<K::NodeKey>> + Into<Line<K::NodeKey>>,
{
    pub key: K,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, SelectionCommand)]
pub struct MoveLine<K = Line>
where
    K: QueryKey + Into<Line<K::NodeKey>>,
{
    pub key: K,
    pub new_location: i32,
}

impl<K> MoveLine<K>
where
    K: QueryKey + Into<Line<K::NodeKey>>,
{
    pub fn new(key: K, new_location: i32) -> Self {
        Self { key, new_location }
    }
}

pub trait RawCommandExecutor<Com>
where
    Com: Command,
{
    type Response: Serialize;
    fn execute(&mut self, command: Com) -> Self::Response;
}

pub trait CommandExecutor<Com>
where
    Com: Command,
{
    type Response: Serialize;
    type Error: Error;
    fn try_execute(&mut self, command: Com) -> Result<Self::Response, Self::Error>;
}

impl<Com, T> CommandExecutor<Com> for T
where
    Com: Command + QueryKey,
    T: QueryContext<Com, OutKey: Command> + RawCommandExecutor<T::OutKey>,
{
    type Response = T::Response;
    type Error = T::Error;

    fn try_execute(&mut self, command: Com) -> Result<Self::Response, Self::Error> {
        let out_com = *self.try_query(command)?;
        Ok(self.execute(out_com))
    }
}
