use std::fmt::{Debug, Display, Formatter};

pub struct IllegalArgumentError {
    pub(crate) msg: String,
}

impl Display for IllegalArgumentError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Illegal argument: {}", self.msg)
    }
}

impl Debug for IllegalArgumentError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Illegal argument: {}", self.msg)
    }
}
