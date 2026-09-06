//! 表达式树变换工具
//! Expression tree transformation utilities

use super::boolean::BooleanExpression;
use super::scalar::ScalarExpression;

/// 递归变换标量表达式。
/// Recursively transform a scalar expression.
///
/// 子节点先重建并变换，当前节点最后交给回调，因此回调接收的是后序重建节点。
/// Children are rebuilt and transformed first, so the callback receives the rebuilt node in post-order.
pub fn transform_scalar_expression<T, F>(
    expression: &ScalarExpression<T>,
    transformer: &mut F,
) -> ScalarExpression<T>
where
    T: Clone,
    F: FnMut(ScalarExpression<T>) -> ScalarExpression<T>,
{
    let rebuilt = match expression {
        ScalarExpression::Constant(value) => ScalarExpression::Constant(value.clone()),
        ScalarExpression::Reference(path) => ScalarExpression::Reference(path.clone()),
        ScalarExpression::SymbolReference(symbol) => {
            ScalarExpression::SymbolReference(symbol.clone())
        }
        ScalarExpression::Unary { operator, operand } => ScalarExpression::Unary {
            operator: *operator,
            operand: Box::new(transform_scalar_expression(operand, transformer)),
        },
        ScalarExpression::Binary {
            operator,
            left,
            right,
        } => ScalarExpression::Binary {
            operator: *operator,
            left: Box::new(transform_scalar_expression(left, transformer)),
            right: Box::new(transform_scalar_expression(right, transformer)),
        },
        ScalarExpression::Function { name, arguments } => ScalarExpression::Function {
            name: name.clone(),
            arguments: arguments
                .iter()
                .map(|argument| transform_scalar_expression(argument, transformer))
                .collect(),
        },
        ScalarExpression::Custom {
            payload,
            description,
        } => ScalarExpression::Custom {
            payload: payload.clone(),
            description: description.clone(),
        },
        ScalarExpression::Conditional {
            condition,
            then_branch,
            else_branch,
        } => ScalarExpression::Conditional {
            condition: Box::new(transform_boolean_scalars(condition, transformer)),
            then_branch: Box::new(transform_scalar_expression(then_branch, transformer)),
            else_branch: Box::new(transform_scalar_expression(else_branch, transformer)),
        },
        ScalarExpression::Boolean(expression) => {
            ScalarExpression::Boolean(Box::new(transform_boolean_scalars(expression, transformer)))
        }
    };
    transformer(rebuilt)
}

/// 递归变换布尔表达式中的标量节点。
/// Recursively transform scalar nodes inside a boolean expression.
pub fn transform_boolean_scalars<T, F>(
    expression: &BooleanExpression<T>,
    transformer: &mut F,
) -> BooleanExpression<T>
where
    T: Clone,
    F: FnMut(ScalarExpression<T>) -> ScalarExpression<T>,
{
    match expression {
        BooleanExpression::Constant(value) => BooleanExpression::Constant(*value),
        BooleanExpression::Comparison {
            operator,
            left,
            right,
        } => BooleanExpression::Comparison {
            operator: *operator,
            left: transform_scalar_expression(left, transformer),
            right: transform_scalar_expression(right, transformer),
        },
        BooleanExpression::In {
            value,
            candidates,
            negated,
        } => BooleanExpression::In {
            value: transform_scalar_expression(value, transformer),
            candidates: candidates
                .iter()
                .map(|candidate| transform_scalar_expression(candidate, transformer))
                .collect(),
            negated: *negated,
        },
        BooleanExpression::PatternMatch {
            value,
            pattern,
            mode,
            negated,
        } => BooleanExpression::PatternMatch {
            value: transform_scalar_expression(value, transformer),
            pattern: transform_scalar_expression(pattern, transformer),
            mode: *mode,
            negated: *negated,
        },
        BooleanExpression::NullCheck {
            path,
            null_check_type,
        } => BooleanExpression::NullCheck {
            path: path.clone(),
            null_check_type: *null_check_type,
        },
        BooleanExpression::And(operands) => BooleanExpression::And(
            operands
                .iter()
                .map(|operand| transform_boolean_scalars(operand, transformer))
                .collect(),
        ),
        BooleanExpression::Or(operands) => BooleanExpression::Or(
            operands
                .iter()
                .map(|operand| transform_boolean_scalars(operand, transformer))
                .collect(),
        ),
        BooleanExpression::Not(operand) => {
            BooleanExpression::Not(Box::new(transform_boolean_scalars(operand, transformer)))
        }
        BooleanExpression::Custom {
            payload,
            description,
        } => BooleanExpression::Custom {
            payload: payload.clone(),
            description: description.clone(),
        },
    }
}

