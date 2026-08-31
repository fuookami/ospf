//! 序列化支持 / Serialization support
//!
//! 提供符号表达式的序列化和反序列化功能。
//! Provides serialization and deserialization for symbol expressions.
//!
//! # 设计说明 / Design Notes
//!
//! 由于 `OwnedSymbol` 包含 `Box<dyn DynSymbol>`，无法直接序列化。
//! 因此采用 `SymbolExpr` 枚举格式支持递归嵌套的表达式。
//!
//! Since `OwnedSymbol` contains `Box<dyn DynSymbol>`, it cannot be directly serialized.
//! Therefore, we use the `SymbolExpr` enum format to support recursively nested expressions.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::symbol::inequality::{
    CanonicalInequality, Comparison, LinearInequality, QuadraticInequality,
};
use crate::symbol::{
    Canonical, CanonicalMonomial, DynSymbol, Linear, LinearMonomial, OwnedSymbol, Quadratic,
    QuadraticMonomial, SymbolDynId,
};
use std::collections::HashMap;

// ============================================================================
// SymbolExpr - 可序列化的符号表达式
// ============================================================================

/// 可序列化的符号表达式 / Serializable symbol expression
///
/// 支持递归嵌套的符号表达式表示。
/// Supports recursively nested symbol expression representation.
///
/// # 示例 / Examples
///
/// ```ignore
/// use ospf_rust_math::symbol::serde::SymbolExpr;
///
/// // 简单符号
/// let simple = SymbolExpr::Simple { name: "x".to_string() };
///
/// // 线性多项式: 2x + 3y + 1
/// let linear = SymbolExpr::Linear {
///     monomials: vec![
///         LinearMonomialExpr { coefficient: 2.0, symbol: SymbolExpr::Simple { name: "x".to_string() } },
///         LinearMonomialExpr { coefficient: 3.0, symbol: SymbolExpr::Simple { name: "y".to_string() } },
///     ],
///     constant: 1.0,
/// };
/// ```
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum SymbolExpr<T> {
    /// 简单符号 / Simple symbol
    Simple {
        /// 符号名称 / Symbol name
        name: String,
    },

    /// 带ID的符号 / Symbol with ID
    WithId {
        /// 符号名称 / Symbol name
        name: String,
        /// 符号ID / Symbol ID
        id: usize,
    },

    /// 复合符号（单参数）/ Composite symbol (single argument)
    Composite {
        /// 运算符名称 / Operator name
        operator: String,
        /// 内部表达式 / Inner expression
        inner: Box<SymbolExpr<T>>,
    },

    /// 复合符号（多参数）/ Composite symbol (multiple arguments)
    CompositeMulti {
        /// 运算符名称 / Operator name
        operator: String,
        /// 参数列表 / Argument list
        args: Vec<SymbolExpr<T>>,
    },

    /// 线性多项式 / Linear polynomial
    Linear {
        monomials: Vec<LinearMonomialExpr<T>>,
        constant: T,
    },

    /// 二次多项式 / Quadratic polynomial
    Quadratic {
        monomials: Vec<QuadraticMonomialExpr<T>>,
        constant: T,
    },

    /// 标准多项式 / Canonical polynomial
    Canonical {
        monomials: Vec<CanonicalMonomialExpr<T>>,
        constant: T,
    },
}

// ============================================================================
// 单项式序列化表示 / Monomial Serialization Representations
// ============================================================================

/// 线性单项式序列化表示 / Linear monomial serialization representation
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct LinearMonomialExpr<T> {
    /// 系数 / Coefficient
    pub coefficient: T,
    /// 符号 / Symbol
    pub symbol: SymbolExpr<T>,
}

/// 二次单项式序列化表示 / Quadratic monomial serialization representation
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct QuadraticMonomialExpr<T> {
    /// 系数 / Coefficient
    pub coefficient: T,
    /// 第一个符号 / First symbol
    pub symbol1: SymbolExpr<T>,
    /// 第二个符号（None 表示线性项）/ Second symbol (None for linear term)
    pub symbol2: Option<SymbolExpr<T>>,
}

