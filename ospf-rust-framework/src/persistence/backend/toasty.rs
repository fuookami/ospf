//! Toasty 持久化后端
//! Toasty persistence backend

use ospf_rust_math::symbol::{BooleanExpression, ExpressionValue};

use crate::persistence::{RepositoryQuery, UpdateAssignments};

/// Toasty 后端标记类型。
/// Toasty backend marker type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ToastyBackend;

/// Toasty 操作类型。
/// Toasty operation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastyOperation {
    Find,
    Count,
    Update,
    Delete,
}

/// Toasty typed repository 计划。
/// Toasty typed repository plan.
#[derive(Debug, Clone, PartialEq)]
pub struct ToastyRepositoryPlan {
    pub operation: ToastyOperation,
    pub where_expr: BooleanExpression<ExpressionValue>,
    pub options: RepositoryQuery,
    pub assignments: UpdateAssignments<ExpressionValue>,
}

impl ToastyRepositoryPlan {
    /// 创建查询计划。
    /// Create a find plan.
    pub fn find(where_expr: BooleanExpression<ExpressionValue>, options: RepositoryQuery) -> Self {
        Self {
            operation: ToastyOperation::Find,
            where_expr,
            options,
            assignments: UpdateAssignments::empty(),
        }
    }

    /// 创建计数计划。
    /// Create a count plan.
    pub fn count(where_expr: BooleanExpression<ExpressionValue>) -> Self {
        Self {
            operation: ToastyOperation::Count,
            where_expr,
            options: RepositoryQuery::default(),
            assignments: UpdateAssignments::empty(),
        }
    }

    /// 创建更新计划。
    /// Create an update plan.
    pub fn update(
        where_expr: BooleanExpression<ExpressionValue>,
        assignments: UpdateAssignments<ExpressionValue>,
    ) -> Self {
        Self {
            operation: ToastyOperation::Update,
            where_expr,
            options: RepositoryQuery::default(),
            assignments,
        }
    }

    /// 创建删除计划。
    /// Create a delete plan.
    pub fn delete(where_expr: BooleanExpression<ExpressionValue>) -> Self {
        Self {
            operation: ToastyOperation::Delete,
            where_expr,
            options: RepositoryQuery::default(),
            assignments: UpdateAssignments::empty(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::{UpdateAssignments, runtime_field};

    #[test]
    fn toasty_plan_keeps_typed_orm_boundary_explicit() {
        let plan = ToastyRepositoryPlan::update(
            runtime_field("status").eq("active"),
            UpdateAssignments::set("status", "inactive".into()),
        );

        assert_eq!(plan.operation, ToastyOperation::Update);
        assert!(plan.assignments.is_not_empty());
    }
}
