use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::Sign::{Negative, Positive};
use crate::navigation::Navigator;
use std::cmp::Ordering;
use std::ops::{Index, IndexMut, Not};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Sign {
    Negative,
    Positive,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct AxisBounds {
    pub origin: i32,
    pub size: i32,
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
    fn location(&self, index: Sign) -> i32 {
        match index {
            Negative => self.origin,
            Positive => self.origin + self.size,
        }
    }

    pub fn center(&self) -> f32 {
        self.origin as f32 + (self.size as f32 / 2f32)
    }

    pub fn cmp(&self, other: &AxisBounds) -> BoundsOrdering {
        assert!(self.size >= 0);
        assert!(other.size >= 0);
        let max = self.origin + self.size;
        let other_max = other.origin + other.size;
        if self.origin < other.origin {
            if max < other.origin {
                BoundsOrdering::Before
            } else if max < other_max {
                BoundsOrdering::PartiallyBefore
            } else {
                BoundsOrdering::Contains
            }
        } else if self.origin > other.origin {
            if self.origin > other_max {
                BoundsOrdering::After
            } else if max > other_max {
                BoundsOrdering::PartiallyAfter
            } else {
                BoundsOrdering::ContainedBy
            }
        } else {
            if max < other_max {
                BoundsOrdering::ContainedBy
            } else if max > other_max {
                BoundsOrdering::Contains
            } else {
                BoundsOrdering::Parallel
            }
        }
    }

    /// splits the target bounds into two portions:
    /// - the first is the portion of the bounds that lie inside these bounds, and the second is a delta that when added first results in the original.
    pub fn clamp(self, container: AxisBounds) -> AxisBounds {
        assert_ne!(self.cmp(&container), BoundsOrdering::Contains);
        let container_max = container.origin + container.size;
        let clamped_min = self.origin.clamp(container.origin, container_max);
        let clamped_max = (self.origin + self.size).clamp(container.origin, container_max);
        AxisBounds {
            origin: clamped_min,
            size: clamped_max - clamped_min,
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Bounds {
    pub horizontal: AxisBounds,
    pub vertical: AxisBounds,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

impl Bounds {
    /// returns the location of the given side of the rect along the axis perpendicular to it's length
    /// (this being the only axis for which there is one value and not two for that side)
    pub fn location(&self, side: Cardinal) -> i32 {
        self[side.axis].location(side.sign)
    }
}

impl Index<Axis> for Bounds {
    type Output = AxisBounds;

    fn index(&self, index: Axis) -> &Self::Output {
        match index {
            Horizontal => &self.horizontal,
            Vertical => &self.vertical,
        }
    }
}

impl IndexMut<Axis> for Bounds {
    fn index_mut(&mut self, index: Axis) -> &mut Self::Output {
        match index {
            Horizontal => &mut self.horizontal,
            Vertical => &mut self.vertical,
        }
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


