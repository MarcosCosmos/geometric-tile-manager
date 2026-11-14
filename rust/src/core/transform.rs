use crate::core::geometry::Axis::{Horizontal, Vertical};
use crate::core::geometry::Sign::{Negative, Positive};
use crate::core::geometry::{Axis, AxisBounds, Bounds, BoundsOrdering, Cardinal, Sign, EAST, NORTH, SOUTH, WEST};
use crate::core::graph::{Graph, NodeId};
use crate::core::navigation::{Caret, CaretIterExt, CaretNavigation, Line, LineNavigation, Nav, Navigation};
use std::cmp::PartialEq;
use std::ops::Index;
use std::os::linux::raw::ino_t;
use itertools::Itertools;

/// MinimalOutward means that the transform container must extend outward in the direction of the reference side if there are any tiles in that direction.
/// That pattern is necessary for deletion and side dragging operations
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TransformConstraints {
    Maximal,
    Minimal,
    OutwardMinimal,
}

/// Generally dividers will be clean lines that don't intersect any tiles,
/// but the start or end may or may not result in cutting through some tiles
/// Note: it is possible for a transform container to start and end with the same pair of ids and have no dividers.
///     in this case it is a single row/column or even single tile.
#[derive(Copy, Clone, Debug)]
pub struct TransformRect {
    pub north_west: NodeId,
    pub north_east: NodeId,
    pub south_west: NodeId,
    pub south_east: NodeId,
}

impl From<NodeId> for TransformRect {
    fn from(value: NodeId) -> Self {
        TransformRect {
            north_west: value,
            north_east: value,
            south_west: value,
            south_east: value,
        }
    }
}

impl TransformRect {
    pub fn single_tile(&self) -> Option<NodeId> {
        if self.north_west == self.north_east
            && self.north_west == self.south_east
            && self.north_west == self.south_west
        {
            Some(self.north_west)
        } else {
            None
        }
    }

    pub fn side(&self, side: Cardinal) -> Line {
        match side {
            Cardinal {
                axis: Horizontal,
                sign: Negative,
            } => Line::new(self.north_west, self.south_west, side),
            Cardinal {
                axis: Horizontal,
                sign: Positive,
            } => Line::new(self.north_east, self.south_east, side),
            Cardinal {
                axis: Vertical,
                sign: Negative,
            } => Line::new(self.north_east, self.north_west, side),
            Cardinal {
                axis: Vertical,
                sign: Positive,
            } => Line::new(self.south_east, self.south_west, side),
        }
    }

    pub fn bounds(&self, graph: &Graph) -> Bounds {
        let min_x = Nav::new(graph, Line::new(self.north_west, self.south_west, WEST)).location();
        let min_y = Nav::new(graph, Line::new(self.north_east, self.north_west, NORTH)).location();
        let max_x = Nav::new(graph, Line::new(self.north_east, self.south_east, WEST)).location();
        let max_y = Nav::new(graph, Line::new(self.south_east, self.south_west, SOUTH)).location();
        Bounds {
            horizontal: AxisBounds {
                origin: min_x,
                size: max_x - min_x,
            },
            vertical: AxisBounds {
                origin: min_y,
                size: max_y - min_y,
            },
        }
    }

    pub fn segments(&self, graph: &Graph, axis: Axis) -> impl Iterator<Item = TransformRect> {
        match axis {
            Horizontal => find_segments(
                graph,
                (self.north_west, self.south_west),
                Some((self.south_east, self.south_west)),
                Cardinal {
                    axis: Horizontal,
                    sign: Positive,
                },
            ),
            Vertical => find_segments(
                graph,
                (self.north_west, self.north_east),
                Some((self.south_west, self.south_east)),
                Cardinal {
                    axis: Vertical,
                    sign: Positive,
                },
            ),
        }
    }

    pub fn into_segmented(&self, graph: &Graph, axis: Axis) -> SegmentedTransformContainer {
        SegmentedTransformContainer::new(self.segments(graph, axis).collect(), axis)
    }
}

#[derive(Clone, Debug)]
pub struct SegmentedTransformContainer {
    pub segments: Vec<TransformRect>,
    pub axis: Axis,
}

