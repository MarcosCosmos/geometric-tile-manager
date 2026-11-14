use crate::core::geometry::Sign::{Negative, Positive};
use crate::core::geometry::{AxisBounds, Cardinal, Sign};
use crate::core::graph::{Graph, NodeId};
use std::iter::{Chain, Map, Once};
use std::ops::{Deref, DerefMut, Index, IndexMut, Not};

/// A caret is a position on the side of a node.
/// they provide the step function for navigating along the gutter of carets parallel to them.
#[derive(Copy, Clone, Debug)]
pub struct Caret {
    pub node_id: NodeId,
    pub side: Cardinal,
}

impl Caret {
    pub fn new(node_id: NodeId, side: Cardinal) -> Self {
        Self {
            node_id: node_id,
            side: side,
        }
    }
}

#[derive(Copy, Clone, Debug)]
// As in, a line between two parallel Carets.
// This line may or may not be fully "connected" (without cutting through the interior of any tiles)
pub struct Line {
    pub positive_id: NodeId,
    pub negative_id: NodeId,
    pub side: Cardinal,
}

impl Line {
    pub fn new(start_id: NodeId, end_id: NodeId, side: Cardinal) -> Self {
        Self {
            positive_id: start_id,
            negative_id: end_id,
            side,
        }
    }
    pub fn caret(&self, sign: Sign) -> Caret {
        Caret::new(self.positive_id, self.side)
    }

    pub fn is_tile(&self) -> bool {
        self.negative_id.is_tile() && self.positive_id.is_tile()
    }
}

impl Index<Sign> for Line {
    type Output = NodeId;

    fn index(&self, index: Sign) -> &Self::Output {
        match index {
            Negative => &self.negative_id,
            Positive => &self.positive_id,
        }
    }
}

impl IndexMut<Sign> for Line {
    fn index_mut(&mut self, index: Sign) -> &mut Self::Output {
        match index {
            Negative => &mut self.negative_id,
            Positive => &mut self.positive_id,
        }
    }
}

pub trait Navigation: Deref<Target: Sized> + Sized {
    type Direction: Copy + Not<Output = Self::Direction>;
    fn location(&self) -> i32;

    fn peek(&self, direction: Self::Direction) -> Option<Self::Target>;

    fn next(&mut self, direction: Self::Direction) -> Option<Self::Target>;

    fn set_target(&mut self, target: Self::Target);

    fn iter(self, direction: Self::Direction) -> NavIter<Self>
    where
        Self: Copy,
    {
        NavIter::new(self, direction)
    }

    fn bounds(&self) -> AxisBounds;
}

pub trait CaretNavigation: Navigation + Deref<Target = Caret> {
    fn set_node(&mut self, node_id: NodeId);

    fn set_side(&mut self, side: Cardinal);

    fn with_id(mut self, node_id: NodeId) -> Self {
        self.set_node(node_id);
        self
    }

    fn with_side(mut self, side: Cardinal) -> Self {
        self.set_side(side);
        self
    }
}

pub trait LineNavigation: Navigation + Deref<Target = Line> {
    /// note: a and b must be supplied in order along the cross axis of the given side or this fails
    fn is_connected(&self) -> bool;
    fn set_side(&mut self, side: Cardinal);
    fn set_nodes(&mut self, negative_id: NodeId, positive_id: NodeId);

    fn with_side(mut self, side: Cardinal) -> Self {
        self.set_side(side);
        self
    }

    fn with_nodes(mut self, negative_id: NodeId, positive_id: NodeId) -> Self {
        self.set_nodes(negative_id, positive_id);
        self
    }
}

/// A transparent wrapper struct that facilitates navigation functionally for types, like Caret or Line, that have navigation related functionality but shouldn't be stored with a persistent reference to the graph.
/// For each type where this is rational, the Navigation trait is implemented on the specialisation of Nav<'a, T>.
#[derive(Copy, Clone, Debug)]
pub struct Nav<'a, T> {
    target: T,
    graph: &'a Graph,
}

impl<'a, T> Nav<'a, T> {
    pub fn new(graph: &'a Graph, target: T) -> Nav<T>
    where
        Self: Navigation,
    {
        Self { target, graph }
    }
}

impl<T> Deref for Nav<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.target
    }
}

impl<T> DerefMut for Nav<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.target
    }
}

impl Navigation for Nav<'_, Caret> {
    type Direction = Sign;

    fn location(&self) -> i32 {
        self.graph.bounds[&self.node_id].location(self.side)
    }

    fn peek(&self, direction: Sign) -> Option<Caret> {
        let location = self.location();
        let search_direction = Cardinal {
            axis: !self.side.axis,
            sign: direction,
        };
        self.graph.intra_surface_neighbourhoods[&self.node_id][search_direction]
            .iter()
            .filter(|id| id.is_tile())
            .map(|&id| Caret::new(id, self.side))
            .find(|&c| Nav::new(self.graph, c).location() == location)
    }

    fn next(&mut self, direction: Sign) -> Option<Caret> {
        let result = self.peek(direction);
        if let Some(position) = result {
            self.set_target(position);
            Some(position)
        } else {
            None
        }
    }

    fn set_target(&mut self, target: Self::Target) {
        self.target = target;
    }

    fn bounds(&self) -> AxisBounds {
        self.graph.bounds[&self.node_id][!self.side.axis]
    }
}

