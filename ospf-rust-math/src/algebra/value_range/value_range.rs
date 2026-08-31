//! ValueRange - 值空间/区间
//! ValueRange - Value range / interval

use std::fmt;
use crate::algebra::concept::{Bounded, Fixed};
use crate::operator::Contains;
use crate::operator::tolerance::{Tolerance, TolerancedEq, TolerancedOrd};
use super::bound::Bound;
use super::interval::{Closed, Interval, IntervalTrait, Open};
use super::value_wrapper::ValueWrapper;

// ============================================================================
// ValueRange<T, IL, IU> - 值空间/区间
// ============================================================================

/// ValueRange - 值空间/区间
/// ValueRange - Value range / interval
///
/// 表示一个数值区间，支持下界和上界，以及开闭性质。
/// Represents a numeric interval, supporting lower and upper bounds with openness.
///
/// # 类型参数 / Type Parameters
/// - `T`: 数值类型
/// - `IL`: 下界开闭性质类型（编译时 `Closed`/`Open` 或运行时 `Interval`）
/// - `IU`: 上界开闭性质类型（编译时 `Closed`/`Open` 或运行时 `Interval`）
/// - `T`: The numeric type
/// - `IL`: Lower bound openness type (compile-time `Closed`/`Open` or runtime `Interval`)
/// - `IU`: Upper bound openness type (compile-time `Closed`/`Open` or runtime `Interval`)
///
/// # 示例 / Examples
///
/// ## 编译时开闭性质（零开销）
/// ## Compile-time openness (zero overhead)
///
/// ```
/// use ospf_rust_math::algebra::value_range::{ValueRange, Bound, ValueWrapper, Closed, Open};
///
/// // 闭区间 [1, 10]
/// let range: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
///     Bound::new(ValueWrapper::finite(1), Closed),
///     Bound::new(ValueWrapper::finite(10), Closed),
/// );
///
/// // 左闭右开区间 [1, 10)
/// let range: ValueRange<i64, Closed, Open> = ValueRange::from_bounds(
///     Bound::new(ValueWrapper::finite(1), Closed),
///     Bound::new(ValueWrapper::finite(10), Open),
/// );
/// ```
///
/// ## 运行时开闭性质（灵活）
/// ## Runtime openness (flexible)
///
/// ```
/// use ospf_rust_math::algebra::value_range::{ValueRange, Bound, ValueWrapper, Interval};
///
/// // [0, +∞) - 半无限区间
/// let range: ValueRange<i64> = ValueRange::from_bounds(
///     Bound::new(ValueWrapper::finite(0), Interval::Closed),
///     Bound::new(ValueWrapper::positive_infinity(), Interval::Open),
/// );
/// ```
#[derive(Clone, Debug)]
pub struct ValueRange<T, IL: IntervalTrait = Interval, IU: IntervalTrait = Interval> {
    /// 下界
    /// Lower bound
    lower_bound: Bound<T, IL>,
    /// 上界
    /// Upper bound
    upper_bound: Bound<T, IU>,
}

impl<T, IL: IntervalTrait, IU: IntervalTrait> ValueRange<T, IL, IU> {
    /// 从边界创建值区间
    /// Create a value range from bounds
    ///
    /// # 参数 / Parameters
    /// - `lower_bound`: 下界
    /// - `upper_bound`: 上界
    ///
    /// # 返回 / Returns
    /// 新的值区间实例
    /// New value range instance
    pub fn from_bounds(lower_bound: Bound<T, IL>, upper_bound: Bound<T, IU>) -> Self {
        Self {
            lower_bound,
            upper_bound,
        }
    }

    /// 获取下界
    /// Get the lower bound
    ///
    /// # 返回 / Returns
    /// 下界的引用
    /// Reference to the lower bound
    pub fn lower_bound(&self) -> &Bound<T, IL> {
        &self.lower_bound
    }

    /// 获取上界
    /// Get the upper bound
    ///
    /// # 返回 / Returns
    /// 上界的引用
    /// Reference to the upper bound
    pub fn upper_bound(&self) -> &Bound<T, IU> {
        &self.upper_bound
    }

    /// 判断下界是否为闭区间
    /// Check if lower bound is closed
    ///
    /// # 返回 / Returns
    /// 如果下界为闭区间返回 `true`，否则返回 `false`
    /// Returns `true` if lower bound is closed, `false` otherwise
    pub fn is_lower_closed(&self) -> bool {
        self.lower_bound.is_closed()
    }

    /// 判断上界是否为闭区间
    /// Check if upper bound is closed
    ///
    /// # 返回 / Returns
    /// 如果上界为闭区间返回 `true`，否则返回 `false`
    /// Returns `true` if upper bound is closed, `false` otherwise
    pub fn is_upper_closed(&self) -> bool {
        self.upper_bound.is_closed()
    }

    /// 判断下界是否为开区间
    /// Check if lower bound is open
    ///
    /// # 返回 / Returns
    /// 如果下界为开区间返回 `true`，否则返回 `false`
    /// Returns `true` if lower bound is open, `false` otherwise
    pub fn is_lower_open(&self) -> bool {
        self.lower_bound.is_open()
    }

