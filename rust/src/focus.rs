use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{Axis, Cardinal, Sign};
use crate::graph::{Graph, NodeId};
use crate::gutter::GutterNavigation;
use crate::navigation::{Caret, Navigator};
use std::ops::Deref;
use crate::graph::NodeId::{Surface, Tile};

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
            history_axis: Axis::Horizontal,
        }
    }

    pub fn current(&self) -> Option<Caret> {
        self.inner
    }

    /// checks if the current caret still exists in the graph, and sets it to none if so
    pub fn sanitize(&mut self, graph: &Graph) {
        self.inner
            .take_if(|c| !graph.bounds.contains_key(&c.node_id));
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

/// The navigator does it's best to stick to facilitate navigating tiles in a stable and minimal fashion.
/// When focusing directionally, it will stick to any gutter it is already in if possible for that direction.
/// Otherwise, it will look for the neighbour in the given direction that is closest (via the center point).
/// If that neighbour is the containing surface, it will try to move to the next surface over. If that succeeds, it will try to check for a similarly nearest tile inside that surface.
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

    fn candidate_neighbours(&self, direction: Cardinal) -> &Vec<NodeId> {
        let current = self.state.inner.unwrap();
        let candidates = &self.graph.intra_surface_neighbourhoods[&current.node_id][direction];
        if candidates.len() == 1 && let Surface(home_id) = candidates[0] {
            let neighbour_surfaces = &self.graph.inter_surface_neighbourhoods[&home_id][direction];
            if let Some(next_surface_id) =
                self.nearest_by_centers(neighbour_surfaces, direction.axis)
            {
                let distant_candidates =
                    &self.graph.intra_surface_neighbourhoods[&next_surface_id][direction];
                if distant_candidates
                    .iter()
                    .any(|id| id.is_tile())
                {
                    return &neighbour_surfaces;
                }
            }
        }

        candidates
    }

    fn nearest_by_centers(&self, candidates: &Vec<NodeId>, axis: Axis) -> Option<NodeId> {
        let center = self.graph.bounds[&self.node_id][axis].center();
        candidates
            .iter()
            .min_by(|a, b| {
                f32::total_cmp(
                    &f32::abs(self.graph.bounds[a][axis].center() - center),
                    &f32::abs(self.graph.bounds[b][axis].center() - center),
                )
            })
            .copied()
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
        self.get_location(self.graph)
    }

    fn peek(&self, direction: Cardinal) -> Option<Caret> {
        let candidates = self.candidate_neighbours(direction);

        if self.state.history_axis == direction.axis
            && let Some(&previous) = self
                .history(direction.sign)
                .last()
                .filter(|c| candidates.contains(&c.node_id))
        {
            Some(previous)
        } else if direction.axis != self.side.axis
            && let Some(caret) = self.navigate_gutter(self.graph).peek(direction.sign)
        {
            Some(caret)
        } else if let Some(neighbour) = self.nearest_by_centers(candidates, direction.axis) {
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
                    .pop_if(|top| top.node_id == destination.node_id)
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

    fn go_to(&mut self, node_id: NodeId) -> &mut Self {
        self.set_caret(Caret::new(node_id, self.side))
    }

    fn face(&mut self, side: Cardinal) -> &mut Self {
        self.state.inner.as_mut().unwrap().side = side;
        self
    }
}
