use crate::command::{Command, RawCommandExecutor};
use crate::geometry::AreaBounds;
use crate::graph::{Graph, NeighbourhoodGroup, NodeId};
use serde::{Deserialize, Serialize};
use std::ops::Deref;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Command)]
pub struct AddSurface(pub AreaBounds);

impl Deref for AddSurface {
    type Target = AreaBounds;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RawCommandExecutor<AddSurface> for Graph {
    type Response = NodeId;

    fn execute(&mut self, command: AddSurface) -> Self::Response {
        let id = NodeId::new();
        self.bounds.insert(id.into(), *command);
        self.tile_neighbourhoods
            .insert(id.into(), NeighbourhoodGroup::new());
        self.surface_neighbourhoods
            .insert(id, NeighbourhoodGroup::new());
        id
    }
}
