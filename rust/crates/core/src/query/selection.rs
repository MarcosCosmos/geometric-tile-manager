use crate::command::Edit;
use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{Axis, Sign};
use crate::query::QueryKey;
use crate::query::area::Area;
use crate::query::caret::Caret;
use crate::query::line::Line;

#[derive(Copy, Clone, Debug, Eq, PartialEq, QueryKey)]
#[query(CaretKey::NodeKey)]
pub struct Selection<CaretKey, LineKey>(Edit<CaretKey, LineKey>)
where
    CaretKey: QueryKey + Into<Caret<CaretKey::NodeKey>>,
    LineKey: QueryKey<NodeKey = CaretKey::NodeKey> + Into<Line<LineKey::NodeKey>>;

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
}

#[derive(Clone, Debug)]
pub enum SelectionArea {
    Negative(SegmentedContainer),
    Positive(SegmentedContainer),
    Bidirectional(SegmentedContainer, SegmentedContainer),
}

impl SelectionArea {
    pub fn part(&self, sign: Sign) -> Option<&SegmentedContainer> {
        match (sign, self) {
            (Negative, SelectionArea::Negative(fragment))
            | (Positive, SelectionArea::Positive(fragment))
            | (Negative, SelectionArea::Bidirectional(fragment, _))
            | (Positive, SelectionArea::Bidirectional(_, fragment)) => Some(fragment),
            _ => None,
        }
    }

    pub fn parts(self) -> (Option<SegmentedContainer>, Option<SegmentedContainer>) {
        match self {
            SelectionArea::Negative(neg) => (Some(neg), None),
            SelectionArea::Positive(pos) => (None, Some(pos)),
            SelectionArea::Bidirectional(neg, pos) => (Some(neg), Some(pos)),
        }
    }

    pub fn parts_ref(&self) -> (Option<&SegmentedContainer>, Option<&SegmentedContainer>) {
        match self {
            SelectionArea::Negative(neg) => (Some(neg), None),
            SelectionArea::Positive(pos) => (None, Some(pos)),
            SelectionArea::Bidirectional(neg, pos) => (Some(neg), Some(pos)),
        }
    }

    pub fn area(&self) -> Area {
        match self {
            SelectionArea::Negative(neg) => neg.area(),
            SelectionArea::Positive(pos) => pos.area(),
            SelectionArea::Bidirectional(neg, pos) => {
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
