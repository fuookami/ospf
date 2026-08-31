//! Rbatis 持久化后端
//! Rbatis persistence backend

use ospf_rust_math::symbol::{BooleanExpression, ExpressionValue};
use crate::persistence::{PersistenceFieldResolver, RepositoryQuery, UpdateAssignments};
use super::sqlx::{

    SqlxDialect, SqlxRepositoryStatementBuilder, SqlxSql, SqlxTranslationError,
    SqlxTranslatorConfig,
};

/// Rbatis 后端标记类型。
/// Rbatis backend marker type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RbatisBackend;

/// Rbatis 参数化语句。
/// Rbatis parameterized statement.
#[derive(Debug, Clone, PartialEq)]
pub struct RbatisStatement {
    pub sql: String,
    pub args: Vec<ExpressionValue>,
}

impl From<SqlxSql> for RbatisStatement {
    fn from(value: SqlxSql) -> Self {
        Self {
            sql: value.sql,
            args: value.params,
        }
    }
}

/// Rbatis 仓储语句构建器。
/// Rbatis repository statement builder.
#[derive(Debug, Clone)]
pub struct RbatisRepositoryStatementBuilder<R> {
    inner: SqlxRepositoryStatementBuilder<R>,
}

impl<R> RbatisRepositoryStatementBuilder<R> {
    /// 创建 Rbatis 语句构建器。
    /// Create an Rbatis statement builder.
    pub fn new(table_name: impl Into<String>, resolver: R) -> Self {
        Self {
            inner: SqlxRepositoryStatementBuilder::with_config(
                table_name,
                resolver,
                SqlxTranslatorConfig::default()
                    .with_dialect(SqlxDialect::Generic)
                    .with_quote_identifiers(true),
            ),
        }
    }

    /// 设置查询列。
    /// Set selected columns.
    pub fn with_select_columns<I, S>(mut self, columns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.inner = self.inner.with_select_columns(columns);
        self
    }

    /// 获取内部 SQL 构建器。
    /// Get the inner SQL builder.
    pub fn inner(&self) -> &SqlxRepositoryStatementBuilder<R> {
        &self.inner
    }
}

impl<R> RbatisRepositoryStatementBuilder<R>
where
    R: PersistenceFieldResolver<String>,
{
    /// 构建 SELECT 语句。
    /// Build a SELECT statement.
    pub fn select_statement(
        &self,
        where_expr: &BooleanExpression<ExpressionValue>,
        options: &RepositoryQuery,
    ) -> Result<RbatisStatement, SqlxTranslationError> {
        self.inner
            .select_statement(where_expr, options)
            .map(Into::into)
    }

    /// 构建 COUNT 语句。
    /// Build a COUNT statement.
    pub fn count_statement(
        &self,
        where_expr: &BooleanExpression<ExpressionValue>,
    ) -> Result<RbatisStatement, SqlxTranslationError> {
        self.inner.count_statement(where_expr).map(Into::into)
    }

    /// 构建 UPDATE 语句。
    /// Build an UPDATE statement.
    pub fn update_statement(
        &self,
        where_expr: &BooleanExpression<ExpressionValue>,
        assignments: &UpdateAssignments<ExpressionValue>,
    ) -> Result<RbatisStatement, SqlxTranslationError> {
        self.inner
            .update_statement(where_expr, assignments)
            .map(Into::into)
    }

    /// 构建 DELETE 语句。
    /// Build a DELETE statement.
    pub fn delete_statement(
        &self,
        where_expr: &BooleanExpression<ExpressionValue>,
    ) -> Result<RbatisStatement, SqlxTranslationError> {
        self.inner.delete_statement(where_expr).map(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::{RepositoryQuery, runtime_field};

    #[test]
    fn rbatis_builder_reuses_parameterized_sql_translator() {
        let builder =
            RbatisRepositoryStatementBuilder::new("users", |path: &str| Some(path.to_string()));
        let statement = builder
            .select_statement(
                &runtime_field("status").eq("active"),
                &RepositoryQuery::new().with_limit(2),
            )
            .unwrap();

        assert_eq!(
            statement.sql,
            r#"SELECT * FROM "users" WHERE "status" = ? LIMIT ?"#
        );
        assert_eq!(statement.args, vec!["active".into(), 2.0.into()]);
    }
}
