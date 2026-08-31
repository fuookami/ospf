//! SQLx 关系查询计划编译器
//! SQLx relational query plan compiler

use super::{
    SqlxDialect, SqlxSql, SqlxTargetConstantBinder, SqlxTranslationError, SqlxTranslator,
    SqlxTranslatorConfig, SqlxTypedSql, SqlxValueType,
};
use crate::persistence::{
    ColumnRef, DiagnosticPersistenceFieldResolver, JoinType, PersistenceFieldResolution,
    PersistenceFieldResolver, QueryAuditSummary, QueryExecutionErrorCategory, QueryExecutionResult,
    QueryExecutionStats, QuerySource, RelationalQueryFailure, RelationalQueryPlan,
};
use ospf_rust_math::symbol::{BooleanExpression, ExpressionValue, PropertyPath};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::time::Instant;

type DetailedResolver = dyn Fn(&PropertyPath) -> PersistenceFieldResolution<String>;

/// SQLx 关系查询数据源注册。
/// SQLx relational query source registration.
///
/// 表名和字段映射必须由适配器显式注册；查询计划不能直接提供物理标识符。
/// Table names and field mappings must be explicitly registered by the adapter; query plans cannot provide physical identifiers.
pub struct SqlxQuerySource {
    /// 通用数据源描述 / Generic source descriptor
    pub source: QuerySource,
    /// 已注册的物理表名 / Registered physical table name
    pub table_name: String,
    resolver: Arc<DetailedResolver>,
    default_columns: Vec<String>,
    field_types: HashMap<String, SqlxValueType>,
}

impl Debug for SqlxQuerySource {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SqlxQuerySource")
            .field("source", &self.source)
            .field("table_name", &self.table_name)
            .field("default_columns", &self.default_columns)
            .field("field_types", &self.field_types)
            .finish()
    }
}

impl Clone for SqlxQuerySource {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            table_name: self.table_name.clone(),
            resolver: Arc::clone(&self.resolver),
            default_columns: self.default_columns.clone(),
            field_types: self.field_types.clone(),
        }
    }
}

impl SqlxQuerySource {
    /// 创建使用兼容可空解析器的数据源注册。
    /// Create a source registration from a compatible nullable resolver.
    pub fn new<R>(source: QuerySource, table_name: impl Into<String>, resolver: R) -> Self
    where
        R: PersistenceFieldResolver<String> + 'static,
    {
        Self {
            source,
            table_name: table_name.into(),
            resolver: Arc::new(move |path| {
                resolver.resolve_field(path).map_or_else(
                    || PersistenceFieldResolution::Missing {
                        path: path.value().to_string(),
                    },
                    PersistenceFieldResolution::Resolved,
                )
            }),
            default_columns: Vec::new(),
            field_types: HashMap::new(),
        }
    }

    /// 创建使用详细字段解析器的数据源注册。
    /// Create a source registration from a diagnostic resolver.
    pub fn with_diagnostic_resolver<R>(
        source: QuerySource,
        table_name: impl Into<String>,
        resolver: R,
    ) -> Self
    where
        R: DiagnosticPersistenceFieldResolver<String> + 'static,
    {
        Self {
            source,
            table_name: table_name.into(),
            resolver: Arc::new(move |path| resolver.resolve_detailed(path)),
            default_columns: Vec::new(),
            field_types: HashMap::new(),
        }
    }

    /// 设置无显式投影时使用的注册字段白名单。
    /// Set the registered field allowlist used without explicit projections.
    pub fn with_default_columns<I, S>(mut self, columns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.default_columns = columns
            .into_iter()
            .map(Into::into)
            .map(|column| column.trim().to_string())
            .collect();
        self
    }

    /// 获取默认投影字段。
    /// Get default projection fields.
    pub fn default_columns(&self) -> &[String] {
        &self.default_columns
    }

    /// 注册字段逻辑类型。
    /// Register a logical field type.
    pub fn with_field_type(mut self, path: impl Into<String>, value_type: SqlxValueType) -> Self {
        self.field_types
            .insert(path.into().trim().to_string(), value_type);
        self
    }

    /// 注册多个字段逻辑类型。
    /// Register multiple logical field types.
    pub fn with_field_types<I, S>(mut self, field_types: I) -> Self
    where
        I: IntoIterator<Item = (S, SqlxValueType)>,
        S: Into<String>,
    {
        self.field_types.extend(
            field_types
                .into_iter()
                .map(|(path, value_type)| (path.into().trim().to_string(), value_type)),
        );
        self
    }

    fn resolve_type(&self, path: &PropertyPath) -> Option<SqlxValueType> {
        self.field_types.get(path.value()).cloned()
    }

    fn resolve_detailed(&self, path: &PropertyPath) -> PersistenceFieldResolution<String> {
        (self.resolver)(path)
    }
}

/// SQLx 关系查询编译结果。
/// Compiled SQLx relational query.
#[derive(Debug, Clone, PartialEq)]
pub struct SqlxCompiledQuery {
    /// 参数化 SQL 语句 / Parameterized SQL statement
    pub statement: SqlxSql,
    /// 编译时使用的查询计划 / Query plan used for compilation
    pub plan: RelationalQueryPlan,
    /// 根数据源的默认投影字段 / Root source default projection fields
    pub root_columns: Vec<String>,
    /// 可审计查询摘要 / Auditable query summary
    pub audit: QueryAuditSummary,
}

/// SQLx 查询执行失败。
/// SQLx query execution failure.
#[derive(Debug)]
pub enum SqlxQueryExecutionError<E> {
    /// 执行请求参数非法 / Invalid execution request parameter
    InvalidRequest(RelationalQueryFailure),
    /// 外部执行器失败 / External executor failure
    Executor(E),
}

/// SQLx 执行器错误分类器。
/// SQLx executor error classifier.
///
/// SQLx 连接、运行时和数据库错误由具体适配器拥有，因此通用查询层不根据错误文本猜测分类。
/// Connection, runtime, and database errors belong to the concrete adapter, so the generic query layer does not guess categories from error text.
pub trait SqlxExecutionErrorClassifier<E> {
    /// 将执行器错误转换为结构化查询失败。
    /// Convert an executor error into a structured query failure.
    fn classify(&self, error: E) -> RelationalQueryFailure;
}

impl<E, F> SqlxExecutionErrorClassifier<E> for F
where
    F: Fn(E) -> RelationalQueryFailure,
{
    fn classify(&self, error: E) -> RelationalQueryFailure {
        self(error)
    }
}

