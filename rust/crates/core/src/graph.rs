use crate::error::{Error, KeyedError, KeyedSnafu};
use crate::geometry::{AreaBounds, Axis, Axis::*, AxisBounds, BoundsOrdering, Cardinal, Sign, Sign::*};
use crate::key::NodeKey;
use crate::key::QueryKey;
use crate::line::Line;
use crate::node::{NodeNotFound, NodeNotFoundSnafu};
use crate::query::{ElementInformation, ElementInformation1D, ElementInformation2D, ElementKind, GroupInformation, Query};
use crate::source::{QuerySource, RawQuerySource};
use markers_derive::NodeKey;
use serde::{Deserialize, Serialize};
use snafu::{ensure, ResultExt};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Index, IndexMut};
use uuid::Uuid;
use crate::area::Area;
use crate::caret::Caret;
use crate::command::transform::SegmentedArea;

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
    pub(crate) bounds: HashMap<NodeId, AreaBounds>,

    pub(crate) containing_surface: HashMap<NodeId, NodeId>,
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

    /// low-level resizing is always done with one side fixed in place (only the non-anchored side actually moves)
    fn transform_node(
        graph: &mut Graph,
        node_id: NodeId,
        axis: Axis,
        new_bounds: AxisBounds,
        packing_tiebreaker: Sign,
    ) {
        graph.bounds.get_mut(&node_id).unwrap()[axis] = new_bounds;
        if graph.query(node_id).kind() == ElementKind::Surface {
            let area = graph
                .query(Area::from(node_id))
                .into_segmented(Cardinal::new(axis, Positive));
            transform_area(graph, &area, new_bounds, packing_tiebreaker);
        }
    }

    /// low-level resizing is always done with one side fixed in place (only the non-anchored side actually moves)
    fn transform_area(
        graph: &mut Graph,
        container: &SegmentedArea,
        new_bounds: AxisBounds,
        packing_tiebreaker: Sign,
    ) {
        if let Ok(node) = NodeId::try_from(container.area()) {
            transform_node(graph, node, container.axis, new_bounds, packing_tiebreaker);
            return;
        }

        let new_segment_bounds = if container.segments.len() == 1 {
            vec![new_bounds]
        } else {
            let old_container_bounds = graph.query(container.area()).bounds()[container.axis];
            let old_container_size = old_container_bounds.size();
            let new_container_size = new_bounds.size();
            let (mut result, repairs): (Vec<AxisBounds>, Vec<AxisBounds>) = container
                .segments
                .iter()
                .map(|&segment| {
                    let original = graph.query(segment).bounds()[container.axis];
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
                        + f64::floor(segment_bounds.size() as f64 * scale) as i32;
                }
            } else {
                let portion =
                    f64::floor(new_container_size as f64 / container.segments.len() as f64) as i32;
                for segment_bounds in result.iter_mut() {
                    segment_bounds[Positive] = segment_bounds[Negative] + portion;
                }
            };

            let estimated_total: i32 = result.iter().map(|bounds| bounds.size()).sum();
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
                let size = segment_bounds.size();
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
            for cross_segment in graph
                .query(*segment)
                .into_segments(!container.axis)
                .collect::<Vec<_>>()
            {
                let subcontainer = graph.query(cross_segment).into_segmented(container.axis);
                transform_area(graph, &subcontainer, segment_bounds, packing_tiebreaker);
            }
        }
    }

    fn repair_connections(graph: &mut Graph, target: Line) {
        update_connections(graph, target, target[Positive], target[Negative]);
    }

    fn update_connections(
        graph: &mut Graph,
        scope: Line,
        replacement_neg: NodeId,
        replacement_pos: NodeId,
    ) {
        let replacement_scope = Line::new(replacement_neg, replacement_pos, scope.side);
        {
            let scope = graph.query(scope);
            let replacement_scope = graph.query(replacement_scope);
            assert!(scope.is_connected());
            assert!(replacement_scope.is_connected());
            assert_eq!(
                scope.bounds().side_location,
                replacement_scope.side_location()
            );
        }
        let originals: Vec<_> = graph.query(scope).into_range().map(|c| *c).collect();

        let opposites: Vec<Caret> = originals
            .iter()
            .filter_map(|caret| graph.inspect_tile_neighbourhood(*caret).try_into().ok())
            .flatten()
            .copied()
            .collect();
        if opposites.len() == 0 {
            return;
        }

        let replacements: Vec<_> = graph
            .query(replacement_scope)
            .into_range()
            .map(|c| *c)
            .collect();

        for opposing_caret in opposites {
            let opposing_bounds = graph.query(opposing_caret).bounds();
            let removals: Vec<_> = originals
                .iter()
                .filter_map(
                    |&caret| match graph.query(caret).bounds().cmp(&opposing_bounds) {
                        BoundsOrdering::Before | BoundsOrdering::After => Some(caret),
                        _ => None,
                    },
                )
                .collect();

            for removal_caret in removals {
                graph
                    .inspect_tile_neighbourhood_mut(removal_caret)
                    .remove(&opposing_id);
                graph
                    .inspect_tile_neighbourhood_mut(opposing_caret)
                    .remove(&removal_caret.id);
            }

            let additions: Vec<_> = originals
                .iter()
                .filter_map(
                    |&caret| match graph.query(caret).bounds().cmp(&opposing_bounds) {
                        BoundsOrdering::Before | BoundsOrdering::After => None,
                        _ => Some(caret),
                    },
                )
                .collect();

            for addition_caret in additions {
                graph
                    .inspect_tile_neighbourhood_mut(addition_caret)
                    .insert(opposing_id);
                graph
                    .inspect_tile_neighbourhood_mut(opposing_caret)
                    .insert(addition_caret.id)
            }
        }
    }

}

impl QuerySource<Uuid> for Graph {
    type OutKey = NodeId;
    type Error = KeyedError<Uuid, NodeNotFound>;

    fn try_query(&self, key: Uuid) -> Result<Query<Self::OutKey>, Self::Error> {
        let id = NodeId(key);
        if !self.bounds.contains_key(&id) {
            Err(NodeNotFound).with_context(|_|KeyedSnafu { key })
        } else {
            Ok(self.query(id))
        }
    }
}
