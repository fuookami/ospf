//! Tolerance - 精度容差比较
//! Tolerance - Tolerance-based comparison

// 重新导出 Epsilon trait 方便使用
// Re-export Epsilon trait for convenience
pub use crate::algebra::concept::Epsilon;
use bigdecimal::BigDecimal;
use num_bigint::{BigInt, BigUint};
use num_rational::{BigRational, Rational32, Rational64};
use std::cmp::Ordering;

// ============================================================================
// Tolerance 结构体 - 精度容差比较器
// ============================================================================

/// Tolerance - 精度容差比较器
/// Tolerance - Tolerance comparator
///
/// 用于存储精度容差值，支持带精度的比较操作。
/// Stores epsilon tolerance value for tolerance-based comparison operations.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::operator::tolerance::{Tolerance, Epsilon};
///
/// // 使用默认精度
/// // Use default epsilon
/// let tolerance = Tolerance::<f64>::default();
///
/// // 使用自定义精度
/// // Use custom epsilon
/// let tolerance = Tolerance::new(1e-8);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Tolerance<T> {
    epsilon: T,
}

impl<T> Tolerance<T> {
    /// 使用指定精度创建比较器
    /// Create comparator with specified epsilon
    ///
    /// # 参数 / Parameters
    /// - `epsilon`: 精度容差值
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::operator::tolerance::Tolerance;
    ///
    /// let tolerance = Tolerance::new(1e-8_f64);
    /// ```
    pub fn new(epsilon: T) -> Self {
        Self { epsilon }
    }

    /// 获取精度容差值
    /// Get the epsilon value
    ///
    /// # 返回 / Returns
    /// 精度容差值的引用
    /// Reference to the epsilon value
    pub fn epsilon(&self) -> &T {
        &self.epsilon
    }
}

impl<T: Epsilon> Tolerance<T> {
    /// 使用类型默认精度创建比较器
    /// Create comparator with type's default epsilon
    ///
    /// # 示例 / Examples
    /// ```
    /// use ospf_rust_math::operator::tolerance::{Tolerance, Epsilon};
    ///
    /// let tolerance = Tolerance::<f64>::default_epsilon();
    /// assert!((tolerance.epsilon() - f64::epsilon()).abs() < 1e-15);
    /// ```
    pub fn default_epsilon() -> Self {
        Self {
            epsilon: T::epsilon(),
        }
    }
}

impl<T: Epsilon> Default for Tolerance<T> {
    fn default() -> Self {
        Self::default_epsilon()
    }
}

// ============================================================================
// TolerancedEq Trait - 带精度等价比较
// ============================================================================

/// TolerancedEq - 带精度容差的等价比较
/// TolerancedEq - Equivalence comparison with tolerance
///
/// 支持使用精度容差进行等价比较。
/// Supports equivalence comparison with epsilon tolerance.
///
/// # 类型参数 / Type Parameters
/// - `Rhs`: 比较目标类型，默认为 `Self`
///
/// # 关联类型 / Associated Types
/// - `Value`: 用于精度容差的值类型，必须实现 `Epsilon`
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::operator::tolerance::{Tolerance, TolerancedEq};
///
/// let a = 1.0_f64;
/// let b = 1.0_f64 + 1e-11_f64; // 差值 1e-11 < epsilon (1e-10)
///
/// // 使用默认精度比较（相等）
/// // Compare with default epsilon (equal)
/// assert!(a.eq_with_default_epsilon(&b));
///
/// // 使用更严格的精度比较（不相等）
/// // Compare with stricter epsilon (not equal)
/// let tolerance = Tolerance::new(1e-12);
/// assert!(!a.eq_within(&b, &tolerance));
/// ```
pub trait TolerancedEq<Rhs = Self> {
    /// 用于精度容差的值类型
    /// Value type used for tolerance
    type Value;

    /// 使用精度比较器判断等价
    /// Check equivalence using tolerance comparator
    ///
    /// # 参数 / Parameters
    /// - `other`: 比较目标
    /// - `tolerance`: 精度比较器
    ///
    /// # 返回 / Returns
    /// 如果两值在精度范围内相等返回 `true`
    /// Returns `true` if values are equal within tolerance
    fn eq_within(&self, other: &Rhs, tolerance: &Tolerance<Self::Value>) -> bool;

