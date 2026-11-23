use crate::area::{Area, AreaQuery};
use crate::command::transform::{TransformDetails, TransformInformation};
use crate::command::{Command, RawCommandExecutor};
use crate::context::QueryContext;
use crate::error::ValidationError;
use crate::error::{KeyedError, KeyedSnafu, QueryError};
use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{AreaBounds, Cardinal};
use crate::graph::{Graph, NodeId};
use crate::key::NodeKey;
use crate::key::QueryKey;
use crate::line::Line;
use crate::node::{NodeNotFound, NodeQuery};
use crate::query::{ElementInformation, ElementKind, Query};
use markers_derive::{QueryKey, ValidationError};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu, ensure};
use std::ops::Deref;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey, Command)]
pub struct ResizeSurface<Node = NodeId>
where
    Node: NodeKey,
{
    pub key: Node,
    pub new_bounds: AreaBounds,
}

impl<Node> ResizeSurface<Node>
where
    Node: NodeKey,
{
    pub fn new(key: Node, new_bounds: AreaBounds) -> Self {
        Self {
            key: key,
            new_bounds,
        }
    }
}

/// The specified node was not a surface.
#[derive(Debug, Snafu, ValidationError)]
pub struct NotASurface;

pub trait ResizeSurfaceQuery: Sized {
    fn into_validated(self) -> Result<Self, NotASurface>;
    fn into_normalized(self) -> Self;
}

impl TransformInformation for Query<'_, ResizeSurface> {
    fn details(&self) -> TransformDetails {
        TransformDetails::Positive(
            Query::new(self.context, Area::from(self.key))
                .into_segmented(Cardinal::new(Horizontal, Negative)),
        )
    }
}

impl ResizeSurfaceQuery for Query<'_, ResizeSurface> {
    fn into_validated(self) -> Result<Self, NotASurface> {
        ensure!(
            Query::new(self.context, self.key).kind() == ElementKind::Surface,
            NotASurfaceSnafu
        );
        Ok(self)
    }

    fn into_normalized(mut self) -> Self {
        self.key = *Query::new(self.context, self.key).into_containing_surface();
        self
    }
}

impl<Node, T> QueryContext<ResizeSurface<Node>> for T
where
    Node: QueryKey + NodeKey,
    T: QueryContext<Node, OutKey = NodeId, Error = KeyedError<Node, NodeNotFound>>,
    QueryError<Node, T::Error, NotASurface>: From<T::Error>,
{
    type OutKey = ResizeSurface;

    type Error = KeyedError<ResizeSurface<Node>, QueryError<Node, T::Error, NotASurface>>;

    fn try_query(&self, key: ResizeSurface<Node>) -> Result<Query<Self::OutKey>, Self::Error> {
        (|| {
            Ok(self
                .query(ResizeSurface::new(
                    *self.try_query(key.key)?,
                    key.new_bounds,
                ))
                .into_normalized()
                .into_validated()?)
        })()
        .with_context(|_| KeyedSnafu { key })
    }
}

impl RawCommandExecutor<ResizeSurface> for Graph {
    type Response = ();

    fn execute(&mut self, command: ResizeSurface) -> Self::Response {
        self.transform_node(
            command.key,
            Horizontal,
            command.new_bounds[Horizontal],
            Positive,
        );
        self.transform_node(
            command.key,
            Vertical,
            command.new_bounds[Horizontal],
            Positive,
        );
    }
}
