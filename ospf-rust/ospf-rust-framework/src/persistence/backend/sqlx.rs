//! SQLx 持久化后端
//! SQLx persistence backend

use ospf_rust_math::Trivalent;
use ospf_rust_math::symbol::{
    BinaryOperator, BooleanExpression, ComparisonOperator, ExpressionValue, NullCheckType,
    PatternMatchMode, ScalarExpression, ScalarFunctionNames, UnaryOperator,
    property_path_from_owned_symbol,
};
use std::fmt::{Display, Formatter};

use crate::persistence::{
    NullsOrder, NullsOrderSupport, PersistenceFieldResolver, SetFromExpression, SetNull, SetValue,
    SortBy, SortDirection, SortItem, UnsupportedPredicatePolicy, UpdateAssignment,
    UpdateAssignments,
};

#[path = "sqlx_relational_query.rs"]
mod sqlx_relational_query;
pub use sqlx_relational_query::*;

/// SQLx 后端标记类型。
/// SQLx backend marker type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SqlxBackend;

/// SQL 方言。
/// SQL dialect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlxDialect {
    /// 通用方言 / Generic dialect
    Generic,
    /// PostgreSQL 方言 / PostgreSQL dialect
    Postgres,
    /// MySQL 方言 / MySQL dialect
    MySql,
    /// SQLite 方言 / SQLite dialect
    Sqlite,
}

impl SqlxDialect {
    /// 返回占位符。
    /// Return a placeholder.
    pub fn placeholder(self, index: usize) -> String {
        match self {
            Self::Postgres => format!("${index}"),
            Self::Generic | Self::MySql | Self::Sqlite => "?".to_string(),
        }
    }

    /// 引用标识符。
    /// Quote an identifier.
    pub fn quote_identifier(self, identifier: &str) -> String {
        let quote = match self {
            Self::MySql => '`',
            Self::Generic | Self::Postgres | Self::Sqlite => '"',
        };
        identifier
            .split('.')
            .map(|segment| {
                let escaped = segment.replace(quote, &format!("{quote}{quote}"));
                format!("{quote}{escaped}{quote}")
            })
            .collect::<Vec<_>>()
            .join(".")
    }
}

impl Default for SqlxDialect {
    fn default() -> Self {
        Self::Generic
    }
}

/// SQLx 查询列的逻辑类型。
/// Logical type of a SQLx query column.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SqlxValueType {
    /// 未声明类型，允许 SQLx 驱动自行处理。
    /// Unspecified type, leaving handling to the SQLx driver.
    Any,
    /// 空值类型 / Null type
    Null,
    /// 布尔类型 / Boolean type
    Boolean,
    /// 数值类型 / Numeric type
    Number,
    /// 字符串类型 / String type
    String,
    /// 适配器自定义类型 / Adapter-defined type
    Custom(String),
}

impl SqlxValueType {
    /// 从表达式运行时值推断逻辑类型。
    /// Infer a logical type from an expression runtime value.
    pub fn from_value(value: &ExpressionValue) -> Self {
        match value {
            ExpressionValue::Null => Self::Null,
            ExpressionValue::Boolean(_) => Self::Boolean,
            ExpressionValue::Number(_) => Self::Number,
            ExpressionValue::String(_) => Self::String,
        }
    }

    /// 返回审计摘要中的稳定类型名称。
    /// Return a stable type name for audit summaries.
    pub fn name(&self) -> &str {
        match self {
            Self::Any => "any",
            Self::Null => "null",
            Self::Boolean => "boolean",
            Self::Number => "number",
            Self::String => "string",
            Self::Custom(name) => name.as_str(),
        }
    }

    /// 判断类型是否接受表达式运行时值。
    /// Check whether this type accepts an expression runtime value.
    pub fn accepts(&self, value: &ExpressionValue) -> bool {
        match self {
            Self::Any => true,
            Self::Null => matches!(value, ExpressionValue::Null),
            Self::Boolean => matches!(value, ExpressionValue::Boolean(_)),
            Self::Number => matches!(value, ExpressionValue::Number(_)),
            Self::String => matches!(value, ExpressionValue::String(_)),
            Self::Custom(_) => false,
        }
    }

    fn compatible_with(&self, other: &Self) -> bool {
        matches!(self, Self::Any) || matches!(other, Self::Any) || self == other
    }
}

/// SQLx 目标列常量绑定器。
/// SQLx target-column constant binder.
///
/// 关系查询计划只携带 `ExpressionValue`，领域值的转换应在适配器边界完成。
/// Relational plans carry only `ExpressionValue`; domain-value conversion belongs at the adapter boundary.
pub trait SqlxTargetConstantBinder: Send + Sync {
    /// 按目标列类型绑定常量，返回转换后的参数值。
    /// Bind a constant for a target column type and return its converted parameter value.
    fn bind(
        &self,
        value: &ExpressionValue,
        target: &SqlxValueType,
    ) -> Result<Option<ExpressionValue>, String>;
}

impl<F> SqlxTargetConstantBinder for F
where
    F: Fn(&ExpressionValue, &SqlxValueType) -> Result<Option<ExpressionValue>, String>
        + Send
        + Sync,
{
    fn bind(
        &self,
        value: &ExpressionValue,
        target: &SqlxValueType,
    ) -> Result<Option<ExpressionValue>, String> {
        self(value, target)
    }
}

/// SQLx translator 配置。
/// SQLx translator configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SqlxTranslatorConfig {
    /// SQL 方言 / SQL dialect
    pub dialect: SqlxDialect,
    /// 是否引用标识符 / Whether identifiers should be quoted
    pub quote_identifiers: bool,
    /// 不支持谓词策略 / Unsupported predicate policy
    pub unsupported_predicate_policy: UnsupportedPredicatePolicy,
    /// 空值排序支持策略 / Null ordering support policy
    pub nulls_order_support: NullsOrderSupport,
}

impl Default for SqlxTranslatorConfig {
    fn default() -> Self {
        Self {
            dialect: SqlxDialect::Generic,
            quote_identifiers: true,
            unsupported_predicate_policy: UnsupportedPredicatePolicy::default(),
            nulls_order_support: NullsOrderSupport::default(),
        }
    }
}

impl SqlxTranslatorConfig {
    /// 设置 SQL 方言。
    /// Set SQL dialect.
    pub fn with_dialect(mut self, dialect: SqlxDialect) -> Self {
        self.dialect = dialect;
        self
    }

    /// 设置是否引用标识符。
    /// Set whether identifiers should be quoted.
    pub fn with_quote_identifiers(mut self, quote_identifiers: bool) -> Self {
        self.quote_identifiers = quote_identifiers;
        self
    }

    /// 设置不支持谓词策略。
    /// Set unsupported predicate policy.
    pub fn with_unsupported_predicate_policy(
        mut self,
        unsupported_predicate_policy: UnsupportedPredicatePolicy,
    ) -> Self {
        self.unsupported_predicate_policy = unsupported_predicate_policy;
        self
    }

    /// 设置空值排序支持策略。
    /// Set null ordering support policy.
    pub fn with_nulls_order_support(mut self, nulls_order_support: NullsOrderSupport) -> Self {
        self.nulls_order_support = nulls_order_support;
        self
    }
}