    /// 判断上界是否为开区间
    /// Check if upper bound is open
    ///
    /// # 返回 / Returns
    /// 如果上界为开区间返回 `true`，否则返回 `false`
    /// Returns `true` if upper bound is open, `false` otherwise
    pub fn is_upper_open(&self) -> bool {
        self.upper_bound.is_open()
    }
}

impl<T: PartialOrd, IL: IntervalTrait, IU: IntervalTrait> ValueRange<T, IL, IU> {
    /// 判断值是否在区间内
    /// Check if value is within the range
    ///
    /// # 参数 / Parameters
    /// - `value`: 要检查的值
    /// - `value`: The value to check
    ///
    /// # 返回 / Returns
    /// 如果值在区间内返回 `true`，否则返回 `false`
    /// Returns `true` if value is within range, `false` otherwise
    pub fn contains_value(&self, value: &ValueWrapper<T>) -> bool {
        self.lower_bound.is_above(value) && self.upper_bound.is_below(value)
    }
}

// ============================================================================
// TolerancedEq 实现 / TolerancedEq implementation
// ============================================================================

impl<T: TolerancedEq, IL: IntervalTrait + PartialEq, IU: IntervalTrait + PartialEq> TolerancedEq
    for ValueRange<T, IL, IU>
{
    type Value = T::Value;

    fn eq_within(&self, other: &Self, tolerance: &Tolerance<Self::Value>) -> bool {
        // 下界相等（使用 tolerance）且上界相等（使用 tolerance）
        // Lower bounds are equal (using tolerance) and upper bounds are equal (using tolerance)
        self.lower_bound.eq_within(&other.lower_bound, tolerance)
            && self.upper_bound.eq_within(&other.upper_bound, tolerance)
    }
}

// ============================================================================
// Tolerance 版本 contains 方法 / Tolerance version contains methods
// ============================================================================

impl<T: TolerancedOrd, IL: IntervalTrait, IU: IntervalTrait> ValueRange<T, IL, IU> {
    /// 判断值是否在区间内（带精度容差）
    /// Check if value is within the range (with tolerance)
    ///
    /// # 参数 / Parameters
    /// - `value`: 要检查的值
    /// - `value`: The value to check
    /// - `tolerance`: 精度容差
    ///
    /// # 返回 / Returns
    /// 如果值在区间内返回 `true`，否则返回 `false`
    /// Returns `true` if value is within range, `false` otherwise
    pub fn contains_value_within(
        &self,
        value: &ValueWrapper<T>,
        tolerance: &Tolerance<T::Value>,
    ) -> bool {
        self.lower_bound.is_above_within(value, tolerance)
            && self.upper_bound.is_below_within(value, tolerance)
    }

    /// 判断值是否在区间内（带精度容差）
    /// Check if value is within the range (with tolerance)
    ///
    /// # 参数 / Parameters
    /// - `value`: 要检查的值
    /// - `value`: The value to check
    /// - `tolerance`: 精度容差
    ///
    /// # 返回 / Returns
    /// 如果值在区间内返回 `true`，否则返回 `false`
    /// Returns `true` if value is within range, `false` otherwise
    pub fn contains_within(&self, value: &T, tolerance: &Tolerance<T::Value>) -> bool
    where
        T: Clone,
    {
        self.contains_value_within(&ValueWrapper::finite(value.clone()), tolerance)
    }
}

// ============================================================================
// Bounded trait 实现 / Bounded trait implementation
// ============================================================================

impl<T, IL: IntervalTrait, IU: IntervalTrait> Bounded for ValueRange<T, IL, IU> {
    fn is_bounded() -> bool {
        // ValueRange 本身是否是有界的取决于其边界是否是有限值
        // 但这个方法返回的是类型的性质，不是实例的性质
        // 对于 ValueRange，我们总是返回 true，因为它总是有一个定义的边界
        // ValueRange's boundedness depends on whether its bounds are finite
        // But this method returns the type's property, not the instance's property
        // For ValueRange, we always return true since it always has defined bounds
        true
    }
}

// ============================================================================
// Fixed trait 实现 / Fixed trait implementation
// ============================================================================

impl<T: PartialEq, IL: IntervalTrait, IU: IntervalTrait> Fixed for ValueRange<T, IL, IU> {
    fn is_fixed() -> bool {
        // 类型层面上，ValueRange 不是固定的
        // 实例层面上，只有当上下界相等且都是闭区间时才固定
        // At type level, ValueRange is not fixed
        // At instance level, it's fixed only when bounds are equal and both closed
        false
    }
}

