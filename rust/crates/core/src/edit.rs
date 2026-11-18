use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{Axis, AxisBounds, BoundsOrdering, Cardinal, Sign};
use crate::graph::{Graph, NodeId};
use crate::query::ElementInformation1D;
use crate::query::ElementNavigation;
use crate::query::area::{Area, AreaInformation};
use crate::query::caret::{Caret, CaretIterExt, CaretLookup};
use crate::query::iter::ElementIteration;
use crate::query::line::{Line, LineLookup};
use crate::query::node::NodeLookup;
use crate::query::source::RawQuerySource;
use crate::query::{
    ElementInformation, ElementInformation2D, ElementKind, GroupInformation, Query,
};
use std::cmp::PartialEq;

fn finalize_gutter(graph: &Graph, gutter: &mut Line, constraints: TransformConstraints) {
    match constraints {
        TransformConstraints::Minimal => {}
        TransformConstraints::Maximal => {
            if let Some(id) = graph
                .query(*gutter)
                .into_end(Negative)
                .into_iter(Negative)
                .ids()
                .last()
            {
                gutter[Negative] = id;
            }
            if let Some(id) = graph
                .query(*gutter)
                .into_end(Positive)
                .into_iter(Positive)
                .ids()
                .last()
            {
                gutter[Positive] = id;
            }
        }
        TransformConstraints::OutwardMinimal => {
            let (cross_neg, cross_pos) = gutter.side.perpendicular_directions();
            if let Some(c) = graph
                .query(*gutter)
                .into_end(Negative)
                .into_iter(Negative)
                .inclusive()
                .find(|c| c.with_side(cross_neg).peek(gutter.side.sign).is_some())
            {
                gutter[Negative] = c.id;
            }
            if let Some(c) = graph
                .query(*gutter)
                .into_end(Positive)
                .into_iter(Positive)
                .inclusive()
                .find(|c| c.with_side(cross_pos).peek(gutter.side.sign).is_some())
            {
                gutter[Positive] = c.id;
            }
        }
    }
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
        let container = graph
            .query(Area::from(node_id))
            .into_segmented(Cardinal::new(axis, Positive));
        transform_container(graph, &container, new_bounds, packing_tiebreaker);
    }
}

/// low-level resizing is always done with one side fixed in place (only the non-anchored side actually moves)
fn transform_container(
    graph: &mut Graph,
    container: &SegmentedContainer,
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
            transform_container(graph, &subcontainer, segment_bounds, packing_tiebreaker);
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
