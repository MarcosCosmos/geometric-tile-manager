use crate::geometry::{Axis::*, Cardinal, Rect, Sign, Sign::*};
use crate::result::{Error, Result};
use crate::transform::{TransformContainer, TransformFragment, TransformStrategy, TransformContent};
use std::cmp::{Ordering, PartialEq};
use std::collections::HashMap;
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

/// Notes:
/// - There are inter-surface and intra-surface neighbourhoods which have different meanings.
/// - Gaps may be present either on the inside of a surface or between windows; in htis sense gaps would potentionally alter poorly written algorithms
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

impl Tile {
    pub fn new(id: Uuid, kind: TileKind, rect: Rect) -> Self {
        Self { id, kind, rect }
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
        if let Some(transform_container) =
            self.build_transform_fragment(target_id, direction, strategy)?
        {
            let segment_count = transform_container.lines.len();
            if transform_container.terminal_line.is_some() {
                segment_count += 1;
            }

            // todo:n
        } else {
            Ok(self.tiles[target_id].rect)
        }
    }

    /// resolves the tile's position along the axis parallel to that side
    fn resolve_side_location(&self, tile_id: &Uuid, side: Cardinal) -> i32 {
        let rect = &self.tiles[tile_id].rect;
        match side {
            Cardinal {
                axis: X,
                sign: Negative,
            } => rect.x,
            Cardinal {
                axis: X,
                sign: Positive,
            } => rect.x + rect.width,
            Cardinal {
                axis: Y,
                sign: Negative,
            } => rect.y,
            Cardinal {
                axis: Y,
                sign: Positive,
            } => rect.y + rect.height,
        }
    }

    /// looks for the last in the line of windows on the `neighbour_direction` side of the reference,
    /// which are parallel along the perpendicular side with sign `sign=parallel_sign` (not both sides).
    /// E.g. if `neighbour_direction=NORTH` and `parallel_sign=Negative` it will scan tiles that are:
    /// - to the north of the reference and,
    /// - parallel to the reference on the eastern side.
    /// Note: the reference tile does not need to be a window, it can be a surface.
    fn furthest_parallel_window(
        &self,
        reference_window_id: &Uuid,
        neighbour_direction: Cardinal,
        parallel_side_sign: Sign,
    ) -> Uuid {
        debug_assert!(self.tiles[&reference_window_id].kind == TileKind::Window);
        let mut result = reference_window_id;
        let alignment_side = Cardinal {
            axis: !neighbour_direction.axis,
            sign: parallel_side_sign,
        };
        let pos = self.resolve_side_location(result, alignment_side);

        while let Some(next_id) = &self.intra_surface_neighbourhoods[result][neighbour_direction]
            .iter()
            .find(|id| {
                self.tiles[id].kind != TileKind::Surface
                    && self.resolve_side_location(id, alignment_side) == pos
            })
        {
            result = next_id;
        }

        *result
    }

