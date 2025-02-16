pub mod bound;
pub mod interval;
pub mod value_range;
pub mod value_range_stc;
pub mod value_wrapper;

pub use bound::*;
pub use interval::*;
pub use value_range::*;
pub use value_range_stc::*;
pub use value_wrapper::*;

struct IllegalArgumentError {
    msg: String,
}
