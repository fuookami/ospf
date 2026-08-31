# expression

:us: English | :cn: [简体中文](README_ch.md)

Runtime expression system for building and evaluating dynamic expressions. This module provides a flexible expression AST with evaluation capabilities.

## Key Types

| Type | Description |
|------|-------------|
| `PropertyPath` | Property path for referencing nested attributes |
| `PathSymbol` | Symbol backed by a property path |
| `ScalarExpression<T>` | Scalar expression AST (constants, references, operations) |
| `BooleanExpression<T>` | Boolean expression AST (comparisons, logical ops) |
| `ExpressionValue` | Runtime value type (null, boolean, number, string) |
| `MapEvaluationContext` | HashMap-based evaluation context |

## Scalar Expression Components

| Variant | Description |
|---------|-------------|
| `Constant(T)` | Scalar constant value |
| `Reference(PropertyPath)` | Property path reference |
| `SymbolReference(OwnedSymbol)` | Symbol reference |
| `Unary { operator, operand }` | Unary operation (negate, abs) |
| `Binary { operator, left, right }` | Binary operation (add, subtract, multiply, divide, power) |
| `Function { name, arguments }` | Function call |
| `Custom { payload, description }` | Custom expression |

## Boolean Expression Components

| Variant | Description |
|---------|-------------|
| `Constant(Trivalent)` | Boolean constant (true, false, unknown) |
| `Comparison { operator, left, right }` | Comparison expression |
| `In { value, candidates, negated }` | Set membership check |
| `PatternMatch { value, pattern, mode, negated }` | Pattern matching |
| `NullCheck { path, null_check_type }` | Null check |
| `And(operands)` | Logical AND |
| `Or(operands)` | Logical OR |
| `Not(operand)` | Logical NOT |

## Operators

| Enum | Values |
|------|--------|
| `UnaryOperator` | Negate, Positive, Abs |
| `BinaryOperator` | Add, Subtract, Multiply, Divide, Modulo, Power |
| `ComparisonOperator` | Eq, Ne, Lt, Le, Gt, Ge |
| `BooleanOperator` | And, Or, Not |
| `PatternMatchMode` | Exact, Prefix, Suffix, Contains, Like, Regex |

## Usage

```rust
use ospf_rust_math::symbol::expression::{ScalarExpression, BooleanExpression, PropertyPath};
use ospf_rust_math::symbol::expression::{eq, and, path, MapEvaluationContext};

// Create a scalar reference expression
let expr = ScalarExpression::<f64>::reference(PropertyPath::parse("user.age"));

// Create a comparison expression
let condition = eq("user.age", 18.0);

// Create a compound boolean expression
let compound = and([eq("user.status", "active"), eq("user.age", 18.0)]);

// Evaluate expression
let mut ctx = MapEvaluationContext::default();
ctx.insert("user.age", 25.0);
ctx.insert("user.status", "active");
let result = condition.evaluate_with(&ctx);
```

## License

This project is licensed under the MIT License.
