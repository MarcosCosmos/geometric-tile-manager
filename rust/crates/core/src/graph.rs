use crate::area::{Area, AreaQuery};
use crate::caret::Caret;
use crate::command::transform::SegmentedArea;
use crate::context::QueryContext;
use crate::error::{KeyedError, KeyedSnafu};
use crate::geometry::{
    AreaBounds, Axis, Axis::*, AxisBounds, BoundsOrdering, Cardinal, Sign, Sign::*,
};
use crate::iter::IterativeQueryNavigation;
use crate::key::NodeKey;
use crate::key::QueryKey;
use crate::line::{Line, LineQuery};
use crate::neighbours::{NeighbourTiles, NeighbourhoodInformation};
use crate::node::NodeNotFound;
use crate::query::{
    ElementInformation, ElementInformation1D, ElementInformation2D, ElementKind, Query,
};
use itertools::Itertools;
use markers_derive::NodeKey;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Index, IndexMut};
use uuid::Uuid;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, NodeKey)]
pub struct NodeId(Uuid);

impl NodeId {
    pub(crate) fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Into<Uuid> for NodeId {
    fn into(self) -> Uuid {
        self.0
    }
}

#[derive(Clone, Debug, Default)]
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
#[derive(Clone, Debug, Default)]
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
                &Query::<Area>::from(node).into_segmented(Cardinal::new(axis, Negative)),
                new_bounds,
                packing_tiebreaker,
            );
        }
    }

    /// low-level resizing is always done with one side fixed in place (only the non-anchored side actually moves)
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
            let old_container_size = old_container_bounds.len();
            let new_container_size = new_bounds.len();
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
                        + f64::floor(segment_bounds.len() as f64 * scale) as i32;
                }
            } else {
                let portion =
                    f64::floor(new_container_size as f64 / container.segments.len() as f64) as i32;
                for segment_bounds in result.iter_mut() {
                    segment_bounds[Positive] = segment_bounds[Negative] + portion;
                }
            };

            let estimated_total: i32 = result.iter().map(|bounds| bounds.len()).sum();
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

            let mut next_origin = new_bounds[Negative];

            for segment_bounds in result.iter_mut() {
                let size = segment_bounds.len();
                segment_bounds[Negative] = next_origin;
                segment_bounds[Positive] = next_origin + size;
            }

            for (segment_bounds, repair) in result.iter_mut().zip(repairs.into_iter()) {
                segment_bounds[Negative] += repair[Negative];
                segment_bounds[Positive] += repair[Positive];
            }

            result
        };

        for (segment, segment_bounds) in container.segments.iter().zip(new_segment_bounds) {
            for cross_segment in Query::new(self, *segment)
                .into_segments(Cardinal::new(!container.axis, Positive))
                .collect::<Vec<_>>()
            {
                let subcontainer = Query::new(self, cross_segment)
                    .into_segmented(Cardinal::new(container.axis, Positive));
                self.transform_area(&subcontainer, segment_bounds, packing_tiebreaker);
            }
        }
    }

    // todo: remove code clones here.

    pub(crate) fn update_connections(
        &mut self,
        scope: Line,
        replacement_neg: NodeId,
        replacement_pos: NodeId,
    ) {
        let replacement_scope = Line::new(replacement_neg, replacement_pos, scope.side);
        let (full_replacement_bounds, replacements) = {
            let replacement_scope = Query::new(self, replacement_scope);
            (
                replacement_scope.bounds().axis_bounds,
                replacement_scope
                    .into_end(Negative)
                    .into_iter(Positive)
                    .take_while_inclusive(|caret| caret.id != scope[Positive])
                    .map(|c| *c)
                    .collect::<Vec<_>>(),
            )
        };

        let originals: Vec<_> = Query::new(self, scope)
            .into_end(Negative)
            .into_iter(Positive)
            .take_while_inclusive(|caret| caret.id != scope[Positive])
            .map(|c| *c)
            .collect();

        let opposites: Vec<_> = originals
            .iter()
            .flat_map(|&caret| Query::new(self, NeighbourTiles(caret)).items().iter())
            .copied()
            .collect();

        for original_caret in originals {
            for &opposing_id in &opposites {
                let opposing_caret = Caret::new(opposing_id, !scope.side);
                let opposing_bounds = Query::new(self, opposing_caret).bounds().axis_bounds;
                match full_replacement_bounds.cmp(&opposing_bounds) {
                    BoundsOrdering::Before | BoundsOrdering::After => {}
                    _ => {
                        self.tile_neighbourhoods
                            .get_mut(&original_caret.id)
                            .unwrap()[original_caret.side]
                            .retain(|id| *id != opposing_id);
                        self.tile_neighbourhoods
                            .get_mut(&opposing_caret.id)
                            .unwrap()[!opposing_caret.side]
                            .retain(|id| *id != original_caret.id);
                    }
                }
            }

            let original_bounds = Query::new(self, original_caret).bounds().axis_bounds;

            for &replacement_caret in &replacements {
                let replacement_bounds = Query::new(self, replacement_caret).bounds().axis_bounds;
                match replacement_bounds.cmp(&original_bounds) {
                    BoundsOrdering::Before | BoundsOrdering::After => {}
                    _ => {
                        let index_for_original =
                            Query::new(self, NeighbourTiles(replacement_caret))
                                .items()
                                .partition_point(|&sib_id| {
                                    Ordering::Greater
                                        != Query::new(
                                            self,
                                            Caret::new(sib_id, replacement_caret.side),
                                        )
                                        .bounds()
                                        .axis_bounds
                                        .cmp(&original_bounds)
                                        .into()
                                });
                        self.tile_neighbourhoods
                            .get_mut(&replacement_caret.id)
                            .unwrap()[replacement_caret.side]
                            .insert(index_for_original, original_caret.id);

                        let index_for_replacement =
                            Query::new(self, NeighbourTiles(original_caret))
                                .items()
                                .partition_point(|&sib_id| {
                                    Ordering::Greater
                                        != Query::new(self, Caret::new(sib_id, original_caret.side))
                                            .bounds()
                                            .axis_bounds
                                            .cmp(&replacement_bounds)
                                            .into()
                                });
                        self.tile_neighbourhoods
                            .get_mut(&original_caret.id)
                            .unwrap()[original_caret.side]
                            .insert(index_for_replacement, replacement_caret.id);
                    }
                }
            }
        }
    }

    /// note: the replacements are attached to the scope, since it's possible that at this stage the scope has no opposites yet
    pub(crate) fn repair_connections(&mut self, target: Line) {
        let targets: Vec<_> = Query::new(self, target)
            .into_end(Negative)
            .into_iter(Positive)
            .take_while_inclusive(|caret| caret.id != target[Positive])
            .map(|c| *c)
            .collect();

        let opposites: Vec<_> = targets
            .iter()
            .flat_map(|&caret| Query::new(self, NeighbourTiles(caret)).items().iter())
            .copied()
            .collect();

        for target_caret in targets {
            let target_bounds = Query::new(self, target_caret).bounds().axis_bounds;
            for &opposing_id in &opposites {
                let opposing_caret = Caret::new(opposing_id, !target.side);
                let opposing_bounds = Query::new(self, opposing_caret).bounds().axis_bounds;
                match target_bounds.cmp(&opposing_bounds) {
                    BoundsOrdering::Before | BoundsOrdering::After => {
                        self.tile_neighbourhoods.get_mut(&target_caret.id).unwrap()
                            [target_caret.side]
                            .retain(|id| *id != opposing_id);
                        self.tile_neighbourhoods
                            .get_mut(&opposing_caret.id)
                            .unwrap()[!opposing_caret.side]
                            .retain(|id| *id != target_caret.id);
                    }
                    _ => {
                        if !Query::new(self, NeighbourTiles(target_caret))
                            .items()
                            .contains(&opposing_caret.id)
                        {
                            let index_for_target = Query::new(self, NeighbourTiles(opposing_caret))
                                .items()
                                .partition_point(|&sib_id| {
                                    Ordering::Greater
                                        != Query::new(self, Caret::new(sib_id, opposing_caret.side))
                                            .bounds()
                                            .axis_bounds
                                            .cmp(&target_bounds)
                                            .into()
                                });
                            self.tile_neighbourhoods
                                .get_mut(&opposing_caret.id)
                                .unwrap()[opposing_caret.side]
                                .insert(index_for_target, target_caret.id);

                            let index_for_replacement =
                                Query::new(self, NeighbourTiles(target_caret))
                                    .items()
                                    .partition_point(|&sib_id| {
                                        Ordering::Greater
                                            != Query::new(
                                                self,
                                                Caret::new(sib_id, target_caret.side),
                                            )
                                            .bounds()
                                            .axis_bounds
                                            .cmp(&opposing_bounds)
                                            .into()
                                    });
                            self.tile_neighbourhoods.get_mut(&target_caret.id).unwrap()
                                [target_caret.side]
                                .insert(index_for_replacement, opposing_caret.id);
                        }
                    }
                }
            }
        }
    }
}

impl QueryContext<Uuid> for Graph {
    type OutKey = NodeId;
    type Error = KeyedError<Uuid, NodeNotFound>;

    fn try_query(&self, key: Uuid) -> Result<Query<Self::OutKey>, Self::Error> {
        let id = NodeId(key);
        if !self.bounds.contains_key(&id) {
            Err(NodeNotFound).with_context(|_| KeyedSnafu { key })
        } else {
            Ok(Query::new(self, id))
        }
    }
}