impl<T: PartialEq, IL: IntervalTrait, IU: IntervalTrait> ValueRange<T, IL, IU> {
    /// 创建点区间（退化为单点）
    /// Create a point range (degenerate to single point)
    ///
    /// # 参数 / Parameters
    /// - `value`: 单点值
    ///
    /// # 返回 / Returns
    /// 退化为单点的闭区间
    /// A closed range degenerating to a point
    pub fn point(value: T) -> Self
    where
        IL: Default,
        IU: Default,
        T: Clone,
    {
        Self {
            lower_bound: Bound::new(ValueWrapper::finite(value.clone()), IL::default()),
            upper_bound: Bound::new(ValueWrapper::finite(value), IU::default()),
        }
    }

    /// 判断区间是否退化为单点
    /// Check if the range degenerates to a single point
    ///
    /// # 返回 / Returns
    /// 如果区间退化为单点返回 `true`，否则返回 `false`
    /// Returns `true` if degenerates to a point, `false` otherwise
    pub fn is_degenerate(&self) -> bool {
        // 上下界相等且都是闭区间时，区间退化为单点
        // When bounds are equal and both closed, the range degenerates to a point
        self.lower_bound.value() == self.upper_bound.value()
            && self.lower_bound.is_closed()
            && self.upper_bound.is_closed()
    }

    /// 获取退化点值（如果存在）
    /// Get the degenerate point value (if exists)
    ///
    /// # 返回 / Returns
    /// 如果区间退化为单点返回 `Some(value)`，否则返回 `None`
    /// Returns `Some(value)` if degenerates to a point, `None` otherwise
    pub fn degenerate_value(&self) -> Option<&T> {
        if self.is_degenerate() {
            self.lower_bound.value().unwrap()
        } else {
            None
        }
    }
}

// ============================================================================
// Contains trait 实现 / Contains trait implementation
// ============================================================================

impl<T: PartialOrd, IL: IntervalTrait, IU: IntervalTrait> Contains<ValueWrapper<T>>
    for ValueRange<T, IL, IU>
{
    fn contains(&self, value: &ValueWrapper<T>) -> bool {
        self.contains_value(value)
    }
}

impl<T: PartialOrd + Clone, IL: IntervalTrait, IU: IntervalTrait> Contains<T>
    for ValueRange<T, IL, IU>
{
    fn contains(&self, value: &T) -> bool {
        self.contains_value(&ValueWrapper::finite(value.clone()))
    }
}

// ============================================================================
// PartialEq 实现 / PartialEq implementation
// ============================================================================

impl<T: PartialEq, IL: IntervalTrait, IU: IntervalTrait> PartialEq for ValueRange<T, IL, IU>
where
    IL: PartialEq,
    IU: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.lower_bound == other.lower_bound && self.upper_bound == other.upper_bound
    }
}

impl<T: Eq, IL: IntervalTrait + Eq, IU: IntervalTrait + Eq> Eq for ValueRange<T, IL, IU> {}

// ============================================================================
// Display 实现 / Display implementation
// ============================================================================

impl<T: fmt::Display, IL: IntervalTrait, IU: IntervalTrait> fmt::Display for ValueRange<T, IL, IU> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}, {}{}",
            self.lower_bound.interval().lower_sign(),
            self.lower_bound.value(),
            self.upper_bound.value(),
            self.upper_bound.interval().upper_sign()
        )
    }
}

// ============================================================================
// Default 实现 / Default implementation
// ============================================================================

impl<T: Default, IL: IntervalTrait + Default, IU: IntervalTrait + Default> Default
    for ValueRange<T, IL, IU>
{
    fn default() -> Self {
        Self {
            lower_bound: Bound::default(),
            upper_bound: Bound::default(),
        }
    }
}

// ============================================================================
// IntervalValue 便捷构造方法 / IntervalValue convenience constructors
// ============================================================================

impl<T> ValueRange<T, Closed, Closed> {
    /// 创建闭区间 `[lower, upper]`
    /// Create a closed interval `[lower, upper]`
    ///
    /// # 参数 / Parameters
    /// - `lower`: 下界
    /// - `upper`: 上界
    ///
    /// # 示例 / Example
    /// ```
    /// use ospf_rust_math::algebra::value_range::IntervalValue;
    ///
    /// let interval = IntervalValue::new(1.0, 10.0);
    /// assert!(interval.contains_value(&5.0.into()));
    /// ```
    pub fn new(lower: T, upper: T) -> Self
    where
        T: Clone,
    {
        Self::from_bounds(
            Bound::new(ValueWrapper::finite(lower), Closed),
            Bound::new(ValueWrapper::finite(upper), Closed),
        )
    }

    /// 创建闭区间 `[lower, upper]`（`new` 的别名）
    /// Create a closed interval `[lower, upper]` (alias of `new`)
    pub fn new_closed(lower: T, upper: T) -> Self
    where
        T: Clone,
    {
        Self::new(lower, upper)
    }
}

impl<T> ValueRange<T, Closed, Open> {
    /// 创建左闭右开区间 `[lower, upper)`
    /// Create a half-open interval `[lower, upper)`
    pub fn new_half_open(lower: T, upper: T) -> Self
    where
        T: Clone,
    {
        Self::from_bounds(
            Bound::new(ValueWrapper::finite(lower), Closed),
            Bound::new(ValueWrapper::finite(upper), Open),
        )
    }
}

