//! 通用关系查询计划
//! Generic relational query plan

use ospf_rust_math::symbol::{
    BooleanExpression, DynSymbol, ExpressionValue, NormalizeConfig, OwnedSymbol, ScalarExpression,
    SymbolDynId, path_owned_symbol, property_path_from_owned_symbol,
};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fmt::{Display, Formatter};

/// 关系查询数据源。
/// Relational query source.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QuerySource {
    /// 适配器注册的数据源名称 / Adapter-registered source name
    pub name: String,
    /// 查询计划中使用的可选别名 / Optional alias used by the query plan
    pub alias: Option<String>,
}

impl QuerySource {
    /// 创建数据源。
    /// Create a query source.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into().trim().to_string(),
            alias: None,
        }
    }

    /// 设置数据源别名。
    /// Set the source alias.
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into().trim().to_string());
        self
    }

    fn normalized(&self) -> Self {
        Self {
            name: self.name.trim().to_string(),
            alias: self.alias.as_deref().map(str::trim).map(str::to_string),
        }
    }
}

/// 关系查询列引用。
/// Relational query column reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ColumnRef {
    /// 数据源名称或别名 / Source name or alias
    pub source: String,
    /// 适配器注册的字段路径 / Adapter-registered field path
    pub path: String,
}

impl ColumnRef {
    /// 创建列引用。
    /// Create a column reference.
    pub fn new(source: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            source: source.into().trim().to_string(),
            path: path.into().trim().to_string(),
        }
    }

    fn normalized(&self) -> Self {
        Self::new(&self.source, &self.path)
    }
}

/// Join 类型。
/// Join type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JoinType {
    /// 内连接 / Inner join
    Inner,
    /// 左外连接 / Left outer join
    Left,
    /// 相关存在性半连接 / Correlated existence semi-join
    Exists,
}

/// Join 基数。
/// Join cardinality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JoinCardinality {
    /// 一对一 / One-to-one
    OneToOne,
    /// 多对一 / Many-to-one
    ManyToOne,
    /// 一对多 / One-to-many
    OneToMany,
    /// 多对多 / Many-to-many
    ManyToMany,
}

impl JoinCardinality {
    /// 判断目标侧是否可能包含多行。
    /// Check whether the target side may contain multiple rows.
    pub const fn is_to_many(self) -> bool {
        matches!(self, Self::OneToMany | Self::ManyToMany)
    }
}

/// Join 规格。
/// Join specification.
#[derive(Debug, Clone, PartialEq)]
pub struct JoinSpec {
    /// Join 类型 / Join type
    pub join_type: JoinType,
    /// 待加入的数据源 / Source to join
    pub source: QuerySource,
    /// Join 关联条件 / Join correlation condition
    pub condition: BooleanExpression<ExpressionValue>,
    /// 声明的数据源基数 / Declared source cardinality
    pub cardinality: JoinCardinality,
}

impl JoinSpec {
    /// 创建 Join 规格。
    /// Create a join specification.
    pub fn new(
        join_type: JoinType,
        source: QuerySource,
        condition: BooleanExpression<ExpressionValue>,
        cardinality: JoinCardinality,
    ) -> Self {
        Self {
            join_type,
            source: source.normalized(),
            condition: snapshot_boolean_expression(&condition),
            cardinality,
        }
    }

    fn normalized(&self) -> Self {
        Self::new(
            self.join_type,
            self.source.clone(),
            self.condition.clone(),
            self.cardinality,
        )
    }
}

/// 投影字段。
/// Projection field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionSpec {
    /// 要返回的列 / Column to return
    pub column: ColumnRef,
    /// 返回结果中的可选别名 / Optional alias in the returned result
    pub alias: Option<String>,
}

impl ProjectionSpec {
    /// 创建投影字段。
    /// Create a projection field.
    pub fn new(column: ColumnRef) -> Self {
        Self {
            column,
            alias: None,
        }
    }

    /// 设置投影别名。
    /// Set the projection alias.
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into().trim().to_string());
        self
    }

    fn normalized(&self) -> Self {
        Self {
            column: self.column.normalized(),
            alias: self.alias.as_deref().map(str::trim).map(str::to_string),
        }
    }
}

/// 排序方向。
/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortDirection {
    /// 升序 / Ascending order
    Ascending,
    /// 降序 / Descending order
    Descending,
}

/// NULL 排序位置。
/// NULL ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NullsOrder {
    /// 不指定 NULL 位置 / Do not specify NULL placement
    Unspecified,
    /// NULL 排在前面 / NULL values first
    First,
    /// NULL 排在后面 / NULL values last
    Last,
}

/// 排序规格。
/// Order specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderSpec {
    /// 要排序的列 / Column to sort by
    pub column: ColumnRef,
    /// 排序方向 / Sort direction
    pub direction: SortDirection,
    /// NULL 值排序位置 / NULL value placement
    pub nulls: NullsOrder,
}

impl OrderSpec {
    /// 创建排序规格。
    /// Create an order specification.
    pub fn new(column: ColumnRef) -> Self {
        Self {
            column,
            direction: SortDirection::Ascending,
            nulls: NullsOrder::Unspecified,
        }
    }

    /// 设置排序方向。
    /// Set the sort direction.
    pub fn with_direction(mut self, direction: SortDirection) -> Self {
        self.direction = direction;
        self
    }

    /// 设置 NULL 排序位置。
    /// Set the NULL ordering.
    pub fn with_nulls(mut self, nulls: NullsOrder) -> Self {
        self.nulls = nulls;
        self
    }

    fn normalized(&self) -> Self {
        Self {
            column: self.column.normalized(),
            direction: self.direction,
            nulls: self.nulls,
        }
    }
}

/// 分页规格。
/// Page specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageSpec {
    /// 单页最大记录数 / Maximum records per page
    pub limit: usize,
    /// 跳过的记录数 / Number of records to skip
    pub offset: usize,
}

impl PageSpec {
    /// 创建分页规格。
    /// Create a page specification.
    pub const fn new(limit: usize, offset: usize) -> Self {
        Self { limit, offset }
    }
}

/// 查询计划限制。
/// Query plan limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelationalQueryLimits {
    /// 允许的最大 Join 数 / Maximum number of allowed joins
    pub max_joins: usize,
    /// 允许的最大数据源深度 / Maximum allowed source depth
    pub max_depth: usize,
    /// 允许的最大投影字段数 / Maximum number of allowed projections
    pub max_projections: usize,
}

/// 查询执行错误分类。
/// Query execution error category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryExecutionErrorCategory {
    /// SQL 生成失败 / SQL generation failure
    SqlGeneration,
    /// 数据源不存在 / Unknown source
    UnknownSource,
    /// 字段不存在或存在歧义 / Unknown or ambiguous column
    UnknownColumn,
    /// Join 结构非法 / Invalid join
    InvalidJoin,
    /// 参数绑定失败 / Parameter binding failure
    ParameterBinding,
    /// 方言不支持 / Unsupported dialect
    UnsupportedDialect,
    /// 查询超时 / Query timeout
    Timeout,
    /// 数据库执行失败 / Database failure
    Database,
}

