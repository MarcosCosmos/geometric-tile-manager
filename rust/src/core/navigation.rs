use std::fmt::Debug;
use crate::core::geometry::Axis::{Horizontal, Vertical};
use crate::core::geometry::Sign::{Negative, Positive};
use crate::core::geometry::{Axis, Bounds, Cardinal, EAST, NORTH, RectBounds, SOUTH, Sign, WEST};
use crate::core::graph::{Graph, NodeId};
use crate::core::transform::{SegmentedContainer, find_segments};
use crate::result::Error;
use std::iter::{Chain, Map, Once};
use std::ops::{Deref, DerefMut, Index, IndexMut, Not};

/// A caret is a position on the side of a node.
/// they provide the step function for navigating along the gutter of carets parallel to them.
#[derive(Copy, Clone, Debug)]
pub struct Caret<Id=NodeId> where Id : Copy + Clone + Debug {
    pub id: Id,
    pub side: Cardinal,
}

impl <Id> Caret<Id> where Id : Copy + Clone + Debug {
    pub fn new(id: Id, side: Cardinal) -> Self {
        Self { id, side }
    }
}

#[derive(Copy, Clone, Debug)]
// As in, a line between two parallel Carets.
// This line may or may not be fully "connected" (without cutting through the interior of any tiles)
pub struct Line {
    negative_id: NodeId,
    positive_id: NodeId,
    pub side: Cardinal,
}

impl Line {
    pub fn new(negative_id: NodeId, positive_id: NodeId, side: Cardinal) -> Self {
        Self {
            negative_id,
            positive_id,
            side,
        }
    }
    pub fn caret(&self, sign: Sign) -> Caret {
        Caret::new(self[sign], self.side)
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

/// Generally dividers will be clean lines that don't intersect any tiles,
/// but the start or end may or may not result in cutting through some tiles
/// Note: it is possible for a transform container to start and end with the same pair of ids and have no dividers.
///     in this case it is a single row/column or even single tile.
#[derive(Copy, Clone, Debug)]
pub struct Area {
    pub north_west: NodeId,
    pub north_east: NodeId,
    pub south_west: NodeId,
    pub south_east: NodeId,
}

impl From<NodeId> for Area {
    fn from(value: NodeId) -> Self {
        Area {
            north_west: value,
            north_east: value,
            south_west: value,
            south_east: value,
        }
    }
}

impl Area {
    pub fn into_single_tile(self) -> Option<NodeId> {
        if self.north_west == self.north_east
            && self.north_west == self.south_east
            && self.north_west == self.south_west
        {
            Some(self.north_west)
        } else {
            None
        }
    }

    pub fn side(&self, side: Cardinal) -> Line {
        match side {
            Cardinal {
                axis: Horizontal,
                sign: Negative,
            } => Line::new(self.north_west, self.south_west, side),
            Cardinal {
                axis: Horizontal,
                sign: Positive,
            } => Line::new(self.north_east, self.south_east, side),
            Cardinal {
                axis: Vertical,
                sign: Negative,
            } => Line::new(self.north_east, self.north_west, side),
            Cardinal {
                axis: Vertical,
                sign: Positive,
            } => Line::new(self.south_east, self.south_west, side),
        }
    }
}

pub trait ElementInformation: Deref<Target: Sized> + Sized {}

pub trait ElementInformationMut: ElementInformation {
    fn set_target(&mut self, target: Self::Target);
}

pub trait ElementInformation1D: ElementInformation {
    fn location(&self) -> i32;
    fn bounds(&self) -> Bounds;
}

pub trait AreaInformation {
    fn bounds(&self) -> RectBounds;
    fn is_connected(&self) -> bool;
}

pub trait ElementNavigation: ElementInformationMut {
    type Direction: Copy + Not<Output = Self::Direction>;

    fn peek(&self, direction: Self::Direction) -> Option<Self::Target>;

    fn next(&mut self, direction: Self::Direction) -> Option<Self::Target>;

    fn iter(self, direction: Self::Direction) -> NavIter<Self>
    where
        Self: Copy,
    {
        NavIter::new(self, direction)
    }
}

pub trait CaretInformationMut: ElementInformationMut + Deref<Target = Caret> {
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

pub trait LineInformation: ElementInformation + Deref<Target = Line> {
    /// note: a and b must be supplied in order along the cross axis of the given side or this fails
    fn is_connected(&self) -> bool;
}

pub trait LineInformationMut: ElementInformationMut + Deref<Target = Line> {
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
pub struct Context<C, T> {
    pub(super) context: C,
    target: T,
}

impl <C, T> Context<C, T> {
    pub(super) fn new(context: C, target: T) -> Self {
        Self { context, target }
    }
}

impl<C, T> Deref for Context<C, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.target
    }
}

impl<C, T> DerefMut for Context<C, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.target
    }
}

