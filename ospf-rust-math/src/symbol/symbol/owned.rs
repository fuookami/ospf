//! 拥有权符号包装
//! Owned symbol wrapper

use super::{DynSymbol, SymbolDynId};
use dyn_clone;
use std::cmp::Ordering;
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};

// ============================================================================
// OwnedSymbol - 拥有权符号包装
// ============================================================================

/// OwnedSymbol - 拥有权符号包装
/// OwnedSymbol - Owned symbol wrapper
///
/// 包装 `Box<dyn DynSymbol>`，提供 `Clone`、`Eq`、`Hash` 实现。
/// Wraps `Box<dyn DynSymbol>`, providing `Clone`, `Eq`, `Hash` implementations.
///
/// # 设计说明 / Design Notes
///
/// - `Clone`: 基于 `dyn_clone::clone_box` 方法
/// - `Eq`/`Hash`: 基于 `dyn_id`，相同 `dyn_id` 的符号视为相等
/// - 用于在多项式中存储符号引用
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::symbol::{OwnedSymbol, DynSymbol, SymbolDynId};
/// use std::fmt::{Display, Formatter, Result};
///
/// // 假设有一个实现了 DynSymbol 的类型
/// # #[derive(Debug, Clone)]
/// # struct SimpleSymbol { id: usize, name: String }
/// # impl Display for SimpleSymbol {
/// #     fn fmt(&self, f: &mut Formatter<'_>) -> Result { write!(f, "{}", self.name) }
/// # }
/// # impl DynSymbol for SimpleSymbol {
/// #     fn name(&self) -> &str { &self.name }
/// #     fn display_name(&self) -> &str { &self.name }
/// #     fn dyn_id(&self) -> SymbolDynId<'_> { SymbolDynId::standalone(self.id) }
/// #     fn as_any(&self) -> &dyn std::any::Any { self }
/// # }
/// let symbol = SimpleSymbol { id: 42, name: "x".to_string() };
/// let owned = OwnedSymbol::new(symbol);
///
/// assert_eq!(owned.dyn_id(), SymbolDynId::standalone(42));
/// ```
pub struct OwnedSymbol(pub Box<dyn DynSymbol>);

impl OwnedSymbol {
    /// 创建新的 OwnedSymbol
    /// Create new OwnedSymbol
    ///
    /// # 示例 / Examples
    ///
    /// ```
    /// use ospf_rust_math::symbol::{OwnedSymbol, DynSymbol, SymbolDynId};
    /// use std::fmt::{Display, Formatter, Result};
    ///
    /// # #[derive(Debug, Clone)]
    /// # struct SimpleSymbol { id: usize, name: String }
    /// # impl Display for SimpleSymbol {
    /// #     fn fmt(&self, f: &mut Formatter<'_>) -> Result { write!(f, "{}", self.name) }
    /// # }
    /// # impl DynSymbol for SimpleSymbol {
    /// #     fn name(&self) -> &str { &self.name }
    /// #     fn display_name(&self) -> &str { &self.name }
    /// #     fn dyn_id(&self) -> SymbolDynId<'_> { SymbolDynId::standalone(self.id) }
    /// #     fn as_any(&self) -> &dyn std::any::Any { self }
    /// # }
    /// let symbol = SimpleSymbol { id: 42, name: "x".to_string() };
    /// let owned = OwnedSymbol::new(symbol);
    /// ```
    pub fn new<S: DynSymbol + 'static>(symbol: S) -> Self {
        Self(Box::new(symbol))
    }

    /// 获取内部符号的引用
    /// Get reference to inner symbol
    pub fn as_ref(&self) -> &dyn DynSymbol {
        self.0.as_ref()
    }

    /// 获取动态标识符
    /// Get dynamic identifier
    pub fn dyn_id(&self) -> SymbolDynId<'_> {
        self.0.dyn_id()
    }

    /// 获取符号名称
    /// Get symbol name
    pub fn name(&self) -> &str {
        self.0.name()
    }

    /// 获取显示名称
    /// Get display name
    pub fn display_name(&self) -> &str {
        self.0.display_name()
    }
}

