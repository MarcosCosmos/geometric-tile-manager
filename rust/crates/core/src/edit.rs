use crate::area::{Area, AreaQuery};
use crate::caret::Caret;
use crate::command::transform::SegmentedArea;
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{Axis, AxisBounds, BoundsOrdering, Cardinal, Sign};
use crate::graph::{Graph, NodeId};
use crate::line::Line;
use crate::query::{
    ElementInformation, ElementInformation1D, ElementInformation2D, ElementKind, GroupInformation,
};
use crate::source::RawQuerySou