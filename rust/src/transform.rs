use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{Axis, Sign};
use crate::graph::{Graph, TileKind};
use crate::gutter::{Gutter, GutterIterExt, GutterNavigation};
use crate::navigation::Navigator;
use std::ops::{Deref, DerefMut};
use uuid::Uuid;

/// MinimalOutward means that the transform container must extend outward in the direction of the reference side if there are any windows in that direction.
/// That pattern is necessary for deletion and side dragging operations
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TransformContraints {
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

pub trait GraphTransformExt
where
    Self: Deref<Target = Graph> + DerefMut<Target = Graph>,
{
    // a helper for normalising partially specified gutters so they are more suitable for various purposes
    fn finalize_gutter(&self, gutter: &mut Gutter, constraints: TransformContraints) {
        match constraints {
            TransformContraints::Minimal => {}
            TransformContraints::Maximal => {
                if let Some(id) = gutter
                    .start()
                    .navigate_gutter(self)
                    .iter(Negative)
                    .ids()
                    .last()
                {
                    gutter.start_id = id;
                }
                if let Some(id) = gutter
                    .end()
                    .navigate_gutter(self)
                    .iter(Positive)
                    .ids()
                    .last()
                {
                    gutter.end_id = id;
                }
            }
            TransformContraints::OutwardMinimal => {
                let (cross_neg, cross_pos) = gutter.side.perpendicular_directions();
                if let Some(c) = gutter
                    .start()
                    .navigate_gutter(self)
                    .iter(Negative)
                    .inclusive()
                    .find(|c| c.clone().face(cross_neg).peek(gutter.side.sign).is_some())
                {
                    gutter.start_id = c.tile_id;
                }
                if let Some(c) = gutter
                    .end()
                    .navigate_gutter(self)
                    .iter(Positive)
                    .inclusive()
                    .find(|c| c.clone().face(cross_pos).peek(gutter.side.sign).is_some())
                {
                    gutter.end_id = c.tile_id;
                }
            }
        }
    }

    /// this is the method you'd generally use when inserting windows (unsure what strategy to employ for removals yet)
    /// only works if starting from a window, which is intended
    fn build_transform_container(&self, gutter: Gutter) -> Option<TransformContainer> {
        debug_assert!(gutter.is_connected_on(self));

        let inward_fragment = self.build_transform_fragment(gutter);

        let maybe_outward_fragment = {
            let initial_neg = gutter.start().navigate_gutter(self).peek(gutter.side.sign);
            let initial_pos = gutter.end().navigate_gutter(self).peek(gutter.side.sign);
            match (initial_neg, initial_pos) {
                (Some(neg), Some(pos)) => {
                    let out_gutter = Gutter::new(
                        neg.tile_id,
                        pos.tile_id,
                        !gutter.side,
                    );
                    if out_gutter.is_connected_on(self){
                        Some(self.build_transform_fragment(out_gutter))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        };

        // the rest of this is just normalising the above backward/forward parts to collapse intermediate Nones and arranged them in natural order
        let content = match maybe_outward_fragment {
            (None) => {
                if gutter.side.sign == Positive {
                    TransformContent::Negative(inward_fragment)
                } else {
                    TransformContent::Positive(inward_fragment)
                }
            }
            Some(outward_fragment) => {
                if gutter.side.sign == Positive {
                    TransformContent::Bidirectional(inward_fragment, outward_fragment)
                } else {
                    TransformContent::Bidirectional(outward_fragment, inward_fragment)
                }
            }
            _ => panic!(),
        };

        Some(TransformContainer {
            axis: !gutter.side.axis,
            content,
        })
    }

    /// note: there is a strict contractual expectation that the starting window(s) are
    ///     in a clean parallel line on the side opposite to the direction we are building in.
    fn build_transform_fragment(&self, gutter: Gutter) -> TransformFragment {
        debug_assert!(self.tiles[&gutter.start_id].kind == TileKind::Window);
        debug_assert!(self.tiles[&gutter.end_id].kind == TileKind::Window);
        // note: we're actually building in the direction opposit our valid gutter
        assert!(gutter.is_connected_on(self));
        let build_direction = !gutter.side;
        let (cross_neg, cross_pos) = build_direction.perpendicular_directions();
        let mut dividers = Vec::new();
        let mut neg_iter = gutter
            .start()
            .navigate_gutter(self)
            .face(cross_neg)
            .iter(build_direction.sign);
        let mut pos_iter = gutter
            .end()
            .navigate_gutter(self)
            .face(cross_pos)
            .iter(build_direction.sign);

        while let (Some(next_neg), Some(next_pos)) = (neg_iter.next(), pos_iter.next()) {
            if Gutter::new(next_neg.tile_id, next_pos.tile_id, build_direction)
                .is_connected_on(self)
            {
                dividers.push((next_neg.tile_id, next_pos.tile_id));
                continue;
            }

            let mut neg_location = next_neg.location();
            let mut pos_location = next_pos.location();

            while neg_location != pos_location {
                if neg_location < pos_location {
                    if let Some(next_neg) = neg_iter.next() {
                        neg_location = next_neg.location();
                    } else {
                        break;
                    }
                } else if neg_location > pos_location {
                    if let Some(next_pos) = pos_iter.next() {
                        pos_location = next_pos.location()
                    } else {
                        break;
                    }
                }
            }
        }

        TransformFragment {
            start: (gutter.start_id, gutter.end_id),
            dividers,
            end: (neg_iter.current.tile_id, pos_iter.current.tile_id),
        }
    }
}
