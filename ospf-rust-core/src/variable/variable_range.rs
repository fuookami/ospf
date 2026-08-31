//! 变量范围定义
//! Variable Range Definitions

use std::fmt::Debug;
use std::ops::Sub;

// ============================================================================
// VariableRange - 变量范围
// ============================================================================

/// 变量范围 / Variable Range
///
/// 表示变量的取值范围，包含可选的下界和上界。
/// Represents the range of a variable, containing optional lower and upper bounds.
///
/// # 示例 / Examples
///
/// ```rust
/// use ospf_rust_core::variable::VariableRange;
///
/// // 创建有界范围 / Create bounded range
/// let range = VariableRange::new(Some(0.0), Some(100.0));
/// assert_eq!(range.lower_bound, Some(0.0));
/// assert_eq!(range.upper_bound, Some(100.0));
///
/// // 创建无界范围 / Create unbounded range
/// let unbounded = VariableRange::<f64>::unbounded();
/// assert_eq!(unbounded.lower_bound, None);
/// assert_eq!(unbounded.upper_bound, None);
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VariableRange<T> {
    /// 下界 / Lower bound
    /// `None` 表示无下界 / `None` means no lower bound
    pub lower_bound: Option<T>,
    /// 上界 / Upper bound
    /// `None` 表示无上界 / `None` means no upper bound
    pub upper_bound: Option<T>,
}

impl<T> VariableRange<T> {
    /// 创建新的变量范围 / Create new variable range
    pub fn new(lower_bound: Option<T>, upper_bound: Option<T>) -> Self {
        Self {
            lower_bound,
            upper_bound,
        }
    }

    /// 创建无界范围 / Create unbounded range
    pub fn unbounded() -> Self
    where
        T: Default,
    {
        Self::default()
    }

    /// 创建有下界的范围 / Create range with lower bound
    pub fn with_lower(lower_bound: T) -> Self {
        Self {
            lower_bound: Some(lower_bound),
            upper_bound: None,
        }
    }

    /// 创建有上界的范围 / Create range with upper bound
    pub fn with_upper(upper_bound: T) -> Self {
        Self {
            lower_bound: None,
            upper_bound: Some(upper_bound),
        }
    }

    /// 创建双边有界的范围 / Create bounded range
    pub fn bounded(lower_bound: T, upper_bound: T) -> Self {
        Self {
            lower_bound: Some(lower_bound),
            upper_bound: Some(upper_bound),
        }
    }

    /// 创建固定值范围（上下界相等）/ Create fixed value range
    pub fn fixed(value: T) -> Self
    where
        T: Clone,
    {
        Self {
            lower_bound: Some(value.clone()),
            upper_bound: Some(value),
        }
    }

    /// 检查是否有下界 / Check if has lower bound
    pub fn has_lower_bound(&self) -> bool {
        self.lower_bound.is_some()
    }

    /// 检查是否有上界 / Check if has upper bound
    pub fn has_upper_bound(&self) -> bool {
        self.upper_bound.is_some()
    }

    /// 检查是否为无界范围 / Check if unbounded
    pub fn is_unbounded(&self) -> bool {
        self.lower_bound.is_none() && self.upper_bound.is_none()
    }

    /// 检查是否为固定值 / Check if fixed value
    pub fn is_fixed(&self) -> bool
    where
        T: PartialEq,
    {
        match (&self.lower_bound, &self.upper_bound) {
            (Some(lb), Some(ub)) => lb == ub,
            _ => false,
        }
    }

    /// 获取范围宽度 / Get range width
    pub fn width(&self) -> Option<T>
    where
        T: Clone + Sub<Output = T>,
    {
        match (&self.lower_bound, &self.upper_bound) {
            (Some(lb), Some(ub)) => Some(ub.clone() - lb.clone()),
            _ => None,
        }
    }

    /// 检查值是否在范围内 / Check if value is within range
    pub fn contains(&self, value: &T) -> bool
    where
        T: PartialOrd,
    {
        let lower_ok = self.lower_bound.as_ref().map_or(true, |lb| value >= lb);
        let upper_ok = self.upper_bound.as_ref().map_or(true, |ub| value <= ub);
        lower_ok && upper_ok
    }

    /// 设置下界 / Set lower bound
    pub fn set_lower(&mut self, bound: T) {
        self.lower_bound = Some(bound);
    }

    /// 设置上界 / Set upper bound
    pub fn set_upper(&mut self, bound: T) {
        self.upper_bound = Some(bound);
    }

    /// 清除下界 / Clear lower bound
    pub fn clear_lower(&mut self) {
        self.lower_bound = None;
    }

