//! 标量表达式 AST
//! Scalar expression AST

use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use crate::symbol::OwnedSymbol;
use super::property_path::{PropertyPath, property_path_from_owned_symbol};
use super::operators::*;
use super::value::ExpressionValue;
use super::boolean::BooleanExpression;
use super::normalize::scalar_structural_key;

/// 解析后的标量表达式。
/// Parsed scalar expression.
pub type ParsedScalarExpression = ScalarExpression<ExpressionValue>;

/// 标量表达式。
/// Scalar expression.
#[derive(Debug, Clone, PartialEq)]
pub enum ScalarExpression<T> {
    /// 标量常量 / Scalar constant
    Constant(T),
    /// 属性路径引用 / Property path reference
    Reference(PropertyPath),
    /// 普通符号引用 / Plain symbol reference
    SymbolReference(OwnedSymbol),
    /// 一元操作 / Unary operation
    Unary {
        /// 操作符 / Operator
        operator: UnaryOperator,
        /// 操作数 / Operand
        operand: Box<ScalarExpression<T>>,
    },
    /// 二元操作 / Binary operation
    Binary {
        /// 操作符 / Operator
        operator: BinaryOperator,
        /// 左操作数 / Left operand
        left: Box<ScalarExpression<T>>,
        /// 右操作数 / Right operand
        right: Box<ScalarExpression<T>>,
    },
    /// 函数调用 / Function call
    Function {
        /// 函数名 / Function name
        name: String,
        /// 参数列表 / Argument list
        arguments: Vec<ScalarExpression<T>>,
    },
    /// 自定义表达式 / Custom expression
    Custom {
        /// 自定义载荷 / Custom payload
        payload: String,
        /// 可选描述 / Optional description
        description: Option<String>,
    },
    /// 条件表达式（if/then/else 或三元 ?:）
    /// Conditional expression (if/then/else or ternary ?:)
    ///
    /// 桥接 BooleanExpression（条件）到 ScalarExpression（分支），
    /// 与 Comparison 的桥接方向相反——Comparison 是标量→布尔，Conditional 是布尔→标量。
    /// Bridges BooleanExpression (condition) to ScalarExpression (branches),
    /// opposite direction from Comparison — Comparison is scalar→boolean, Conditional is boolean→scalar.
    Conditional {
        /// 条件布尔表达式 / Condition boolean expression
        condition: Box<BooleanExpression<T>>,
        /// 条件为真时的分支 / Branch when condition is true
        then_branch: Box<ScalarExpression<T>>,
        /// 条件为假时的分支 / Branch when condition is false
        else_branch: Box<ScalarExpression<T>>,
    },
    /// 布尔包装表达式 / Boolean wrapper expression
    ///
    /// 将布尔表达式包装为标量值，用于公式返回布尔结果的场景（如 x > 0 && y > 0）。
    /// Wraps a boolean expression as a scalar value, for formulas returning boolean results
    /// (e.g., x > 0 && y > 0).
    Boolean(Box<BooleanExpression<T>>),
}

impl<T> ScalarExpression<T> {
    /// 创建常量表达式。
    /// Create constant expression.
    pub fn constant(value: T) -> Self {
        Self::Constant(value)
    }

    /// 创建路径引用表达式。
    /// Create path reference expression.
    pub fn reference(path: impl Into<PropertyPath>) -> Self {
        Self::Reference(path.into())
    }

    /// 创建符号引用表达式。
    /// Create symbol reference expression.
    pub fn symbol_reference(symbol: OwnedSymbol) -> Self {
        Self::SymbolReference(symbol)
    }

    /// 创建一元表达式。
    /// Create unary expression.
    pub fn unary(operator: UnaryOperator, operand: Self) -> Self {
        Self::Unary {
            operator,
            operand: Box::new(operand),
        }
    }

