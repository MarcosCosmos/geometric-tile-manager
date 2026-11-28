use crate::command::{Command, RawCommandExecutor};
use crate::context::QueryContext;
use crate::error::{KeyedError, QueryError};
use crate::geometry::AreaBounds;
use crate::graph::{Graph, NeighbourhoodGroup, NodeId};
use crate::key::QueryKey;
use crate::query::Query;
use geotm_macros::QueryKey;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Command, QueryKey)]
pub struct AddSurface(pub AreaBounds);

impl Deref for AddSurface {
    type Target = AreaBounds;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for AddSurface {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl QueryContext<AddSurface> for Graph {
    type OutKey = AddSurface;

    type ErrorDetail = QueryError<AddSurface, !, !>;

    fn resolve(
        &self,
        key: AddSurface,
    ) -> Result<Query<'_, Self::OutKey>, KeyedError<AddSurface, Self::ErrorDetail>> {
        Ok(Query::new(self, key))
    }
}

impl RawCommandExecutor<AddSurface> for Graph {
    type Response = NodeId;

    fn execute(&mut self, command: AddSurface) -> Self::Response {
        let id = NodeId::new();
        self.bounds.insert(id, *command);
        self.tile_neighbourhoods
            .insert(id, NeighbourhoodGroup::new());
        self.surface_neighbourhoods
            .insert(id, NeighbourhoodGroup::new());
        id
    }
}