impl SegmentedTransformContainer {
    pub fn new(segments: Vec<TransformRect>, axis: Axis) -> Self {
        Self { segments, axis }
    }
    pub fn get_rect(&self) -> TransformRect {
        let start = self.segments.first().unwrap();
        let end = self.segments.last().unwrap();
        match self.axis {
            Horizontal => TransformRect {
                north_west: start.north_west,
                north_east: end.north_west,
                south_east: end.south_east,
                south_west: start.south_west,
            },
            Vertical => TransformRect {
                north_west: start.north_west,
                north_east: start.north_east,
                south_east: end.south_east,
                south_west: end.south_west,
            },
        }
    }

    pub fn get_bounds(&self, graph: &Graph) -> Bounds {
        self.get_rect().bounds(graph)
    }

    pub fn single_tile(&self) -> Option<NodeId> {
        if self.segments.len() == 1 {
            self.segments[0].single_tile()
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub enum TransformSelection {
    Negative(SegmentedTransformContainer),
    Positive(SegmentedTransformContainer),
    Bidirectional(SegmentedTransformContainer, SegmentedTransformContainer),
}

impl TransformSelection {
    pub fn get_sign(&self, sign: Sign) -> Option<&SegmentedTransformContainer> {
        match (sign, self) {
            (Negative, TransformSelection::Negative(fragment)) => Some(fragment),
            (Positive, TransformSelection::Positive(fragment)) => Some(fragment),
            (Negative, TransformSelection::Bidirectional(fragment, _)) => Some(fragment),
            (Positive, TransformSelection::Bidirectional(_, fragment)) => Some(fragment),
            _ => None,
        }
    }

    pub fn parts(
        self,
    ) -> (
        Option<SegmentedTransformContainer>,
        Option<SegmentedTransformContainer>,
    ) {
        match self {
            TransformSelection::Negative(neg) => (Some(neg), None),
            TransformSelection::Positive(pos) => (None, Some(pos)),
            TransformSelection::Bidirectional(neg, pos) => (Some(neg), Some(pos)),
        }
    }

    pub fn parts_ref(
        &self,
    ) -> (
        Option<&SegmentedTransformContainer>,
        Option<&SegmentedTransformContainer>,
    ) {
        match self {
            TransformSelection::Negative(neg) => (Some(neg), None),
            TransformSelection::Positive(pos) => (None, Some(pos)),
            TransformSelection::Bidirectional(neg, pos) => (Some(neg), Some(pos)),
        }
    }

    pub fn iter(&self) -> TransformContentIterator<&SegmentedTransformContainer> {
        TransformContentIterator(self.parts_ref())
    }

    pub fn get_rect(&self) -> TransformRect {
        match self {
            TransformSelection::Negative(neg) => neg.get_rect(),
            TransformSelection::Positive(pos) => pos.get_rect(),
            TransformSelection::Bidirectional(neg, pos) => {
                let start = neg.segments.first().unwrap();
                let end = pos.segments.last().unwrap();
                match neg.axis {
                    Horizontal => TransformRect {
                        north_west: start.north_west,
                        north_east: end.north_west,
                        south_east: end.south_east,
                        south_west: start.south_west,
                    },
                    Vertical => TransformRect {
                        north_west: start.north_west,
                        north_east: start.north_east,
                        south_east: end.south_east,
                        south_west: end.south_west,
                    },
                }
            }
        }
    }

    pub fn get_bounds(&self, graph: &Graph) -> Bounds {
        self.get_rect().bounds(graph)
    }
}

impl IntoIterator for TransformSelection {
    type Item = (Sign, SegmentedTransformContainer);
    type IntoIter = TransformContentIterator<SegmentedTransformContainer>;

    fn into_iter(self) -> Self::IntoIter {
        TransformContentIterator(self.parts())
    }
}

pub struct TransformContentIterator<T>((Option<T>, Option<T>));

impl<T> Iterator for TransformContentIterator<T> {
    type Item = (Sign, T);

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .0
            .take()
            .map(|fragment| (Negative, fragment))
            .or_else(|| self.0.1.take().map(|fragment| (Positive, fragment)))
    }
}

fn finalize_gutter(graph: &Graph, gutter: &mut Line, constraints: TransformConstraints) {
    match constraints {
        TransformConstraints::Minimal => {}
        TransformConstraints::Maximal => {
            if let Some(id) = Nav::new(graph, gutter.caret(Negative))
                .iter(Negative)
                .ids()
                .last()
            {
                gutter.positive_id = id;
            }
            if let Some(id) = Nav::new(graph, gutter.caret(Positive))
                .iter(Positive)
                .ids()
                .last()
            {
                gutter.negative_id = id;
            }
        }
        TransformConstraints::OutwardMinimal => {
            let (cross_neg, cross_pos) = gutter.side.perpendicular_directions();
            if let Some(c) = Nav::new(graph, gutter.caret(Negative))
                .iter(Negative)
                .inclusive()
                .find(|c| c.with_side(cross_neg).peek(gutter.side.sign).is_some())
            {
                gutter.positive_id = c.node_id;
            }
            if let Some(c) = Nav::new(graph, gutter.caret(Positive))
                .iter(Positive)
                .inclusive()
                .find(|c| c.with_side(cross_pos).peek(gutter.side.sign).is_some())
            {
                gutter.negative_id = c.node_id;
            }
        }
    }
}

/// this is the method you'd generally use when inserting tiles (unsure what strategy to employ for removals yet)
/// only works if starting from a tile, which is intended
fn find_selection(graph: &Graph, line: Line) -> TransformSelection {
    assert!(Nav::new(graph, line).is_connected());

    let inward = SegmentedTransformContainer::new(
        find_segments(
            graph,
            (line.positive_id, line.negative_id),
            None,
            !line.side,
        )
        .collect(),
        line.side.axis,
    );

    let maybe_outward_segments = {
        let initial_neg = Nav::new(graph, line.caret(Negative)).peek(line.side.sign);
        let initial_pos = Nav::new(graph, line.caret(Positive)).peek(line.side.sign);
        match (initial_neg, initial_pos) {
            (Some(neg), Some(pos)) => {
                if Nav::new(graph, Line::new(neg.node_id, pos.node_id, !line.side)).is_connected() {
                    Some(SegmentedTransformContainer::new(
                        find_segments(graph, (neg.node_id, pos.node_id), None, line.side).collect(),
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
fn find_segments(
    graph: &Graph,
    start: (NodeId, NodeId),
    end: Option<(NodeId, NodeId)>,
    build_direction: Cardinal,
) -> impl Iterator<Item = TransformRect> {
    let mut iter = Nav::new(graph, Line::new(start.0, start.1, !build_direction))
        .iter(build_direction.sign)
        .include_if(|line| line.is_tile())
        .take_while_inclusive(move |line| end.is_none_or(|(neg_end, pos_end)| neg_end != line.negative_id && pos_end != line.positive_id));
    std::iter::from_fn(move || {
        if let (Some(each_start), Some(each_end)) = (iter.next(), iter.next()) {
            let (a, b) = (each_start.positive_id, each_start.negative_id);
            let (c, d) = (each_end.positive_id, each_end.negative_id);
            Some(match build_direction {
                Cardinal {
                    axis: Horizontal,
                    sign: Positive,
                } => TransformRect {
                    north_west: a,
                    north_east: c,
                    south_east: d,
                    south_west: b,
                },
                Cardinal {
                    axis: Horizontal,
                    sign: Negative,
                } => TransformRect {
                    north_west: c,
                    north_east: a,
                    south_east: b,
                    south_west: d,
                },
                Cardinal {
                    axis: Vertical,
                    sign: Positive,
                } => TransformRect {
                    north_west: a,
                    north_east: b,
                    south_west: c,
                    south_east: d,
                },
                Cardinal {
                    axis: Vertical,
                    sign: Negative,
                } => TransformRect {
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
    new_bounds: AxisBounds,
    packing_tiebreaker: Sign,
) {
    graph.bounds.get_mut(&node_id).unwrap()[axis] = new_bounds;
    if node_id.is_surface() {
        let container = TransformRect::from(node_id).into_segmented(graph, axis);
        transform_container(graph, &container, new_bounds, packing_tiebreaker);
    }
}

/// low-level resizing is always done with one side fixed in place (only the non-anchored side actually moves)
fn transform_container(
    graph: &mut Graph,
    container: &SegmentedTransformContainer,
    new_bounds: AxisBounds,
    packing_tiebreaker: Sign,
) {
    if let Some(node_id) = container.single_tile() {
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
        let old_container_bounds = container.get_bounds(graph)[container.axis];
        let (mut result, repairs): (Vec<AxisBounds>, Vec<AxisBounds>) = container
            .segments
            .iter()
            .map(|segment| {
                let original = segment.bounds(graph)[container.axis];
                let clamped = original.clamp(old_container_bounds);
                let repair = AxisBounds {
                    origin: original.origin - clamped.origin,
                    size: original.size - clamped.size,
                };
                (clamped, repair)
            })
            .unzip();

        if old_container_bounds.size > 0 {
            let scale = new_bounds.size as f64 / old_container_bounds.size as f64;
            for segment_bounds in result.iter_mut() {
                segment_bounds.size = f64::floor(segment_bounds.size as f64 * scale) as i32;
            }
        } else {
            let portion =
                f64::floor(new_bounds.size as f64 / container.segments.len() as f64) as i32;
            for segment_bounds in result.iter_mut() {
                segment_bounds.size = portion;
            }
        };

        let estimated_total: i32 = result.iter().map(|axis_bounds| axis_bounds.size).sum();
        let mut remainder = new_bounds.size - estimated_total;
        let signum = remainder.signum();
        if remainder as usize >= container.segments.len() {
            let excess_remainder = remainder / container.segments.len() as i32;
            for bounds in result.iter_mut() {
                bounds.size += excess_remainder;
            }
            remainder = remainder.abs() + container.segments.len() as i32;
        }
        if remainder != 0 {
            match packing_tiebreaker {
                Negative => {
                    for bounds in result.iter_mut().take(remainder as usize) {
                        bounds.size += signum
                    }
                }
                Positive => {
                    for bounds in result.iter_mut().rev().take(remainder as usize) {
                        bounds.size += signum;
                    }
                }
            }
        }

        let mut next_origin = new_bounds.origin;

        for segment_bounds in result.iter_mut() {
            segment_bounds.origin = next_origin;
            next_origin += segment_bounds.size;
        }

        for (segment_bounds, repair) in result.iter_mut().zip(repairs.into_iter()) {
            segment_bounds.origin += repair.origin;
            segment_bounds.size += repair.size;
        }

        result
    };

    for (segment, segment_bounds) in container.segments.iter().zip(new_segment_bounds) {
        for cross_segment in segment.segments(graph, !container.axis).collect::<Vec<_>>() {
            let subcontainer = cross_segment.into_segmented(graph, container.axis);
            transform_container(graph, &subcontainer, segment_bounds, packing_tiebreaker);
        }
    }
}

fn repair_connections(graph: &Graph, target: Line) {
    update_connections(graph, target, target.negative_id, target.positive_id);
}

fn update_connections(
    graph: &Graph,
    original: Line,
    replacement_neg: NodeId,
    replacement_pos: NodeId,
) {
    let original = Nav::new(graph, original);
    let replacement = Nav::new(
        graph,
        Line::new(replacement_neg, replacement_pos, original.side),
    );
    assert!(original.is_connected());
    assert!(replacement.is_connected());
    assert_eq!(original.location(), replacement.location());

    let original_span = original.bounds();
    
    let originals = Nav::new(graph, original.caret(Negative))
        .iter(Positive)
        .inclusive()
        .take_while_inclusive(|c| c.node_id != original.positive_id);

    let opposites = 
        .flat_map(|c| graph.intra_surface_neighbourhoods[&c.node_id][original.side].iter().copied())
        .filter(|&node_id| match Nav::new(graph, Caret::new(node_id, !original.side)).bounds().cmp(&original_span) {
            BoundsOrdering::Before | BoundsOrdering::After => false,
            _ => true,
        });

    let replacements = Nav::new(graph, replacement.caret(Negative)).iter(Positive).inclusive().take_while_inclusive(|c| c.node_id != replacement.negative_id);
    
}
