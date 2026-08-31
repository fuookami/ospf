//! 函数符号主路径（Kotlin 对齐）
//! Function symbol entry path (Kotlin-aligned)

#[path = "../functions/big_m.rs"]
mod big_m;

#[path = "../functions/abs.rs"]
pub mod abs;
#[path = "../functions/balance_ternary.rs"]
pub mod balance_ternary;
#[path = "../functions/binaryzation.rs"]
pub mod binaryzation;
#[path = "../functions/first.rs"]
pub mod first;
#[path = "../functions/if_then.rs"]
pub mod if_then;
#[path = "../functions/in_step_range.rs"]
pub mod in_step_range;
#[path = "../functions/inequality.rs"]
pub mod inequality;
#[path = "../functions/logic.rs"]
pub mod logic;
#[path = "../functions/masking.rs"]
pub mod masking;
#[path = "../functions/max_min.rs"]
pub mod max_min;
#[path = "../functions/min_max.rs"]
pub mod min_max;
#[path = "../functions/mod_function.rs"]
pub mod mod_function;
#[path = "../functions/one_of.rs"]
pub mod one_of;
#[path = "../functions/piecewise.rs"]
pub mod piecewise;
#[path = "../functions/product.rs"]
pub mod product;
#[path = "../functions/quadratic_function.rs"]
pub mod quadratic_function;
#[path = "../functions/rounding.rs"]
pub mod rounding;
#[path = "../functions/same_as.rs"]
pub mod same_as;
#[path = "../functions/satisfied_amount.rs"]
pub mod satisfied_amount;
#[path = "../functions/semantic.rs"]
pub mod semantic;
#[path = "../functions/semi.rs"]
pub mod semi;
#[path = "../functions/sigmoid.rs"]
pub mod sigmoid;
#[path = "../functions/slack.rs"]
pub mod slack;
#[path = "../functions/trigonometric.rs"]
pub mod trigonometric;

pub use abs::*;
pub use balance_ternary::*;
pub use binaryzation::*;
pub use first::*;
pub use if_then::*;
pub use in_step_range::*;
pub use inequality::*;
pub use logic::*;
pub use masking::*;
pub use max_min::*;
pub use min_max::*;
pub use mod_function::*;
pub use one_of::*;
pub use piecewise::*;
pub use product::*;
pub use quadratic_function::*;
pub use rounding::*;
pub use same_as::*;
pub use satisfied_amount::*;
pub use semantic::*;
pub use semi::*;
pub use sigmoid::*;
pub use slack::*;
pub use trigonometric::*;

#[cfg(test)]
#[path = "../functions/p0_evaluation_tests.rs"]
mod p0_evaluation_tests;