impl<T> ValueRange<T, Open, Closed> {
    /// 创建左开右闭区间 `(lower, upper]`
    /// Create a half-open interval `(lower, upper]`
    pub fn new_open_closed(lower: T, upper: T) -> Self
    where
        T: Clone,
    {
        Self::from_bounds(
            Bound::new(ValueWrapper::finite(lower), Open),
            Bound::new(ValueWrapper::finite(upper), Closed),
        )
    }
}

impl<T> ValueRange<T, Open, Open> {
    /// 创建开区间 `(lower, upper)`
    /// Create an open interval `(lower, upper)`
    pub fn new_open(lower: T, upper: T) -> Self
    where
        T: Clone,
    {
        Self::from_bounds(
            Bound::new(ValueWrapper::finite(lower), Open),
            Bound::new(ValueWrapper::finite(upper), Open),
        )
    }
}

// ============================================================================
// 区间算术运算 / Interval Arithmetic Operations
// ============================================================================

use num_traits::{One, Zero};
use std::ops::{Add, AddAssign, Mul, Neg, Sub};

impl<T, IL, IU, JL, JU> Add<ValueRange<T, JL, JU>> for ValueRange<T, IL, IU>
where
    T: Add<Output = T> + Clone,
    IL: IntervalTrait,
    IU: IntervalTrait,
    JL: IntervalTrait,
    JU: IntervalTrait,
{
    type Output = ValueRange<T, Interval, Interval>;

    fn add(self, rhs: ValueRange<T, JL, JU>) -> Self::Output {
        // [a, b] + [c, d] = [a+c, b+d]
        // 区间加法：下界相加，上界相加
        // Interval addition: lower bounds add, upper bounds add
        let lower = self.lower_bound.into_value() + rhs.lower_bound.into_value();
        let upper = self.upper_bound.into_value() + rhs.upper_bound.into_value();

        ValueRange::from_bounds(
            Bound::new(lower, Interval::Closed),
            Bound::new(upper, Interval::Closed),
        )
    }
}

impl<T, IL, IU, JL, JU> Sub<ValueRange<T, JL, JU>> for ValueRange<T, IL, IU>
where
    T: Sub<Output = T> + Clone,
    IL: IntervalTrait,
    IU: IntervalTrait,
    JL: IntervalTrait,
    JU: IntervalTrait,
{
    type Output = ValueRange<T, Interval, Interval>;

    fn sub(self, rhs: ValueRange<T, JL, JU>) -> Self::Output {
        // [a, b] - [c, d] = [a-d, b-c]
        // 区间减法：下界减上界，上界减下界
        // Interval subtraction: lower minus upper, upper minus lower
        let lower = self.lower_bound.into_value() - rhs.upper_bound.into_value();
        let upper = self.upper_bound.into_value() - rhs.lower_bound.into_value();

        ValueRange::from_bounds(
            Bound::new(lower, Interval::Closed),
            Bound::new(upper, Interval::Closed),
        )
    }
}

impl<T, IL, IU> Neg for ValueRange<T, IL, IU>
where
    T: Neg<Output = T> + Clone,
    IL: IntervalTrait,
    IU: IntervalTrait,
{
    type Output = ValueRange<T, Interval, Interval>;

    fn neg(self) -> Self::Output {
        // -[a, b] = [-b, -a]
        // 区间取负：上界变下界，下界变上界，符号取反
        // Interval negation: upper becomes lower, lower becomes upper, signs flipped
        let lower = -self.upper_bound.into_value();
        let upper = -self.lower_bound.into_value();

        ValueRange::from_bounds(
            Bound::new(lower, Interval::Closed),
            Bound::new(upper, Interval::Closed),
        )
    }
}

impl<T, IL, IU, JL, JU> Mul<ValueRange<T, JL, JU>> for ValueRange<T, IL, IU>
where
    T: Mul<Output = T> + Clone + PartialOrd + Zero,
    IL: IntervalTrait,
    IU: IntervalTrait,
    JL: IntervalTrait,
    JU: IntervalTrait,
{
    type Output = ValueRange<T, Interval, Interval>;

    fn mul(self, rhs: ValueRange<T, JL, JU>) -> Self::Output {
        // [a, b] * [c, d] 需要计算所有四个端点的乘积，然后取最小和最大
        // [a, b] * [c, d] needs to compute all four endpoint products, then take min and max
        let a = self.lower_bound.into_value();
        let b = self.upper_bound.into_value();
        let c = rhs.lower_bound.into_value();
        let d = rhs.upper_bound.into_value();

        let ac = a.clone() * c.clone();
        let ad = a * d.clone();
        let bc = b.clone() * c;
        let bd = b * d;

        // 找最小值和最大值
        // Find minimum and maximum
        let mut products = [ac, ad, bc, bd];
        products.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
        let [lower, _, _, upper] = products;

        ValueRange::from_bounds(
            Bound::new(lower, Interval::Closed),
            Bound::new(upper, Interval::Closed),
        )
    }
}