impl CaretNavigation for Nav<'_, Caret> {
    fn set_node(&mut self, node_id: NodeId) {
        self.node_id = node_id;
    }

    fn set_side(&mut self, side: Cardinal) {
        self.side = side;
    }
}

impl LineNavigation for Nav<'_, Line> {
    fn is_connected(&self) -> bool {
        Nav::new(self.graph, self.caret(Negative))
            .iter(Positive)
            .inclusive()
            .any(|n| n.node_id == self.negative_id)
    }

    fn set_side(&mut self, side: Cardinal) {
        self.side = side;
    }

    fn set_nodes(&mut self, negative_id: NodeId, positive_id: NodeId) {
        self.negative_id = negative_id;
        self.positive_id = positive_id;
    }
}

impl Navigation for Nav<'_, Line> {
    type Direction = Sign;

    fn location(&self) -> i32 {
        let negative_location =
            Nav::new(self.graph, Caret::new(self.negative_id, self.side)).location();
        let end_location = Nav::new(self.graph, Caret::new(self.positive_id, self.side)).location();
        match self.side.sign {
            Positive => i32::min(negative_location, end_location),
            Negative => i32::max(negative_location, end_location),
        }
    }

    fn peek(&self, direction: Self::Direction) -> Option<Line> {
        let target_side = Cardinal::new(self.side.axis, direction);
        let mut candidate = *self.with_side(target_side);
        if self.is_tile()
            && self.side.sign != direction
            && Nav::new(self.graph, candidate).is_connected()
        {
            Some(candidate)
        } else {
            let (cross_neg, cross_pos) = self.side.perpendicular_directions();
            let mut current_neg = Nav::new(self.graph, Caret::new(self.negative_id, cross_neg));
            let mut current_pos = Nav::new(self.graph, Caret::new(self.positive_id, cross_pos));
            let mut neg_iter = current_neg.iter(direction);
            let mut pos_iter = current_pos.iter(direction);
            if let (Some(next_neg), Some(next_pos)) = (neg_iter.next(), pos_iter.next()) {
                if self.is_tile() && self.side.sign == direction && self.is_connected() {
                    let candidate = Line::new(next_neg.node_id, next_pos.node_id, !self.side);
                    assert!(Nav::new(self.graph, candidate).is_connected());
                    Some(candidate)
                } else {
                    current_neg = next_neg;
                    current_pos = next_neg;
                    loop {
                        let mut neg_location = current_neg.with_side(target_side).location();
                        let mut pos_location = current_pos.with_side(target_side).location();
                        while neg_location != pos_location {
                            if neg_location < pos_location {
                                if let Some(next_neg) = neg_iter.next() {
                                    current_neg = next_neg;
                                    neg_location = current_neg.with_side(target_side).location();
                                } else {
                                    continue;
                                }
                            } else if neg_location > pos_location {
                                if let Some(next_pos) = pos_iter.next() {
                                    current_pos = next_pos;
                                    pos_location = current_neg.with_side(target_side).location();
                                } else {
                                    continue;
                                }
                            }
                        }

                        candidate = Line::new(next_neg.node_id, next_pos.node_id, target_side);
                        if !Nav::new(self.graph, candidate).is_connected()
                            && let (Some(next_neg), Some(next_pos)) =
                                (neg_iter.next(), pos_iter.next())
                        {
                            current_neg = next_neg;
                            current_pos = next_pos;
                        } else {
                            break Some(candidate);
                        }
                    }
                }
            } else {
                None
            }
        }
    }

    fn next(&mut self, direction: Self::Direction) -> Option<Line> {
        let result = self.peek(direction);
        if let Some(position) = result {
            self.target = position;
            Some(position)
        } else {
            None
        }
    }

    fn set_target(&mut self, target: Self::Target) {
        self.target = target;
    }

    fn bounds(&self) -> AxisBounds {
        Nav::new(self.graph, self.caret(Negative))
            .bounds()
            .span(Nav::new(self.graph, self.caret(Positive)).bounds())
    }
}

pub trait CaretIterExt
where
    Self: Iterator<Item: Deref<Target = Caret>>,
{
    fn ids(self) -> Map<Self, fn(Self::Item) -> NodeId>
    where
        Self: Sized,
    {
        self.map(|c| c.node_id)
    }
}

impl<T: ?Sized> CaretIterExt for T where T: Iterator<Item: Deref<Target = Caret>> {}

#[derive(Copy, Clone, Debug)]
pub struct NavIter<T>
where
    T: Navigation + Copy,
    T::Target: Sized,
{
    current: T,
    direction: T::Direction,
}

impl<T> NavIter<T>
where
    T: Navigation + Copy,
{
    pub fn new(navigator: T, direction: T::Direction) -> Self {
        Self {
            current: navigator,
            direction,
        }
    }
    pub fn inclusive(self) -> Chain<Once<T>, Self> {
        std::iter::once(self.current).chain(self)
    }
    pub fn include_if<P>(self, predicate: P) -> impl Iterator<Item = T>
    where
        P: FnMut(&T) -> bool,
    {
        std::iter::once(self.current).filter(predicate).chain(self)
    }
}

impl<T> Iterator for NavIter<T>
where
    T: Navigation + Copy,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.next(self.direction).is_some() {
            Some(self.current)
        } else {
            None
        }
    }
}
