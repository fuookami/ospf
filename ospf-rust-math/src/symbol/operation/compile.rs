//! 编译求值模块
//! Compile evaluation module
//!
//! 提供将符号多项式编译为高效求值函数的能力。
//! Provides the ability to compile symbolic polynomials into efficient evaluation functions.
//!
//! # 设计思想 / Design Philosophy
//!
//! 符号多项式的求值通常需要遍历所有项，存在运行时开销。
//! 通过编译为闭包，可以消除符号查找开销，显著提升批量求值性能。
//!
//! Symbolic polynomial evaluation typically requires traversing all terms, incurring runtime overhead.
//! By compiling into closures, symbol lookup overhead can be eliminated, significantly improving batch evaluation performance.
//!
//! # 示例 / Example
//!
//! ```rust,ignore
//! use ospf_rust_math::symbol::operation::compile::{CompileEval, CompileGradient};
//! use ospf_rust_math::symbol::{Linear, LinearMonomial};
//!
//! // 编译线性多项式 2x + 3y + 1
//! let poly = Linear::new(vec![
//!     LinearMonomial::new(2.0_f64, x.clone()),
//!     LinearMonomial::new(3.0_f64, y.clone()),
//! ], 1.0_f64);
//! let symbols = vec![x, y];
//!
//! // 编译为求值函数
//! let eval_fn = poly.compile_eval(&symbols);
//! let result = eval_fn(&[2.0_f64, 3.0_f64]); // x=2, y=3 => 2*2 + 3*3 + 1 = 14
//!
//! // 编译为梯度函数
//! let grad_fn = poly.compile_gradient(&symbols);
//! let gradient = grad_fn(&[2.0_f64, 3.0_f64]); // [2.0, 3.0]
//! ```

use crate::symbol::operation::Differentiate;
use crate::symbol::{Linear, OwnedSymbol, Quadratic};
use std::collections::HashMap;
use std::rc::Rc;

// ============================================================================
// CompileEval trait - 编译求值 trait
// ============================================================================

/// 编译求值 trait / Compile evaluation trait
///
/// 将多项式编译为高效的求值函数。
/// Compiles polynomial into an efficient evaluation function.
///
/// # 类型参数 / Type Parameters
///
/// - `T`: 数值类型
///
/// # 示例 / Example
///
/// ```rust,ignore
/// let eval_fn = poly.compile_eval(&symbols);
/// let result = eval_fn(&[x_val, y_val]);
/// ```
pub trait CompileEval<T> {
    /// 编译后的求值函数类型
    /// Compiled evaluation function type
    ///
    /// 函数签名：`fn(values: &[T]) -> T`
    /// Function signature: `fn(values: &[T]) -> T`
    type EvalFn;

    /// 编译为求值函数
    /// Compile to evaluation function
    ///
    /// # 参数 / Arguments
    ///
    /// - `symbols`: 符号列表（确定参数顺序）
    /// - `symbols`: Symbol list (determines parameter order)
    ///
    /// # 返回 / Returns
    ///
    /// 编译后的求值函数
    /// Compiled evaluation function
    fn compile_eval(&self, symbols: &[OwnedSymbol]) -> Self::EvalFn;
}

// ============================================================================
// CompileGradient trait - 编译梯度 trait
// ============================================================================

/// 编译梯度 trait / Compile gradient trait
///
/// 将多项式编译为梯度计算函数。
/// Compiles polynomial into gradient computation function.
///
/// # 类型参数 / Type Parameters
///
/// - `T`: 数值类型
///
/// # 示例 / Example
///
/// ```rust,ignore
/// let grad_fn = poly.compile_gradient(&symbols);
/// let gradient = grad_fn(&[x_val, y_val]); // 返回梯度向量
/// ```
pub trait CompileGradient<T> {
    /// 编译后的梯度函数类型
    /// Compiled gradient function type
    ///
    /// 函数签名：`fn(values: &[T]) -> Vec<T>`
    /// Function signature: `fn(values: &[T]) -> Vec<T>`
    type GradFn;

    /// 编译为梯度计算函数
    /// Compile to gradient computation function
    ///
    /// # 参数 / Arguments
    ///
    /// - `symbols`: 符号列表（确定参数顺序）
    /// - `symbols`: Symbol list (determines parameter order)
    ///
    /// # 返回 / Returns
    ///
    /// 编译后的梯度函数
    /// Compiled gradient function
    fn compile_gradient(&self, symbols: &[OwnedSymbol]) -> Self::GradFn;
}

// ============================================================================
// Linear 实现 / Linear Implementation
// ============================================================================

