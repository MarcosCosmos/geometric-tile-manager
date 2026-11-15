use crate::core::graph::NodeId;
use std::fmt;
use std::fmt::Formatter;

#[derive(Debug)]
pub enum Error {
    NodeNotFound(NodeId),
    NodeAlreadyExists(NodeId),
    // note: this can probably be treated as a warning externally
    NothingToTransform,
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::NodeNotFound(id) => {
                write!(f, "No node found with with id: {id:?}")
            }
            Error::NodeAlreadyExists(id) => {
                write!(f, "A node already exists with id: {id:?}")
            }
            Error::NothingToTransform => {
                write!(
                    f,
                    "There is no space to perform an oriented transform here (are you trying to expand outside the bounds of a surface?)"
                )
            }
        }
    }
}

impl std::error::Error for Error {}
