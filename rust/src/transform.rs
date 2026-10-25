use uuid::Uuid;
use crate::geometry::{Axis, Cardinal};

/// MinimalOutward means that the transform container must extend outward in the direction of the reference side if there are any windows in that direction.
/// That pattern is necessary for deletion and side dragging operations
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TransformStrategy {
    Maximal,
    Minimal,
    OutwardMinimal,
}

/// Generally dividers will be clean lines that don't intersect any windows,
/// but the start or end may or may not result in cutting through some tiles
/// Note: it is possible for a transform container to start and end with the same pair of ids and have no dividers.
///     in this case it is a single row/column or even single window.
#[derive(Clone, Debug)]
pub struct TransformFragment {
    pub start: (Uuid, Uuid),
    pub dividers: Vec<(Uuid, Uuid)>,
    pub end: (Uuid, Uuid),
}


#[derive(Clone, Debug)]
pub enum TransformContent {
    Negative(TransformFragment),
    Positive(TransformFragment),
    Bidirectional(TransformFragment, TransformFragment),
}

#[derive(Clone, Debug)]
pub struct TransformContainer {
    pub axis: Axis,
    pub content: TransformContent,
}