// 引用乘法：&ValueRange * &ValueRange -> ValueRange
// Reference multiplication: &ValueRange * &ValueRange -> ValueRange
// 这使得 ValueRange 自动满足 MulRef trait
// This makes ValueRange automatically satisfy MulRef trait
impl<'a, 'b, T, IL, IU, JL, JU> Mul<&'b ValueRange<T, JL, JU>> for &'a ValueRange<T, IL, IU>
where
    T: Mul<Output = T> + Clone + PartialOrd + Zero,
    IL: IntervalTrait,
    IU: IntervalTrait,
    JL: IntervalTrait,
    JU: IntervalTrait,
{
    type Output = ValueRange<T, Interval, Interval>;

    fn mul(self, rhs: &'b ValueRange<T, JL, JU>) -> Self::Output {
        // [a, b] * [c, d] 需要计算所有四个端点的乘积，然后取最小和最大
        // [a, b] * [c, d] needs to compute all four endpoint products, then take min and max
        let a = self.lower_bound.value().clone();
        let b = self.upper_bound.value().clone();
        let c = rhs.lower_bound().value().clone();
        let d = rhs.upper_bound().value().clone();

        let ac = a.clone() * c.clone();
        let ad = a * d.clone();
        let bc = b.clone() * c;
        let bd = b * d;

        // 找最小值和最大值
        // Find minimum and maximum
        let mut products = [ac, ad, bc, bd];
        products.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
        let [lower, _, _, upper] = products;

        ValueRange::from_bounds(
            Bound::new(lower, Interval::Closed),
            Bound::new(upper, Interval::Closed),
        )
    }
}

impl<T, IL, IU> Mul<T> for ValueRange<T, IL, IU>
where
    T: Mul<Output = T> + Clone + PartialOrd + Zero,
    IL: IntervalTrait,
    IU: IntervalTrait,
{
    type Output = ValueRange<T, Interval, Interval>;

    fn mul(self, rhs: T) -> Self::Output {
        // [a, b] * k
        let a = self.lower_bound.into_value();
        let b = self.upper_bound.into_value();

        // 将 rhs 包装为 ValueWrapper
        // Wrap rhs as ValueWrapper
        let non_negative = rhs >= T::zero();
        let rhs_wrapped = ValueWrapper::finite(rhs);
        if non_negative {
            // k >= 0: [a*k, b*k]
            let lower = a * rhs_wrapped.clone();
            let upper = b * rhs_wrapped;
            ValueRange::from_bounds(
                Bound::new(lower, Interval::Closed),
                Bound::new(upper, Interval::Closed),
            )
        } else {
            // k < 0: [b*k, a*k] (区间反转)
            // k < 0: [b*k, a*k] (interval reversed)
            let lower = b * rhs_wrapped.clone();
            let upper = a * rhs_wrapped;
            ValueRange::from_bounds(
                Bound::new(lower, Interval::Closed),
                Bound::new(upper, Interval::Closed),
            )
        }
    }
}

// ============================================================================
// Zero trait 实现 (仅针对 ValueRange<T, Interval, Interval>)
// Zero trait implementation (only for ValueRange<T, Interval, Interval>)
// ============================================================================

impl<T> Zero for ValueRange<T, Interval, Interval>
where
    T: Zero + PartialEq + Clone,
{
    fn zero() -> Self {
        // 零区间：[0, 0]，退化为单点
        // Zero interval: [0, 0], degenerates to a point
        Self {
            lower_bound: Bound::new(ValueWrapper::finite(T::zero()), Interval::Closed),
            upper_bound: Bound::new(ValueWrapper::finite(T::zero()), Interval::Closed),
        }
    }

    fn is_zero(&self) -> bool {
        // 检查是否为零区间
        // Check if this is a zero interval
        self.lower_bound.value() == &ValueWrapper::finite(T::zero())
            && self.upper_bound.value() == &ValueWrapper::finite(T::zero())
    }
}

// ============================================================================
// One trait 实现 (仅针对 ValueRange<T, Interval, Interval>)
// One trait implementation (only for ValueRange<T, Interval, Interval>)
// ============================================================================

impl<T> One for ValueRange<T, Interval, Interval>
where
    T: One + Zero + PartialOrd + Clone + std::ops::Mul<Output = T>,
{
    fn one() -> Self {
        // 一区间：[1, 1]，退化为单点
        // One interval: [1, 1], degenerates to a point
        Self {
            lower_bound: Bound::new(ValueWrapper::finite(T::one()), Interval::Closed),
            upper_bound: Bound::new(ValueWrapper::finite(T::one()), Interval::Closed),
        }
    }
}

// ============================================================================
// AddAssign trait 实现 / AddAssign trait implementation
// ============================================================================

