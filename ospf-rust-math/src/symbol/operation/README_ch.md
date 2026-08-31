# operation

:us: English | :cn: [简体中文](README_ch.md)

符号运算模块，提供多项式类型转换、求值、微分和输出功能。

## 核心 Trait

| Trait | 描述 |
|-------|------|
| `ToLinear` | 转换多项式为线性形式 |
| `ToQuadratic` | 转换多项式为二次形式 |
| `ToCanonical` | 转换多项式为标准形式 |
| `Evaluate` | 使用值提供器求值多项式 |
| `EvaluateOrdered` | 使用有序变量值求值 |
| `Differentiate` | 计算符号一阶导数 |
| `SecondOrderDifferentiate` | 计算符号二阶导数 |
| `ToMatrixForm` | 转换为矩阵表示 |
| `ToLaTeX` | 生成 LaTeX 输出 |
| `CompileEval` | 编译以快速求值 |
| `CompileGradient` | 编译以计算梯度 |

## 矩阵形式

| 类型 | 描述 |
|------|------|
| `LinearMatrixForm` | 线性多项式表示为 `c^T * x + b` |
| `QuadraticMatrixForm` | 二次多项式表示为 `x^T * Q * x + c^T * x + e` |

## 运算功能

### 类型转换
- `ToLinear::to_linear()` - 提取线性项（丢弃高阶项）
- `ToQuadratic::to_quadratic()` - 提取二次项（丢弃高阶项）
- `ToCanonical::to_canonical()` - 转换为广义标准形式

### 求值
- `Evaluate::evaluate(&self, provider)` - 使用变量值求值
- `EvaluateOrdered::evaluate_ordered(&self, values)` - 使用有序值求值

### 微分
- `Differentiate::differentiate(&self, symbol)` - 对符号计算导数
- `SecondOrderDifferentiate::second_order_differentiate(&self, s1, s2)` - 计算二阶导数

### 矩阵形式
- `ToMatrixForm::to_matrix_form(&self, symbols)` - 提取系数矩阵和向量

### LaTeX 输出
- `ToLaTeX::to_latex(&self)` - 基础 LaTeX 输出
- `ToLaTeX::to_latex_with_options(&self, options)` - 使用 `LatexOptions` 自定义 LaTeX

### 编译
- `CompileEval::compile_eval(&self, symbols)` - 创建快速求值函数
- `CompileGradient::compile_gradient(&self, symbols)` - 创建梯度计算函数

## 许可证

本项目采用 MIT 许可证。