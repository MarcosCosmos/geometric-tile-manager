use std::cmp::Ordering;
use std::ops::{Index, Not};
use crate::geometry::Axis::{X, Y};
use crate::geometry::Orientation::{Start, End};

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
pub enum Orientation {
    Start,
    End,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// Usually associated with a side of a box
pub struct Cardinal {
    pub axis: Axis,
    pub orientation: Orientation,
}

#[derive(Copy, Clone, PartialEq, Eq)]
/// Usually associated with a corner of a box
pub struct Diagonal {
    pub x: Orientation,
    pub y: Orientation,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// a directed version of a rect meant to help compare positions with respect to a direction
/// this helps simplify directed operations like inserts
#[derive(Copy, Clone, Debug)]
pub struct DirectedRect(pub Rect, pub Cardinal);

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y}
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

impl Index<Axis> for Diagonal {
    type Output = Orientation;
    fn index(&self, index: Axis) -> &Self::Output {
        match index {
            X => &self.x,
            Y => &self.y,
        }
    }
}

impl Cardinal {
    /// as in the corners associated with this direction's the side of a box
    pub fn adjacent_diagonals(&self) -> (Diagonal, Diagonal) {
        if self.axis == X {
            (
                Diagonal { x: self.orientation, y: Start },
                Diagonal { x: self.orientation, y: End },
            )
        } else {
            (
                Diagonal { x: Start, y: self.orientation },
                Diagonal { x: End, y: self.orientation },
            )
        }
    }
    pub fn cross_directions(&self) -> (Cardinal, Cardinal) {
        let axis = !self.axis;
        (
            Cardinal { axis, orientation: Start },
            Cardinal { axis, orientation: End },
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
        if self.orientation == End {
            natural
        } else {
            natural.reverse()
        }
    }
}

impl Diagonal {
    /// as in the sides associated with the corner of a box.
    /// The result is always axis align, i.e. (x, y)
    pub fn adjacent_cardinals(&self) -> (Cardinal, Cardinal) {
        ( Cardinal { axis: X, orientation: self.x }, Cardinal { axis: Y, orientation: self.y })
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

impl Not for Orientation {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            Start => End,
            End => Start,
        }
    }
}

impl Not for Cardinal {
    type Output = Self;
    fn not(self) -> Self::Output {
        Self {
            axis: self.axis,
            orientation: !self.orientation,
        }
    }
}

impl Not for Diagonal {
    type Output = Self;
    fn not(self) -> Self::Output {
        Self {
            x: !self.x,
            y: !self.y,
        }
    }
}

pub const NORTH: Cardinal = Cardinal {
 axis: Y,
 orientation: Start,
};
pub const SOUTH: Cardinal = Cardinal {
    axis: Y,
    orientation: End,
};
pub const EAST: Cardinal = Cardinal {
    axis: X,
    orientation: End,
};
pub const WEST: Cardinal = Cardinal {
    axis: X,
    orientation: Start,
};
pub const NORTH_WEST: Diagonal = Diagonal {
    x: Start,
    y: Start,
};
pub const NORTH_EAST: Diagonal = Diagonal {
    x: End,
    y: Start,
};
pub const SOUTH_EAST: Diagonal = Diagonal {
    x: End,
    y: End,
};
pub const SOUTH_WEST: Diagonal = Diagonal {
    x: Start,
    y: End,
};