impl<T> AddAssign<ValueRange<T, Interval, Interval>> for ValueRange<T, Interval, Interval>
where
    T: Add<Output = T> + Clone,
{
    fn add_assign(&mut self, rhs: ValueRange<T, Interval, Interval>) {
        // [a, b] += [c, d] => [a+c, b+d]
        // 区间加法赋值：下界相加，上界相加
        // Interval addition assignment: lower bounds add, upper bounds add
        let lower = self.lower_bound.value().clone() + rhs.lower_bound.into_value();
        let upper = self.upper_bound.value().clone() + rhs.upper_bound.into_value();

        self.lower_bound = Bound::new(lower, Interval::Closed);
        self.upper_bound = Bound::new(upper, Interval::Closed);
    }
}

impl<T> AddAssign<&ValueRange<T, Interval, Interval>> for ValueRange<T, Interval, Interval>
where
    T: Add<Output = T> + Clone,
{
    fn add_assign(&mut self, rhs: &ValueRange<T, Interval, Interval>) {
        // [a, b] += &[c, d] => [a+c, b+d]
        // 区间加法赋值（引用版本）：下界相加，上界相加
        // Interval addition assignment (reference version): lower bounds add, upper bounds add
        let lower = self.lower_bound.value().clone() + rhs.lower_bound().value().clone();
        let upper = self.upper_bound.value().clone() + rhs.upper_bound().value().clone();

        self.lower_bound = Bound::new(lower, Interval::Closed);
        self.upper_bound = Bound::new(upper, Interval::Closed);
    }
}

// ============================================================================
// Div trait 实现（标量除法）/ Div trait implementation (scalar division)
// ============================================================================

