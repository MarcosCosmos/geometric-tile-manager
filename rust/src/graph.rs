use crate::geometry::{Axis, Axis::*, Cardinal, Diagonal, Orientation, Orientation::*, Point, Rect};
use crate::ops::ContainerStrategy;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::write;
use std::ops::{Index, IndexMut};
use uuid::Uuid;
// /// todo: these are currently mostly explanatory as to why I don't directly name other structs these structs, figure out if we'll actually use them.
// pub struct Side {
//     direction: Cardinal,
//     edge: Edge,
// }
//
// pub struct Corner {
//     direction: Diagonal,
//     location: Point,
// }

use crate::result::{Error, Result};

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

struct Corner {
    pub tile_id: Uuid,
    pub direction: Diagonal,
}

/// A simple transform end is just the far side of a window
/// In more complex scenario occurs when the windows on the far side of a row/column don't line up neatly.
/// In that case, the farthest box has to be artificially subdivided some distance along its length.
enum TransformEnd {
    Simple(Uuid),
    Partial(Uuid, i32),
}

struct DirectedTransformContainer {
    pub origin: Uuid,
    pub direction: Cardinal,
    pub lines: Vec<(Uuid, Uuid)>,
    pub terminal_line: Option<(TransformEnd, TransformEnd)>,
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
            orientation: End,
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
                orientation: Start,
            } => &self.north,
            Cardinal {
                axis: Y,
                orientation: End,
            } => &self.south,
            Cardinal {
                axis: X,
                orientation: Start,
            } => &self.west,
            Cardinal {
                axis: X,
                orientation: End,
            } => &self.east,
        }
    }
}