/// 关系查询结构化失败。
/// Structured relational query failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationalQueryFailure {
    /// 错误分类 / Error category
    pub category: QueryExecutionErrorCategory,
    /// 失败字段 / Failing plan field
    pub field: Option<String>,
    /// 失败原因 / Failure reason
    pub reason: String,
}

impl Display for RelationalQueryFailure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(field) = &self.field {
            write!(formatter, "{}: {}", field, self.reason)
        } else {
            formatter.write_str(&self.reason)
        }
    }
}

impl std::error::Error for RelationalQueryFailure {}

/// 可审计的查询摘要。
/// Auditable query summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryAuditSummary {
    /// 参数化 SQL 模板 / Parameterized SQL template
    pub sql_template: String,
    /// 参数类型摘要 / Parameter type summary
    pub parameter_types: Vec<String>,
    /// SQL 方言 / SQL dialect
    pub dialect: String,
}

/// 查询执行统计。
/// Query execution statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QueryExecutionStats {
    /// 执行耗时（毫秒） / Execution duration in milliseconds
    pub duration_ms: u128,
    /// 返回行数 / Returned row count
    pub returned_rows: u64,
    /// 扫描行数（无法可靠获取时为空） / Scanned rows when reliably available
    pub scanned_rows: Option<u64>,
    /// 扫描行数是否精确 / Whether scanned rows are exact
    pub scanned_rows_exact: bool,
    /// 结果是否被上限截断 / Whether the result was truncated
    pub truncated: bool,
}

/// 带执行统计的查询结果。
/// Query result with execution statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryExecutionResult<T> {
    /// 查询结果 / Query value
    pub value: T,
    /// 执行统计 / Execution statistics
    pub stats: QueryExecutionStats,
}

impl Default for RelationalQueryLimits {
    fn default() -> Self {
        Self {
            max_joins: 16,
            max_depth: 16,
            max_projections: 128,
        }
    }
}

/// 关系查询计划校验错误。
/// Relational query plan validation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationalQueryValidationError {
    /// 发生错误的计划字段 / Plan field containing the error
    pub field: String,
    /// 校验失败原因 / Reason for validation failure
    pub reason: String,
}

impl Display for RelationalQueryValidationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.field, self.reason)
    }
}

impl std::error::Error for RelationalQueryValidationError {}

/// 关系查询计划容器。
/// Relational query plan container.
///
/// 计划边界会递归复制拥有型表达式树，并将动态符号元数据快照为稳定值；Rust 的表达式负载不包含 Kotlin
/// `Any` 式不透明容器，因此无需为可变集合或循环容器增加运行时拒绝协议。
/// The plan boundary recursively copies owned expression trees and snapshots dynamic symbol metadata as stable
/// values. Rust expression payloads do not contain Kotlin-style opaque `Any` containers, so no runtime rejection
/// protocol for mutable collections or cyclic containers is required here.
///
/// `canonical` 只编码规范化后的表达式形状和字面量类型，不编码标量字面量原值。
/// `canonical` encodes normalized expression shape and literal types, but not scalar literal values.
#[derive(Debug, Clone, PartialEq)]
pub struct RelationalQueryPlan {
    root: QuerySource,
    joins: Vec<JoinSpec>,
    predicate: Option<BooleanExpression<ExpressionValue>>,
    projections: Vec<ProjectionSpec>,
    distinct: bool,
    group_by: Vec<ColumnRef>,
    order_by: Vec<OrderSpec>,
    page: Option<PageSpec>,
    offset_without_limit: Option<usize>,
    root_key: Vec<ColumnRef>,
}

impl RelationalQueryPlan {
    /// 创建关系查询计划。
    /// Create a relational query plan.
    pub fn new(root: QuerySource) -> Self {
        Self {
            root: root.normalized(),
            joins: Vec::new(),
            predicate: None,
            projections: Vec::new(),
            distinct: false,
            group_by: Vec::new(),
            order_by: Vec::new(),
            page: None,
            offset_without_limit: None,
            root_key: Vec::new(),
        }
    }

    /// 获取根数据源。
    /// Get the root source.
    pub fn root(&self) -> &QuerySource {
        &self.root
    }

    /// 获取按顺序应用的 Join。
    /// Get joins applied in order.
    pub fn joins(&self) -> &[JoinSpec] {
        &self.joins
    }

    /// 获取根查询谓词。
    /// Get the root query predicate.
    pub fn predicate(&self) -> Option<&BooleanExpression<ExpressionValue>> {
        self.predicate.as_ref()
    }

    /// 获取投影字段。
    /// Get projection fields.
    pub fn projections(&self) -> &[ProjectionSpec] {
        &self.projections
    }

    /// 获取是否去重。
    /// Get whether the projection is distinct.
    pub const fn distinct(&self) -> bool {
        self.distinct
    }

    /// 获取分组字段。
    /// Get grouping fields.
    pub fn group_by(&self) -> &[ColumnRef] {
        &self.group_by
    }

    /// 获取排序规格。
    /// Get ordering specifications.
    pub fn order_by(&self) -> &[OrderSpec] {
        &self.order_by
    }

    /// 获取分页规格。
    /// Get the pagination specification.
    pub const fn page(&self) -> Option<PageSpec> {
        self.page
    }

    /// 获取无上限分页的偏移量。
    /// Get the offset for an unbounded-offset page.
    pub const fn offset_without_limit(&self) -> Option<usize> {
        self.offset_without_limit
    }

    /// 获取根粒度计数使用的根键。
    /// Get the root key used for root-granularity counts.
    pub fn root_key(&self) -> &[ColumnRef] {
        &self.root_key
    }

    /// 设置 Join 列表，并复制输入集合。
    /// Set joins while copying the input collection.
    pub fn with_joins(mut self, joins: impl IntoIterator<Item = JoinSpec>) -> Self {
        self.joins = joins.into_iter().map(|join| join.normalized()).collect();
        self
    }

    /// 设置谓词。
    /// Set the predicate.
    pub fn with_predicate(mut self, predicate: BooleanExpression<ExpressionValue>) -> Self {
        self.predicate = Some(snapshot_boolean_expression(&predicate));
        self
    }

    /// 设置投影列表，并复制输入集合。
    /// Set projections while copying the input collection.
    pub fn with_projections(
        mut self,
        projections: impl IntoIterator<Item = ProjectionSpec>,
    ) -> Self {
        self.projections = projections
            .into_iter()
            .map(|projection| projection.normalized())
            .collect();
        self
    }

    /// 设置去重。
    /// Set distinct projection mode.
    pub const fn with_distinct(mut self, distinct: bool) -> Self {
        self.distinct = distinct;
        self
    }

    /// 设置分组字段，并复制输入集合。
    /// Set grouping fields while copying the input collection.
    pub fn with_group_by(mut self, group_by: impl IntoIterator<Item = ColumnRef>) -> Self {
        self.group_by = group_by
            .into_iter()
            .map(|column| column.normalized())
            .collect();
        self
    }