    /// 使用默认精度判断等价
    /// Check equivalence using default epsilon
    ///
    /// # 参数 / Parameters
    /// - `other`: 比较目标
    ///
    /// # 返回 / Returns
    /// 如果两值在默认精度范围内相等返回 `true`
    /// Returns `true` if values are equal within default epsilon
    fn eq_with_default_epsilon(&self, other: &Rhs) -> bool
    where
        Self::Value: Epsilon,
    {
        self.eq_within(other, &Tolerance::default())
    }

    /// 使用精度比较器判断不等价
    /// Check inequivalence using tolerance comparator
    ///
    /// # 参数 / Parameters
    /// - `other`: 比较目标
    /// - `tolerance`: 精度比较器
    ///
    /// # 返回 / Returns
    /// 如果两值在精度范围内不相等返回 `true`
    /// Returns `true` if values are not equal within tolerance
    fn ne_within(&self, other: &Rhs, tolerance: &Tolerance<Self::Value>) -> bool {
        !self.eq_within(other, tolerance)
    }

    /// 使用默认精度判断不等价
    /// Check inequivalence using default epsilon
    ///
    /// # 参数 / Parameters
    /// - `other`: 比较目标
    ///
    /// # 返回 / Returns
    /// 如果两值在默认精度范围内不相等返回 `true`
    /// Returns `true` if values are not equal within default epsilon
    fn ne_with_default_epsilon(&self, other: &Rhs) -> bool
    where
        Self::Value: Epsilon,
    {
        self.ne_within(other, &Tolerance::default())
    }
}

// ============================================================================
// TolerancedOrd Trait - 带精度序比较
// ============================================================================

/// TolerancedOrd - 带精度容差的顺序比较
/// TolerancedOrd - Ordering comparison with tolerance
///
/// 支持使用精度容差进行顺序比较。
/// Supports ordering comparison with epsilon tolerance.
///
/// # 示例 / Examples
/// ```
/// use ospf_rust_math::operator::tolerance::{Tolerance, TolerancedOrd};
/// use std::cmp::Ordering;
///
/// let a = 1.00001_f64;
/// let b = 1.00002_f64;
///
/// // 使用默认精度比较
/// // Compare with default epsilon
/// assert!(a.lt_with_default_epsilon(&b));
///
/// // 使用更宽松的精度比较（视为相等）
/// // Compare with looser epsilon (considered equal)
/// let tolerance = Tolerance::new(1e-3);
/// assert_eq!(a.cmp_within(&b, &tolerance), Ordering::Equal);
/// ```
pub trait TolerancedOrd<Rhs = Self>: TolerancedEq<Rhs> {
    /// 使用精度比较器进行顺序比较
    /// Ordering comparison using tolerance comparator
    ///
    /// # 参数 / Parameters
    /// - `other`: 比较目标
    /// - `tolerance`: 精度比较器
    ///
    /// # 返回 / Returns
    /// 返回 `Ordering` 枚举值
    /// Returns `Ordering` enum value
    fn cmp_within(&self, other: &Rhs, tolerance: &Tolerance<Self::Value>) -> Ordering;

    /// 使用默认精度进行顺序比较
    /// Ordering comparison using default epsilon
    ///
    /// # 参数 / Parameters
    /// - `other`: 比较目标
    ///
    /// # 返回 / Returns
    /// 返回 `Ordering` 枚举值
    /// Returns `Ordering` enum value
    fn cmp_with_default_epsilon(&self, other: &Rhs) -> Ordering
    where
        Self::Value: Epsilon,
    {
        self.cmp_within(other, &Tolerance::default())
    }

    /// 使用精度比较器判断小于
    /// Check less than using tolerance comparator
    fn lt_within(&self, other: &Rhs, tolerance: &Tolerance<Self::Value>) -> bool {
        matches!(self.cmp_within(other, tolerance), Ordering::Less)
    }

    /// 使用精度比较器判断小于等于
    /// Check less than or equal using tolerance comparator
    fn le_within(&self, other: &Rhs, tolerance: &Tolerance<Self::Value>) -> bool {
        !matches!(self.cmp_within(other, tolerance), Ordering::Greater)
    }