/// 递归变换布尔表达式节点。
/// Recursively transform boolean expression nodes.
pub fn transform_boolean_expression<T, F>(
    expression: &BooleanExpression<T>,
    transformer: &mut F,
) -> BooleanExpression<T>
where
    T: Clone,
    F: FnMut(BooleanExpression<T>) -> BooleanExpression<T>,
{
    let rebuilt = match expression {
        BooleanExpression::Constant(value) => BooleanExpression::Constant(*value),
        BooleanExpression::Comparison {
            operator,
            left,
            right,
        } => BooleanExpression::Comparison {
            operator: *operator,
            left: left.clone(),
            right: right.clone(),
        },
        BooleanExpression::In {
            value,
            candidates,
            negated,
        } => BooleanExpression::In {
            value: value.clone(),
            candidates: candidates.clone(),
            negated: *negated,
        },
        BooleanExpression::PatternMatch {
            value,
            pattern,
            mode,
            negated,
        } => BooleanExpression::PatternMatch {
            value: value.clone(),
            pattern: pattern.clone(),
            mode: *mode,
            negated: *negated,
        },
        BooleanExpression::NullCheck {
            path,
            null_check_type,
        } => BooleanExpression::NullCheck {
            path: path.clone(),
            null_check_type: *null_check_type,
        },
        BooleanExpression::And(operands) => BooleanExpression::And(
            operands
                .iter()
                .map(|operand| transform_boolean_expression(operand, transformer))
                .collect(),
        ),
        BooleanExpression::Or(operands) => BooleanExpression::Or(
            operands
                .iter()
                .map(|operand| transform_boolean_expression(operand, transformer))
                .collect(),
        ),
        BooleanExpression::Not(operand) => {
            BooleanExpression::Not(Box::new(transform_boolean_expression(operand, transformer)))
        }
        BooleanExpression::Custom {
            payload,
            description,
        } => BooleanExpression::Custom {
            payload: payload.clone(),
            description: description.clone(),
        },
    };
    transformer(rebuilt)
}

/// 标量表达式变换扩展。
/// Scalar expression transformation extension.
pub trait ScalarExpressionTransform<T>: Sized {
    /// 以后序方式变换标量表达式节点及其嵌套布尔分支。
    /// Transform scalar nodes and nested boolean branches in post-order.
    fn transform<F>(&self, transformer: F) -> Self
    where
        T: Clone,
        F: FnMut(ScalarExpression<T>) -> ScalarExpression<T>;
}

impl<T> ScalarExpressionTransform<T> for ScalarExpression<T> {
    fn transform<F>(&self, mut transformer: F) -> Self
    where
        T: Clone,
        F: FnMut(ScalarExpression<T>) -> ScalarExpression<T>,
    {
        transform_scalar_expression(self, &mut transformer)
    }
}

/// 布尔表达式变换扩展。
/// Boolean expression transformation extension.
pub trait BooleanExpressionTransform<T>: Sized {
    /// 以后序方式变换布尔表达式节点。
    /// Transform boolean expression nodes in post-order.
    fn transform<F>(&self, transformer: F) -> Self
    where
        T: Clone,
        F: FnMut(BooleanExpression<T>) -> BooleanExpression<T>;

    /// 只变换布尔树中的标量节点。
    /// Transform only scalar nodes inside the boolean tree.
    fn transform_scalars<F>(&self, transformer: F) -> Self
    where
        T: Clone,
        F: FnMut(ScalarExpression<T>) -> ScalarExpression<T>;

    /// 只变换布尔节点，标量子树保持不变。
    /// Transform only boolean nodes while preserving scalar subtrees.
    fn transform_booleans<F>(&self, transformer: F) -> Self
    where
        T: Clone,
        F: FnMut(BooleanExpression<T>) -> BooleanExpression<T>;
}