impl Clone for OwnedSymbol {
    fn clone(&self) -> Self {
        Self(dyn_clone::clone_box(&*self.0))
    }
}

impl Debug for OwnedSymbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("OwnedSymbol")
            .field(&self.0.as_ref())
            .finish()
    }
}

impl Display for OwnedSymbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.as_ref())
    }
}

impl PartialEq for OwnedSymbol {
    fn eq(&self, other: &Self) -> bool {
        self.dyn_id() == other.dyn_id()
    }
}

impl Eq for OwnedSymbol {}

impl Hash for OwnedSymbol {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dyn_id().hash(state);
    }
}

// ============================================================================
// PartialOrd for OwnedSymbol
// ============================================================================

impl PartialOrd for OwnedSymbol {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OwnedSymbol {
    fn cmp(&self, other: &Self) -> Ordering {
        // 先比较 parent_type，再比较 parent_id，最后比较 index
        let self_id = self.dyn_id();
        let other_id = other.dyn_id();

        match self_id.parent_type.cmp(other_id.parent_type) {
            Ordering::Equal => match self_id.parent_id.cmp(&other_id.parent_id) {
                Ordering::Equal => self_id.index.cmp(&other_id.index),
                ord => ord,
            },
            ord => ord,
        }
    }
}

// ============================================================================
// 运算实现 / Operation Implementations
// ============================================================================

use crate::algebra::concept::Scalar;
use crate::symbol::{Linear, LinearMonomial, QuadraticMonomial};
use num_traits::{One, Zero};
use std::ops::{Add, Mul, Sub};

// O1: OwnedSymbol * OwnedSymbol → QuadraticMonomial<T>
// 符号 × 符号 = 二次单项式
// Symbol × Symbol = Quadratic monomial
// 默认使用 f64 作为系数类型
impl Mul for OwnedSymbol {
    type Output = QuadraticMonomial<f64>;

    fn mul(self, rhs: OwnedSymbol) -> Self::Output {
        QuadraticMonomial::quadratic(1.0, self, rhs)
    }
}

// O2: OwnedSymbol * T → LinearMonomial<T>
// 符号 × 标量 = 线性单项式
// Symbol × Scalar = Linear monomial
// 使用 Scalar trait 约束，避免与符号乘法冲突
impl<T: Scalar + Mul<T, Output = T>> Mul<T> for OwnedSymbol {
    type Output = LinearMonomial<T>;

    fn mul(self, rhs: T) -> Self::Output {
        LinearMonomial::new(rhs, self)
    }
}

// O4: OwnedSymbol * LinearMonomial<T> → QuadraticMonomial<T>
// 符号 × 线性单项式 = 二次单项式
// Symbol × Linear monomial = Quadratic monomial
impl<T: Scalar + Mul<T, Output = T>> Mul<LinearMonomial<T>> for OwnedSymbol {
    type Output = QuadraticMonomial<T>;

    fn mul(self, rhs: LinearMonomial<T>) -> Self::Output {
        QuadraticMonomial::quadratic(rhs.coefficient, self, rhs.symbol)
    }
}

// O5: LinearMonomial<T> * OwnedSymbol → QuadraticMonomial<T>
// 线性单项式 × 符号 = 二次单项式
// Linear monomial × Symbol = Quadratic monomial
impl<T: Scalar + Mul<T, Output = T>> Mul<OwnedSymbol> for LinearMonomial<T> {
    type Output = QuadraticMonomial<T>;

    fn mul(self, rhs: OwnedSymbol) -> Self::Output {
        QuadraticMonomial::quadratic(self.coefficient, self.symbol, rhs)
    }
}

// ============================================================================
// 加法运算 / Addition Operations
// ============================================================================

// O10: OwnedSymbol + OwnedSymbol → Linear<T>
// 符号 + 符号 = 线性多项式
// Symbol + Symbol = Linear polynomial
impl Add for OwnedSymbol {
    type Output = Linear<f64>;

