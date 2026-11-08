use crate::geometry::{Axis::*, Cardinal, Rect, Sign, Sign::*};
use crate::result::{Error, Result};
use crate::transform::{
    TransformContainer, TransformContent, TransformFragment, TransformStrategy,
};
use std::cmp::{Ordering, PartialEq};
use std::collections::HashMap;
use std::collections::hash_map::IntoIter;
use std::iter::Peekable;
use std::ops::{Index, IndexMut};
use uuid::Uuid;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TileKind {
    Surface,
    Window,
}

#[derive(Copy, Clone, Debug)]
pub struct Tile {
    pub kind: TileKind,
    pub id: Uuid,
    pub rect: Rect,
}

#[derive(Clone, Debug, Default)]
struct Neighbourhood {
    north: Vec<Uuid>,
    south: Vec<Uuid>,
    east: Vec<Uuid>,
    west: Vec<Uuid>,
}

/// Notes:
/// - There are inter-surface and intra-surface neighbourhoods which have different meanings.
/// - Gaps may be present either on the inside of a surface or between windows; in htis sense gaps would potentially alter poorly written algorithms
/// - Storing tiles seperately from neighboy
#[derive(Clone, Debug, Default)]
pub struct Graph {
    tiles: HashMap<Uuid, Tile>,
    /// Note: each surface has an entry in its own interior neighbourhood.
    /// The meaning of direction is semi-inverted in that the surface's edges face inwards not outwards, but directionality is otherwise the same.
    /// E.g. the eastern neighbourhood of a surface faces inwardly east from the western side, whereas a normal window would face outwardly east from the eastern side.
    intra_surface_neighbourhoods: HashMap<Uuid, Neighbourhood>,
    /// inter_surface_neighbours are the neighbourhoods between surfaces, and therefore face outwards.
    inter_surface_neighbourhoods: HashMap<Uuid, Neighbourhood>,
}

impl Neighbourhood {
    pub fn new() -> Self {
        Self::default()
    }
    /// inserts the tile's id in order
    pub fn insert<'a, F>(&mut self, direction: Cardinal, tile_lookup: F, subject: &Tile)
    where
        F: Fn(&Uuid) -> &'a Tile,
    {
        let cmp_direction = Cardinal {
            axis: !direction.axis,
            sign: Positive,
        };
        let neighbours = &mut self[direction];
        let pos = neighbours.partition_point(|sib_id| {
            cmp_direction.cmp(&tile_lookup(sib_id).rect, &subject.rect) != Ordering::Greater
        });
        neighbours.insert(pos, subject.id);
    }

    pub fn replace(&mut self, direction: Cardinal, old: &Uuid, new: Uuid) {
        let dest = &mut self[direction];
        let pos = dest.iter().position(|x| x == old).unwrap();
        dest[pos] = new;
    }
}

impl Index<Cardinal> for Neighbourhood {
    type Output = Vec<Uuid>;
    fn index(&self, index: Cardinal) -> &Self::Output {
        match index {
            Cardinal {
                axis: Y,
                sign: Negative,
            } => &self.north,
            Cardinal {
                axis: Y,
                sign: Positive,
            } => &self.south,
            Cardinal {
                axis: X,
                sign: Negative,
            } => &self.west,
            Cardinal {
                axis: X,
                sign: Positive,
            } => &self.east,
        }
    }
}

impl IndexMut<Cardinal> for Neighbourhood {
    fn index_mut(&mut self, index: Cardinal) -> &mut Self::Output {
        match index {
            Cardinal {
                axis: Y,
                sign: Negative,
            } => &mut self.north,
            Cardinal {
                axis: Y,
                sign: Positive,
            } => &mut self.south,
            Cardinal {
                axis: X,
                sign: Negative,
            } => &mut self.west,
            Cardinal {
                axis: X,
                sign: Positive,
            } => &mut self.east,
        }
    }
}

impl Tile {
    pub fn new(id: Uuid, kind: TileKind, rect: Rect) -> Self {
        Self { id, kind, rect }
    }
}