/// 标准单项式序列化表示 / Canonical monomial serialization representation
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct CanonicalMonomialExpr<T> {
    /// 系数 / Coefficient
    pub coefficient: T,
    /// 幂次映射 / Powers mapping
    pub powers: Vec<(SymbolExpr<T>, i32)>,
}

// ============================================================================
// 不等式序列化表示 / Inequality Serialization Representations
// ============================================================================

/// 线性不等式序列化表示 / Linear inequality serialization representation
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct LinearInequalityExpr<T> {
    /// 左侧多项式 / Left-hand side polynomial
    pub lhs: LinearMonomialExprs<T>,
    /// 比较运算符 / Comparison operator
    pub comparison: ComparisonExpr,
    /// 右侧常数 / Right-hand side constant
    pub rhs: T,
}

/// 二次不等式序列化表示 / Quadratic inequality serialization representation
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct QuadraticInequalityExpr<T> {
    /// 左侧多项式 / Left-hand side polynomial
    pub lhs: QuadraticMonomialExprs<T>,
    /// 比较运算符 / Comparison operator
    pub comparison: ComparisonExpr,
    /// 右侧常数 / Right-hand side constant
    pub rhs: T,
}

/// 标准不等式序列化表示 / Canonical inequality serialization representation
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct CanonicalInequalityExpr<T> {
    /// 左侧多项式 / Left-hand side polynomial
    pub lhs: CanonicalMonomialExprs<T>,
    /// 比较运算符 / Comparison operator
    pub comparison: ComparisonExpr,
    /// 右侧常数 / Right-hand side constant
    pub rhs: T,
}

/// 线性多项式（仅单项式列表）/ Linear polynomial (monomials only)
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct LinearMonomialExprs<T> {
    /// 单项式列表 / Monomial list
    pub monomials: Vec<LinearMonomialExpr<T>>,
    /// 常数项 / Constant term
    pub constant: T,
}

/// 二次多项式（仅单项式列表）/ Quadratic polynomial (monomials only)
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct QuadraticMonomialExprs<T> {
    /// 单项式列表 / Monomial list
    pub monomials: Vec<QuadraticMonomialExpr<T>>,
    /// 常数项 / Constant term
    pub constant: T,
}

/// 标准多项式（仅单项式列表）/ Canonical polynomial (monomials only)
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct CanonicalMonomialExprs<T> {
    /// 单项式列表 / Monomial list
    pub monomials: Vec<CanonicalMonomialExpr<T>>,
    /// 常数项 / Constant term
    pub constant: T,
}

/// 比较运算符序列化表示 / Comparison operator serialization representation
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonExpr {
    /// 小于 / Less than
    Less,
    /// 小于等于 / Less than or equal
    LessEqual,
    /// 大于 / Greater than
    Greater,
    /// 大于等于 / Greater than or equal
    GreaterEqual,
    /// 等于 / Equal
    Equal,
}

impl From<Comparison> for ComparisonExpr {
    fn from(c: Comparison) -> Self {
        match c {
            Comparison::Less => ComparisonExpr::Less,
            Comparison::LessEqual => ComparisonExpr::LessEqual,
            Comparison::Greater => ComparisonExpr::Greater,
            Comparison::GreaterEqual => ComparisonExpr::GreaterEqual,
            Comparison::Equal => ComparisonExpr::Equal,
        }
    }
}

impl From<ComparisonExpr> for Comparison {
    fn from(c: ComparisonExpr) -> Self {
        match c {
            ComparisonExpr::Less => Comparison::Less,
            ComparisonExpr::LessEqual => Comparison::LessEqual,
            ComparisonExpr::Greater => Comparison::Greater,
            ComparisonExpr::GreaterEqual => Comparison::GreaterEqual,
            ComparisonExpr::Equal => Comparison::Equal,
        }
    }
}

// ============================================================================
// 序列化 trait / Serialization Traits
// ============================================================================

/// 转换为可序列化表达式 / Convert to serializable expression
pub trait ToSerializable<T> {
    /// 输出类型 / Output type
    type Output;

    /// 转换为可序列化格式 / Convert to serializable format
    fn to_serializable(&self) -> Self::Output;
}

/// 从可序列化表达式创建 / Create from serializable expression
pub trait FromSerializable<T>: Sized {
    /// 从可序列化格式创建 / Create from serializable format
    fn from_serializable(expr: &SymbolExpr<T>) -> Option<Self>;
}