    fn add(self, rhs: OwnedSymbol) -> Self::Output {
        Linear::new(
            vec![
                LinearMonomial::new(1.0, self),
                LinearMonomial::new(1.0, rhs),
            ],
            0.0,
        )
    }
}

// O11: OwnedSymbol + T → Linear<T>
// 符号 + 标量 = 线性多项式
// Symbol + Scalar = Linear polynomial
impl<T: Scalar + Zero + One> Add<T> for OwnedSymbol {
    type Output = Linear<T>;

    fn add(self, rhs: T) -> Self::Output {
        Linear::new(vec![LinearMonomial::new(T::one(), self)], rhs)
    }
}

// O12: T + OwnedSymbol → Linear<T>
// 标量 + 符号 = 线性多项式
// Scalar + Symbol = Linear polynomial
// 使用宏为具体类型实现，避免孤儿规则问题
macro_rules! impl_add_scalar_for_owned_symbol {
    ($($t:ty),*) => {
        $(
            impl Add<OwnedSymbol> for $t {
                type Output = Linear<$t>;

                fn add(self, rhs: OwnedSymbol) -> Self::Output {
                    Linear::new(
                        vec![LinearMonomial::new(<$t as num_traits::One>::one(), rhs)],
                        self,
                    )
                }
            }
        )*
    };
}

impl_add_scalar_for_owned_symbol!(f32, f64, i8, i16, i32, i64, i128, isize);

// O13: OwnedSymbol + LinearMonomial<T> → Linear<T>
// 符号 + 线性单项式 = 线性多项式
// Symbol + Linear monomial = Linear polynomial
impl<T: Scalar + Zero + One> Add<LinearMonomial<T>> for OwnedSymbol {
    type Output = Linear<T>;

    fn add(self, rhs: LinearMonomial<T>) -> Self::Output {
        Linear::new(vec![LinearMonomial::new(T::one(), self), rhs], T::zero())
    }
}

// O14: LinearMonomial<T> + OwnedSymbol → Linear<T>
// 线性单项式 + 符号 = 线性多项式
// Linear monomial + Symbol = Linear polynomial
impl<T: Scalar + Zero + One> Add<OwnedSymbol> for LinearMonomial<T> {
    type Output = Linear<T>;

    fn add(self, rhs: OwnedSymbol) -> Self::Output {
        Linear::new(vec![self, LinearMonomial::new(T::one(), rhs)], T::zero())
    }
}

// ============================================================================
// 减法运算 / Subtraction Operations
// ============================================================================

// O30: OwnedSymbol - OwnedSymbol → Linear<T>
// 符号 - 符号 = 线性多项式
// Symbol - Symbol = Linear polynomial
impl Sub for OwnedSymbol {
    type Output = Linear<f64>;

    fn sub(self, rhs: OwnedSymbol) -> Self::Output {
        Linear::new(
            vec![
                LinearMonomial::new(1.0, self),
                LinearMonomial::new(-1.0, rhs),
            ],
            0.0,
        )
    }
}

// O31: OwnedSymbol - T → Linear<T>
// 符号 - 标量 = 线性多项式
// Symbol - Scalar = Linear polynomial
impl<T: Scalar + Zero + One + std::ops::Neg<Output = T>> Sub<T> for OwnedSymbol {
    type Output = Linear<T>;

