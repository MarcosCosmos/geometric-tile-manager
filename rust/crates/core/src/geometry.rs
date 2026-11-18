use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::Sign::{Negative, Positive};
use crate::query::NodeKey;
use crate::query::QueryKey;
use std::cmp::Ordering;
use std::ops::{Index, IndexMut, Not};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Sign {
    Negative,
    Positive,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct AxisBounds {
    negative: i32,
    positive: i32,
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum BoundsOrdering {
    Before,
    PartiallyBefore,
    Contains,
    Parallel,
    ContainedBy,
    PartiallyAfter,
    After,
}

impl Into<Ordering> for BoundsOrdering {
    fn into(self) -> Ordering {
        match self {
            BoundsOrdering::Before | BoundsOrdering::PartiallyBefore | BoundsOrdering::Contains => {
                Ordering::Less
            }
            BoundsOrdering::Parallel => Ordering::Equal,
            BoundsOrdering::ContainedBy
            | BoundsOrdering::PartiallyAfter
            | BoundsOrdering::After => Ordering::Greater,
        }
    }
}

impl AxisBounds {
    pub fn new(negative: i32, positive: i32) -> Self {
        Self { negative, positive }
    }
    pub fn size(&self) -> i32 {
        assert!(self.positive >= self.negative);
        self.positive - self.negative
    }

    pub fn center(&self) -> f32 {
        self.negative as f32 + (self.size() as f32 / 2f32)
    }

    pub fn cmp(&self, other: &AxisBounds) -> BoundsOrdering {
        if self.negative < other.negative {
            if self.positive < other.negative {
                BoundsOrdering::Before
            } else if self.negative < other.positive {
                BoundsOrdering::PartiallyBefore
            } else {
                BoundsOrdering::Contains
            }
        } else if self.negative > other.negative {
            if self.negative > other.positive {
                BoundsOrdering::After
            } else if self.positive > other.positive {
                BoundsOrdering::PartiallyAfter
            } else {
                BoundsOrdering::ContainedBy
            }
        } else {
            if self.positive < other.positive {
                BoundsOrdering::ContainedBy
            } else if self.positive > other.positive {
                BoundsOrdering::Contains
            } else {
                BoundsOrdering::Parallel
            }
        }
    }

    /// splits the target bounds into two portions:
    /// - the first is the portion of the bounds that lie inside these bounds, and the second is a delta that when added first results in the original.
    pub fn clamp(self, other: &AxisBounds) -> AxisBounds {
        assert_ne!(self.cmp(&other), BoundsOrdering::Contains);
        AxisBounds {
            negative: self.negative.clamp(other.negative, other.positive),
            positive: self.positive.clamp(other.negative, other.positive),
        }
    }
    /// creates a new axis bounds that strictly contains both the target bounds
    pub fn span(&self, other: &AxisBounds) -> AxisBounds {
        AxisBounds {
            negative: self.negative.min(other.negative),
            positive: self.positive.max(other.positive),
        }
    }
}

impl Index<Sign> for AxisBounds {
    type Output = i32;

    fn index(&self, index: Sign) -> &Self::Output {
        match index {
            Negative => &self.negative,
            Positive => &self.positive,
        }
    }
}

impl IndexMut<Sign> for AxisBounds {
    fn index_mut(&mut self, index: Sign) -> &mut Self::Output {
        match index {
            Negative => &mut self.negative,
            Positive => &mut self.positive,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct AreaBounds {
    horizontal: AxisBounds,
    vertical: AxisBounds,
}

impl AreaBounds {
    pub fn new(horizontal: AxisBounds, vertical: AxisBounds) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct LineBounds {
    pub side: Cardinal,
    pub axis_bounds: AxisBounds,
    pub side_location: i32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
/// Usually associated with a side of a box
pub struct Cardinal {
    pub axis: Axis,
    pub sign: Sign,
}

impl Cardinal {
    pub fn new(axis: Axis, sign: Sign) -> Self {
        Self { axis, sign }
    }

    pub fn perpendicular_directions(&self) -> (Cardinal, Cardinal) {
        let axis = !self.axis;
        (
            Cardinal {
                axis,
                sign: Negative,
            },
            Cardinal {
                axis,
                sign: Positive,
            },
        )
    }
}

impl Index<Axis> for AreaBounds {
    type Output = AxisBounds;

    fn index(&self, index: Axis) -> &Self::Output {
        match index {
            Horizontal => &self.horizontal,
            Vertical => &self.vertical,
        }
    }
}

impl IndexMut<Axis> for AreaBounds {
    fn index_mut(&mut self, index: Axis) -> &mut Self::Output {
        match index {
            Horizontal => &mut self.horizontal,
            Vertical => &mut self.vertical,
        }
    }
}

impl Index<Cardinal> for AreaBounds {
    type Output = i32;

    fn index(&self, index: Cardinal) -> &Self::Output {
        &self[index.axis][index.sign]
    }
}

impl IndexMut<Cardinal> for AreaBounds {
    fn index_mut(&mut self, index: Cardinal) -> &mut Self::Output {
        &mut self[index.axis][index.sign]
    }
}

impl Not for Axis {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            Horizontal => Vertical,
            Vertical => Horizontal,
        }
    }
}

impl Not for Sign {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            Negative => Positive,
            Positive => Negative,
        }
    }
}

impl Not for Cardinal {
    type Output = Self;
    fn not(self) -> Self::Output {
        Self {
            axis: self.axis,
            sign: !self.sign,
        }
    }
}

pub const NORTH: Cardinal = Cardinal {
    axis: Vertical,
    sign: Negative,
};
pub const SOUTH: Cardinal = Cardinal {
    axis: Vertical,
    sign: Positive,
};
pub const EAST: Cardinal = Cardinal {
    axis: Horizontal,
    sign: Positive,
};
pub const WEST: Cardinal = Cardinal {
    axis: Horizontal,
    sign: Negative,
};

#[derive(Copy, Clone, PartialEq, Eq)]
/// Usually associated with a corner of a box
pub struct Diagonal {
    horizontal: Sign,
    vertical: Sign,
}

impl Not for Diagonal {
    type Output = Self;
    fn not(self) -> Self::Output {
        Self {
            horizontal: !self.horizontal,
            vertical: !self.vertical,
        }
    }
}

impl Index<Axis> for Diagonal {
    type Output = Sign;

    fn index(&self, index: Axis) -> &Self::Output {
        match index {
            Horizontal => &self.horizontal,
            Vertical => &self.vertical,
        }
    }
}

impl IndexMut<Axis> for Diagonal {
    fn index_mut(&mut self, index: Axis) -> &mut Self::Output {
        match index {
            Horizontal => &mut self.horizontal,
            Vertical => &mut self.vertical,
        }
    }
}

pub const NORTH_WEST: Diagonal = Diagonal {
    horizontal: Negative,
    vertical: Negative,
};
pub const NORTH_EAST: Diagonal = Diagonal {
    horizontal: Positive,
    vertical: Negative,
};
pub const SOUTH_EAST: Diagonal = Diagonal {
    horizontal: Positive,
    vertical: Positive,
};
pub const SOUTH_WEST: Diagonal = Diagonal {
    horizontal: Negative,
    vertical: Positive,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, QueryKey, NodeKey)]
#[query(Self)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
