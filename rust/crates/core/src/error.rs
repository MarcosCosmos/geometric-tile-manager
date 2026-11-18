use crate::graph::NodeId;
use std::fmt;
use std::fmt::{Debug, Formatter, write};
use uuid::Uuid;

#[derive(Debug)]
pub enum Error<N>
where
    N: Debug,
{
    NodeNotFound(N),
    NodeAlreadyExists(N),
    MultipleSurfaces(N),
    NotFullyConnected(N),
    NotASingleNode(N),
    NoNeighbours(N),
}

impl<N> fmt::Display for Error<N>
where
    N: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::NodeNotFound(key) => {
                write!(f, "No node found with with id: {key:?}")
            }
            Error::NodeAlreadyExists(key) => {
                write!(f, "A node already exists with id: {key:?}")
            }
            Error::MultipleSurfaces(key) => {
                write!(
                    f,
                    "Invalid attempt to reference multi-node element spanning multiple surfaces (i.e. monitors): {key:?}"
                )
            }
            Error::NotFullyConnected(key) => {
                write!(
                    f,
                    "This operation requires that none of the target's exterior lines cross the interior of a node. Requirement not satisfied for target: {key:?}"
                )
            }
            Error::NotASingleNode(key) => {
                write!(
                    f,
                    "Ambiguous attempt to convert a target that spans multiple nodes into a single node without qualifiers: {key:?}"
                )
            }
            Error::NoNeighbours(key) => {
                write!(f, "The target caret has no opposing neighbours")
            }
        }
    }
}

impl<N> std::error::Error for Error<N> where N: Debug {}