    /// 设置排序规格，并复制输入集合。
    /// Set ordering specifications while copying the input collection.
    pub fn with_order_by(mut self, order_by: impl IntoIterator<Item = OrderSpec>) -> Self {
        self.order_by = order_by
            .into_iter()
            .map(|order| order.normalized())
            .collect();
        self
    }

    /// 设置分页。
    /// Set pagination.
    pub const fn with_page(mut self, page: PageSpec) -> Self {
        self.page = Some(page);
        self.offset_without_limit = None;
        self
    }

    /// 设置仅偏移、不限制返回数量的分页。
    /// Set pagination with an offset and no result limit.
    pub const fn with_offset_without_limit(mut self, offset: usize) -> Self {
        self.page = None;
        self.offset_without_limit = Some(offset);
        self
    }

    /// 设置根键，并复制输入集合。
    /// Set root keys while copying the input collection.
    pub fn with_root_key(mut self, root_key: impl IntoIterator<Item = ColumnRef>) -> Self {
        self.root_key = root_key
            .into_iter()
            .map(|column| column.normalized())
            .collect();
        self
    }

    /// 校验计划结构。
    /// Validate the plan structure.
    pub fn validate(&self) -> Result<(), RelationalQueryValidationError> {
        self.validate_with_limits(RelationalQueryLimits::default())
    }

    /// 使用限制校验计划结构。
    /// Validate the plan structure with explicit limits.
    pub fn validate_with_limits(
        &self,
        limits: RelationalQueryLimits,
    ) -> Result<(), RelationalQueryValidationError> {
        let failure = |field: &str, reason: &str| {
            Err(RelationalQueryValidationError {
                field: field.to_string(),
                reason: reason.to_string(),
            })
        };

        if self.root.name.is_empty() {
            return failure("root", "Root source name must not be blank");
        }
        if self.root.alias.as_deref() == Some("") {
            return failure("root.alias", "Root source alias must not be blank");
        }
        if self.root.alias.as_deref() == Some(self.root.name.as_str()) {
            return failure("root.alias", "Root source alias must differ from its name");
        }
        if self.joins.len() > limits.max_joins {
            return failure("joins", "Join count exceeds configured limit");
        }
        if self.joins.len().saturating_add(1) > limits.max_depth {
            return failure("joins", "Join depth exceeds configured limit");
        }
        if self.projections.len() > limits.max_projections {
            return failure("projections", "Projection count exceeds configured limit");
        }
        if let Some(page) = self.page
            && page.limit == 0
        {
            return failure(
                "page",
                "Page limit must be positive and offset must not be negative",
            );
        }
        if self.root_key.iter().any(empty_column) {
            return failure("rootKey", "Root key contains an empty column reference");
        }
        if self
            .projections
            .iter()
            .any(|projection| empty_column(&projection.column))
        {
            return failure(
                "projections",
                "Projection contains an empty column reference",
            );
        }
        if self.group_by.iter().any(empty_column) {
            return failure("groupBy", "Group by contains an empty column reference");
        }
        if self
            .order_by
            .iter()
            .any(|order| empty_column(&order.column))
        {
            return failure("orderBy", "Order by contains an empty column reference");
        }

        let mut names = HashSet::from([self.root.name.clone()]);
        let mut aliases = HashSet::new();
        let mut bound_sources = HashSet::from([self.root.name.clone()]);
        if let Some(alias) = &self.root.alias {
            aliases.insert(alias.clone());
            bound_sources.insert(alias.clone());
        }

        for (index, join) in self.joins.iter().enumerate() {
            let field = format!("joins[{index}]");
            if join.source.name.is_empty() {
                return failure(
                    &format!("{field}.source"),
                    "Join source name must not be blank",
                );
            }
            if aliases.contains(&join.source.name) {
                return failure(
                    &format!("{field}.source"),
                    &format!(
                        "Join source name conflicts with an alias: {}",
                        join.source.name
                    ),
                );
            }
            if !names.insert(join.source.name.clone()) {
                return failure(
                    &format!("{field}.source"),
                    &format!("Join source is duplicated: {}", join.source.name),
                );
            }
            if let Some(alias) = &join.source.alias {
                if alias.is_empty() {
                    return failure(
                        &format!("{field}.source.alias"),
                        "Join source alias must not be blank",
                    );
                }
                if alias == &join.source.name || names.contains(alias) || aliases.contains(alias) {
                    return failure(
                        &format!("{field}.source.alias"),
                        &format!("Join source alias is ambiguous: {alias}"),
                    );
                }
                aliases.insert(alias.clone());
            }
            if join.condition.collect_references().is_empty() {
                return failure(
                    &format!("{field}.condition"),
                    "Join condition must contain a qualified column relationship",
                );
            }
            if contains_boolean_constant(&join.condition) {
                return failure(
                    &format!("{field}.condition"),
                    "Join condition must not contain a boolean constant",
                );
            }
            let target_sources = source_names(&join.source);
            if let Some(unknown) = join
                .condition
                .collect_references()
                .into_iter()
                .filter_map(|reference| source_qualifier(reference.value()).map(str::to_string))
                .find(|qualifier| {
                    !bound_sources.contains(qualifier.as_str())
                        && !target_sources.contains(qualifier.as_str())
                })
            {
                return failure(
                    &format!("{field}.condition"),
                    &format!(
                        "Join condition references an unknown or not-yet-bound source: {unknown}"
                    ),
                );
            }
            if !contains_exact_column_correlation(&join.condition, &target_sources, &bound_sources)
            {
                return failure(
                    &format!("{field}.condition"),
                    "Join condition must relate the joined source to an already bound source",
                );
            }
            if join.join_type == JoinType::Exists && !join.cardinality.is_to_many() {
                return failure(
                    &format!("{field}.cardinality"),
                    "Exists join requires a to-many cardinality declaration",
                );
            }
            if join.join_type != JoinType::Exists {
                bound_sources.extend(target_sources);
            }
        }
        Ok(())
    }

