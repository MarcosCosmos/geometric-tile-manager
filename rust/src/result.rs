use std::fmt;
use std::fmt::{write, Formatter};
use uuid::Uuid;
#[derive(Debug)]
pub enum Error {
    TileNotFound(Uuid),
    TileAlreadyExists(Uuid),
    // note: this can probably be treated as a warning externally
    NothingToTransform,
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::TileNotFound(id) => {
                write!(f, "No tile found with with id: {id}")
            },
            Error::TileAlreadyExists(id) => {
                write!(f, "A tile already exists with id: {id}")
            },
            Error::NothingToTransform => {
                write!(f, "There is no space to perform an oriented transform here (are you trying to expand outside the bounds of a surface?)")
            }
        }
    }
}

impl std::error::Error for Error {}