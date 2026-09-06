# expression

:us: [English](README.md) | :cn: 简体中文

运行时表达式系统，用于构建、求值、解析和序列化动态表达式。本模块提供灵活的表达式 AST，支持条件分支、函数调用、属性路径引用与三值逻辑。

## 模块结构

```
expression/
├── mod.rs           # 模块注册 + 布尔解析器/序列化/测试
├── property_path.rs # PropertyPath 属性路径 + PathSymbol 路径符号
├── operators.rs     # 操作符枚举（一元/二元/比较/模式匹配/布尔/空值检查）
├── value.rs         # ExpressionValue 运行时值类型
├── scalar.rs        # ScalarExpression<T> 标量表达式 AST
├── boolean.rs       # BooleanExpression<T> 布尔表达式 AST
├── dsl.rs           # DSL trait + PathBuilder + 便捷构造函数
├── evaluation.rs    # EvaluationContext 求值上下文 + 求值函数
├── normalize.rs     # 布尔表达式规范化 + structural_key
├── transform.rs     # 标量/布尔 AST 后序变换
├── math_functions.rs # ScalarFunctionEvaluator trait + MathFunctionEvaluator（17 个 math.* 函数）
└── scalar_parser.rs # 标量表达式解析器（可选，需要 "parser" feature）
```

## 核心类型

| 类型 | 描述 |
|------|------|
| `PropertyPath` | 属性路径（如 `user.address.city`），支持分段、父子路径、子路径判断 |
| `PathSymbol` | 路径符号，桥接 `PropertyPath` 与 `DynSymbol` |
| `ExpressionValue` | 运行时值枚举（`Null` / `Boolean` / `Number` / `String`） |
| `ScalarExpression<T>` | 标量表达式 AST（常量、引用、运算、函数、条件、布尔包装） |
| `BooleanExpression<T>` | 布尔表达式 AST（比较、In、模式匹配、空值检查、逻辑运算） |
| `PathBuilder<T>` | 路径构建器，链式构造引用与比较表达式 |
| `MapEvaluationContext` | 基于 HashMap 的求值上下文 |
| `EmptyEvaluationContext` | 空求值上下文（所有路径均未定义） |

## 标量表达式变体

| 变体 | 描述 |
|------|------|
| `Constant(T)` | 标量常量值 |
| `Reference(PropertyPath)` | 属性路径引用 |
| `SymbolReference(OwnedSymbol)` | 动态符号引用 |
| `Unary { operator, operand }` | 一元运算（取负、正号、绝对值） |
| `Binary { operator, left, right }` | 二元运算（加、减、乘、除、取模、幂） |
| `Function { name, arguments }` | 函数调用（如 `abs`、`math.sqrt`） |
| `Conditional { condition, then_branch, else_branch }` | 条件表达式（`if/then/else` 或三元 `?:`） |
| `Boolean(Box<BooleanExpression<T>>)` | 布尔包装表达式（布尔作为标量值） |
| `Custom { payload, description }` | 自定义表达式 |

## 布尔表达式变体

| 变体 | 描述 |
|------|------|
| `Constant(Trivalent)` | 三值逻辑常量（真、假、未知） |
| `Comparison { operator, left, right }` | 比较表达式 |
| `In { value, candidates, negated }` | 集合成员判断 |
| `PatternMatch { value, pattern, mode, negated }` | 模式匹配（like、regex、前缀/后缀/包含） |
| `NullCheck { path, null_check_type }` | 空值检查（`is null` / `is not null`） |
| `And(operands)` | 逻辑与（支持多操作数） |
| `Or(operands)` | 逻辑或（支持多操作数） |
| `Not(operand)` | 逻辑非 |
| `Custom { payload, description }` | 自定义布尔表达式 |

## 运算符

| 枚举 | 值 |
|------|------|
| `UnaryOperator` | `Negate`、`Positive`、`Abs` |
| `BinaryOperator` | `Add`、`Subtract`、`Multiply`、`Divide`、`Modulo`、`Power` |
| `ComparisonOperator` | `Eq`、`Ne`、`Lt`、`Le`、`Gt`、`Ge` |
| `PatternMatchMode` | `Like`、`Exact`、`Prefix`、`Suffix`、`Contains`、`Regex` |
| `BooleanOperator` | `And`、`Or` |
| `NullCheckType` | `IsNull`、`IsNotNull` |

## 求值

求值通过 `EvaluationContext` 提供属性路径的值，可注入 `ScalarFunctionEvaluator` 支持自定义函数。

| 函数 | 描述 |
|------|------|
| `evaluate_boolean(expr, ctx)` | 求值布尔表达式，返回三值逻辑结果 |
| `evaluate_boolean_with_evaluator(expr, ctx, evaluator)` | 求值布尔表达式（带函数求值器） |
| `evaluate_scalar_expression(expr, ctx, evaluator)` | 求值标量表达式（带函数求值器） |

`MathFunctionEvaluator` 内置 17 个 math.* 函数：`sqrt`、`pow`、`log`、`log10`、`exp`、`sin`、`cos`、`tan`、`asin`、`acos`、`atan`、`floor`、`ceil`、`round`、`max`、`min`、`abs`。

## 结构保持的表达式变换

`ScalarExpressionTransform` 和 `BooleanExpressionTransform` 提供结构保持的后序重写。使用 `transform_scalars` 改写布尔树中的标量节点，使用 `transform_booleans` 改写布尔节点。回调接收子节点重建后的当前节点，包括 `Conditional` 和 `Boolean` 标量分支中的子树。

