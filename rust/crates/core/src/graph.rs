use crate::error::Error;
use crate::geometry::{AreaBounds, Axis::*, Cardinal, Sign::*};
use crate::query::Query;
use crate::query::key::NodeKey;
use crate::query::key::QueryKey;
use crate::query::selection::TransformConstraints;
use crate::query::source::{QuerySource, RawQuerySource};
use anyhow::Result;
use markers_derive::NodeKey;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Index, IndexMut};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::query::line::Line;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, NodeKey)]
pub struct NodeId(Uuid);

impl NodeId {
    fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Into<Uuid> for NodeId {
    fn into(self) -> Uuid {
        self.0
    }
}

#[derive(Clone, Debug, Default)]
struct NeighbourhoodGroup {
    north: Vec<NodeId>,
    south: Vec<NodeId>,
    east: Vec<NodeId>,
    west: Vec<NodeId>,
}

/// Notes:
/// - There are inter-surface and intra-surface neighbourhoods which have different meanings.
/// - Gaps may be present either on the inside of a surface or between tiles; in htis sense gaps would potentially alter poorly written algorithms
/// - Storing tiles seperately from neighboy
#[derive(Clone, Debug, Default)]
pub struct Graph {
    pub(super) bounds: HashMap<NodeId, AreaBounds>,

    pub(super) containing_surface: HashMap<NodeId, NodeId>,
    /// Note: each surface has an entry in its own interior neighbourhood.
    /// The meaning of direction is semi-inverted in that the surface's edges face inwards not outwards, but directionality is otherwise the same.
    /// E.g. the eastern neighbourhood of a surface faces inwardly east from the western side, whereas a normal tile would face outwardly east from the eastern side.
    pub(super) tile_neighbourhoods: HashMap<NodeId, NeighbourhoodGroup>,
    /// inter_surface_neighbours are the neighbourhoods between surfaces, and therefore face outwards.
    pub(super) surface_neighbourhoods: HashMap<NodeId, NeighbourhoodGroup>,
}

impl Index<Cardinal> for NeighbourhoodGroup {
    type Output = Vec<NodeId>;
    fn index(&self, index: Cardinal) -> &Self::Output {
        match index {
            Cardinal {
                axis: Vertical,
                sign: Negative,
            } => &self.north,
            Cardinal {
                axis: Vertical,
                sign: Positive,
            } => &self.south,
            Cardinal {
                axis: Horizontal,
                sign: Negative,
            } => &self.west,
            Cardinal {
                axis: Horizontal,
                sign: Positive,
            } => &self.east,
        }
    }
}

impl IndexMut<Cardinal> for NeighbourhoodGroup {
    fn index_mut(&mut self, index: Cardinal) -> &mut Vec<NodeId> {
        match index {
            Cardinal {
                axis: Vertical,
                sign: Negative,
            } => &mut self.north,
            Cardinal {
                axis: Vertical,
                sign: Positive,
            } => &mut self.south,
            Cardinal {
                axis: Horizontal,
                sign: Negative,
            } => &mut self.west,
            Cardinal {
                axis: Horizontal,
                sign: Positive,
            } => &mut self.east,
        }
    }
}

impl NeighbourhoodGroup {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Graph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_surface(&mut self, bounds: AreaBounds) -> NodeId {
        let id = NodeId::new();
        self.bounds.insert(id.into(), bounds);
        self.tile_neighbourhoods
            .insert(id.into(), NeighbourhoodGroup::new());
        self.surface_neighbourhoods
            .insert(id, NeighbourhoodGroup::new());
        id
    }

    pub fn new_tile(
        &mut self,
        direction: Cardinal,
        origin: Line,
        constraints: TransformConstraints,
    ) -> Result<NodeId, Error<Line>> {
        let origin = self.query(origin);
        let bounds: AreaBounds; // todo
        let id = NodeId::new();
        self.bounds.insert(id, bounds);
        self.containing_surface
            .insert(id, self.containing_surface[&origin[Negative]]);

        // todo: all of this only works if strictly splitting the target. A more complicated strategy is needed for resizing all along the parallel

        // let mut tile_neighbourhood = NodeNeighbourhood::new();
        //
        // // adjust connections
        // // simple part: the direction parallel to the insertion
        // {
        //     let opposite = !direction;
        //     let neighbours_of_tile = &mut tile_neighbourhood[direction];
        //     {
        //         let neighbours_of_target =
        //             &mut self.tile_neighbourhoods.get_mut(target_id).unwrap()[direction];
        //         std::mem::swap(neighbours_of_target, neighbours_of_tile);
        //         neighbours_of_target.push(id); // our new tile is guaranteed to be the only neighbour, so we can simply push
        //     }
        //
        //     // todo: maybe this is not sufficient because our new tile could erase another neighbour (?)
        //     // might be worth extracting to a method anyway.
        //     for id in neighbours_of_tile {
        //         self.tile_neighbourhoods
        //             .get_mut(id)
        //             .unwrap()
        //             .replace(opposite, target_id, *id);
        //     }
        // }

        // todo: figure out how mix tile neighbour with other gutter corrections?

        Ok(id)
    }
}

impl QuerySource<Uuid> for Graph {
    type OutKey = NodeId;

    fn try_query(&self, target: Uuid) -> Result<Query<Self::OutKey>> {
        let id = NodeId(target);
        if self.bounds.contains_key(&id) {
            Ok(self.query(id))
        } else {
            Err(Error::NodeNotFound(target).into())
        }
    }
}
