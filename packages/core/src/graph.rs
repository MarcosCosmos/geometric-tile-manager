use crate::area::{Area, AreaQuery};
use crate::caret::Caret;
use crate::command::transform_area::SegmentedArea;
use crate::context::QueryContext;
use crate::error::{KeyedError, KeyedSnafu};
use crate::geometry::{
    AreaBounds, Axis, Axis::*, AxisBounds, BoundsAlignment, Cardinal, Sign, Sign::*,
};
use crate::iter::{IterativeQueryNavigation, QueryIter};
use crate::key::NodeKey;
use crate::key::QueryKey;
use crate::line::{Line, LineQuery};
use crate::neighbours::{NeighbourSurfaces, NeighbourTiles, NeighbourhoodQuery};
use crate::node::{NodeNotFound, NodeQuery};
use crate::query::{
    ElementInformation, ElementInformation1D, ElementInformation2D, ElementKind, GroupInformation,
    Query,
};
use geotm_macros::{Encode, NodeKey, QueryKey};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use std::collections::HashMap;
use std::collections::hash_map::Keys;
use std::fmt::{Debug, Display, Formatter, write};
use std::hash::Hash;
use std::ops::{Index, IndexMut};
use uuid::Uuid;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, QueryKey, NodeKey, Encode)]
#[encode("{}")]
pub struct NodeId(Uuid);