// ============================================================================
// 实现序列化转换 / Implement Serialization Conversions
// ============================================================================

use std::fmt::{Display, Formatter, Result};

/// 用于反序列化的简单符号 / Simple symbol for deserialization
#[derive(Debug, Clone)]
struct SimpleSymbolForSerde {
    id: usize,
    name: String,
}

impl Display for SimpleSymbolForSerde {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.name)
    }
}

impl DynSymbol for SimpleSymbolForSerde {
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

impl<T: Clone> SymbolExpr<T> {
    /// 转换为 OwnedSymbol / Convert to OwnedSymbol
    pub fn to_owned_symbol(&self) -> Option<OwnedSymbol> {
        match self {
            SymbolExpr::Simple { name } => {
                // 为简单符号生成一个基于名称哈希的ID
                // Generate an ID based on name hash for simple symbols
                let id = name.chars().map(|c| c as usize).sum();
                Some(OwnedSymbol::new(SimpleSymbolForSerde {
                    id,
                    name: name.clone(),
                }))
            }
            SymbolExpr::WithId { name, id } => Some(OwnedSymbol::new(SimpleSymbolForSerde {
                id: *id,
                name: name.clone(),
            })),
            _ => None, // 复杂表达式暂不支持直接转换为 OwnedSymbol
        }
    }
}

// ============================================================================
// Linear 序列化实现 / Linear Serialization Implementation
// ============================================================================

impl<T: Clone> Linear<T> {
    /// 转换为可序列化格式 / Convert to serializable format
    pub fn to_serializable(
        &self,
        symbol_to_expr: impl Fn(&OwnedSymbol) -> SymbolExpr<T>,
    ) -> LinearMonomialExprs<T> {
        LinearMonomialExprs {
            monomials: self
                .monomials
                .iter()
                .map(|m| LinearMonomialExpr {
                    coefficient: m.coefficient.clone(),
                    symbol: symbol_to_expr(&m.symbol),
                })
                .collect(),
            constant: self.constant.clone(),
        }
    }

    /// 从可序列化格式创建 / Create from serializable format
    pub fn from_serializable(
        expr: &LinearMonomialExprs<T>,
        expr_to_symbol: impl Fn(&SymbolExpr<T>) -> Option<OwnedSymbol>,
    ) -> Option<Self> {
        let monomials: Vec<LinearMonomial<T>> = expr
            .monomials
            .iter()
            .filter_map(|m| {
                expr_to_symbol(&m.symbol).map(|symbol| LinearMonomial {
                    coefficient: m.coefficient.clone(),
                    symbol,
                })
            })
            .collect();

        if monomials.len() == expr.monomials.len() {
            Some(Linear {
                monomials,
                constant: expr.constant.clone(),
            })
        } else {
            None
        }
    }
}

// ============================================================================
// Quadratic 序列化实现 / Quadratic Serialization Implementation
// ============================================================================

impl<T: Clone> Quadratic<T> {
    /// 转换为可序列化格式 / Convert to serializable format
    pub fn to_serializable(
        &self,
        symbol_to_expr: impl Fn(&OwnedSymbol) -> SymbolExpr<T>,
    ) -> QuadraticMonomialExprs<T> {
        QuadraticMonomialExprs {
            monomials: self
                .monomials
                .iter()
                .map(|m| QuadraticMonomialExpr {
                    coefficient: m.coefficient.clone(),
                    symbol1: symbol_to_expr(&m.symbol1),
                    symbol2: m.symbol2.as_ref().map(|s| symbol_to_expr(s)),
                })
                .collect(),
            constant: self.constant.clone(),
        }
    }

