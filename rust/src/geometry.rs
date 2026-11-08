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
//
// #[derive(Copy, Clone, PartialEq, Eq)]
// /// Usually associated with a corner of a box
// pub struct Diagonal {
//     pub x: Sign,
//     pub y: Sign,
// }

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

// impl Index<Axis> for Diagonal {
//     type Output = Sign;
//     fn index(&self, index: Axis) -> &Self::Output {
//         match index {
//             X => &self.x,
//             Y => &self.y,
//         }
//     }
// }

impl Cardinal {
    // /// as in the corners associated with this direction's the side of a box
    // pub fn adjacent_diagonals(&self) -> (Diagonal, Diagonal) {
    //     if self.axis == X {
    //         (
    //             Diagonal { x: self.sign, y: Negative },
    //             Diagonal { x: self.sign, y: Positive },
    //         )
    //     } else {
    //         (
    //             Diagonal { x: Negative, y: self.sign },
    //             Diagonal { x: Positive, y: self.sign },
    //         )
    //     }
    // }
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

// impl Diagonal {
//     /// as in the sides associated with the corner of a box.
//     /// The result is always axis align, i.e. (x, y)
//     pub fn adjacent_cardinals(&self) -> (Cardinal, Cardinal) {
//         (Cardinal { axis: X, sign: self.x }, Cardinal { axis: Y, sign: self.y })
//     }
// }

impl Rect {
    /// returns the position of the given side of the rect along the axis perpendicular to it's length
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

// impl Not for Diagonal {
//     type Output = Self;
//     fn not(self) -> Self::Output {
//         Self {
//             x: !self.x,
//             y: !self.y,
//         }
//     }
// }

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
// pub const NORTH_WEST: Diagonal = Diagonal {
//     x: Negative,
//     y: Negative,
// };
// pub const NORTH_EAST: Diagonal = Diagonal {
//     x: Positive,
//     y: Negative,
// };
// pub const SOUTH_EAST: Diagonal = Diagonal {
//     x: Positive,
//     y: Positive,
// };
// pub const SOUTH_WEST: Diagonal = Diagonal {
//     x: Negative,
//     y: Positive,
// };