impl<T> CompileEval<T> for Linear<T>
where
    T: Clone + 'static + std::ops::Add<Output = T> + std::ops::Mul<Output = T> + num_traits::Zero,
{
    type EvalFn = Rc<dyn Fn(&[T]) -> T>;

    fn compile_eval(&self, symbols: &[OwnedSymbol]) -> Self::EvalFn {
        // 建立符号到索引的映射
        // Build symbol to index mapping
        let symbol_index: HashMap<OwnedSymbol, usize> = symbols
            .iter()
            .enumerate()
            .map(|(i, s)| (s.clone(), i))
            .collect();

        // 收集编译后的项：(系数, 符号索引)
        // Collect compiled terms: (coefficient, symbol index)
        let mut terms: Vec<(T, Option<usize>)> = Vec::new();

        for monomial in &self.monomials {
            let idx = symbol_index.get(&monomial.symbol).copied();
            terms.push((monomial.coefficient.clone(), idx));
        }

        let constant = self.constant.clone();

        // 返回编译后的闭包
        // Return compiled closure
        Rc::new(move |values: &[T]| {
            let mut result = constant.clone();
            for (coef, idx) in &terms {
                if let Some(i) = idx {
                    result = result + coef.clone() * values[*i].clone();
                }
            }
            result
        })
    }
}

impl<T> CompileGradient<T> for Linear<T>
where
    T: Clone
        + 'static
        + std::ops::Add<Output = T>
        + std::ops::Mul<Output = T>
        + num_traits::Zero
        + PartialEq
        + for<'a> std::ops::AddAssign<&'a T>,
{
    type GradFn = Rc<dyn Fn(&[T]) -> Vec<T>>;

    fn compile_gradient(&self, symbols: &[OwnedSymbol]) -> Self::GradFn {
        // 线性多项式的梯度就是系数
        // Gradient of linear polynomial is just the coefficients
        let gradients: Vec<T> = symbols.iter().map(|s| self.partial_derivative(s)).collect();

        Rc::new(move |_values: &[T]| gradients.clone())
    }
}

// ============================================================================
// Quadratic 实现 / Quadratic Implementation
// ============================================================================

impl<T> CompileEval<T> for Quadratic<T>
where
    T: Clone + 'static + std::ops::Add<Output = T> + std::ops::Mul<Output = T> + num_traits::Zero,
{
    type EvalFn = Rc<dyn Fn(&[T]) -> T>;

    fn compile_eval(&self, symbols: &[OwnedSymbol]) -> Self::EvalFn {
        // 建立符号到索引的映射
        // Build symbol to index mapping
        let symbol_index: HashMap<OwnedSymbol, usize> = symbols
            .iter()
            .enumerate()
            .map(|(i, s)| (s.clone(), i))
            .collect();

        // 编译后的项类型
        // Compiled term type
        enum CompiledTerm<T> {
            /// 常数项 / Constant term
            /// 线性项 (系数, 符号索引) / Linear term (coefficient, symbol index)
            Linear(T, usize),
            /// 二次项 (系数, 符号1索引, 符号2索引) / Quadratic term (coefficient, symbol1 index, symbol2 index)
            Quadratic(T, usize, usize),
        }

        let mut terms: Vec<CompiledTerm<T>> = Vec::new();

        for monomial in &self.monomials {
            match &monomial.symbol2 {
                None => {
                    // 线性项
                    // Linear term
                    if let Some(&idx) = symbol_index.get(&monomial.symbol1) {
                        terms.push(CompiledTerm::Linear(monomial.coefficient.clone(), idx));
                    }
                }
                Some(sym2) => {
                    // 二次项
                    // Quadratic term
                    if let (Some(&idx1), Some(&idx2)) =
                        (symbol_index.get(&monomial.symbol1), symbol_index.get(sym2))
                    {
                        terms.push(CompiledTerm::Quadratic(
                            monomial.coefficient.clone(),
                            idx1,
                            idx2,
                        ));
                    }
                }
            }
        }

        let constant = self.constant.clone();

        Rc::new(move |values: &[T]| {
            let mut result = constant.clone();
            for term in &terms {
                match term {
                    CompiledTerm::Linear(c, i) => {
                        result = result + c.clone() * values[*i].clone();
                    }
                    CompiledTerm::Quadratic(c, i, j) => {
                        result = result + c.clone() * values[*i].clone() * values[*j].clone();
                    }
                }
            }
            result
        })
    }
}

