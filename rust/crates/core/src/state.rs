use crate::geometry::AreaBounds;
use crate::graph::NodeId;
use crate::iter::KeyIter;
use crate::key::QueryKey;
use crate::query::{ElementInformation2D, Query};
use markers_derive::{Command, QueryKey, TransformKey};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::Keys;

/// Bulk query for all nodes in the graph.
/// Like neighbour queries, its iteration encodes Query<NodeId> instead of Query<Caret>. Unlike neighbourhood queries, its serialization will be all the data about all of those nodes.
/// Mainly intended for 'rendering' purposes, though ideally commands should return a changeset that is basically a subset of this viewing affected nodes.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey)]
#[node(NodeId)]
pub struct State;

pub trait StateInformation: IntoIterator<Item: ElementInformation2D> {}

impl<'a> IntoIterator for Query<'a, State> {
    type Item = Query<'a, NodeId>;
    /// Note: The type of the inner iter should not be considered stable information
    type IntoIter = KeyIter<'a, NodeId, Keys<'a, NodeId, AreaBounds>>;

    fn into_iter(self) -> Self::IntoIter {
        KeyIter::new(self.context, self.context.bounds.keys())
    }
}