impl ElementInformation for Context<&Graph,Caret> {}
impl<T> ElementInformationMut for Context<&Graph,T>
where
    Self: ElementInformation + Deref<Target = T>,
{
    fn set_target(&mut self, target: Self::Target) {
        self.target = target;
    }
}

impl ElementInformation1D for Context<&Graph, Caret> {
    fn location(&self) -> i32 {
        self.context.bounds[&self.id][self.side]
    }

    fn bounds(&self) -> Bounds {
        self.context.bounds[&self.id][!self.side.axis]
    }
}

impl CaretInformationMut for Context<&Graph, Caret> {
    fn set_node(&mut self, node_id: NodeId) {
        self.id = node_id;
    }

    fn set_side(&mut self, side: Cardinal) {
        self.side = side;
    }
}

impl ElementNavigation for Context<&Graph, Caret> {
    type Direction = Sign;

    fn peek(&self, direction: Sign) -> Option<Caret> {
        let location = self.location();
        let search_direction = Cardinal {
            axis: !self.side.axis,
            sign: direction,
        };
        self.context.get_tile_neighbourhood(Caret::new(self.id, search_direction))
            .iter()
            .filter(|id| id.is_tile())
            .map(|&id| Caret::new(id, self.side))
            .find(|&c| self.context.info(c).location() == location)
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
}

impl ElementInformation for Context<&Graph, Line> {}

impl ElementInformation1D for Context<&Graph, Line> {
    fn location(&self) -> i32 {
        let negative_location = self.context.info(self.caret(Negative)).location();
        let end_location = self.context.info(self.caret(Positive)).location();
        match self.side.sign {
            Positive => i32::min(negative_location, end_location),
            Negative => i32::max(negative_location, end_location),
        }
    }

    fn bounds(&self) -> Bounds {
        self.context
            .info(self.caret(Negative))
            .bounds()
            .span(&self.context.info(self.caret(Positive)).bounds())
    }
}

impl LineInformation for Context<&Graph, Line> {
    fn is_connected(&self) -> bool {
        self.context
            .info(self.caret(Negative))
            .iter(Positive)
            .inclusive()
            .any(|n| n.id == self.negative_id)
    }
}

impl LineInformationMut for Context<&Graph, Line> {
    fn set_side(&mut self, side: Cardinal) {
        self.side = side;
    }

    fn set_nodes(&mut self, negative_id: NodeId, positive_id: NodeId) {
        self.negative_id = negative_id;
        self.positive_id = positive_id;
    }
}

impl ElementNavigation for Context<&Graph, Line> {
    type Direction = Sign;

    fn peek(&self, direction: Self::Direction) -> Option<Line> {
        let target_side = Cardinal::new(self.side.axis, direction);
        let mut candidate = *self.with_side(target_side);
        if self.is_tile()
            && self.side.sign != direction
            && self.context.info(candidate).is_connected()
        {
            Some(candidate)
        } else {
            let (cross_neg, cross_pos) = self.side.perpendicular_directions();
            let mut current_neg = self.context.info(Caret::new(self.negative_id, cross_neg));
            let mut current_pos = self.context.info(Caret::new(self.positive_id, cross_pos));
            let mut neg_iter = current_neg.iter(direction);
            let mut pos_iter = current_pos.iter(direction);
            if let (Some(next_neg), Some(next_pos)) = (neg_iter.next(), pos_iter.next()) {
                if self.is_tile() && self.side.sign == direction && self.is_connected() {
                    let candidate = Line::new(next_neg.id, next_pos.id, !self.side);
                    assert!(self.context.info(candidate).is_connected());
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
                                    pos_location = current_pos.with_side(target_side).location();
                                } else {
                                    continue;
                                }
                            }
                        }

                        candidate =
                            Line::new(current_neg.id, current_pos.id, target_side);
                        if !self.context.info(candidate).is_connected()
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
}

impl ElementInformation for Context<&Graph, Area> {}

impl AreaInformation for Context<&Graph, Area> {
    fn bounds(&self) -> RectBounds {
        RectBounds {
            horizontal: Bounds::new(
                self.context.info(self.side(WEST)).location(),
                self.context.info(self.side(EAST)).location(),
            ),
            vertical: Bounds::new(
                self.context.info(self.side(NORTH)).location(),
                self.context.info(self.side(SOUTH)).location(),
            ),
        }
    }

