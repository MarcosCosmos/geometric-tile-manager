use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{Axis, Sign};
use uuid::Uuid;

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

impl TransformContent {
    pub fn get_sign(&self, sign: Sign) -> Option<&TransformFragment> {
        match (sign, self) {
            (Negative, TransformContent::Negative(fragment)) => Some(fragment),
            (Positive, TransformContent::Positive(fragment)) => Some(fragment),

            (Negative, TransformContent::Bidirectional(fragment, _)) => Some(fragment),
            (Positive, TransformContent::Bidirectional(_, fragment)) => Some(fragment),
            _ => None,
        }
    }

    pub fn parts(self) -> (Option<TransformFragment>, Option<TransformFragment>) {
        match self {
            TransformContent::Negative(neg) => (Some(neg), None),
            TransformContent::Positive(pos) => (None, Some(pos)),
            TransformContent::Bidirectional(neg, pos) => (Some(neg), Some(pos)),
        }
    }

    pub fn parts_ref(&self) -> (Option<&TransformFragment>, Option<&TransformFragment>) {
        match self {
            TransformContent::Negative(neg) => (Some(neg), None),
            TransformContent::Positive(pos) => (None, Some(pos)),
            TransformContent::Bidirectional(neg, pos) => (Some(neg), Some(pos)),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (Sign, &TransformFragment)> {
        self.into_iter()
    }
}

impl IntoIterator for TransformContent {
    type Item = (Sign, TransformFragment);
    type IntoIter = TransformContentIterator<TransformFragment>;

    fn into_iter(self) -> Self::IntoIter {
        TransformContentIterator(self.parts())
    }
}
impl<'a> IntoIterator for &'a TransformContent {
    type Item = (Sign, &'a TransformFragment);
    type IntoIter = TransformContentIterator<&'a TransformFragment>;

    fn into_iter(self) -> Self::IntoIter {
        TransformContentIterator(self.parts_ref())
    }
}

pub struct TransformContentIterator<T>((Option<T>, Option<T>));

impl<T> Iterator for TransformContentIterator<T> {
    type Item = (Sign, T);

    fn next(&mut self) -> Option<Self::Item> {
        self.0
            .0
            .take()
            .map(|fragment| (Negative, fragment))
            .or_else(|| self.0.1.take().map(|fragment| (Positive, fragment)))
    }
}
