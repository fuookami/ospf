//! 通过引用获取单位元和零元的 traits
//! Traits for getting identity and zero elements by reference
//!
//! 这些 traits 提供对单位元和零元的静态引用访问，
//! 解决了在泛型约束中无法直接比较引用的问题。
//! These traits provide static reference access to identity and zero elements,
//! solving the problem of not being able to directly compare references in generic constraints.
//!
//! # 设计说明 / Design Notes
//!
//! 对于 `Copy` 类型（如 `f64`, `i32`），返回静态引用是安全的。
//! For `Copy` types (like `f64`, `i32`), returning static references is safe.

use num_traits::{One, Zero};

/// 通过静态引用获取单位元 / Get multiplicative identity by static reference
///
/// 提供对单位元的静态引用访问，用于泛型代码中避免额外的所有权约束。
/// Provides static reference access to the multiplicative identity,
/// avoiding additional ownership constraints in generic code.
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::operator::OneRef;
///
/// fn check_is_one<T: OneRef + PartialEq + 'static>(val: &T) -> bool {
///     val == T::one_ref()
/// }
///
/// assert!(check_is_one(&1.0_f64));
/// assert!(!check_is_one(&2.0_f64));
/// ```
pub trait OneRef: One {
    /// 获取单位元的静态引用 / Get a static reference to the multiplicative identity
    ///
    /// 返回指向单位元的静态引用。
    /// Returns a static reference to the identity element.
    fn one_ref() -> &'static Self;
}

/// 通过静态引用获取零元 / Get additive identity by static reference
///
/// 提供对零元的静态引用访问，用于泛型代码中避免额外的所有权约束。
/// Provides static reference access to the additive identity,
/// avoiding additional ownership constraints in generic code.
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::operator::ZeroRef;
///
/// fn check_is_zero<T: ZeroRef + PartialEq + 'static>(val: &T) -> bool {
///     val == T::zero_ref()
/// }
///
/// assert!(check_is_zero(&0.0_f64));
/// assert!(!check_is_zero(&1.0_f64));
/// ```
pub trait ZeroRef: Zero {
    /// 获取零元的静态引用 / Get a static reference to the additive identity
    ///
    /// 返回指向零元的静态引用。
    /// Returns a static reference to the zero element.
    fn zero_ref() -> &'static Self;
}

/// 通过静态引用获取负单位元 / Get negative multiplicative identity by static reference
///
/// 提供对负单位元的静态引用访问，用于在泛型代码中比较 `-1` 而不需要 `Neg` trait。
/// Provides static reference access to the negative multiplicative identity,
/// allowing comparison with `-1` in generic code without requiring the `Neg` trait.
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::operator::NegOneRef;
///
/// fn check_is_neg_one<T: NegOneRef + PartialEq + 'static>(val: &T) -> bool {
///     val == T::neg_one_ref()
/// }
///
/// assert!(check_is_neg_one(&-1.0_f64));
/// assert!(!check_is_neg_one(&1.0_f64));
/// ```
pub trait NegOneRef: One {
    /// 获取负单位元的静态引用 / Get a static reference to the negative multiplicative identity
    ///
    /// 返回指向负单位元的静态引用。
    /// Returns a static reference to the negative identity element.
    fn neg_one_ref() -> &'static Self;
}

/// 获取数字 2 / Get the number 2
///
/// 提供对数字 2 的访问，用于在泛型代码中计算一半值。
/// Provides access to the number 2, used for computing half values in generic code.
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::operator::Two;
///
/// fn compute_half<T: Two + std::ops::Div<T, Output = T>>(val: T) -> T {
///     val / T::two()
/// }
///
/// assert_eq!(compute_half(4.0_f64), 2.0);
/// ```
pub trait Two {
    /// 返回数字 2 / Return the number 2
    fn two() -> Self;
}

// ============================================================================
// 标准类型的实现 / Implementations for standard types
// ============================================================================

