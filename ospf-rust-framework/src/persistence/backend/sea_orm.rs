//! SeaORM 持久化后端
//! SeaORM persistence backend

use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use ospf_rust_math::Trivalent;
use ospf_rust_math::symbol::{

    BinaryOperator, BooleanExpression, ComparisonOperator, ExpressionValue, NullCheckType,
    PatternMatchMode, ScalarExpression, ScalarFunctionNames, UnaryOperator,
    property_path_from_owned_symbol,
};
use sea_orm::sea_query::{Alias, Condition, Expr, ExprTrait, Func, Order, SimpleExpr, Value};
use sea_orm::{
    ConnectionTrait, DbErr, Delete, EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Select, Update,
};

use crate::persistence::{
    NullsOrder, PersistenceFieldResolver, RepositoryQuery, SetFromExpression, SetNull, SetValue,
    SortBy, SortDirection, SortItem, UnsupportedPredicatePolicy, UpdateAssignment,
    UpdateAssignments,
};

/// SeaORM 后端标记类型。
/// SeaORM backend marker type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SeaOrmBackend;

/// SeaORM translator 配置。
/// SeaORM translator configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeaOrmTranslatorConfig {
    pub unsupported_predicate_policy: UnsupportedPredicatePolicy,
    pub emulate_nulls_order: bool,
}

impl Default for SeaOrmTranslatorConfig {
    fn default() -> Self {
        Self {
            unsupported_predicate_policy: UnsupportedPredicatePolicy::default(),
            emulate_nulls_order: true,
        }
    }
}

impl SeaOrmTranslatorConfig {
    /// 设置不支持谓词策略。
    /// Set unsupported predicate policy.
    pub fn with_unsupported_predicate_policy(
        mut self,
        unsupported_predicate_policy: UnsupportedPredicatePolicy,
    ) -> Self {
        self.unsupported_predicate_policy = unsupported_predicate_policy;
        self
    }

    /// 设置是否用 CASE 表达式模拟 NULLS FIRST/LAST。
    /// Set whether NULLS FIRST/LAST should be emulated with CASE expressions.
    pub fn with_emulate_nulls_order(mut self, emulate_nulls_order: bool) -> Self {
        self.emulate_nulls_order = emulate_nulls_order;
        self
    }
}

/// SeaORM 翻译错误。
/// SeaORM translation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeaOrmTranslationError {
    UnsupportedPredicate(String),
    UnresolvedField(String),
    InvalidExpression(String),
}

impl Display for SeaOrmTranslationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPredicate(message) => write!(f, "unsupported predicate: {message}"),
            Self::UnresolvedField(message) => write!(f, "unresolved field: {message}"),
            Self::InvalidExpression(message) => write!(f, "invalid expression: {message}"),
        }
    }
}

impl std::error::Error for SeaOrmTranslationError {}

/// SeaORM 仓储错误。
/// SeaORM repository error.
#[derive(Debug)]
pub enum SeaOrmRepositoryError {
    Translation(SeaOrmTranslationError),
    Database(DbErr),
}

impl Display for SeaOrmRepositoryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Translation(error) => write!(f, "{error}"),
            Self::Database(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SeaOrmRepositoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Translation(error) => Some(error),
            Self::Database(error) => Some(error),
        }
    }
}

impl From<SeaOrmTranslationError> for SeaOrmRepositoryError {
    fn from(value: SeaOrmTranslationError) -> Self {
        Self::Translation(value)
    }
}

impl From<DbErr> for SeaOrmRepositoryError {
    fn from(value: DbErr) -> Self {
        Self::Database(value)
    }
}

/// SeaORM 排序表达式。
/// SeaORM order expression.
#[derive(Debug, Clone)]
pub struct SeaOrmOrderBy {
    pub expression: SimpleExpr,
    pub order: Order,
}

/// SeaORM 更新赋值表达式。
/// SeaORM update assignment expression.
#[derive(Debug, Clone)]
pub struct SeaOrmUpdateAssignment {
    pub column: String,
    pub value: SimpleExpr,
}

/// SeaORM 表达式翻译器。
/// SeaORM expression translator.
#[derive(Debug, Clone)]
pub struct SeaOrmTranslator<R> {
    resolver: R,
    config: SeaOrmTranslatorConfig,
}

