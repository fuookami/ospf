//! 标量表达式 AST
//! Scalar expression AST

use super::boolean::BooleanExpression;
use super::normalize::scalar_structural_key;
use super::operators::*;
use super::property_path::{PropertyPath, property_path_from_owned_symbol};
use super::value::ExpressionValue;
use crate::symbol::OwnedSymbol;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};

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
            } => condition.is_constant() && then_branch.is_constant() && else_branch.is_constant(),
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

// ============================================================================
// 标量表达式测试 / Scalar expression tests
// ============================================================================

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::Trivalent;
    use crate::symbol::expression::{PathSymbol, path_owned_symbol};

    use super::*;

    /// 构造 `a = 1` 比较，作为标量测试中的通用叶子条件。
    /// Build the `a = 1` comparison as the shared leaf condition in scalar tests.
    fn leaf_condition() -> BooleanExpression<ExpressionValue> {
        BooleanExpression::eq(
            ScalarExpression::reference("a"),
            ScalarExpression::constant(ExpressionValue::Number(1.0)),
        )
    }

    // ========================================================================
    // 构造函数 / Constructors
    // ========================================================================

    #[test]
    fn constant_and_reference_constructors_build_expected_variants() {
        let constant = ScalarExpression::constant(ExpressionValue::Number(1.0));
        let reference = ScalarExpression::<ExpressionValue>::reference("user.age");
        let symbol_reference =
            ScalarExpression::<ExpressionValue>::symbol_reference(path_owned_symbol("order.price"));

        assert_eq!(constant, ScalarExpression::Constant(ExpressionValue::Number(1.0)));
        assert_eq!(
            reference,
            ScalarExpression::Reference(PropertyPath::parse("user.age"))
        );
        assert!(matches!(
            symbol_reference,
            ScalarExpression::SymbolReference(_)
        ));
    }

    #[test]
    fn reference_constructor_accepts_owned_and_borrowed_paths() {
        let from_str = ScalarExpression::<ExpressionValue>::reference("user.age");
        let from_path =
            ScalarExpression::<ExpressionValue>::reference(PropertyPath::parse("user.age"));

        assert_eq!(from_str, from_path);
    }

    #[test]
    fn unary_binary_and_function_constructors_store_children() {
        let unary = ScalarExpression::unary(
            UnaryOperator::Negate,
            ScalarExpression::constant(ExpressionValue::Number(2.0)),
        );
        let binary = ScalarExpression::binary(
            BinaryOperator::Divide,
            ScalarExpression::<ExpressionValue>::reference("total"),
            ScalarExpression::constant(ExpressionValue::Number(4.0)),
        );
        let function = ScalarExpression::<ExpressionValue>::function(
            ScalarFunctionNames::ABS,
            vec![ScalarExpression::<ExpressionValue>::reference("delta")],
        );

        let ScalarExpression::Unary { operator, operand } = unary else {
            panic!("expected unary expression");
        };
        assert_eq!(operator, UnaryOperator::Negate);
        assert_eq!(*operand, ScalarExpression::constant(ExpressionValue::Number(2.0)));

        let ScalarExpression::Binary {
            operator,
            left,
            right,
        } = binary
        else {
            panic!("expected binary expression");
        };
        assert_eq!(operator, BinaryOperator::Divide);
        assert_eq!(*left, ScalarExpression::reference("total"));
        assert_eq!(*right, ScalarExpression::constant(ExpressionValue::Number(4.0)));

        let ScalarExpression::Function { name, arguments } = function else {
            panic!("expected function expression");
        };
        assert_eq!(name, "abs");
        assert_eq!(arguments, vec![ScalarExpression::reference("delta")]);
    }

    #[test]
    fn custom_conditional_and_boolean_constructors_store_payloads() {
        let custom = ScalarExpression::<ExpressionValue>::custom("raw", Some("shown".to_string()));
        let conditional = ScalarExpression::conditional(
            leaf_condition(),
            ScalarExpression::constant(ExpressionValue::Number(2.0)),
            ScalarExpression::constant(ExpressionValue::Number(3.0)),
        );
        let boolean = ScalarExpression::boolean_expr(leaf_condition());

        assert_eq!(
            custom,
            ScalarExpression::Custom {
                payload: "raw".to_string(),
                description: Some("shown".to_string()),
            }
        );
        assert!(matches!(conditional, ScalarExpression::Conditional { .. }));
        assert!(matches!(boolean, ScalarExpression::Boolean(_)));
    }

    #[test]
    fn binary_helpers_map_to_expected_operators() {
        let left = ScalarExpression::reference("a");
        let right = ScalarExpression::constant(ExpressionValue::Number(2.0));
        let helpers = [
            (
                ScalarExpression::add_expr(left.clone(), right.clone()),
                BinaryOperator::Add,
            ),
            (
                ScalarExpression::subtract_expr(left.clone(), right.clone()),
                BinaryOperator::Subtract,
            ),
            (
                ScalarExpression::multiply_expr(left.clone(), right.clone()),
                BinaryOperator::Multiply,
            ),
            (
                ScalarExpression::divide_expr(left.clone(), right.clone()),
                BinaryOperator::Divide,
            ),
        ];

        for (expression, expected) in helpers {
            let ScalarExpression::Binary { operator, .. } = expression else {
                panic!("expected binary expression");
            };
            assert_eq!(operator, expected);
        }
    }

    // ========================================================================
    // 结构查询 / Structure queries
    // ========================================================================

    #[test]
    fn type_name_covers_every_variant() {
        assert_eq!(
            ScalarExpression::<ExpressionValue>::constant(ExpressionValue::Number(1.0)).type_name(),
            "Constant"
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::reference("a").type_name(),
            "Reference"
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::symbol_reference(path_owned_symbol("a"))
                .type_name(),
            "SymbolReference"
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::unary(
                UnaryOperator::Abs,
                ScalarExpression::constant(ExpressionValue::Number(1.0)),
            )
            .type_name(),
            "Unary"
        );
        assert_eq!(
            ScalarExpression::add_expr(
                ScalarExpression::constant(ExpressionValue::Number(1.0)),
                ScalarExpression::constant(ExpressionValue::Number(2.0)),
            )
            .type_name(),
            "Binary"
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::function("f", Vec::new()).type_name(),
            "Function"
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::custom("raw", None).type_name(),
            "Custom"
        );
        assert_eq!(
            ScalarExpression::conditional(
                leaf_condition(),
                ScalarExpression::constant(ExpressionValue::Number(1.0)),
                ScalarExpression::constant(ExpressionValue::Number(2.0)),
            )
            .type_name(),
            "Conditional"
        );
        assert_eq!(
            ScalarExpression::boolean_expr(leaf_condition()).type_name(),
            "Boolean"
        );
    }

    #[test]
    fn is_constant_requires_all_children_to_be_constant() {
        let constant = || ScalarExpression::constant(ExpressionValue::Number(1.0));

        assert!(constant().is_constant());
        assert!(ScalarExpression::unary(UnaryOperator::Negate, constant()).is_constant());
        assert!(ScalarExpression::add_expr(constant(), constant()).is_constant());
        assert!(
            ScalarExpression::<ExpressionValue>::function("abs", vec![constant()]).is_constant()
        );
        assert!(
            ScalarExpression::<ExpressionValue>::function("f", Vec::new()).is_constant(),
            "无参函数是常量表达式 / A function without arguments is constant"
        );
        assert!(
            ScalarExpression::conditional(
                BooleanExpression::eq(
                    ScalarExpression::constant(ExpressionValue::Number(1.0)),
                    ScalarExpression::constant(ExpressionValue::Number(1.0)),
                ),
                constant(),
                constant(),
            )
            .is_constant(),
            "全常量条件与分支保持常量性 / An all-constant condition with constant branches stays constant"
        );
        assert!(
            !ScalarExpression::conditional(leaf_condition(), constant(), constant()).is_constant(),
            "含引用的条件让条件表达式不再是常量 / A referencing condition makes the conditional non-constant"
        );

        let reference = ScalarExpression::<ExpressionValue>::reference("a");
        assert!(!reference.is_constant());
        assert!(!ScalarExpression::unary(UnaryOperator::Abs, reference.clone()).is_constant());
        assert!(!ScalarExpression::add_expr(constant(), reference.clone()).is_constant());
        assert!(!ScalarExpression::<ExpressionValue>::function("abs", vec![reference]).is_constant());
    }

    #[test]
    fn is_constant_is_false_for_custom_and_symbol_reference() {
        // Custom 与 SymbolReference 都视为不透明引用，永远不是常量。
        // Custom and SymbolReference are opaque references and are never constant.
        assert!(!ScalarExpression::<ExpressionValue>::custom("raw", None).is_constant());
        assert!(
            !ScalarExpression::<ExpressionValue>::symbol_reference(path_owned_symbol("a"))
                .is_constant()
        );
    }

    #[test]
    fn is_constant_for_boolean_wrapper_follows_wrapped_expression() {
        let constant_wrapper = ScalarExpression::boolean_expr(BooleanExpression::<
            ExpressionValue,
        >::constant(Trivalent::True));
        let reference_wrapper = ScalarExpression::boolean_expr(leaf_condition());

        assert!(constant_wrapper.is_constant());
        assert!(!reference_wrapper.is_constant());
    }

    #[test]
    fn contains_reference_detects_references_at_any_depth() {
        let reference = ScalarExpression::<ExpressionValue>::reference("a");
        let constant = || ScalarExpression::constant(ExpressionValue::Number(1.0));

        assert!(!constant().contains_reference());
        assert!(reference.contains_reference());
        assert!(
            ScalarExpression::unary(UnaryOperator::Abs, reference.clone()).contains_reference()
        );
        assert!(!ScalarExpression::add_expr(constant(), constant()).contains_reference());
        assert!(ScalarExpression::add_expr(constant(), reference.clone()).contains_reference());
        assert!(
            ScalarExpression::<ExpressionValue>::function("f", vec![constant(), reference])
                .contains_reference()
        );
        assert!(ScalarExpression::<ExpressionValue>::custom("raw", None).contains_reference());
    }

    #[test]
    fn contains_reference_sees_inside_conditional_and_boolean_wrapper() {
        let reference = ScalarExpression::<ExpressionValue>::reference("a");
        let constant = || ScalarExpression::constant(ExpressionValue::Number(1.0));

        let conditional = ScalarExpression::conditional(
            leaf_condition(),
            constant(),
            constant(),
        );
        assert!(
            conditional.contains_reference(),
            "条件分支的比较引用也应被识别 / The comparison reference in the condition is detected"
        );

        let branch_reference = ScalarExpression::conditional(
            BooleanExpression::<ExpressionValue>::constant(Trivalent::True),
            reference.clone(),
            constant(),
        );
        assert!(branch_reference.contains_reference());

        let wrapper = ScalarExpression::boolean_expr(leaf_condition());
        assert!(wrapper.contains_reference());

        let constant_wrapper = ScalarExpression::boolean_expr(BooleanExpression::<
            ExpressionValue,
        >::constant(Trivalent::True));
        assert!(!constant_wrapper.contains_reference());
    }

    // ========================================================================
    // 引用收集 / Reference collection
    // ========================================================================

    #[test]
    fn collect_references_gathers_paths_from_every_branch() {
        let expression = ScalarExpression::<ExpressionValue>::conditional(
            BooleanExpression::and(vec![
                BooleanExpression::ge(
                    ScalarExpression::reference("age"),
                    ScalarExpression::constant(ExpressionValue::Number(18.0)),
                ),
                BooleanExpression::is_not_null("profile.email"),
            ]),
            ScalarExpression::add_expr(
                ScalarExpression::reference("score"),
                ScalarExpression::reference("bonus"),
            ),
            ScalarExpression::symbol_reference(path_owned_symbol("fallback.value")),
        );

        let references = expression.collect_references();

        for path in ["age", "profile.email", "score", "bonus", "fallback.value"] {
            assert!(
                references.contains(&PropertyPath::parse(path)),
                "missing reference: {path}"
            );
        }
        assert_eq!(references.len(), 5);
    }

    #[test]
    fn collect_references_ignores_constants_and_non_path_symbols() {
        let expression = ScalarExpression::add_expr(
            ScalarExpression::constant(ExpressionValue::Number(1.0)),
            ScalarExpression::symbol_reference(OwnedSymbol::new(
                crate::symbol::test_utils::SimpleSymbol::new("plain"),
            )),
        );

        assert!(expression.collect_references().is_empty());
        assert!(ScalarExpression::<ExpressionValue>::custom("raw", None)
            .collect_references()
            .is_empty());
    }

    #[test]
    fn collect_references_into_merges_with_existing_paths() {
        let expression = ScalarExpression::<ExpressionValue>::function(
            ScalarFunctionNames::ABS,
            vec![ScalarExpression::reference("delta")],
        );
        let mut references = HashSet::new();
        references.insert(PropertyPath::parse("pre.existing"));

        expression.collect_references_into(&mut references);

        assert_eq!(references.len(), 2);
        assert!(references.contains(&PropertyPath::parse("pre.existing")));
        assert!(references.contains(&PropertyPath::parse("delta")));
    }

    #[test]
    fn collect_references_deduplicates_repeated_paths() {
        let expression = ScalarExpression::add_expr(
            ScalarExpression::<ExpressionValue>::reference("a"),
            ScalarExpression::<ExpressionValue>::reference("a"),
        );

        assert_eq!(expression.collect_references().len(), 1);
    }

    // ========================================================================
    // 深度 / Depth
    // ========================================================================

    #[test]
    fn depth_counts_nesting_levels() {
        let constant = || ScalarExpression::constant(ExpressionValue::Number(1.0));

        assert_eq!(constant().depth(), 1);
        assert_eq!(ScalarExpression::<ExpressionValue>::reference("a").depth(), 1);
        assert_eq!(
            ScalarExpression::<ExpressionValue>::symbol_reference(path_owned_symbol("a")).depth(),
            1
        );
        assert_eq!(ScalarExpression::<ExpressionValue>::custom("raw", None).depth(), 1);
        assert_eq!(
            ScalarExpression::unary(UnaryOperator::Negate, constant()).depth(),
            2
        );
        assert_eq!(ScalarExpression::add_expr(constant(), constant()).depth(), 2);
    }

    #[test]
    fn depth_for_binary_and_function_uses_the_deepest_child() {
        let binary = ScalarExpression::binary(
            BinaryOperator::Add,
            ScalarExpression::reference("a"),
            ScalarExpression::<ExpressionValue>::function(
                ScalarFunctionNames::ABS,
                vec![ScalarExpression::reference("b")],
            ),
        );
        assert_eq!(binary.depth(), 3);

        let function = ScalarExpression::<ExpressionValue>::function(
            "f",
            vec![
                ScalarExpression::constant(ExpressionValue::Number(1.0)),
                ScalarExpression::add_expr(
                    ScalarExpression::reference("a"),
                    ScalarExpression::reference("b"),
                ),
            ],
        );
        assert_eq!(function.depth(), 3);

        assert_eq!(
            ScalarExpression::<ExpressionValue>::function("f", Vec::new()).depth(),
            1,
            "无参函数深度为 1 / A function without arguments has depth 1"
        );
    }

    #[test]
    fn depth_for_conditional_ignores_the_condition_subtree() {
        // 条件子树的深度不计入 Conditional 深度，只有分支参与计算。
        // The condition subtree does not contribute to Conditional depth; only branches do.
        let deep_condition = BooleanExpression::and(vec![leaf_condition(), leaf_condition()]);
        assert_eq!(deep_condition.depth(), 2);

        let expression = ScalarExpression::conditional(
            deep_condition,
            ScalarExpression::constant(ExpressionValue::Number(1.0)),
            ScalarExpression::constant(ExpressionValue::Number(2.0)),
        );
        assert_eq!(expression.depth(), 2);

        let deeper_branch = ScalarExpression::conditional(
            BooleanExpression::<ExpressionValue>::constant(Trivalent::True),
            ScalarExpression::add_expr(
                ScalarExpression::reference("a"),
                ScalarExpression::reference("b"),
            ),
            ScalarExpression::constant(ExpressionValue::Number(0.0)),
        );
        assert_eq!(deeper_branch.depth(), 3);
    }

    #[test]
    fn depth_for_boolean_wrapper_includes_wrapped_expression() {
        assert_eq!(
            ScalarExpression::boolean_expr(
                BooleanExpression::<ExpressionValue>::constant(Trivalent::True)
            )
            .depth(),
            2
        );
        assert_eq!(
            ScalarExpression::boolean_expr(BooleanExpression::and(vec![
                leaf_condition(),
                leaf_condition(),
            ]))
            .depth(),
            3
        );
    }

    // ========================================================================
    // 转换与显示 / Conversions and display
    // ========================================================================

    #[test]
    fn from_generic_value_wraps_into_constant() {
        assert_eq!(
            ScalarExpression::<i32>::from(5_i32),
            ScalarExpression::<i32>::constant(5)
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::from(ExpressionValue::Null),
            ScalarExpression::constant(ExpressionValue::Null)
        );
    }

    #[test]
    fn from_literal_conversions_build_runtime_constants() {
        assert_eq!(
            ScalarExpression::<ExpressionValue>::from(18_i32),
            ScalarExpression::constant(ExpressionValue::Number(18.0))
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::from(true),
            ScalarExpression::constant(ExpressionValue::Boolean(true))
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::from(2.5_f64),
            ScalarExpression::constant(ExpressionValue::Number(2.5))
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::from("active"),
            ScalarExpression::constant(ExpressionValue::String("active".to_string()))
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::from("active".to_string()),
            ScalarExpression::constant(ExpressionValue::String("active".to_string()))
        );
    }

    #[test]
    fn display_renders_every_variant() {
        assert_eq!(
            ScalarExpression::<ExpressionValue>::constant(ExpressionValue::Number(1.0)).to_string(),
            "1"
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::reference("user.age").to_string(),
            "user.age"
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::symbol_reference(path_owned_symbol("user.age"))
                .to_string(),
            "user.age"
        );
        assert_eq!(
            ScalarExpression::unary(
                UnaryOperator::Negate,
                ScalarExpression::constant(ExpressionValue::Number(1.0)),
            )
            .to_string(),
            "-(1)"
        );
        assert_eq!(
            ScalarExpression::add_expr(
                ScalarExpression::reference("a"),
                ScalarExpression::constant(ExpressionValue::Number(1.0)),
            )
            .to_string(),
            "(a + 1)"
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::function(
                ScalarFunctionNames::ABS,
                vec![ScalarExpression::reference("x")],
            )
            .to_string(),
            "abs(x)"
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::function(
                "max",
                vec![
                    ScalarExpression::constant(ExpressionValue::Number(1.0)),
                    ScalarExpression::constant(ExpressionValue::Number(2.0)),
                ],
            )
            .to_string(),
            "max(1, 2)"
        );
        assert_eq!(
            ScalarExpression::conditional(
                leaf_condition(),
                ScalarExpression::constant(ExpressionValue::Number(2.0)),
                ScalarExpression::constant(ExpressionValue::Number(3.0)),
            )
            .to_string(),
            "if (a = 1) then 2 else 3"
        );
        assert_eq!(
            ScalarExpression::boolean_expr(leaf_condition()).to_string(),
            "Bool(a = 1)"
        );
    }

    #[test]
    fn display_prefers_custom_description_over_payload() {
        assert_eq!(
            ScalarExpression::<ExpressionValue>::custom("raw", None).to_string(),
            "raw"
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::custom("raw", Some("shown".to_string())).to_string(),
            "shown"
        );
    }

    // ========================================================================
    // 结构键 / Structural keys
    // ========================================================================

    #[test]
    fn structural_key_is_stable_and_structure_sensitive() {
        let build = || {
            ScalarExpression::add_expr(
                ScalarExpression::reference("a"),
                ScalarExpression::constant(ExpressionValue::Number(1.0)),
            )
        };

        assert_eq!(build().structural_key(), build().structural_key());
        assert_eq!(build().structural_key(), "Bin:Add:Ref:a:Const:1");
        assert_eq!(
            ScalarExpression::<ExpressionValue>::constant(ExpressionValue::Number(1.0))
                .structural_key(),
            "Const:1"
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::reference("a").structural_key(),
            "Ref:a"
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::symbol_reference(path_owned_symbol("user.name"))
                .structural_key(),
            "SymRef:user.name"
        );
        assert_eq!(
            ScalarExpression::unary(
                UnaryOperator::Negate,
                ScalarExpression::constant(ExpressionValue::Number(1.0)),
            )
            .structural_key(),
            "Unary:Negate:Const:1"
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::function(
                ScalarFunctionNames::ABS,
                vec![ScalarExpression::reference("x")],
            )
            .structural_key(),
            "Func:abs:Ref:x"
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::custom("raw", None).structural_key(),
            "Custom:raw"
        );
        assert_eq!(
            ScalarExpression::<ExpressionValue>::custom("raw", Some("shown".to_string()))
                .structural_key(),
            "Custom:shown"
        );
        assert_eq!(
            ScalarExpression::boolean_expr(leaf_condition()).structural_key(),
            "Bool:Cmp:Eq:Ref:a:Const:1"
        );
        assert_eq!(
            ScalarExpression::conditional(
                leaf_condition(),
                ScalarExpression::constant(ExpressionValue::Number(2.0)),
                ScalarExpression::constant(ExpressionValue::Number(3.0)),
            )
            .structural_key(),
            "Cond:Cmp:Eq:Ref:a:Const:1:Const:2:Const:3"
        );

        // 交换左右操作数必须改变结构键，避免错误去重。
        // Swapping operands must change the structural key to avoid wrong deduplication.
        assert_ne!(
            ScalarExpression::add_expr(
                ScalarExpression::<ExpressionValue>::reference("a"),
                ScalarExpression::<ExpressionValue>::reference("b"),
            )
            .structural_key(),
            ScalarExpression::add_expr(
                ScalarExpression::<ExpressionValue>::reference("b"),
                ScalarExpression::<ExpressionValue>::reference("a"),
            )
            .structural_key()
        );
    }

    #[test]
    fn symbol_reference_structural_key_uses_symbol_name_not_path_prefix() {
        let from_path = ScalarExpression::<ExpressionValue>::symbol_reference(path_owned_symbol(
            "user.name",
        ));
        let from_helper = ScalarExpression::<ExpressionValue>::symbol_reference(
            PathSymbol::from_path_str("user.name").into_owned_symbol(),
        );

        assert_eq!(from_path.structural_key(), from_helper.structural_key());
        assert_eq!(from_path.collect_references(), from_helper.collect_references());
    }
}