```rust
use ospf_rust_math::symbol::expression::{
    BooleanExpressionTransform, ExpressionValue, ScalarExpression,
};

let predicate = BooleanExpression::eq(
    ScalarExpression::reference("age"),
    ScalarExpression::constant(ExpressionValue::from(18)),
);

let rewritten = predicate.transform_scalars(|scalar| match scalar {
    ScalarExpression::Constant(ExpressionValue::Number(value)) => {
        ScalarExpression::constant(ExpressionValue::Number(value + 1.0))
    }
    scalar => scalar,
});
```

这些工具只重建共享 AST，不求值，也不修改原表达式。`transform_scalar_expression` 还会遍历标量条件表达式和布尔包装表达式中嵌套的布尔分支。

## 布尔规范化

| 函数 | 描述 |
|------|------|
| `flatten_boolean_expression` | 扁平化嵌套的 And/Or |
| `constant_fold_boolean_expression` | 常量折叠（消除 `True`/`False` 操作数） |
| `deduplicate_boolean_expression` | 去重相同操作数 |
| `eliminate_double_negation` | 消除双重否定 `Not(Not(x))` |
| `apply_de_morgan` | 应用德摩根定律 |
| `sort_boolean_operands` | 排序操作数以规范化 |
| `normalize_boolean_expression` | 综合规范化（含配置） |

## 使用示例

### 构造与求值

```rust
use ospf_rust_math::symbol::expression::{
    BooleanExpression, ExpressionValue,
    eq, and, MapEvaluationContext, EvaluateBoolean,
};

// 创建比较表达式：user.age == 18
let condition: BooleanExpression<ExpressionValue> = eq("user.age", 18.0);

// 创建复合布尔表达式：user.status == "active" && user.age == 18
let compound: BooleanExpression<ExpressionValue> = and([
    eq("user.status", "active"),
    eq("user.age", 18.0),
]);

// 求值上下文
let mut ctx = MapEvaluationContext::default();
ctx.insert("user.age", ExpressionValue::Number(25.0));
ctx.insert("user.status", ExpressionValue::String("active".to_string()));

// 求值（25 != 18，结果为假）
let result = condition.evaluate_with(&ctx);
assert_eq!(result, ospf_rust_math::Trivalent::False);
```

### 标量表达式与条件

```rust
use ospf_rust_math::symbol::expression::{
    ScalarExpression, BooleanExpression, ExpressionValue, MapEvaluationContext,
    evaluate_scalar_expression, MathFunctionEvaluator,
};

let x = ScalarExpression::<ExpressionValue>::reference("x");

// 构造标量表达式：x * 2 + 3
let expr = ScalarExpression::add_expr(
    ScalarExpression::multiply_expr(x.clone(), 2.0.into()),
    3.0.into(),
);

// 构造条件表达式：if x > 0 then x else 0
let condition = BooleanExpression::gt(x.clone(), 0.0.into());
let conditional = ScalarExpression::conditional(condition, x, 0.0.into());

let ctx = MapEvaluationContext::from_string_map([
    ("x", ExpressionValue::Number(5.0)),
]);

let result = evaluate_scalar_expression(&expr, &ctx, &MathFunctionEvaluator);
assert_eq!(result, Some(ExpressionValue::Number(13.0)));
```

### 表达式解析（需要 "parser" feature）

```rust
use ospf_rust_math::symbol::expression::{
    parse_scalar_expression, evaluate_scalar_expression,
    MapEvaluationContext, MathFunctionEvaluator, ExpressionValue,
};

// 解析标量表达式字符串
let expr = parse_scalar_expression("if math.sqrt(x) > 2 then x else 0 fi").unwrap();

let ctx = MapEvaluationContext::from_string_map([
    ("x", ExpressionValue::Number(16.0)),
]);

let result = evaluate_scalar_expression(&expr, &ctx, &MathFunctionEvaluator);
assert_eq!(result, Some(ExpressionValue::Number(16.0)));
```

支持的解析语法：

- 算术：`+`、`-`、`*`、`/`、`%`、`^`、`**`
- 比较：`>`、`<`、`>=`、`<=`、`==`、`!=`、`<>`
- 逻辑：`&&`、`||`、`!`、`and`、`or`、`not`
- 条件：`? :` 三元、`if/then/else/fi`
- 函数：`name(args)`、`math.sqrt`、`math.pow`、`math.PI`、`math.E` 等
- 字面量：数字、字符串、`true`、`false`、`null`

### 序列化（需要 "serde" feature）

```rust
use ospf_rust_math::symbol::expression::{
    ScalarExpression, ExpressionValue, scalar_expression_from_json,
};

// AST -> JSON 字符串
let expr = ScalarExpression::<ExpressionValue>::constant(ExpressionValue::Number(42.0));
let json = expr.to_json_string()?;

// JSON -> AST
let restored: ScalarExpression<ExpressionValue> = scalar_expression_from_json(&json)?;
```

`BooleanExpression<T>` 同样提供 `to_json_string()` / `to_json_string_pretty()`，以及 `boolean_expression_from_json()` 反序列化入口。

## Feature Flags

| Flag | 描述 |
|------|------|
| `parser` | 启用标量表达式解析（`parse_scalar_expression`） |
| `serde` | 启用 JSON 序列化/反序列化 |

## 许可证

本项目采用 MIT 许可证。
