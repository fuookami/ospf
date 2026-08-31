//! 标量类型标记
//! Scalar type marker
//!
//! 用于区分标量与符号类型，避免运算符冲突。
//! Used to distinguish scalar types from symbol types, avoiding operator conflicts.

use std::fmt::Debug;

// ============================================================================
// Scalar - 标量类型标记
// ============================================================================

/// 标量类型标记 / Scalar type marker
///
/// 用于区分标量与符号类型，避免运算符冲突。
/// Used to distinguish scalar types from symbol types, avoiding operator conflicts.
///
/// # 设计说明 / Design Notes
///
/// - 只有实现了此 trait 的类型才能用于多项式的标量运算
/// - `OwnedSymbol` 不实现此 trait，从而避免 `Mul<T>` 与 `Mul<OwnedSymbol>` 的冲突
/// - 用户可以为自定义数值类型（如 `BigDecimal`、`Rational`）实现此 trait
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::algebra::concept::Scalar;
///
/// // 检查类型是否为标量
/// fn is_scalar<T: Scalar>() -> bool { true }
///
/// assert!(is_scalar::<f64>());
/// assert!(is_scalar::<i32>());
/// ```
pub trait Scalar: Clone + Debug + 'static {}

// ============================================================================
// 自动实现 / Auto Implementations
// ============================================================================

// 浮点类型 / Floating-point types
impl Scalar for f32 {}
impl Scalar for f64 {}

// 有符号整数 / Signed integers
impl Scalar for i8 {}
impl Scalar for i16 {}
impl Scalar for i32 {}
impl Scalar for i64 {}
impl Scalar for i128 {}
impl Scalar for isize {}

// 无符号整数 / Unsigned integers
impl Scalar for u8 {}
impl Scalar for u16 {}
impl Scalar for u32 {}
impl Scalar for u64 {}
impl Scalar for u128 {}
impl Scalar for usize {}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_marker() {
        // 编译期检查 / Compile-time check
        fn check_scalar<T: Scalar>() {}

        check_scalar::<f32>();
        check_scalar::<f64>();
        check_scalar::<i32>();
        check_scalar::<i64>();
        check_scalar::<u32>();
        check_scalar::<u64>();
    }

    #[test]
    fn test_scalar_clone() {
        // 验证 Scalar 类型可以 clone
        let a: f64 = 3.14;
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn test_scalar_debug() {
        // 验证 Scalar 类型可以 debug
        let a: f64 = 3.14;
        let debug_str = format!("{:?}", a);
        assert!(!debug_str.is_empty());
    }
}
