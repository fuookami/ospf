//! Diesel 持久化后端
//! Diesel persistence backend

use ospf_rust_math::symbol::{
    BooleanExpression, ExpressionValue, PropertyPath, ScalarExpression,
    property_path_from_owned_symbol,
};

use crate::persistence::{PersistenceFieldResolver, SortBy, UpdateAssignments};

/// Diesel 后端标记类型。
/// Diesel backend marker type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DieselBackend;

/// Diesel typed adapter 计划。
/// Diesel typed adapter plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DieselExpressionPlan<C> {
    /// 已解析字段 / Resolved fields
    pub fields: Vec<C>,
    /// 未解析路径 / Unresolved paths
    pub unresolved_paths: Vec<String>,
    /// 是否需要 boxed query / Whether a boxed query is required
    pub requires_boxed_query: bool,
}

impl<C> Default for DieselExpressionPlan<C> {
    fn default() -> Self {
        Self {
            fields: Vec::new(),
            unresolved_paths: Vec::new(),
            requires_boxed_query: false,
        }
    }
}

impl<C> DieselExpressionPlan<C>
where
    C: PartialEq,
{
    /// 判断所有字段是否均已解析。
    /// Check whether every field has been resolved.
    pub fn is_fully_resolved(&self) -> bool {
        self.unresolved_paths.is_empty()
    }

    fn push_field(&mut self, field: C) {
        if !self.fields.contains(&field) {
            self.fields.push(field);
        }
    }

    fn push_unresolved(&mut self, path: &PropertyPath) {
        let path = path.value().to_string();
        if !self.unresolved_paths.contains(&path) {
            self.unresolved_paths.push(path);
        }
    }
}

/// Diesel 表达式计划构建器。
/// Diesel expression plan builder.
#[derive(Debug, Clone)]
pub struct DieselExpressionPlanner<R> {
    resolver: R,
}

impl<R> DieselExpressionPlanner<R> {
    /// 创建计划构建器。
    /// Create a planner.
    pub fn new(resolver: R) -> Self {
        Self { resolver }
    }
}

impl<R> DieselExpressionPlanner<R> {
    /// 获取 resolver。
    /// Get the resolver.
    pub fn resolver(&self) -> &R {
        &self.resolver
    }
}

impl<R> DieselExpressionPlanner<R> {
    /// 拆出 resolver。
    /// Split into resolver.
    pub fn into_resolver(self) -> R {
        self.resolver
    }
}

impl<R> DieselExpressionPlanner<R> {
    /// 从查询谓词与排序创建字段计划。
    /// Create a field plan from predicate and sorting.
    pub fn plan_query<C>(
        &self,
        where_expr: &BooleanExpression<ExpressionValue>,
        sort_by: Option<&SortBy>,
    ) -> DieselExpressionPlan<C>
    where
        R: PersistenceFieldResolver<C>,
        C: PartialEq,
    {
        let mut plan = DieselExpressionPlan::default();
        self.collect_boolean(where_expr, &mut plan);
        if let Some(sort_by) = sort_by {
            for item in &sort_by.items {
                self.resolve_path(&item.path, &mut plan);
            }
        }
        plan.requires_boxed_query = true;
        plan
    }

    /// 从更新赋值创建字段计划。
    /// Create a field plan from update assignments.
    pub fn plan_update<C>(
        &self,
        where_expr: &BooleanExpression<ExpressionValue>,
        assignments: &UpdateAssignments<ExpressionValue>,
    ) -> DieselExpressionPlan<C>
    where
        R: PersistenceFieldResolver<C>,
        C: PartialEq,
    {
        let mut plan = DieselExpressionPlan::default();
        self.collect_boolean(where_expr, &mut plan);
        for assignment in &assignments.items {
            self.resolve_path(assignment.path(), &mut plan);
        }
        plan.requires_boxed_query = true;
        plan
    }

    fn resolve_path<C>(&self, path: &PropertyPath, plan: &mut DieselExpressionPlan<C>)
    where
        R: PersistenceFieldResolver<C>,
        C: PartialEq,
    {
        if let Some(field) = self.resolver.resolve_field(path) {
            plan.push_field(field);
        } else {
            plan.push_unresolved(path);
        }
    }