impl<R> SeaOrmTranslator<R> {
    /// 创建翻译器。
    /// Create a translator.
    pub fn new(resolver: R) -> Self {
        Self {
            resolver,
            config: SeaOrmTranslatorConfig::default(),
        }
    }

    /// 使用配置创建翻译器。
    /// Create a translator with configuration.
    pub fn with_config(resolver: R, config: SeaOrmTranslatorConfig) -> Self {
        Self { resolver, config }
    }

    /// 获取配置。
    /// Get configuration.
    pub fn config(&self) -> SeaOrmTranslatorConfig {
        self.config
    }

    /// 设置配置。
    /// Set configuration.
    pub fn set_config(&mut self, config: SeaOrmTranslatorConfig) {
        self.config = config;
    }
}

/// SeaORM 异步表达式仓储适配器。
/// SeaORM async expression repository adapter.
#[derive(Debug, Clone)]
pub struct SeaOrmRepository<E, R, C> {
    db: C,
    translator: SeaOrmTranslator<R>,
    entity: PhantomData<E>,
}

impl<E, R, C> SeaOrmRepository<E, R, C> {
    /// 使用 resolver 创建仓储。
    /// Create a repository with a resolver.
    pub fn new(db: C, resolver: R) -> Self {
        Self {
            db,
            translator: SeaOrmTranslator::new(resolver),
            entity: PhantomData,
        }
    }

    /// 使用翻译器创建仓储。
    /// Create a repository with a translator.
    pub fn with_translator(db: C, translator: SeaOrmTranslator<R>) -> Self {
        Self {
            db,
            translator,
            entity: PhantomData,
        }
    }

    /// 获取连接。
    /// Get the connection.
    pub fn db(&self) -> &C {
        &self.db
    }

    /// 获取翻译器。
    /// Get the translator.
    pub fn translator(&self) -> &SeaOrmTranslator<R> {
        &self.translator
    }

    /// 获取可变翻译器。
    /// Get the mutable translator.
    pub fn translator_mut(&mut self) -> &mut SeaOrmTranslator<R> {
        &mut self.translator
    }

    /// 拆出连接和翻译器。
    /// Split into the connection and translator.
    pub fn into_parts(self) -> (C, SeaOrmTranslator<R>) {
        (self.db, self.translator)
    }
}

impl<E, R, C> SeaOrmRepository<E, R, C>
where
    E: EntityTrait,
    E::Model: FromQueryResult + Send + Sync + 'static,
    R: PersistenceFieldResolver<String>,
    C: ConnectionTrait,
{
    /// 查询实体。
    /// Find entities.
    pub async fn find(
        &self,
        where_expr: &BooleanExpression<ExpressionValue>,
    ) -> Result<Vec<E::Model>, SeaOrmRepositoryError> {
        self.find_with_options(where_expr, &RepositoryQuery::default())
            .await
    }

    /// 查询实体，带排序和分页选项。
    /// Find entities with sort and pagination options.
    pub async fn find_with_options(
        &self,
        where_expr: &BooleanExpression<ExpressionValue>,
        options: &RepositoryQuery,
    ) -> Result<Vec<E::Model>, SeaOrmRepositoryError> {
        let condition = self.translator.translate_boolean(where_expr)?;
        let select = self.apply_query_options(E::find().filter(condition), options)?;
        Ok(select.all(&self.db).await?)
    }

    /// 计数。
    /// Count matching entities.
    pub async fn count(
        &self,
        where_expr: &BooleanExpression<ExpressionValue>,
    ) -> Result<u64, SeaOrmRepositoryError> {
        let condition = self.translator.translate_boolean(where_expr)?;
        Ok(E::find()
            .filter(condition)
            .paginate(&self.db, 1)
            .num_items()
            .await?)
    }

    /// 更新。
    /// Update matching entities.
    pub async fn update(
        &self,
        where_expr: &BooleanExpression<ExpressionValue>,
        assignments: &UpdateAssignments<ExpressionValue>,
    ) -> Result<u64, SeaOrmRepositoryError> {
        if assignments.is_empty() {
            return Ok(0);
        }
        let condition = self.translator.translate_boolean(where_expr)?;
        let assignments = self.translator.translate_update_assignments(assignments)?;
        let mut update = Update::many(E::default()).filter(condition);
        for assignment in assignments {
            update = update.col_expr(Alias::new(assignment.column), assignment.value);
        }
        Ok(update.exec(&self.db).await?.rows_affected)
    }

    /// 删除。
    /// Delete matching entities.
    pub async fn delete(
        &self,
        where_expr: &BooleanExpression<ExpressionValue>,
    ) -> Result<u64, SeaOrmRepositoryError> {
        let condition = self.translator.translate_boolean(where_expr)?;
        Ok(Delete::many(E::default())
            .filter(condition)
            .exec(&self.db)
            .await?
            .rows_affected)
    }

    /// 检查是否存在。
    /// Check whether any matching entity exists.
    pub async fn exists(
        &self,
        where_expr: &BooleanExpression<ExpressionValue>,
    ) -> Result<bool, SeaOrmRepositoryError> {
        Ok(self.count(where_expr).await? > 0)
    }

    fn apply_query_options(
        &self,
        mut select: Select<E>,
        options: &RepositoryQuery,
    ) -> Result<Select<E>, SeaOrmRepositoryError> {
        if let Some(sort_by) = &options.sort_by {
            for order in self.translator.translate_sort_by(sort_by)? {
                select = select.order_by(order.expression, order.order);
            }
        }
        if let Some(limit) = options.limit {
            select = select.limit(limit as u64);
        }
        if let Some(offset) = options.offset {
            select = select.offset(offset as u64);
        }
        Ok(select)
    }
}

