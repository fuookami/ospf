//! 仓储 API
//! Repository API

use super::{SortBy, UpdateAssignments};
use crate::persistence::query::{
    ColumnRef, NullsOrder as QueryNullsOrder, OrderSpec, PageSpec, RelationalQueryPlan,
    RelationalQueryValidationError, SortDirection as QuerySortDirection,
};
use ospf_rust_math::symbol::{BooleanExpression, ExpressionValue};

/// 仓储查询选项。
/// Repository query options.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RepositoryQuery {
    /// 排序描述 / Sort descriptor
    pub sort_by: Option<SortBy>,
    /// 返回数量限制 / Result limit
    pub limit: Option<usize>,
    /// 偏移量 / Result offset
    pub offset: Option<usize>,
}

impl RepositoryQuery {
    /// 创建空查询选项。
    /// Create empty query options.
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置排序。
    /// Set sorting.
    pub fn with_sort_by(mut self, sort_by: SortBy) -> Self {
        self.sort_by = Some(sort_by);
        self
    }

    /// 设置返回数量限制。
    /// Set result limit.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// 设置偏移量。
    /// Set result offset.
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// 从仓储查询选项构造并校验关系查询计划。
/// Build and validate a relational query plan from repository query options.
///
/// 该入口由不同数据库后端共享，确保谓词、排序和分页在进入后端 translator 前具有相同的 RQP 语义。
/// Backends share this entry point so predicates, ordering, and pagination have the same RQP semantics before reaching a backend translator.
pub fn build_relational_query_plan(
    base_plan: RelationalQueryPlan,
    where_expr: &BooleanExpression<ExpressionValue>,
    options: &RepositoryQuery,
) -> Result<RelationalQueryPlan, RelationalQueryValidationError> {
    let predicate = match base_plan.predicate() {
        Some(base_predicate) => {
            BooleanExpression::and(vec![base_predicate.clone(), where_expr.clone()])
        }
        None => where_expr.clone(),
    };
    let mut plan = base_plan.with_predicate(predicate);
    if let Some(sort_by) = &options.sort_by {
        let orders = sort_by
            .items
            .iter()
            .map(|item| {
                OrderSpec::new(ColumnRef::new(plan.root().name.clone(), item.path.value()))
                    .with_direction(match item.direction {
                        super::SortDirection::Asc => QuerySortDirection::Ascending,
                        super::SortDirection::Desc => QuerySortDirection::Descending,
                    })
                    .with_nulls(match item.nulls {
                        Some(super::NullsOrder::NullsFirst) => QueryNullsOrder::First,
                        Some(super::NullsOrder::NullsLast) => QueryNullsOrder::Last,
                        None => QueryNullsOrder::Unspecified,
                    })
            })
            .collect::<Vec<_>>();
        plan = plan.with_order_by(orders);
    }
    if let Some(limit) = options.limit {
        plan = plan.with_page(PageSpec::new(limit, options.offset.unwrap_or(0)));
    } else if let Some(offset) = options.offset {
        plan = plan.with_offset_without_limit(offset);
    }
    plan.validate().map(|()| plan)
}

/// 表达式仓储接口。
/// Expression repository interface.
pub trait ExpressionRepository<E, T = ExpressionValue> {
    /// 错误类型 / Error type
    type Error;

    /// 查询实体。
    /// Find entities.
    fn find(&self, where_expr: &BooleanExpression<T>) -> std::result::Result<Vec<E>, Self::Error> {
        self.find_with_options(where_expr, &RepositoryQuery::default())
    }

    /// 查询实体，带排序和分页选项。
    /// Find entities with sort and pagination options.
    fn find_with_options(
        &self,
        where_expr: &BooleanExpression<T>,
        options: &RepositoryQuery,
    ) -> std::result::Result<Vec<E>, Self::Error>;

    /// 计数。
    /// Count matching entities.
    fn count(&self, where_expr: &BooleanExpression<T>) -> std::result::Result<u64, Self::Error>;

    /// 更新。
    /// Update matching entities.
    fn update(
        &self,
        where_expr: &BooleanExpression<T>,
        assignments: &UpdateAssignments<T>,
    ) -> std::result::Result<u64, Self::Error>;

    /// 删除。
    /// Delete matching entities.
    fn delete(&self, where_expr: &BooleanExpression<T>) -> std::result::Result<u64, Self::Error>;

    /// 检查是否存在。
    /// Check whether any matching entity exists.
    fn exists(&self, where_expr: &BooleanExpression<T>) -> std::result::Result<bool, Self::Error> {
        self.count(where_expr).map(|count| count > 0)
    }
}

/// 兼容旧占位命名。
/// Compatibility alias for the old placeholder name.
pub trait RepositoryApi {}

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_math::symbol::{BooleanExpression, ExpressionValue};

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct User {
        id: u64,
    }

    struct MockRepository {
        count: u64,
    }

    impl ExpressionRepository<User> for MockRepository {
        type Error = String;

        fn find_with_options(
            &self,
            _where_expr: &BooleanExpression<ExpressionValue>,
            options: &RepositoryQuery,
        ) -> std::result::Result<Vec<User>, Self::Error> {
            let limit = options.limit.unwrap_or(1);
            Ok((0..limit as u64).map(|id| User { id }).collect())
        }

        fn count(
            &self,
            _where_expr: &BooleanExpression<ExpressionValue>,
        ) -> std::result::Result<u64, Self::Error> {
            Ok(self.count)
        }

        fn update(
            &self,
            _where_expr: &BooleanExpression<ExpressionValue>,
            _assignments: &UpdateAssignments<ExpressionValue>,
        ) -> std::result::Result<u64, Self::Error> {
            Ok(0)
        }

        fn delete(
            &self,
            _where_expr: &BooleanExpression<ExpressionValue>,
        ) -> std::result::Result<u64, Self::Error> {
            Ok(0)
        }
    }

    #[test]
    fn repository_default_find_and_exists_delegate_to_required_methods() {
        let repository = MockRepository { count: 2 };
        let where_expr = BooleanExpression::true_constant();

        assert_eq!(repository.find(&where_expr).unwrap(), vec![User { id: 0 }]);
        assert_eq!(
            repository
                .find_with_options(&where_expr, &RepositoryQuery::new().with_limit(3))
                .unwrap(),
            vec![User { id: 0 }, User { id: 1 }, User { id: 2 }]
        );
        assert!(repository.exists(&where_expr).unwrap());
    }
}
