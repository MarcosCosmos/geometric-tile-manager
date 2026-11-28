use crate::context::QueryContext;
use crate::error::KeyedError;
use crate::graph::Graph;
use crate::key::QueryKey;
use geotm_macros::Command;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fmt::Debug;

pub mod add_surface;
pub mod add_tile;
pub mod transform;

pub trait Command: Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned {}

// guides are a todo: demo first.
// A guide for how to execute certain commands. Can be either strict or hint;
// If hint, the algorithm will try to resolve it into a more reasonable selection if possible.
// Additionally, commands will fail if a strict guide is invalid, but proceed if a hint is invalid.
// Guides are mostly lines but may later be extended to include bounding boxes for limiting a resize?
// A good example of the importance of hinting is deleting a tile:
// - If there is a guide selection/line that doesn't need to be expanded to be usable, it should usually be preferred, as expanding carries a risk of pancaking another tile into invisibility.
// pub enum Guide<T>()

pub trait RawCommandExecutor<Com>
where
    Com: Command,
{
    type Response: Serialize;
    fn execute(&mut self, command: Com) -> Self::Response;
}

pub trait CommandExecutor<Com>
where
    Com: Command + QueryKey,
{
    type Response: Serialize;
    type ErrorDetail: Error + 'static;
    fn try_execute(
        &mut self,
        command: Com,
    ) -> Result<Self::Response, KeyedError<Com, Self::ErrorDetail>>;
}

impl<C> CommandExecutor<C> for Graph
where
    C: Command + QueryKey,
    Self: QueryContext<C, OutKey: Command> + RawCommandExecutor<<Self as QueryContext<C>>::OutKey>,
{
    type Response = <Self as RawCommandExecutor<<Self as QueryContext<C>>::OutKey>>::Response;
    type ErrorDetail = <Self as QueryContext<C>>::ErrorDetail;

    fn try_execute(
        &mut self,
        command: C,
    ) -> Result<Self::Response, KeyedError<C, Self::ErrorDetail>> {
        let out_com = *(*self).resolve(command)?;
        Ok(self.execute(out_com))
    }
}
