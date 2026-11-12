use crate::geometry::Axis::{X, Y};
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{Axis, Cardinal, Sign};
use crate::graph::{Graph};
use crate::gutter::{Gutter, GutterIterExt, GutterNavigation};
use crate::navigation::{Caret, Navigator};
use std::cmp::Ordering;
use std::iter::Scan;
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

impl TransformFragment {
    pub fn gutters(&self, direction: Cardinal) -> impl Iterator<Item = Gutter> {
        std::iter::once(Gutter::new(self.start.0, self.end.0, !direction))
            .chain(
                self.dividers
                    .iter()
                    .chain(std::iter::once(&self.end))
                    .map(move |(start, end)| Gutter::new(*start, *end, direction)),
            )
    }

    pub fn segments(&self, direction: Cardinal) -> impl Iterator<Item = (Gutter, Gutter)> {
        self.gutters(direction).zip(self.gutters(direction).skip(1))
    }
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

        let inward_fragment = self.build_transform_fragment((gutter.start_id, gutter.end_id), None, !gutter.side);

        let maybe_outward_fragment = {
            let initial_neg = gutter.start().navigate_gutter(self).peek(gutter.side.sign);
            let initial_pos = gutter.end().navigate_gutter(self).peek(gutter.side.sign);
            match (initial_neg, initial_pos) {
                (Some(neg), Some(pos)) => {

                    if Gutter::new(neg.tile_id, pos.tile_id, !gutter.side).is_connected_on(self) {
                        Some(self.build_transform_fragment((neg.tile_id, pos.tile_id), None, gutter.side))
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
    /// note: we're actually building in the direction opposite to that of our valid and well-connected gutter, since that ensures the gutter is strictly included.
    fn build_transform_fragment(
        &self,
        start: (Uuid, Uuid),
        end: Option<(Uuid, Uuid)>,
        build_direction: Cardinal,
    ) -> TransformFragment {
        assert!(Gutter::new(start.0, start.1, !build_direction).is_connected_on(self));
        let (cross_neg, cross_pos) = build_direction.perpendicular_directions();
        let mut dividers = Vec::new();
        let mut current_neg = Caret::new(start.0, cross_neg).navigate_gutter(self);
        let mut current_pos = Caret::new(start.0, cross_pos).navigate_gutter(self);
        let mut neg_iter = current_neg.iter(build_direction.sign);
        let mut pos_iter = current_pos.iter(build_direction.sign);

        while let (Some(next_neg), Some(next_pos)) = (neg_iter.next(), pos_iter.next()) {
            current_neg = next_neg;
            current_pos = next_pos;
            if end.is_some_and(|(neg_end, pos_end)| {
                current_neg.tile_id == neg_end || next_pos.tile_id == pos_end
            }) {
                break;
            }
            if Gutter::new(current_neg.tile_id, current_pos.tile_id, build_direction)
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
                        current_neg = next_neg;
                        neg_location = next_neg.location();
                    } else {
                        break;
                    }
                } else if neg_location > pos_location {
                    if let Some(next_pos) = pos_iter.next() {
                        current_pos = next_pos;
                        pos_location = next_pos.location()
                    } else {
                        break;
                    }
                }
            }
        }

        TransformFragment {
            start,
            dividers,
            end: (current_neg.tile_id, current_pos.tile_id),
        }
    }


    /// low-level resizing is always done with one side fixed in place (only the non-anchored side actually moves)
    fn resize_tile(&mut self, tile_id: Uuid, fragment_direction: Cardinal, size_delta: i32, anchor_end: bool) {
        let delta_signedness = size_delta.cmp(&0);
        if delta_signedness == Ordering::Equal {
            return;
        }
        // todo: the anchoring is not always so transparent as this, sometimes there is a shift, will need to rework this to handle the conditional translation that varies based on position
        let rect = &mut self.tiles.get_mut(&tile_id).unwrap().rect;
        let location_delta = match (fragment_direction.sign, delta_signedness, anchor_end) {
            (Negative, _, true) => 0,

            (Positive, Ordering::Less, true) => size_delta,
            (Positive, Ordering::Greater, true) => -size_delta,
            (Positive, Ordering::Less, false) => {}
            (Positive, Ordering::Greater, false) => {}
            (Negative, Ordering::Less, false) => {},
            (Negative, Ordering::Greater, false) => {},
            (_, Ordering::Equal, _) => panic!(),
        };
        match fragment_direction.axis {
            X => {
                rect.width += size_delta;
                rect.x += location_delta;
            }
            Y => {
                rect.height += size_delta;
                rect.y += location_delta;
            }
        }
    }

    /// low-level resizing is always done with one side fixed in place (only the non-anchored side actually moves)
    fn resize_transform_fragment(
        &mut self,
        fragment: TransformFragment,
        fragment_direction: Cardinal,
        size_delta: i32,
        anchor_end: bool, // this is used recursively and should be defaulted to true for external calls
    ) {
        if fragment.start.0 == fragment.end.1 {
            assert_eq!(fragment.start.0, fragment.start.1);
            assert_eq!(fragment.end.0, fragment.end.1);
            self.resize_tile(fragment.start.0, fragment_direction, size_delta, anchor_end);
            return;
        }

        let segment_count: i32 = fragment.dividers.len() as i32 + 1;
        let deltas = if segment_count == 1 {
            vec![(size_delta, size_delta.abs())]
        } else {
            let positions: Vec<i32> = fragment
                .gutters(fragment_direction)
                .map(|gutter| gutter.location_on(self))
                .collect();
            let start_position = positions[0];
            let end_position = positions.last().unwrap();
            let initial_size = i32::abs(end_position - start_position);
            let result_size = initial_size + size_delta;
            assert!(result_size >= 0);
            let scale = result_size as f64 / initial_size as f64;
            let mut sizes: Vec<(i32, i32)> = positions
                .iter()
                .zip(positions.iter().skip(1))
                .map(|(a, b)| {
                    let initial = i32::abs(b - a);
                    let result = f64::floor(initial as f64 * scale) as i32;
                    (initial, result)
                })
                .collect();

            let remainder = result_size - sizes.iter().map(|(_, result)| *result).sum::<i32>();
            assert!(remainder >= 0);
            assert!(remainder < segment_count);
            sizes
                .iter_mut()
                .rev()
                .take(remainder as usize)
                .for_each(|(_, result)| {
                    *result += 1;
                });
            let iter = sizes
                .into_iter();
            let result = sizes.into_iter().map(|(old, new)| (new - old, 0)).collect();
            match fragment_direction.sign {

            }
            result
        };

        for (i, ((start, end), segment_size_delta)) in fragment.segments(fragment_direction).zip(deltas).enumerate()
        {
            if segment_size_delta == 0 {
                continue;
            }
            let anchor_segment = anchor_end && i == (segment_count as usize - 1);
            // subdivide on the cross axis to find relative subdivisions on this axis and recurse into them
            let cross_pos = Cardinal::new(!fragment_direction.axis, Positive);
            for (cross_start, cross_end) in self
                .build_transform_fragment(
                    (start.start_id, end.start_id),
                    Some((start.end_id, end.end_id)),
                    cross_pos
                )
                .segments(cross_pos)
            {
                let subfragment = match fragment_direction.sign {
                    Positive => self.build_transform_fragment((cross_start.start_id, cross_end.start_id), Some((cross_start.end_id, cross_end.end_id)), fragment_direction),
                    Negative => self.build_transform_fragment((cross_start.end_id, cross_end.end_id), Some((cross_start.start_id, cross_end.start_id)), fragment_direction),
                };
                // recursively adjust subfragments
                self.resize_transform_fragment(subfragment, fragment_direction, segment_size_delta, anchor_segment);
            }
        }
    }
}


fn pack_adjustments<I: &mut[(i32,i32)::]>>(all_sizes: I) -> Scan<I, i32, fn(&mut i32, (i32, i32)) -> Option<(i32, i32)>> {
    all_sizes.scan(0i32, |shift, (old, new)| {
        let size_delta = old - new;
        *shift += size_delta.abs();
        Some((size_delta, *shift))
    })
}