    /// 从可序列化格式创建 / Create from serializable format
    pub fn from_serializable(
        expr: &QuadraticMonomialExprs<T>,
        expr_to_symbol: impl Fn(&SymbolExpr<T>) -> Option<OwnedSymbol>,
    ) -> Option<Self> {
        let monomials: Vec<QuadraticMonomial<T>> = expr
            .monomials
            .iter()
            .filter_map(|m| {
                let symbol1 = expr_to_symbol(&m.symbol1)?;
                let symbol2 = m.symbol2.as_ref().map(|s| expr_to_symbol(s)).flatten();

                Some(QuadraticMonomial {
                    coefficient: m.coefficient.clone(),
                    symbol1,
                    symbol2,
                })
            })
            .collect();

        if monomials.len() == expr.monomials.len() {
            Some(Quadratic {
                monomials,
                constant: expr.constant.clone(),
            })
        } else {
            None
        }
    }
}

// ============================================================================
// Canonical 序列化实现 / Canonical Serialization Implementation
// ============================================================================

impl<T: Clone> Canonical<T> {
    /// 转换为可序列化格式 / Convert to serializable format
    pub fn to_serializable(
        &self,
        symbol_to_expr: impl Fn(&OwnedSymbol) -> SymbolExpr<T>,
    ) -> CanonicalMonomialExprs<T> {
        CanonicalMonomialExprs {
            monomials: self
                .monomials
                .iter()
                .map(|m| CanonicalMonomialExpr {
                    coefficient: m.coefficient.clone(),
                    powers: m
                        .powers
                        .iter()
                        .map(|(s, e)| (symbol_to_expr(s), *e))
                        .collect(),
                })
                .collect(),
            constant: self.constant.clone(),
        }
    }

    /// 从可序列化格式创建 / Create from serializable format
    pub fn from_serializable(
        expr: &CanonicalMonomialExprs<T>,
        expr_to_symbol: impl Fn(&SymbolExpr<T>) -> Option<OwnedSymbol>,
    ) -> Option<Self> {
        let monomials: Vec<CanonicalMonomial<T>> = expr
            .monomials
            .iter()
            .filter_map(|m| {
                let powers: HashMap<OwnedSymbol, i32> = m
                    .powers
                    .iter()
                    .filter_map(|(s, e)| expr_to_symbol(s).map(|symbol| (symbol, *e)))
                    .collect();

                if powers.len() == m.powers.len() {
                    Some(CanonicalMonomial {
                        coefficient: m.coefficient.clone(),
                        powers,
                    })
                } else {
                    None
                }
            })
            .collect();

        if monomials.len() == expr.monomials.len() {
            Some(Canonical {
                monomials,
                constant: expr.constant.clone(),
            })
        } else {
            None
        }
    }
}

// ============================================================================
// 不等式序列化实现 / Inequality Serialization Implementation
// ============================================================================

impl<T: Clone> LinearInequality<T> {
    /// 转换为可序列化格式 / Convert to serializable format
    pub fn to_serializable(
        &self,
        symbol_to_expr: impl Fn(&OwnedSymbol) -> SymbolExpr<T>,
    ) -> LinearInequalityExpr<T> {
        LinearInequalityExpr {
            lhs: self.lhs.to_serializable(symbol_to_expr),
            comparison: self.comparison.into(),
            rhs: self.rhs.clone(),
        }
    }

    /// 从可序列化格式创建 / Create from serializable format
    pub fn from_serializable(
        expr: &LinearInequalityExpr<T>,
        expr_to_symbol: impl Fn(&SymbolExpr<T>) -> Option<OwnedSymbol>,
    ) -> Option<Self> {
        Some(LinearInequality {
            lhs: Linear::from_serializable(&expr.lhs, &expr_to_symbol)?,
            comparison: expr.comparison.into(),
            rhs: expr.rhs.clone(),
        })
    }
}

impl<T: Clone> QuadraticInequality<T> {
    /// 转换为可序列化格式 / Convert to serializable format
    pub fn to_serializable(
        &self,
        symbol_to_expr: impl Fn(&OwnedSymbol) -> SymbolExpr<T>,
    ) -> QuadraticInequalityExpr<T> {
        QuadraticInequalityExpr {
            lhs: self.lhs.to_serializable(symbol_to_expr),
            comparison: self.comparison.into(),
            rhs: self.rhs.clone(),
        }
    }

