//! 单项式单元格
//! Monomial Cell

use ospf_rust_math::symbol::OwnedSymbol;

/// 线性单项式单元格 / Linear Monomial Cell
///
/// 用于表示线性表达式中的单项式。
/// Used to represent monomial in linear expression.
#[derive(Debug, Clone)]
pub struct LinearMonomialCell {
    /// 系数 / Coefficient
    pub coefficient: f64,
    /// 符号 / Symbol
    pub symbol: OwnedSymbol,
}

impl LinearMonomialCell {
    /// 创建新的线性单项式单元格 / Create new linear monomial cell
    pub fn new(coefficient: f64, symbol: OwnedSymbol) -> Self {
        Self {
            coefficient,
            symbol,
        }
    }
}

/// 二次单项式单元格 / Quadratic Monomial Cell
///
/// 用于表示二次表达式中的单项式。
/// Used to represent monomial in quadratic expression.
#[derive(Debug, Clone)]
pub struct QuadraticMonomialCell {
    /// 系数 / Coefficient
    pub coefficient: f64,
    /// 第一个符号 / First symbol
    pub symbol1: OwnedSymbol,
    /// 第二个符号（可选）/ Second symbol (optional)
    pub symbol2: Option<OwnedSymbol>,
}

impl QuadraticMonomialCell {
    /// 创建二次项 / Create quadratic term
    pub fn quadratic(coefficient: f64, symbol1: OwnedSymbol, symbol2: OwnedSymbol) -> Self {
        Self {
            coefficient,
            symbol1,
            symbol2: Some(symbol2),
        }
    }

    /// 创建线性项 / Create linear term
    pub fn linear(coefficient: f64, symbol: OwnedSymbol) -> Self {
        Self {
            coefficient,
            symbol1: symbol,
            symbol2: None,
        }
    }

    /// 是否为二次项 / Whether it's quadratic
    pub fn is_quadratic(&self) -> bool {
        self.symbol2.is_some()
    }
}
