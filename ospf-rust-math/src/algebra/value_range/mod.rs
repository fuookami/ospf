pub use bound::*;
pub use interval::*;
pub use value_range::*;
// pub use value_range_stc::*;
pub use value_wrapper::*;

pub mod bound;
pub mod interval;
pub mod value_range;
// pub mod value_range_stc;
pub mod value_wrapper;

pub struct IllegalArgumentError {
    msg: String,
}

impl std::fmt::Display for IllegalArgumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Illegal argument: {}", self.msg)
    }
}

impl std::fmt::Debug for IllegalArgumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "IllegalArgumentError {{ msg: {} }}", self.msg)
    }
}