impl<T, IL, IU> std::ops::Div<T> for ValueRange<T, IL, IU>
where
    T: std::ops::Div<Output = T> + Clone + PartialOrd + Zero,
    IL: IntervalTrait,
    IU: IntervalTrait,
{
    type Output = ValueRange<T, Interval, Interval>;

    fn div(self, rhs: T) -> Self::Output {
        // [a, b] / k 等价于 [a, b] * (1/k)
        // 但我们直接实现除法以避免精度损失
        // [a, b] / k is equivalent to [a, b] * (1/k)
        // But we implement division directly to avoid precision loss
        let a = self.lower_bound.into_value();
        let b = self.upper_bound.into_value();

        // 将 rhs 包装为 ValueWrapper
        // Wrap rhs as ValueWrapper
        let non_negative = rhs >= T::zero();
        let rhs_wrapped = ValueWrapper::finite(rhs);
        if non_negative {
            // k > 0: [a/k, b/k]
            let lower = a / rhs_wrapped.clone();
            let upper = b / rhs_wrapped;
            ValueRange::from_bounds(
                Bound::new(lower, Interval::Closed),
                Bound::new(upper, Interval::Closed),
            )
        } else {
            // k < 0: [b/k, a/k] (区间反转)
            // k < 0: [b/k, a/k] (interval reversed)
            let lower = b / rhs_wrapped.clone();
            let upper = a / rhs_wrapped;
            ValueRange::from_bounds(
                Bound::new(lower, Interval::Closed),
                Bound::new(upper, Interval::Closed),
            )
        }
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::value_range::{Closed, Interval, Open};

    // ========================================================================
    // 基本功能测试 / Basic functionality tests
    // ========================================================================

    #[test]
    fn test_value_range_closed() {
        let range: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Closed),
        );

        assert!(range.is_lower_closed());
        assert!(range.is_upper_closed());
        assert!(!range.is_lower_open());
        assert!(!range.is_upper_open());
    }

    #[test]
    fn test_value_range_open() {
        let range: ValueRange<i64, Open, Open> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Open),
            Bound::new(ValueWrapper::finite(10), Open),
        );

        assert!(range.is_lower_open());
        assert!(range.is_upper_open());
        assert!(!range.is_lower_closed());
        assert!(!range.is_upper_closed());
    }

    #[test]
    fn test_value_range_mixed_compile_time() {
        // [1, 10) - 左闭右开
        let range: ValueRange<i64, Closed, Open> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Open),
        );

        assert!(range.is_lower_closed());
        assert!(range.is_upper_open());

        // (1, 10] - 左开右闭
        let range: ValueRange<i64, Open, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Open),
            Bound::new(ValueWrapper::finite(10), Closed),
        );

        assert!(range.is_lower_open());
        assert!(range.is_upper_closed());
    }

    #[test]
    fn test_value_range_runtime_interval() {
        let range: ValueRange<i64> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Interval::Closed),
            Bound::new(ValueWrapper::finite(10), Interval::Open),
        );

        assert!(range.is_lower_closed());
        assert!(range.is_upper_open());
    }

    // ========================================================================
    // contains 测试 / contains tests
    // ========================================================================

    #[test]
    fn test_contains_closed() {
        let range: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Closed),
        );

        // 闭区间 [1, 10]
        assert!(range.contains_value(&ValueWrapper::finite(1))); // 边界值
        assert!(range.contains_value(&ValueWrapper::finite(5))); // 中间值
        assert!(range.contains_value(&ValueWrapper::finite(10))); // 边界值
        assert!(!range.contains_value(&ValueWrapper::finite(0))); // 小于下界
        assert!(!range.contains_value(&ValueWrapper::finite(11))); // 大于上界
    }

    #[test]
    fn test_contains_open() {
        let range: ValueRange<i64, Open, Open> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Open),
            Bound::new(ValueWrapper::finite(10), Open),
        );

        // 开区间 (1, 10)
        assert!(!range.contains_value(&ValueWrapper::finite(1))); // 边界值不包含
        assert!(range.contains_value(&ValueWrapper::finite(5))); // 中间值
        assert!(!range.contains_value(&ValueWrapper::finite(10))); // 边界值不包含
        assert!(!range.contains_value(&ValueWrapper::finite(0))); // 小于下界
        assert!(!range.contains_value(&ValueWrapper::finite(11))); // 大于上界
    }

    #[test]
    fn test_contains_mixed_compile_time() {
        // [1, 10) - 左闭右开
        let range: ValueRange<i64, Closed, Open> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Open),
        );

        assert!(range.contains_value(&ValueWrapper::finite(1))); // 左边界包含
        assert!(!range.contains_value(&ValueWrapper::finite(10))); // 右边界不包含
        assert!(range.contains_value(&ValueWrapper::finite(5))); // 中间值

        // (1, 10] - 左开右闭
        let range: ValueRange<i64, Open, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Open),
            Bound::new(ValueWrapper::finite(10), Closed),
        );

        assert!(!range.contains_value(&ValueWrapper::finite(1))); // 左边界不包含
        assert!(range.contains_value(&ValueWrapper::finite(10))); // 右边界包含
        assert!(range.contains_value(&ValueWrapper::finite(5))); // 中间值
    }

    #[test]
    fn test_contains_mixed_runtime() {
        let range: ValueRange<i64> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Interval::Closed),
            Bound::new(ValueWrapper::finite(10), Interval::Open),
        );

        // 半开半闭区间 [1, 10)
        assert!(range.contains_value(&ValueWrapper::finite(1))); // 闭区间边界值
        assert!(range.contains_value(&ValueWrapper::finite(5))); // 中间值
        assert!(!range.contains_value(&ValueWrapper::finite(10))); // 开区间边界值不包含
    }

    #[test]
    fn test_contains_infinity() {
        let range: ValueRange<i64> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(0), Interval::Closed),
            Bound::new(ValueWrapper::positive_infinity(), Interval::Open),
        );

        // [0, +∞)
        assert!(range.contains_value(&ValueWrapper::finite(0)));
        assert!(range.contains_value(&ValueWrapper::finite(100)));
        assert!(!range.contains_value(&ValueWrapper::finite(-1)));
        assert!(!range.contains_value(&ValueWrapper::positive_infinity())); // 开区间不包含 +∞
    }

    // ========================================================================
    // is_degenerate 测试 / is_degenerate tests
    // ========================================================================

    #[test]
    fn test_is_degenerate() {
        // 退化为单点 [5, 5]
        let degenerate: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(5), Closed),
            Bound::new(ValueWrapper::finite(5), Closed),
        );
        assert!(degenerate.is_degenerate());
        assert_eq!(degenerate.degenerate_value(), Some(&5));

        // 非退化 [1, 10]
        let non_degenerate: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Closed),
        );
        assert!(!non_degenerate.is_degenerate());
        assert_eq!(non_degenerate.degenerate_value(), None);

        // 值相等但开区间 (5, 5) - 不包含任何值，但不是退化
        let open_same: ValueRange<i64, Open, Open> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(5), Open),
            Bound::new(ValueWrapper::finite(5), Open),
        );
        assert!(!open_same.is_degenerate()); // 开区间不退化

        // 值相等但混合开闭 [5, 5) - 不是退化
        let mixed: ValueRange<i64, Closed, Open> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(5), Closed),
            Bound::new(ValueWrapper::finite(5), Open),
        );
        assert!(!mixed.is_degenerate()); // 混合开闭不退化
    }

    // ========================================================================
    // Display 测试 / Display tests
    // ========================================================================

    #[test]
    fn test_display() {
        let closed: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Closed),
        );
        assert_eq!(format!("{}", closed), "[1, 10]");

        let open: ValueRange<i64, Open, Open> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Open),
            Bound::new(ValueWrapper::finite(10), Open),
        );
        assert_eq!(format!("{}", open), "(1, 10)");

        let mixed: ValueRange<i64, Closed, Open> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Open),
        );
        assert_eq!(format!("{}", mixed), "[1, 10)");

        let infinity: ValueRange<i64> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::negative_infinity(), Interval::Open),
            Bound::new(ValueWrapper::positive_infinity(), Interval::Open),
        );
        assert_eq!(format!("{}", infinity), "(-∞, +∞)");
    }

    // ========================================================================
    // 相等性测试 / Equality tests
    // ========================================================================

    #[test]
    fn test_eq() {
        let a: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Closed),
        );
        let b: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Closed),
        );
        let c: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(20), Closed),
        );

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // ========================================================================
    // Contains trait 测试 / Contains trait tests
    // ========================================================================

    #[test]
    fn test_contains_trait() {
        let range: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Closed),
        );

        // 测试 Contains<T> 实现
        // Test Contains<T> implementation
        use crate::operator::Contains;
        assert!(range.contains(&5_i64));
        assert!(range.contains(&1_i64));
        assert!(range.contains(&10_i64));
        assert!(!range.contains(&0_i64));
        assert!(!range.contains(&11_i64));
    }

    // ========================================================================
    // 区间算术运算测试 / Interval arithmetic tests
    // ========================================================================

    #[test]
    fn test_interval_add() {
        // [1, 10] + [2, 5] = [3, 15]
        let a: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Closed),
        );
        let b: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(2), Closed),
            Bound::new(ValueWrapper::finite(5), Closed),
        );

        let result = a + b;
        assert_eq!(result.lower_bound().value(), &ValueWrapper::finite(3));
        assert_eq!(result.upper_bound().value(), &ValueWrapper::finite(15));
    }

    #[test]
    fn test_interval_sub() {
        // [1, 10] - [2, 5] = [-4, 8]
        let a: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Closed),
        );
        let b: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(2), Closed),
            Bound::new(ValueWrapper::finite(5), Closed),
        );

        let result = a - b;
        assert_eq!(result.lower_bound().value(), &ValueWrapper::finite(-4));
        assert_eq!(result.upper_bound().value(), &ValueWrapper::finite(8));
    }

    #[test]
    fn test_interval_mul() {
        // [2, 3] * [4, 5] = [8, 15]
        let a: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(2), Closed),
            Bound::new(ValueWrapper::finite(3), Closed),
        );
        let b: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(4), Closed),
            Bound::new(ValueWrapper::finite(5), Closed),
        );

        let result = a * b;
        assert_eq!(result.lower_bound().value(), &ValueWrapper::finite(8));
        assert_eq!(result.upper_bound().value(), &ValueWrapper::finite(15));

        // [-1, 2] * [3, 4] = [-4, 8]
        let c: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(-1), Closed),
            Bound::new(ValueWrapper::finite(2), Closed),
        );
        let d: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(3), Closed),
            Bound::new(ValueWrapper::finite(4), Closed),
        );

        let result2 = c * d;
        assert_eq!(result2.lower_bound().value(), &ValueWrapper::finite(-4));
        assert_eq!(result2.upper_bound().value(), &ValueWrapper::finite(8));
    }

    #[test]
    fn test_interval_neg() {
        // -[1, 10] = [-10, -1]
        let a: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(1), Closed),
            Bound::new(ValueWrapper::finite(10), Closed),
        );

        let result = -a;
        assert_eq!(result.lower_bound().value(), &ValueWrapper::finite(-10));
        assert_eq!(result.upper_bound().value(), &ValueWrapper::finite(-1));
    }

    #[test]
    fn test_interval_zero() {
        use num_traits::Zero;

        // Zero 只对 ValueRange<T, Interval, Interval> 实现
        // Zero is only implemented for ValueRange<T, Interval, Interval>
        let zero: ValueRange<i64> = ValueRange::zero();
        assert!(zero.is_zero());
        assert!(zero.is_degenerate());
        assert_eq!(zero.lower_bound().value(), &ValueWrapper::finite(0));
        assert_eq!(zero.upper_bound().value(), &ValueWrapper::finite(0));
    }

    #[test]
    fn test_interval_one() {
        use num_traits::One;

        // One 只对 ValueRange<T, Interval, Interval> 实现
        // One is only implemented for ValueRange<T, Interval, Interval>
        let one: ValueRange<i64> = ValueRange::one();
        assert_eq!(one.lower_bound().value(), &ValueWrapper::finite(1));
        assert_eq!(one.upper_bound().value(), &ValueWrapper::finite(1));
        assert!(one.is_degenerate());
    }

    #[test]
    fn test_interval_div() {
        // [4, 8] / 2 = [2, 4]
        let a: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(4), Closed),
            Bound::new(ValueWrapper::finite(8), Closed),
        );

        let result = a / 2;
        assert_eq!(result.lower_bound().value(), &ValueWrapper::finite(2));
        assert_eq!(result.upper_bound().value(), &ValueWrapper::finite(4));

        // [4, 8] / -2 = [-4, -2] (区间反转)
        // [4, 8] / -2 = [-4, -2] (interval reversed)
        let b: ValueRange<i64, Closed, Closed> = ValueRange::from_bounds(
            Bound::new(ValueWrapper::finite(4), Closed),
            Bound::new(ValueWrapper::finite(8), Closed),
        );

        let result2 = b / -2;
        assert_eq!(result2.lower_bound().value(), &ValueWrapper::finite(-4));
        assert_eq!(result2.upper_bound().value(), &ValueWrapper::finite(-2));
    }
}