/// SQL 片段。
/// SQL fragment.
#[derive(Debug, Clone, PartialEq)]
pub struct SqlxSql {
    /// SQL 语句 / SQL statement
    pub sql: String,
    /// 参数列表 / Parameter list
    pub params: Vec<ExpressionValue>,
}

/// 带参数类型信息的 SQLx 翻译结果。
/// SQLx translation result carrying parameter type information.
#[derive(Debug, Clone, PartialEq)]
pub struct SqlxTypedSql {
    /// SQL 语句 / SQL statement
    pub sql: String,
    /// 参数列表 / Parameter list
    pub params: Vec<ExpressionValue>,
    /// 参数逻辑类型 / Logical parameter types
    pub parameter_types: Vec<SqlxValueType>,
}

impl SqlxSql {
    /// 创建 SQL 片段。
    /// Create a SQL fragment.
    pub fn new(sql: impl Into<String>, params: Vec<ExpressionValue>) -> Self {
        Self {
            sql: sql.into(),
            params,
        }
    }

    /// 创建无参数 SQL 片段。
    /// Create a SQL fragment without parameters.
    pub fn raw(sql: impl Into<String>) -> Self {
        Self::new(sql, Vec::new())
    }
}

/// SQLx 翻译错误。
/// SQLx translation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SqlxTranslationError {
    /// 不支持的谓词 / Unsupported predicate
    UnsupportedPredicate(String),
    /// 参数绑定失败 / Parameter binding failure
    ParameterBinding(String),
    /// 未解析的字段 / Unresolved field
    UnresolvedField(String),
    /// 无效表达式 / Invalid expression
    InvalidExpression(String),
}

impl Display for SqlxTranslationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPredicate(message) => write!(f, "unsupported predicate: {message}"),
            Self::ParameterBinding(message) => write!(f, "parameter binding failed: {message}"),
            Self::UnresolvedField(message) => write!(f, "unresolved field: {message}"),
            Self::InvalidExpression(message) => write!(f, "invalid expression: {message}"),
        }
    }
}

impl std::error::Error for SqlxTranslationError {}

/// SQLx 表达式翻译器。
/// SQLx expression translator.
#[derive(Clone)]
pub struct SqlxTranslator<R> {
    resolver: R,
    config: SqlxTranslatorConfig,
    type_resolver: Option<
        std::sync::Arc<
            dyn Fn(&ospf_rust_math::symbol::PropertyPath) -> Option<SqlxValueType> + Send + Sync,
        >,
    >,
    target_constant_binder: Option<std::sync::Arc<dyn SqlxTargetConstantBinder>>,
}

impl<R> std::fmt::Debug for SqlxTranslator<R> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SqlxTranslator")
            .field("config", &self.config)
            .field("has_type_resolver", &self.type_resolver.is_some())
            .field(
                "has_target_constant_binder",
                &self.target_constant_binder.is_some(),
            )
            .finish()
    }
}

impl<R> SqlxTranslator<R> {
    /// 创建翻译器。
    /// Create a translator.
    pub fn new(resolver: R) -> Self {
        Self {
            resolver,
            config: SqlxTranslatorConfig::default(),
            type_resolver: None,
            target_constant_binder: None,
        }
    }

    /// 使用配置创建翻译器。
    /// Create a translator with configuration.
    pub fn with_config(resolver: R, config: SqlxTranslatorConfig) -> Self {
        Self {
            resolver,
            config,
            type_resolver: None,
            target_constant_binder: None,
        }
    }

    /// 获取配置。
    /// Get configuration.
    pub fn config(&self) -> SqlxTranslatorConfig {
        self.config
    }

    /// 设置配置。
    /// Set configuration.
    pub fn set_config(&mut self, config: SqlxTranslatorConfig) {
        self.config = config;
    }

    /// 设置字段类型解析器。
    /// Set the field type resolver.
    pub fn with_type_resolver<F>(mut self, resolver: F) -> Self
    where
        F: Fn(&ospf_rust_math::symbol::PropertyPath) -> Option<SqlxValueType>
            + Send
            + Sync
            + 'static,
    {
        self.type_resolver = Some(std::sync::Arc::new(resolver));
        self
    }

    /// 设置目标列常量绑定器。
    /// Set the target-column constant binder.
    pub fn with_target_constant_binder<B>(mut self, binder: B) -> Self
    where
        B: SqlxTargetConstantBinder + 'static,
    {
        self.target_constant_binder = Some(std::sync::Arc::new(binder));
        self
    }
}

impl<R> SqlxTranslator<R>
where
    R: PersistenceFieldResolver<String>,
{
    /// 翻译标量表达式。
    /// Translate a scalar expression.
    pub fn translate_scalar(
        &self,
        expression: &ScalarExpression<ExpressionValue>,
    ) -> Result<SqlxSql, SqlxTranslationError> {
        let mut ctx = SqlxTranslationContext::new(self);
        ctx.translate_scalar(expression)
    }

    /// 翻译标量表达式并返回参数逻辑类型。
    /// Translate a scalar expression and return logical parameter types.
    pub fn translate_scalar_typed(
        &self,
        expression: &ScalarExpression<ExpressionValue>,
    ) -> Result<SqlxTypedSql, SqlxTranslationError> {
        let mut ctx = SqlxTranslationContext::new(self);
        ctx.translate_scalar_typed(expression)
    }

    /// 翻译布尔表达式。
    /// Translate a boolean expression.
    pub fn translate_boolean(
        &self,
        expression: &BooleanExpression<ExpressionValue>,
    ) -> Result<SqlxSql, SqlxTranslationError> {
        let mut ctx = SqlxTranslationContext::new(self);
        ctx.translate_boolean(expression)
    }

    /// 翻译布尔表达式并返回参数逻辑类型。
    /// Translate a boolean expression and return logical parameter types.
    pub fn translate_boolean_typed(
        &self,
        expression: &BooleanExpression<ExpressionValue>,
    ) -> Result<SqlxTypedSql, SqlxTranslationError> {
        let mut ctx = SqlxTranslationContext::new(self);
        ctx.translate_boolean_typed(expression)
    }

    /// 使用临时字段类型解析器和目标绑定器翻译布尔表达式。
    /// Translate a boolean expression with temporary field-type and target-binder adapters.
    pub fn translate_boolean_with_context(
        &self,
        expression: &BooleanExpression<ExpressionValue>,
        type_resolver: &dyn Fn(&ospf_rust_math::symbol::PropertyPath) -> Option<SqlxValueType>,
        target_constant_binder: Option<&dyn SqlxTargetConstantBinder>,
    ) -> Result<SqlxTypedSql, SqlxTranslationError> {
        let mut ctx = SqlxTranslationContext::with_overrides(
            self,
            Some(type_resolver),
            target_constant_binder,
        );
        ctx.translate_boolean_typed(expression)
    }

    /// 翻译排序。
    /// Translate sorting.
    pub fn translate_sort_by(&self, sort_by: &SortBy) -> Result<String, SqlxTranslationError> {
        if sort_by.is_empty() {
            return Ok(String::new());
        }
        let mut parts = Vec::new();
        for item in &sort_by.items {
            parts.extend(self.translate_sort_item(item)?);
        }
        Ok(parts.join(", "))
    }

    /// 翻译更新赋值。
    /// Translate update assignments.
    pub fn translate_update_assignments(
        &self,
        assignments: &UpdateAssignments<ExpressionValue>,
    ) -> Result<SqlxSql, SqlxTranslationError> {
        let mut ctx = SqlxTranslationContext::new(self);
        ctx.translate_update_assignments(assignments)
    }

    fn resolve_column(
        &self,
        path: &ospf_rust_math::symbol::PropertyPath,
    ) -> Result<String, SqlxTranslationError> {
        let field = self
            .resolver
            .resolve_field(path)
            .ok_or_else(|| SqlxTranslationError::UnresolvedField(path.value().to_string()))?;
        Ok(if self.config.quote_identifiers {
            self.config.dialect.quote_identifier(&field)
        } else {
            field
        })
    }

    fn translate_sort_item(&self, item: &SortItem) -> Result<Vec<String>, SqlxTranslationError> {
        let column = self.resolve_column(&item.path)?;
        let direction = match item.direction {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        };
        let mut parts = Vec::new();
        if let Some(nulls) = &item.nulls {
            if self.config.nulls_order_support.is_supported(item) {
                let nulls_sql = match nulls {
                    NullsOrder::NullsFirst => "NULLS FIRST",
                    NullsOrder::NullsLast => "NULLS LAST",
                };
                parts.push(format!("{column} {direction} {nulls_sql}"));
            } else {
                let null_rank = match nulls {
                    NullsOrder::NullsFirst => {
                        format!("CASE WHEN {column} IS NULL THEN 0 ELSE 1 END ASC")
                    }
                    NullsOrder::NullsLast => {
                        format!("CASE WHEN {column} IS NULL THEN 1 ELSE 0 END ASC")
                    }
                };
                parts.push(null_rank);
                parts.push(format!("{column} {direction}"));
            }
        } else {
            parts.push(format!("{column} {direction}"));
        }
        Ok(parts)
    }
}

