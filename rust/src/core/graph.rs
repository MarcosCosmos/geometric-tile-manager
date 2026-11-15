use crate::core::geometry::{Axis::*, Bounds, Cardinal, RectBounds, Sign::*};
use crate::core::navigation::{Caret, Context, Line, RawInformationSource};
use crate::core::transform::TransformConstraints;
use crate::result::Result;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::sync::atomic::Ordering::SeqCst;
use uuid::Uuid;
use crate::core::graph::NodeId::Surface;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TileId(Uuid);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct SurfaceId(Uuid);

impl TileId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl SurfaceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum NodeId {
    Tile(TileId),
    Surface(SurfaceId),
}

impl NodeId {
    pub fn is_tile(&self) -> bool {
        match self {
            NodeId::Tile(_) => true,
            _ => false,
        }
    }

    pub fn is_surface(&self) -> bool {
        !self.is_tile()
    }

    pub fn unwrap_tile(self) -> TileId {
        match self {
            NodeId::Tile(id) => id,
            _ => panic!(),
        }
    }

    pub fn unwrap_surface(self) -> SurfaceId {
        match self {
            NodeId::Surface(id) => id,
            _ => panic!(),
        }
    }
}

impl From<TileId> for NodeId {
    fn from(value: TileId) -> Self {
        Self::Tile(value)
    }
}

impl From<SurfaceId> for NodeId {
    fn from(value: SurfaceId) -> Self {
        Self::Surface(value)
    }
}

#[derive(Clone, Debug)]
pub(super) struct Neighbourhood<T> {
    north: Vec<T>,
    south: Vec<T>,
    east: Vec<T>,
    west: Vec<T>,
}

/// Notes:
/// - There are inter-surface and intra-surface neighbourhoods which have different meanings.
/// - Gaps may be present either on the inside of a surface or between tiles; in htis sense gaps would potentially alter poorly written algorithms
/// - Storing tiles seperately from neighboy
#[derive(Clone, Debug, Default)]
pub struct Graph {
    pub(super) bounds: HashMap<NodeId, RectBounds>,

    pub(super) containing_surface: HashMap<TileId, SurfaceId>,
    /// Note: each surface has an entry in its own interior neighbourhood.
    /// The meaning of direction is semi-inverted in that the surface's edges face inwards not outwards, but directionality is otherwise the same.
    /// E.g. the eastern neighbourhood of a surface faces inwardly east from the western side, whereas a normal tile would face outwardly east from the eastern side.
    tile_neighbourhoods: HashMap<NodeId, Neighbourhood<NodeId>>,
    /// inter_surface_neighbours are the neighbourhoods between surfaces, and therefore face outwards.
    surface_neighbourhoods: HashMap<SurfaceId, Neighbourhood<NodeId>>,
}

impl <T> Index<Cardinal> for Neighbourhood<T> {
    type Output = Vec<T>;
    fn index(&self, index: Cardinal) -> &Self::Output {
        match index {
            Cardinal {
                axis: Vertical,
                sign: Negative,
            } => &self.north,
            Cardinal {
                axis: Vertical,
                sign: Positive,
            } => &self.south,
            Cardinal {
                axis: Horizontal,
                sign: Negative,
            } => &self.west,
            Cardinal {
                axis: Horizontal,
                sign: Positive,
            } => &self.east,
        }
    }
}

impl <T> IndexMut<Cardinal> for Neighbourhood<T> {
    fn index_mut(&mut self, index: Cardinal) -> &mut Vec<T> {
        match index {
            Cardinal {
                axis: Vertical,
                sign: Negative,
            } => &mut self.north,
            Cardinal {
                axis: Vertical,
                sign: Positive,
            } => &mut self.south,
            Cardinal {
                axis: Horizontal,
                sign: Negative,
            } => &mut self.west,
            Cardinal {
                axis: Horizontal,
                sign: Positive,
            } => &mut self.east,
        }
    }
}

impl <T> Default for Neighbourhood<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl <T> Neighbourhood<T> {
    pub fn new() -> Self {
        Self {
            north: Vec::new(),
            south: Vec::new(),
            east: Vec::new(),
            west: Vec::new(),
        }
    }
}

