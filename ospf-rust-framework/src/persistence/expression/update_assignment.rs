//! 更新赋值表达式
//! Update assignment expression

use ospf_rust_math::symbol::{ExpressionValue, PropertyPath, ScalarExpression};

/// 更新赋值集合 / Update assignment collection
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateAssignments<T = ExpressionValue> {
    /// 赋值项列表 / Assignment items
    pub items: Vec<UpdateAssignment<T>>,
}

impl<T> Default for UpdateAssignments<T> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<T> UpdateAssignments<T> {
    /// 创建空赋值集合。
    /// Create an empty assignment set.
    pub fn empty() -> Self {
        Self::default()
    }

    /// 创建设置值赋值。
    /// Create a set-value assignment.
    pub fn set(path: impl Into<PropertyPath>, value: T) -> Self {
        Self {
            items: vec![UpdateAssignment::SetValue(SetValue::new(path, value))],
        }
    }

    /// 创建设置空值赋值。
    /// Create a set-null assignment.
    pub fn set_null(path: impl Into<PropertyPath>) -> Self {
        Self {
            items: vec![UpdateAssignment::SetNull(SetNull::new(path))],
        }
    }

    /// 创建表达式赋值。
    /// Create an expression assignment.
    pub fn set_expr(path: impl Into<PropertyPath>, expression: ScalarExpression<T>) -> Self {
        Self {
            items: vec![UpdateAssignment::SetFromExpression(SetFromExpression::new(
                path, expression,
            ))],
        }
    }

    /// 组合赋值集合。
    /// Combine assignment sets.
    pub fn then(mut self, other: Self) -> Self {
        self.items.extend(other.items);
        self
    }

    /// 添加设置值赋值。
    /// Add a set-value assignment.
    pub fn then_set(mut self, path: impl Into<PropertyPath>, value: T) -> Self {
        self.items
            .push(UpdateAssignment::SetValue(SetValue::new(path, value)));
        self
    }

    /// 添加设置空值赋值。
    /// Add a set-null assignment.
    pub fn then_set_null(mut self, path: impl Into<PropertyPath>) -> Self {
        self.items
            .push(UpdateAssignment::SetNull(SetNull::new(path)));
        self
    }

    /// 添加表达式赋值。
    /// Add an expression assignment.
    pub fn then_set_expr(
        mut self,
        path: impl Into<PropertyPath>,
        expression: ScalarExpression<T>,
    ) -> Self {
        self.items
            .push(UpdateAssignment::SetFromExpression(SetFromExpression::new(
                path, expression,
            )));
        self
    }

    /// 判断赋值集合是否为空。
    /// Check whether the assignment set is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 判断赋值集合是否非空。
    /// Check whether the assignment set is non-empty.
    pub fn is_not_empty(&self) -> bool {
        !self.is_empty()
    }
}

impl<T> std::ops::Add for UpdateAssignments<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.then(rhs)
    }
}

/// 更新赋值项 / Update assignment item
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateAssignment<T = ExpressionValue> {
    /// 设置字段值 / Set a field value
    SetValue(SetValue<T>),
    /// 设置字段为空 / Set a field to null
    SetNull(SetNull),
    /// 通过表达式设置字段值 / Set a field from an expression
    SetFromExpression(SetFromExpression<T>),
}

impl<T> UpdateAssignment<T> {
    /// 获取被更新字段路径。
    /// Get the updated field path.
    pub fn path(&self) -> &PropertyPath {
        match self {
            Self::SetValue(value) => &value.path,
            Self::SetNull(value) => &value.path,
            Self::SetFromExpression(value) => &value.path,
        }
    }
}

/// 设置值赋值项 / Set-value assignment item
#[derive(Debug, Clone, PartialEq)]
pub struct SetValue<T = ExpressionValue> {
    /// 字段路径 / Field path
    pub path: PropertyPath,
    /// 设置值 / Value to set
    pub value: T,
}

impl<T> SetValue<T> {
    /// 创建设置值赋值项。
    /// Create a set-value assignment item.
    pub fn new(path: impl Into<PropertyPath>, value: T) -> Self {
        Self {
            path: path.into(),
            value,
        }
    }
}

/// 设置空值赋值项 / Set-null assignment item
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetNull {
    /// 字段路径 / Field path
    pub path: PropertyPath,
}

impl SetNull {
    /// 创建设置空值赋值项。
    /// Create a set-null assignment item.
    pub fn new(path: impl Into<PropertyPath>) -> Self {
        Self { path: path.into() }
    }
}

/// 表达式赋值项 / Expression assignment item
#[derive(Debug, Clone, PartialEq)]
pub struct SetFromExpression<T = ExpressionValue> {
    /// 字段路径 / Field path
    pub path: PropertyPath,
    /// 赋值表达式 / Assignment expression
    pub expression: ScalarExpression<T>,
}

impl<T> SetFromExpression<T> {
    /// 创建表达式赋值项。
    /// Create an expression assignment item.
    pub fn new(path: impl Into<PropertyPath>, expression: ScalarExpression<T>) -> Self {
        Self {
            path: path.into(),
            expression,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_math::symbol::{ExpressionValue, ScalarExpression};

    #[test]
    fn update_assignments_builds_set_null_and_expression_items() {
        let assignments = UpdateAssignments::set("status", ExpressionValue::from("inactive"))
            .then_set_null("deleted_at")
            .then_set_expr(
                "score",
                ScalarExpression::add_expr(
                    ScalarExpression::reference("score"),
                    ScalarExpression::constant(ExpressionValue::from(1)),
                ),
            );

        assert_eq!(assignments.items.len(), 3);
        assert_eq!(assignments.items[0].path().value(), "status");
        assert_eq!(assignments.items[1].path().value(), "deleted_at");
        assert_eq!(assignments.items[2].path().value(), "score");
        assert!(assignments.is_not_empty());
    }
}
