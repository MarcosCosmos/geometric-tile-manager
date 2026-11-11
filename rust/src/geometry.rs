use crate::geometry::Axis::{X, Y};
use crate::geometry::Sign::{Negative, Positive};
use std::cmp::Ordering;
use std::ops::{Index, Not};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Point {
    x: i32,
    y: i32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    start: Point,
    end: Point,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Sign {
    Negative,
    Positive,
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
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

impl Index<Axis> for Point {
    type Output = i32;
    fn index(&self, index: Axis) -> &Self::Output {
        match index {
            X => &self.x,
            Y => &self.y,
        }
    }
}

impl Cardinal {
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

    pub fn cmp(&self, a: &Rect, b: &Rect) -> Ordering {
        let natural = match self.axis {
            X if (a.x + a.width) < b.x => Ordering::Less,
            X if a.x > (b.x + b.width) => Ordering::Greater,
            Y if (a.y + a.height) < b.y => Ordering::Less,
            Y if a.y > (b.y + b.height) => Ordering::Greater,
            _ => Ordering::Equal,
        };
        if self.sign == Positive {
            natural
        } else {
            natural.reverse()
        }
    }
}

impl Rect {
    /// returns the location of the given side of the rect along the axis perpendicular to it's length
    /// (this being the only axis for which there is one value and not two for that side)
    pub fn get_side_location(&self, side: Cardinal) -> i32 {
        match side {
            Cardinal {
                axis: X,
                sign: Negative,
            } => self.x,
            Cardinal {
                axis: X,
                sign: Positive,
            } => self.x + self.width,
            Cardinal {
                axis: Y,
                sign: Negative,
            } => self.y,
            Cardinal {
                axis: Y,
                sign: Positive,
            } => self.y + self.height,
        }
    }

    pub fn get_center(&self, axis: Axis) -> f32 {
        match axis {
            X => self.x as f32 + (self.width as f32 / 2f32),
            Y => self.y as f32 + (self.height as f32 / 2f32),
        }
    }
}

impl Not for Axis {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            X => Y,
            Y => X,
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
    axis: Y,
    sign: Negative,
};
pub const SOUTH: Cardinal = Cardinal {
    axis: Y,
    sign: Positive,
};
pub const EAST: Cardinal = Cardinal {
    axis: X,
    sign: Positive,
};
pub const WEST: Cardinal = Cardinal {
    axis: X,
    sign: Negative,
};
