# 二次模型中的线性函数组合

线性模型函数与二次模型函数的用途不同。线性输入函数生成的行通常仍是线性的，因此可以在二次模型中复用。接收二次多项式输入的函数则是在非线性输入上组合相同运算，可能引入二次等式。

## 复用线性输入函数

线性表达式可作为二次目标和约束的组成部分。若输入为线性表达式且生成约束仍为线性，`MinFunction`、`MaxFunction`、`AbsFunction`、`MaskingFunction`、`IfFunction` 和精确组合函数等线性函数可直接复用。它们仍遵循普通 MILP 的精确性与有限界要求，包括[基数、正部、区间裁剪、互补和距离函数](../linear-functional/composite-cardinality)、[顺序统计](../linear-functional/order-statistics)及[CVaR](../linear-functional/risk-global-constraints)。Rust 中，涉及辅助变量的表达式应通过所属符号提供的注册 solver-index 映射生成。

`ProductFunction` 将两个线性表达式相乘并返回二次表达式，不创建结果变量，也不做线性化。需要支持二次表达式的求解器。

`QuadraticLinearFunction` 通过等式将二次表达式绑定到可正可负的实辅助变量，使该值能复用于线性输入函数接口。此等式本身是二次的，因此模型为 MIQCP，且可能非凸。

## 对二次输入组合函数

`QuadraticAbsFunction`、`QuadraticMaxFunction`、`QuadraticMinMaxFunction`、`QuadraticMaskingFunction` 和 `QuadraticIfFunction` 等二次对应函数可对二次输入多项式执行熟悉的函数语义。每个实际含二次项的输入都由可正可负的桥接变量及精确二次等式表示；仿射输入无需桥接。之后函数内部的线性化约束使用桥接值构造。

这样可以避免把两个二次表达式继续相乘，产生三次或四次项。模型仍然不是线性的：它会包含 MIQCP 约束，应使用支持所需二次约束的求解器。整个模型决定凸性与性能。

## 模型保证

| 构造 | 保证 | 求解器/模型类别 |
|---|---|---|
| 有界整数乘积或精确选择器 | 对任意目标方向均精确 | MILP |
| 连续因子的 McCormick 包络 | 在声明的矩形域上为凸包松弛 | LP/MILP 松弛 |
| 非线性分段函数 | 断点间按给定点线性插值；无自动误差界 | 含分段选择变量的 MILP |
| CVaR 精确候选枚举 | 对任意目标方向均精确 | MILP |
| CVaR 上图 | 最小化时在最优解处收紧；结果上界精确表达 `CVaR <= limit` 的可行性，但不强制数值相等 | LP/MILP 上图 |
| 二次桥接等式 | 与二次输入精确相等 | MIQCP，可能非凸 |

应按所需语义选择构造。松弛可用于界或分解，但不能称为精确乘积；PWL 模型近似原曲线；CVaR 上图表达式只有在被最小化时才会在最优解处收紧，结果上界保留精确的风险可行性判断，但不固定辅助表达式等于 CVaR；二次桥接保留等式语义但会改变求解器类别。

参见[乘积表达式](./product)、[二次输入函数](./quadratic-abs)，以及线性模型中的[组合函数、基数与距离](../linear-functional/composite-cardinality)、[顺序统计](../linear-functional/order-statistics)、[松弛区间](../linear-functional/slack-range)、[乘积与选择](../linear-functional/products-selection)、[非线性近似与阶梯计费](../linear-functional/nonlinear-tariffs)和[风险度量与全局约束](../linear-functional/risk-global-constraints)。
