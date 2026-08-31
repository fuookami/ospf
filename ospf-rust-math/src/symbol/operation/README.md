# operation

:us: English | :cn: [简体中文](README_ch.md)

Symbolic operation module providing polynomial type conversion, evaluation, differentiation, and output capabilities.

## Key Traits

| Trait | Description |
|-------|-------------|
| `ToLinear` | Convert polynomial to linear form |
| `ToQuadratic` | Convert polynomial to quadratic form |
| `ToCanonical` | Convert polynomial to canonical form |
| `Evaluate` | Evaluate polynomial with value provider |
| `EvaluateOrdered` | Evaluate with ordered variable values |
| `Differentiate` | Compute symbolic first derivative |
| `SecondOrderDifferentiate` | Compute symbolic second derivative |
| `ToMatrixForm` | Convert to matrix representation |
| `ToLaTeX` | Generate LaTeX output |
| `CompileEval` | Compile for fast evaluation |
| `CompileGradient` | Compile for gradient computation |

## Matrix Forms

| Type | Description |
|------|-------------|
| `LinearMatrixForm` | Linear polynomial as `c^T * x + b` |
| `QuadraticMatrixForm` | Quadratic polynomial as `x^T * Q * x + c^T * x + e` |

## Operations

### Type Conversion
- `ToLinear::to_linear()` - Extract linear terms (drops higher-order terms)
- `ToQuadratic::to_quadratic()` - Extract quadratic terms (drops higher-order terms)
- `ToCanonical::to_canonical()` - Convert to general canonical form

### Evaluation
- `Evaluate::evaluate(&self, provider)` - Evaluate with variable values
- `EvaluateOrdered::evaluate_ordered(&self, values)` - Evaluate with ordered values

### Differentiation
- `Differentiate::differentiate(&self, symbol)` - Compute derivative w.r.t. symbol
- `SecondOrderDifferentiate::second_order_differentiate(&self, s1, s2)` - Compute second derivative

### Matrix Form
- `ToMatrixForm::to_matrix_form(&self, symbols)` - Extract coefficient matrix and vector

### LaTeX Output
- `ToLaTeX::to_latex(&self)` - Basic LaTeX output
- `ToLaTeX::to_latex_with_options(&self, options)` - Customized LaTeX with `LatexOptions`

### Compilation
- `CompileEval::compile_eval(&self, symbols)` - Create fast evaluation function
- `CompileGradient::compile_gradient(&self, symbols)` - Create gradient computation function

## License

This project is licensed under the MIT License.