impl NodeId {
    pub(crate) fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl From<NodeId> for Uuid {
    fn from(val: NodeId) -> Self {
        val.0
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct NeighbourhoodGroup {
    north: Vec<NodeId>,
    south: Vec<NodeId>,
    east: Vec<NodeId>,
    west: Vec<NodeId>,
}

/// Notes:
/// - There are inter-surface and intra-surface neighbourhoods which have different meanings.
/// - Gaps may be present either on the inside of a surface or between tiles; in htis sense gaps would potentially alter poorly written algorithms
/// - Storing tiles seperately from neighboy
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Graph {
    pub(crate) bounds: HashMap<NodeId, AreaBounds>,
    pub(crate) containing_surfaces: HashMap<NodeId, NodeId>,
    /// Note: each surface has an entry in its own interior neighbourhood.
    /// The meaning of direction is semi-inverted in that the surface's edges face inwards not outwards, but directionality is otherwise the same.
    /// E.g. the eastern neighbourhood of a surface faces inwardly east from the western side, whereas a normal tile would face outwardly east from the eastern side.
    pub(crate) tile_neighbourhoods: HashMap<NodeId, NeighbourhoodGroup>,
    /// inter_surface_neighbours are the neighbourhoods between surfaces, and therefore face outwards.
    pub(crate) surface_neighbourhoods: HashMap<NodeId, NeighbourhoodGroup>,
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

    /// low-level resizing is always done with one side fixed in place (only the non-anchored side actually moves)
    pub(crate) fn transform_node(
        &mut self,
        node_id: NodeId,
        axis: Axis,
        new_bounds: AxisBounds,
        packing_tiebreaker: Sign,
    ) {
        self.bounds.get_mut(&node_id).unwrap()[axis] = new_bounds;
        let node = Query::new(self, node_id);
        if node.kind() == ElementKind::Surface {
            self.transform_area(
                &node.to_interior().unwrap().to_segmented(axis),
                new_bounds,
                packing_tiebreaker,
            );
        }
    }

    pub(crate) fn transform_area(
        &mut self,
        container: &SegmentedArea,
        new_bounds: AxisBounds,
        packing_tiebreaker: Sign,
    ) {
        if let Ok(node) = NodeId::try_from(container.area()) {
            self.transform_node(node, container.axis, new_bounds, packing_tiebreaker);
            return;
        }

        // todo: move this into a query maybe?
        let new_segment_bounds = if container.segments.len() == 1 {
            vec![new_bounds]
        } else {
            let old_container_bounds = Query::new(self, container.area()).bounds()[container.axis];
            let old_container_size = old_container_bounds.extent();
            let new_container_size = new_bounds.extent();
            let (mut result, repairs): (Vec<AxisBounds>, Vec<AxisBounds>) = container
                .segments
                .iter()
                .map(|&segment| {
                    let original = Query::new(self, segment).bounds()[container.axis];
                    let clamped = original.clamp(&old_container_bounds);
                    let repair = AxisBounds::new(
                        original[Negative] - clamped[Negative],
                        original[Positive] - clamped[Positive],
                    );
                    (clamped, repair)
                })
                .unzip();

            if old_container_size > 0 {
                let scale = new_container_size as f64 / old_container_size as f64;
                for segment_bounds in result.iter_mut() {
                    segment_bounds[Positive] = segment_bounds[Negative]
                        + f64::floor(segment_bounds.extent() as f64 * scale) as i32;
                }
            } else {
                let portion =
                    f64::floor(new_container_size as f64 / container.segments.len() as f64) as i32;
                for segment_bounds in result.iter_mut() {
                    segment_bounds[Positive] = segment_bounds[Negative] + portion;
                }
            };

            let estimated_total: i32 = result.iter().map(|bounds| bounds.extent()).sum();
            let mut remainder = new_container_size - estimated_total;
            let signum = remainder.signum();
            if remainder as usize >= container.segments.len() {
                let excess_remainder = remainder / container.segments.len() as i32;
                for bounds in result.iter_mut() {
                    bounds[Positive] += excess_remainder;
                }
                remainder = remainder.abs() + container.segments.len() as i32;
            }
            if remainder != 0 {
                match packing_tiebreaker {
                    Negative => {
                        for bounds in result.iter_mut().take(remainder as usize) {
                            bounds[Positive] += signum
                        }
                    }
                    Positive => {
                        for bounds in result.iter_mut().rev().take(remainder as usize) {
                            bounds[Positive] += signum;
                        }
                    }
                }
            }

            let next_origin = new_bounds[Negative];

            for segment_bounds in result.iter_mut() {
                let size = segment_bounds.extent();
                segment_bounds[Negative] = next_origin;
                segment_bounds[Positive] = next_origin + size;
            }

            for (segment_bounds, repair) in result.iter_mut().zip(repairs) {
                segment_bounds[Negative] += repair[Negative];
                segment_bounds[Positive] += repair[Positive];
            }

            result
        };

        for (segment, segment_bounds) in container.segments.iter().zip(new_segment_bounds) {
            for cross_segment in Query::new(self, *segment)
                .to_segments(!container.axis)
                .collect::<Vec<_>>()
            {
                let subcontainer = Query::new(self, cross_segment).to_segmented(container.axis);
                self.transform_area(&subcontainer, segment_bounds, packing_tiebreaker);
            }
        }
    }

    fn find_insertion_index<T>(&self, destination: T, target: NodeId) -> Option<usize>
    where
        T: QueryKey + Into<Caret>,
        for<'a> Query<'a, T>: IntoIterator<Item = Query<'a, NodeId>>,
        for<'a> Query<'a, Caret>: Ord + PartialOrd,
    {
        let side = Into::<Caret>::into(destination).side;
        let destination = Query::new(self, destination);
        if !destination.into_iter().any(|node| *node == target) {
            let target = Query::new(self, Caret::new(target, side));
            Some(
                destination
                    .into_iter()
                    .position(|sib| sib.to_side(target.side) <= target)
                    .unwrap_or(0),
            )
        } else {
            None
        }
    }

    fn link<T>(&mut self, a: T, b: T)
    where
        T: QueryKey + Into<Caret>,
        for<'a> Query<'a, T>: IntoIterator<Item = Query<'a, NodeId>>,
        for<'a> Query<'a, Caret>: Ord + PartialOrd,
        Self: NeighbourIndexMut<T>,
    {
        let a_id = a.into().id;
        let b_id = b.into().id;

        debug_assert_ne!(a_id, b_id);

        if let Some(index) = self.find_insertion_index(a, b_id) {
            self.neighbours_mut(a).insert(index, b_id);
        }

        if let Some(index) = self.find_insertion_index(b, a_id) {
            self.neighbours_mut(b).insert(index, a_id);
        }
    }

    fn unlink<T>(&mut self, a: T, b: T)
    where
        T: QueryKey + Into<Caret>,
        for<'a> Query<'a, T>: NeighbourhoodQuery,
        Self: NeighbourIndexMut<T>,
    {
        debug_assert_ne!(a.into().id, b.into().id);
        self.neighbours_mut(a).retain(|id| id != &b.into().id);
        self.neighbours_mut(b).retain(|id| id != &a.into().id)
    }

    pub(crate) fn replace_connections(&mut self, target: Line, replacement: Line) {
        debug_assert!(Query::new(self, target).is_connected());
        debug_assert!(Query::new(self, replacement).is_connected());
        let target = Query::new(self, target);

        let targets: Vec<_> = target.into_iter().map(|c| *c).collect();

        let opposites: Vec<_> = targets
            .iter()
            .flat_map(|&caret| {
                Query::new(self, NeighbourTiles(caret))
                    .into_iter()
                    .map(|query| *query)
            })
            .collect();

        let (full_replacement_bounds, replacements) = {
            let replacement = Query::new(self, replacement);
            (
                replacement.bounds().axis_bounds,
                replacement.into_iter().map(|c| *c).collect::<Vec<_>>(),
            )
        };

        // sentinels are special and connect on the same side as where they are located.
        let opposing_side = if target.kind() == ElementKind::Surface {
            target.side
        } else {
            !target.side
        };

        let target = *target;

        println!("og: {:?}", targets);
        println!("op: {:?}", opposites);
        println!("rep: {:?}", replacements);

        for &target_caret in &targets {
            for &opposing_id in &opposites {
                let opposing_caret = Caret::new(opposing_id, opposing_side);
                let opposing_bounds = Query::new(self, opposing_caret).bounds().axis_bounds;
                match full_replacement_bounds.alignment(&opposing_bounds) {
                    BoundsAlignment::Before | BoundsAlignment::After => {}
                    _ => {
                        self.unlink(NeighbourTiles(target_caret), NeighbourTiles(opposing_caret));
                    }
                }
            }
        }

        // sentinels are also special in that they don't connect to each other
        if target != replacement {
            for original_caret in targets {
                let original_bounds = Query::new(self, original_caret).bounds().axis_bounds;

                for &replacement_caret in &replacements {
                    let replacement_bounds =
                        Query::new(self, replacement_caret).bounds().axis_bounds;
                    match replacement_bounds.alignment(&original_bounds) {
                        BoundsAlignment::Before | BoundsAlignment::After => {}
                        _ => {
                            self.link(
                                NeighbourTiles(original_caret),
                                NeighbourTiles(replacement_caret),
                            );
                        }
                    }
                }
            }
        }
    }

    /// note: the replacements are attached to the scope, since it's possible that at this stage the scope has no opposites yet
    pub(crate) fn update_connections(&mut self, target: Line) {
        let targets: Vec<_> = Query::new(self, target).into_iter().map(|c| *c).collect();

        let opposites: Vec<_> = targets
            .iter()
            .flat_map(|&caret| {
                Query::new(self, NeighbourTiles(caret))
                    .into_iter()
                    .map(|query| *query)
            })
            .collect();

        println!("og: {:?}", targets);
        println!("op: {:?}", opposites);

        for target_caret in targets {
            let target_bounds = Query::new(self, target_caret).bounds().axis_bounds;
            for &opposing_id in &opposites {
                let opposing_caret = Caret::new(opposing_id, !target.side);
                let opposing_bounds = Query::new(self, opposing_caret).bounds().axis_bounds;
                match target_bounds.alignment(&opposing_bounds) {
                    BoundsAlignment::Before | BoundsAlignment::After => {
                        self.unlink(NeighbourTiles(target_caret), NeighbourTiles(opposing_caret));
                    }
                    _ => {
                        self.link(NeighbourTiles(target_caret), NeighbourTiles(opposing_caret));
                    }
                }
            }
        }
    }

    pub fn iter(&self) -> QueryIter<NodeId, Keys<NodeId, AreaBounds>> {
        QueryIter::new(self, self.bounds.keys())
    }
}

impl<'a> IntoIterator for &'a Graph {
    type Item = Query<'a, NodeId>;
    /// Note: The type of the inner iter should not be considered stable information
    type IntoIter = QueryIter<'a, NodeId, Keys<'a, NodeId, AreaBounds>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// a mirror of the index trait specifically to facilitate neighbourhood queries.
/// Don't want to make it public because I don't want to expose the implementation details of the graph other than through queries.
trait NeighbourIndexMut<Idx>
where
    Idx: QueryKey + Into<Caret>,
{
    fn neighbours_mut(&mut self, index: Idx) -> &mut Vec<NodeId>;
}

impl NeighbourIndexMut<NeighbourTiles> for Graph {
    fn neighbours_mut(&mut self, index: NeighbourTiles) -> &mut Vec<NodeId> {
        let mut caret: Caret = index.into();
        &mut self.tile_neighbourhoods.get_mut(&caret.id).unwrap()[caret.side]
    }
}

impl NeighbourIndexMut<NeighbourSurfaces> for Graph {
    fn neighbours_mut(&mut self, index: NeighbourSurfaces) -> &mut Vec<NodeId> {
        let caret: Caret = index.into();
        &mut self.surface_neighbourhoods.get_mut(&caret.id).unwrap()[caret.side]
    }
}

impl QueryContext<Uuid> for Graph {
    type OutKey = NodeId;
    type ErrorDetail = NodeNotFound;

    fn resolve(
        &self,
        key: Uuid,
    ) -> Result<Query<'_, Self::OutKey>, KeyedError<Uuid, Self::ErrorDetail>> {
        let id = NodeId(key);
        if !self.bounds.contains_key(&id) {
            Err(NodeNotFound).with_context(|_| KeyedSnafu { key })
        } else {
            Ok(Query::new(self, id))
        }
    }
}

impl QueryContext<NodeId> for Graph {
    type OutKey = NodeId;
    type ErrorDetail = <Self as QueryContext<Uuid>>::ErrorDetail;

    fn resolve(
        &self,
        key: NodeId,
    ) -> Result<Query<'_, Self::OutKey>, KeyedError<NodeId, Self::ErrorDetail>> {
        Ok(self
            .resolve(key.0)
            .map_err(|_| NodeNotFound)
            .with_context(|_| KeyedSnafu { key })?)
    }
}