impl<R> SeaOrmTranslator<R>
where
    R: PersistenceFieldResolver<String>,
{
    /// 翻译标量表达式。
    /// Translate a scalar expression.
    pub fn translate_scalar(
        &self,
        expression: &ScalarExpression<ExpressionValue>,
    ) -> Result<SimpleExpr, SeaOrmTranslationError> {
        self.translate_scalar_expr(expression)
    }

    /// 翻译布尔表达式。
    /// Translate a boolean expression.
    pub fn translate_boolean(
        &self,
        expression: &BooleanExpression<ExpressionValue>,
    ) -> Result<Condition, SeaOrmTranslationError> {
        self.translate_condition(expression)
    }

    /// 翻译排序。
    /// Translate sorting.
    pub fn translate_sort_by(
        &self,
        sort_by: &SortBy,
    ) -> Result<Vec<SeaOrmOrderBy>, SeaOrmTranslationError> {
        let mut orders = Vec::new();
        for item in &sort_by.items {
            orders.extend(self.translate_sort_item(item)?);
        }
        Ok(orders)
    }

    /// 翻译更新赋值。
    /// Translate update assignments.
    pub fn translate_update_assignments(
        &self,
        assignments: &UpdateAssignments<ExpressionValue>,
    ) -> Result<Vec<SeaOrmUpdateAssignment>, SeaOrmTranslationError> {
        assignments
            .items
            .iter()
            .map(|assignment| self.translate_update_assignment(assignment))
            .collect()
    }

    fn resolve_column(
        &self,
        path: &ospf_rust_math::symbol::PropertyPath,
    ) -> Result<SimpleExpr, SeaOrmTranslationError> {
        Ok(Expr::col(column_ref(&self.resolve_field_name(path)?)).into())
    }

    fn resolve_field_name(
        &self,
        path: &ospf_rust_math::symbol::PropertyPath,
    ) -> Result<String, SeaOrmTranslationError> {
        let field = self
            .resolver
            .resolve_field(path)
            .ok_or_else(|| SeaOrmTranslationError::UnresolvedField(path.value().to_string()))?;
        Ok(field)
    }

    fn translate_scalar_expr(
        &self,
        expression: &ScalarExpression<ExpressionValue>,
    ) -> Result<SimpleExpr, SeaOrmTranslationError> {
        match expression {
            ScalarExpression::Constant(value) => {
                Ok(Expr::val(expression_value_to_value(value)).into())
            }
            ScalarExpression::Reference(path) => self.resolve_column(path),
            ScalarExpression::SymbolReference(symbol) => {
                let path = property_path_from_owned_symbol(symbol).ok_or_else(|| {
                    SeaOrmTranslationError::UnsupportedPredicate(format!(
                        "symbol reference '{}' is not a property path",
                        symbol.display_name()
                    ))
                })?;
                self.resolve_column(path)
            }
            ScalarExpression::Unary { operator, operand } => {
                let operand = self.translate_scalar_expr(operand)?;
                match operator {
                    UnaryOperator::Negate => Ok(Expr::expr(operand).mul(-1).into()),
                    UnaryOperator::Positive => Ok(operand),
                    UnaryOperator::Abs => Ok(Func::abs(operand).into()),
                }
            }
            ScalarExpression::Binary {
                operator,
                left,
                right,
            } => {
                let left = self.translate_scalar_expr(left)?;
                let right = self.translate_scalar_expr(right)?;
                match operator {
                    BinaryOperator::Add => Ok(Expr::expr(left).add(right).into()),
                    BinaryOperator::Subtract => Ok(Expr::expr(left).sub(right).into()),
                    BinaryOperator::Multiply => Ok(Expr::expr(left).mul(right).into()),
                    BinaryOperator::Divide => Ok(Expr::expr(left).div(right).into()),
                    BinaryOperator::Modulo => Ok(Expr::expr(left).modulo(right).into()),
                    BinaryOperator::Power => Err(SeaOrmTranslationError::UnsupportedPredicate(
                        "POWER scalar expression is not supported by SeaORM translator".to_string(),
                    )),
                }
            }
            ScalarExpression::Function { name, arguments } => {
                self.translate_scalar_function(name, arguments)
            }
            ScalarExpression::Custom { description, .. } => {
                Err(SeaOrmTranslationError::UnsupportedPredicate(
                    description
                        .clone()
                        .unwrap_or_else(|| "custom scalar expression is not supported".to_string()),
                ))
            }
        }
    }

    fn translate_scalar_function(
        &self,
        name: &str,
        arguments: &[ScalarExpression<ExpressionValue>],
    ) -> Result<SimpleExpr, SeaOrmTranslationError> {
        let lowered = name.to_ascii_lowercase();
        let args = arguments
            .iter()
            .map(|argument| self.translate_scalar_expr(argument))
            .collect::<Result<Vec<_>, _>>()?;
        match lowered.as_str() {
            ScalarFunctionNames::ABS => self.exact_function("abs", args, 1),
            ScalarFunctionNames::LOWER => self.exact_function("lower", args, 1),
            ScalarFunctionNames::UPPER => self.exact_function("upper", args, 1),
            ScalarFunctionNames::TRIM => self.exact_function("trim", args, 1),
            ScalarFunctionNames::LENGTH => self.exact_function("length", args, 1),
            ScalarFunctionNames::COALESCE => {
                if args.is_empty() {
                    Err(SeaOrmTranslationError::InvalidExpression(
                        "coalesce expects at least one argument".to_string(),
                    ))
                } else {
                    Ok(Func::cust(Alias::new("COALESCE")).args(args).into())
                }
            }
            _ => Err(SeaOrmTranslationError::UnsupportedPredicate(format!(
                "unsupported scalar function: {name}"
            ))),
        }
    }

    fn exact_function(
        &self,
        name: &str,
        args: Vec<SimpleExpr>,
        expected: usize,
    ) -> Result<SimpleExpr, SeaOrmTranslationError> {
        if args.len() != expected {
            return Err(SeaOrmTranslationError::InvalidExpression(format!(
                "{name} expects {expected} argument(s)"
            )));
        }
        Ok(Func::cust(Alias::new(name.to_ascii_uppercase()))
            .args(args)
            .into())
    }

    fn translate_condition(
        &self,
        expression: &BooleanExpression<ExpressionValue>,
    ) -> Result<Condition, SeaOrmTranslationError> {
        match expression {
            BooleanExpression::Constant(value) => Ok(match value {
                Trivalent::True => Condition::all(),
                Trivalent::False | Trivalent::Unknown => Condition::all().add(Expr::cust("1 = 0")),
            }),
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
                let column = self.resolve_column(path)?;
                let expr = match null_check_type {
                    NullCheckType::IsNull => Expr::expr(column).is_null(),
                    NullCheckType::IsNotNull => Expr::expr(column).is_not_null(),
                };
                Ok(Condition::all().add(expr))
            }
            BooleanExpression::And(operands) => self.translate_logical(true, operands),
            BooleanExpression::Or(operands) => self.translate_logical(false, operands),
            BooleanExpression::Not(operand) => {
                Ok(Condition::all().add(self.translate_condition(operand)?.not()))
            }
            BooleanExpression::Custom { description, .. } => self.unsupported_condition(
                description
                    .as_deref()
                    .unwrap_or("custom boolean expression"),
            ),
        }
    }

    fn translate_comparison(
        &self,
        operator: ComparisonOperator,
        left: &ScalarExpression<ExpressionValue>,
        right: &ScalarExpression<ExpressionValue>,
    ) -> Result<Condition, SeaOrmTranslationError> {
        if let Some(condition) = self.translate_null_comparison(operator, left, right)? {
            return Ok(condition);
        }
        let left = self.translate_scalar_expr(left)?;
        let right = self.translate_scalar_expr(right)?;
        let expr = match operator {
            ComparisonOperator::Eq => Expr::expr(left).eq(right),
            ComparisonOperator::Ne => Expr::expr(left).ne(right),
            ComparisonOperator::Lt => Expr::expr(left).lt(right),
            ComparisonOperator::Le => Expr::expr(left).lte(right),
            ComparisonOperator::Gt => Expr::expr(left).gt(right),
            ComparisonOperator::Ge => Expr::expr(left).gte(right),
        };
        Ok(Condition::all().add(expr))
    }

    fn translate_null_comparison(
        &self,
        operator: ComparisonOperator,
        left: &ScalarExpression<ExpressionValue>,
        right: &ScalarExpression<ExpressionValue>,
    ) -> Result<Option<Condition>, SeaOrmTranslationError> {
        let left_null = matches!(left, ScalarExpression::Constant(ExpressionValue::Null));
        let right_null = matches!(right, ScalarExpression::Constant(ExpressionValue::Null));
        if !left_null && !right_null {
            return Ok(None);
        }
        if left_null && right_null {
            return Ok(Some(match operator {
                ComparisonOperator::Eq | ComparisonOperator::Le | ComparisonOperator::Ge => {
                    Condition::all()
                }
                ComparisonOperator::Ne | ComparisonOperator::Lt | ComparisonOperator::Gt => {
                    Condition::all().add(Expr::cust("1 = 0"))
                }
            }));
        }
        let nullable = if left_null { right } else { left };
        let nullable = self.translate_scalar_expr(nullable)?;
        let expr = match operator {
            ComparisonOperator::Eq => Expr::expr(nullable).is_null(),
            ComparisonOperator::Ne => Expr::expr(nullable).is_not_null(),
            _ => {
                return self
                    .unsupported_condition("ordered comparison with NULL is not supported")
                    .map(Some);
            }
        };
        Ok(Some(Condition::all().add(expr)))
    }

    fn translate_in(
        &self,
        value: &ScalarExpression<ExpressionValue>,
        candidates: &[ScalarExpression<ExpressionValue>],
        negated: bool,
    ) -> Result<Condition, SeaOrmTranslationError> {
        if candidates.is_empty() {
            return self.unsupported_condition("IN candidates must not be empty");
        }
        let value = self.translate_scalar_expr(value)?;
        let candidates = candidates
            .iter()
            .map(|candidate| self.translate_scalar_expr(candidate))
            .collect::<Result<Vec<_>, _>>()?;
        let expr = if negated {
            Expr::expr(value).is_not_in(candidates)
        } else {
            Expr::expr(value).is_in(candidates)
        };
        Ok(Condition::all().add(expr))
    }

    fn translate_pattern_match(
        &self,
        value: &ScalarExpression<ExpressionValue>,
        pattern: &ScalarExpression<ExpressionValue>,
        mode: PatternMatchMode,
        negated: bool,
    ) -> Result<Condition, SeaOrmTranslationError> {
        if matches!(mode, PatternMatchMode::Regex) {
            return self.unsupported_condition(
                "regex pattern match is not supported by SeaORM translator",
            );
        }
        let value = self.translate_scalar_expr(value)?;
        let pattern = self.translate_pattern(pattern, mode)?;
        let expr = if negated {
            Expr::expr(value).not_like(pattern)
        } else {
            Expr::expr(value).like(pattern)
        };
        Ok(Condition::all().add(expr))
    }

    fn translate_pattern(
        &self,
        pattern: &ScalarExpression<ExpressionValue>,
        mode: PatternMatchMode,
    ) -> Result<String, SeaOrmTranslationError> {
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
                Ok(pattern)
            }
            (PatternMatchMode::Like, _) => self.unsupported_string(
                "non-constant LIKE pattern is not supported by SeaORM translator",
            ),
            (_, _) => self.unsupported_string("non-constant pattern requires LIKE mode"),
        }
    }

    fn translate_logical(
        &self,
        all: bool,
        operands: &[BooleanExpression<ExpressionValue>],
    ) -> Result<Condition, SeaOrmTranslationError> {
        let mut condition = if all {
            Condition::all()
        } else {
            Condition::any()
        };
        for operand in operands {
            condition = condition.add(self.translate_condition(operand)?);
        }
        Ok(condition)
    }

    fn translate_sort_item(
        &self,
        item: &SortItem,
    ) -> Result<Vec<SeaOrmOrderBy>, SeaOrmTranslationError> {
        let column = self.resolve_column(&item.path)?;
        let order = match item.direction {
            SortDirection::Asc => Order::Asc,
            SortDirection::Desc => Order::Desc,
        };
        let mut orders = Vec::new();
        if let Some(nulls) = &item.nulls {
            if self.config.emulate_nulls_order {
                let null_rank = match nulls {
                    NullsOrder::NullsFirst => Expr::case(column.clone().is_null(), 0).finally(1),
                    NullsOrder::NullsLast => Expr::case(column.clone().is_null(), 1).finally(0),
                };
                orders.push(SeaOrmOrderBy {
                    expression: null_rank.into(),
                    order: Order::Asc,
                });
            }
        }
        orders.push(SeaOrmOrderBy {
            expression: column,
            order,
        });
        Ok(orders)
    }

    fn translate_update_assignment(
        &self,
        assignment: &UpdateAssignment<ExpressionValue>,
    ) -> Result<SeaOrmUpdateAssignment, SeaOrmTranslationError> {
        match assignment {
            UpdateAssignment::SetValue(SetValue { path, value }) => Ok(SeaOrmUpdateAssignment {
                column: self.resolve_field_name(path)?,
                value: Expr::val(expression_value_to_value(value)).into(),
            }),
            UpdateAssignment::SetNull(SetNull { path }) => Ok(SeaOrmUpdateAssignment {
                column: self.resolve_field_name(path)?,
                value: Expr::val(Value::String(None)).into(),
            }),
            UpdateAssignment::SetFromExpression(SetFromExpression { path, expression }) => {
                Ok(SeaOrmUpdateAssignment {
                    column: self.resolve_field_name(path)?,
                    value: self.translate_scalar_expr(expression)?,
                })
            }
        }
    }

    fn unsupported_condition(&self, reason: &str) -> Result<Condition, SeaOrmTranslationError> {
        match self.config.unsupported_predicate_policy {
            UnsupportedPredicatePolicy::AlwaysFalse => {
                Ok(Condition::all().add(Expr::cust("1 = 0")))
            }
            UnsupportedPredicatePolicy::FailFast => Err(
                SeaOrmTranslationError::UnsupportedPredicate(reason.to_string()),
            ),
            UnsupportedPredicatePolicy::ClientFilter => {
                Err(SeaOrmTranslationError::UnsupportedPredicate(format!(
                    "ClientFilter is not implemented for SeaORM translator: {reason}"
                )))
            }
        }
    }

    fn unsupported_expr(&self, reason: &str) -> Result<SimpleExpr, SeaOrmTranslationError> {
        match self.config.unsupported_predicate_policy {
            UnsupportedPredicatePolicy::AlwaysFalse => Ok(Expr::cust("1 = 0").into()),
            UnsupportedPredicatePolicy::FailFast => Err(
                SeaOrmTranslationError::UnsupportedPredicate(reason.to_string()),
            ),
            UnsupportedPredicatePolicy::ClientFilter => {
                Err(SeaOrmTranslationError::UnsupportedPredicate(format!(
                    "ClientFilter is not implemented for SeaORM translator: {reason}"
                )))
            }
        }
    }

    fn unsupported_string(&self, reason: &str) -> Result<String, SeaOrmTranslationError> {
        match self.config.unsupported_predicate_policy {
            UnsupportedPredicatePolicy::AlwaysFalse => Ok("1 = 0".to_string()),
            UnsupportedPredicatePolicy::FailFast => Err(
                SeaOrmTranslationError::UnsupportedPredicate(reason.to_string()),
            ),
            UnsupportedPredicatePolicy::ClientFilter => {
                Err(SeaOrmTranslationError::UnsupportedPredicate(format!(
                    "ClientFilter is not implemented for SeaORM translator: {reason}"
                )))
            }
        }
    }
}

