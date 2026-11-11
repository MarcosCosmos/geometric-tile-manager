use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{Axis, Cardinal, Sign};
use crate::graph::{Graph, TileKind};
use crate::gutter::GutterNavigation;
use crate::navigation::{Caret, Navigator};
use std::ops::Deref;
use uuid::Uuid;

pub struct FocusState {
    inner: Option<Caret>,
    negative_history: Vec<Caret>,
    positive_history: Vec<Caret>,
    history_axis: Axis,
}

impl FocusState {
    pub fn new() -> Self {
        Self {
            inner: None,
            negative_history: Vec::new(),
            positive_history: Vec::new(),
            history_axis: Axis::X,
        }
    }

    pub fn current(&self) -> Option<Caret> {
        self.inner
    }

    /// checks if the current caret still exists in the graph, and sets it to none if so
    pub fn sanitize(&mut self, graph: &Graph) {
        self.inner
            .take_if(|c| !graph.tiles.contains_key(&c.tile_id));
    }

    /// returns none if there is no current focus
    pub fn try_navigate<'b>(&mut self, graph: &'b Graph) -> Option<FocusNav<'_, 'b>> {
        self.sanitize(graph);
        if self.inner.is_some() {
            Some(FocusNav { state: self, graph })
        } else {
            None
        }
    }

    pub fn navigate<'b>(&mut self, graph: &'b Graph, default: Caret) -> FocusNav<'_, 'b> {
        self.sanitize(graph);
        self.inner.get_or_insert(default);
        FocusNav { state: self, graph }
    }
}

pub struct FocusNav<'a, 'b> {
    state: &'a mut FocusState,
    graph: &'b Graph,
}

/// The navigator does it's best to stick to facilitate navigating windows in a stable and minimal fashion.
/// When focusing directionally, it will stick to any gutter it is already in if possible for that direction.
/// Otherwise, it will look for the neighbour in the given direction that is closest (via the center point).
/// If that neighbour is the containing surface, it will try to move to the next surface over. If that succeeds, it will try to check for a similarly nearest window inside that surface.
/// When moving outside of a gutter, the resulting caret will always be on the side facing opposite the requested direction.
/// It also maintains miminal single-axis history to ensure these movements are reversible.
///
/// Note: the navigator is a short-lived controller that wraps an internal state and an immutable graph reference, so that said graph reference can be freely released.
/// todo: this may potentially manage shrinking and expanding selection gutters about the focus in future
impl<'a, 'b> FocusNav<'a, 'b> {
    fn history(&self, sign: Sign) -> &Vec<Caret> {
        match sign {
            Negative => &self.state.negative_history,
            Positive => &self.state.positive_history,
        }
    }

    fn history_mut(&mut self, sign: Sign) -> &mut Vec<Caret> {
        match sign {
            Negative => &mut self.state.negative_history,
            Positive => &mut self.state.positive_history,
        }
    }

    fn candidate_neighbours(&self, direction: Cardinal) -> &Vec<Uuid> {
        let current = self.state.inner.unwrap();
        let mut candidates = &self.graph.intra_surface_neighbourhoods[&current.tile_id][direction];
        if candidates.len() == 1 && self.graph.tiles[&candidates[0]].kind == TileKind::Surface {
            let other_canidates =
                &self.graph.intra_surface_neighbourhoods[&candidates[0]][direction];
            if other_canidates
                .iter()
                .any(|id| self.graph.tiles[id].kind != TileKind::Surface)
            {
                &other_canidates
            } else {
                candidates
            }
        } else {
            candidates
        }
    }
}

impl Deref for FocusNav<'_, '_> {
    type Target = Caret;

    fn deref(&self) -> &Self::Target {
        self.state.inner.as_ref().unwrap()
    }
}

impl<'a, 'b> Navigator for FocusNav<'_, '_> {
    type Direction = Cardinal;

    fn location(&self) -> i32 {
        self.location_on(self.graph)
    }

    fn peek(&self, direction: Cardinal) -> Option<Caret> {
        let candidates = self.candidate_neighbours(direction);

        if self.state.history_axis == direction.axis
            && let Some(&previous) = self
                .history(direction.sign)
                .last()
                .filter(|c| candidates.contains(&c.tile_id))
        {
            Some(previous)
        } else if direction.axis != self.side.axis
            && let Some(caret) = self.navigate_gutter(self.graph).peek(direction.sign)
        {
            Some(caret)
        } else if let Some(&neighbour) = {
            let axis = !direction.axis;
            let center = self.graph.tiles[&self.tile_id].rect.get_center(axis);
            candidates.iter().min_by(|a, b| {
                f32::total_cmp(
                    &f32::abs(self.graph.tiles[a].rect.get_center(axis) - center),
                    &f32::abs(self.graph.tiles[b].rect.get_center(axis) - center),
                )
            })
        } {
            Some(Caret::new(neighbour, !direction))
        } else {
            None
        }
    }

    /// Returns none if there is no neighbour to focus in the given direction
    /// Note: if the result is none, the current focus is unchanged
    fn next(&mut self, direction: Cardinal) -> Option<Caret> {
        /// for now, we try not to assume that the state memory is up to date
        /// we'll have to decide whether or not to deliberately clean it after operations but for now it is relatively safer to do a check
        /// also this draft doesn't currently account for content swapping, which would probably be supported by mapping layer instead of managing more complex graph manipulations
        /// for that we will probably need id type parameterisation for carets, e.g. with a newtype for outer ids, and have outer ids be what is stored in the nav state.
        if let Some(destination) = self.peek(direction) {
            if direction.axis == self.state.history_axis {
                let past = self.history_mut(direction.sign);
                if past
                    .pop_if(|top| top.tile_id == destination.tile_id)
                    .is_none()
                {
                    past.clear();
                }
            } else {
                self.state.negative_history.clear();
                self.state.positive_history.clear();
                self.state.history_axis = direction.axis;
                let current = self.state.inner?;
                self.history_mut(!direction.sign).push(current);
            }
            self.state.inner = Some(destination);
            Some(destination)
        } else {
            None
        }
    }

    /// This allows arbitrary updates to the focus.
    /// Always clears the history
    fn set_caret(&mut self, destination: Caret) -> &mut Self {
        self.state.negative_history.clear();
        self.state.positive_history.clear();
        self.state.inner = Some(destination);
        self
    }

    fn go_to(&mut self, tile_id: Uuid) -> &mut Self {
        self.set_caret(Caret::new(tile_id, self.side))
    }

    fn face(&mut self, side: Cardinal) -> &mut Self {
        self.state.inner.as_mut().unwrap().side = side;
        self
    }
}