    /// 创建二元表达式。
    /// Create binary expression.
    pub fn binary(operator: BinaryOperator, left: Self, right: Self) -> Self {
        Self::Binary {
            operator,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// 创建函数调用表达式。
    /// Create function call expression.
    pub fn function(name: impl Into<String>, arguments: Vec<Self>) -> Self {
        Self::Function {
            name: name.into(),
            arguments,
        }
    }

    /// 创建自定义表达式。
    /// Create custom expression.
    pub fn custom(payload: impl Into<String>, description: Option<String>) -> Self {
        Self::Custom {
            payload: payload.into(),
            description,
        }
    }

    /// 创建条件表达式。
    /// Create conditional expression.
    pub fn conditional(
        condition: BooleanExpression<T>,
        then_branch: Self,
        else_branch: Self,
    ) -> Self {
        Self::Conditional {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        }
    }

    /// 创建布尔包装表达式。
    /// Create boolean wrapper expression.
    pub fn boolean_expr(expr: BooleanExpression<T>) -> Self {
        Self::Boolean(Box::new(expr))
    }

    /// 创建加法表达式。
    /// Create addition expression.
    pub fn add_expr(left: Self, right: Self) -> Self {
        Self::binary(BinaryOperator::Add, left, right)
    }

    /// 创建减法表达式。
    /// Create subtraction expression.
    pub fn subtract_expr(left: Self, right: Self) -> Self {
        Self::binary(BinaryOperator::Subtract, left, right)
    }

    /// 创建乘法表达式。
    /// Create multiplication expression.
    pub fn multiply_expr(left: Self, right: Self) -> Self {
        Self::binary(BinaryOperator::Multiply, left, right)
    }

    /// 创建除法表达式。
    /// Create division expression.
    pub fn divide_expr(left: Self, right: Self) -> Self {
        Self::binary(BinaryOperator::Divide, left, right)
    }

    /// 获取表达式类型名。
    /// Get expression type name.
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Constant(_) => "Constant",
            Self::Reference(_) => "Reference",
            Self::SymbolReference(_) => "SymbolReference",
            Self::Unary { .. } => "Unary",
            Self::Binary { .. } => "Binary",
            Self::Function { .. } => "Function",
            Self::Custom { .. } => "Custom",
            Self::Conditional { .. } => "Conditional",
            Self::Boolean(_) => "Boolean",
        }
    }