    /// 清除上界 / Clear upper bound
    pub fn clear_upper(&mut self) {
        self.upper_bound = None;
    }

    /// 映射范围值 / Map range values
    pub fn map<U, F>(self, f: F) -> VariableRange<U>
    where
        F: Fn(T) -> U + Clone,
    {
        VariableRange {
            lower_bound: self.lower_bound.map(f.clone()),
            upper_bound: self.upper_bound.map(f),
        }
    }
}

impl<T: Clone + PartialOrd> VariableRange<T> {
    /// 与另一个范围取交集 / Intersect with another range
    pub fn intersect(&self, other: &VariableRange<T>) -> VariableRange<T> {
        let lower_bound = match (&self.lower_bound, &other.lower_bound) {
            (Some(a), Some(b)) => Some(if a >= b { a.clone() } else { b.clone() }),
            (Some(a), None) => Some(a.clone()),
            (None, Some(b)) => Some(b.clone()),
            (None, None) => None,
        };

        let upper_bound = match (&self.upper_bound, &other.upper_bound) {
            (Some(a), Some(b)) => Some(if a <= b { a.clone() } else { b.clone() }),
            (Some(a), None) => Some(a.clone()),
            (None, Some(b)) => Some(b.clone()),
            (None, None) => None,
        };

        VariableRange {
            lower_bound,
            upper_bound,
        }
    }

    /// 与另一个范围取并集 / Union with another range
    pub fn union(&self, other: &VariableRange<T>) -> VariableRange<T> {
        let lower_bound = match (&self.lower_bound, &other.lower_bound) {
            (Some(a), Some(b)) => Some(if a <= b { a.clone() } else { b.clone() }),
            (Some(a), None) => Some(a.clone()),
            (None, Some(b)) => Some(b.clone()),
            (None, None) => None,
        };

        let upper_bound = match (&self.upper_bound, &other.upper_bound) {
            (Some(a), Some(b)) => Some(if a >= b { a.clone() } else { b.clone() }),
            (Some(a), None) => Some(a.clone()),
            (None, Some(b)) => Some(b.clone()),
            (None, None) => None,
        };

        VariableRange {
            lower_bound,
            upper_bound,
        }
    }
}

// ============================================================================
// 常用范围类型别名 / Common Range Type Aliases
// ============================================================================

/// f64 类型变量范围 / f64 variable range
pub type F64Range = VariableRange<f64>;

/// i64 类型变量范围 / i64 variable range
pub type I64Range = VariableRange<i64>;

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_creation() {
        let range = VariableRange::new(Some(0.0), Some(100.0));
        assert_eq!(range.lower_bound, Some(0.0));
        assert_eq!(range.upper_bound, Some(100.0));
        assert!(range.has_lower_bound());
        assert!(range.has_upper_bound());
        assert!(!range.is_unbounded());
    }

    #[test]
    fn test_unbounded_range() {
        let range: VariableRange<f64> = VariableRange::unbounded();
        assert!(range.is_unbounded());
        assert!(!range.has_lower_bound());
        assert!(!range.has_upper_bound());
    }

    #[test]
    fn test_fixed_range() {
        let range = VariableRange::fixed(5.0);
        assert!(range.is_fixed());
        assert_eq!(range.lower_bound, Some(5.0));
        assert_eq!(range.upper_bound, Some(5.0));
    }

    #[test]
    fn test_contains() {
        let range = VariableRange::bounded(0.0, 100.0);
        assert!(range.contains(&50.0));
        assert!(range.contains(&0.0));
        assert!(range.contains(&100.0));
        assert!(!range.contains(&-1.0));
        assert!(!range.contains(&101.0));
    }

    #[test]
    fn test_width() {
        let range = VariableRange::bounded(10.0, 50.0);
        assert_eq!(range.width(), Some(40.0));

        let unbounded: VariableRange<f64> = VariableRange::unbounded();
        assert_eq!(unbounded.width(), None);
    }

    #[test]
    fn test_intersect() {
        let r1 = VariableRange::bounded(0.0, 100.0);
        let r2 = VariableRange::bounded(50.0, 150.0);
        let intersect = r1.intersect(&r2);
        assert_eq!(intersect.lower_bound, Some(50.0));
        assert_eq!(intersect.upper_bound, Some(100.0));
    }

    #[test]
    fn test_union() {
        let r1 = VariableRange::bounded(0.0, 100.0);
        let r2 = VariableRange::bounded(50.0, 150.0);
        let union = r1.union(&r2);
        assert_eq!(union.lower_bound, Some(0.0));
        assert_eq!(union.upper_bound, Some(150.0));
    }
}