    /// 返回稳定的规范化计划表示。
    /// Return a stable canonical representation of the normalized plan.
    pub fn canonical(&self) -> String {
        let mut canonical = String::new();
        canonical.push_str("root=");
        append_source(&mut canonical, &self.root);
        canonical.push_str("|joins=");
        canonical.push_str(&self.joins.len().to_string());
        for join in &self.joins {
            canonical.push(';');
            canonical.push_str(&format!("{:?}:{:?}:", join.join_type, join.cardinality));
            append_source(&mut canonical, &join.source);
            canonical.push(':');
            append_token(&mut canonical, &normalized_key(&join.condition));
        }
        canonical.push_str("|predicate=");
        append_token(
            &mut canonical,
            &self
                .predicate
                .as_ref()
                .map(normalized_key)
                .unwrap_or_default(),
        );
        canonical.push_str("|projections=");
        canonical.push_str(&self.projections.len().to_string());
        for projection in &self.projections {
            canonical.push(';');
            append_column(&mut canonical, &projection.column);
            canonical.push(':');
            append_token(
                &mut canonical,
                projection.alias.as_deref().unwrap_or_default(),
            );
        }
        canonical.push_str("|distinct=");
        canonical.push_str(if self.distinct { "true" } else { "false" });
        canonical.push_str("|groupBy=");
        append_columns(&mut canonical, &self.group_by);
        canonical.push_str("|orderBy=");
        canonical.push_str(&self.order_by.len().to_string());
        for order in &self.order_by {
            canonical.push(';');
            append_column(&mut canonical, &order.column);
            canonical.push(':');
            canonical.push_str(&format!("{:?}:{:?}", order.direction, order.nulls));
        }
        canonical.push_str("|page=");
        if let Some(page) = self.page {
            canonical.push_str(&format!("{}:{}", page.limit, page.offset));
        } else if let Some(offset) = self.offset_without_limit {
            canonical.push_str(&format!("offset:{offset}"));
        }
        canonical.push_str("|rootKey=");
        append_columns(&mut canonical, &self.root_key);
        canonical
    }