    /// looks for the one neighbour of the reference window (id=`reference_id`) which is:
    /// - on the `reference_side` side of the reference and
    /// - parallel to the reference on the perpendicular side with `sign=parallel_sign` (not both sides).
    /// E.g. if `reference_side=NORTH` and `parallel_sign=Negative` it will search for a tile which is:
    /// - to the north of the reference and,
    /// - parallel to the reference on the eastern side.
    /// Notes:
    /// - This only considers intra-surface neighbourhoods
    fn next_parallel_window(
        &self,
        reference_window_id: &Uuid,
        neighbour_direction: Cardinal,
        parallel_side_sign: Sign,
    ) -> Option<Uuid> {
        debug_assert!(self.tiles[&reference_window_id].kind == TileKind::Window);
        let alignment_side = Cardinal {
            axis: !neighbour_direction.axis,
            sign: parallel_side_sign,
        };
        let pos = self.resolve_side_location(reference_window_id, alignment_side);
        self.intra_surface_neighbourhoods[reference_window_id][neighbour_direction]
            .iter()
            .find(|id| {
                self.tiles[id].kind == TileKind::Window
                    && self.resolve_side_location(id, alignment_side) == pos
            })
            .copied()
    }
    /// note: a and b must be supplied in order along the cross axis of the given side or this fails
    fn are_cleanly_connected(
        &self,
        negative_window_id: &Uuid,
        positive_window_id: &Uuid,
        parallel_side: Cardinal,
    ) -> bool {
        let line_direction = Cardinal {
            axis: !parallel_side.axis,
            sign: Positive,
        };
        let end_position = self.resolve_side_location(positive_window_id, parallel_side);
        let mut current = *negative_window_id;
        let mut current_position = self.resolve_side_location(&current, parallel_side);

        while &current != positive_window_id && current_position < end_position {
            if let Some(next) =
                self.next_parallel_window(&current, line_direction, parallel_side.sign)
            {
                current = next;
                current_position = self.resolve_side_location(&current, parallel_side);
            }
        }

        current_position == end_position
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
        let mut negative_reference_id: Uuid;
        let mut positive_reference_id: Uuid;
        match strategy {
            TransformStrategy::Maximal => {
                negative_reference_id = self.furthest_parallel_window(
                    reference_tile_id,
                    cross_neg,
                    reference_side.sign,
                );
                positive_reference_id = self.furthest_parallel_window(
                    reference_tile_id,
                    cross_pos,
                    reference_side.sign,
                );
            }
            TransformStrategy::Minimal => {
                negative_reference_id = *reference_tile_id;
                positive_reference_id = *reference_tile_id;
            }
            TransformStrategy::OutwardMinimal => {
                negative_reference_id = *reference_tile_id;
                positive_reference_id = *reference_tile_id;
                // by definition if there's nothing parallel in one direction there must be something parallel in on the other axis for the same corner
                // this recovery logic is needed if we are moving strictly away from a window (e.g. to move it's edge in closer or further away from its center)
                // but we might make it configurable
                while self
                    .next_parallel_window(&negative_reference_id, reference_side, Negative)
                    .is_none()
                    && let Some(next) = self.next_parallel_window(
                        &negative_reference_id,
                        cross_neg,
                        reference_side.sign,
                    )
                {
                    negative_reference_id = next;
                }
                while self
                    .next_parallel_window(&positive_reference_id, reference_side, Positive)
                    .is_none()
                    && let Some(next) = self.next_parallel_window(
                        &positive_reference_id,
                        cross_pos,
                        reference_side.sign,
                    )
                {
                    positive_reference_id = next;
                }
            }
        }

        let maybe_outward = if let (Some(initial_negative), Some(initial_positive)) = (
            self.next_parallel_window(&negative_reference_id, reference_side, Negative),
            self.next_parallel_window(&positive_reference_id, reference_side, Positive),
        ) {
            Some(self.build_transform_fragment(
                initial_negative,
                initial_positive,
                reference_side,
            ))
        } else {
            None
        };

        let maybe_inward = if self.tiles[reference_tile_id].kind == TileKind::Window {
            Some(self.build_transform_fragment(
                positive_reference_id,
                negative_reference_id,
                !reference_side,
            ))
        } else {
            None
        };

        match (maybe_outward, maybe_inward) {
            (None, None) => None,
            some => {
                let content = match some {
                    (Some(outward), None) => {
                        if reference_side.sign == Negative {
                            TransformContent::Negative(outward)
                        } else {
                            TransformContent::Positive(outward)
                        }
                    },
                    (None, Some(inward)) => {
                        if reference_side.sign == Negative {
                            TransformContent::Positive(inward)
                        } else {
                            TransformContent::Negative(inward)
                        }
                    },
                    (Some(outward), Some(inward)) => {
                        if reference_side.sign == Negative {
                            TransformContent::Bidirectional(outward, inward)
                        } else {
                            TransformContent::Bidirectional(inward, outward)
                        }
                    },
                    _ => panic!(),
                };
                Some(TransformContainer{
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
        negative_start_id: Uuid,
        positive_start_id: Uuid,
        direction: Cardinal,
    ) -> TransformFragment {
        debug_assert!(self.tiles[&negative_start_id].kind == TileKind::Window);
        debug_assert!(self.tiles[&positive_start_id].kind == TileKind::Window);
        assert!(self.are_cleanly_connected(&negative_start_id, &positive_start_id, direction));
        let mut dividers = Vec::new();
        let mut negative_id: Uuid = negative_start_id;
        let mut positive_id: Uuid = positive_start_id;
        // note: this checks for a clean connection on the side of the direction we are building in
        // which is not the same as the expected clean line on opposite side
        let mut connected_cleanly =
            self.are_cleanly_connected(&negative_id, &positive_id, direction);

        while let (Some(next_negative), Some(next_positive)) = (
            self.next_parallel_window(&negative_id, direction, Negative),
            self.next_parallel_window(&positive_id, direction, Positive),
        ) {
            if connected_cleanly {
                dividers.push((negative_id, positive_id));
            }

            negative_id = next_negative;
            positive_id = next_positive;
            let mut negative_location = self.resolve_side_location(&negative_id, direction);
            let mut positive_location = self.resolve_side_location(&positive_id, direction);

            while negative_location != positive_location {
                if negative_location < positive_location {
                    if let Some(id) = self.next_parallel_window(&negative_id, direction, Negative) {
                        negative_id = id;
                        negative_location = self.resolve_side_location(&negative_id, direction);
                    } else {
                        break;
                    }
                } else if negative_location > positive_location {
                    if let Some(id) = self.next_parallel_window(&positive_id, direction, Positive) {
                        positive_id = id;
                        positive_location = self.resolve_side_location(&positive_id, direction);
                    } else {
                        break;
                    }
                }
            }

            connected_cleanly = negative_location == positive_location
                && self.are_cleanly_connected(&negative_id, &positive_id, direction);
        }

        TransformFragment {
            start: (negative_start_id, positive_start_id),
            dividers,
            end: (negative_id, positive_id),
        }
    }
}
