//! Value Range - 值空间/区间
//! Value Range - Value range / interval
//!
//! 本模块提供了值空间（区间）的实现，支持：
//! This module provides implementation of value ranges (intervals), supporting:
//!
//! - 为没有原生无穷大的类型添加无穷大概念
//! - Adding infinity concept to types without native infinity
//! - 编译时和运行时开闭性质
//! - Compile-time and runtime openness/closedness
//! - 完整的区间代数运算
//! - Complete interval algebra operations
//!
//! # 模块结构 / Module Structure
//!
//! - [`ValueWrapper<T>`] - 值包装器，为任意数值类型添加无穷大支持
//! - [`IntervalTrait`] - 开闭性质抽象 trait
//! - [`Closed`] - 闭区间标记类型（编译时）
//! - [`Open`] - 开区间标记类型（编译时）
//! - [`Interval`] - 运行时开闭性质枚举
//! - [`Bound<T, I>`] - 边界
//! - [`ValueRange<T, IL, IU>`] - 值空间/区间
//!
//! # 编译时 vs 运行时开闭性质 / Compile-time vs Runtime Openness
//!
//! ## 编译时开闭性质（零开销）
//! ## Compile-time openness (zero overhead)
//!
//! 使用 `Closed` 或 `Open` 类型标记时，开闭性质在编译时确定。
//! When using `Closed` or `Open` type markers, openness is determined at compile time.
//!
//! - `Closed` 和 `Open` 是零大小类型（ZST），不占用内存
//! - 编译器可以内联优化所有方法调用
//! - 支持混合开闭区间：`ValueRange<T, Closed, Open>` 表示 `[a, b)`
//!
//! ```
//! use ospf_rust_math::algebra::value_range::{ValueRange, Bound, ValueWrapper, Closed, Open};
//!
//! // 闭区间 [1, 10] - 使用 from_bounds
//! let range: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
//!     Bound::new(ValueWrapper::finite(1), Closed),
//!     Bound::new(ValueWrapper::finite(10), Closed),
//! );
//!
//! // 左闭右开区间 [1, 10) - 使用 from_bounds
//! let range: ValueRange<i64, Closed, Open> = ValueRange::from_bounds(
//!     Bound::new(ValueWrapper::finite(1), Closed),
//!     Bound::new(ValueWrapper::finite(10), Open),
//! );
//!
//! // 左开右闭区间 (1, 10] - 使用 from_bounds
//! let range: ValueRange<i64, Open, Closed> = ValueRange::from_bounds(
//!     Bound::new(ValueWrapper::finite(1), Open),
//!     Bound::new(ValueWrapper::finite(10), Closed),
//! );
//! ```
//!
//! ## 运行时开闭性质（灵活）
//! ## Runtime openness (flexible)
//!
//! 使用 `Interval` 枚举时，开闭性质在运行时确定。
//! When using `Interval` enum, openness is determined at runtime.
//!
//! ```
//! use ospf_rust_math::algebra::value_range::{ValueRange, Bound, ValueWrapper, Interval};
//!
//! // [0, +∞) - 半无限区间
//! let range: ValueRange<i64> = ValueRange::from_bounds(
//!     Bound::new(ValueWrapper::finite(0), Interval::Closed),
//!     Bound::new(ValueWrapper::positive_infinity(), Interval::Open),
//! );
//!
//! // 根据条件动态决定开闭性质
//! let lower_is_closed = true;
//! let lower_interval = if lower_is_closed { Interval::Closed } else { Interval::Open };
//! let range: ValueRange<i64> = ValueRange::from_bounds(
//!     Bound::new(ValueWrapper::finite(1), lower_interval),
//!     Bound::new(ValueWrapper::finite(10), Interval::Closed),
//! );
//! ```

pub mod bound;
pub mod interval;
pub mod value_range;
pub mod value_wrapper;

// 重新导出主要类型 / Re-export main types
pub use bound::Bound;
pub use interval::{Closed, Interval, IntervalTrait, Open};
pub use value_range::ValueRange;
pub use value_wrapper::ValueWrapper;

// ============================================================================
// 区间多项式类型别名 / Interval Polynomial Type Aliases
// ============================================================================

/// 区间类型别名 / Interval type alias
///
/// 表示一个闭区间 `[lower, upper]`。
/// Represents a closed interval `[lower, upper]`.
///
/// # 示例 / Example
/// ```
/// use ospf_rust_math::algebra::value_range::IntervalValue;
///
/// let interval = IntervalValue::new(1.0, 10.0);
/// assert!(interval.contains_value(&5.0.into()));
/// ```
pub type IntervalValue<T> = ValueRange<T, Closed, Closed>;

/// 半开区间类型别名 / Half-open interval type alias
///
/// 表示左闭右开区间 `[lower, upper)`。
/// Represents a half-open interval `[lower, upper)`.
pub type HalfOpenInterval<T> = ValueRange<T, Closed, Open>;

/// 开区间类型别名 / Open interval type alias
///
/// 表示开区间 `(lower, upper)`。
/// Represents an open interval `(lower, upper)`.
pub type OpenInterval<T> = ValueRange<T, Open, Open>;

/// 动态区间类型别名 / Dynamic interval type alias
///
/// 表示运行时确定开闭性质的区间。
/// Represents an interval with runtime-determined openness.
pub type DynamicInterval<T> = ValueRange<T, Interval, Interval>;