impl<T> CompileGradient<T> for Quadratic<T>
where
    T: Clone
        + 'static
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + std::ops::Mul<Output = T>
        + num_traits::Zero
        + num_traits::One
        + PartialEq
        + PartialOrd,
{
    type GradFn = Rc<dyn Fn(&[T]) -> Vec<T>>;

    fn compile_gradient(&self, symbols: &[OwnedSymbol]) -> Self::GradFn {
        // 建立符号到索引的映射
        // Build symbol to index mapping
        let symbol_index: HashMap<OwnedSymbol, usize> = symbols
            .iter()
            .enumerate()
            .map(|(i, s)| (s.clone(), i))
            .collect();

        // 编译梯度项
        // Compile gradient terms
        // ∂/∂xi (a xi) = a
        // ∂/∂xi (a xi²) = 2a xi
        // ∂/∂xi (a xi xj) = a xj (当 i ≠ j 时)

        // 对于每个符号，收集其梯度贡献
        // For each symbol, collect its gradient contributions
        let n = symbols.len();
        let mut gradient_terms: Vec<Vec<(T, Option<usize>, Option<usize>)>> = vec![Vec::new(); n];

        for monomial in &self.monomials {
            let coef = &monomial.coefficient;

            match &monomial.symbol2 {
                None => {
                    // 线性项 a*xi 的梯度：∂/∂xi = a
                    // Linear term a*xi gradient: ∂/∂xi = a
                    if let Some(&idx) = symbol_index.get(&monomial.symbol1) {
                        gradient_terms[idx].push((coef.clone(), None, None));
                    }
                }
                Some(sym2) => {
                    // 二次项
                    // Quadratic term
                    if let (Some(&idx1), Some(&idx2)) =
                        (symbol_index.get(&monomial.symbol1), symbol_index.get(sym2))
                    {
                        if idx1 == idx2 {
                            // a*xi² 的梯度：∂/∂xi = 2a*xi
                            // Gradient of a*xi²: ∂/∂xi = 2a*xi
                            let two = T::one() + T::one();
                            gradient_terms[idx1].push((coef.clone() * two, Some(idx1), None));
                        } else {
                            // a*xi*xj 的梯度：∂/∂xi = a*xj, ∂/∂xj = a*xi
                            // Gradient of a*xi*xj: ∂/∂xi = a*xj, ∂/∂xj = a*xi
                            gradient_terms[idx1].push((coef.clone(), Some(idx2), None));
                            gradient_terms[idx2].push((coef.clone(), Some(idx1), None));
                        }
                    }
                }
            }
        }

        let constant_gradient: Vec<T> = vec![T::zero(); n];

        Rc::new(move |values: &[T]| {
            let mut gradients = constant_gradient.clone();

            for (i, terms) in gradient_terms.iter().enumerate() {
                for (coef, idx1, _idx2) in terms {
                    match idx1 {
                        None => {
                            gradients[i] = gradients[i].clone() + coef.clone();
                        }
                        Some(j) => {
                            gradients[i] = gradients[i].clone() + coef.clone() * values[*j].clone();
                        }
                    }
                }
            }

            gradients
        })
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol::symbol::{DynSymbol, SymbolDynId};
    use crate::symbol::{LinearMonomial, OwnedSymbol, QuadraticMonomial};
    use std::any::Any;

    /// 测试用的简单符号 / Simple symbol for testing
    #[derive(Debug, Clone)]
    struct TestSymbol {
        id: usize,
        name: String,
    }

    impl std::fmt::Display for TestSymbol {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.name)
        }
    }

    impl TestSymbol {
        fn new(name: &str, id: usize) -> Self {
            Self {
                id,
                name: name.to_string(),
            }
        }
    }

    impl DynSymbol for TestSymbol {
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

    fn make_symbol(name: &str, id: usize) -> OwnedSymbol {
        OwnedSymbol::new(TestSymbol::new(name, id))
    }

    #[test]
    fn test_linear_compile_eval() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);

        // 创建多项式：2x + 3y + 1
        // Create polynomial: 2x + 3y + 1
        let poly: Linear<f64> = Linear::new(
            vec![
                LinearMonomial::new(2.0, x.clone()),
                LinearMonomial::new(3.0, y.clone()),
            ],
            1.0,
        );

        let symbols = vec![x, y];
        let eval_fn = poly.compile_eval(&symbols);

        // 测试求值
        // Test evaluation
        let result = eval_fn(&[3.0_f64, 4.0_f64]); // x=3, y=4
        // 2*3 + 3*4 + 1 = 6 + 12 + 1 = 19
        assert!((result - 19.0_f64).abs() < 1e-10);

        let result2 = eval_fn(&[0.0_f64, 0.0_f64]); // x=0, y=0
        assert!((result2 - 1.0_f64).abs() < 1e-10); // 常数项
    }

    #[test]
    fn test_linear_compile_gradient() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);

        // 创建多项式：2x + 3y + 1
        // Create polynomial: 2x + 3y + 1
        let poly: Linear<f64> = Linear::new(
            vec![
                LinearMonomial::new(2.0, x.clone()),
                LinearMonomial::new(3.0, y.clone()),
            ],
            1.0,
        );

        let symbols = vec![x, y];
        let grad_fn = poly.compile_gradient(&symbols);

        // 梯度应该是常数：[2.0, 3.0]
        // Gradient should be constant: [2.0, 3.0]
        let gradient = grad_fn(&[10.0_f64, 20.0_f64]);
        assert!((gradient[0] - 2.0_f64).abs() < 1e-10);
        assert!((gradient[1] - 3.0_f64).abs() < 1e-10);
    }

    #[test]
    fn test_quadratic_compile_eval() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);

        // 创建多项式：x² + 2xy + y² + 1
        // Create polynomial: x² + 2xy + y² + 1
        let poly: Quadratic<f64> = Quadratic::new(
            vec![
                QuadraticMonomial::quadratic(1.0, x.clone(), x.clone()), // x²
                QuadraticMonomial::quadratic(2.0, x.clone(), y.clone()), // 2xy
                QuadraticMonomial::quadratic(1.0, y.clone(), y.clone()), // y²
            ],
            1.0,
        );

        let symbols = vec![x, y];
        let eval_fn = poly.compile_eval(&symbols);

        // 测试求值：(x+y)² + 1
        // Test evaluation: (x+y)² + 1
        let result = eval_fn(&[2.0_f64, 3.0_f64]); // x=2, y=3
        assert!((result - 26.0_f64).abs() < 1e-10); // 4 + 12 + 9 + 1 = 26
    }

    #[test]
    fn test_quadratic_compile_gradient() {
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);

        // 创建多项式：x² + 2xy + y²
        // Create polynomial: x² + 2xy + y²
        let poly: Quadratic<f64> = Quadratic::new(
            vec![
                QuadraticMonomial::quadratic(1.0, x.clone(), x.clone()), // x²
                QuadraticMonomial::quadratic(2.0, x.clone(), y.clone()), // 2xy
                QuadraticMonomial::quadratic(1.0, y.clone(), y.clone()), // y²
            ],
            0.0,
        );

        let symbols = vec![x, y];
        let grad_fn = poly.compile_gradient(&symbols);

        // 梯度：∂f/∂x = 2x + 2y, ∂f/∂y = 2x + 2y
        // Gradient: ∂f/∂x = 2x + 2y, ∂f/∂y = 2x + 2y
        let gradient = grad_fn(&[2.0_f64, 3.0_f64]);
        assert!((gradient[0] - 10.0_f64).abs() < 1e-10); // 2*2 + 2*3 = 10
        assert!((gradient[1] - 10.0_f64).abs() < 1e-10); // 2*2 + 2*3 = 10

        let gradient2 = grad_fn(&[1.0_f64, 1.0_f64]);
        assert!((gradient2[0] - 4.0_f64).abs() < 1e-10); // 2*1 + 2*1 = 4
        assert!((gradient2[1] - 4.0_f64).abs() < 1e-10);
    }

    #[test]
    fn test_compiled_eval_batch() {
        // 测试批量求值正确性 / Test batch evaluation correctness
        let x = make_symbol("x", 1);
        let y = make_symbol("y", 2);
        let z = make_symbol("z", 3);

        // 创建多项式：2x + 3y + 4z + 5
        // Create polynomial: 2x + 3y + 4z + 5
        let poly: Linear<f64> = Linear::new(
            vec![
                LinearMonomial::new(2.0, x.clone()),
                LinearMonomial::new(3.0, y.clone()),
                LinearMonomial::new(4.0, z.clone()),
            ],
            5.0,
        );

        let symbols = vec![x, y, z];
        let eval_fn = poly.compile_eval(&symbols);

        // 批量测试
        // Batch test
        for i in 0..100 {
            let x_val = i as f64;
            let y_val = (i + 1) as f64;
            let z_val = (i + 2) as f64;

            let expected = 2.0_f64 * x_val + 3.0_f64 * y_val + 4.0_f64 * z_val + 5.0_f64;
            let result = eval_fn(&[x_val, y_val, z_val]);

            assert!((result - expected).abs() < 1e-10_f64);
        }
    }
}