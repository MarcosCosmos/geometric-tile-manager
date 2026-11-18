use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{Axis, Cardinal, LineBounds, Sign};
use crate::graph::{Graph, NodeId};
use crate::query::caret::{Caret, CaretInformationMut};
use crate::query::neighbours::{
    NeighbourSurfaces, NeighbourTiles, NeighbourhoodInformation, Neighbours,
};
use crate::query::source::{QuerySource, RawQuerySource};
use crate::query::{
    ElementInformation, ElementInformation1D, ElementInformationMut, ElementKind,
    ElementNavigation, ElementNavigationMut, Query,
};
use std::ops::Deref;
use uuid::Uuid;

pub struct FocusState {
    target: Option<Caret>,
    negative_history: Vec<Caret>,
    positive_history: Vec<Caret>,
    history_axis: Axis,
}

impl FocusState {
    pub fn new() -> Self {
        Self {
            target: None,
            negative_history: Vec::new(),
            positive_history: Vec::new(),
            history_axis: Axis::Horizontal,
        }
    }

    pub fn current(&self) -> Option<Caret> {
        self.target
    }

    /// checks if the current caret still exists in the graph, and sets it to none if so
    pub fn sanitize(&mut self, graph: &Graph) {
        self.target
            .take_if(|c| graph.try_query(Into::<Uuid>::into(c.id)).is_err());
    }

    /// returns none if there is no current focus
    pub fn try_navigate<'b>(&mut self, graph: &'b Graph) -> Option<FocusContext<'_, 'b>> {
        self.sanitize(graph);
        if self.target.is_some() {
            Some(FocusContext { state: self, graph })
        } else {
            None
        }
    }

    pub fn navigate<'b>(&mut self, graph: &'b Graph, default: Caret) -> FocusContext<'_, 'b> {
        self.sanitize(graph);
        self.target.get_or_insert(default);
        FocusContext { state: self, graph }
    }
}

pub struct FocusContext<'a, 'b> {
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
impl<'a, 'b> FocusContext<'a, 'b> {
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

    fn nearest_by_centers(&self, candidates: &[NodeId], axis: Axis) -> Option<NodeId> {
        let center = self.graph.bounds[&self.id][axis].center();
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

impl Deref for FocusContext<'_, '_> {
    type Target = Caret;

    fn deref(&self) -> &Self::Target {
        self.state.target.as_ref().unwrap()
    }
}

impl ElementInformation for FocusContext<'_, '_> {
    fn kind(&self) -> ElementKind {
        todo!()
    }
}

impl ElementInformationMut for FocusContext<'_, '_> {
    /// This allows arbitrary updates to the focus.
    /// Always clears the history
    fn set_target(&mut self, target: Caret) {
        self.state.negative_history.clear();
        self.state.positive_history.clear();
        self.state.target = Some(target);
    }
}

impl ElementInformation1D for FocusContext<'_, '_> {
    fn bounds(&self) -> LineBounds {
        self.graph.query(*self.deref()).bounds()
    }
}

impl<'a, 'b> ElementNavigation for FocusContext<'_, '_> {
    type Direction = Cardinal;
    fn peek(&self, direction: Cardinal) -> Option<Caret> {
        let current = **self;
        let candidates = {
            let current_node = Query::<NodeId>::from(self.graph.query(current.id));
            let mut prelims: Query<Neighbours> = match current_node.kind() {
                ElementKind::Surface => self.graph.query(NeighbourSurfaces(current).into()),
                ElementKind::Tile => {
                    let immediate = self.graph.query(NeighbourTiles(current));
                    let items = immediate.items();
                    if items.len() == 1 && self.graph.query(items[0]).kind() == ElementKind::Surface
                    {
                        self.graph.query(NeighbourSurfaces(current).into())
                    } else {
                        immediate.into()
                    }
                }
                _ => panic!(),
            };
            let targets = match *prelims {
                Neighbours::Surfaces(_)
                    if let Some(next_surface_id) =
                        self.nearest_by_centers(prelims.items(), direction.axis) =>
                {
                    let distant = self
                        .graph
                        .query(NeighbourTiles(Caret::new(next_surface_id, direction)));
                    if distant
                        .items()
                        .iter()
                        .any(|&id| self.graph.query(id).kind() == ElementKind::Tile)
                    {
                        distant.into()
                    } else {
                        prelims
                    }
                }
                _ => prelims,
            };
            targets.items()
        };

        if self.state.history_axis == direction.axis
            && let Some(&previous) = self
                .history(direction.sign)
                .last()
                .filter(|c| candidates.contains(&c.id))
        {
            Some(previous)
        } else if direction.axis != self.side.axis
            && let Some(caret) = self.graph.query(*self.deref()).peek(direction.sign)
        {
            Some(caret)
        } else if let Some(neighbour) = self.nearest_by_centers(candidates, direction.axis) {
            Some(Caret::new(neighbour, !direction))
        } else {
            None
        }
    }
}

impl CaretInformationMut for FocusContext<'_, '_> {
    fn set_node(&mut self, node_id: NodeId) {
        self.state.target.as_mut().unwrap().id = node_id;
    }

    fn set_side(&mut self, side: Cardinal) {
        self.state.target.as_mut().unwrap().side = side;
    }
}

impl ElementNavigationMut for FocusContext<'_, '_> {
    /// Returns none if there is no neighbour to focus in the given direction
    /// Note: if the result is none, the current focus is unchanged
    fn next(&mut self, direction: Cardinal) -> Option<&mut Self> {
        /// for now, we try not to assume that the state memory is up to date
        /// we'll have to decide whether or not to deliberately clean it after operations but for now it is relatively safer to do a check
        /// also this draft doesn't currently account for content swapping, which would probably be supported by mapping layer instead of managing more complex graph manipulations
        /// for that we will probably need id type parameterisation for carets, e.g. with a newtype for outer ids, and have outer ids be what is stored in the nav state.
        if let Some(destination) = self.peek(direction) {
            if direction.axis == self.state.history_axis {
                let past = self.history_mut(direction.sign);
                if past.pop_if(|top| top.id == destination.id).is_none() {
                    past.clear();
                }
            } else {
                self.state.negative_history.clear();
                self.state.positive_history.clear();
                self.state.history_axis = direction.axis;
                let current = self.state.target?;
                self.history_mut(!direction.sign).push(current);
            }
            self.state.target = Some(destination);
            Some(self)
        } else {
            None
        }
    }
}
