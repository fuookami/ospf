//! 装载效能约束限制 / Loading effectiveness constraint limits.
//!
//! 包含建议装载、物品顺序、拖车等各类装载约束和目标。

pub mod advice_load_amount_limit;
pub mod advice_load_weight_limit;
pub mod item_ahead_load_limit;
pub mod item_order_reverse_limit;
pub mod item_reserve_limit;
pub mod item_reweigh_needed_limit;
pub mod priority_order_limit;
pub mod same_destination_adjacent;
pub mod same_source_adjacent_limit;
pub mod source_early_limit;
pub mod trailer_change_limit;
pub mod trailer_circling_limit;

/// 应用建议装载数量限制 / Apply advice load amount limits
pub use advice_load_amount_limit::apply_advice_load_amount_limits;
/// 应用建议装载重量限制 / Apply advice load weight limits
pub use advice_load_weight_limit::apply_advice_load_weight_limits;
/// 应用物品提前装载限制 / Apply item ahead load limits
pub use item_ahead_load_limit::apply_item_ahead_load_limits;
/// 应用物品顺序反转限制 / Apply item order reverse limits
pub use item_order_reverse_limit::apply_item_order_reverse_limits;
/// 应用物品保留限制 / Apply item reserve limits
pub use item_reserve_limit::apply_item_reserve_limits;
/// 应用物品重新称重限制 / Apply item reweigh needed limits
pub use item_reweigh_needed_limit::apply_item_reweigh_needed_limits;
/// 应用优先级顺序限制 / Apply priority order limits
pub use priority_order_limit::apply_priority_order_limits;
/// 应用同目的地邻接限制 / Apply same destination adjacent limits
pub use same_destination_adjacent::apply_same_destination_adjacent_limits;
/// 应用同来源邻接限制 / Apply same source adjacent limits
pub use same_source_adjacent_limit::apply_same_source_adjacent_limits;
/// 应用来源早期限制 / Apply source early limits
pub use source_early_limit::apply_source_early_limits;
/// 应用拖车更换限制 / Apply trailer change limits
pub use trailer_change_limit::apply_trailer_change_limits;
/// 应用拖车循环限制 / Apply trailer circling limits
pub use trailer_circling_limit::apply_trailer_circling_limits;
