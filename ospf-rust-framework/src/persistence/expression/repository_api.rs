//! 仓储 API
//! Repository API

use ospf_rust_math::symbol::{BooleanExpression, ExpressionValue};
use super::{SortBy, UpdateAssignments};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RepositoryQuery {
    pub sort_by: Option<SortBy>,
    pub limit: Option<usize>,
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

/// 表达式仓储接口。
/// Expression repository interface.
pub trait ExpressionRepository<E, T = ExpressionValue> {
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