pub(super) struct NeighbourhoodContext<'a, 'b, T> where T: Copy + Into<NodeId> {
    bounds: &'a HashMap<NodeId, RectBounds>,
    side: Cardinal,
    entries: &'b mut Vec<T>,
}

impl <T> NeighbourhoodContext<'_, '_, T> where T: Copy + Into<NodeId> + Eq + PartialEq {
    /// inserts the tile's id in order
    pub fn insert(
        &mut self,
        subject_id: T,
    ) where NodeId: From<T>
    {
        let subject_bounds = self.bounds[&subject_id.into()][!self.side.axis];
        let pos = self.entries.partition_point(|sib_id| {
            Ordering::Greater != self.bounds[&NodeId::from(*sib_id)][!self.side.axis].cmp(&subject_bounds).into()
        });
        self.entries.insert(pos, subject_id);
    }

    pub fn replace(&mut self, old: &T, new: T) {
        let pos = self.entries.iter().position(|x| x == old).unwrap();
        self.entries[pos] = new;
    }

    pub fn remove(&mut self, target: &T) {
        self.entries.retain(|x| x != target);
    }
}

impl Graph {
    pub fn new() -> Self {
        Self::default()
    }

    pub(super) fn get_tile_neighbourhood(&self, caret: Caret) -> &Vec<NodeId> {
        &self.tile_neighbourhoods[&caret.id][caret.side]
    }

    pub(super) fn get_surface_neighbourhood(&self, caret: Caret<SurfaceId>) -> &Vec<NodeId> {
        &self.surface_neighbourhoods[&caret.id][caret.side]
    }

    pub(super) fn get_tile_neighbourhood_mut(&mut self, caret: Caret) -> NeighbourhoodContext<NodeId> {
        NeighbourhoodContext { bounds: &self.bounds, side: caret.side, entries: &mut self.tile_neighbourhoods.get_mut(&caret.id).unwrap()[caret.side] }
    }

    pub(super) fn get_surface_neighbourhood_mut(&mut self, caret: Caret) -> NeighbourhoodContext<NodeId> {
        NeighbourhoodContext { bounds: &self.bounds, side: caret.side, entries: &mut self.surface_neighbourhoods.get_mut(&caret.id.unwrap_surface()).unwrap()[caret.side] }
    }

    pub fn new_surface(&mut self, bounds: RectBounds) -> SurfaceId {
        let id = SurfaceId::new();
        self.bounds.insert(id.into(), bounds);
        self.tile_neighbourhoods
            .insert(id.into(), Neighbourhood::new());
        self.surface_neighbourhoods
            .insert(id, Neighbourhood::new());
        id
    }

    pub fn new_tile(
        &mut self,
        direction: Cardinal,
        origin: Line,
        constraints: TransformConstraints,
    ) -> Result<TileId> {
        let origin = self.info(origin);
        let bounds: RectBounds ; // todo
        let id = TileId::new();
        self.bounds.insert(id.into(), bounds);
        self.containing_surface.insert(
            id,
            self.containing_surface[&origin[Negative].unwrap_tile()],
        );

        // todo: all of this only works if strictly splitting the target. A more complicated strategy is needed for resizing all along the parallel

        let mut tile_neighbourhood = Neighbourhood::new();

        // adjust connections
        // simple part: the direction parallel to the insertion
        {
            let opposite = !direction;
            let neighbours_of_tile = &mut tile_neighbourhood[direction];
            {
                let neighbours_of_target = &mut self
                    .tile_neighbourhoods
                    .get_mut(target_id)
                    .unwrap()[direction];
                std::mem::swap(neighbours_of_target, neighbours_of_tile);
                neighbours_of_target.push(id); // our new tile is guaranteed to be the only neighbour, so we can simply push
            }

            // todo: maybe this is not sufficient because our new tile could erase another neighbour (?)
            // might be worth extracting to a method anyway.
            for id in neighbours_of_tile {
                self.tile_neighbourhoods
                    .get_mut(id)
                    .unwrap()
                    .replace(opposite, target_id, *id);
            }
        }

        // todo: figure out how mix tile neighbour with other gutter corrections?

        Ok(id)
    }
}

