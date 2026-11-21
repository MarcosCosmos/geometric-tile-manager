use crate::geometry::AreaBounds;
use markers_derive::Command;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Command)]
pub struct AddSurface {
    pub bounds: AreaBounds,
}

impl AddSurface {
    pub fn new(bounds: AreaBounds) -> Self {
        Self { bounds }
    }
}