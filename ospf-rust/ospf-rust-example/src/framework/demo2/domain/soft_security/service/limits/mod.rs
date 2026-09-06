//! 软安全约束限制 / Soft security constraint limits.
pub mod adjacent_separation_limit;
pub mod advice_ballast_weight_limit;
pub mod divide_empty_loading_limit;
pub mod empty_hated_limit;
pub mod main_deck_door_empty_limit;
pub mod separation_limit;

pub use adjacent_separation_limit::apply_adjacent_separation_limits;
pub use advice_ballast_weight_limit::apply_advice_ballast_weight_limits;
pub use divide_empty_loading_limit::{EmptyFlagVariables, apply_divide_empty_loading_limits};
pub use empty_hated_limit::apply_empty_hated_limits;
pub use main_deck_door_empty_limit::apply_main_deck_door_empty_limits;
pub use separation_limit::apply_separation_limits;
