use geowm_core::context::QueryContext;
use geowm_core::caret::{Caret, CaretNavigationMut};
use geowm_core::geometry::{Axis, Cardinal, Sign, Sign::{*}};
use geowm_core::graph::{Graph, NodeId};
use geowm_core::neighbours::{NeighbourSurfaces, NeighbourTiles, Neighbours};
use geowm_core::query::{ElementInformation, ElementInformation1D, ElementInformation2D, ElementKind, Navigation, NavigationMut, Query};
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
    pub fn try_navigate<'b>(&mut self, graph: &'b Graph) -> Option<FocusNavigator<'_, 'b>> {
        self.sanitize(graph);
        if self.target.is_some() {
            Some(FocusNavigator { state: self, graph })
        } else {
            None
        }
    }

    pub fn navigate<'b>(&mut self, graph: &'b Graph, default: Caret) -> FocusNavigator<'_, 'b> {
        self.sanitize(graph);
        self.target.get_or_insert(default);
        FocusNavigator { state: self, graph }
    }
}

pub struct FocusNavigator<'a, 'b> {
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
impl<'a, 'b> FocusNavigator<'a, 'b> {
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

    fn nearest_by_centers<T>(&self, candidates: T, axis: Axis) -> Option<NodeId>
    where
        T: Iterator<Item = Query<'a, NodeId>>,
    {
        let center = Query::new(self.graph, **self).bounds().axis_bounds.center();
        Some(
            *candidates
                .min_by(|a, b| {
                    f32::total_cmp(
                        &f32::abs(a.bounds()[axis].center() - center),
                        &f32::abs(b.bounds()[axis].center() - center),
                    )
                })?,
        )
    }
}

impl Deref for FocusNavigator<'_, '_> {
    type Target = Caret;

    fn deref(&self) -> &Self::Target {
        self.state.target.as_ref().unwrap()
    }
}

impl<'a, 'b> Navigation for FocusNavigator<'_, '_> {
    type Direction = Cardinal;
    fn peek(&self, direction: Cardinal) -> Option<Caret> {
        let current = Query::new(self.graph, **self);
        let candidates = {
            let mut prelims: Query<Neighbours> = match current.kind() {
                ElementKind::Surface => Query::<NeighbourSurfaces>::from(current).into(),
                ElementKind::Tile => {
                    let immediate = Query::<NeighbourTiles>::from(current);
                    if let Some(node) = immediate.into_iter().next()
                        && node.kind() == ElementKind::Surface
                    {
                        Query::<NeighbourSurfaces>::from(current).into()
                    } else {
                        immediate.into()
                    }
                }
                _ => panic!(),
            };
            match *prelims {
                Neighbours::Surfaces(_)
                    if let Some(next_surface_id) =
                        self.nearest_by_centers(prelims.into_iter(), direction.axis) =>
                {
                    let distant = Query::new(self
                        .graph, NeighbourTiles(Caret::new(next_surface_id, direction)));
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

        if self.state.history_axis == direction.axis
            && let Some(&previous) = self
                .history(direction.sign)
                .last()
                .filter(|c| candidates.into_iter().any(|id| *id == c.id))
        {
            Some(previous)
        } else if direction.axis != self.side.axis
            && let Some(caret) = Query::new(self.graph, *self.deref()).peek(direction.sign)
        {
            Some(caret)
        } else if let Some(neighbour) =
            self.nearest_by_centers(candidates.into_iter(), direction.axis)
        {
            Some(Caret::new(neighbour, !direction))
        } else {
            None
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
        /// for now, we try not to assume that the state memory is up to date
        /// we'll have to decide whether or not to deliberately clean it after operations but for now it is relatively safer to do a check
        /// also this draft doesn't currently account for content swapping, which would probably be supported by mapping layer instead of managing more complex graph manipulations
        /// for that we will probably need id type parameterisation for carets, e.g. with a newtype for outer ids, and have outer ids be what is stored in the nav state.
        let destination = self.peek(direction)?;
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
    }

    /// This allows arbitrary updates to the focus.
    /// Always clears the history
    fn set_target(&mut self, target: Caret) {
        self.state.negative_history.clear();
        self.state.positive_history.clear();
        self.state.target = Some(target);
    }
}