fn column_ref(field: &str) -> Alias {
    Alias::new(field)
}

fn expression_value_to_value(value: &ExpressionValue) -> Value {
    match value {
        ExpressionValue::Null => Value::String(None),
        ExpressionValue::Boolean(value) => Value::Bool(Some(*value)),
        ExpressionValue::Number(value) => Value::Double(Some(*value)),
        ExpressionValue::String(value) => Value::String(Some(Box::new(value.clone()))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::{
        PredicateSchema, SortBy, UpdateAssignments, lower, path, runtime_field,
    };
    use ospf_rust_math::symbol::ScalarExpressionDsl;
    use sea_orm::sea_query::{MysqlQueryBuilder, PostgresQueryBuilder, Query, SqliteQueryBuilder};
    use sea_orm::{DbBackend, MockDatabase, MockExecResult, Transaction};
    use std::collections::BTreeMap;

    mod user_entity {
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
        #[sea_orm(table_name = "users")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i32,
            pub status: String,
            pub age: i32,
            pub name: String,
            pub score: i32,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }

    fn translator() -> SeaOrmTranslator<PredicateSchema> {
        let schema = PredicateSchema::new("Users")
            .with_field_name("id", "user_id")
            .with_field_name("status", "user_status")
            .with_field_name("age", "age")
            .with_field_name("name", "name")
            .with_field_name("score", "score");
        SeaOrmTranslator::new(schema)
    }

    fn select_sql(condition: Condition) -> String {
        Query::select()
            .column(Alias::new("user_id"))
            .from(Alias::new("users"))
            .cond_where(condition)
            .to_string(PostgresQueryBuilder)
    }

    #[test]
    fn translates_boolean_expression_to_sea_query_condition() {
        let expression = runtime_field("status").eq("active") & runtime_field("age").ge(18);

        let condition = translator().translate_boolean(&expression).unwrap();
        let sql = select_sql(condition);

        assert_eq!(
            sql,
            r#"SELECT "user_id" FROM "users" WHERE "user_status" = 'active' AND "age" >= 18"#
        );
    }

    #[test]
    fn translates_scalar_functions_and_like_patterns() {
        let expression = lower(path("name").as_scalar()).eq_expr("alice")
            & runtime_field("name").pattern_match("Al", PatternMatchMode::Prefix, false);

        let condition = translator().translate_boolean(&expression).unwrap();
        let sql = select_sql(condition);

        assert_eq!(
            sql,
            r#"SELECT "user_id" FROM "users" WHERE LOWER("name") = 'alice' AND "name" LIKE 'Al%'"#
        );
    }

    #[test]
    fn translates_sort_with_nulls_emulation() {
        let sort = SortBy::asc_nulls("name", NullsOrder::NullsLast);
        let orders = translator().translate_sort_by(&sort).unwrap();
        let mut query = Query::select();
        query
            .column(Alias::new("user_id"))
            .from(Alias::new("users"));
        for order in orders {
            query.order_by_expr(order.expression, order.order);
        }

        let sql = query.to_string(MysqlQueryBuilder);

        assert_eq!(
            sql,
            "SELECT `user_id` FROM `users` ORDER BY (CASE WHEN (`name` IS NULL) THEN 1 ELSE 0 END) ASC, `name` ASC"
        );
    }

    #[test]
    fn translates_update_assignments_to_expressions() {
        let assignments = UpdateAssignments::set("status", ExpressionValue::from("inactive"))
            .then_set_null("name")
            .then_set_expr(
                "score",
                ScalarExpression::add_expr(
                    ScalarExpression::reference("score"),
                    ScalarExpression::constant(ExpressionValue::from(1)),
                ),
            );

        let assignments = translator()
            .translate_update_assignments(&assignments)
            .unwrap();
        let mut query = Query::update();
        query.table(Alias::new("users"));
        for assignment in assignments {
            query.value(Alias::new(assignment.column), assignment.value);
        }

        let sql = query.to_string(PostgresQueryBuilder);

        assert_eq!(
            sql,
            r#"UPDATE "users" SET "user_status" = 'inactive', "name" = NULL, "score" = "score" + 1"#
        );
    }

    #[test]
    fn fail_fast_reports_unsupported_predicate() {
        let translator = SeaOrmTranslator::with_config(
            PredicateSchema::new("Users").with_same_field("name"),
            SeaOrmTranslatorConfig::default()
                .with_unsupported_predicate_policy(UnsupportedPredicatePolicy::FailFast),
        );
        let expression = runtime_field("name").regex("^A");

        let err = translator.translate_boolean(&expression).unwrap_err();

        assert!(matches!(
            err,
            SeaOrmTranslationError::UnsupportedPredicate(_)
        ));
    }

    #[test]
    fn sqlite_builder_can_render_translated_condition() {
        let condition = translator()
            .translate_boolean(&runtime_field("status").ne("inactive"))
            .unwrap();
        let sql = Query::select()
            .column(Alias::new("user_id"))
            .from(Alias::new("users"))
            .cond_where(condition)
            .to_string(SqliteQueryBuilder);

        assert_eq!(
            sql,
            r#"SELECT "user_id" FROM "users" WHERE "user_status" <> 'inactive'"#
        );
    }

    #[tokio::test]
    async fn sea_orm_repository_executes_with_mock_database() {
        let db = MockDatabase::new(DbBackend::Postgres)
            .append_query_results([[user_entity::Model {
                id: 1,
                status: "active".to_string(),
                age: 20,
                name: "Alice".to_string(),
                score: 7,
            }]])
            .append_query_results([[BTreeMap::from([(
                "num_items".to_string(),
                Value::BigInt(Some(1)),
            )])]])
            .append_exec_results([
                MockExecResult {
                    last_insert_id: 0,
                    rows_affected: 2,
                },
                MockExecResult {
                    last_insert_id: 0,
                    rows_affected: 3,
                },
            ])
            .into_connection();
        let repository = SeaOrmRepository::<user_entity::Entity, _, _>::new(
            db,
            PredicateSchema::new("Users")
                .with_same_field("id")
                .with_same_field("status")
                .with_same_field("age")
                .with_same_field("name")
                .with_same_field("score"),
        );
        let where_expr = runtime_field("status").eq("active");

        let rows = repository
            .find_with_options(
                &where_expr,
                &RepositoryQuery::new()
                    .with_sort_by(SortBy::desc("score"))
                    .with_limit(10),
            )
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(repository.count(&where_expr).await.unwrap(), 1);
        assert_eq!(
            repository
                .update(
                    &where_expr,
                    &UpdateAssignments::set("score", ExpressionValue::from(8)),
                )
                .await
                .unwrap(),
            2
        );
        assert_eq!(repository.delete(&where_expr).await.unwrap(), 3);

        let (db, _) = repository.into_parts();
        assert_eq!(
            db.into_transaction_log(),
            [
                Transaction::from_sql_and_values(
                    DbBackend::Postgres,
                    r#"SELECT "users"."id", "users"."status", "users"."age", "users"."name", "users"."score" FROM "users" WHERE "status" = $1 ORDER BY "score" DESC LIMIT $2"#,
                    ["active".into(), 10u64.into()]
                ),
                Transaction::from_sql_and_values(
                    DbBackend::Postgres,
                    r#"SELECT COUNT(*) AS num_items FROM (SELECT "users"."id", "users"."status", "users"."age", "users"."name", "users"."score" FROM "users" WHERE "status" = $1) AS "sub_query""#,
                    ["active".into()]
                ),
                Transaction::from_sql_and_values(
                    DbBackend::Postgres,
                    r#"UPDATE "users" SET "score" = $1 WHERE "status" = $2"#,
                    [8.0.into(), "active".into()]
                ),
                Transaction::from_sql_and_values(
                    DbBackend::Postgres,
                    r#"DELETE FROM "users" WHERE "status" = $1"#,
                    ["active".into()]
                ),
            ]
        );
    }
}