/// SQLx repository statement builder.
/// SQLx 仓储语句构建器。
#[derive(Debug, Clone)]
pub struct SqlxRepositoryStatementBuilder<R> {
    table_name: String,
    select_columns: Vec<String>,
    translator: SqlxTranslator<R>,
}

impl<R> SqlxRepositoryStatementBuilder<R> {
    /// 创建仓储语句构建器。
    /// Create a repository statement builder.
    pub fn new(table_name: impl Into<String>, resolver: R) -> Self {
        Self {
            table_name: table_name.into(),
            select_columns: Vec::new(),
            translator: SqlxTranslator::new(resolver),
        }
    }

    /// 使用配置创建仓储语句构建器。
    /// Create a repository statement builder with configuration.
    pub fn with_config(
        table_name: impl Into<String>,
        resolver: R,
        config: SqlxTranslatorConfig,
    ) -> Self {
        Self {
            table_name: table_name.into(),
            select_columns: Vec::new(),
            translator: SqlxTranslator::with_config(resolver, config),
        }
    }

    /// 设置查询列。
    /// Set selected columns.
    pub fn with_select_columns<I, S>(mut self, columns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.select_columns = columns.into_iter().map(Into::into).collect();
        self
    }

    /// 获取内部 translator。
    /// Get inner translator.
    pub fn translator(&self) -> &SqlxTranslator<R> {
        &self.translator
    }

    /// 获取内部 translator 的可变引用。
    /// Get a mutable reference to the inner translator.
    pub fn translator_mut(&mut self) -> &mut SqlxTranslator<R> {
        &mut self.translator
    }
}

impl<R> SqlxRepositoryStatementBuilder<R>
where
    R: PersistenceFieldResolver<String>,
{
    /// 构建 SELECT 语句。
    /// Build a SELECT statement.
    pub fn select_statement(
        &self,
        where_expr: &BooleanExpression<ExpressionValue>,
        options: &crate::persistence::RepositoryQuery,
    ) -> Result<SqlxSql, SqlxTranslationError> {
        let mut ctx = SqlxTranslationContext::new(&self.translator);
        let where_sql = ctx.translate_boolean_sql(where_expr)?;
        let columns = self.select_columns_sql();
        let mut sql = format!(
            "SELECT {columns} FROM {} WHERE {where_sql}",
            self.table_name_sql()
        );
        if let Some(sort_by) = &options.sort_by {
            let order_by = self.translator.translate_sort_by(sort_by)?;
            if !order_by.is_empty() {
                sql.push_str(" ORDER BY ");
                sql.push_str(&order_by);
            }
        }
        self.push_limit_offset(&mut ctx, &mut sql, options);
        Ok(ctx.finish(sql))
    }

    /// 构建 COUNT 语句。
    /// Build a COUNT statement.
    pub fn count_statement(
        &self,
        where_expr: &BooleanExpression<ExpressionValue>,
    ) -> Result<SqlxSql, SqlxTranslationError> {
        let mut ctx = SqlxTranslationContext::new(&self.translator);
        let where_sql = ctx.translate_boolean_sql(where_expr)?;
        Ok(ctx.finish(format!(
            "SELECT COUNT(*) FROM {} WHERE {where_sql}",
            self.table_name_sql()
        )))
    }

    /// 构建 UPDATE 语句。
    /// Build an UPDATE statement.
    pub fn update_statement(
        &self,
        where_expr: &BooleanExpression<ExpressionValue>,
        assignments: &UpdateAssignments<ExpressionValue>,
    ) -> Result<SqlxSql, SqlxTranslationError> {
        if assignments.is_empty() {
            return Err(SqlxTranslationError::InvalidExpression(
                "update assignments must not be empty".to_string(),
            ));
        }
        let mut ctx = SqlxTranslationContext::new(&self.translator);
        let assignments_sql = ctx.translate_update_assignments_sql(assignments)?;
        let where_sql = ctx.translate_boolean_sql(where_expr)?;
        Ok(ctx.finish(format!(
            "UPDATE {} SET {assignments_sql} WHERE {where_sql}",
            self.table_name_sql()
        )))
    }

    /// 构建 DELETE 语句。
    /// Build a DELETE statement.
    pub fn delete_statement(
        &self,
        where_expr: &BooleanExpression<ExpressionValue>,
    ) -> Result<SqlxSql, SqlxTranslationError> {
        let mut ctx = SqlxTranslationContext::new(&self.translator);
        let where_sql = ctx.translate_boolean_sql(where_expr)?;
        Ok(ctx.finish(format!(
            "DELETE FROM {} WHERE {where_sql}",
            self.table_name_sql()
        )))
    }

    fn table_name_sql(&self) -> String {
        if self.translator.config.quote_identifiers {
            self.translator
                .config
                .dialect
                .quote_identifier(&self.table_name)
        } else {
            self.table_name.clone()
        }
    }

    fn select_columns_sql(&self) -> String {
        if self.select_columns.is_empty() {
            "*".to_string()
        } else if self.translator.config.quote_identifiers {
            self.select_columns
                .iter()
                .map(|column| self.translator.config.dialect.quote_identifier(column))
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            self.select_columns.join(", ")
        }
    }

    fn push_limit_offset(
        &self,
        ctx: &mut SqlxTranslationContext<'_, R>,
        sql: &mut String,
        options: &crate::persistence::RepositoryQuery,
    ) {
        if let Some(limit) = options.limit {
            let placeholder = ctx.push_param(ExpressionValue::from(limit));
            sql.push_str(" LIMIT ");
            sql.push_str(&placeholder);
        }
        if let Some(offset) = options.offset {
            let placeholder = ctx.push_param(ExpressionValue::from(offset));
            sql.push_str(" OFFSET ");
            sql.push_str(&placeholder);
        }
    }
}