    /// 从可序列化格式创建 / Create from serializable format
    pub fn from_serializable(
        expr: &QuadraticInequalityExpr<T>,
        expr_to_symbol: impl Fn(&SymbolExpr<T>) -> Option<OwnedSymbol>,
    ) -> Option<Self> {
        Some(QuadraticInequality {
            lhs: Quadratic::from_serializable(&expr.lhs, &expr_to_symbol)?,
            comparison: expr.comparison.into(),
            rhs: expr.rhs.clone(),
        })
    }
}

impl<T: Clone> CanonicalInequality<T> {
    /// 转换为可序列化格式 / Convert to serializable format
    pub fn to_serializable(
        &self,
        symbol_to_expr: impl Fn(&OwnedSymbol) -> SymbolExpr<T>,
    ) -> CanonicalInequalityExpr<T> {
        CanonicalInequalityExpr {
            lhs: self.lhs.to_serializable(symbol_to_expr),
            comparison: self.comparison.into(),
            rhs: self.rhs.clone(),
        }
    }

    /// 从可序列化格式创建 / Create from serializable format
    pub fn from_serializable(
        expr: &CanonicalInequalityExpr<T>,
        expr_to_symbol: impl Fn(&SymbolExpr<T>) -> Option<OwnedSymbol>,
    ) -> Option<Self> {
        Some(CanonicalInequality {
            lhs: Canonical::from_serializable(&expr.lhs, &expr_to_symbol)?,
            comparison: expr.comparison.into(),
            rhs: expr.rhs.clone(),
        })
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_expr_creation() {
        let simple = SymbolExpr::Simple::<f64> {
            name: "x".to_string(),
        };
        assert!(matches!(simple, SymbolExpr::Simple { .. }));

        let with_id = SymbolExpr::WithId::<f64> {
            name: "y".to_string(),
            id: 42,
        };
        assert!(matches!(with_id, SymbolExpr::WithId { .. }));
    }

    #[test]
    fn test_symbol_expr_to_owned_symbol() {
        let simple = SymbolExpr::Simple::<f64> {
            name: "x".to_string(),
        };
        let owned = simple.to_owned_symbol();
        assert!(owned.is_some());
        assert_eq!(owned.unwrap().name(), "x");

        let with_id = SymbolExpr::WithId::<f64> {
            name: "y".to_string(),
            id: 42,
        };
        let owned2 = with_id.to_owned_symbol();
        assert!(owned2.is_some());
        assert_eq!(owned2.unwrap().name(), "y");
    }

    #[test]
    fn test_linear_to_serializable() {
        let symbol = OwnedSymbol::new(SimpleSymbolForSerde {
            id: 1,
            name: "x".to_string(),
        });

        let linear = Linear::new(vec![LinearMonomial::new(2.0, symbol)], 1.0);

        let serialized = linear.to_serializable(|s| SymbolExpr::Simple {
            name: s.name().to_string(),
        });
        assert_eq!(serialized.monomials.len(), 1);
        assert_eq!(serialized.constant, 1.0);
        assert_eq!(serialized.monomials[0].coefficient, 2.0);
    }

    #[test]
    fn test_linear_from_serializable() {
        let expr = LinearMonomialExprs {
            monomials: vec![LinearMonomialExpr {
                coefficient: 3.0,
                symbol: SymbolExpr::Simple {
                    name: "x".to_string(),
                },
            }],
            constant: 2.0,
        };

        let restored = Linear::from_serializable(&expr, |s| {
            if let SymbolExpr::Simple { name } = s {
                Some(OwnedSymbol::new(SimpleSymbolForSerde {
                    id: 0,
                    name: name.clone(),
                }))
            } else {
                None
            }
        });

        assert!(restored.is_some());
        let linear = restored.unwrap();
        assert_eq!(linear.monomials.len(), 1);
        assert_eq!(linear.constant, 2.0);
    }

    #[test]
    fn test_comparison_expr_conversion() {
        assert_eq!(Comparison::from(ComparisonExpr::Less), Comparison::Less);
        assert_eq!(
            Comparison::from(ComparisonExpr::LessEqual),
            Comparison::LessEqual
        );
        assert_eq!(
            Comparison::from(ComparisonExpr::Greater),
            Comparison::Greater
        );
        assert_eq!(
            Comparison::from(ComparisonExpr::GreaterEqual),
            Comparison::GreaterEqual
        );
        assert_eq!(Comparison::from(ComparisonExpr::Equal), Comparison::Equal);
    }
}