    /// 获取规范化计划的 SHA-256 摘要。
    /// Get the SHA-256 digest of the canonical plan.
    pub fn canonical_hash(&self) -> String {
        let digest = Sha256::digest(self.canonical().as_bytes());
        digest.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}

fn empty_column(column: &ColumnRef) -> bool {
    column.source.is_empty() || column.path.is_empty()
}

fn source_names(source: &QuerySource) -> HashSet<String> {
    let mut names = HashSet::from([source.name.clone()]);
    if let Some(alias) = &source.alias {
        names.insert(alias.clone());
    }
    names
}

fn source_qualifier(path: &str) -> Option<&str> {
    let (qualifier, field) = path.split_once('.')?;
    (!qualifier.is_empty() && !field.is_empty()).then_some(qualifier)
}

fn normalized_key(expression: &BooleanExpression<ExpressionValue>) -> String {
    let normalized = normalize_for_canonical(expression);
    canonical_query_key(&normalized)
}

fn normalize_for_canonical(
    expression: &BooleanExpression<ExpressionValue>,
) -> BooleanExpression<ExpressionValue> {
    let prepared = match expression {
        BooleanExpression::Constant(_)
        | BooleanExpression::NullCheck { .. }
        | BooleanExpression::Custom { .. } => expression.clone(),
        BooleanExpression::Comparison {
            operator,
            left,
            right,
        } => BooleanExpression::Comparison {
            operator: *operator,
            left: normalize_scalar_for_canonical(left),
            right: normalize_scalar_for_canonical(right),
        },
        BooleanExpression::In {
            value,
            candidates,
            negated,
        } => BooleanExpression::In {
            value: normalize_scalar_for_canonical(value),
            candidates: candidates
                .iter()
                .map(normalize_scalar_for_canonical)
                .collect(),
            negated: *negated,
        },
        BooleanExpression::PatternMatch {
            value,
            pattern,
            mode,
            negated,
        } => BooleanExpression::PatternMatch {
            value: normalize_scalar_for_canonical(value),
            pattern: normalize_scalar_for_canonical(pattern),
            mode: *mode,
            negated: *negated,
        },
        BooleanExpression::And(operands) => {
            BooleanExpression::And(operands.iter().map(normalize_for_canonical).collect())
        }
        BooleanExpression::Or(operands) => {
            BooleanExpression::Or(operands.iter().map(normalize_for_canonical).collect())
        }
        BooleanExpression::Not(operand) => {
            BooleanExpression::Not(Box::new(normalize_for_canonical(operand)))
        }
    };

    deduplicate_for_canonical(prepared.normalize_with_config(NormalizeConfig {
        deduplicate: false,
        sort_operands: false,
        ..NormalizeConfig::default()
    }))
}

fn normalize_scalar_for_canonical(
    expression: &ScalarExpression<ExpressionValue>,
) -> ScalarExpression<ExpressionValue> {
    match expression {
        ScalarExpression::Constant(value) => ScalarExpression::Constant(value.clone()),
        ScalarExpression::Reference(path) => ScalarExpression::Reference(path.clone()),
        ScalarExpression::SymbolReference(symbol) => {
            ScalarExpression::SymbolReference(symbol.clone())
        }
        ScalarExpression::Unary { operator, operand } => ScalarExpression::Unary {
            operator: *operator,
            operand: Box::new(normalize_scalar_for_canonical(operand)),
        },
        ScalarExpression::Binary {
            operator,
            left,
            right,
        } => ScalarExpression::Binary {
            operator: *operator,
            left: Box::new(normalize_scalar_for_canonical(left)),
            right: Box::new(normalize_scalar_for_canonical(right)),
        },
        ScalarExpression::Function { name, arguments } => ScalarExpression::Function {
            name: name.clone(),
            arguments: arguments
                .iter()
                .map(normalize_scalar_for_canonical)
                .collect(),
        },
        ScalarExpression::Custom {
            payload,
            description,
        } => ScalarExpression::Custom {
            payload: payload.clone(),
            description: description.clone(),
        },
        ScalarExpression::Conditional {
            condition,
            then_branch,
            else_branch,
        } => ScalarExpression::Conditional {
            condition: Box::new(normalize_for_canonical(condition)),
            then_branch: Box::new(normalize_scalar_for_canonical(then_branch)),
            else_branch: Box::new(normalize_scalar_for_canonical(else_branch)),
        },
        ScalarExpression::Boolean(expression) => {
            ScalarExpression::Boolean(Box::new(normalize_for_canonical(expression)))
        }
    }
}

fn deduplicate_for_canonical(
    expression: BooleanExpression<ExpressionValue>,
) -> BooleanExpression<ExpressionValue> {
    match expression {
        BooleanExpression::And(operands) => {
            let mut seen = HashSet::new();
            let operands = operands
                .into_iter()
                .map(deduplicate_for_canonical)
                .filter(|operand| seen.insert(normalization_key(operand)))
                .collect::<Vec<_>>();
            match operands.as_slice() {
                [] => BooleanExpression::true_constant(),
                [operand] => operand.clone(),
                _ => BooleanExpression::And(operands),
            }
        }
        BooleanExpression::Or(operands) => {
            let mut seen = HashSet::new();
            let operands = operands
                .into_iter()
                .map(deduplicate_for_canonical)
                .filter(|operand| seen.insert(normalization_key(operand)))
                .collect::<Vec<_>>();
            match operands.as_slice() {
                [] => BooleanExpression::false_constant(),
                [operand] => operand.clone(),
                _ => BooleanExpression::Or(operands),
            }
        }
        BooleanExpression::Not(operand) => {
            BooleanExpression::Not(Box::new(deduplicate_for_canonical(*operand)))
        }
        expression => expression,
    }
}

fn normalization_key(expression: &BooleanExpression<ExpressionValue>) -> String {
    match expression {
        BooleanExpression::Constant(value) => {
            canonical_node("BooleanConstant", &[format!("{value:?}")])
        }
        BooleanExpression::Comparison {
            operator,
            left,
            right,
        } => canonical_node(
            "Comparison",
            &[
                format!("{operator:?}"),
                normalization_scalar_key(left),
                normalization_scalar_key(right),
            ],
        ),
        BooleanExpression::In {
            value,
            candidates,
            negated,
        } => {
            let mut candidates = candidates
                .iter()
                .map(normalization_scalar_key)
                .collect::<Vec<_>>();
            candidates.sort();
            canonical_node(
                "In",
                &[
                    negated.to_string(),
                    normalization_scalar_key(value),
                    canonical_node("Candidates", &candidates),
                ],
            )
        }
        BooleanExpression::PatternMatch {
            value,
            pattern,
            mode,
            negated,
        } => canonical_node(
            "PatternMatch",
            &[
                format!("{mode:?}"),
                negated.to_string(),
                normalization_scalar_key(value),
                normalization_scalar_key(pattern),
            ],
        ),
        BooleanExpression::NullCheck {
            path,
            null_check_type,
        } => canonical_node(
            "NullCheck",
            &[format!("{null_check_type:?}"), path.value().to_string()],
        ),
        BooleanExpression::And(operands) => {
            let mut operands = operands.iter().map(normalization_key).collect::<Vec<_>>();
            operands.sort();
            canonical_node("And", &operands)
        }
        BooleanExpression::Or(operands) => {
            let mut operands = operands.iter().map(normalization_key).collect::<Vec<_>>();
            operands.sort();
            canonical_node("Or", &operands)
        }
        BooleanExpression::Not(operand) => canonical_node("Not", &[normalization_key(operand)]),
        BooleanExpression::Custom {
            payload,
            description,
        } => canonical_node(
            "BooleanCustom",
            &[
                string_literal_key(payload),
                description_key(description.as_ref()),
            ],
        ),
    }
}

fn normalization_scalar_key(expression: &ScalarExpression<ExpressionValue>) -> String {
    match expression {
        ScalarExpression::Constant(value) => {
            canonical_node("ScalarConstant", &[expression_value_key(value)])
        }
        ScalarExpression::Reference(path) => {
            canonical_node("ScalarReference", &[path.value().to_string()])
        }
        ScalarExpression::SymbolReference(symbol) => {
            canonical_node("ScalarSymbolReference", &[symbol_identity_key(symbol)])
        }
        ScalarExpression::Unary { operator, operand } => canonical_node(
            "ScalarUnary",
            &[format!("{operator:?}"), normalization_scalar_key(operand)],
        ),
        ScalarExpression::Binary {
            operator,
            left,
            right,
        } => canonical_node(
            "ScalarBinary",
            &[
                format!("{operator:?}"),
                normalization_scalar_key(left),
                normalization_scalar_key(right),
            ],
        ),
        ScalarExpression::Function { name, arguments } => canonical_node(
            "ScalarFunction",
            &[
                name.clone(),
                canonical_node(
                    "Arguments",
                    &arguments
                        .iter()
                        .map(normalization_scalar_key)
                        .collect::<Vec<_>>(),
                ),
            ],
        ),
        ScalarExpression::Custom {
            payload,
            description,
        } => canonical_node(
            "ScalarCustom",
            &[
                string_literal_key(payload),
                description_key(description.as_ref()),
            ],
        ),
        ScalarExpression::Conditional {
            condition,
            then_branch,
            else_branch,
        } => canonical_node(
            "ScalarConditional",
            &[
                normalization_key(condition),
                normalization_scalar_key(then_branch),
                normalization_scalar_key(else_branch),
            ],
        ),
        ScalarExpression::Boolean(expression) => {
            canonical_node("ScalarBoolean", &[normalization_key(expression)])
        }
    }
}

fn canonical_query_key(expression: &BooleanExpression<ExpressionValue>) -> String {
    match expression {
        BooleanExpression::Constant(value) => {
            canonical_node("BooleanConstant", &[format!("{value:?}")])
        }
        BooleanExpression::Comparison {
            operator,
            left,
            right,
        } => canonical_node(
            "Comparison",
            &[
                format!("{operator:?}"),
                canonical_query_scalar_key(left),
                canonical_query_scalar_key(right),
            ],
        ),
        BooleanExpression::In {
            value,
            candidates,
            negated,
        } => {
            let mut candidates = candidates
                .iter()
                .map(canonical_query_scalar_key)
                .collect::<Vec<_>>();
            candidates.sort();
            canonical_node(
                "In",
                &[
                    negated.to_string(),
                    canonical_query_scalar_key(value),
                    canonical_node("Candidates", &candidates),
                ],
            )
        }
        BooleanExpression::PatternMatch {
            value,
            pattern,
            mode,
            negated,
        } => canonical_node(
            "PatternMatch",
            &[
                format!("{mode:?}"),
                negated.to_string(),
                canonical_query_scalar_key(value),
                canonical_query_scalar_key(pattern),
            ],
        ),
        BooleanExpression::NullCheck {
            path,
            null_check_type,
        } => canonical_node(
            "NullCheck",
            &[format!("{null_check_type:?}"), path.value().to_string()],
        ),
        BooleanExpression::And(operands) => {
            let mut operands = operands.iter().map(canonical_query_key).collect::<Vec<_>>();
            operands.sort();
            canonical_node("And", &operands)
        }
        BooleanExpression::Or(operands) => {
            let mut operands = operands.iter().map(canonical_query_key).collect::<Vec<_>>();
            operands.sort();
            canonical_node("Or", &operands)
        }
        BooleanExpression::Not(operand) => canonical_node("Not", &[canonical_query_key(operand)]),
        BooleanExpression::Custom {
            payload: _,
            description,
        } => canonical_node(
            "BooleanCustom",
            &[string_shape_key(), description_key(description.as_ref())],
        ),
    }
}

fn canonical_query_scalar_key(expression: &ScalarExpression<ExpressionValue>) -> String {
    match expression {
        ScalarExpression::Constant(value) => {
            canonical_node("ScalarConstant", &[expression_value_shape_key(value)])
        }
        ScalarExpression::Reference(path) => {
            canonical_node("ScalarReference", &[path.value().to_string()])
        }
        ScalarExpression::SymbolReference(symbol) => {
            canonical_node("ScalarSymbolReference", &[symbol_identity_key(symbol)])
        }
        ScalarExpression::Unary { operator, operand } => canonical_node(
            "ScalarUnary",
            &[format!("{operator:?}"), canonical_query_scalar_key(operand)],
        ),
        ScalarExpression::Binary {
            operator,
            left,
            right,
        } => canonical_node(
            "ScalarBinary",
            &[
                format!("{operator:?}"),
                canonical_query_scalar_key(left),
                canonical_query_scalar_key(right),
            ],
        ),
        ScalarExpression::Function { name, arguments } => canonical_node(
            "ScalarFunction",
            &[
                name.clone(),
                canonical_node(
                    "Arguments",
                    &arguments
                        .iter()
                        .map(canonical_query_scalar_key)
                        .collect::<Vec<_>>(),
                ),
            ],
        ),
        ScalarExpression::Custom {
            payload: _,
            description,
        } => canonical_node(
            "ScalarCustom",
            &[string_shape_key(), description_key(description.as_ref())],
        ),
        ScalarExpression::Conditional {
            condition,
            then_branch,
            else_branch,
        } => canonical_node(
            "ScalarConditional",
            &[
                canonical_query_key(condition),
                canonical_query_scalar_key(then_branch),
                canonical_query_scalar_key(else_branch),
            ],
        ),
        ScalarExpression::Boolean(expression) => {
            canonical_node("ScalarBoolean", &[canonical_query_key(expression)])
        }
    }
}

fn canonical_node(tag: &str, parts: &[String]) -> String {
    let mut key = String::new();
    append_token(&mut key, tag);
    key.push_str(&parts.len().to_string());
    for part in parts {
        append_token(&mut key, part);
    }
    key
}

fn expression_value_key(value: &ExpressionValue) -> String {
    match value {
        ExpressionValue::Null => canonical_node("null", &[]),
        ExpressionValue::Boolean(value) => canonical_node("boolean", &[value.to_string()]),
        ExpressionValue::Number(value) => canonical_node("number", &[value.to_string()]),
        ExpressionValue::String(value) => canonical_node("string", &[value.clone()]),
    }
}

fn expression_value_shape_key(value: &ExpressionValue) -> String {
    match value {
        ExpressionValue::Null => canonical_node("null", &[]),
        ExpressionValue::Boolean(_) => canonical_node("boolean", &[]),
        ExpressionValue::Number(_) => canonical_node("number", &[]),
        ExpressionValue::String(_) => canonical_node("string", &[]),
    }
}

fn string_literal_key(value: &str) -> String {
    canonical_node("string", &[value.to_string()])
}

fn string_shape_key() -> String {
    canonical_node("string", &[])
}

fn description_key(description: Option<&String>) -> String {
    description
        .map(|value| canonical_node("description", &[value.clone()]))
        .unwrap_or_else(|| canonical_node("description", &[]))
}

fn symbol_identity_key(symbol: &OwnedSymbol) -> String {
    let id = symbol.dyn_id();
    canonical_node(
        "SymbolIdentity",
        &[
            canonical_node("parentType", &[id.parent_type.to_string()]),
            id.parent_id.to_string(),
            id.index.to_string(),
        ],
    )
}

fn snapshot_boolean_expression(
    expression: &BooleanExpression<ExpressionValue>,
) -> BooleanExpression<ExpressionValue> {
    match expression {
        BooleanExpression::Constant(value) => BooleanExpression::Constant(value.clone()),
        BooleanExpression::Comparison {
            operator,
            left,
            right,
        } => BooleanExpression::Comparison {
            operator: *operator,
            left: snapshot_scalar_expression(left),
            right: snapshot_scalar_expression(right),
        },
        BooleanExpression::In {
            value,
            candidates,
            negated,
        } => BooleanExpression::In {
            value: snapshot_scalar_expression(value),
            candidates: candidates.iter().map(snapshot_scalar_expression).collect(),
            negated: *negated,
        },
        BooleanExpression::PatternMatch {
            value,
            pattern,
            mode,
            negated,
        } => BooleanExpression::PatternMatch {
            value: snapshot_scalar_expression(value),
            pattern: snapshot_scalar_expression(pattern),
            mode: *mode,
            negated: *negated,
        },
        BooleanExpression::NullCheck {
            path,
            null_check_type,
        } => BooleanExpression::NullCheck {
            path: path.clone(),
            null_check_type: *null_check_type,
        },
        BooleanExpression::And(operands) => {
            BooleanExpression::And(operands.iter().map(snapshot_boolean_expression).collect())
        }
        BooleanExpression::Or(operands) => {
            BooleanExpression::Or(operands.iter().map(snapshot_boolean_expression).collect())
        }
        BooleanExpression::Not(operand) => {
            BooleanExpression::Not(Box::new(snapshot_boolean_expression(operand)))
        }
        BooleanExpression::Custom {
            payload,
            description,
        } => BooleanExpression::Custom {
            payload: payload.clone(),
            description: description.clone(),
        },
    }
}

fn snapshot_scalar_expression(
    expression: &ScalarExpression<ExpressionValue>,
) -> ScalarExpression<ExpressionValue> {
    match expression {
        ScalarExpression::Constant(value) => ScalarExpression::Constant(value.clone()),
        ScalarExpression::Reference(path) => ScalarExpression::Reference(path.clone()),
        ScalarExpression::SymbolReference(symbol) => {
            ScalarExpression::SymbolReference(snapshot_symbol(symbol))
        }
        ScalarExpression::Unary { operator, operand } => ScalarExpression::Unary {
            operator: *operator,
            operand: Box::new(snapshot_scalar_expression(operand)),
        },
        ScalarExpression::Binary {
            operator,
            left,
            right,
        } => ScalarExpression::Binary {
            operator: *operator,
            left: Box::new(snapshot_scalar_expression(left)),
            right: Box::new(snapshot_scalar_expression(right)),
        },
        ScalarExpression::Function { name, arguments } => ScalarExpression::Function {
            name: name.clone(),
            arguments: arguments.iter().map(snapshot_scalar_expression).collect(),
        },
        ScalarExpression::Custom {
            payload,
            description,
        } => ScalarExpression::Custom {
            payload: payload.clone(),
            description: description.clone(),
        },
        ScalarExpression::Conditional {
            condition,
            then_branch,
            else_branch,
        } => ScalarExpression::Conditional {
            condition: Box::new(snapshot_boolean_expression(condition)),
            then_branch: Box::new(snapshot_scalar_expression(then_branch)),
            else_branch: Box::new(snapshot_scalar_expression(else_branch)),
        },
        ScalarExpression::Boolean(expression) => {
            ScalarExpression::Boolean(Box::new(snapshot_boolean_expression(expression)))
        }
    }
}

fn snapshot_symbol(symbol: &OwnedSymbol) -> OwnedSymbol {
    if let Some(path) = property_path_from_owned_symbol(symbol) {
        return path_owned_symbol(path.clone());
    }

    let id = symbol.dyn_id();
    OwnedSymbol::new(SnapshotSymbol {
        parent_type: id.parent_type.to_string(),
        parent_id: id.parent_id,
        index: id.index,
        name: symbol.name().to_string(),
        display_name: symbol.display_name().to_string(),
    })
}

#[derive(Debug, Clone)]
struct SnapshotSymbol {
    parent_type: String,
    parent_id: usize,
    index: usize,
    name: String,
    display_name: String,
}

impl std::fmt::Display for SnapshotSymbol {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.display_name)
    }
}

