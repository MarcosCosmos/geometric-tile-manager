use serde::{Deserialize, Serialize};
use crate::command::{AddTile, DeleteNode, MoveLine, SelectionCommand};
use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{Axis, Sign};
use crate::query::{Query, QueryKey};
use crate::query::area::Area;
use crate::query::line::{Line, LineQuery};
use markers_derive::QueryKey;
use crate::graph::Graph;
use crate::query::source::RawQuerySource;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey)]
#[node(<C::Key as QueryKey>::NodeKey)]
pub struct Selection<C>(pub C)
where
    C: SelectionCommand<Key: QueryKey> + Into<Line<<C::Key as QueryKey>::NodeKey>>;

impl<C> Into<Line<<C::Key as QueryKey>::NodeKey>> for Selection<C>
where
    C: SelectionCommand<Key: QueryKey> + Into<Line<<C::Key as QueryKey>::NodeKey>>,
{
    fn into(self) -> Line<<C::Key as QueryKey>::NodeKey> {
        self.0.into()
    }
}

/// MinimalOutward means that the transform container must extend outward in the direction of the reference side if there are any tiles in that direction.
/// That pattern is necessary for deletion and side dragging operations
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub enum TransformConstraints {
    Maximal,
    Minimal,
    OutwardMinimal,
}

#[derive(Clone, Debug, Serialize)]
pub struct SegmentedArea {
    pub segments: Vec<Area>,
    pub axis: Axis,
}

impl SegmentedArea {
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
}

#[derive(Clone, Debug, Serialize)]
pub enum SelectionDetails {
    Negative(SegmentedArea),
    Positive(SegmentedArea),
    Bidirectional(SegmentedArea, SegmentedArea),
}

impl SelectionDetails {
    pub fn part(&self, sign: Sign) -> Option<&SegmentedArea> {
        match (sign, self) {
            (Negative, SelectionDetails::Negative(fragment))
            | (Positive, SelectionDetails::Positive(fragment))
            | (Negative, SelectionDetails::Bidirectional(fragment, _))
            | (Positive, SelectionDetails::Bidirectional(_, fragment)) => Some(fragment),
            _ => None,
        }
    }

    pub fn parts(self) -> (Option<SegmentedArea>, Option<SegmentedArea>) {
        match self {
            SelectionDetails::Negative(neg) => (Some(neg), None),
            SelectionDetails::Positive(pos) => (None, Some(pos)),
            SelectionDetails::Bidirectional(neg, pos) => (Some(neg), Some(pos)),
        }
    }

    pub fn parts_ref(&self) -> (Option<&SegmentedArea>, Option<&SegmentedArea>) {
        match self {
            SelectionDetails::Negative(neg) => (Some(neg), None),
            SelectionDetails::Positive(pos) => (None, Some(pos)),
            SelectionDetails::Bidirectional(neg, pos) => (Some(neg), Some(pos)),
        }
    }

    pub fn area(&self) -> Area {
        match self {
            SelectionDetails::Negative(neg) => neg.area(),
            SelectionDetails::Positive(pos) => pos.area(),
            SelectionDetails::Bidirectional(neg, pos) => {
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

pub trait SelectionInformation {
    fn details(&self) -> SelectionDetails;
}

impl <'a> SelectionInformation for Query<'a, Selection<AddTile>> {

}

impl <'a> SelectionInformation for Query<'a, Selection<DeleteNode>> {

}

impl <'a> SelectionInformation for Query<'a, Selection<MoveLine>> {

}

/// this is the method you'd generally use when inserting tiles (unsure what strategy to employ for removals yet)
/// only works if starting from a tile, which is intended
/// todo: transform selection will be given it's own query type with the edit command as part of the arg, so it can resolve the correct shape fully.
fn into_transform_selection(mut self) -> SelectionDetails {
    self.into_validated()
        .expect("Invalid selection starting line");
    let inward = self
        .with_side(self.side)
        .into_scan_area()
        .into_segmented(!self.side);

    let maybe_outward_segments = {
        let (cross_neg, cross_pos) = self.side.perpendicular_directions();
        let initial_neg = self
            .into_end(Negative)
            .with_side(cross_neg)
            .into_next(!self.side.sign);
        let initial_pos = self
            .into_end(Negative)
            .with_side(cross_pos)
            .into_next(!self.side.sign);
        match (initial_neg, initial_pos) {
            (Some(neg), Some(pos)) => {
                let reverse_line = self.context.query(Line::new(neg.id, pos.id, !self.side));
                if reverse_line.is_connected() {
                    Some(reverse_line.into_scan_area().into_segmented(self.side))
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
            if self.side.sign == Positive {
                SelectionDetails::Negative(inward)
            } else {
                SelectionDetails::Positive(inward)
            }
        }
        Some(outward_fragment) => {
            if self.side.sign == Positive {
                SelectionDetails::Bidirectional(inward, outward_fragment)
            } else {
                SelectionDetails::Bidirectional(outward_fragment, inward)
            }
        }
        _ => panic!(),
    }
}

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
