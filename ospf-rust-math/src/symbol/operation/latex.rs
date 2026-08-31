//! LaTeX 输出 Trait 定义
//! LaTeX output trait definitions
//!
//! 本模块提供 LaTeX 格式输出的 trait 定义。
//! This module provides trait definitions for LaTeX format output.

use std::fmt::Display;

// ============================================================================
// LaTeX 输出选项 / LaTeX Output Options
// ============================================================================

/// LaTeX 输出选项 / LaTeX output options
///
/// 控制多项式和不等式在 LaTeX 中的显示格式。
/// Controls the display format of polynomials and inequalities in LaTeX.
#[derive(Clone, Debug, Default)]
pub struct LatexOptions {
    /// 是否使用紧凑格式 / Whether to use compact format
    pub compact: bool,
    /// 是否显示系数为 1 的项 / Whether to show terms with coefficient 1
    pub show_ones: bool,
    /// 是否使用 cdot 表示乘法 / Whether to use cdot for multiplication
    pub use_cdot: bool,
}

impl LatexOptions {
    /// 创建默认选项
    /// Create default options
    pub fn new() -> Self {
        Self::default()
    }

    /// 使用紧凑格式
    /// Use compact format
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// 显示系数为 1 的项
    /// Show terms with coefficient 1
    pub fn show_ones(mut self) -> Self {
        self.show_ones = true;
        self
    }

    /// 使用 cdot 表示乘法
    /// Use cdot for multiplication
    pub fn use_cdot(mut self) -> Self {
        self.use_cdot = true;
        self
    }
}

// ============================================================================
// LaTeX 输出 Trait / LaTeX Output Trait
// ============================================================================

/// 转换为 LaTeX 格式 / Convert to LaTeX format
///
/// 将数学表达式转换为 LaTeX 格式字符串。
/// Converts mathematical expressions to LaTeX format string.
///
/// # 示例 / Examples
///
/// ```rust,ignore
/// use ospf_rust_math::symbol::operation::{ToLaTeX, LatexOptions};
///
/// let polynomial = ...;
/// let latex = polynomial.to_latex(&LatexOptions::default());
/// println!("{}", latex);  // 输出 LaTeX 格式
/// ```
pub trait ToLaTeX {
    /// 转换为 LaTeX 格式
    /// Convert to LaTeX format
    ///
    /// # 参数 / Arguments
    ///
    /// - `options`: LaTeX 输出选项 / LaTeX output options
    fn to_latex(&self, options: &LatexOptions) -> String;
}

// ============================================================================
// 辅助函数 / Helper Functions
// ============================================================================

/// 格式化系数 / Format coefficient
///
/// 根据选项格式化系数，处理特殊值（如 1、-1）。
/// Format coefficient according to options, handling special values (like 1, -1).
///
/// # 参数 / Arguments
///
/// - `coeff`: 系数值 / Coefficient value
/// - `options`: LaTeX 输出选项 / LaTeX output options
/// - `is_first`: 是否是第一项 / Whether this is the first term
///
/// # 返回值 / Returns
///
/// 返回格式化后的系数字符串，可能为空字符串（对于系数为 1 的情况）。
/// Returns formatted coefficient string, may be empty string (for coefficient 1 cases).
pub fn format_coefficient<T>(coeff: &T, options: &LatexOptions, is_first: bool) -> String
where
    T: Display
        + num_traits::Zero
        + num_traits::One
        + PartialEq
        + std::ops::Neg<Output = T>
        + Clone
        + PartialOrd,
{
    if coeff.is_zero() {
        return String::new();
    }

    let zero = T::zero();
    let is_negative = *coeff < zero;
    let abs_coeff = if is_negative {
        -coeff.clone()
    } else {
        coeff.clone()
    };

    if coeff.is_one() || abs_coeff.is_one() {
        // 系数为 ±1
        // Coefficient is ±1
        if options.show_ones {
            if is_first {
                if is_negative {
                    "-1".to_string()
                } else {
                    "1".to_string()
                }
            } else {
                if is_negative {
                    " - 1".to_string()
                } else {
                    " + 1".to_string()
                }
            }
        } else {
            if is_first {
                if is_negative {
                    "-".to_string()
                } else {
                    String::new()
                }
            } else {
                if is_negative {
                    " - ".to_string()
                } else {
                    " + ".to_string()
                }
            }
        }
    } else {
        // 普通系数
        // Normal coefficient
        if is_first {
            if is_negative {
                format!("-{}", abs_coeff)
            } else {
                format!("{}", coeff)
            }
        } else {
            if is_negative {
                format!(" - {}", abs_coeff)
            } else {
                format!(" + {}", coeff)
            }
        }
    }
}

/// 格式化符号名称 / Format symbol name
///
/// 将符号名称转换为 LaTeX 变量格式。
/// Converts symbol name to LaTeX variable format.
///
/// # 参数 / Arguments
///
/// - `name`: 符号名称 / Symbol name
///
/// # 返回值 / Returns
///
/// 返回 LaTeX 格式的符号名称。
/// Returns LaTeX formatted symbol name.
///
/// # 示例 / Examples
///
/// - `"x"` → `"x"`
/// - `"alpha"` → `"\\alpha"`
/// - `"x1"` → `"x_1"`
pub fn format_symbol_name(name: &str) -> String {
    // 检查是否是希腊字母
    // Check if it's a Greek letter
    let greek_letters = [
        "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta", "iota", "kappa",
        "lambda", "mu", "nu", "xi", "omicron", "pi", "rho", "sigma", "tau", "upsilon", "phi",
        "chi", "psi", "omega", "Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Eta",
        "Theta", "Iota", "Kappa", "Lambda", "Mu", "Nu", "Xi", "Omicron", "Pi", "Rho", "Sigma",
        "Tau", "Upsilon", "Phi", "Chi", "Psi", "Omega",
    ];

    if greek_letters.contains(&name) {
        format!("\\{}", name)
    } else if name.len() > 1 {
        // 多字符变量名，使用下标
        // Multi-character variable name, use subscript
        format!(
            "{}_{}",
            name.chars()
                .next()
                .expect("name has at least 1 char / name 至少有 1 个字符"),
            &name[1..]
        )
    } else {
        name.to_string()
    }
}

/// 格式化比较符号 / Format comparison symbol
///
/// 将比较符号转换为 LaTeX 格式。
/// Converts comparison symbol to LaTeX format.
///
/// # 参数 / Arguments
///
/// - `symbol`: 比较符号（如 "<=", ">=", "<", ">"）
/// - `symbol`: Comparison symbol (e.g., "<=", ">=", "<", ">")
///
/// # 返回值 / Returns
///
/// 返回 LaTeX 格式的比较符号。
/// Returns LaTeX formatted comparison symbol.
pub fn format_comparison(symbol: &str) -> String {
    match symbol {
        "<=" => "\\le".to_string(),
        ">=" => "\\ge".to_string(),
        "<" => "<".to_string(),
        ">" => ">".to_string(),
        "==" => "=".to_string(),
        "!=" => "\\ne".to_string(),
        _ => symbol.to_string(),
    }
}