    /// 使用精度比较器判断大于
    /// Check greater than using tolerance comparator
    fn gt_within(&self, other: &Rhs, tolerance: &Tolerance<Self::Value>) -> bool {
        matches!(self.cmp_within(other, tolerance), Ordering::Greater)
    }

    /// 使用精度比较器判断大于等于
    /// Check greater than or equal using tolerance comparator
    fn ge_within(&self, other: &Rhs, tolerance: &Tolerance<Self::Value>) -> bool {
        !matches!(self.cmp_within(other, tolerance), Ordering::Less)
    }

    /// 使用默认精度判断小于
    fn lt_with_default_epsilon(&self, other: &Rhs) -> bool
    where
        Self::Value: Epsilon,
    {
        self.lt_within(other, &Tolerance::default())
    }

    /// 使用默认精度判断小于等于
    fn le_with_default_epsilon(&self, other: &Rhs) -> bool
    where
        Self::Value: Epsilon,
    {
        self.le_within(other, &Tolerance::default())
    }

    /// 使用默认精度判断大于
    fn gt_with_default_epsilon(&self, other: &Rhs) -> bool
    where
        Self::Value: Epsilon,
    {
        self.gt_within(other, &Tolerance::default())
    }

    /// 使用默认精度判断大于等于
    fn ge_with_default_epsilon(&self, other: &Rhs) -> bool
    where
        Self::Value: Epsilon,
    {
        self.ge_within(other, &Tolerance::default())
    }
}

// ============================================================================
// 宏：为类型实现 TolerancedEq 和 TolerancedOrd
// Macro: Implement TolerancedEq and TolerancedOrd for types
// ============================================================================

/// 为基本类型实现 TolerancedEq（Value = Self）
/// Implement TolerancedEq for basic types (Value = Self)
macro_rules! impl_toleranced_eq {
    ($type:ty) => {
        impl TolerancedEq for $type {
            type Value = $type;

            fn eq_within(&self, other: &Self, tolerance: &Tolerance<Self::Value>) -> bool {
                let diff = (self - other).abs();
                diff <= *tolerance.epsilon()
            }
        }
    };
}

/// 为整数类型实现 TolerancedEq（精确比较）
/// Implement TolerancedEq for integer types (exact comparison)
macro_rules! impl_toleranced_eq_exact {
    ($type:ty) => {
        impl TolerancedEq for $type {
            type Value = $type;

            fn eq_within(&self, other: &Self, _tolerance: &Tolerance<Self::Value>) -> bool {
                self == other
            }
        }
    };
}