    fn is_connected(&self) -> bool {
        self.context.info(self.side(NORTH)).is_connected()
            && self.context.info(self.side(EAST)).is_connected()
            && self.context.info(self.side(SOUTH)).is_connected()
            && self.context.info(self.side(WEST)).is_connected()
    }
}

impl Context<&Graph, Area> {
    pub fn segments(&self, axis: Axis) -> impl Iterator<Item = Area> {
        match axis {
            Horizontal => find_segments(
                self.context,
                (self.north_west, self.south_west),
                Some((self.south_east, self.south_west)),
                Cardinal {
                    axis: Horizontal,
                    sign: Positive,
                },
            ),
            Vertical => find_segments(
                self.context,
                (self.north_west, self.north_east),
                Some((self.south_west, self.south_east)),
                Cardinal {
                    axis: Vertical,
                    sign: Positive,
                },
            ),
        }
    }

    pub fn into_segmented(self, axis: Axis) -> SegmentedContainer {
        SegmentedContainer::new(self.segments(axis).collect(), axis)
    }
}

pub(super) trait RawInformationSource<T> {
    fn info(&self, target: T) -> Context<&Self, T>;
}

pub trait InformationSource<T>: RawInformationSource<T> {
    fn find(&self, target: T) -> Result<Context<&Self, T>, Error>;
}

impl<T> RawInformationSource<T> for Graph {
    fn info(&self, target: T) -> Context<&Self, T> {
        Context {
            context: self,
            target,
        }
    }
}
impl InformationSource<Caret> for Graph {
    fn find(&self, target: Caret) -> Result<Context<&Self, Caret>, Error> {
        if self.bounds.contains_key(&target.id) {
            Ok(self.info(target))
        } else {
            Err(Error::NodeNotFound(target.id))
        }
    }
}

impl InformationSource<Line> for Graph {
    fn find(&self, target: Line) -> Result<Context<&Self, Line>, Error> {
        if !self.bounds.contains_key(&target[Negative]) {
            Err(Error::NodeNotFound(target[Negative]))
        } else if !self.bounds.contains_key(&target[Positive]) {
            Err(Error::NodeNotFound(target[Positive]))
        } else {
            Ok(self.info(target))
        }
    }
}

impl InformationSource<Area> for Graph {
    fn find(&self, target: Area) -> Result<Context<&Self, Area>, Error> {
        if !self.bounds.contains_key(&target.north_west) {
            Err(Error::NodeNotFound(target.north_west))
        } else if !self.bounds.contains_key(&target.north_west) {
            Err(Error::NodeNotFound(target.north_east))
        } else if !self.bounds.contains_key(&target.south_east) {
            Err(Error::NodeNotFound(target.south_east))
        } else if !self.bounds.contains_key(&target.south_west) {
            Err(Error::NodeNotFound(target.south_east))
        } else {
            Ok(self.info(target))
        }
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
        self.map(|c| c.id)
    }
}

impl<T: ?Sized> CaretIterExt for T where T: Iterator<Item: Deref<Target = Caret>> {}

#[derive(Copy, Clone, Debug)]
pub struct NavIter<T>
where
    T: ElementNavigation + Copy,
    T::Target: Sized,
{
    current: T,
    direction: T::Direction,
}

impl<T> NavIter<T>
where
    T: ElementNavigation + Copy,
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
    T: ElementNavigation + Copy,
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
