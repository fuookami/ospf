# expression

:us: [English](README.md) | :cn: 简体中文

运行时表达式系统，用于构建和求值动态表达式。本模块提供灵活的表达式 AST 及求值能力。

## 核心类型

| 类型 | 描述 |
|------|------|
| `PropertyPath` | 属性路径，用于引用嵌套属性 |
| `PathSymbol` | 由属性路径支持的符号 |
| `ScalarExpression<T>` | 标量表达式 AST（常量、引用、运算） |
| `BooleanExpression<T>` | 布尔表达式 AST（比较、逻辑运算） |
| `ExpressionValue` | 运行时值类型（空值、布尔、数字、字符串） |
| `MapEvaluationContext` | 基于 HashMap 的求值上下文 |

## 标量表达式组件

| 变体 | 描述 |
|------|------|
| `Constant(T)` | 标量常量值 |
| `Reference(PropertyPath)` | 属性路径引用 |
| `SymbolReference(OwnedSymbol)` | 符号引用 |
| `Unary { operator, operand }` | 一元运算（取负、绝对值） |
| `Binary { operator, left, right }` | 二元运算（加、减、乘、除、幂） |
| `Function { name, arguments }` | 函数调用 |
| `Custom { payload, description }` | 自定义表达式 |

## 布尔表达式组件

| 变体 | 描述 |
|------|------|
| `Constant(Trivalent)` | 布尔常量（真、假、未知） |
| `Comparison { operator, left, right }` | 比较表达式 |
| `In { value, candidates, negated }` | 集合成员判断 |
| `PatternMatch { value, pattern, mode, negated }` | 模式匹配 |
| `NullCheck { path, null_check_type }` | 空值检查 |
| `And(operands)` | 逻辑与 |
| `Or(operands)` | 逻辑或 |
| `Not(operand)` | 逻辑非 |

## 运算符

| 枚举 | 值 |
|------|------|
| `UnaryOperator` | Negate, Positive, Abs |
| `BinaryOperator` | Add, Subtract, Multiply, Divide, Modulo, Power |
| `ComparisonOperator` | Eq, Ne, Lt, Le, Gt, Ge |
| `BooleanOperator` | And, Or, Not |
| `PatternMatchMode` | Exact, Prefix, Suffix, Contains, Like, Regex |

## 使用示例

```rust
use ospf_rust_math::symbol::expression::{ScalarExpression, BooleanExpression, PropertyPath};
use ospf_rust_math::symbol::expression::{eq, and, path, MapEvaluationContext};

// 创建标量引用表达式
let expr = ScalarExpression::<f64>::reference(PropertyPath::parse("user.age"));

// 创建比较表达式
let condition = eq("user.age", 18.0);

// 创建复合布尔表达式
let compound = and([eq("user.status", "active"), eq("user.age", 18.0)]);

// 求值表达式
let mut ctx = MapEvaluationContext::default();
ctx.insert("user.age", 25.0);
ctx.insert("user.status", "active");
let result = condition.evaluate_with(&ctx);
```

## 许可证

本项目采用 MIT 许可证。
