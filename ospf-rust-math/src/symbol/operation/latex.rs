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

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 默认选项辅助构造 / Helper to build default options.
    fn defaults() -> LatexOptions {
        LatexOptions::default()
    }

    #[test]
    fn default_options_are_all_off() {
        // 默认必须为"紧凑、隐藏 1、不用 cdot"，保证输出最常规的 LaTeX。
        // Defaults must be plain: not compact, show ones suppressed, no cdot.
        let options = defaults();
        assert!(!options.compact);
        assert!(!options.show_ones);
        assert!(!options.use_cdot);
    }

    #[test]
    fn option_builders_set_only_their_own_flag() {
        // 每个构建器只翻转自己的开关，不影响其它选项。
        // Each builder flips only its own flag and leaves the others untouched.
        let compact = LatexOptions::new().compact();
        assert!(compact.compact);
        assert!(!compact.show_ones);
        assert!(!compact.use_cdot);

        let show_ones = LatexOptions::new().show_ones();
        assert!(show_ones.show_ones);
        assert!(!show_ones.compact);
        assert!(!show_ones.use_cdot);

        let cdot = LatexOptions::new().use_cdot();
        assert!(cdot.use_cdot);
        assert!(!cdot.compact);
        assert!(!cdot.show_ones);
    }

    #[test]
    fn option_builders_chain() {
        // 构建器必须可链式组合 / Builders must compose in a chain.
        let options = LatexOptions::new().compact().show_ones().use_cdot();
        assert!(options.compact);
        assert!(options.show_ones);
        assert!(options.use_cdot);
    }

    #[test]
    fn new_matches_default() {
        // new() 必须等价于 default() / new() must equal default().
        let from_new = LatexOptions::new();
        let from_default = LatexOptions::default();
        assert_eq!(from_new.compact, from_default.compact);
        assert_eq!(from_new.show_ones, from_default.show_ones);
        assert_eq!(from_new.use_cdot, from_default.use_cdot);
    }

    #[test]
    fn zero_coefficient_renders_as_nothing() {
        // 零系数必须整体省略，不能留下 "0x" 这种噪声项。
        // A zero coefficient must be dropped entirely rather than leaving a "0x" term.
        assert_eq!(format_coefficient(&0.0_f64, &defaults(), true), "");
        assert_eq!(format_coefficient(&0.0_f64, &defaults(), false), "");
        assert_eq!(format_coefficient(&0.0_f64, &LatexOptions::new().show_ones(), false), "");
    }

    #[test]
    fn unit_coefficient_omits_the_digit_by_default() {
        // 默认隐藏 1：首项省略，后续项只留符号。
        // By default the digit 1 is hidden: omitted for the first term, sign-only afterwards.
        assert_eq!(format_coefficient(&1.0_f64, &defaults(), true), "");
        assert_eq!(format_coefficient(&1.0_f64, &defaults(), false), " + ");
        assert_eq!(format_coefficient(&(-1.0_f64), &defaults(), true), "-");
        assert_eq!(format_coefficient(&(-1.0_f64), &defaults(), false), " - ");
    }

    #[test]
    fn unit_coefficient_keeps_the_digit_when_show_ones_is_on() {
        // 开启 show_ones 后必须显式写出 1 / With show_ones on, the digit 1 must be written.
        let options = LatexOptions::new().show_ones();
        assert_eq!(format_coefficient(&1.0_f64, &options, true), "1");
        assert_eq!(format_coefficient(&1.0_f64, &options, false), " + 1");
        assert_eq!(format_coefficient(&(-1.0_f64), &options, true), "-1");
        assert_eq!(format_coefficient(&(-1.0_f64), &options, false), " - 1");
    }

    #[test]
    fn ordinary_coefficients_carry_their_sign_correctly() {
        // 普通系数：首项不带前导运算符，后续项带 "+"/"-" 且用绝对值。
        // Ordinary coefficients: the first term has no leading operator; later terms carry
        // an explicit "+"/"-" and use the absolute value.
        assert_eq!(format_coefficient(&2.5_f64, &defaults(), true), "2.5");
        assert_eq!(format_coefficient(&2.5_f64, &defaults(), false), " + 2.5");
        assert_eq!(format_coefficient(&(-2.5_f64), &defaults(), true), "-2.5");
        assert_eq!(format_coefficient(&(-2.5_f64), &defaults(), false), " - 2.5");
    }

    #[test]
    fn single_character_names_pass_through_unchanged() {
        // 单字符变量名不做下标处理 / Single-character names are not subscripted.
        assert_eq!(format_symbol_name("x"), "x");
        assert_eq!(format_symbol_name("y"), "y");
        assert_eq!(format_symbol_name("Z"), "Z");
    }

    #[test]
    fn greek_letters_become_latex_commands() {
        // 希腊字母必须转成 LaTeX 命令 / Greek letters must become LaTeX commands.
        assert_eq!(format_symbol_name("alpha"), "\\alpha");
        assert_eq!(format_symbol_name("Omega"), "\\Omega");
        assert_eq!(format_symbol_name("pi"), "\\pi");
    }

    #[test]
    fn multi_character_names_become_subscripts() {
        // 多字符变量名转成"首字符_其余"，与 LaTeX 习惯一致。
        // Multi-character names become "<first>_<rest>", matching LaTeX convention.
        assert_eq!(format_symbol_name("x1"), "x_1");
        assert_eq!(format_symbol_name("abc"), "a_bc");
        assert_eq!(format_symbol_name("count"), "c_ount");
    }

    #[test]
    fn greek_check_is_exact_and_case_sensitive() {
        // 希腊字母判定必须精确匹配：复数字母或大小写变体不得被误判。
        // The Greek check must match exactly: near-misses or case variants must not be
        // mistaken for Greek letters.
        assert_eq!(format_symbol_name("alphas"), "a_lphas", "复数形式不得当作希腊字母");
        assert_eq!(format_symbol_name("ALPHA"), "A_LPHA", "全大写变体不在表中");
    }

    #[test]
    fn empty_name_is_returned_as_is() {
        // 空名必须原样返回，不能 panic / An empty name must pass through without panicking.
        assert_eq!(format_symbol_name(""), "");
    }

    #[test]
    fn comparison_symbols_map_to_latex() {
        // 比较符号必须映射为对应 LaTeX 命令 / Comparison symbols map to their LaTeX commands.
        assert_eq!(format_comparison("<="), "\\le");
        assert_eq!(format_comparison(">="), "\\ge");
        assert_eq!(format_comparison("!="), "\\ne");
        assert_eq!(format_comparison("=="), "=");
        assert_eq!(format_comparison("<"), "<");
        assert_eq!(format_comparison(">"), ">");
    }

    #[test]
    fn unknown_comparison_symbols_pass_through() {
        // 未知符号原样返回，便于发现拼写错误而不是静默吞掉。
        // Unknown symbols pass through so typos surface instead of being silently dropped.
        assert_eq!(format_comparison("=<"), "=<");
        assert_eq!(format_comparison(""), "");
        assert_eq!(format_comparison("approximately"), "approximately");
    }
}