impl DynSymbol for SnapshotSymbol {
    fn name(&self) -> &str {
        &self.name
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    fn dyn_id(&self) -> SymbolDynId<'_> {
        SymbolDynId::component(&self.parent_type, self.parent_id, self.index)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn contains_boolean_constant(expression: &BooleanExpression<ExpressionValue>) -> bool {
    match expression {
        BooleanExpression::Constant(_) => true,
        BooleanExpression::Comparison { left, right, .. } => {
            contains_boolean_constant_scalar(left) || contains_boolean_constant_scalar(right)
        }
        BooleanExpression::In {
            value, candidates, ..
        } => {
            contains_boolean_constant_scalar(value)
                || candidates.iter().any(contains_boolean_constant_scalar)
        }
        BooleanExpression::PatternMatch { value, pattern, .. } => {
            contains_boolean_constant_scalar(value) || contains_boolean_constant_scalar(pattern)
        }
        BooleanExpression::NullCheck { .. } => false,
        BooleanExpression::And(operands) | BooleanExpression::Or(operands) => {
            operands.iter().any(contains_boolean_constant)
        }
        BooleanExpression::Not(operand) => contains_boolean_constant(operand),
        BooleanExpression::Custom { .. } => false,
    }
}

fn contains_boolean_constant_scalar(expression: &ScalarExpression<ExpressionValue>) -> bool {
    match expression {
        ScalarExpression::Constant(_)
        | ScalarExpression::Reference(_)
        | ScalarExpression::SymbolReference(_)
        | ScalarExpression::Custom { .. } => false,
        ScalarExpression::Unary { operand, .. } => contains_boolean_constant_scalar(operand),
        ScalarExpression::Binary { left, right, .. } => {
            contains_boolean_constant_scalar(left) || contains_boolean_constant_scalar(right)
        }
        ScalarExpression::Function { arguments, .. } => {
            arguments.iter().any(contains_boolean_constant_scalar)
        }
        ScalarExpression::Conditional {
            condition,
            then_branch,
            else_branch,
        } => {
            contains_boolean_constant(condition)
                || contains_boolean_constant_scalar(then_branch)
                || contains_boolean_constant_scalar(else_branch)
        }
        ScalarExpression::Boolean(_) => true,
    }
}

/// 判断表达式是否包含精确的列对列关联。
/// Check whether an expression contains an exact column-to-column correlation.
pub fn contains_exact_column_correlation(
    expression: &BooleanExpression<ExpressionValue>,
    target_sources: &HashSet<String>,
    bound_sources: &HashSet<String>,
) -> bool {
    match expression {
        BooleanExpression::Comparison { left, right, .. } => {
            let (ScalarExpression::Reference(left), ScalarExpression::Reference(right)) =
                (left, right)
            else {
                return false;
            };
            let Some(left_qualifier) = source_qualifier(left.value()) else {
                return false;
            };
            let Some(right_qualifier) = source_qualifier(right.value()) else {
                return false;
            };
            (target_sources.contains(left_qualifier) && bound_sources.contains(right_qualifier))
                || (target_sources.contains(right_qualifier)
                    && bound_sources.contains(left_qualifier))
        }
        BooleanExpression::And(operands) => operands.iter().any(|operand| {
            contains_exact_column_correlation(operand, target_sources, bound_sources)
        }),
        BooleanExpression::Or(operands) => {
            !operands.is_empty()
                && operands.iter().all(|operand| {
                    contains_exact_column_correlation(operand, target_sources, bound_sources)
                })
        }
        BooleanExpression::Not(operand) => {
            contains_exact_column_correlation(operand, target_sources, bound_sources)
        }
        BooleanExpression::Constant(_)
        | BooleanExpression::In { .. }
        | BooleanExpression::PatternMatch { .. }
        | BooleanExpression::NullCheck { .. }
        | BooleanExpression::Custom { .. } => false,
    }
}

fn append_token(target: &mut String, value: &str) {
    target.push_str(&value.encode_utf16().count().to_string());
    target.push(':');
    target.push_str(value);
}

fn append_source(target: &mut String, source: &QuerySource) {
    append_token(target, &source.name);
    target.push(':');
    append_token(target, source.alias.as_deref().unwrap_or_default());
}

fn append_column(target: &mut String, column: &ColumnRef) {
    append_token(target, &column.source);
    target.push(':');
    append_token(target, &column.path);
}

fn append_columns(target: &mut String, columns: &[ColumnRef]) {
    target.push_str(&columns.len().to_string());
    for column in columns {
        target.push(';');
        append_column(target, column);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    fn join_condition() -> BooleanExpression<ExpressionValue> {
        BooleanExpression::eq(
            ScalarExpression::reference("o.id"),
            ScalarExpression::reference("i.order_id"),
        )
    }

    #[test]
    fn rejects_duplicate_sources_and_invalid_exists_joins() {
        let duplicate =
            RelationalQueryPlan::new(QuerySource::new("orders")).with_joins([JoinSpec::new(
                JoinType::Inner,
                QuerySource::new("orders").with_alias("o2"),
                join_condition(),
                JoinCardinality::OneToOne,
            )]);
        assert!(duplicate.validate().is_err());

        let invalid_exists = RelationalQueryPlan::new(QuerySource::new("orders").with_alias("o"))
            .with_joins([JoinSpec::new(
                JoinType::Exists,
                QuerySource::new("items").with_alias("i"),
                join_condition(),
                JoinCardinality::ManyToOne,
            )]);
        assert!(invalid_exists.validate().is_err());
    }

    #[test]
    fn rejects_uncorrelated_and_arithmetic_join_conditions() {
        let constant =
            RelationalQueryPlan::new(QuerySource::new("orders")).with_joins([JoinSpec::new(
                JoinType::Inner,
                QuerySource::new("items"),
                BooleanExpression::true_constant(),
                JoinCardinality::OneToMany,
            )]);
        assert!(constant.validate().is_err());

        let arithmetic = RelationalQueryPlan::new(QuerySource::new("orders").with_alias("o"))
            .with_joins([JoinSpec::new(
                JoinType::Inner,
                QuerySource::new("items").with_alias("i"),
                BooleanExpression::eq(
                    ScalarExpression::reference("o.id"),
                    ScalarExpression::binary(
                        ospf_rust_math::symbol::BinaryOperator::Add,
                        ScalarExpression::reference("i.order_id"),
                        ScalarExpression::constant(ExpressionValue::from(1)),
                    ),
                ),
                JoinCardinality::OneToMany,
            )]);
        assert!(arithmetic.validate().is_err());
    }

    #[test]
    fn rejects_boolean_scalar_join_conditions() {
        let condition = BooleanExpression::eq(
            ScalarExpression::boolean_expr(BooleanExpression::eq(
                ScalarExpression::reference("o.id"),
                ScalarExpression::reference("i.order_id"),
            )),
            ScalarExpression::constant(ExpressionValue::from(true)),
        );
        let plan =
            RelationalQueryPlan::new(QuerySource::new("orders").with_alias("o")).with_joins([
                JoinSpec::new(
                    JoinType::Inner,
                    QuerySource::new("items").with_alias("i"),
                    condition,
                    JoinCardinality::OneToMany,
                ),
            ]);

        assert!(plan.validate().is_err());
    }

    #[test]
    fn defensively_copies_plan_inputs_and_produces_sha256_digest() {
        let mut joins = Vec::new();
        let plan = RelationalQueryPlan::new(QuerySource::new(" orders ")).with_joins(joins.clone());
        joins.push(JoinSpec::new(
            JoinType::Inner,
            QuerySource::new("items"),
            join_condition(),
            JoinCardinality::OneToMany,
        ));

        assert!(plan.joins().is_empty());
        assert_eq!(plan.canonical_hash().len(), 64);
        assert_eq!(plan.canonical_hash(), plan.clone().canonical_hash());
        assert_eq!(plan.root().name, "orders");
    }

    #[test]
    fn canonicalization_sorts_boolean_operands() {
        let first = BooleanExpression::eq(
            ScalarExpression::reference("o.a"),
            ScalarExpression::constant(ExpressionValue::from(1)),
        );
        let second = BooleanExpression::eq(
            ScalarExpression::reference("o.b"),
            ScalarExpression::constant(ExpressionValue::from(2)),
        );
        let left = RelationalQueryPlan::new(QuerySource::new("orders"))
            .with_predicate(BooleanExpression::and(vec![first.clone(), second.clone()]));
        let right = RelationalQueryPlan::new(QuerySource::new("orders"))
            .with_predicate(BooleanExpression::and(vec![second, first]));

        assert_eq!(left.canonical(), right.canonical());
    }

    #[test]
    fn canonicalization_uses_literal_shapes_and_nested_boolean_normalization() {
        fn plan_with_literal(value: f64) -> RelationalQueryPlan {
            RelationalQueryPlan::new(QuerySource::new("orders")).with_predicate(
                BooleanExpression::eq(
                    ScalarExpression::reference("orders.id"),
                    ScalarExpression::constant(ExpressionValue::from(value)),
                ),
            )
        }

        let one = plan_with_literal(1.0);
        let two = plan_with_literal(2.0);
        assert_eq!(one.canonical(), two.canonical());
        assert_eq!(one.canonical_hash(), two.canonical_hash());

        fn comparison(path: &str, value: f64) -> BooleanExpression<ExpressionValue> {
            BooleanExpression::eq(
                ScalarExpression::reference(path),
                ScalarExpression::constant(ExpressionValue::from(value)),
            )
        }

        let first = comparison("orders.id", 1.0);
        let second = comparison("orders.status", 2.0);
        let third = comparison("orders.version", 3.0);
        let nested_condition = BooleanExpression::and(vec![
            BooleanExpression::and(vec![first.clone(), second.clone()]),
            third.clone(),
        ]);
        let flat_condition = BooleanExpression::and(vec![third, second, first]);
        let nested = RelationalQueryPlan::new(QuerySource::new("orders")).with_predicate(
            BooleanExpression::eq(
                ScalarExpression::reference("orders.id"),
                ScalarExpression::conditional(
                    nested_condition,
                    ScalarExpression::constant(ExpressionValue::from(1.0)),
                    ScalarExpression::constant(ExpressionValue::from(2.0)),
                ),
            ),
        );
        let flat = RelationalQueryPlan::new(QuerySource::new("orders")).with_predicate(
            BooleanExpression::eq(
                ScalarExpression::reference("orders.id"),
                ScalarExpression::conditional(
                    flat_condition,
                    ScalarExpression::constant(ExpressionValue::from(3.0)),
                    ScalarExpression::constant(ExpressionValue::from(4.0)),
                ),
            ),
        );
        assert_eq!(nested.canonical(), flat.canonical());
        assert_eq!(nested.canonical_hash(), flat.canonical_hash());
    }

    #[test]
    fn canonicalization_sorts_membership_candidates_and_omits_custom_text() {
        fn membership(candidates: Vec<ScalarExpression<ExpressionValue>>) -> RelationalQueryPlan {
            RelationalQueryPlan::new(QuerySource::new("orders")).with_predicate(
                BooleanExpression::in_expr(
                    ScalarExpression::reference("orders.status"),
                    candidates,
                    false,
                ),
            )
        }

        let first = membership(vec![
            ScalarExpression::constant(ExpressionValue::from(1.0)),
            ScalarExpression::constant(ExpressionValue::from("active")),
        ]);
        let second = membership(vec![
            ScalarExpression::constant(ExpressionValue::from("active")),
            ScalarExpression::constant(ExpressionValue::from(2.0)),
        ]);
        assert_eq!(first.canonical(), second.canonical());
        assert_eq!(first.canonical_hash(), second.canonical_hash());

        let different_shape = membership(vec![
            ScalarExpression::constant(ExpressionValue::from(true)),
            ScalarExpression::constant(ExpressionValue::from("active")),
        ]);
        assert_ne!(first.canonical(), different_shape.canonical());

        let custom_a = RelationalQueryPlan::new(QuerySource::new("orders")).with_predicate(
            BooleanExpression::Custom {
                payload: "first".to_string(),
                description: Some("custom".to_string()),
            },
        );
        let custom_b = RelationalQueryPlan::new(QuerySource::new("orders")).with_predicate(
            BooleanExpression::Custom {
                payload: "second".to_string(),
                description: Some("custom".to_string()),
            },
        );
        assert_eq!(custom_a.canonical(), custom_b.canonical());
        assert_eq!(custom_a.canonical_hash(), custom_b.canonical_hash());
    }

    #[test]
    fn snapshots_mutable_symbol_names_at_plan_boundary() {
        #[derive(Debug, Clone)]
        struct MutableSymbol {
            name: Rc<Cell<&'static str>>,
            id: usize,
        }

        impl std::fmt::Display for MutableSymbol {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.name())
            }
        }

        impl DynSymbol for MutableSymbol {
            fn name(&self) -> &str {
                self.name.get()
            }

            fn display_name(&self) -> &str {
                self.name.get()
            }

            fn dyn_id(&self) -> SymbolDynId<'_> {
                SymbolDynId::standalone(self.id)
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }

        let current_name = Rc::new(Cell::new("orders.total"));
        let symbol = OwnedSymbol::new(MutableSymbol {
            name: current_name.clone(),
            id: 7,
        });
        let plan = RelationalQueryPlan::new(QuerySource::new("orders")).with_predicate(
            BooleanExpression::eq(
                ScalarExpression::symbol_reference(symbol),
                ScalarExpression::constant(ExpressionValue::from(1.0)),
            ),
        );
        let canonical = plan.canonical();
        current_name.set("orders.changed");

        let predicate = plan.predicate().expect("predicate must be set");
        let frozen_symbol = match predicate {
            BooleanExpression::Comparison {
                left: ScalarExpression::SymbolReference(symbol),
                ..
            } => symbol,
            _ => panic!("expected symbol reference"),
        };
        assert_eq!(frozen_symbol.name(), "orders.total");
        assert_eq!(canonical, plan.canonical());
    }

    #[test]
    fn canonical_token_lengths_match_utf16_code_units() {
        let plan = RelationalQueryPlan::new(QuerySource::new("😀"));

        assert!(plan.canonical().starts_with("root=2:😀"));
    }
}
