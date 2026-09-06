# expression

:us: English | :cn: [简体中文](README_ch.md)

Runtime expression system for building, evaluating, parsing, and serializing dynamic expressions. This module provides a flexible expression AST supporting conditional branching, function calls, property path references, and three-valued logic.

## Module Structure

```
expression/
├── mod.rs           # Module registration + boolean parser/serialization/tests
├── property_path.rs # PropertyPath + PathSymbol
├── operators.rs     # Operator enums (Unary/Binary/Comparison/PatternMatch/Boolean/NullCheck)
├── value.rs         # ExpressionValue runtime value type
├── scalar.rs        # ScalarExpression<T> scalar expression AST
├── boolean.rs       # BooleanExpression<T> boolean expression AST
├── dsl.rs           # DSL traits + PathBuilder + convenience constructors
├── evaluation.rs    # EvaluationContext + evaluation functions
├── normalize.rs     # Boolean normalization + structural_key
├── transform.rs     # Post-order scalar/boolean AST transforms
├── math_functions.rs # ScalarFunctionEvaluator trait + MathFunctionEvaluator (17 math.* functions)
└── scalar_parser.rs # Scalar expression parser (optional, requires "parser" feature)
```

## Key Types

| Type | Description |
|------|-------------|
| `PropertyPath` | Property path (e.g., `user.address.city`), with segments, parent/child paths, sub-path checks |
| `PathSymbol` | Path symbol bridging `PropertyPath` and `DynSymbol` |
| `ExpressionValue` | Runtime value enum (`Null` / `Boolean` / `Number` / `String`) |
| `ScalarExpression<T>` | Scalar expression AST (constant, reference, operations, function, conditional, boolean wrapper) |
| `BooleanExpression<T>` | Boolean expression AST (comparison, In, pattern match, null check, logical ops) |
| `PathBuilder<T>` | Path builder for chaining reference and comparison construction |
| `MapEvaluationContext` | HashMap-based evaluation context |
| `EmptyEvaluationContext` | Empty evaluation context (all paths undefined) |

## Scalar Expression Variants

| Variant | Description |
|---------|-------------|
| `Constant(T)` | Scalar constant value |
| `Reference(PropertyPath)` | Property path reference |
| `SymbolReference(OwnedSymbol)` | Dynamic symbol reference |
| `Unary { operator, operand }` | Unary operation (negation, positive, absolute value) |
| `Binary { operator, left, right }` | Binary operation (add, subtract, multiply, divide, modulo, power) |
| `Function { name, arguments }` | Function call (e.g., `abs`, `math.sqrt`) |
| `Conditional { condition, then_branch, else_branch }` | Conditional expression (`if/then/else` or ternary `?:`) |
| `Boolean(Box<BooleanExpression<T>>)` | Boolean wrapper expression (boolean as scalar value) |
| `Custom { payload, description }` | Custom expression |

## Boolean Expression Variants

| Variant | Description |
|---------|-------------|
| `Constant(Trivalent)` | Three-valued logic constant (true, false, unknown) |
| `Comparison { operator, left, right }` | Comparison expression |
| `In { value, candidates, negated }` | Set membership check |
| `PatternMatch { value, pattern, mode, negated }` | Pattern matching (like, regex, prefix/suffix/contains) |
| `NullCheck { path, null_check_type }` | Null check (`is null` / `is not null`) |
| `And(operands)` | Logical AND (supports multiple operands) |
| `Or(operands)` | Logical OR (supports multiple operands) |
| `Not(operand)` | Logical NOT |
| `Custom { payload, description }` | Custom boolean expression |

## Operators

| Enum | Values |
|------|--------|
| `UnaryOperator` | `Negate`, `Positive`, `Abs` |
| `BinaryOperator` | `Add`, `Subtract`, `Multiply`, `Divide`, `Modulo`, `Power` |
| `ComparisonOperator` | `Eq`, `Ne`, `Lt`, `Le`, `Gt`, `Ge` |
| `PatternMatchMode` | `Like`, `Exact`, `Prefix`, `Suffix`, `Contains`, `Regex` |
| `BooleanOperator` | `And`, `Or` |
| `NullCheckType` | `IsNull`, `IsNotNull` |

## Evaluation

Evaluation is driven by `EvaluationContext`, which provides values for property paths. A `ScalarFunctionEvaluator` can be injected to support custom functions.

| Function | Description |
|----------|-------------|
| `evaluate_boolean(expr, ctx)` | Evaluate a boolean expression, returning a three-valued result |
| `evaluate_boolean_with_evaluator(expr, ctx, evaluator)` | Evaluate a boolean expression (with function evaluator) |
| `evaluate_scalar_expression(expr, ctx, evaluator)` | Evaluate a scalar expression (with function evaluator) |

`MathFunctionEvaluator` ships 17 math.* functions: `sqrt`, `pow`, `log`, `log10`, `exp`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `floor`, `ceil`, `round`, `max`, `min`, `abs`.

## Structural Expression Transforms