/// 为类型实现 TolerancedOrd
/// Implement TolerancedOrd for types
macro_rules! impl_toleranced_ord {
    ($type:ty) => {
        impl TolerancedOrd for $type {
            fn cmp_within(&self, other: &Self, tolerance: &Tolerance<Self::Value>) -> Ordering {
                let diff = self - other;
                if diff.abs() <= *tolerance.epsilon() {
                    Ordering::Equal
                } else if diff < <$type as Default>::default() {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
        }
    };
}

/// 为整数类型实现 TolerancedOrd（精确比较）
/// Implement TolerancedOrd for integer types (exact comparison)
macro_rules! impl_toleranced_ord_exact {
    ($type:ty) => {
        impl TolerancedOrd for $type {
            fn cmp_within(&self, other: &Self, _tolerance: &Tolerance<Self::Value>) -> Ordering {
                self.cmp(other)
            }
        }
    };
}

// ============================================================================
// 浮点数 TolerancedEq 实现 / Floating point TolerancedEq implementations
// ============================================================================

impl_toleranced_eq!(f64);
impl_toleranced_eq!(f32);

// ============================================================================
// 浮点数 TolerancedOrd 实现 / Floating point TolerancedOrd implementations
// ============================================================================

impl TolerancedOrd for f64 {
    fn cmp_within(&self, other: &Self, tolerance: &Tolerance<Self::Value>) -> Ordering {
        let diff = self - other;
        if diff.abs() <= *tolerance.epsilon() {
            Ordering::Equal
        } else if diff < 0.0 {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

impl TolerancedOrd for f32 {
    fn cmp_within(&self, other: &Self, tolerance: &Tolerance<Self::Value>) -> Ordering {
        let diff = self - other;
        if diff.abs() <= *tolerance.epsilon() {
            Ordering::Equal
        } else if diff < 0.0 {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

// ============================================================================
// 整数 TolerancedEq 实现 / Integer TolerancedEq implementations
// 整数精确比较，精度为 0
// ============================================================================

impl_toleranced_eq_exact!(i64);
impl_toleranced_eq_exact!(i32);
impl_toleranced_eq_exact!(i128);
impl_toleranced_eq_exact!(i16);
impl_toleranced_eq_exact!(i8);
impl_toleranced_eq_exact!(isize);
impl_toleranced_eq_exact!(u64);
impl_toleranced_eq_exact!(u32);
impl_toleranced_eq_exact!(u128);
impl_toleranced_eq_exact!(u16);
impl_toleranced_eq_exact!(u8);
impl_toleranced_eq_exact!(usize);

// ============================================================================
// 整数 TolerancedOrd 实现 / Integer TolerancedOrd implementations
// ============================================================================

impl_toleranced_ord_exact!(i64);
impl_toleranced_ord_exact!(i32);
impl_toleranced_ord_exact!(i128);
impl_toleranced_ord_exact!(i16);
impl_toleranced_ord_exact!(i8);
impl_toleranced_ord_exact!(isize);
impl_toleranced_ord_exact!(u64);
impl_toleranced_ord_exact!(u32);
impl_toleranced_ord_exact!(u128);
impl_toleranced_ord_exact!(u16);
impl_toleranced_ord_exact!(u8);
impl_toleranced_ord_exact!(usize);

// ============================================================================
// BigDecimal TolerancedEq 实现 / BigDecimal TolerancedEq implementation
// ============================================================================

impl TolerancedEq for BigDecimal {
    type Value = BigDecimal;

    fn eq_within(&self, other: &Self, tolerance: &Tolerance<Self::Value>) -> bool {
        let diff = (self - other).abs();
        diff <= *tolerance.epsilon()
    }
}

// ============================================================================
// BigDecimal TolerancedOrd 实现 / BigDecimal TolerancedOrd implementation
// ============================================================================

impl TolerancedOrd for BigDecimal {
    fn cmp_within(&self, other: &Self, tolerance: &Tolerance<Self::Value>) -> Ordering {
        let diff = self - other;
        if diff.abs() <= *tolerance.epsilon() {
            Ordering::Equal
        } else if diff < BigDecimal::from(0) {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

// ============================================================================
// BigInt TolerancedEq 实现 / BigInt TolerancedEq implementation
// 整数精确比较，精度为 0
// ============================================================================

impl TolerancedEq for BigInt {
    type Value = BigInt;

    fn eq_within(&self, other: &Self, _tolerance: &Tolerance<Self::Value>) -> bool {
        self == other
    }
}

// ============================================================================
// BigInt TolerancedOrd 实现 / BigInt TolerancedOrd implementation
// ============================================================================

impl TolerancedOrd for BigInt {
    fn cmp_within(&self, other: &Self, _tolerance: &Tolerance<Self::Value>) -> Ordering {
        self.cmp(other)
    }
}

// ============================================================================
// BigUint TolerancedEq 实现 / BigUint TolerancedEq implementation
// ============================================================================

impl TolerancedEq for BigUint {
    type Value = BigUint;

    fn eq_within(&self, other: &Self, _tolerance: &Tolerance<Self::Value>) -> bool {
        self == other
    }
}

// ============================================================================
// BigUint TolerancedOrd 实现 / BigUint TolerancedOrd implementation
// ============================================================================

impl TolerancedOrd for BigUint {
    fn cmp_within(&self, other: &Self, _tolerance: &Tolerance<Self::Value>) -> Ordering {
        self.cmp(other)
    }
}

// ============================================================================
// Rational TolerancedEq 实现 / Rational TolerancedEq implementations
// 有理数精确比较，精度为 0
// ============================================================================

impl TolerancedEq for Rational64 {
    type Value = Rational64;

    fn eq_within(&self, other: &Self, _tolerance: &Tolerance<Self::Value>) -> bool {
        self == other
    }
}

impl TolerancedEq for Rational32 {
    type Value = Rational32;

    fn eq_within(&self, other: &Self, _tolerance: &Tolerance<Self::Value>) -> bool {
        self == other
    }
}

impl TolerancedEq for BigRational {
    type Value = BigRational;

    fn eq_within(&self, other: &Self, _tolerance: &Tolerance<Self::Value>) -> bool {
        self == other
    }
}

// ============================================================================
// Rational TolerancedOrd 实现 / Rational TolerancedOrd implementations
// ============================================================================

impl TolerancedOrd for Rational64 {
    fn cmp_within(&self, other: &Self, _tolerance: &Tolerance<Self::Value>) -> Ordering {
        self.cmp(other)
    }
}

impl TolerancedOrd for Rational32 {
    fn cmp_within(&self, other: &Self, _tolerance: &Tolerance<Self::Value>) -> Ordering {
        self.cmp(other)
    }
}

impl TolerancedOrd for BigRational {
    fn cmp_within(&self, other: &Self, _tolerance: &Tolerance<Self::Value>) -> Ordering {
        self.cmp(other)
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::Zero;

    // ========================================================================
    // Epsilon 测试 / Epsilon tests
    // ========================================================================

    #[test]
    fn test_f64_epsilon() {
        assert!((f64::epsilon() - 1e-10).abs() < 1e-15);
        assert_eq!(f64::zero(), 0.0);
    }

    #[test]
    fn test_f32_epsilon() {
        assert!((f32::epsilon() - 1e-6).abs() < 1e-10);
        assert_eq!(f32::zero(), 0.0);
    }

    #[test]
    fn test_i64_epsilon() {
        assert_eq!(i64::epsilon(), 0);
        assert_eq!(i64::zero(), 0);
    }

    #[test]
    fn test_bigdecimal_epsilon() {
        let eps = BigDecimal::epsilon();
        assert!(eps > BigDecimal::from(0));
        assert!(eps < BigDecimal::from(1));
    }

    // ========================================================================
    // Tolerance 测试 / Tolerance tests
    // ========================================================================

    #[test]
    fn test_tolerance_new() {
        let tolerance = Tolerance::new(1e-8_f64);
        assert!((*tolerance.epsilon() - 1e-8).abs() < 1e-15);
    }

    #[test]
    fn test_tolerance_default() {
        let tolerance = Tolerance::<f64>::default();
        assert!((*tolerance.epsilon() - 1e-10).abs() < 1e-15);
    }

    // ========================================================================
    // TolerancedEq 测试 / TolerancedEq tests
    // ========================================================================

    #[test]
    fn test_f64_eq_within() {
        // 使用差值小于 epsilon 的值
        // Use values with difference smaller than epsilon
        let a = 1.0_f64;
        let b = 1.0_f64 + 1e-11_f64; // 差值 1e-11 < epsilon (1e-10)

        // 默认精度 1e-10，应该相等
        assert!(a.eq_with_default_epsilon(&b));

        // 更严格的精度，不相等
        let tolerance = Tolerance::new(1e-12);
        assert!(!a.eq_within(&b, &tolerance));
    }

    #[test]
    fn test_f32_eq_within() {
        // 使用差值小于 epsilon 的值
        // Use values with difference smaller than epsilon
        let a = 1.0_f32;
        let b = 1.0_f32 + 1e-7_f32; // 差值 1e-7 < epsilon (1e-6)

        // 默认精度 1e-6，应该相等
        assert!(a.eq_with_default_epsilon(&b));

        // 更严格的精度，不相等
        let tolerance = Tolerance::new(1e-8_f32);
        assert!(!a.eq_within(&b, &tolerance));
    }

    #[test]
    fn test_i64_eq_within() {
        let a = 42_i64;
        let b = 42_i64;
        let c = 43_i64;

        assert!(a.eq_with_default_epsilon(&b));
        assert!(!a.eq_with_default_epsilon(&c));
    }

    #[test]
    fn test_bigdecimal_eq_within() {
        // 使用差值小于 epsilon (1e-20) 的值
        // Use values with difference smaller than epsilon (1e-20)
        let a: BigDecimal = "1.0".parse().unwrap();
        let b: BigDecimal = "1.000000000000000000001".parse().unwrap(); // 差值 1e-21 < 1e-20

        // 默认精度 1e-20，应该相等
        assert!(a.eq_with_default_epsilon(&b));

        // 更严格的精度，不相等
        let tolerance = Tolerance::new(BigDecimal::new(num_bigint::BigInt::from(1), 25)); // 1e-25
        assert!(!a.eq_within(&b, &tolerance));
    }

    // ========================================================================
    // TolerancedOrd 测试 / TolerancedOrd tests
    // ========================================================================

    #[test]
    fn test_f64_cmp_within() {
        let a = 1.0_f64;
        let b = 1.0_f64 + 1e-11_f64; // 差值 1e-11 < epsilon (1e-10)
        let c = 2.0_f64;

        // a 和 b 在默认精度下相等
        assert_eq!(a.cmp_with_default_epsilon(&b), Ordering::Equal);

        // a 小于 c
        assert!(a.lt_with_default_epsilon(&c));
    }

    #[test]
    fn test_f64_lt_within() {
        let a = 1.0_f64;
        let b = 1.001_f64;

        // 默认精度 1e-10，a < b
        assert!(a.lt_with_default_epsilon(&b));

        // 更宽松的精度，视为相等，不满足 <
        let tolerance = Tolerance::new(1e-2);
        assert!(!a.lt_within(&b, &tolerance));
    }

    #[test]
    fn test_f64_le_within() {
        let a = 1.0_f64;
        let b = 1.0000000001_f64;
        let c = 2.0_f64;

        // 默认精度，a <= b（视为相等）
        assert!(a.le_with_default_epsilon(&b));

        // a <= c
        assert!(a.le_with_default_epsilon(&c));
    }

    #[test]
    fn test_f64_gt_within() {
        let a = 1.001_f64;
        let b = 1.0_f64;

        // 默认精度 1e-10，a > b
        assert!(a.gt_with_default_epsilon(&b));

        // 更宽松的精度，视为相等，不满足 >
        let tolerance = Tolerance::new(1e-2);
        assert!(!a.gt_within(&b, &tolerance));
    }

    #[test]
    fn test_f64_ge_within() {
        let a = 1.0000000001_f64;
        let b = 1.0_f64;
        let c = 0.5_f64;

        // 默认精度，a >= b（视为相等）
        assert!(a.ge_with_default_epsilon(&b));

        // a >= c
        assert!(a.ge_with_default_epsilon(&c));
    }

    #[test]
    fn test_i64_cmp_within() {
        let a = 42_i64;
        let b = 42_i64;
        let c = 100_i64;

        assert_eq!(a.cmp_with_default_epsilon(&b), Ordering::Equal);
        assert!(a.lt_with_default_epsilon(&c));
        assert!(c.gt_with_default_epsilon(&a));
    }

    #[test]
    fn test_bigdecimal_cmp_within() {
        // 使用差值小于 epsilon (1e-20) 的值
        // Use values with difference smaller than epsilon (1e-20)
        let a: BigDecimal = "1.0".parse().unwrap();
        let b: BigDecimal = "1.000000000000000000001".parse().unwrap(); // 差值 1e-21 < 1e-20
        let c: BigDecimal = "2.0".parse().unwrap();

        // a 和 b 在默认精度下相等
        assert_eq!(a.cmp_with_default_epsilon(&b), Ordering::Equal);

        // a 小于 c
        assert!(a.lt_with_default_epsilon(&c));
    }

    // ========================================================================
    // ne_within 测试 / ne_within tests
    // ========================================================================

    #[test]
    fn test_f64_ne_within() {
        let a = 1.0_f64;
        let b = 1.0_f64 + 1e-11_f64; // 差值 1e-11 < epsilon (1e-10)
        let c = 2.0_f64;

        // 默认精度，a 和 b 相等，ne 返回 false
        assert!(!a.ne_with_default_epsilon(&b));

        // a 和 c 不相等
        assert!(a.ne_with_default_epsilon(&c));
    }
}