struct SqlxTranslationContext<'a, R> {
    translator: &'a SqlxTranslator<R>,
    params: Vec<ExpressionValue>,
    parameter_types: Vec<SqlxValueType>,
    type_resolver:
        Option<&'a dyn Fn(&ospf_rust_math::symbol::PropertyPath) -> Option<SqlxValueType>>,
    target_constant_binder: Option<&'a dyn SqlxTargetConstantBinder>,
}

impl<'a, R> SqlxTranslationContext<'a, R>
where
    R: PersistenceFieldResolver<String>,
{
    fn new(translator: &'a SqlxTranslator<R>) -> Self {
        Self {
            translator,
            params: Vec::new(),
            parameter_types: Vec::new(),
            type_resolver: None,
            target_constant_binder: None,
        }
    }

    fn with_overrides(
        translator: &'a SqlxTranslator<R>,
        type_resolver: Option<
            &'a dyn Fn(&ospf_rust_math::symbol::PropertyPath) -> Option<SqlxValueType>,
        >,
        target_constant_binder: Option<&'a dyn SqlxTargetConstantBinder>,
    ) -> Self {
        Self {
            translator,
            params: Vec::new(),
            parameter_types: Vec::new(),
            type_resolver,
            target_constant_binder,
        }
    }

    fn finish(&self, sql: impl Into<String>) -> SqlxSql {
        SqlxSql::new(sql, self.params.clone())
    }

    fn finish_typed(&self, sql: impl Into<String>) -> SqlxTypedSql {
        SqlxTypedSql {
            sql: sql.into(),
            params: self.params.clone(),
            parameter_types: self.parameter_types.clone(),
        }
    }

    fn push_typed_param(
        &mut self,
        value: ExpressionValue,
        parameter_type: Option<SqlxValueType>,
    ) -> String {
        self.params.push(value);
        let inferred_type = SqlxValueType::from_value(&self.params[self.params.len() - 1]);
        self.parameter_types
            .push(parameter_type.unwrap_or(inferred_type));
        self.translator
            .config
            .dialect
            .placeholder(self.params.len())
    }

    fn push_param(&mut self, value: ExpressionValue) -> String {
        self.push_typed_param(value, None)
    }

    fn push_bound_param(
        &mut self,
        value: &ExpressionValue,
        target: Option<&SqlxValueType>,
    ) -> Result<String, SqlxTranslationError> {
        let bound = self.bind_constant(value, target)?;
        Ok(self.push_typed_param(bound, target.cloned()))
    }

    fn resolve_value_type(
        &self,
        path: &ospf_rust_math::symbol::PropertyPath,
    ) -> Option<SqlxValueType> {
        if let Some(resolver) = self.type_resolver {
            resolver(path)
        } else {
            self.translator
                .type_resolver
                .as_deref()
                .and_then(|resolver| resolver(path))
        }
    }

    fn target_constant_binder(&self) -> Option<&dyn SqlxTargetConstantBinder> {
        self.target_constant_binder.or_else(|| {
            self.translator
                .target_constant_binder
                .as_deref()
                .map(|binder| binder as &dyn SqlxTargetConstantBinder)
        })
    }

    fn bind_constant(
        &self,
        value: &ExpressionValue,
        target: Option<&SqlxValueType>,
    ) -> Result<ExpressionValue, SqlxTranslationError> {
        let Some(target) = target else {
            return Ok(value.clone());
        };
        if let Some(binder) = self.target_constant_binder() {
            match binder.bind(value, target) {
                Ok(Some(bound)) => {
                    let accepts =
                        target.accepts(&bound) || matches!(target, SqlxValueType::Custom(_));
                    if accepts {
                        return Ok(bound);
                    }
                    return Err(SqlxTranslationError::ParameterBinding(format!(
                        "target {} rejected the bound {} value",
                        target.name(),
                        SqlxValueType::from_value(&bound).name(),
                    )));
                }
                Ok(None) => {}
                Err(reason) => {
                    return Err(SqlxTranslationError::ParameterBinding(format!(
                        "failed to bind {} for target {}: {reason}",
                        SqlxValueType::from_value(value).name(),
                        target.name(),
                    )));
                }
            }
        }
        if target.accepts(value) {
            Ok(value.clone())
        } else {
            Err(SqlxTranslationError::ParameterBinding(format!(
                "no compatible binding for {} with target {}",
                SqlxValueType::from_value(value).name(),
                target.name(),
            )))
        }
    }

    fn scalar_reference_path(
        expression: &ScalarExpression<ExpressionValue>,
    ) -> Option<ospf_rust_math::symbol::PropertyPath> {
        match expression {
            ScalarExpression::Reference(path) => Some(path.clone()),
            ScalarExpression::SymbolReference(symbol) => {
                property_path_from_owned_symbol(symbol).cloned()
            }
            _ => None,
        }
    }

    fn scalar_expression_type(
        &self,
        expression: &ScalarExpression<ExpressionValue>,
    ) -> Option<SqlxValueType> {
        match expression {
            ScalarExpression::Constant(value) => Some(SqlxValueType::from_value(value)),
            ScalarExpression::Reference(path) => self.resolve_value_type(path),
            ScalarExpression::SymbolReference(symbol) => property_path_from_owned_symbol(symbol)
                .and_then(|path| self.resolve_value_type(path)),
            ScalarExpression::Unary { operand, .. } => self.scalar_expression_type(operand),
            ScalarExpression::Binary { left, right, .. } => {
                let left_type = self.scalar_expression_type(left);
                let right_type = self.scalar_expression_type(right);
                match (left_type, right_type) {
                    (Some(left), Some(right)) if left.compatible_with(&right) => Some(left),
                    (Some(left), None) => Some(left),
                    (None, Some(right)) => Some(right),
                    _ => None,
                }
            }
            ScalarExpression::Function { name, arguments } => {
                let lowered = name.to_ascii_lowercase();
                match lowered.as_str() {
                    ScalarFunctionNames::LOWER
                    | ScalarFunctionNames::UPPER
                    | ScalarFunctionNames::TRIM => Some(SqlxValueType::String),
                    ScalarFunctionNames::LENGTH => Some(SqlxValueType::Number),
                    ScalarFunctionNames::ABS => arguments
                        .first()
                        .and_then(|argument| self.scalar_expression_type(argument)),
                    ScalarFunctionNames::COALESCE => arguments
                        .iter()
                        .find_map(|argument| self.scalar_expression_type(argument)),
                    _ => None,
                }
            }
            ScalarExpression::Conditional {
                then_branch,
                else_branch,
                ..
            } => {
                let then_type = self.scalar_expression_type(then_branch);
                let else_type = self.scalar_expression_type(else_branch);
                match (then_type, else_type) {
                    (Some(then_type), Some(else_type)) if then_type.compatible_with(&else_type) => {
                        Some(then_type)
                    }
                    (Some(then_type), None) => Some(then_type),
                    (None, Some(else_type)) => Some(else_type),
                    _ => None,
                }
            }
            ScalarExpression::Boolean(_) => Some(SqlxValueType::Boolean),
            ScalarExpression::Custom { .. } => None,
        }
    }

    fn translate_scalar(
        &mut self,
        expression: &ScalarExpression<ExpressionValue>,
    ) -> Result<SqlxSql, SqlxTranslationError> {
        let sql = self.translate_scalar_sql(expression)?;
        Ok(self.finish(sql))
    }

    fn translate_scalar_typed(
        &mut self,
        expression: &ScalarExpression<ExpressionValue>,
    ) -> Result<SqlxTypedSql, SqlxTranslationError> {
        let sql = self.translate_scalar_sql(expression)?;
        Ok(self.finish_typed(sql))
    }

    fn translate_scalar_sql(
        &mut self,
        expression: &ScalarExpression<ExpressionValue>,
    ) -> Result<String, SqlxTranslationError> {
        match expression {
            ScalarExpression::Constant(value) => self.push_bound_param(value, None),
            ScalarExpression::Reference(path) => self.translator.resolve_column(path),
            ScalarExpression::SymbolReference(symbol) => {
                let path = property_path_from_owned_symbol(symbol).ok_or_else(|| {
                    SqlxTranslationError::UnsupportedPredicate(format!(
                        "symbol reference '{}' is not a property path",
                        symbol.display_name()
                    ))
                })?;
                self.translator.resolve_column(path)
            }
            ScalarExpression::Unary { operator, operand } => {
                let operand = self.translate_scalar_sql(operand)?;
                match operator {
                    UnaryOperator::Negate => Ok(format!("(-{operand})")),
                    UnaryOperator::Positive => Ok(format!("(+{operand})")),
                    UnaryOperator::Abs => Ok(format!("ABS({operand})")),
                }
            }
            ScalarExpression::Binary {
                operator,
                left,
                right,
            } => {
                let left = self.translate_scalar_sql(left)?;
                let right = self.translate_scalar_sql(right)?;
                let operator = match operator {
                    BinaryOperator::Add => "+",
                    BinaryOperator::Subtract => "-",
                    BinaryOperator::Multiply => "*",
                    BinaryOperator::Divide => "/",
                    BinaryOperator::Modulo => "%",
                    BinaryOperator::Power => {
                        return Ok(format!("POWER({left}, {right})"));
                    }
                };
                Ok(format!("({left} {operator} {right})"))
            }
            ScalarExpression::Function { name, arguments } => {
                self.translate_scalar_function(name, arguments)
            }
            ScalarExpression::Conditional { .. } => {
                Err(SqlxTranslationError::UnsupportedPredicate(
                    "conditional scalar expression is not supported by SQLx translator".to_string(),
                ))
            }
            ScalarExpression::Boolean(_) => Err(SqlxTranslationError::UnsupportedPredicate(
                "boolean scalar expression is not supported by SQLx translator".to_string(),
            )),
            ScalarExpression::Custom { description, .. } => {
                Err(SqlxTranslationError::UnsupportedPredicate(
                    description
                        .clone()
                        .unwrap_or_else(|| "custom scalar expression is not supported".to_string()),
                ))
            }
        }
    }

    fn translate_scalar_function(
        &mut self,
        name: &str,
        arguments: &[ScalarExpression<ExpressionValue>],
    ) -> Result<String, SqlxTranslationError> {
        let lowered = name.to_ascii_lowercase();
        let args = arguments
            .iter()
            .map(|argument| self.translate_scalar_sql(argument))
            .collect::<Result<Vec<_>, _>>()?;
        match lowered.as_str() {
            ScalarFunctionNames::ABS => self.exact_function("ABS", &args, 1),
            ScalarFunctionNames::LOWER => self.exact_function("LOWER", &args, 1),
            ScalarFunctionNames::UPPER => self.exact_function("UPPER", &args, 1),
            ScalarFunctionNames::TRIM => self.exact_function("TRIM", &args, 1),
            ScalarFunctionNames::LENGTH => self.exact_function("LENGTH", &args, 1),
            ScalarFunctionNames::COALESCE => {
                if args.is_empty() {
                    Err(SqlxTranslationError::InvalidExpression(
                        "coalesce expects at least one argument".to_string(),
                    ))
                } else {
                    Ok(format!("COALESCE({})", args.join(", ")))
                }
            }
            _ => Err(SqlxTranslationError::UnsupportedPredicate(format!(
                "unsupported scalar function: {name}"
            ))),
        }
    }

    fn exact_function(
        &self,
        sql_name: &str,
        args: &[String],
        expected: usize,
    ) -> Result<String, SqlxTranslationError> {
        if args.len() != expected {
            return Err(SqlxTranslationError::InvalidExpression(format!(
                "{sql_name} expects {expected} argument(s)"
            )));
        }
        Ok(format!("{sql_name}({})", args.join(", ")))
    }

    fn translate_boolean(
        &mut self,
        expression: &BooleanExpression<ExpressionValue>,
    ) -> Result<SqlxSql, SqlxTranslationError> {
        let sql = self.translate_boolean_sql(expression)?;
        Ok(self.finish(sql))
    }

    fn translate_boolean_typed(
        &mut self,
        expression: &BooleanExpression<ExpressionValue>,
    ) -> Result<SqlxTypedSql, SqlxTranslationError> {
        let sql = self.translate_boolean_sql(expression)?;
        Ok(self.finish_typed(sql))
    }

    fn translate_boolean_sql(
        &mut self,
        expression: &BooleanExpression<ExpressionValue>,
    ) -> Result<String, SqlxTranslationError> {
        match expression {
            BooleanExpression::Constant(value) => match value {
                Trivalent::True => Ok("1 = 1".to_string()),
                Trivalent::False | Trivalent::Unknown => Ok("1 = 0".to_string()),
            },
            BooleanExpression::Comparison {
                operator,
                left,
                right,
            } => self.translate_comparison(*operator, left, right),
            BooleanExpression::In {
                value,
                candidates,
                negated,
            } => self.translate_in(value, candidates, *negated),
            BooleanExpression::PatternMatch {
                value,
                pattern,
                mode,
                negated,
            } => self.translate_pattern_match(value, pattern, *mode, *negated),
            BooleanExpression::NullCheck {
                path,
                null_check_type,
            } => {
                let column = self.translator.resolve_column(path)?;
                let sql = match null_check_type {
                    NullCheckType::IsNull => format!("{column} IS NULL"),
                    NullCheckType::IsNotNull => format!("{column} IS NOT NULL"),
                };
                Ok(sql)
            }
            BooleanExpression::And(operands) => self.translate_logical("AND", operands, true),
            BooleanExpression::Or(operands) => self.translate_logical("OR", operands, false),
            BooleanExpression::Not(operand) => {
                let operand = self.translate_boolean_sql(operand)?;
                Ok(format!("NOT ({operand})"))
            }
            BooleanExpression::Custom { description, .. } => self.unsupported(
                description
                    .as_deref()
                    .unwrap_or("custom boolean expression"),
            ),
        }
    }

    fn translate_comparison(
        &mut self,
        operator: ComparisonOperator,
        left: &ScalarExpression<ExpressionValue>,
        right: &ScalarExpression<ExpressionValue>,
    ) -> Result<String, SqlxTranslationError> {
        if let Some(sql) = self.translate_null_comparison(operator, left, right)? {
            return Ok(sql);
        }
        let left_type = if matches!(left, ScalarExpression::Constant(_)) {
            Self::scalar_reference_path(right)
                .and_then(|path| self.resolve_value_type(&path))
                .or_else(|| self.scalar_expression_type(right))
        } else {
            self.scalar_expression_type(left)
        };
        let right_type = if matches!(right, ScalarExpression::Constant(_)) {
            Self::scalar_reference_path(left)
                .and_then(|path| self.resolve_value_type(&path))
                .or_else(|| self.scalar_expression_type(left))
        } else {
            self.scalar_expression_type(right)
        };
        let left = match left {
            ScalarExpression::Constant(value) => {
                self.push_bound_param(value, left_type.as_ref())?
            }
            _ => self.translate_scalar_sql(left)?,
        };
        let right = match right {
            ScalarExpression::Constant(value) => {
                self.push_bound_param(value, right_type.as_ref())?
            }
            _ => self.translate_scalar_sql(right)?,
        };
        if let (Some(left_type), Some(right_type)) = (&left_type, &right_type) {
            if !left_type.compatible_with(right_type) {
                return Err(SqlxTranslationError::ParameterBinding(format!(
                    "comparison operands have incompatible types: {} and {}",
                    left_type.name(),
                    right_type.name(),
                )));
            }
        }
        Ok(format!("{left} {} {right}", comparison_sql(operator)))
    }

    fn translate_null_comparison(
        &mut self,
        operator: ComparisonOperator,
        left: &ScalarExpression<ExpressionValue>,
        right: &ScalarExpression<ExpressionValue>,
    ) -> Result<Option<String>, SqlxTranslationError> {
        let left_null = matches!(left, ScalarExpression::Constant(ExpressionValue::Null));
        let right_null = matches!(right, ScalarExpression::Constant(ExpressionValue::Null));
        if !left_null && !right_null {
            return Ok(None);
        }
        if left_null && right_null {
            return Ok(Some(match operator {
                ComparisonOperator::Eq | ComparisonOperator::Le | ComparisonOperator::Ge => {
                    "1 = 1".to_string()
                }
                ComparisonOperator::Ne | ComparisonOperator::Lt | ComparisonOperator::Gt => {
                    "1 = 0".to_string()
                }
            }));
        }
        let nullable = if left_null { right } else { left };
        let nullable = self.translate_scalar_sql(nullable)?;
        match operator {
            ComparisonOperator::Eq => Ok(Some(format!("{nullable} IS NULL"))),
            ComparisonOperator::Ne => Ok(Some(format!("{nullable} IS NOT NULL"))),
            _ => self
                .unsupported("ordered comparison with NULL is not supported")
                .map(Some),
        }
    }

    fn translate_in(
        &mut self,
        value: &ScalarExpression<ExpressionValue>,
        candidates: &[ScalarExpression<ExpressionValue>],
        negated: bool,
    ) -> Result<String, SqlxTranslationError> {
        if candidates.is_empty() {
            return self.unsupported("IN candidates must not be empty");
        }
        let path = Self::scalar_reference_path(value).ok_or_else(|| {
            SqlxTranslationError::UnsupportedPredicate(
                "IN value must be a direct column reference".to_string(),
            )
        })?;
        let target_type = self.resolve_value_type(&path);
        let value = self.translate_scalar_sql(value)?;
        let candidates = candidates
            .iter()
            .map(|candidate| {
                let ScalarExpression::Constant(candidate) = candidate else {
                    return Err(SqlxTranslationError::UnsupportedPredicate(
                        "IN candidates must be scalar constants".to_string(),
                    ));
                };
                self.push_bound_param(candidate, target_type.as_ref())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let keyword = if negated { "NOT IN" } else { "IN" };
        Ok(format!("{value} {keyword} ({})", candidates.join(", ")))
    }

    fn translate_pattern_match(
        &mut self,
        value: &ScalarExpression<ExpressionValue>,
        pattern: &ScalarExpression<ExpressionValue>,
        mode: PatternMatchMode,
        negated: bool,
    ) -> Result<String, SqlxTranslationError> {
        if matches!(mode, PatternMatchMode::Regex) {
            return self
                .unsupported("regex pattern match is not supported by generic SQLx translator");
        }
        let value = self.translate_scalar_sql(value)?;
        let pattern = self.translate_pattern(pattern, mode)?;
        let operator = if negated { "NOT LIKE" } else { "LIKE" };
        Ok(format!("{value} {operator} {pattern}"))
    }

    fn translate_pattern(
        &mut self,
        pattern: &ScalarExpression<ExpressionValue>,
        mode: PatternMatchMode,
    ) -> Result<String, SqlxTranslationError> {
        match (mode, pattern) {
            (
                PatternMatchMode::Exact
                | PatternMatchMode::Prefix
                | PatternMatchMode::Suffix
                | PatternMatchMode::Contains
                | PatternMatchMode::Like,
                ScalarExpression::Constant(ExpressionValue::String(pattern)),
            ) => {
                let pattern = match mode {
                    PatternMatchMode::Exact | PatternMatchMode::Like => pattern.clone(),
                    PatternMatchMode::Prefix => format!("{pattern}%"),
                    PatternMatchMode::Suffix => format!("%{pattern}"),
                    PatternMatchMode::Contains => format!("%{pattern}%"),
                    PatternMatchMode::Regex => unreachable!(),
                };
                self.push_bound_param(&ExpressionValue::String(pattern), None)
            }
            (PatternMatchMode::Like, expression) => self.translate_scalar_sql(expression),
            (_, _) => self.unsupported("non-constant pattern requires LIKE mode"),
        }
    }

    fn translate_logical(
        &mut self,
        operator: &str,
        operands: &[BooleanExpression<ExpressionValue>],
        empty_value: bool,
    ) -> Result<String, SqlxTranslationError> {
        if operands.is_empty() {
            return Ok(if empty_value { "1 = 1" } else { "1 = 0" }.to_string());
        }
        let operands = operands
            .iter()
            .map(|operand| {
                self.translate_boolean_sql(operand)
                    .map(|sql| format!("({sql})"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(operands.join(&format!(" {operator} ")))
    }

    fn translate_update_assignments(
        &mut self,
        assignments: &UpdateAssignments<ExpressionValue>,
    ) -> Result<SqlxSql, SqlxTranslationError> {
        let sql = self.translate_update_assignments_sql(assignments)?;
        Ok(self.finish(sql))
    }

    fn translate_update_assignments_sql(
        &mut self,
        assignments: &UpdateAssignments<ExpressionValue>,
    ) -> Result<String, SqlxTranslationError> {
        let parts = assignments
            .items
            .iter()
            .map(|assignment| self.translate_update_assignment(assignment))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(parts.join(", "))
    }

    fn translate_update_assignment(
        &mut self,
        assignment: &UpdateAssignment<ExpressionValue>,
    ) -> Result<String, SqlxTranslationError> {
        match assignment {
            UpdateAssignment::SetValue(SetValue { path, value }) => {
                let column = self.translator.resolve_column(path)?;
                let target = self.resolve_value_type(path);
                let value = self.push_bound_param(value, target.as_ref())?;
                Ok(format!("{column} = {value}"))
            }
            UpdateAssignment::SetNull(SetNull { path }) => {
                let column = self.translator.resolve_column(path)?;
                Ok(format!("{column} = NULL"))
            }
            UpdateAssignment::SetFromExpression(SetFromExpression { path, expression }) => {
                let column = self.translator.resolve_column(path)?;
                let expression = self.translate_scalar_sql(expression)?;
                Ok(format!("{column} = {expression}"))
            }
        }
    }

    fn unsupported(&self, reason: &str) -> Result<String, SqlxTranslationError> {
        match self.translator.config.unsupported_predicate_policy {
            UnsupportedPredicatePolicy::AlwaysFalse => Ok("1 = 0".to_string()),
            UnsupportedPredicatePolicy::FailFast => Err(
                SqlxTranslationError::UnsupportedPredicate(reason.to_string()),
            ),
            UnsupportedPredicatePolicy::ClientFilter => {
                Err(SqlxTranslationError::UnsupportedPredicate(format!(
                    "ClientFilter is not implemented for SQLx translator: {reason}"
                )))
            }
        }
    }
}

fn comparison_sql(operator: ComparisonOperator) -> &'static str {
    match operator {
        ComparisonOperator::Eq => "=",
        ComparisonOperator::Ne => "<>",
        ComparisonOperator::Lt => "<",
        ComparisonOperator::Le => "<=",
        ComparisonOperator::Gt => ">",
        ComparisonOperator::Ge => ">=",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::{
        PredicateSchema, SortBy, UpdateAssignments, lower, path, runtime_field,
    };
    use ospf_rust_math::symbol::ScalarExpressionDsl;

    fn translator() -> SqlxTranslator<PredicateSchema> {
        let schema = PredicateSchema::new("Users")
            .with_field_name("id", "user_id")
            .with_field_name("status", "user_status")
            .with_field_name("age", "age")
            .with_field_name("name", "name")
            .with_field_name("score", "score");
        SqlxTranslator::with_config(
            schema,
            SqlxTranslatorConfig::default().with_dialect(SqlxDialect::Postgres),
        )
    }

    fn statement_builder() -> SqlxRepositoryStatementBuilder<PredicateSchema> {
        let schema = PredicateSchema::new("Users")
            .with_field_name("id", "user_id")
            .with_field_name("status", "user_status")
            .with_field_name("age", "age")
            .with_field_name("name", "name")
            .with_field_name("score", "score");
        SqlxRepositoryStatementBuilder::with_config(
            "users",
            schema,
            SqlxTranslatorConfig::default().with_dialect(SqlxDialect::Postgres),
        )
        .with_select_columns(["user_id", "user_status", "age"])
    }

    #[test]
    fn translates_boolean_expression_to_numbered_sql() {
        let status = runtime_field("status");
        let age = runtime_field("age");
        let expression = status.eq("active") & age.ge(18);

        let sql = translator().translate_boolean(&expression).unwrap();

        assert_eq!(
            sql.sql,
            r#"("user_status" = $1) AND ("age" >= $2)"#.to_string()
        );
        assert_eq!(
            sql.params,
            vec![ExpressionValue::from("active"), ExpressionValue::from(18)]
        );
    }

    #[test]
    fn translates_scalar_functions_and_like_patterns() {
        let expression = lower(path("name").as_scalar()).eq_expr("alice")
            & runtime_field("name").pattern_match("Al", PatternMatchMode::Prefix, false);

        let sql = translator().translate_boolean(&expression).unwrap();

        assert_eq!(
            sql.sql,
            r#"(LOWER("name") = $1) AND ("name" LIKE $2)"#.to_string()
        );
        assert_eq!(
            sql.params,
            vec![ExpressionValue::from("alice"), ExpressionValue::from("Al%")]
        );
    }

    #[test]
    fn translates_null_checks_and_null_comparison() {
        let expression =
            runtime_field("status").eq(ExpressionValue::Null) | runtime_field("name").is_not_null();

        let sql = translator().translate_boolean(&expression).unwrap();

        assert_eq!(
            sql.sql,
            r#"("user_status" IS NULL) OR ("name" IS NOT NULL)"#.to_string()
        );
        assert!(sql.params.is_empty());
    }

    #[test]
    fn translates_sort_with_nulls_fallback() {
        let translator = SqlxTranslator::with_config(
            PredicateSchema::new("Users").with_field_name("name", "name"),
            SqlxTranslatorConfig::default()
                .with_dialect(SqlxDialect::MySql)
                .with_nulls_order_support(NullsOrderSupport::Never),
        );
        let sort = SortBy::asc_nulls("name", NullsOrder::NullsLast);

        let sql = translator.translate_sort_by(&sort).unwrap();

        assert_eq!(
            sql,
            "CASE WHEN `name` IS NULL THEN 1 ELSE 0 END ASC, `name` ASC"
        );
    }

    #[test]
    fn translates_update_assignments() {
        let assignments = UpdateAssignments::set("status", ExpressionValue::from("inactive"))
            .then_set_null("name")
            .then_set_expr(
                "score",
                ScalarExpression::add_expr(
                    ScalarExpression::reference("score"),
                    ScalarExpression::constant(ExpressionValue::from(1)),
                ),
            );

        let sql = translator()
            .translate_update_assignments(&assignments)
            .unwrap();

        assert_eq!(
            sql.sql,
            r#""user_status" = $1, "name" = NULL, "score" = ("score" + $2)"#
        );
        assert_eq!(
            sql.params,
            vec![ExpressionValue::from("inactive"), ExpressionValue::from(1)]
        );
    }

    #[test]
    fn fail_fast_reports_unsupported_predicate() {
        let translator = SqlxTranslator::with_config(
            PredicateSchema::new("Users").with_same_field("name"),
            SqlxTranslatorConfig::default()
                .with_unsupported_predicate_policy(UnsupportedPredicatePolicy::FailFast),
        );
        let expression = runtime_field("name").regex("^A");

        let err = translator.translate_boolean(&expression).unwrap_err();

        assert!(matches!(err, SqlxTranslationError::UnsupportedPredicate(_)));
    }

    #[test]
    fn reports_unsupported_conditional_and_boolean_scalar_expressions() {
        let conditional = ScalarExpression::conditional(
            runtime_field("age").ge(18),
            ScalarExpression::constant(ExpressionValue::from(1)),
            ScalarExpression::constant(ExpressionValue::from(0)),
        );
        let boolean = ScalarExpression::boolean_expr(runtime_field("status").eq("active"));

        let conditional_err = translator().translate_scalar(&conditional).unwrap_err();
        let boolean_err = translator().translate_scalar(&boolean).unwrap_err();

        assert!(matches!(
            conditional_err,
            SqlxTranslationError::UnsupportedPredicate(message)
                if message.contains("conditional scalar expression")
        ));
        assert!(matches!(
            boolean_err,
            SqlxTranslationError::UnsupportedPredicate(message)
                if message.contains("boolean scalar expression")
        ));
    }

    #[test]
    fn builds_select_statement_with_sort_and_pagination() {
        let where_expr = runtime_field("status").eq("active");
        let options = crate::persistence::RepositoryQuery::new()
            .with_sort_by(SortBy::desc("age"))
            .with_limit(10)
            .with_offset(20);

        let statement = statement_builder()
            .select_statement(&where_expr, &options)
            .unwrap();

        assert_eq!(
            statement.sql,
            r#"SELECT "user_id", "user_status", "age" FROM "users" WHERE "user_status" = $1 ORDER BY "age" DESC LIMIT $2 OFFSET $3"#
        );
        assert_eq!(
            statement.params,
            vec![
                ExpressionValue::from("active"),
                ExpressionValue::from(10),
                ExpressionValue::from(20)
            ]
        );
    }

    #[test]
    fn builds_count_and_delete_statements() {
        let where_expr = runtime_field("age").ge(18);
        let builder = statement_builder();

        let count = builder.count_statement(&where_expr).unwrap();
        let delete = builder.delete_statement(&where_expr).unwrap();

        assert_eq!(
            count.sql,
            r#"SELECT COUNT(*) FROM "users" WHERE "age" >= $1"#
        );
        assert_eq!(count.params, vec![ExpressionValue::from(18)]);
        assert_eq!(delete.sql, r#"DELETE FROM "users" WHERE "age" >= $1"#);
        assert_eq!(delete.params, vec![ExpressionValue::from(18)]);
    }

    #[test]
    fn builds_update_statement_with_continuous_postgres_placeholders() {
        let where_expr = runtime_field("id").eq(7);
        let assignments = UpdateAssignments::set("status", ExpressionValue::from("inactive"))
            .then_set_expr(
                "score",
                ScalarExpression::add_expr(
                    ScalarExpression::reference("score"),
                    ScalarExpression::constant(ExpressionValue::from(1)),
                ),
            );

        let statement = statement_builder()
            .update_statement(&where_expr, &assignments)
            .unwrap();

        assert_eq!(
            statement.sql,
            r#"UPDATE "users" SET "user_status" = $1, "score" = ("score" + $2) WHERE "user_id" = $3"#
        );
        assert_eq!(
            statement.params,
            vec![
                ExpressionValue::from("inactive"),
                ExpressionValue::from(1),
                ExpressionValue::from(7)
            ]
        );
    }

    #[test]
    fn rejects_empty_update_assignments() {
        let err = statement_builder()
            .update_statement(
                &runtime_field("id").eq(1),
                &UpdateAssignments::<ExpressionValue>::empty(),
            )
            .unwrap_err();

        assert!(matches!(err, SqlxTranslationError::InvalidExpression(_)));
    }

    #[test]
    fn typed_translation_records_target_column_type() {
        let translator = translator()
            .with_type_resolver(|path| (path.value() == "age").then_some(SqlxValueType::Number));
        let expression = BooleanExpression::eq(
            ScalarExpression::reference("age"),
            ScalarExpression::constant(ExpressionValue::from(18)),
        );

        let statement = translator.translate_boolean_typed(&expression).unwrap();

        assert_eq!(statement.params, vec![ExpressionValue::from(18)]);
        assert_eq!(statement.parameter_types, vec![SqlxValueType::Number]);
    }

    #[test]
    fn value_types_accept_matching_values_and_reject_mismatches() {
        assert!(SqlxValueType::Number.accepts(&ExpressionValue::from(1)));
        assert!(!SqlxValueType::Number.accepts(&ExpressionValue::from("1")));
        assert!(SqlxValueType::Any.accepts(&ExpressionValue::from("1")));
    }

    #[test]
    fn typed_translation_rejects_incompatible_target_constant() {
        let translator = translator()
            .with_type_resolver(|path| (path.value() == "age").then_some(SqlxValueType::Number));
        let expression = BooleanExpression::eq(
            ScalarExpression::reference("age"),
            ScalarExpression::constant(ExpressionValue::from("18")),
        );

        let error = translator.translate_boolean_typed(&expression).unwrap_err();

        assert!(matches!(error, SqlxTranslationError::ParameterBinding(_)));
    }

    #[test]
    fn target_constant_binder_can_convert_a_value() {
        let translator = translator()
            .with_type_resolver(|path| (path.value() == "age").then_some(SqlxValueType::Number))
            .with_target_constant_binder(|value: &ExpressionValue, target: &SqlxValueType| {
                if matches!(target, SqlxValueType::Number)
                    && matches!(value, ExpressionValue::String(value) if value == "18")
                {
                    Ok(Some(ExpressionValue::from(18)))
                } else {
                    Ok(None)
                }
            });
        let expression = BooleanExpression::eq(
            ScalarExpression::reference("age"),
            ScalarExpression::constant(ExpressionValue::from("18")),
        );

        let statement = translator.translate_boolean_typed(&expression).unwrap();

        assert_eq!(statement.params, vec![ExpressionValue::from(18)]);
        assert_eq!(statement.parameter_types, vec![SqlxValueType::Number]);
    }

    #[test]
    fn target_constant_binder_none_uses_default_compatible_binding() {
        let translator = translator()
            .with_type_resolver(|path| (path.value() == "age").then_some(SqlxValueType::Number))
            .with_target_constant_binder(|_: &ExpressionValue, _: &SqlxValueType| Ok(None));
        let expression = BooleanExpression::eq(
            ScalarExpression::reference("age"),
            ScalarExpression::constant(ExpressionValue::from(18)),
        );

        let statement = translator.translate_boolean_typed(&expression).unwrap();

        assert_eq!(statement.params, vec![ExpressionValue::from(18)]);
        assert_eq!(statement.parameter_types, vec![SqlxValueType::Number]);
    }

    #[test]
    fn target_constant_binder_failure_is_structured() {
        let translator = translator()
            .with_type_resolver(|path| (path.value() == "age").then_some(SqlxValueType::Number))
            .with_target_constant_binder(|_: &ExpressionValue, _: &SqlxValueType| {
                Err("binder rejected value".to_string())
            });
        let expression = BooleanExpression::eq(
            ScalarExpression::reference("age"),
            ScalarExpression::constant(ExpressionValue::from(18)),
        );

        let error = translator.translate_boolean_typed(&expression).unwrap_err();

        assert!(matches!(
            error,
            SqlxTranslationError::ParameterBinding(message)
                if message.contains("binder rejected value")
        ));
    }

    #[test]
    fn typed_in_translation_requires_candidates_compatible_with_the_column() {
        let translator = translator()
            .with_type_resolver(|path| (path.value() == "age").then_some(SqlxValueType::Number));
        let expression = BooleanExpression::in_expr(
            ScalarExpression::reference("age"),
            vec![ScalarExpression::constant(ExpressionValue::from("18"))],
            false,
        );

        let error = translator.translate_boolean_typed(&expression).unwrap_err();

        assert!(matches!(error, SqlxTranslationError::ParameterBinding(_)));
    }
}