/// A gutter is a position on the side of a window.
/// They can also be iterated, to scan through windows parallel to the current one along on the associated side.
#[derive(Copy, Clone, Debug)]
pub struct Gutter<'a> {
    pub current_id: Uuid,
    pub graph: &'a Graph,
    pub side: Cardinal,
    pub iter_direction: Sign,
}

impl<'a> Gutter<'a> {
    pub fn new(graph: &'a Graph, start_id: Uuid, side: Cardinal, iter_direction: Sign) -> Self {
        Self {
            current_id: start_id,
            graph,
            side,
            iter_direction,
        }
    }

    pub fn rev(self) -> Self {
        let mut result = self;
        result.iter_direction = !self.iter_direction;
        result
    }

    pub fn jump(&mut self, direction: Cardinal) -> Option<Uuid> {
        let neighbours = &self.graph.intra_surface_neighbourhoods[&self.current_id][direction];
        let result = if self.iter_direction == Positive {
            neighbours.first()
        } else {
            neighbours.last()
        };

        if let Some(&id) = result {
            self.current_id = id;
            self.side = !direction;
            Some(id)
        } else {
            None
        }
    }
}

impl Iterator for Gutter<'_> {
    type Item = Uuid;

    fn next(&mut self) -> Option<Self::Item> {
        let location = self.graph.tiles[&self.current_id]
            .rect
            .get_side_location(self.side);
        let searchDirection = Cardinal {
            axis: !self.side.axis,
            sign: self.iter_direction,
        };
        let result = self.graph.intra_surface_neighbourhoods[&self.current_id][searchDirection]
            .iter()
            .find(|id| {
                let tile = self.graph.tiles[id];
                tile.kind == TileKind::Window && tile.rect.get_side_location(self.side) == location
            });
        if let Some(&id) = result {
            self.current_id = id;
            Some(id)
        } else {
            None
        }
    }
}