    fn collect_boolean<C>(
        &self,
        expression: &BooleanExpression<ExpressionValue>,
        plan: &mut DieselExpressionPlan<C>,
    ) where
        R: PersistenceFieldResolver<C>,
        C: PartialEq,
    {
        match expression {
            BooleanExpression::Comparison { left, right, .. } => {
                self.collect_scalar(left, plan);
                self.collect_scalar(right, plan);
            }
            BooleanExpression::In {
                value, candidates, ..
            } => {
                self.collect_scalar(value, plan);
                for candidate in candidates {
                    self.collect_scalar(candidate, plan);
                }
            }
            BooleanExpression::PatternMatch { value, pattern, .. } => {
                self.collect_scalar(value, plan);
                self.collect_scalar(pattern, plan);
            }
            BooleanExpression::NullCheck { path, .. } => self.resolve_path(path, plan),
            BooleanExpression::And(operands) | BooleanExpression::Or(operands) => {
                for operand in operands {
                    self.collect_boolean(operand, plan);
                }
            }
            BooleanExpression::Not(operand) => self.collect_boolean(operand, plan),
            BooleanExpression::Constant(_) | BooleanExpression::Custom { .. } => {}
        }
    }

    fn collect_scalar<C>(
        &self,
        expression: &ScalarExpression<ExpressionValue>,
        plan: &mut DieselExpressionPlan<C>,
    ) where
        R: PersistenceFieldResolver<C>,
        C: PartialEq,
    {
        match expression {
            ScalarExpression::Reference(path) => self.resolve_path(path, plan),
            ScalarExpression::SymbolReference(symbol) => {
                if let Some(path) = property_path_from_owned_symbol(symbol) {
                    self.resolve_path(path, plan);
                }
            }
            ScalarExpression::Unary { operand, .. } => self.collect_scalar(operand, plan),
            ScalarExpression::Binary { left, right, .. } => {
                self.collect_scalar(left, plan);
                self.collect_scalar(right, plan);
            }
            ScalarExpression::Function { arguments, .. } => {
                for argument in arguments {
                    self.collect_scalar(argument, plan);
                }
            }
            ScalarExpression::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_boolean(condition, plan);
                self.collect_scalar(then_branch, plan);
                self.collect_scalar(else_branch, plan);
            }
            ScalarExpression::Boolean(expression) => self.collect_boolean(expression, plan),
            ScalarExpression::Constant(_) | ScalarExpression::Custom { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::{SortBy, runtime_field};

    #[test]
    fn diesel_planner_collects_resolved_and_unresolved_fields() {
        let planner = DieselExpressionPlanner::new(|path: &str| match path {
            "status" => Some("users.status"),
            "age" => Some("users.age"),
            _ => None,
        });
        let expression = runtime_field("status").eq("active") & runtime_field("age").ge(18);

        let plan = planner.plan_query(&expression, Some(&SortBy::asc("missing")));

        assert_eq!(plan.fields, vec!["users.status", "users.age"]);
        assert_eq!(plan.unresolved_paths, vec!["missing"]);
        assert!(plan.requires_boxed_query);
        assert!(!plan.is_fully_resolved());
    }

    #[test]
    fn diesel_planner_collects_fields_from_conditional_and_boolean_scalars() {
        let planner = DieselExpressionPlanner::new(|path: &str| match path {
            "status" => Some("users.status"),
            "score" => Some("users.score"),
            "age" => Some("users.age"),
            _ => None,
        });
        let conditional = ScalarExpression::conditional(
            runtime_field("status").eq("active"),
            ScalarExpression::reference("score"),
            ScalarExpression::boolean_expr(runtime_field("age").ge(18)),
        );
        let expression = BooleanExpression::eq(
            conditional,
            ScalarExpression::constant(ExpressionValue::from(1)),
        );

        let plan = planner.plan_query(&expression, None);

        assert_eq!(
            plan.fields,
            vec!["users.status", "users.score", "users.age"]
        );
        assert!(plan.is_fully_resolved());
    }
}
