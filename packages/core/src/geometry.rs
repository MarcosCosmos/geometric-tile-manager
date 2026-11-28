use crate::geometry::Axis::{Horizontal, Vertical};
use crate::geometry::Sign::{Negative, Positive};
use geotm_macros::Encode;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::Debug;
use std::fmt::{Display, Formatter, write};
use std::ops::{Index, IndexMut, Not};

#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl Display for Axis {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Horizontal => write!(f, "X"),
            Vertical => write!(f, "Y"),
        }
    }
}

impl Debug for Axis {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Sign {
    Negative,
    Positive,
}

impl Display for Sign {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Negative => write!(f, "-"),
            Positive => write!(f, "+"),
        }
    }
}

impl Debug for Sign {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

#[derive(Copy, Clone, Default, Eq, PartialEq, Hash, Serialize, Deserialize, Encode)]
#[encode("{}..{}")]
pub struct AxisBounds {
    pub negative: i32,
    pub positive: i32,
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub enum BoundsAlignment {
    Before,
    PartiallyBefore,
    Contains,
    Parallel,
    ContainedBy,
    PartiallyAfter,
    After,
}

impl From<BoundsAlignment> for Ordering {
    fn from(val: BoundsAlignment) -> Self {
        match val {
            BoundsAlignment::Before
            | BoundsAlignment::PartiallyBefore
            | BoundsAlignment::Contains => Ordering::Less,
            BoundsAlignment::Parallel => Ordering::Equal,
            BoundsAlignment::ContainedBy
            | BoundsAlignment::PartiallyAfter
            | BoundsAlignment::After => Ordering::Greater,
        }
    }
}

impl AxisBounds {
    pub fn new(negative: i32, positive: i32) -> Self {
        Self { negative, positive }
    }
    pub fn extent(&self) -> i32 {
        assert!(self.positive >= self.negative);
        self.positive - self.negative
    }

    pub fn center(&self) -> i32 {
        self.negative + (self.extent() / 2)
    }

    pub fn alignment(&self, other: &AxisBounds) -> BoundsAlignment {
        if self.negative < other.negative {
            if self.positive < other.negative {
                BoundsAlignment::Before
            } else if self.negative < other.positive {
                BoundsAlignment::PartiallyBefore
            } else {
                BoundsAlignment::Contains
            }
        } else if self.negative > other.negative {
            if self.negative > other.positive {
                BoundsAlignment::After
            } else if self.positive > other.positive {
                BoundsAlignment::PartiallyAfter
            } else {
                BoundsAlignment::ContainedBy
            }
        } else {
            if self.positive < other.positive {
                BoundsAlignment::ContainedBy
            } else if self.positive > other.positive {
                BoundsAlignment::Contains
            } else {
                BoundsAlignment::Parallel
            }
        }
    }

    /// splits the target bounds into two portions:
    /// - the first is the portion of the bounds that lie inside these bounds, and the second is a delta that when added first results in the original.
    pub fn clamp(self, other: &AxisBounds) -> AxisBounds {
        assert_ne!(self.alignment(other), BoundsAlignment::Contains);
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

    pub fn distance_to(&self, other: i32) -> i32 {
        i32::max(
            0,
            i32::min(
                i32::abs(self.negative - other),
                i32::abs(self.positive - other),
            ),
        )
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

impl PartialOrd<Self> for AxisBounds {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AxisBounds {
    fn cmp(&self, other: &Self) -> Ordering {
        self.alignment(other).into()
    }
}

#[derive(Copy, Clone, Default, Eq, PartialEq, Hash, Serialize, Deserialize, Encode)]
#[encode("(X: {horizontal}, Y: {vertical})")]
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

#[derive(Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Encode)]
#[encode("({side_location}/{axis_bounds}).{side}")]
pub struct LineBounds {
    pub side: Cardinal,
    pub side_location: i32,
    pub axis_bounds: AxisBounds,
}

impl From<LineBounds> for AreaBounds {
    fn from(value: LineBounds) -> Self {
        let narrow_bounds = AxisBounds::new(value.side_location, value.side_location);
        match value.side.axis {
            Horizontal => AreaBounds::new(narrow_bounds, value.axis_bounds),
            Vertical => AreaBounds::new(value.axis_bounds, narrow_bounds),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl Display for Cardinal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Cardinal {
                    axis: Horizontal,
                    sign: Negative,
                } => "W",
                Cardinal {
                    axis: Horizontal,
                    sign: Positive,
                } => "E",
                Cardinal {
                    axis: Vertical,
                    sign: Negative,
                } => "N",
                Cardinal {
                    axis: Vertical,
                    sign: Positive,
                } => "S",
            }
        )
    }
}

impl Debug for Cardinal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
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

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Usually associated with a corner of a box
pub struct Diagonal {
    pub horizontal: Sign,
    pub vertical: Sign,
}

impl Debug for Diagonal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
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

impl Display for Diagonal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Diagonal {
                    horizontal: Negative,
                    vertical: Negative,
                } => "NW",
                Diagonal {
                    horizontal: Negative,
                    vertical: Positive,
                } => "SW",
                Diagonal {
                    horizontal: Positive,
                    vertical: Negative,
                } => "NE",
                Diagonal {
                    horizontal: Positive,
                    vertical: Positive,
                } => "SE",
            }
        )
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
