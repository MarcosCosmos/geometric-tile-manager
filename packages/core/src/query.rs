use crate::area::{Area, AreaQuery};
use crate::caret::{Caret, CaretQuery};
use crate::error::ValidationError;
use crate::geometry::Sign::{Negative, Positive};
use crate::geometry::{AreaBounds, LineBounds};
use crate::graph::{Graph, NodeId};
use crate::key::QueryKey;
use crate::line::{Line, LineError, LineQuery};
use crate::neighbours::{NeighbourSurfaces, NeighbourTiles, NeighbourhoodQuery, Neighbours};
use crate::node::NodeQuery;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, DerefMut, Not};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ElementKind {
    Tile,
    Surface,
    Mixed,
}

impl Display for ElementKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

pub trait Standardisation: Sized {
    type Error;
    fn to_normalized(self) -> Self {
        self
    }

    fn to_validated(self) -> Result<Self, Self::Error> {
        Ok(self)
    }
}

// todo: add serialisation requirement to output the results once other testing has started.
pub trait GraphQuery: Deref<Target: QueryKey> + DerefMut + Sized + Copy {
    type Query<Q>: GraphQuery
    where
        Q: QueryKey;
    fn to<Q>(self, key: Q) -> Self::Query<Q>
    where
        Q: QueryKey;
}

pub trait Information: Deref<Target: QueryKey> + Sized {}

pub trait ElementInformation: Information {
    fn kind(&self) -> ElementKind;
}

pub trait ElementInformation1D: ElementInformation {
    fn bounds(&self) -> LineBounds;
}

pub trait ElementInformation2D: ElementInformation {
    fn bounds(&self) -> AreaBounds;
}

pub trait GroupInformation: ElementInformation {
    fn is_connected(&self) -> bool;
}

pub trait GroupQuery: GraphQuery + GroupInformation + TryInto<Self::Query<NodeId>> {}

pub trait Navigation: Deref<Target: QueryKey> + Sized {
    type Direction: Copy + Not<Output = Self::Direction>;

    fn peek(&self, direction: Self::Direction) -> Option<Self::Target>;
}

pub trait NavigationMut: Navigation {
    fn next(&mut self, direction: Self::Direction) -> Option<&mut Self>;
    fn set_target(&mut self, target: Self::Target);
}

/// A transparent wrapper struct containing a key and a context that facilitates a (pure) functional query language for describing the graph.
/// There are 4 main motivations for this architecture:
/// 1. It lends itself to both serialization and borrow-checker-friendly code, since the graph `&` can be dropped freely.
/// 2. It facilitates an implementation validation and dry-runs of commands that is fully unified with the actual manipulation logic.
/// 3. It insulates the notional representation of the Graph from the implementation details of how it is stored.
/// 2. It minimizes the core logic changes required for incremental feature build-out, as many features can be described in terms of a functional composition of more instrinsic/canonical features.
///
/// The general idea is to key type `K: QueryKey`  is a POD struct with no natural member methods (they may implement traits like from/into/deref for coercive purposes, etc, as long as they don't conflict with Query<K>).
/// The actual details related to the keyed element are instead implemented on the Query wrapper:
/// - Each key `K` should correspond to some serializable struct (`XInformation`) which describes the data about that element.
/// - Implement, for `Query<K>`, an `XProvider` trait which constructs each of the `XInformation` fields on the fly, via a method of the form `field_name(&self)`.
/// - Optionally implement, for `Query<K>`, some `XQuery` trait providing utility methods for arriving at some related query: `into_y(self, args) -> YQuery`. Collectively, these XQuery traits are induce a domain-specific query language for navigating the graph.
///   - By convention, From/Into should be implemented instead of into_y when there are 0 args, since y is generally always going to relate to some concrete key type.
///   - Note: in principle, `Query<K>` should also mirror all `(Try)From/Into` traits of K that exist, mainly for convenience. A query_impl macro is provided for this, but it is not a procedural macro (yet).
/// - `GraphQuery` implements Deref<K> and DerefMut, rather than exposing the key directly, so that it acts more like a smart `K` and there is less
#[derive(Copy, Clone, Debug)]
pub struct Query<'a, K>
where
    K: QueryKey,
{
    pub context: &'a Graph,
    key: K,
}

impl<'a, K> Query<'a, K>
where
    K: QueryKey,
{
    pub fn new(context: &'a Graph, key: K) -> Self {
        Self { context, key }
    }
}

impl<K> Deref for Query<'_, K>
where
    K: QueryKey,
{
    type Target = K;

    fn deref(&self) -> &Self::Target {
        &self.key
    }
}

impl<K> DerefMut for Query<'_, K>
where
    K: QueryKey,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.key
    }
}

impl<'a, K> GraphQuery for Query<'a, K>
where
    K: QueryKey,
{
    type Query<Q>
        = Query<'a, Q>
    where
        Q: QueryKey;

    fn to<Q>(self, key: Q) -> Self::Query<Q>
    where
        Q: QueryKey,
    {
        Query::new(self.context, key)
    }
}
impl<'a, K> Information for Query<'a, K> where K: QueryKey {}

impl<'a, K> GroupQuery for Query<'a, K>
where
    Self: GroupInformation,
    K: QueryKey,
    Query<'a, NodeId>: From<Query<'a, K>>,
{
}

impl<K> PartialEq<Self> for Query<'_, K>
where
    K: QueryKey,
{
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<K> Eq for Query<'_, K> where K: QueryKey {}

impl<K> PartialOrd<Self> for Query<'_, K>
where
    K: QueryKey,
    Self: Ord,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