impl SqlxCompiledQuery {
    /// 使用外部执行器执行并收集结果统计。
    /// Execute through an external executor and collect result statistics.
    ///
    /// SQLx 本身负责连接和行映射；该方法只约束执行边界，不持有数据库连接。
    /// SQLx owns connections and row mapping; this method only defines the execution boundary and does not own a database connection.
    pub fn execute_with<T, E, F>(
        &self,
        max_returned_rows: Option<usize>,
        executor: F,
    ) -> Result<QueryExecutionResult<Vec<T>>, SqlxQueryExecutionError<E>>
    where
        F: FnOnce(&SqlxSql) -> Result<Vec<T>, E>,
    {
        if max_returned_rows == Some(0) {
            return Err(SqlxQueryExecutionError::InvalidRequest(
                RelationalQueryFailure {
                    category: QueryExecutionErrorCategory::SqlGeneration,
                    field: Some("maxReturnedRows".to_string()),
                    reason: "maxReturnedRows must be positive".to_string(),
                },
            ));
        }
        let started = Instant::now();
        let mut rows = executor(&self.statement).map_err(SqlxQueryExecutionError::Executor)?;
        let truncated = max_returned_rows.is_some_and(|limit| rows.len() > limit);
        if let Some(limit) = max_returned_rows {
            rows.truncate(limit);
        }
        Ok(QueryExecutionResult {
            stats: QueryExecutionStats {
                duration_ms: started.elapsed().as_millis(),
                returned_rows: rows.len() as u64,
                scanned_rows: None,
                scanned_rows_exact: false,
                truncated,
            },
            value: rows,
        })
    }

    /// 使用显式错误分类器执行并收集结果统计。
    /// Execute with an explicit error classifier and collect result statistics.
    ///
    /// 调用方负责把具体 SQLx/数据库错误映射为 `Timeout`、`UnsupportedDialect` 或 `Database` 等分类；本方法不会从错误字符串推断语义。
    /// The caller maps concrete SQLx/database errors to categories such as `Timeout`, `UnsupportedDialect`, or `Database`; this method never infers semantics from error strings.
    pub fn execute_with_classifier<T, E, F, C>(
        &self,
        max_returned_rows: Option<usize>,
        executor: F,
        classifier: C,
    ) -> Result<QueryExecutionResult<Vec<T>>, RelationalQueryFailure>
    where
        F: FnOnce(&SqlxSql) -> Result<Vec<T>, E>,
        C: SqlxExecutionErrorClassifier<E>,
    {
        match self.execute_with(max_returned_rows, executor) {
            Ok(result) => Ok(result),
            Err(SqlxQueryExecutionError::InvalidRequest(failure)) => Err(failure),
            Err(SqlxQueryExecutionError::Executor(error)) => Err(classifier.classify(error)),
        }
    }
}

#[derive(Debug, Clone)]
struct BoundSource {
    definition: SqlxQuerySource,
    source: QuerySource,
}

#[derive(Debug, Default)]
struct SqlFragment {
    sql: String,
    params: Vec<ExpressionValue>,
    parameter_types: Vec<SqlxValueType>,
}

impl SqlFragment {
    fn append(&mut self, fragment: SqlFragment, dialect: SqlxDialect) {
        let offset = self.params.len();
        self.sql
            .push_str(&shift_placeholders(&fragment.sql, dialect, offset));
        self.params.extend(fragment.params);
        self.parameter_types.extend(fragment.parameter_types);
    }
}

/// SQLx 关系查询计划编译器。
/// SQLx relational query plan compiler.
#[derive(Clone)]
pub struct SqlxRelationalQueryCompiler {
    sources: HashMap<String, SqlxQuerySource>,
    config: SqlxTranslatorConfig,
    target_constant_binder: Option<Arc<dyn SqlxTargetConstantBinder>>,
}

impl Debug for SqlxRelationalQueryCompiler {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SqlxRelationalQueryCompiler")
            .field("sources", &self.sources)
            .field("config", &self.config)
            .field(
                "has_target_constant_binder",
                &self.target_constant_binder.is_some(),
            )
            .finish()
    }
}

impl SqlxRelationalQueryCompiler {
    /// 使用默认 SQLx 配置创建编译器。
    /// Create a compiler with the default SQLx configuration.
    pub fn new<I>(sources: I) -> Self
    where
        I: IntoIterator<Item = (String, SqlxQuerySource)>,
    {
        Self::with_config(
            sources,
            SqlxTranslatorConfig::default().with_unsupported_predicate_policy(
                crate::persistence::UnsupportedPredicatePolicy::FailFast,
            ),
        )
    }

    /// 使用指定 SQLx 配置创建编译器。
    /// Create a compiler with an explicit SQLx configuration.
    pub fn with_config<I>(sources: I, config: SqlxTranslatorConfig) -> Self
    where
        I: IntoIterator<Item = (String, SqlxQuerySource)>,
    {
        Self {
            sources: sources
                .into_iter()
                .map(|(name, source)| (name.trim().to_string(), source))
                .collect(),
            config,
            target_constant_binder: None,
        }
    }

    /// 设置目标列常量绑定器。
    /// Set the target-column constant binder.
    pub fn with_target_constant_binder<B>(mut self, binder: B) -> Self
    where
        B: SqlxTargetConstantBinder + 'static,
    {
        self.target_constant_binder = Some(Arc::new(binder));
        self
    }

    /// 从数据源列表创建编译器。
    /// Create a compiler from a source list.
    pub fn from_sources<I>(sources: I) -> Self
    where
        I: IntoIterator<Item = SqlxQuerySource>,
    {
        Self::new(sources.into_iter().map(|source| {
            let name = source.source.name.clone();
            (name, source)
        }))
    }

    /// 获取编译器配置。
    /// Get compiler configuration.
    pub const fn config(&self) -> SqlxTranslatorConfig {
        self.config
    }