impl IndexMut<Cardinal> for Neighbourhood {
    fn index_mut(&mut self, index: Cardinal) -> &mut Self::Output {
        match index {
            Cardinal {
                axis: Y,
                orientation: Start,
            } => &mut self.north,
            Cardinal {
                axis: Y,
                orientation: End,
            } => &mut self.south,
            Cardinal {
                axis: X,
                orientation: Start,
            } => &mut self.west,
            Cardinal {
                axis: X,
                orientation: End,
            } => &mut self.east,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Graph {
    tiles: HashMap<Uuid, Tile>,
    neighbourhoods: HashMap<Uuid, Neighbourhood>,
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
        self.neighbourhoods.insert(id, Neighbourhood::new());
        id
    }

    pub fn insert_window(
        &mut self,
        window_id: Uuid,
        target_id: &Uuid,
        direction: Cardinal,
        strategy: ContainerStrategy,
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

        // todo: this only works if strictly splitting the target. A more complicated strategy is needed for resizing all along the parallel

        let mut window_neighbourhood = Neighbourhood::new();

        // adjust connections
        // simple part: the direction parallel to the insertion
        {
            let opposite = !direction;
            let neighbours_of_window = &mut window_neighbourhood[direction];
            {
                let neighbours_of_target =
                    &mut self.neighbourhoods.get_mut(target_id).unwrap()[direction];
                std::mem::swap(neighbours_of_target, neighbours_of_window);
                neighbours_of_target.push(window_id); // our new window is guaranteed to be the only neighbour, so we can simply push
            }

            // todo: maybe this is not sufficient because our new window could erase another neighbour (?)
            // might be worth extracting to a method anyway.
            for id in neighbours_of_window {
                self.neighbourhoods
                    .get_mut(id)
                    .unwrap()
                    .replace(opposite, target_id, window_id);
            }
        }

        // the perpendicular is a little more complicated
        for orientation in [Start, End] {
            let cross_direction = Cardinal {
                axis: !direction.axis,
                orientation,
            };
            let cross_opposite = !cross_direction;
            // we have to move the neighbour vector out of the tile map in order to hold other mutable references
            let mut neighbours_of_target = std::mem::replace(
                &mut self.neighbourhoods.get_mut(target_id).unwrap()[cross_direction],
                Vec::new(),
            );
            let former_neighbours_of_target = neighbours_of_target.extract_if(.., |id| {
                direction.cmp(&self.tiles[&id].rect, &window.rect) != Ordering::Less
            });
            let neighbours_of_window = &mut window_neighbourhood[cross_direction];
            neighbours_of_window.extend(former_neighbours_of_target);
            // swap out the old neighbours; todo: this might not be correct enough (see above)
            for id in neighbours_of_window {
                self.neighbourhoods
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
                self.neighbourhoods.get_mut(&shared_id).unwrap().insert(
                    cross_opposite,
                    |id| &self.tiles[id],
                    &window,
                );
                window_neighbourhood.insert(
                    cross_direction,
                    |id| &self.tiles[id],
                    &self.tiles[&shared_id],
                );
            }

            // now swap it back in
            std::mem::swap(
                &mut neighbours_of_target,
                &mut self.neighbourhoods.get_mut(target_id).unwrap()[cross_direction],
            );
        }

        self.tiles.insert(window_id, window);
        self.neighbourhoods.insert(window_id, window_neighbourhood);

        Ok(window.rect)
    }

    /// Creates a space for a new window on the specified side of the target tile.
    /// The method both calculates where the space will be and displaces other windows as necessary to permit it.
    /// Note: if the target is a surface, space is made inside, otherwise it is made beside.
    fn make_space(
        &mut self,
        target_id: &Uuid,
        side: Cardinal,
        strategy: ContainerStrategy,
    ) -> Result<Rect> {
        // todo: this is redundant because of internal similar calls. Might actually be easier to go with command/message processing so we can do prechecks?a
        if !self.tiles.contains_key(target_id) {
            return Err(Error::TileNotFound(*target_id));
        }

        // todo: should this actually be an oriented container?
        if let Some(transform_container) = self.find_directed_container(target_id, side, strategy)? {
            let segment_count = transform_container.lines.len();
            if transform_container.terminal_line.is_some() {
                segment_count += 1;
            }


            // todo:n
        } else {
            Ok(self.tiles[target_id].rect)
        }
    }

    fn resolve_corner_position(&self, tile_id: &Uuid, direction: Diagonal) -> Point {
        let rect = &self.tiles[tile_id].rect;
        match direction {
            Diagonal { x: Start, y: Start } => Point::new(rect.x, rect.y),
            Diagonal { x: End, y: Start } => Point::new(rect.x + rect.width, rect.y),
            Diagonal { x: Start, y: End } => Point::new(rect.x, rect.y + rect.height),
            Diagonal { x: End, y: End } => Point::new(rect.x + rect.width, rect.y + rect.height),
        }
    }

    /// resolves the side's position along the axis parallel to the corresponding line
    fn resolve_side_position(&self, tile_id: &Uuid, side: Cardinal) -> i32 {
        let rect = &self.tiles[tile_id].rect;
        match side {
            Cardinal { axis: X, orientation: Start } => rect.x,
            Cardinal { axis: X, orientation: End } => rect.x + rect.width,
            Cardinal { axis: Y, orientation: Start } => rect.y,
            Cardinal { axis: Y, orientation: End } => rect.y + rect.height,
        }
    }

    /// scans through the graph until it hits a wall (either a surface or a window that is not aligned with the initial)
    fn furthest_window_in_line(
        &self,
        initial_id: &Uuid,
        direction: Cardinal,
        cross_orientation: Orientation,
    ) -> Uuid {
        let mut result = initial_id;
        let cross_direction = Cardinal {
            axis: !direction.axis,
            orientation: cross_orientation,
        };
        let pos = self.resolve_side_position(result, cross_direction);

        while let Some(next_id) = &self.neighbourhoods[result]
            [direction]
            .iter()
            .find(|id| self.tiles[id].kind != TileKind::Surface
                && self.resolve_side_position(id, cross_direction) == pos
            ) {
            result = next_id;
        }

        *result
    }

    /// the far corner as in the corner on the far side of the next box.
    /// The starting point is always notionally on the far side of the initial tile
    fn next_tile_in_line(
        &self,
        initial_id: &Uuid,
        direction: Cardinal,
        cross_orientation: Orientation,
    ) -> Option<Uuid> {

        let cross_direction = Cardinal {
            axis: !direction.axis,
            orientation: cross_orientation,
        };
        let pos = self.resolve_side_position(initial_id, cross_direction);
        self.neighbourhoods[initial_id][direction]
            .iter()
            .find(|id| self.resolve_side_position(id, cross_direction) == pos)
            .copied()
    }

    fn next_window_in_line(
        &self,
        initial_id: &Uuid,
        direction: Cardinal,
        cross_orientation: Orientation,
    ) -> Option<Uuid> {
        self.next_tile_in_line(initial_id, direction, cross_orientation)
            .filter(|id| self.tiles[id].kind == TileKind::Window)
    }

    /// note: a and b must be supplied in order along the cross axis of the given side or this fails
    fn has_clean_line_between(&self, id_a: &Uuid, id_b: &Uuid, side: Cardinal) -> bool {
        let search_direction = Cardinal {
            axis: !side.axis,
            orientation: End
        };
        let end_position = self.resolve_side_position(id_b, side);
        let mut current = *id_a;
        let mut current_position = self.resolve_side_position(&current, side);

        while &current != id_b && current_position < end_position {
            if let Some(next) = self.next_tile_in_line(&current, search_direction, End) {
                current = next;
                current_position = self.resolve_side_position(&current, side);
            }
        }

        current_position == end_position
    }

    fn find_transform_container(
        &self,
        target_id: &Uuid,
        side: Cardinal,
        strategy: ContainerStrategy,
    ) -> Result<Option<DirectedTransformContainer>> {
        if !self.tiles.contains_key(target_id) {
            return Err(Error::TileNotFound(*target_id));
        }
        let target = self.tiles[target_id];
        let mut start_id: Uuid;
        let mut end_id: Uuid;
        match strategy {
            ContainerStrategy::Maximal => {
                start_id = self.furthest_window_in_line(target_id, cross_start, side.orientation);
                end_id = self.furthest_window_in_line(target_id, cross_end, side.orientation);
            }
            ContainerStrategy::Minimal => {
                start_id = *target_id;
                end_id = *target_id;
                // todo: this part should only be used in the strictly directed case, which we don't have  params for yet
                // next_start = self.next_tile_in_line(&start_id, side, Start);
                // next_end = self.next_tile_in_line(&start_id, side, End);
                // while {
                //     next_start = self.next_tile_in_line(&start_id, side, Start);
                //     next_start.is_none()
                // }
                // {
                //     start_id = self
                //         .next_tile_in_line(&start_id, cross_start, side.orientation)
                //         .unwrap();
                // }
                // while {
                //     next_end = self.next_tile_in_line(&end_id, side, End);
                //     next_end.is_none()
                // }
                // {
                //     end_id = self
                //         .next_tile_in_line(&end_id, cross_end, side.orientation)
                //         .unwrap();
                // }
            }
        }

        if &start_id == target_id && &end_id == target_id && target.kind == TileKind::Surface {
            // we're dealing with an empty surface, so a transform container would be redundant
            return Ok(None);
        }

        // todo call the inner find_directed_container etc as needed according to whether we want both directions, and merge, and return result with possible NothingToTransform error

    }


    // todo: figure out how to use this to compute the bidirectional form.
    fn find_directed_container(
        &self,
        initial_start: &Uuid,
        initial_end: &Uuid,
        side: Cardinal
    ) -> Option<DirectedTransformContainer> {
        let (cross_start, cross_end) = side.cross_directions();

        let mut lines = Vec::new();
        let mut start_id: Uuid = *initial_start;
        let mut end_id: Uuid = *initial_end;
        let mut next_start: Option<Uuid>;
        let mut next_end: Option<Uuid>;
        let mut start_pos: i32;
        let mut end_pos: i32;

        next_start = self.next_window_in_line(&start_id, side, Start);
        next_end = self.next_window_in_line(&start_id, side, End);

        if next_start.is_none() || next_end.is_none()  {
            return None;
        }

        // todo: figure out what to do with initial start

        while let (Some(_start_id), Some(_end_id)) = (next_start, next_end) {
            start_id = _start_id;
            end_id = _end_id;
            start_pos = self.resolve_side_position(&start_id, side);
            end_pos = self.resolve_side_position(&end_id, side);
            while start_pos != end_pos {
                if start_pos < end_pos {
                    next_start = self.next_window_in_line(&start_id, side, Start);
                    if let Some(id) = next_start {
                        start_id = id;
                        start_pos = self.resolve_side_position(&start_id, side);
                    } else {
                        break;
                    }
                } else if start_pos > end_pos {
                    next_end = self.next_window_in_line(&end_id, side, End);
                    if let Some(id) = next_end {
                        end_id = id;
                        end_pos = self.resolve_side_position(&end_id, side);
                    } else {
                        break;
                    }
                }
            }
            if start_pos == end_pos {
                if self.has_clean_line_between(&start_id, &end_id, side) {
                    // it's only a valid segment if there is a clean line through the middle
                    lines.push((start_id, end_id));
                }
                next_start = self.next_window_in_line(&start_id, side, Start);
                next_end = self.next_tile_in_line(&end_id, side, End);
            }
        }

        let terminal_line = if self.has_clean_line_between(&start_id, &end_id, side) {
            None
        } else {
            start_pos = self.resolve_side_position(&start_id, side);
            end_pos = self.resolve_side_position(&end_id, side);
            if start_pos > end_pos {
                Some((TransformEnd::Partial(start_id, end_pos), TransformEnd::Simple(end_id)))
            } else {
                Some((TransformEnd::Simple(start_id), TransformEnd::Partial(end_id, start_pos)))
            }
        };

        Ok(Some(DirectedTransformContainer {
            origin: *target_id,
            direction: side,
            lines,
            terminal_line,
        }))
    }
}