    fn sub(self, rhs: T) -> Self::Output {
        Linear::new(vec![LinearMonomial::new(T::one(), self)], -rhs)
    }
}

// O32: T - OwnedSymbol → Linear<T>
// 标量 - 符号 = 线性多项式
// Scalar - Symbol = Linear polynomial
// 使用宏为具体类型实现，避免孤儿规则问题
macro_rules! impl_sub_scalar_for_owned_symbol {
    ($($t:ty),*) => {
        $(
            impl Sub<OwnedSymbol> for $t {
                type Output = Linear<$t>;

                fn sub(self, rhs: OwnedSymbol) -> Self::Output {
                    Linear::new(
                        vec![LinearMonomial::new(-<$t as num_traits::One>::one(), rhs)],
                        self,
                    )
                }
            }
        )*
    };
}

impl_sub_scalar_for_owned_symbol!(f32, f64, i8, i16, i32, i64, i128, isize);

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::Any;
    use std::fmt::{Display, Formatter, Result};

    /// 测试用的简单符号
    #[derive(Debug, Clone)]
    struct SimpleSymbol {
        id: usize,
        name: String,
    }

    impl Display for SimpleSymbol {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write!(f, "{}", self.name)
        }
    }

    impl DynSymbol for SimpleSymbol {
        fn name(&self) -> &str {
            &self.name
        }

        fn display_name(&self) -> &str {
            &self.name
        }

        fn dyn_id(&self) -> SymbolDynId<'_> {
            SymbolDynId::standalone(self.id)
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn test_owned_symbol_creation() {
        let symbol = SimpleSymbol {
            id: 42,
            name: "x".to_string(),
        };
        let owned = OwnedSymbol::new(symbol);

        assert_eq!(owned.dyn_id(), SymbolDynId::standalone(42));
        assert_eq!(owned.name(), "x");
    }

    #[test]
    fn test_owned_symbol_equality() {
        let s1 = SimpleSymbol {
            id: 42,
            name: "x".to_string(),
        };
        let s2 = SimpleSymbol {
            id: 42,
            name: "y".to_string(),
        }; // 相同 id，不同 name
        let s3 = SimpleSymbol {
            id: 43,
            name: "x".to_string(),
        };

        let o1 = OwnedSymbol::new(s1);
        let o2 = OwnedSymbol::new(s2);
        let o3 = OwnedSymbol::new(s3);

        // 基于 dyn_id 比较，不基于 name
        assert_eq!(o1, o2);
        assert_ne!(o1, o3);
    }

    #[test]
    fn test_owned_symbol_hash() {
        use std::collections::HashSet;

        let s1 = SimpleSymbol {
            id: 42,
            name: "x".to_string(),
        };
        let s2 = SimpleSymbol {
            id: 42,
            name: "y".to_string(),
        };
        let s3 = SimpleSymbol {
            id: 43,
            name: "z".to_string(),
        };

        let o1 = OwnedSymbol::new(s1);
        let o2 = OwnedSymbol::new(s2);
        let o3 = OwnedSymbol::new(s3);

        let mut set = HashSet::new();
        set.insert(o1);
        set.insert(o2); // 相同 dyn_id，不会重复插入
        set.insert(o3);

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_owned_symbol_ordering() {
        let s1 = SimpleSymbol {
            id: 1,
            name: "x".to_string(),
        };
        let s2 = SimpleSymbol {
            id: 2,
            name: "y".to_string(),
        };

        let o1 = OwnedSymbol::new(s1);
        let o2 = OwnedSymbol::new(s2);

        assert!(o1 < o2);
        assert!(o2 > o1);
    }

    #[test]
    fn test_owned_symbol_debug() {
        let symbol = SimpleSymbol {
            id: 42,
            name: "x".to_string(),
        };
        let owned = OwnedSymbol::new(symbol);

        let debug_str = format!("{:?}", owned);
        assert!(debug_str.contains("OwnedSymbol"));
    }

    #[test]
    fn test_owned_symbol_clone() {
        let symbol = SimpleSymbol {
            id: 42,
            name: "x".to_string(),
        };
        let owned = OwnedSymbol::new(symbol);
        let cloned = owned.clone();

        assert_eq!(owned, cloned);
        assert_eq!(cloned.name(), "x");
    }
}