impl<T> BooleanExpressionTransform<T> for BooleanExpression<T> {
    fn transform<F>(&self, transformer: F) -> Self
    where
        T: Clone,
        F: FnMut(BooleanExpression<T>) -> BooleanExpression<T>,
    {
        let rebuilt = self.transform_scalars(|expression| expression);
        rebuilt.transform_booleans(transformer)
    }

    fn transform_scalars<F>(&self, mut transformer: F) -> Self
    where
        T: Clone,
        F: FnMut(ScalarExpression<T>) -> ScalarExpression<T>,
    {
        transform_boolean_scalars(self, &mut transformer)
    }

    fn transform_booleans<F>(&self, mut transformer: F) -> Self
    where
        T: Clone,
        F: FnMut(BooleanExpression<T>) -> BooleanExpression<T>,
    {
        transform_boolean_expression(self, &mut transformer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Trivalent;
    use crate::symbol::expression::{BinaryOperator, ExpressionValue, PropertyPath};

    #[test]
    fn transforms_scalar_nodes_inside_boolean_nodes() {
        let expression = BooleanExpression::eq(
            ScalarExpression::reference("age"),
            ScalarExpression::constant(ExpressionValue::from(18)),
        );
        let transformed = expression.transform_scalars(|scalar| match scalar {
            ScalarExpression::Constant(ExpressionValue::Number(value)) => {
                ScalarExpression::constant(ExpressionValue::Number(value + 3.0))
            }
            scalar => scalar,
        });

        assert_eq!(
            transformed,
            BooleanExpression::eq(
                ScalarExpression::reference("age"),
                ScalarExpression::constant(ExpressionValue::from(21)),
            )
        );
    }

    #[test]
    fn invokes_boolean_transformer_in_post_order() {
        let expression = BooleanExpression::<ExpressionValue>::and(vec![
            BooleanExpression::constant(Trivalent::True),
            BooleanExpression::not_expr(BooleanExpression::constant(Trivalent::False)),
        ]);
        let mut visited = Vec::new();
        let transformed = expression.transform_booleans(|expression| {
            visited.push(expression.type_name());
            expression
        });

        assert!(matches!(transformed, BooleanExpression::And(_)));
        assert_eq!(
            visited,
            vec!["BooleanConstant", "BooleanConstant", "Not", "And"]
        );
    }

    #[test]
    fn transforms_nested_conditional_scalar_and_boolean_branches() {
        let expression = ScalarExpression::conditional(
            BooleanExpression::not_expr(BooleanExpression::eq(
                ScalarExpression::reference(PropertyPath::parse("status")),
                ScalarExpression::constant(ExpressionValue::from(1)),
            )),
            ScalarExpression::binary(
                BinaryOperator::Add,
                ScalarExpression::constant(ExpressionValue::from(2)),
                ScalarExpression::constant(ExpressionValue::from(3)),
            ),
            ScalarExpression::constant(ExpressionValue::from(0)),
        );
        let transformed = expression.transform(|scalar| match scalar {
            ScalarExpression::Constant(ExpressionValue::Number(value)) => {
                ScalarExpression::constant(ExpressionValue::Number(value + 10.0))
            }
            scalar => scalar,
        });

        let ScalarExpression::Conditional {
            condition,
            then_branch,
            else_branch,
        } = transformed
        else {
            panic!("expected conditional expression");
        };
        let BooleanExpression::Not(operand) = *condition else {
            panic!("expected NOT condition");
        };
        let BooleanExpression::Comparison { right, .. } = *operand else {
            panic!("expected comparison condition");
        };
        assert_eq!(right, ScalarExpression::constant(ExpressionValue::from(11)));
        let ScalarExpression::Binary { left, right, .. } = *then_branch else {
            panic!("expected binary then branch");
        };
        assert_eq!(*left, ScalarExpression::constant(ExpressionValue::from(12)));
        assert_eq!(
            *right,
            ScalarExpression::constant(ExpressionValue::from(13))
        );
        assert_eq!(
            *else_branch,
            ScalarExpression::constant(ExpressionValue::from(10))
        );
    }

    #[test]
    fn scalar_transform_keeps_boolean_nodes_unchanged() {
        let expression = ScalarExpression::boolean_expr(BooleanExpression::eq(
            ScalarExpression::reference("age"),
            ScalarExpression::constant(ExpressionValue::from(18)),
        ));
        let transformed = expression.transform(|scalar| scalar);

        assert_eq!(transformed.structural_key(), expression.structural_key());
    }
}