`ScalarExpressionTransform` and `BooleanExpressionTransform` provide structure-preserving, post-order rewrites. Use `transform_scalars` to rewrite scalar nodes inside a boolean tree and `transform_booleans` to rewrite boolean nodes. The callbacks receive rebuilt child nodes, including branches inside `Conditional` and `Boolean` scalar variants.

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

The helpers rebuild the shared AST without evaluating or mutating the original expression. `transform_scalar_expression` also traverses boolean branches nested in scalar conditionals and boolean wrappers.

## Boolean Normalization

| Function | Description |
|----------|-------------|
| `flatten_boolean_expression` | Flatten nested And/Or |
| `constant_fold_boolean_expression` | Constant folding (eliminate `True`/`False` operands) |
| `deduplicate_boolean_expression` | Deduplicate identical operands |
| `eliminate_double_negation` | Eliminate double negation `Not(Not(x))` |
| `apply_de_morgan` | Apply De Morgan's laws |
| `sort_boolean_operands` | Sort operands for canonicalization |
| `normalize_boolean_expression` | Comprehensive normalization (configurable) |

## Usage

### Construction and Evaluation

```rust
use ospf_rust_math::symbol::expression::{
    BooleanExpression, ExpressionValue,
    eq, and, MapEvaluationContext, EvaluateBoolean,
};

// Create comparison: user.age == 18
let condition: BooleanExpression<ExpressionValue> = eq("user.age", 18.0);

// Create compound boolean: user.status == "active" && user.age == 18
let compound: BooleanExpression<ExpressionValue> = and([
    eq("user.status", "active"),
    eq("user.age", 18.0),
]);

// Evaluation context
let mut ctx = MapEvaluationContext::default();
ctx.insert("user.age", ExpressionValue::Number(25.0));
ctx.insert("user.status", ExpressionValue::String("active".to_string()));

// Evaluate (25 != 18, result is false)
let result = condition.evaluate_with(&ctx);
assert_eq!(result, ospf_rust_math::Trivalent::False);
```

### Scalar Expressions and Conditionals

```rust
use ospf_rust_math::symbol::expression::{
    ScalarExpression, BooleanExpression, ExpressionValue, MapEvaluationContext,
    evaluate_scalar_expression, MathFunctionEvaluator,
};

let x = ScalarExpression::<ExpressionValue>::reference("x");

// Build scalar expression: x * 2 + 3
let expr = ScalarExpression::add_expr(
    ScalarExpression::multiply_expr(x.clone(), 2.0.into()),
    3.0.into(),
);

// Build conditional: if x > 0 then x else 0
let condition = BooleanExpression::gt(x.clone(), 0.0.into());
let conditional = ScalarExpression::conditional(condition, x, 0.0.into());

let ctx = MapEvaluationContext::from_string_map([
    ("x", ExpressionValue::Number(5.0)),
]);

let result = evaluate_scalar_expression(&expr, &ctx, &MathFunctionEvaluator);
assert_eq!(result, Some(ExpressionValue::Number(13.0)));
```

### Expression Parsing (requires "parser" feature)

```rust
use ospf_rust_math::symbol::expression::{
    parse_scalar_expression, evaluate_scalar_expression,
    MapEvaluationContext, MathFunctionEvaluator, ExpressionValue,
};

// Parse scalar expression string
let expr = parse_scalar_expression("if math.sqrt(x) > 2 then x else 0 fi").unwrap();

let ctx = MapEvaluationContext::from_string_map([
    ("x", ExpressionValue::Number(16.0)),
]);

let result = evaluate_scalar_expression(&expr, &ctx, &MathFunctionEvaluator);
assert_eq!(result, Some(ExpressionValue::Number(16.0)));
```

Supported parsing syntax:

- Arithmetic: `+`, `-`, `*`, `/`, `%`, `^`, `**`
- Comparison: `>`, `<`, `>=`, `<=`, `==`, `!=`, `<>`
- Logical: `&&`, `||`, `!`, `and`, `or`, `not`
- Conditional: `? :` ternary, `if/then/else/fi`
- Functions: `name(args)`, `math.sqrt`, `math.pow`, `math.PI`, `math.E`, etc.
- Literals: numbers, strings, `true`, `false`, `null`

### Serialization (requires "serde" feature)

```rust
use ospf_rust_math::symbol::expression::{
    ScalarExpression, ExpressionValue, scalar_expression_from_json,
};

// AST -> JSON string
let expr = ScalarExpression::<ExpressionValue>::constant(ExpressionValue::Number(42.0));
let json = expr.to_json_string()?;

// JSON -> AST
let restored: ScalarExpression<ExpressionValue> = scalar_expression_from_json(&json)?;
```

`BooleanExpression<T>` likewise provides `to_json_string()` / `to_json_string_pretty()`, plus the `boolean_expression_from_json()` deserialization entry point.

## Feature Flags

| Flag | Description |
|------|-------------|
| `parser` | Enable scalar expression parsing (`parse_scalar_expression`) |
| `serde` | Enable JSON serialization/deserialization |

## License

This project is licensed under the MIT License.