    /// 编译关系查询计划。
    /// Compile a relational query plan.
    pub fn compile(
        &self,
        plan: &RelationalQueryPlan,
    ) -> Result<SqlxCompiledQuery, RelationalQueryFailure> {
        self.validate_plan(plan)?;
        let built = self.build_from(plan, true)?;
        let mut statement = SqlFragment {
            sql: format!(
                "SELECT {}{}",
                if plan.distinct() { "DISTINCT " } else { "" },
                self.select_columns(plan, &built.bound)?,
            ),
            params: built.params,
            parameter_types: built.parameter_types,
        };
        statement.sql.push(' ');
        statement.sql.push_str(&built.from_sql);

        let mut predicates = Vec::new();
        if let Some(predicate) = plan.predicate() {
            let translated = self.translate_boolean(
                predicate,
                &built.bound,
                "predicate",
                QueryExecutionErrorCategory::SqlGeneration,
            )?;
            predicates.push(translated);
        }
        predicates.extend(built.predicates);
        self.append_predicates(&mut statement, predicates);

        if !plan.group_by().is_empty() {
            let columns = plan
                .group_by()
                .iter()
                .enumerate()
                .map(|(index, column)| {
                    self.resolve_column(column, &built.bound, &format!("groupBy[{index}]"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            statement.sql.push_str(" GROUP BY ");
            statement.sql.push_str(&columns.join(", "));
        }
        if !plan.order_by().is_empty() {
            let orders = plan
                .order_by()
                .iter()
                .enumerate()
                .map(|(index, order)| self.resolve_order(order, &built.bound, index))
                .collect::<Result<Vec<_>, _>>()?;
            statement.sql.push_str(" ORDER BY ");
            statement.sql.push_str(&orders.join(", "));
        }
        self.append_page(&mut statement, plan.page());
        Ok(self.compiled(statement, plan, built.root_columns))
    }

    /// 编译按根键去重的计数查询。
    /// Compile a root-key distinct count query.
    pub fn compile_count(
        &self,
        plan: &RelationalQueryPlan,
    ) -> Result<SqlxSql, RelationalQueryFailure> {
        let statement = self.compile_count_typed(plan)?;
        Ok(SqlxSql::new(statement.sql, statement.params))
    }

    /// 编译按根键去重的计数查询并保留参数类型摘要。
    /// Compile a root-key distinct count query while preserving parameter types.
    ///
    /// 旧的 `compile_count` 入口继续返回 `SqlxSql` 以保持兼容；需要审计参数类型或由适配器执行类型感知绑定时使用此入口。
    /// The legacy `compile_count` entry point still returns `SqlxSql` for compatibility; use this entry point for parameter-type auditing or adapter-side typed binding.
    pub fn compile_count_typed(
        &self,
        plan: &RelationalQueryPlan,
    ) -> Result<SqlxTypedSql, RelationalQueryFailure> {
        self.validate_plan(plan)?;
        if plan.root_key().len() != 1 {
            return Err(self.failure(
                QueryExecutionErrorCategory::SqlGeneration,
                Some("rootKey"),
                "Root-granularity count requires exactly one root key column",
            ));
        }
        let built = self.build_from(plan, false)?;
        let root_key = &plan.root_key()[0];
        if root_key.source != plan.root().name
            && plan.root().alias.as_deref() != Some(&root_key.source)
        {
            return Err(self.failure(
                QueryExecutionErrorCategory::SqlGeneration,
                Some("rootKey"),
                "Root key must belong to the root query source",
            ));
        }
        let root_key = self.resolve_column(root_key, &built.bound, "rootKey")?;
        let mut statement = SqlFragment {
            sql: format!("SELECT COUNT(DISTINCT {root_key}) {}", built.from_sql),
            params: built.params,
            parameter_types: built.parameter_types,
        };
        let mut predicates = Vec::new();
        if let Some(predicate) = plan.predicate() {
            predicates.push(self.translate_boolean(
                predicate,
                &built.bound,
                "predicate",
                QueryExecutionErrorCategory::SqlGeneration,
            )?);
        }
        predicates.extend(built.predicates);
        self.append_predicates(&mut statement, predicates);
        Ok(SqlxTypedSql {
            sql: statement.sql,
            params: statement.params,
            parameter_types: statement.parameter_types,
        })
    }

    fn compiled(
        &self,
        statement: SqlFragment,
        plan: &RelationalQueryPlan,
        root_columns: Vec<String>,
    ) -> SqlxCompiledQuery {
        let sql = SqlxSql::new(statement.sql, statement.params);
        let parameter_types = if statement.parameter_types.len() == sql.params.len() {
            statement
                .parameter_types
                .iter()
                .map(|value_type| value_type.name().to_string())
                .collect()
        } else {
            sql.params.iter().map(expression_value_type).collect()
        };
        SqlxCompiledQuery {
            statement: sql.clone(),
            plan: plan.clone(),
            root_columns,
            audit: QueryAuditSummary {
                sql_template: sql.sql.clone(),
                parameter_types,
                dialect: format!("{:?}", self.config.dialect),
            },
        }
    }

    fn validate_plan(&self, plan: &RelationalQueryPlan) -> Result<(), RelationalQueryFailure> {
        plan.validate().map_err(|error| {
            let category = if error.field.starts_with("joins") {
                QueryExecutionErrorCategory::InvalidJoin
            } else {
                QueryExecutionErrorCategory::SqlGeneration
            };
            self.failure(category, Some(error.field), error.reason)
        })
    }

    fn build_from(
        &self,
        plan: &RelationalQueryPlan,
        require_root_projection: bool,
    ) -> Result<BuiltFrom, RelationalQueryFailure> {
        let root_definition = self.sources.get(&plan.root().name).ok_or_else(|| {
            self.failure(
                QueryExecutionErrorCategory::UnknownSource,
                Some("root"),
                format!("Unknown root query source: {}", plan.root().name),
            )
        })?;
        let root = BoundSource {
            definition: root_definition.clone(),
            source: plan.root().clone(),
        };
        let root_columns = root
            .definition
            .default_columns()
            .iter()
            .enumerate()
            .map(|(index, path)| {
                self.resolve_source_column(&root, path, &format!("root.default_columns[{index}]"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut bound = vec![root.clone()];
        let mut statement = SqlFragment {
            sql: format!("FROM {}", self.table_sql(&root)),
            params: Vec::new(),
            parameter_types: Vec::new(),
        };
        let mut predicates = Vec::new();

        for (index, join) in plan.joins().iter().enumerate() {
            let field = format!("joins[{index}]");
            let definition = self.sources.get(&join.source.name).ok_or_else(|| {
                self.failure(
                    QueryExecutionErrorCategory::UnknownSource,
                    Some(format!("{field}.source")),
                    format!("Unknown join query source: {}", join.source.name),
                )
            })?;
            let target = BoundSource {
                definition: definition.clone(),
                source: join.source.clone(),
            };
            for (column_index, column) in target.definition.default_columns().iter().enumerate() {
                self.resolve_source_column(
                    &target,
                    column,
                    &format!("{field}.source.default_columns[{column_index}]"),
                )?;
            }
            if join.join_type == JoinType::Exists && target.definition.default_columns().is_empty()
            {
                return Err(self.failure(
                    QueryExecutionErrorCategory::InvalidJoin,
                    Some(format!("{field}.source.default_columns")),
                    "Exists join requires at least one explicitly registered target column",
                ));
            }
            let mut condition_bound = bound.clone();
            condition_bound.push(target.clone());
            let condition = self.translate_boolean(
                &join.condition,
                &condition_bound,
                &format!("{field}.condition"),
                QueryExecutionErrorCategory::InvalidJoin,
            )?;
            match join.join_type {
                JoinType::Inner | JoinType::Left => {
                    statement.sql.push_str(match join.join_type {
                        JoinType::Inner => " INNER JOIN ",
                        JoinType::Left => " LEFT JOIN ",
                        JoinType::Exists => unreachable!(),
                    });
                    statement.sql.push_str(&self.table_sql(&target));
                    statement.sql.push_str(" ON ");
                    statement.append(condition, self.config.dialect);
                    bound.push(target);
                }
                JoinType::Exists => {
                    let mut exists = SqlFragment {
                        sql: format!("EXISTS (SELECT 1 FROM {} WHERE ", self.table_sql(&target)),
                        params: Vec::new(),
                        parameter_types: Vec::new(),
                    };
                    exists.append(condition, self.config.dialect);
                    exists.sql.push(')');
                    predicates.push(exists);
                }
            }
        }

        if require_root_projection && plan.projections().is_empty() && root_columns.is_empty() {
            return Err(self.failure(
                QueryExecutionErrorCategory::SqlGeneration,
                Some("root.default_columns"),
                "A default projection must contain at least one explicitly registered column",
            ));
        }
        Ok(BuiltFrom {
            from_sql: statement.sql,
            bound,
            params: statement.params,
            parameter_types: statement.parameter_types,
            predicates,
            root_columns,
        })
    }

    fn select_columns(
        &self,
        plan: &RelationalQueryPlan,
        bound: &[BoundSource],
    ) -> Result<String, RelationalQueryFailure> {
        if plan.projections().is_empty() {
            let root = bound.first().ok_or_else(|| {
                self.failure(
                    QueryExecutionErrorCategory::SqlGeneration,
                    Some("root"),
                    "Root query source was not bound",
                )
            })?;
            return root
                .definition
                .default_columns()
                .iter()
                .enumerate()
                .map(|(index, path)| {
                    self.resolve_source_column(
                        root,
                        path,
                        &format!("root.default_columns[{index}]"),
                    )
                })
                .collect::<Result<Vec<String>, RelationalQueryFailure>>()
                .map(|columns| columns.join(", "));
        }
        plan.projections()
            .iter()
            .enumerate()
            .map(|(index, projection)| {
                let column = self.resolve_column(
                    &projection.column,
                    bound,
                    &format!("projections[{index}]"),
                )?;
                Ok(match &projection.alias {
                    Some(alias) => format!("{column} AS {}", self.quote(alias)),
                    None => column,
                })
            })
            .collect::<Result<Vec<String>, RelationalQueryFailure>>()
            .map(|columns| columns.join(", "))
    }

    fn resolve_order(
        &self,
        order: &crate::persistence::query::OrderSpec,
        bound: &[BoundSource],
        index: usize,
    ) -> Result<String, RelationalQueryFailure> {
        let column = self.resolve_column(&order.column, bound, &format!("orderBy[{index}]"))?;
        let direction = match order.direction {
            crate::persistence::query::SortDirection::Ascending => "ASC",
            crate::persistence::query::SortDirection::Descending => "DESC",
        };
        let nulls = match order.nulls {
            crate::persistence::query::NullsOrder::Unspecified => {
                return Ok(format!("{column} {direction}"));
            }
            crate::persistence::query::NullsOrder::First => "0 ELSE 1",
            crate::persistence::query::NullsOrder::Last => "1 ELSE 0",
        };
        Ok(format!(
            "CASE WHEN {column} IS NULL THEN {nulls} END ASC, {column} {direction}"
        ))
    }

    fn append_page(&self, statement: &mut SqlFragment, page: Option<crate::persistence::PageSpec>) {
        if let Some(page) = page {
            let limit = placeholder(self.config.dialect, statement.params.len() + 1);
            statement.params.push(ExpressionValue::from(page.limit));
            statement.parameter_types.push(SqlxValueType::Number);
            statement.sql.push_str(" LIMIT ");
            statement.sql.push_str(&limit);
            if page.offset > 0 {
                let offset = placeholder(self.config.dialect, statement.params.len() + 1);
                statement.params.push(ExpressionValue::from(page.offset));
                statement.parameter_types.push(SqlxValueType::Number);
                statement.sql.push_str(" OFFSET ");
                statement.sql.push_str(&offset);
            }
        }
    }

    fn append_predicates(&self, statement: &mut SqlFragment, predicates: Vec<SqlFragment>) {
        if predicates.is_empty() {
            return;
        }
        statement.sql.push_str(" WHERE ");
        for (index, predicate) in predicates.into_iter().enumerate() {
            if index > 0 {
                statement.sql.push_str(" AND ");
            }
            statement.sql.push('(');
            let offset = statement.params.len();
            statement.sql.push_str(&shift_placeholders(
                &predicate.sql,
                self.config.dialect,
                offset,
            ));
            statement.sql.push(')');
            statement.params.extend(predicate.params);
            statement.parameter_types.extend(predicate.parameter_types);
        }
    }

    fn translate_boolean(
        &self,
        expression: &BooleanExpression<ExpressionValue>,
        bound: &[BoundSource],
        field: &str,
        fallback_category: QueryExecutionErrorCategory,
    ) -> Result<SqlFragment, RelationalQueryFailure> {
        for reference in expression.collect_references() {
            self.resolve_field_path(reference.value(), bound, field)?;
        }
        let resolver = |path: &str| self.resolve_field_path(path, bound, field).ok();
        let type_resolver =
            |path: &PropertyPath| self.resolve_field_type(path, bound, field).ok().flatten();
        let translator =
            SqlxTranslator::with_config(resolver, self.config.with_quote_identifiers(false));
        translator
            .translate_boolean_with_context(
                expression,
                &type_resolver,
                self.target_constant_binder
                    .as_deref()
                    .map(|binder| binder as &dyn SqlxTargetConstantBinder),
            )
            .map(|statement| SqlFragment {
                sql: statement.sql,
                params: statement.params,
                parameter_types: statement.parameter_types,
            })
            .map_err(|error| self.translation_failure(error, field, fallback_category))
    }

    fn resolve_column(
        &self,
        column: &ColumnRef,
        bound: &[BoundSource],
        field: &str,
    ) -> Result<String, RelationalQueryFailure> {
        self.resolve_field_path(&format!("{}.{}", column.source, column.path), bound, field)
    }

    fn resolve_source_column(
        &self,
        source: &BoundSource,
        path: &str,
        field: &str,
    ) -> Result<String, RelationalQueryFailure> {
        let path = PropertyPath::parse(path);
        let column = match source.definition.resolve_detailed(&path) {
            PersistenceFieldResolution::Resolved(column) => column,
            PersistenceFieldResolution::Missing { path } => {
                return Err(self.failure(
                    QueryExecutionErrorCategory::UnknownColumn,
                    Some(field),
                    format!("Unknown registered default column: {path}"),
                ));
            }
            PersistenceFieldResolution::Ambiguous { path, candidates } => {
                return Err(self.failure(
                    QueryExecutionErrorCategory::UnknownColumn,
                    Some(field),
                    format!(
                        "Ambiguous registered default column {path}: {}",
                        candidates.join(", ")
                    ),
                ));
            }
            PersistenceFieldResolution::InvalidConfiguration { reason } => {
                return Err(self.failure(
                    QueryExecutionErrorCategory::SqlGeneration,
                    Some(field),
                    reason,
                ));
            }
        };
        let qualifier = source
            .source
            .alias
            .as_deref()
            .unwrap_or(&source.definition.table_name);
        let physical = qualify_column(qualifier, &column);
        Ok(self.quote(&physical))
    }

    fn resolve_field_path(
        &self,
        path: &str,
        bound: &[BoundSource],
        field: &str,
    ) -> Result<String, RelationalQueryFailure> {
        let normalized = path.trim();
        let (source_qualifier, field_path) = match normalized.split_once('.') {
            Some((source, column)) if !source.is_empty() && !column.is_empty() => {
                (Some(source), column)
            }
            _ => (None, normalized),
        };
        let candidates = bound
            .iter()
            .filter(|source| {
                source_qualifier.is_none_or(|qualifier| {
                    qualifier == source.source.name
                        || source.source.alias.as_deref() == Some(qualifier)
                })
            })
            .collect::<Vec<_>>();
        if source_qualifier.is_some() && candidates.is_empty() {
            return Err(self.failure(
                QueryExecutionErrorCategory::UnknownColumn,
                Some(field),
                format!("Unknown query source qualifier: {normalized}"),
            ));
        }
        let mut resolved = Vec::new();
        let mut ambiguous = Vec::new();
        for source in candidates {
            match source
                .definition
                .resolve_detailed(&PropertyPath::parse(field_path))
            {
                PersistenceFieldResolution::Resolved(column) => resolved.push((source, column)),
                PersistenceFieldResolution::Missing { .. } => {}
                PersistenceFieldResolution::Ambiguous { candidates, .. } => {
                    ambiguous.extend(candidates.into_iter().map(|candidate| {
                        if candidate.contains('.') {
                            candidate
                        } else {
                            format!("{}.{}", source.source.name, candidate)
                        }
                    }));
                }
                PersistenceFieldResolution::InvalidConfiguration { reason } => {
                    return Err(self.failure(
                        QueryExecutionErrorCategory::SqlGeneration,
                        Some(field),
                        reason,
                    ));
                }
            }
        }
        if !ambiguous.is_empty() {
            return Err(self.failure(
                QueryExecutionErrorCategory::UnknownColumn,
                Some(field),
                format!(
                    "Ambiguous query column {normalized}: {}",
                    ambiguous.join(", ")
                ),
            ));
        }
        if resolved.is_empty() {
            return Err(self.failure(
                QueryExecutionErrorCategory::UnknownColumn,
                Some(field),
                format!("Unknown query column: {normalized}"),
            ));
        }
        if resolved.len() > 1 {
            return Err(self.failure(
                QueryExecutionErrorCategory::UnknownColumn,
                Some(field),
                format!("Ambiguous query column: {normalized}"),
            ));
        }
        let Some((source, column)) = resolved.pop() else {
            return Err(self.failure(
                QueryExecutionErrorCategory::UnknownColumn,
                Some(field),
                format!("Unknown query column: {normalized}"),
            ));
        };
        let qualifier = source
            .source
            .alias
            .as_deref()
            .unwrap_or(&source.definition.table_name);
        let physical = qualify_column(qualifier, &column);
        Ok(self.quote(&physical))
    }

    fn resolve_field_type(
        &self,
        path: &PropertyPath,
        bound: &[BoundSource],
        field: &str,
    ) -> Result<Option<SqlxValueType>, RelationalQueryFailure> {
        let normalized = path.value().trim();
        let (source_qualifier, field_path) = match normalized.split_once('.') {
            Some((source, column)) if !source.is_empty() && !column.is_empty() => {
                (Some(source), column)
            }
            _ => (None, normalized),
        };
        let candidates = bound
            .iter()
            .filter(|source| {
                source_qualifier.is_none_or(|qualifier| {
                    qualifier == source.source.name
                        || source.source.alias.as_deref() == Some(qualifier)
                })
            })
            .collect::<Vec<_>>();
        if source_qualifier.is_some() && candidates.is_empty() {
            return Err(self.failure(
                QueryExecutionErrorCategory::UnknownColumn,
                Some(field),
                format!("Unknown query source qualifier: {normalized}"),
            ));
        }
        let logical_path = PropertyPath::parse(field_path);
        let mut resolved = Vec::new();
        for source in candidates {
            match source.definition.resolve_detailed(&logical_path) {
                PersistenceFieldResolution::Resolved(_) => resolved.push(source),
                PersistenceFieldResolution::Missing { .. } => {}
                PersistenceFieldResolution::Ambiguous { path, candidates } => {
                    return Err(self.failure(
                        QueryExecutionErrorCategory::UnknownColumn,
                        Some(field),
                        format!("Ambiguous query column {path}: {}", candidates.join(", ")),
                    ));
                }
                PersistenceFieldResolution::InvalidConfiguration { reason } => {
                    return Err(self.failure(
                        QueryExecutionErrorCategory::SqlGeneration,
                        Some(field),
                        reason,
                    ));
                }
            }
        }
        if resolved.len() > 1 {
            return Err(self.failure(
                QueryExecutionErrorCategory::UnknownColumn,
                Some(field),
                format!("Ambiguous query column: {normalized}"),
            ));
        }
        Ok(resolved
            .first()
            .and_then(|source| source.definition.resolve_type(&logical_path)))
    }

    fn table_sql(&self, source: &BoundSource) -> String {
        let table = self.quote(&source.definition.table_name);
        match &source.source.alias {
            Some(alias) => format!("{table} AS {}", self.quote(alias)),
            None => table,
        }
    }

    fn quote(&self, identifier: &str) -> String {
        if self.config.quote_identifiers {
            self.config.dialect.quote_identifier(identifier)
        } else {
            identifier.to_string()
        }
    }

    fn translation_failure(
        &self,
        error: SqlxTranslationError,
        field: &str,
        fallback_category: QueryExecutionErrorCategory,
    ) -> RelationalQueryFailure {
        let category = match error {
            SqlxTranslationError::UnresolvedField(_) => QueryExecutionErrorCategory::UnknownColumn,
            SqlxTranslationError::ParameterBinding(_) => {
                QueryExecutionErrorCategory::ParameterBinding
            }
            SqlxTranslationError::UnsupportedPredicate(_) => fallback_category,
            SqlxTranslationError::InvalidExpression(_) => fallback_category,
        };
        self.failure(category, Some(field), error.to_string())
    }

    fn failure(
        &self,
        category: QueryExecutionErrorCategory,
        field: Option<impl Into<String>>,
        reason: impl Into<String>,
    ) -> RelationalQueryFailure {
        RelationalQueryFailure {
            category,
            field: field.map(Into::into),
            reason: reason.into(),
        }
    }
}

#[derive(Debug)]
struct BuiltFrom {
    from_sql: String,
    bound: Vec<BoundSource>,
    params: Vec<ExpressionValue>,
    parameter_types: Vec<SqlxValueType>,
    predicates: Vec<SqlFragment>,
    root_columns: Vec<String>,
}

fn expression_value_type(value: &ExpressionValue) -> String {
    match value {
        ExpressionValue::Null => "null",
        ExpressionValue::Boolean(_) => "boolean",
        ExpressionValue::Number(_) => "number",
        ExpressionValue::String(_) => "string",
    }
    .to_string()
}

fn placeholder(dialect: SqlxDialect, index: usize) -> String {
    dialect.placeholder(index)
}

fn qualify_column(qualifier: &str, column: &str) -> String {
    let column = column.rsplit_once('.').map_or(column, |(_, column)| column);
    format!("{qualifier}.{column}")
}

fn shift_placeholders(sql: &str, dialect: SqlxDialect, offset: usize) -> String {
    if offset == 0 || dialect != SqlxDialect::Postgres {
        return sql.to_string();
    }
    let mut shifted = String::with_capacity(sql.len());
    let chars = sql.chars().collect::<Vec<_>>();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '$' {
            let start = index + 1;
            let mut end = start;
            while end < chars.len() && chars[end].is_ascii_digit() {
                end += 1;
            }
            if end > start {
                let number = chars[start..end]
                    .iter()
                    .collect::<String>()
                    .parse::<usize>()
                    .unwrap_or(0);
                shifted.push('$');
                shifted.push_str(&(number + offset).to_string());
                index = end;
                continue;
            }
        }
        shifted.push(chars[index]);
        index += 1;
    }
    shifted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::{
        JoinCardinality, JoinSpec, PredicateSchema, ProjectionSpec, QuerySource,
    };
    use ospf_rust_math::symbol::ScalarExpression;

    fn sources() -> HashMap<String, SqlxQuerySource> {
        let orders = PredicateSchema::new("orders")
            .with_field_name("id", "id")
            .with_field_name("status", "status");
        let items = PredicateSchema::new("items")
            .with_field_name("id", "id")
            .with_field_name("order_id", "order_id")
            .with_field_name("material", "material");
        HashMap::from([
            (
                "orders".to_string(),
                SqlxQuerySource::new(QuerySource::new("orders"), "orders", orders)
                    .with_default_columns(["id", "status"])
                    .with_field_types([
                        ("id", SqlxValueType::Number),
                        ("status", SqlxValueType::String),
                    ]),
            ),
            (
                "items".to_string(),
                SqlxQuerySource::new(QuerySource::new("items"), "order_items", items)
                    .with_default_columns(["order_id", "material"])
                    .with_field_types([
                        ("order_id", SqlxValueType::Number),
                        ("material", SqlxValueType::String),
                    ]),
            ),
        ])
    }

    fn join_condition() -> BooleanExpression<ExpressionValue> {
        BooleanExpression::eq(
            ScalarExpression::reference("o.id"),
            ScalarExpression::reference("i.order_id"),
        )
    }

    #[test]
    fn compiles_join_predicate_and_pagination() {
        let compiler = SqlxRelationalQueryCompiler::with_config(
            sources(),
            SqlxTranslatorConfig::default().with_dialect(SqlxDialect::Postgres),
        );
        let plan = RelationalQueryPlan::new(QuerySource::new("orders").with_alias("o"))
            .with_joins([JoinSpec::new(
                JoinType::Inner,
                QuerySource::new("items").with_alias("i"),
                join_condition(),
                JoinCardinality::OneToMany,
            )])
            .with_predicate(BooleanExpression::eq(
                ScalarExpression::reference("o.status"),
                ScalarExpression::constant(ExpressionValue::from("confirmed")),
            ))
            .with_projections([ProjectionSpec::new(ColumnRef::new("o", "id"))])
            .with_distinct(true)
            .with_page(crate::persistence::PageSpec::new(10, 5));
        let compiled = compiler.compile(&plan).unwrap();

        assert!(compiled.statement.sql.contains("INNER JOIN"));
        assert!(compiled.statement.sql.contains("LIMIT $"));
        assert!(compiled.statement.sql.contains("OFFSET $"));
        assert_eq!(compiled.statement.params.len(), 3);
        assert_eq!(
            compiled.audit.parameter_types,
            vec!["string", "number", "number"]
        );
    }

    #[test]
    fn exists_join_and_root_count_preserve_root_granularity() {
        let compiler = SqlxRelationalQueryCompiler::new(sources());
        let plan = RelationalQueryPlan::new(QuerySource::new("orders").with_alias("o"))
            .with_joins([JoinSpec::new(
                JoinType::Exists,
                QuerySource::new("items").with_alias("i"),
                join_condition(),
                JoinCardinality::OneToMany,
            )])
            .with_root_key([ColumnRef::new("o", "id")]);
        let statement = compiler.compile_count(&plan).unwrap();

        assert!(statement.sql.contains("COUNT(DISTINCT"));
        assert!(statement.sql.contains("EXISTS"));
    }

    #[test]
    fn typed_root_count_preserves_parameter_types_and_order() {
        let compiler = SqlxRelationalQueryCompiler::with_config(
            sources(),
            SqlxTranslatorConfig::default().with_dialect(SqlxDialect::Postgres),
        );
        let plan = RelationalQueryPlan::new(QuerySource::new("orders").with_alias("o"))
            .with_joins([JoinSpec::new(
                JoinType::Exists,
                QuerySource::new("items").with_alias("i"),
                BooleanExpression::and(vec![
                    join_condition(),
                    BooleanExpression::eq(
                        ScalarExpression::reference("i.material"),
                        ScalarExpression::constant(ExpressionValue::from("M-001")),
                    ),
                ]),
                JoinCardinality::OneToMany,
            )])
            .with_predicate(BooleanExpression::eq(
                ScalarExpression::reference("o.status"),
                ScalarExpression::constant(ExpressionValue::from("confirmed")),
            ))
            .with_root_key([ColumnRef::new("o", "id")]);

        let statement = compiler.compile_count_typed(&plan).unwrap();

        assert_eq!(
            statement.params,
            vec![
                ExpressionValue::from("confirmed"),
                ExpressionValue::from("M-001")
            ]
        );
        assert_eq!(
            statement.parameter_types,
            vec![SqlxValueType::String, SqlxValueType::String]
        );
        assert!(statement.sql.contains("COUNT(DISTINCT"));
        assert!(statement.sql.contains("\"o\".\"status\" = $1"));
        assert!(statement.sql.contains("\"i\".\"material\" = $2"));
    }

    #[test]
    fn exists_predicates_follow_root_predicate_parameters() {
        let compiler = SqlxRelationalQueryCompiler::with_config(
            sources(),
            SqlxTranslatorConfig::default().with_dialect(SqlxDialect::Postgres),
        );
        let plan = RelationalQueryPlan::new(QuerySource::new("orders").with_alias("o"))
            .with_joins([JoinSpec::new(
                JoinType::Exists,
                QuerySource::new("items").with_alias("i"),
                BooleanExpression::and(vec![
                    join_condition(),
                    BooleanExpression::eq(
                        ScalarExpression::reference("i.material"),
                        ScalarExpression::constant(ExpressionValue::from("M-001")),
                    ),
                ]),
                JoinCardinality::OneToMany,
            )])
            .with_predicate(BooleanExpression::eq(
                ScalarExpression::reference("o.status"),
                ScalarExpression::constant(ExpressionValue::from("confirmed")),
            ))
            .with_projections([ProjectionSpec::new(ColumnRef::new("o", "id"))]);

        let compiled = compiler.compile(&plan).unwrap();

        assert_eq!(
            compiled.statement.params,
            vec![
                ExpressionValue::from("confirmed"),
                ExpressionValue::from("M-001")
            ]
        );
        assert!(compiled.statement.sql.contains("\"o\".\"status\" = $1"));
        assert!(compiled.statement.sql.contains("\"i\".\"material\" = $2"));
    }

    #[test]
    fn left_join_null_order_uses_null_rank_fallback() {
        let compiler = SqlxRelationalQueryCompiler::new(sources());
        let plan = RelationalQueryPlan::new(QuerySource::new("orders").with_alias("o"))
            .with_joins([JoinSpec::new(
                JoinType::Left,
                QuerySource::new("items").with_alias("i"),
                join_condition(),
                JoinCardinality::OneToMany,
            )])
            .with_projections([ProjectionSpec::new(ColumnRef::new("o", "id"))])
            .with_order_by([
                crate::persistence::OrderSpec::new(ColumnRef::new("i", "material"))
                    .with_nulls(crate::persistence::QueryNullsOrder::Last),
            ]);

        let compiled = compiler.compile(&plan).unwrap();

        assert!(compiled.statement.sql.contains(
            "CASE WHEN \"i\".\"material\" IS NULL THEN 1 ELSE 0 END ASC, \"i\".\"material\" ASC"
        ));
    }

    #[test]
    fn rejects_ambiguous_unqualified_columns_and_unknown_sources() {
        let compiler = SqlxRelationalQueryCompiler::new(sources());
        let ambiguous = RelationalQueryPlan::new(QuerySource::new("orders").with_alias("o"))
            .with_joins([JoinSpec::new(
                JoinType::Inner,
                QuerySource::new("items").with_alias("i"),
                join_condition(),
                JoinCardinality::OneToMany,
            )])
            .with_predicate(BooleanExpression::eq(
                ScalarExpression::reference("id"),
                ScalarExpression::constant(ExpressionValue::from(1)),
            ));
        let failure = compiler.compile(&ambiguous).unwrap_err();
        assert_eq!(failure.category, QueryExecutionErrorCategory::UnknownColumn);

        let missing = compiler.compile(&RelationalQueryPlan::new(QuerySource::new("missing")));
        assert_eq!(
            missing.unwrap_err().category,
            QueryExecutionErrorCategory::UnknownSource
        );
    }

    #[test]
    fn preserves_diagnostic_resolver_ambiguity() {
        let orders = PredicateSchema::new("orders")
            .with_field_name("id", "id")
            .with_field_name("id", "legacy_id");
        let compiler = SqlxRelationalQueryCompiler::new(HashMap::from([(
            "orders".to_string(),
            SqlxQuerySource::with_diagnostic_resolver(QuerySource::new("orders"), "orders", orders),
        )]));
        let plan = RelationalQueryPlan::new(QuerySource::new("orders").with_alias("o"))
            .with_projections([ProjectionSpec::new(ColumnRef::new("o", "id"))]);

        let failure = compiler.compile(&plan).unwrap_err();

        assert_eq!(failure.category, QueryExecutionErrorCategory::UnknownColumn);
        assert_eq!(failure.field.as_deref(), Some("projections[0]"));
        assert!(failure.reason.contains("Ambiguous"));
    }

    #[test]
    fn implicit_projections_use_only_registered_default_columns() {
        let compiler = SqlxRelationalQueryCompiler::new(sources());
        let plan = RelationalQueryPlan::new(QuerySource::new("orders"));

        let compiled = compiler.compile(&plan).unwrap();

        assert!(compiled.statement.sql.contains("\"orders\".\"id\""));
        assert!(compiled.statement.sql.contains("\"orders\".\"status\""));
        assert!(!compiled.statement.sql.contains("secret"));
    }

    #[test]
    fn rejects_unregistered_physical_columns() {
        let compiler = SqlxRelationalQueryCompiler::new(sources());
        let plan = RelationalQueryPlan::new(QuerySource::new("orders").with_alias("o"))
            .with_predicate(BooleanExpression::eq(
                ScalarExpression::reference("o.secret"),
                ScalarExpression::constant(ExpressionValue::from("hidden")),
            ));

        let failure = compiler.compile(&plan).unwrap_err();

        assert_eq!(failure.category, QueryExecutionErrorCategory::UnknownColumn);
        assert_eq!(failure.field.as_deref(), Some("predicate"));
    }

    #[test]
    fn classifies_incompatible_predicate_constants_as_parameter_binding() {
        let compiler = SqlxRelationalQueryCompiler::new(sources());
        let plan = RelationalQueryPlan::new(QuerySource::new("orders").with_alias("o"))
            .with_predicate(BooleanExpression::eq(
                ScalarExpression::reference("o.id"),
                ScalarExpression::constant(ExpressionValue::from("not-a-number")),
            ));

        let failure = compiler.compile(&plan).unwrap_err();

        assert_eq!(
            failure.category,
            QueryExecutionErrorCategory::ParameterBinding
        );
        assert_eq!(failure.field.as_deref(), Some("predicate"));
    }

    #[test]
    fn target_binder_can_adapt_relation_predicate_constants() {
        let compiler = SqlxRelationalQueryCompiler::new(sources()).with_target_constant_binder(
            |value: &ExpressionValue, target: &SqlxValueType| {
                if matches!(target, SqlxValueType::Number)
                    && matches!(value, ExpressionValue::String(value) if value == "7")
                {
                    Ok(Some(ExpressionValue::from(7)))
                } else {
                    Ok(None)
                }
            },
        );
        let plan = RelationalQueryPlan::new(QuerySource::new("orders").with_alias("o"))
            .with_predicate(BooleanExpression::eq(
                ScalarExpression::reference("o.id"),
                ScalarExpression::constant(ExpressionValue::from("7")),
            ));

        let compiled = compiler.compile(&plan).unwrap();

        assert_eq!(compiled.statement.params, vec![ExpressionValue::from(7)]);
        assert_eq!(compiled.audit.parameter_types, vec!["number"]);
    }

    #[test]
    fn target_binder_errors_are_classified_as_parameter_binding() {
        let compiler = SqlxRelationalQueryCompiler::new(sources()).with_target_constant_binder(
            |_: &ExpressionValue, _: &SqlxValueType| Err("binder rejected value".to_string()),
        );
        let plan = RelationalQueryPlan::new(QuerySource::new("orders").with_alias("o"))
            .with_predicate(BooleanExpression::eq(
                ScalarExpression::reference("o.status"),
                ScalarExpression::constant(ExpressionValue::from("confirmed")),
            ));

        let failure = compiler.compile(&plan).unwrap_err();

        assert_eq!(
            failure.category,
            QueryExecutionErrorCategory::ParameterBinding
        );
        assert!(failure.reason.contains("binder rejected value"));
    }

    #[test]
    fn classifies_in_candidates_against_the_registered_column_type() {
        let compiler = SqlxRelationalQueryCompiler::new(sources());
        let plan = RelationalQueryPlan::new(QuerySource::new("orders").with_alias("o"))
            .with_predicate(BooleanExpression::in_expr(
                ScalarExpression::reference("o.id"),
                vec![ScalarExpression::constant(ExpressionValue::from("7"))],
                false,
            ));

        let failure = compiler.compile(&plan).unwrap_err();

        assert_eq!(
            failure.category,
            QueryExecutionErrorCategory::ParameterBinding
        );
    }

    #[test]
    fn compiles_mysql_join_and_pagination() {
        let compiler = SqlxRelationalQueryCompiler::with_config(
            sources(),
            SqlxTranslatorConfig::default().with_dialect(SqlxDialect::MySql),
        );
        let plan = RelationalQueryPlan::new(QuerySource::new("orders").with_alias("o"))
            .with_joins([JoinSpec::new(
                JoinType::Inner,
                QuerySource::new("items").with_alias("i"),
                join_condition(),
                JoinCardinality::OneToMany,
            )])
            .with_predicate(BooleanExpression::eq(
                ScalarExpression::reference("o.status"),
                ScalarExpression::constant(ExpressionValue::from("confirmed")),
            ))
            .with_projections([ProjectionSpec::new(ColumnRef::new("o", "id"))])
            .with_distinct(true)
            .with_page(crate::persistence::PageSpec::new(10, 5));

        let compiled = compiler.compile(&plan).unwrap();

        assert!(compiled.statement.sql.contains("INNER JOIN"));
        assert!(compiled.statement.sql.contains("LIMIT ? OFFSET ?"));
        assert!(compiled.statement.sql.contains("`o`.`status` = ?"));
    }

    #[test]
    fn uses_registered_table_name_when_logical_source_has_no_alias() {
        let compiler = SqlxRelationalQueryCompiler::new(sources());
        let plan = RelationalQueryPlan::new(QuerySource::new("items"))
            .with_projections([ProjectionSpec::new(ColumnRef::new("items", "order_id"))]);

        let compiled = compiler.compile(&plan).unwrap();

        assert_eq!(
            compiled.statement.sql,
            r#"SELECT "order_items"."order_id" FROM "order_items""#
        );
    }

    #[test]
    fn remaps_qualified_resolver_columns_to_query_aliases() {
        let items =
            PredicateSchema::new("items").with_field_name("order_id", "order_items.order_id");
        let compiler = SqlxRelationalQueryCompiler::new(HashMap::from([(
            "items".to_string(),
            SqlxQuerySource::new(QuerySource::new("items"), "order_items", items),
        )]));
        let plan = RelationalQueryPlan::new(QuerySource::new("items").with_alias("i"))
            .with_projections([ProjectionSpec::new(ColumnRef::new("i", "order_id"))]);

        let compiled = compiler.compile(&plan).unwrap();

        assert_eq!(
            compiled.statement.sql,
            r#"SELECT "i"."order_id" FROM "order_items" AS "i""#
        );
    }

    #[test]
    fn classifies_join_translation_failures_as_invalid_joins() {
        let compiler = SqlxRelationalQueryCompiler::new(sources());
        let condition = BooleanExpression::and(vec![
            join_condition(),
            BooleanExpression::Custom {
                payload: "unsupported".to_string(),
                description: None,
            },
        ]);
        let plan =
            RelationalQueryPlan::new(QuerySource::new("orders").with_alias("o")).with_joins([
                JoinSpec::new(
                    JoinType::Inner,
                    QuerySource::new("items").with_alias("i"),
                    condition,
                    JoinCardinality::OneToMany,
                ),
            ]);

        let failure = compiler.compile(&plan).unwrap_err();

        assert_eq!(failure.category, QueryExecutionErrorCategory::InvalidJoin);
        assert_eq!(failure.field.as_deref(), Some("joins[0].condition"));
    }

    #[test]
    fn execute_with_reports_truncation() {
        let compiler = SqlxRelationalQueryCompiler::new(sources());
        let plan = RelationalQueryPlan::new(QuerySource::new("orders"))
            .with_projections([ProjectionSpec::new(ColumnRef::new("orders", "id"))]);
        let compiled = compiler.compile(&plan).unwrap();
        let result = compiled
            .execute_with(Some(1), |_| Ok::<_, ()>(vec![1, 2]))
            .unwrap();
        assert_eq!(result.value, vec![1]);
        assert!(result.stats.truncated);
    }

    #[test]
    fn execute_with_rejects_non_positive_row_limit() {
        let compiler = SqlxRelationalQueryCompiler::new(sources());
        let plan = RelationalQueryPlan::new(QuerySource::new("orders"))
            .with_projections([ProjectionSpec::new(ColumnRef::new("orders", "id"))]);
        let compiled = compiler.compile(&plan).unwrap();
        let error = compiled
            .execute_with(Some(0), |_| Ok::<_, ()>(vec![1]))
            .unwrap_err();

        match error {
            SqlxQueryExecutionError::InvalidRequest(failure) => {
                assert_eq!(failure.category, QueryExecutionErrorCategory::SqlGeneration);
                assert_eq!(failure.field.as_deref(), Some("maxReturnedRows"));
            }
            SqlxQueryExecutionError::Executor(()) => {
                panic!("invalid row limit must fail before invoking the executor")
            }
        }
    }

    #[test]
    fn execute_with_classifier_preserves_explicit_execution_categories() {
        #[derive(Debug, Clone, Copy)]
        enum AdapterError {
            Timeout,
            UnsupportedDialect,
            Database,
        }

        let compiler = SqlxRelationalQueryCompiler::new(sources());
        let plan = RelationalQueryPlan::new(QuerySource::new("orders"))
            .with_projections([ProjectionSpec::new(ColumnRef::new("orders", "id"))]);
        let compiled = compiler.compile(&plan).unwrap();

        for (error, category) in [
            (AdapterError::Timeout, QueryExecutionErrorCategory::Timeout),
            (
                AdapterError::UnsupportedDialect,
                QueryExecutionErrorCategory::UnsupportedDialect,
            ),
            (
                AdapterError::Database,
                QueryExecutionErrorCategory::Database,
            ),
        ] {
            let failure = compiled
                .execute_with_classifier(
                    None,
                    move |_| Err::<Vec<i32>, _>(error),
                    |error| RelationalQueryFailure {
                        category: match error {
                            AdapterError::Timeout => QueryExecutionErrorCategory::Timeout,
                            AdapterError::UnsupportedDialect => {
                                QueryExecutionErrorCategory::UnsupportedDialect
                            }
                            AdapterError::Database => QueryExecutionErrorCategory::Database,
                        },
                        field: None,
                        reason: "adapter classified execution failure".to_string(),
                    },
                )
                .unwrap_err();
            assert_eq!(failure.category, category);
        }
    }

    #[test]
    fn explicit_projections_still_validate_root_default_columns() {
        let orders = PredicateSchema::new("orders").with_field_name("id", "id");
        let compiler = SqlxRelationalQueryCompiler::new(HashMap::from([(
            "orders".to_string(),
            SqlxQuerySource::new(QuerySource::new("orders"), "orders", orders)
                .with_default_columns(["missing"]),
        )]));
        let plan = RelationalQueryPlan::new(QuerySource::new("orders"))
            .with_projections([ProjectionSpec::new(ColumnRef::new("orders", "id"))]);

        let failure = compiler.compile(&plan).unwrap_err();
        assert_eq!(failure.category, QueryExecutionErrorCategory::UnknownColumn);
        assert_eq!(failure.field.as_deref(), Some("root.default_columns[0]"));
    }
}