// 静态常量 / Static constants
const ONE_F32: f32 = 1.0;
const ONE_F64: f64 = 1.0;
const ONE_I8: i8 = 1;
const ONE_I16: i16 = 1;
const ONE_I32: i32 = 1;
const ONE_I64: i64 = 1;
const ONE_I128: i128 = 1;
const ONE_ISIZE: isize = 1;
const ONE_U8: u8 = 1;
const ONE_U16: u16 = 1;
const ONE_U32: u32 = 1;
const ONE_U64: u64 = 1;
const ONE_U128: u128 = 1;
const ONE_USIZE: usize = 1;

const ZERO_F32: f32 = 0.0;
const ZERO_F64: f64 = 0.0;
const ZERO_I8: i8 = 0;
const ZERO_I16: i16 = 0;
const ZERO_I32: i32 = 0;
const ZERO_I64: i64 = 0;
const ZERO_I128: i128 = 0;
const ZERO_ISIZE: isize = 0;
const ZERO_U8: u8 = 0;
const ZERO_U16: u16 = 0;
const ZERO_U32: u32 = 0;
const ZERO_U64: u64 = 0;
const ZERO_U128: u128 = 0;
const ZERO_USIZE: usize = 0;

// 负单位元静态常量（仅对有符号类型）/ Negative one static constants (signed types only)
const NEG_ONE_F32: f32 = -1.0;
const NEG_ONE_F64: f64 = -1.0;
const NEG_ONE_I8: i8 = -1;
const NEG_ONE_I16: i16 = -1;
const NEG_ONE_I32: i32 = -1;
const NEG_ONE_I64: i64 = -1;
const NEG_ONE_I128: i128 = -1;
const NEG_ONE_ISIZE: isize = -1;

// 为数值类型实现 OneRef
impl OneRef for f32 {
    fn one_ref() -> &'static Self {
        &ONE_F32
    }
}

impl OneRef for f64 {
    fn one_ref() -> &'static Self {
        &ONE_F64
    }
}

impl OneRef for i8 {
    fn one_ref() -> &'static Self {
        &ONE_I8
    }
}

impl OneRef for i16 {
    fn one_ref() -> &'static Self {
        &ONE_I16
    }
}

impl OneRef for i32 {
    fn one_ref() -> &'static Self {
        &ONE_I32
    }
}

impl OneRef for i64 {
    fn one_ref() -> &'static Self {
        &ONE_I64
    }
}

impl OneRef for i128 {
    fn one_ref() -> &'static Self {
        &ONE_I128
    }
}

impl OneRef for isize {
    fn one_ref() -> &'static Self {
        &ONE_ISIZE
    }
}

impl OneRef for u8 {
    fn one_ref() -> &'static Self {
        &ONE_U8
    }
}

impl OneRef for u16 {
    fn one_ref() -> &'static Self {
        &ONE_U16
    }
}

impl OneRef for u32 {
    fn one_ref() -> &'static Self {
        &ONE_U32
    }
}

impl OneRef for u64 {
    fn one_ref() -> &'static Self {
        &ONE_U64
    }
}

impl OneRef for u128 {
    fn one_ref() -> &'static Self {
        &ONE_U128
    }
}

impl OneRef for usize {
    fn one_ref() -> &'static Self {
        &ONE_USIZE
    }
}

// 为数值类型实现 ZeroRef
impl ZeroRef for f32 {
    fn zero_ref() -> &'static Self {
        &ZERO_F32
    }
}

impl ZeroRef for f64 {
    fn zero_ref() -> &'static Self {
        &ZERO_F64
    }
}

impl ZeroRef for i8 {
    fn zero_ref() -> &'static Self {
        &ZERO_I8
    }
}

impl ZeroRef for i16 {
    fn zero_ref() -> &'static Self {
        &ZERO_I16
    }
}

impl ZeroRef for i32 {
    fn zero_ref() -> &'static Self {
        &ZERO_I32
    }
}

impl ZeroRef for i64 {
    fn zero_ref() -> &'static Self {
        &ZERO_I64
    }
}

impl ZeroRef for i128 {
    fn zero_ref() -> &'static Self {
        &ZERO_I128
    }
}

