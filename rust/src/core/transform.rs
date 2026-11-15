use crate::core::geometry::Axis::{Horizontal, Vertical};
use crate::core::geometry::Sign::{Negative, Positive};
use crate::core::geometry::{Axis, Bounds, BoundsOrdering, Cardinal, Sign};
use crate::core::graph::{Graph, NodeId};
use crate::core::navigation::CaretIterExt;
use crate::core::navigation::ElementNavigation;
use crate::core::navigation::Line;
use crate::core::navigation::RawInformationSource;
use crate::core::navigation::{
    Area, CaretInformationMut, ElementInformation1D, LineInformation,
};
use crate::core::navigation::{AreaInformation, Caret};
use itertools::Itertools;
use std::cmp::PartialEq;

/// MinimalOutward means that the transform container must extend outward in the direction of the reference side if there are any tiles in that direction.
/// That pattern is necessary for deletion and side dragging operations
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TransformConstraints {
    Maximal,
    Minimal,
    OutwardMinimal,
}

#[derive(Clone, Debug)]
pub struct SegmentedContainer {
    pub segments: Vec<Area>,
    pub axis: Axis,
}

impl SegmentedContainer {
    pub fn new(segments: Vec<Area>, axis: Axis) -> Self {
        Self { segments, axis }
    }

    pub fn area(&self) -> Area {
        let start = self.segments.first().unwrap();
        let end = self.segments.last().unwrap();
        match self.axis {
            Horizontal => Area {
                north_west: start.north_west,
                north_east: end.north_west,
                south_east: end.south_east,
                south_west: start.south_west,
            },
            Vertical => Area {
                north_west: start.north_west,
                north_east: start.north_east,
                south_east: end.south_east,
                south_west: end.south_west,
            },
        }
    }

