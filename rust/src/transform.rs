use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{Axis, AxisBounds, Bounds, Cardinal, NORTH, SOUTH, Sign, WEST};
use crate::graph::{Graph, NodeId};
use crate::gutter::{GutterIterExt, GutterNavigation};
use crate::navigation::{Caret, Line, Navigator};

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

    pub fn get_bounds(&self, graph: &Graph) -> Bounds {
        let min_x = Line::new(self.north_west, self.south_west, WEST).get_location(graph);
        let min_y = Line::new(self.north_east, self.north_west, NORTH).get_location(graph);
        let max_x = Line::new(self.north_east, self.south_east, WEST).get_location(graph);
        let max_y = Line::new(self.south_east, self.south_west, SOUTH).get_location(graph);
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

    pub fn get_segments(&self, graph: &Graph, axis: Axis) -> Vec<TransformRect> {
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
        self.get_rect().get_bounds(graph)
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
        self.get_rect().get_bounds(graph)
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
            if let Some(id) = gutter
                .start()
                .navigate_gutter(graph)
                .iter(Negative)
                .ids()
                .last()
            {
                gutter.start_id = id;
            }
            if let Some(id) = gutter
                .end()
                .navigate_gutter(graph)
                .iter(Positive)
                .ids()
                .last()
            {
                gutter.end_id = id;
            }
        }
        TransformConstraints::OutwardMinimal => {
            let (cross_neg, cross_pos) = gutter.side.perpendicular_directions();
            if let Some(c) = gutter
                .start()
                .navigate_gutter(graph)
                .iter(Negative)
                .inclusive()
                .find(|c| c.clone().face(cross_neg).peek(gutter.side.sign).is_some())
            {
                gutter.start_id = c.node_id;
            }
            if let Some(c) = gutter
                .end()
                .navigate_gutter(graph)
                .iter(Positive)
                .inclusive()
                .find(|c| c.clone().face(cross_pos).peek(gutter.side.sign).is_some())
            {
                gutter.end_id = c.node_id;
            }
        }
    }
}

/// this is the method you'd generally use when inserting tiles (unsure what strategy to employ for removals yet)
/// only works if starting from a tile, which is intended
fn find_selection(graph: &Graph, gutter: Line) -> TransformSelection {
    assert!(gutter.is_connected_on(graph));

    let inward = SegmentedTransformContainer::new(
        find_segments(graph, (gutter.start_id, gutter.end_id), None, !gutter.side),
        gutter.side.axis,
    );

    let maybe_outward_segments = {
        let initial_neg = gutter.start().navigate_gutter(graph).peek(gutter.side.sign);
        let initial_pos = gutter.end().navigate_gutter(graph).peek(gutter.side.sign);
        match (initial_neg, initial_pos) {
            (Some(neg), Some(pos)) => {
                if Line::new(neg.node_id, pos.node_id, !gutter.side).is_connected_on(graph) {
                    Some(SegmentedTransformContainer::new(
                        find_segments(
                            graph,
                            (neg.node_id, pos.node_id),
                            None,
                            gutter.side,
                        ),
                        gutter.side.axis,
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
            if gutter.side.sign == Positive {
                TransformSelection::Negative(inward)
            } else {
                TransformSelection::Positive(inward)
            }
        }
        Some(outward_fragment) => {
            if gutter.side.sign == Positive {
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
) -> Vec<TransformRect> {
    assert!(Line::new(start.0, start.1, !build_direction).is_connected_on(graph));
    let (cross_neg, cross_pos) = build_direction.perpendicular_directions();
    let mut segments = Vec::new();
    let mut current_start = start;
    let mut current_neg = Caret::new(start.0, cross_neg).navigate_gutter(graph);
    let mut current_pos = Caret::new(start.1, cross_pos).navigate_gutter(graph);
    let mut neg_iter = current_neg.iter(build_direction.sign);
    let mut pos_iter = current_pos.iter(build_direction.sign);

    while let (Some(next_neg), Some(next_pos)) = (neg_iter.next(), pos_iter.next()) {
        current_neg = next_neg;
        current_pos = next_pos;
        if end.is_some_and(|(neg_end, pos_end)| {
            current_neg.node_id == neg_end || next_pos.node_id == pos_end
        }) {
            break;
        }
        if Line::new(current_neg.node_id, current_pos.node_id, build_direction)
            .is_connected_on(graph)
        {
            segments.push(orient_rect(
                current_start,
                (next_neg.node_id, next_pos.node_id),
                build_direction,
            ));
            if let (Some(next_neg), Some(next_pos)) = (neg_iter.next(), pos_iter.next()) {
                current_neg = next_neg;
                current_pos = next_pos;
                current_start = (current_neg.node_id, current_neg.node_id);
                continue; // by definition of being next after our connected line, they are also connected, so we can skip the location check.
            }
        }

        let mut neg_location = next_neg.location();
        let mut pos_location = next_pos.location();

        while neg_location != pos_location {
            if neg_location < pos_location {
                if let Some(next_neg) = neg_iter.next() {
                    current_neg = next_neg;
                    neg_location = next_neg.location();
                } else {
                    break;
                }
            } else if neg_location > pos_location {
                if let Some(next_pos) = pos_iter.next() {
                    current_pos = next_pos;
                    pos_location = next_pos.location()
                } else {
                    break;
                }
            }
        }
    }

    if build_direction.sign == Negative {
        segments.reverse();
    }
    segments
}

/// low-level resizing is always done with one side fixed in place (only the non-anchored side actually moves)
fn transform_tile(
    graph: &mut Graph,
    node_id: NodeId,
    axis: Axis,
    new_bounds: AxisBounds,
    packing_tiebreaker: Sign,
) {
    graph.bounds.get_mut(&node_id).unwrap()[axis] = new_bounds;
    // todo: this should check if the tile happens to be a surface and if so apply the transform internally, but we'll deal with that later
}

/// low-level resizing is always done with one side fixed in place (only the non-anchored side actually moves)
fn apply_new_bounds(
    graph: &mut Graph,
    container: &SegmentedTransformContainer,
    new_bounds: AxisBounds,
    packing_tiebreaker: Sign,
) {
    if let Some(node_id) = container.single_tile() {
        transform_tile(graph, node_id, container.axis, new_bounds, packing_tiebreaker);
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
                let original = segment.get_bounds(graph)[container.axis];
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
        if remainder as usize >= container.segments.len() {
            let over_remainder = remainder / container.segments.len() as i32;
            for bounds in result.iter_mut() {
                bounds.size += over_remainder;
            }
            remainder %= container.segments.len() as i32;
        }
        if remainder > 0 {
            match packing_tiebreaker {
                Negative => {
                    for bounds in result.iter_mut().take(remainder as usize) {
                        bounds.size += 1
                    }
                }
                Positive => {
                    for bounds in result.iter_mut().rev().take(remainder as usize) {
                        bounds.size += 1
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
        for cross_segment in segment.get_segments(graph, !container.axis) {
            let subcontainer = SegmentedTransformContainer::new(
                cross_segment.get_segments(graph, container.axis),
                container.axis,
            );
            apply_new_bounds(graph, &subcontainer, segment_bounds, packing_tiebreaker)
        }
    }
}

fn orient_rect(
    start: (NodeId, NodeId),
    end: (NodeId, NodeId),
    direction: Cardinal,
) -> TransformRect {
    let (a, b) = start;
    let (c, d) = end;
    match direction {
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
    }
}