impl ZeroRef for isize {
    fn zero_ref() -> &'static Self {
        &ZERO_ISIZE
    }
}

impl ZeroRef for u8 {
    fn zero_ref() -> &'static Self {
        &ZERO_U8
    }
}

impl ZeroRef for u16 {
    fn zero_ref() -> &'static Self {
        &ZERO_U16
    }
}

impl ZeroRef for u32 {
    fn zero_ref() -> &'static Self {
        &ZERO_U32
    }
}

impl ZeroRef for u64 {
    fn zero_ref() -> &'static Self {
        &ZERO_U64
    }
}

impl ZeroRef for u128 {
    fn zero_ref() -> &'static Self {
        &ZERO_U128
    }
}

impl ZeroRef for usize {
    fn zero_ref() -> &'static Self {
        &ZERO_USIZE
    }
}

// 为有符号数值类型实现 NegOneRef
impl NegOneRef for f32 {
    fn neg_one_ref() -> &'static Self {
        &NEG_ONE_F32
    }
}

impl NegOneRef for f64 {
    fn neg_one_ref() -> &'static Self {
        &NEG_ONE_F64
    }
}

impl NegOneRef for i8 {
    fn neg_one_ref() -> &'static Self {
        &NEG_ONE_I8
    }
}

impl NegOneRef for i16 {
    fn neg_one_ref() -> &'static Self {
        &NEG_ONE_I16
    }
}

impl NegOneRef for i32 {
    fn neg_one_ref() -> &'static Self {
        &NEG_ONE_I32
    }
}

impl NegOneRef for i64 {
    fn neg_one_ref() -> &'static Self {
        &NEG_ONE_I64
    }
}

impl NegOneRef for i128 {
    fn neg_one_ref() -> &'static Self {
        &NEG_ONE_I128
    }
}

impl NegOneRef for isize {
    fn neg_one_ref() -> &'static Self {
        &NEG_ONE_ISIZE
    }
}

// 为数值类型实现 Two
impl Two for f32 {
    fn two() -> Self {
        2.0
    }
}

impl Two for f64 {
    fn two() -> Self {
        2.0
    }
}

impl Two for i8 {
    fn two() -> Self {
        2
    }
}

impl Two for i16 {
    fn two() -> Self {
        2
    }
}

impl Two for i32 {
    fn two() -> Self {
        2
    }
}

impl Two for i64 {
    fn two() -> Self {
        2
    }
}

impl Two for i128 {
    fn two() -> Self {
        2
    }
}

impl Two for isize {
    fn two() -> Self {
        2
    }
}

impl Two for u8 {
    fn two() -> Self {
        2
    }
}

impl Two for u16 {
    fn two() -> Self {
        2
    }
}

impl Two for u32 {
    fn two() -> Self {
        2
    }
}

impl Two for u64 {
    fn two() -> Self {
        2
    }
}

impl Two for u128 {
    fn two() -> Self {
        2
    }
}

impl Two for usize {
    fn two() -> Self {
        2
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_one_ref_f64() {
        let one: &f64 = OneRef::one_ref();
        assert_eq!(*one, 1.0);
        assert!(std::ptr::eq(one, OneRef::one_ref())); // 确保是同一个静态引用
    }

    #[test]
    fn test_zero_ref_f64() {
        let zero: &f64 = ZeroRef::zero_ref();
        assert_eq!(*zero, 0.0);
        assert!(std::ptr::eq(zero, ZeroRef::zero_ref())); // 确保是同一个静态引用
    }

    #[test]
    fn test_one_ref_i32() {
        let one: &i32 = OneRef::one_ref();
        assert_eq!(*one, 1);
    }

    #[test]
    fn test_zero_ref_i32() {
        let zero: &i32 = ZeroRef::zero_ref();
        assert_eq!(*zero, 0);
    }

    #[test]
    fn test_comparison_with_ref() {
        let val = 1.0_f64;
        assert!(&val == <f64 as OneRef>::one_ref());

        let val2 = 0.0_f64;
        assert!(&val2 == <f64 as ZeroRef>::zero_ref());
    }
}
