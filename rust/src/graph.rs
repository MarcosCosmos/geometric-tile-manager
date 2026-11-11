use crate::geometry::{Axis::*, Cardinal, Rect, Sign::*};
use crate::result::{Error, Result};
use crate::transform::TransformContraints;
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

/// Notes:
/// - There are inter-surface and intra-surface neighbourhoods which have different meanings.
/// - Gaps may be present either on the inside of a surface or between windows; in htis sense gaps would potentially alter poorly written algorithms
/// - Storing tiles seperately from neighboy
#[derive(Clone, Debug, Default)]
pub struct Graph {
    pub tiles: HashMap<Uuid, Tile>,
    /// Note: each surface has an entry in its own interior neighbourhood.
    /// The meaning of direction is semi-inverted in that the surface's edges face inwards not outwards, but directionality is otherwise the same.
    /// E.g. the eastern neighbourhood of a surface faces inwardly east from the western side, whereas a normal window would face outwardly east from the eastern side.
    pub intra_surface_neighbourhoods: HashMap<Uuid, Neighbourhood>,
    /// inter_surface_neighbours are the neighbourhoods between surfaces, and therefore face outwards.
    pub inter_surface_neighbourhoods: HashMap<Uuid, Neighbourhood>,
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

    pub fn new_window(
        &mut self,
        target_id: &Uuid,
        direction: Cardinal,
        strategy: TransformContraints,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        // todo: this is redundant because of internal similar calls. Might actually be easier to go with command/message processing so we can do prechecks?a
        if !self.tiles.contains_key(target_id) {
            return Err(Error::TileNotFound(*target_id));
        }

        let rect: Rect; // todo

        self.tiles.insert(id, Tile::new(id, TileKind::Window, rect));

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
                neighbours_of_target.push(id); // our new window is guaranteed to be the only neighbour, so we can simply push
            }

            // todo: maybe this is not sufficient because our new window could erase another neighbour (?)
            // might be worth extracting to a method anyway.
            for id in neighbours_of_window {
                self.intra_surface_neighbourhoods
                    .get_mut(id)
                    .unwrap()
                    .replace(opposite, target_id, *id);
            }
        }

        // todo: figure out how mix window neighbour with other gutter corrections?

        Ok(id)
    }
}