    /// 判断表达式是否是常量表达式。
    /// Check whether the expression is constant.
    pub fn is_constant(&self) -> bool {
        match self {
            Self::Constant(_) => true,
            Self::Reference(_) | Self::SymbolReference(_) | Self::Custom { .. } => false,
            Self::Unary { operand, .. } => operand.is_constant(),
            Self::Binary { left, right, .. } => left.is_constant() && right.is_constant(),
            Self::Function { arguments, .. } => arguments.iter().all(Self::is_constant),
            Self::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                condition.is_constant()
                    && then_branch.is_constant()
                    && else_branch.is_constant()
            }
            Self::Boolean(expr) => expr.is_constant(),
        }
    }

    /// 判断表达式是否包含引用。
    /// Check whether the expression contains references.
    pub fn contains_reference(&self) -> bool {
        match self {
            Self::Constant(_) => false,
            Self::Reference(_) | Self::SymbolReference(_) | Self::Custom { .. } => true,
            Self::Unary { operand, .. } => operand.contains_reference(),
            Self::Binary { left, right, .. } => {
                left.contains_reference() || right.contains_reference()
            }
            Self::Function { arguments, .. } => arguments.iter().any(Self::contains_reference),
            Self::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                !condition.collect_references().is_empty()
                    || then_branch.contains_reference()
                    || else_branch.contains_reference()
            }
            Self::Boolean(expr) => !expr.collect_references().is_empty(),
        }
    }

    /// 收集表达式中的属性路径引用。
    /// Collect property path references in the expression.
    pub fn collect_references(&self) -> HashSet<PropertyPath> {
        let mut references = HashSet::new();
        self.collect_references_into(&mut references);
        references
    }

    /// 将属性路径引用收集到给定集合。
    /// Collect property path references into the given set.
    pub fn collect_references_into(&self, references: &mut HashSet<PropertyPath>) {
        match self {
            Self::Reference(path) => {
                references.insert(path.clone());
            }
            Self::SymbolReference(symbol) => {
                if let Some(path) = property_path_from_owned_symbol(symbol) {
                    references.insert(path.clone());
                }
            }
            Self::Unary { operand, .. } => operand.collect_references_into(references),
            Self::Binary { left, right, .. } => {
                left.collect_references_into(references);
                right.collect_references_into(references);
            }
            Self::Function { arguments, .. } => {
                for argument in arguments {
                    argument.collect_references_into(references);
                }
            }
            Self::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                condition.collect_references_into(references);
                then_branch.collect_references_into(references);
                else_branch.collect_references_into(references);
            }
            Self::Boolean(expr) => expr.collect_references_into(references),
            Self::Constant(_) | Self::Custom { .. } => {}
        }
    }

    /// 获取标量表达式深度。
    /// Get scalar expression depth.
    pub fn depth(&self) -> usize {
        match self {
            Self::Constant(_)
            | Self::Reference(_)
            | Self::SymbolReference(_)
            | Self::Custom { .. } => 1,
            Self::Unary { operand, .. } => 1 + operand.depth(),
            Self::Binary { left, right, .. } => 1 + left.depth().max(right.depth()),
            Self::Function { arguments, .. } => {
                1 + arguments.iter().map(Self::depth).max().unwrap_or(0)
            }
            Self::Conditional {
                then_branch,
                else_branch,
                ..
            } => 1 + then_branch.depth().max(else_branch.depth()),
            Self::Boolean(expr) => 1 + expr.depth(),
        }
    }

    /// 获取结构键，用于表达式去重和排序。
    /// Get structural key for expression deduplication and sorting.
    pub fn structural_key(&self) -> String
    where
        T: Display,
    {
        scalar_structural_key(self)
    }
}

impl<T> From<T> for ScalarExpression<T> {
    fn from(value: T) -> Self {
        Self::constant(value)
    }
}

macro_rules! impl_runtime_scalar_from_literal {
    ($($type:ty),* $(,)?) => {
        $(
            impl From<$type> for ScalarExpression<ExpressionValue> {
                fn from(value: $type) -> Self {
                    Self::constant(ExpressionValue::from(value))
                }
            }
        )*
    };
}

impl_runtime_scalar_from_literal!(
    bool, f32, f64, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize
);

impl From<&str> for ScalarExpression<ExpressionValue> {
    fn from(value: &str) -> Self {
        Self::constant(ExpressionValue::from(value))
    }
}

impl From<String> for ScalarExpression<ExpressionValue> {
    fn from(value: String) -> Self {
        Self::constant(ExpressionValue::from(value))
    }
}

impl<T: Display> Display for ScalarExpression<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Constant(value) => write!(f, "{}", value),
            Self::Reference(path) => write!(f, "{}", path),
            Self::SymbolReference(symbol) => write!(f, "{}", symbol),
            Self::Unary { operator, operand } => write!(f, "{}({})", operator.symbol(), operand),
            Self::Binary {
                operator,
                left,
                right,
            } => write!(f, "({} {} {})", left, operator.symbol(), right),
            Self::Function { name, arguments } => {
                write!(f, "{}(", name)?;
                for (index, argument) in arguments.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", argument)?;
                }
                write!(f, ")")
            }
            Self::Custom {
                payload,
                description,
            } => write!(f, "{}", description.as_deref().unwrap_or(payload)),
            Self::Conditional {
                condition,
                then_branch,
                else_branch,
            } => write!(
                f,
                "if ({}) then {} else {}",
                condition, then_branch, else_branch
            ),
            Self::Boolean(expr) => write!(f, "Bool({})", expr),
        }
    }
}