impl Graph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_surface(&mut self, rect: Rect) -> Uuid {
        let id = Uuid::new_v4();
        self.tiles
            .insert(id, Tile::new(id, TileKind::Surface, rect));
        self.intra_surface_neighbourhoods
            .insert(id, Neighbourhood::new());
        id
    }

    pub fn insert_window(
        &mut self,
        window_id: Uuid,
        target_id: &Uuid,
        direction: Cardinal,
        strategy: TransformStrategy,
    ) -> Result<Rect> {
        // todo: this is redundant because of internal similar calls. Might actually be easier to go with command/message processing so we can do prechecks?a
        if !self.tiles.contains_key(target_id) {
            return Err(Error::TileNotFound(*target_id));
        }

        let window = Tile::new(
            window_id,
            TileKind::Window,
            self.make_space(target_id, direction),
        );

        // todo: all of this only works if strictly splitting the target. A more complicated strategy is needed for resizing all along the parallel

        let mut window_neighbourhood = Neighbourhood::new();

        // adjust connections
        // simple part: the direction parallel to the insertion
        {
            let opposite = !direction;
            let neighbours_of_window = &mut window_neighbourhood[direction];
            {
                let neighbours_of_target = &mut self
                    .intra_surface_neighbourhoods
                    .get_mut(target_id)
                    .unwrap()[direction];
                std::mem::swap(neighbours_of_target, neighbours_of_window);
                neighbours_of_target.push(window_id); // our new window is guaranteed to be the only neighbour, so we can simply push
            }

            // todo: maybe this is not sufficient because our new window could erase another neighbour (?)
            // might be worth extracting to a method anyway.
            for id in neighbours_of_window {
                self.intra_surface_neighbourhoods
                    .get_mut(id)
                    .unwrap()
                    .replace(opposite, target_id, window_id);
            }
        }

        // the perpendicular is a little more complicated
        for sign in [Negative, Positive] {
            let cross_direction = Cardinal {
                axis: !direction.axis,
                sign,
            };
            let cross_opposite = !cross_direction;
            // we have to move the neighbour vector out of the tile map in order to hold other mutable references
            let mut neighbours_of_target = std::mem::replace(
                &mut self
                    .intra_surface_neighbourhoods
                    .get_mut(target_id)
                    .unwrap()[cross_direction],
                Vec::new(),
            );
            let former_neighbours_of_target = neighbours_of_target.extract_if(.., |id| {
                direction.cmp(&self.tiles[&id].rect, &window.rect) != Ordering::Less
            });
            let neighbours_of_window = &mut window_neighbourhood[cross_direction];
            neighbours_of_window.extend(former_neighbours_of_target);
            // swap out the old neighbours; todo: this might not be correct enough (see above)
            for id in neighbours_of_window {
                self.intra_surface_neighbourhoods
                    .get_mut(id)
                    .unwrap()
                    .replace(direction, target_id, window_id);
            }

            let target = &self.tiles[target_id];
            //now
            let shared_neighbours = neighbours_of_target.iter().filter(|id| {
                direction.cmp(&self.tiles[id].rect, &target.rect) == Ordering::Greater
            });

            for shared_id in shared_neighbours {
                self.intra_surface_neighbourhoods
                    .get_mut(&shared_id)
                    .unwrap()
                    .insert(cross_opposite, |id| &self.tiles[id], &window);
                window_neighbourhood.insert(
                    cross_direction,
                    |id| &self.tiles[id],
                    &self.tiles[&shared_id],
                );
            }

            // now swap it back in
            std::mem::swap(
                &mut neighbours_of_target,
                &mut self
                    .intra_surface_neighbourhoods
                    .get_mut(target_id)
                    .unwrap()[cross_direction],
            );
        }

        self.tiles.insert(window_id, window);
        self.intra_surface_neighbourhoods
            .insert(window_id, window_neighbourhood);

        Ok(window.rect)
    }

    /// Creates a space for a new window on the specified side of the target tile.
    /// The method both calculates where the space will be and displaces other windows as necessary to permit it.
    /// Note: if the target is a surface, space is made inside, otherwise it is made beside.
    fn make_space(
        &mut self,
        target_id: &Uuid,
        direction: Cardinal,
        strategy: TransformStrategy,
    ) -> Result<Rect> {
        // todo: this is redundant because of internal similar calls. Might actually be easier to go with command/message processing so we can do prechecks?a
        if !self.tiles.contains_key(target_id) {
            return Err(Error::TileNotFound(*target_id));
        }

        // todo: should this actually be an oriented container?
        if let Some(container) = self.build_transform_container(target_id, direction, strategy)
        {
            // todo
        } else {
            Ok(self.tiles[target_id].rect)
        }
    }

    /// note: a and b must be supplied in order along the cross axis of the given side or this fails
    fn are_cleanly_connected(&self, a: &Uuid, b: &Uuid, connected_side: Cardinal) -> bool {
        let scan_direction = Cardinal {
            axis: !connected_side.axis,
            sign: Positive,
        };
        let end_location = self.tiles[&b].rect.get_side_location(scan_direction);
        Gutter::new(self, *a, connected_side, Positive)
            .take_while(|id| self.tiles[id].rect.get_side_location(scan_direction) <= end_location)
            .any(|id| id == *b)
    }

    /// this is the method you'd generally use when inserting windows (unsure what strategy to employ for removals yet)
    fn build_transform_container(
        &self,
        reference_tile_id: &Uuid,
        reference_side: Cardinal,
        strategy: TransformStrategy,
    ) -> Option<TransformContainer> {
        debug_assert!(self.tiles.contains_key(reference_tile_id));
        let (cross_neg, cross_pos) = reference_side.perpendicular_directions();
        let mut neg_reference_id = *reference_tile_id;
        let mut pos_reference_id = *reference_tile_id;
        if strategy != TransformStrategy::Minimal {
            let cross_gutter = Gutter::new(self, *reference_tile_id, reference_side, Positive);
            match strategy {
                TransformStrategy::Maximal => {
                    if let Some(id) = cross_gutter.rev().last() {
                        neg_reference_id = id;
                    }
                    if let Some(id) = cross_gutter.last() {
                        pos_reference_id = id;
                    }
                }
                TransformStrategy::OutwardMinimal => {
                    if let Some(id) = std::iter::once(*reference_tile_id)
                        .chain(cross_gutter.rev())
                        .find(|id| {
                            Gutter::new(self, *id, cross_neg, reference_side.sign)
                                .next()
                                .is_some()
                        })
                    {
                        neg_reference_id = id;
                    }
                    if let Some(id) = std::iter::once(*reference_tile_id)
                        .chain(cross_gutter)
                        .find(|id| {
                            Gutter::new(self, *id, cross_pos, reference_side.sign)
                                .next()
                                .is_some()
                        })
                    {
                        pos_reference_id = id;
                    }
                }
                _ => panic!(),
            }
        }

        let maybe_backward_fragment = if self.tiles[reference_tile_id].kind == TileKind::Window {
            Some(self.build_transform_fragment(neg_reference_id, pos_reference_id, !reference_side))
        } else {
            None
        };

        let maybe_forward_fragment = {
            let initial_neg =
                Gutter::new(self, neg_reference_id, cross_neg, reference_side.sign).next();
            let initial_pos =
                Gutter::new(self, pos_reference_id, cross_pos, reference_side.sign).next();
            match (initial_neg, initial_pos) {
                (Some(initial_neg_id), Some(initial_pos_id)) => Some(
                    self.build_transform_fragment(initial_neg_id, initial_pos_id, reference_side),
                ),
                _ => None,
            }
        };

        // the rest of this is just normalising the above backward/forward parts to collapse intermediate Nones and arranged them in natural order
        match (maybe_backward_fragment, maybe_forward_fragment) {
            (None, None) => None,
            some => {
                let content = match some {
                    (None, Some(forward)) => {
                        if reference_side.sign == Positive {
                            TransformContent::Positive(forward)
                        } else {
                            TransformContent::Negative(forward)
                        }
                    }
                    (Some(inward), None) => {
                        if reference_side.sign == Positive {
                            TransformContent::Negative(inward)
                        } else {
                            TransformContent::Positive(inward)
                        }
                    }
                    (Some(inward), Some(outward)) => {
                        if reference_side.sign == Positive {
                            TransformContent::Bidirectional(inward, outward)
                        } else {
                            TransformContent::Bidirectional(outward, inward)
                        }
                    }
                    _ => panic!(),
                };
                Some(TransformContainer {
                    axis: reference_side.axis,
                    content,
                })
            }
        }
    }

    /// note: there is a strict contractual expectation that the starting window(s) are
    ///     in a clean parallel line on the side opposite to the direction we are building in.
    fn build_transform_fragment(
        &self,
        neg_start_id: Uuid,
        pos_start_id: Uuid,
        build_direction: Cardinal,
    ) -> TransformFragment {
        debug_assert!(self.tiles[&neg_start_id].kind == TileKind::Window);
        debug_assert!(self.tiles[&pos_start_id].kind == TileKind::Window);
        assert!(self.are_cleanly_connected(&neg_start_id, &pos_start_id, build_direction));
        let (cross_neg, cross_pos) = build_direction.perpendicular_directions();
        let mut dividers = Vec::new();
        let mut neg_gutter = Gutter::new(self, neg_start_id, cross_neg, build_direction.sign)
            .map(|id| (id, self.tiles[&id].rect.get_side_location(build_direction)));
        let mut pos_gutter = Gutter::new(self, pos_start_id, cross_pos, build_direction.sign)
            .map(|id| (id, self.tiles[&id].rect.get_side_location(build_direction)));
        let mut neg_id: Uuid = neg_start_id;
        let mut pos_id: Uuid = pos_start_id;

        while let (Some(next_neg), Some(next_pos)) = (neg_gutter.next(), pos_gutter.next()) {
            if self.are_cleanly_connected(&neg_id, &pos_id, build_direction) {
                dividers.push((neg_id, pos_id));
            }
            let mut neg_location;
            let mut pos_location;

            (neg_id, neg_location) = next_neg;
            (pos_id, pos_location) = next_pos;

            while neg_location != pos_location {
                if neg_location < pos_location {
                    if let Some(next_neg) = neg_gutter.next() {
                        (neg_id, neg_location) = next_neg;
                    } else {
                        break;
                    }
                } else if neg_location > pos_location {
                    if let Some(next_pos) = pos_gutter.next() {
                        (pos_id, pos_location) = next_pos;
                    } else {
                        break;
                    }
                }
            }
        }

        TransformFragment {
            start: (neg_start_id, pos_start_id),
            dividers,
            end: (neg_id, pos_id),
        }
    }
}
