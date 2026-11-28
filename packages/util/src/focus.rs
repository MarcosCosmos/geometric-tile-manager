use crate::coord::Coord;
use geotm_core::caret::{Caret, CaretNavigationMut, CaretQuery};
use geotm_core::context::QueryContext;
use geotm_core::geometry::{Axis, Cardinal};
use geotm_core::graph::{Graph, NodeId};
use geotm_core::neighbours::{NeighbourSurfaces, NeighbourTiles, Neighbours};
use geotm_core::node::NodeQuery;
use geotm_core::query::{
    ElementInformation, ElementInformation1D, ElementInformation2D, ElementKind, GraphQuery,
    Navigation, NavigationMut, Query,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Deref;
use uuid::Uuid;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct HistoryCursor {
    coord: Coord,
    axis: Axis,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FocusState {
    target: Option<Caret>,
    cursors: HashMap<NodeId, HistoryCursor>,
}

impl FocusState {
    pub fn new() -> Self {
        Self {
            target: None,
            cursors: HashMap::new(),
        }
    }

    pub fn current(&self) -> Option<Caret> {
        self.target
    }

    /// checks if the current caret still exists in the graph, and sets it to none if so
    fn prune(&mut self, graph: &Graph) {
        self.target
            .take_if(|c| graph.resolve(Uuid::from(c.id)).is_err());
        self.cursors
            .retain(|&id, _| graph.resolve(Uuid::from(id)).is_ok());
    }

    /// returns none if there is no current focus
    pub fn try_navigate<'b>(&mut self, graph: &'b Graph) -> Option<FocusNavigator<'_, 'b>> {
        self.prune(graph);
        if self.target.is_some() {
            Some(FocusNavigator {
                state: self,
                context: graph,
            })
        } else {
            None
        }
    }

    pub fn navigate<'b>(&mut self, graph: &'b Graph, default: Caret) -> FocusNavigator<'_, 'b> {
        self.prune(graph);
        let was_none = self.target.is_none();
        let mut nav = FocusNavigator {
            state: self,
            context: graph,
        };
        if was_none {
            nav.set_target(default);
        }
        nav
    }
}

impl Default for FocusState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct FocusNavigator<'a, 'b> {
    state: &'a mut FocusState,
    context: &'b Graph,
}

/// The navigator does it's best to stick to facilitate navigating tiles in a stable and minimal fashion.
/// When focusing directionally, it will stick to any gutter it is already in if possible for that direction.
/// Otherwise, it will look for the neighbour in the given direction that is closest (via the center Coord).
/// If that neighbour is the containing surface, it will try to move to the next surface over. If that succeeds, it will try to check for a similarly nearest tile inside that surface.
/// When moving outside of a gutter, the resulting caret will always be on the side facing opposite the requested direction.
/// It also maintains miminal single-axis history to ensure these movements are reversible.
///
/// Note: the navigator is a short-lived controller that wraps an internal state and an immutable graph reference, so that said graph reference can be freely released.
/// todo: this may potentially manage shrinking and expanding selection gutters about the focus in future
impl<'a, 'b> FocusNavigator<'a, 'b> {
    fn active_cursor(&self) -> &HistoryCursor {
        &self.state.cursors[&*Query::new(self.context, (**self).id).to_surface()]
    }
}

impl Deref for FocusNavigator<'_, '_> {
    type Target = Caret;

    fn deref(&self) -> &Self::Target {
        self.state.target.as_ref().unwrap()
    }
}

impl Navigation for FocusNavigator<'_, '_> {
    type Direction = Cardinal;
    fn peek(&self, direction: Cardinal) -> Option<Caret> {
        let current = Query::new(self.context, **self);

        if direction.axis != self.side.axis
            && let Some(result) = current.peek(direction.sign)
        {
            Some(result)
        } else {
            let cursor = self.active_cursor();
            let axis = direction.axis;
            let historical_location = cursor.coord[axis];
            let candidates = {
                let prelims: Query<Neighbours> = match current.kind() {
                    ElementKind::Surface => current.to(NeighbourSurfaces(*current)).into(),
                    ElementKind::Tile => {
                        let immediate = current.to(NeighbourTiles(*current));
                        if let Some(node) = immediate.into_iter().next()
                            && node.kind() == ElementKind::Surface
                        {
                            current
                                .to(NeighbourSurfaces(*node.to_side(direction)))
                                .into()
                        } else {
                            immediate.into()
                        }
                    }
                    _ => panic!(),
                };
                match *prelims {
                    Neighbours::Surfaces(_)
                        if let Some(next_surface) = prelims.into_iter().min_by_key(|node| {
                            node.bounds()[axis].distance_to(historical_location)
                        }) =>
                    {
                        let distant =
                            next_surface.to(NeighbourTiles(*next_surface.to_side(direction)));
                        if distant
                            .into_iter()
                            .any(|node| node.kind() == ElementKind::Tile)
                        {
                            distant.into()
                        } else {
                            prelims
                        }
                    }
                    _ => prelims,
                }
            };
            candidates
                .into_iter()
                .min_by_key(|node| node.bounds()[axis].distance_to(historical_location))
                .map(|node| Caret::new(*node, !direction))
        }
    }
}

impl CaretNavigationMut for FocusNavigator<'_, '_> {
    fn set_node(&mut self, node_id: NodeId) {
        self.state.target.as_mut().unwrap().id = node_id;
    }

    fn set_side(&mut self, side: Cardinal) {
        self.state.target.as_mut().unwrap().side = side;
    }
}

impl NavigationMut for FocusNavigator<'_, '_> {
    /// Returns none if there is no neighbour to focus in the given direction
    /// Note: if the result is none, the current focus is unchanged
    fn next(&mut self, direction: Cardinal) -> Option<&mut Self> {
        // for now, we try not to assume that the state memory is up to date
        // we'll have to decide whether or not to deliberately clean it after operations but for now it is relatively safer to do a check
        // also this draft doesn't currently account for content swapping, which would probably be supported by mapping layer instead of managing more complex graph manipulations
        // for that we will probably need id type parameterisation for carets, e.g. with a newtype for outer ids, and have outer ids be what is stored in the nav state.
        let target = self.peek(direction)?;
        self.state.target = Some(target);
        // this is the history for the new target now!
        let target = Query::new(self.context, target);
        let target_surface_id = *target.to_node().to_surface();
        let bounds = target.bounds();
        if let Some(cursor) = self.state.cursors.get_mut(&target_surface_id) {
            if direction.axis == cursor.axis {
                cursor.coord[direction.axis] = bounds.side_location;
            } else {
                cursor.coord[cursor.axis] = bounds.axis_bounds.center();
            }
            cursor.axis = direction.axis;
        } else {
            let axis = target.side.axis;
            let mut location = Coord::default();
            location[axis] = bounds.axis_bounds.center();
            location[!axis] = bounds.side_location;
            self.state.cursors.insert(
                target_surface_id,
                HistoryCursor {
                    coord: location,
                    axis,
                },
            );
        }

        Some(self)
    }

    /// This allows arbitrary updates to the focus.
    /// Always clears the history
    fn set_target(&mut self, target: Caret) {
        if target != **self {
            self.state.target = Some(target);
            let target = Query::new(self.context, target);
            let bounds = target.bounds();
            let surface_id = *target.to(target.id).to_surface();
            let axis = target.side.axis;
            let mut location = Coord::default();
            location[axis] = bounds.axis_bounds.center();
            location[!axis] = bounds.side_location;
            self.state.cursors.insert(
                surface_id,
                HistoryCursor {
                    coord: location,
                    axis,
                },
            );
        }
    }
}