    pub fn as_single_tile(&self) -> Option<NodeId> {
        if self.segments.len() == 1 {
            self.segments[0].into_single_tile()
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub enum TransformSelection {
    Negative(SegmentedContainer),
    Positive(SegmentedContainer),
    Bidirectional(SegmentedContainer, SegmentedContainer),
}

impl TransformSelection {
    pub fn part(&self, sign: Sign) -> Option<&SegmentedContainer> {
        match (sign, self) {
            (Negative, TransformSelection::Negative(fragment))
            | (Positive, TransformSelection::Positive(fragment))
            | (Negative, TransformSelection::Bidirectional(fragment, _))
            | (Positive, TransformSelection::Bidirectional(_, fragment)) => Some(fragment),
            _ => None,
        }
    }

    pub fn parts(self) -> (Option<SegmentedContainer>, Option<SegmentedContainer>) {
        match self {
            TransformSelection::Negative(neg) => (Some(neg), None),
            TransformSelection::Positive(pos) => (None, Some(pos)),
            TransformSelection::Bidirectional(neg, pos) => (Some(neg), Some(pos)),
        }
    }

    pub fn parts_ref(&self) -> (Option<&SegmentedContainer>, Option<&SegmentedContainer>) {
        match self {
            TransformSelection::Negative(neg) => (Some(neg), None),
            TransformSelection::Positive(pos) => (None, Some(pos)),
            TransformSelection::Bidirectional(neg, pos) => (Some(neg), Some(pos)),
        }
    }

    pub fn area(&self) -> Area {
        match self {
            TransformSelection::Negative(neg) => neg.area(),
            TransformSelection::Positive(pos) => pos.area(),
            TransformSelection::Bidirectional(neg, pos) => {
                let start = neg.segments.first().unwrap();
                let end = pos.segments.last().unwrap();
                match neg.axis {
                    Horizontal => Area {
                        north_west: start.north_west,
                        north_east: end.north_west,
                        south_east: end.south_east,
                        south_west: start.south_west,
                    },
                    Vertical => Area {
                        north_west: start.north_west,
                        north_east: start.north_east,
                        south_east: end.south_east,
                        south_west: end.south_west,
                    },
                }
            }
        }
    }
}

fn finalize_gutter(graph: &Graph, gutter: &mut Line, constraints: TransformConstraints) {
    match constraints {
        TransformConstraints::Minimal => {}
        TransformConstraints::Maximal => {
            if let Some(id) = graph
                .info(gutter.caret(Negative))
                .iter(Negative)
                .ids()
                .last()
            {
                gutter[Negative] = id;
            }
            if let Some(id) = graph
                .info(gutter.caret(Positive))
                .iter(Positive)
                .ids()
                .last()
            {
                gutter[Positive] = id;
            }
        }
        TransformConstraints::OutwardMinimal => {
            let (cross_neg, cross_pos) = gutter.side.perpendicular_directions();
            if let Some(c) = graph
                .info(gutter.caret(Negative))
                .iter(Negative)
                .inclusive()
                .find(|c| c.with_side(cross_neg).peek(gutter.side.sign).is_some())
            {
                gutter[Negative] = c.id;
            }
            if let Some(c) = graph
                .info(gutter.caret(Positive))
                .iter(Positive)
                .inclusive()
                .find(|c| c.with_side(cross_pos).peek(gutter.side.sign).is_some())
            {
                gutter[Positive] = c.id;
            }
        }
    }
}

/// this is the method you'd generally use when inserting tiles (unsure what strategy to employ for removals yet)
/// only works if starting from a tile, which is intended
fn find_selection(graph: &Graph, line: Line) -> TransformSelection {
    assert!(graph.info(line).is_connected());

    let inward = SegmentedContainer::new(
        find_segments(graph, (line[Negative], line[Positive]), None, !line.side).collect(),
        line.side.axis,
    );

    let maybe_outward_segments = {
        let initial_neg = graph.info(line.caret(Negative)).peek(line.side.sign);
        let initial_pos = graph.info(line.caret(Positive)).peek(line.side.sign);
        match (initial_neg, initial_pos) {
            (Some(neg), Some(pos)) => {
                if graph
                    .info(Line::new(neg.id, pos.id, !line.side))
                    .is_connected()
                {
                    Some(SegmentedContainer::new(
                        find_segments(graph, (neg.id, pos.id), None, line.side).collect(),
                        line.side.axis,
                    ))
                } else {
                    None
                }
            }
            _ => None,
        }
    };

    // the rest of this is just normalising the above backward/forward parts to collapse intermediate Nones and arranged them in natural order
    match maybe_outward_segments {
        (None) => {
            if line.side.sign == Positive {
                TransformSelection::Negative(inward)
            } else {
                TransformSelection::Positive(inward)
            }
        }
        Some(outward_fragment) => {
            if line.side.sign == Positive {
                TransformSelection::Bidirectional(inward, outward_fragment)
            } else {
                TransformSelection::Bidirectional(outward_fragment, inward)
            }
        }
        _ => panic!(),
    }
}

/// note: there is a strict contractual expectation that the starting tile(s) are
///     in a clean parallel line on the side opposite to the direction we are building in.
/// note: we're actually building in the direction opposite to that of our valid and well-connected gutter, since that ensures the gutter is strictly included.
pub(super) fn find_segments(
    graph: &Graph,
    start: (NodeId, NodeId),
    end: Option<(NodeId, NodeId)>,
    build_direction: Cardinal,
) -> impl Iterator<Item = Area> {
    let mut iter = graph
        .info(Line::new(start.0, start.1, !build_direction))
        .iter(build_direction.sign)
        .include_if(|line| line.is_tile())
        .take_while_inclusive(move |line| {
            end.is_none_or(|(neg_end, pos_end)| {
                neg_end != line[Positive] && pos_end != line[Negative]
            })
        });
    std::iter::from_fn(move || {
        if let (Some(each_start), Some(each_end)) = (iter.next(), iter.next()) {
            let (a, b) = (each_start[Negative], each_start[Positive]);
            let (c, d) = (each_end[Negative], each_end[Positive]);
            Some(match build_direction {
                Cardinal {
                    axis: Horizontal,
                    sign: Positive,
                } => Area {
                    north_west: a,
                    north_east: c,
                    south_east: d,
                    south_west: b,
                },
                Cardinal {
                    axis: Horizontal,
                    sign: Negative,
                } => Area {
                    north_west: c,
                    north_east: a,
                    south_east: b,
                    south_west: d,
                },
                Cardinal {
                    axis: Vertical,
                    sign: Positive,
                } => Area {
                    north_west: a,
                    north_east: b,
                    south_west: c,
                    south_east: d,
                },
                Cardinal {
                    axis: Vertical,
                    sign: Negative,
                } => Area {
                    north_west: c,
                    north_east: d,
                    south_west: a,
                    south_east: b,
                },
            })
        } else {
            None
        }
    })
}

/// low-level resizing is always done with one side fixed in place (only the non-anchored side actually moves)
fn transform_node(
    graph: &mut Graph,
    node_id: NodeId,
    axis: Axis,
    new_bounds: Bounds,
    packing_tiebreaker: Sign,
) {
    graph.bounds.get_mut(&node_id).unwrap()[axis] = new_bounds;
    if node_id.is_surface() {
        let container = graph.info(Area::from(node_id)).into_segmented(axis);
        transform_container(graph, &container, new_bounds, packing_tiebreaker);
    }
}

/// low-level resizing is always done with one side fixed in place (only the non-anchored side actually moves)
fn transform_container(
    graph: &mut Graph,
    container: &SegmentedContainer,
    new_bounds: Bounds,
    packing_tiebreaker: Sign,
) {
    if let Some(node_id) = container.as_single_tile() {
        transform_node(
            graph,
            node_id,
            container.axis,
            new_bounds,
            packing_tiebreaker,
        );
        return;
    }

    let new_segment_bounds = if container.segments.len() == 1 {
        vec![new_bounds]
    } else {
        let old_container_bounds = graph.info(container.area()).bounds()[container.axis];
        let old_container_size = old_container_bounds.size();
        let new_container_size = new_bounds.size();
        let (mut result, repairs): (Vec<Bounds>, Vec<Bounds>) = container
            .segments
            .iter()
            .map(|&segment| {
                let original = graph.info(segment).bounds()[container.axis];
                let clamped = original.clamp(&old_container_bounds);
                let repair = Bounds::new(
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
            .info(*segment)
            .segments(!container.axis)
            .collect::<Vec<_>>()
        {
            let subcontainer = graph.info(cross_segment).into_segmented(container.axis);
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
        let scope = graph.info(scope);
        let replacement_scope = graph.info(replacement_scope);
        assert!(scope.is_connected());
        assert!(replacement_scope.is_connected());
        assert_eq!(scope.location(), replacement_scope.location());
    }
    let originals: Vec<Caret> = graph
        .info(scope.caret(Negative))
        .iter(Positive)
        .inclusive()
        .map(|info| *info)
        .take_while_inclusive(|&caret| caret.id != scope[Negative])
        .collect();

    let opposites: Vec<_> = originals.iter().flat_map(|caret| graph.get_tile_neighbourhood(*caret).iter().copied()).collect();
    if opposites.len() == 0 {
        return;
    }

    let replacements: Vec<_> = graph
        .info(replacement_scope.caret(Negative))
        .iter(Positive)
        .inclusive()
        .map(|info| *info)
        .take_while_inclusive(|&caret| caret.id != replacement_scope[Positive])
        .collect();



    for opposing_id in opposites 
    {
        let opposing_caret = Caret::new(opposing_id, !scope.side);
        let opposing_bounds = graph.info(opposing_caret).bounds();
        let removals: Vec<_> = originals.iter().filter_map(|&caret| match graph.info(caret).bounds().cmp(&opposing_bounds) {
            BoundsOrdering::Before | BoundsOrdering::After => Some(caret),
            _ => None,
        }).collect();

        for removal_caret in removals {
            graph.get_tile_neighbourhood_mut(removal_caret).remove(&opposing_id);
            graph.get_tile_neighbourhood_mut(opposing_caret).remove(&removal_caret.id);
        }

        let additions: Vec<_> = originals.iter().filter_map(|&caret| match graph.info(caret).bounds().cmp(&opposing_bounds) {
            BoundsOrdering::Before | BoundsOrdering::After => None,
            _ => Some(caret),
        })
            .collect();

        for addition_caret in additions {
            graph.get_tile_neighbourhood_mut(addition_caret).insert(opposing_id);
            graph.get_tile_neighbourhood_mut(opposing_caret).insert(addition_caret.id)
        }
    }
}
