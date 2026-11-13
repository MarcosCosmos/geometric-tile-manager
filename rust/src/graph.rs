use crate::geometry::{Axis::*, AxisBounds, Bounds, Cardinal, Sign::*};
use crate::navigation::Line;
use crate::result::Result;
use crate::transform::TransformConstraints;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::{Index, IndexMut};
use uuid::Uuid;

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

#[derive(Clone, Debug, Default)]
struct Neighbourhood {
    north: Vec<NodeId>,
    south: Vec<NodeId>,
    east: Vec<NodeId>,
    west: Vec<NodeId>,
}

/// Notes:
/// - There are inter-surface and intra-surface neighbourhoods which have different meanings.
/// - Gaps may be present either on the inside of a surface or between tiles; in htis sense gaps would potentially alter poorly written algorithms
/// - Storing tiles seperately from neighboy
#[derive(Clone, Debug, Default)]
pub struct Graph {
    pub bounds: HashMap<NodeId, Bounds>,

    pub containing_surface: HashMap<TileId, SurfaceId>,
    /// Note: each surface has an entry in its own interior neighbourhood.
    /// The meaning of direction is semi-inverted in that the surface's edges face inwards not outwards, but directionality is otherwise the same.
    /// E.g. the eastern neighbourhood of a surface faces inwardly east from the western side, whereas a normal tile would face outwardly east from the eastern side.
    pub intra_surface_neighbourhoods: HashMap<NodeId, Neighbourhood>,
    /// inter_surface_neighbours are the neighbourhoods between surfaces, and therefore face outwards.
    pub inter_surface_neighbourhoods: HashMap<SurfaceId, Neighbourhood>,
}

impl Neighbourhood {
    pub fn new() -> Self {
        Self::default()
    }
    /// inserts the tile's id in order
    pub fn insert<'a, F>(&mut self, direction: Cardinal, subject_id: NodeId, subject_bounds: &AxisBounds, bounds_lookup: F)
    where
        F: Fn(&NodeId) -> &'a AxisBounds,
    {
        let cmp_direction = Cardinal {
            axis: !direction.axis,
            sign: Positive,
        };
        let neighbours = &mut self[direction];
        let pos = neighbours.partition_point(|sib_id| {
            Ordering::Greater != bounds_lookup(sib_id).cmp(subject_bounds).into()
        });
        neighbours.insert(pos, subject_id);
    }

    pub fn replace(&mut self, direction: Cardinal, old: &NodeId, new: NodeId) {
        let dest = &mut self[direction];
        let pos = dest.iter().position(|x| x == old).unwrap();
        dest[pos] = new;
    }
}

impl Index<Cardinal> for Neighbourhood {
    type Output = Vec<NodeId>;
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

impl IndexMut<Cardinal> for Neighbourhood {
    fn index_mut(&mut self, index: Cardinal) -> &mut Self::Output {
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


impl Graph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_surface(&mut self, bounds: Bounds) -> SurfaceId {
        let id = SurfaceId::new();
        self.bounds.insert(id.into(), bounds);
        self.intra_surface_neighbourhoods
            .insert(id.into(), Neighbourhood::new());
        self.inter_surface_neighbourhoods
            .insert(id, Neighbourhood::new());
        id
    }

    pub fn new_tile(
        &mut self,
        direction: Cardinal,
        origin: Line,
        constraints: TransformConstraints,
    ) -> Result<Uuid> {
        assert!(origin.is_connected_on(self));
        let bounds: Bounds; // todo
        let id = TileId::new();
        self.bounds.insert(id.into(), bounds);
        self.containing_surface.insert(id, self.containing_surface[&origin.start_id.unwrap_tile()]);

        // todo: all of this only works if strictly splitting the target. A more complicated strategy is needed for resizing all along the parallel

        let mut tile_neighbourhood = Neighbourhood::new();

        // adjust connections
        // simple part: the direction parallel to the insertion
        {
            let opposite = !direction;
            let neighbours_of_tile = &mut tile_neighbourhood[direction];
            {
                let neighbours_of_target = &mut self
                    .intra_surface_neighbourhoods
                    .get_mut(target_id)
                    .unwrap()[direction];
                std::mem::swap(neighbours_of_target, neighbours_of_tile);
                neighbours_of_target.push(id); // our new tile is guaranteed to be the only neighbour, so we can simply push
            }

            // todo: maybe this is not sufficient because our new tile could erase another neighbour (?)
            // might be worth extracting to a method anyway.
            for id in neighbours_of_tile {
                self.intra_surface_neighbourhoods
                    .get_mut(id)
                    .unwrap()
                    .replace(opposite, target_id, *id);
            }
        }

        // todo: figure out how mix tile neighbour with other gutter corrections?

        Ok(id)
    }
}
