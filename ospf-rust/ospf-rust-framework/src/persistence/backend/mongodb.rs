//! MongoDB 持久化后端
//! MongoDB persistence backend

use ospf_rust_math::Trivalent;
use ospf_rust_math::symbol::{
    BooleanExpression, ComparisonOperator, ExpressionValue, NullCheckType, PatternMatchMode,
    ScalarExpression,
};
use serde_json::{Map, Number, Value, json};
use std::fmt::{Display, Formatter};

use crate::persistence::{
    PersistenceFieldResolver, UnsupportedPredicatePolicy, UpdateAssignment, UpdateAssignments,
};

/// MongoDB 后端标记类型。
/// MongoDB backend marker type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MongoDbBackend;

/// MongoDB 配置。
/// MongoDB configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MongoDbConfig {
    /// 连接 URI / Connection URI
    pub uri: String,
    /// 数据库名 / Database name
    pub database: String,
    /// 默认 collection 名称 / Default collection name
    pub default_collection: Option<String>,
}

impl MongoDbConfig {
    /// 创建配置。
    /// Create configuration.
    pub fn new(uri: impl Into<String>, database: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            database: database.into(),
            default_collection: None,
        }
    }

    /// 设置默认 collection。
    /// Set default collection.
    pub fn with_default_collection(mut self, collection: impl Into<String>) -> Self {
        self.default_collection = Some(collection.into());
        self
    }
}

/// MongoDB client handle。
/// MongoDB client handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MongoDbClientHandle<C> {
    /// 配置 / Configuration
    pub config: MongoDbConfig,
    /// 客户端实例 / Client instance
    pub client: C,
}

/// MongoDB 翻译错误。
/// MongoDB translation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MongoDbTranslationError {
    /// 不支持的谓词 / Unsupported predicate
    UnsupportedPredicate(String),
    /// 未解析的字段 / Unresolved field
    UnresolvedField(String),
    /// 无效表达式 / Invalid expression
    InvalidExpression(String),
}

impl Display for MongoDbTranslationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPredicate(message) => write!(f, "unsupported predicate: {message}"),
            Self::UnresolvedField(message) => write!(f, "unresolved field: {message}"),
            Self::InvalidExpression(message) => write!(f, "invalid expression: {message}"),
        }
    }
}

impl std::error::Error for MongoDbTranslationError {}

/// MongoDB 表达式翻译器。
/// MongoDB expression translator.
#[derive(Debug, Clone)]
pub struct MongoDbTranslator<R> {
    resolver: R,
    unsupported_predicate_policy: UnsupportedPredicatePolicy,
}

impl<R> MongoDbTranslator<R> {
    /// 创建翻译器。
    /// Create a translator.
    pub fn new(resolver: R) -> Self {
        Self {
            resolver,
            unsupported_predicate_policy: UnsupportedPredicatePolicy::default(),
        }
    }

    /// 设置 unsupported policy。
    /// Set unsupported policy.
    pub fn with_unsupported_predicate_policy(
        mut self,
        unsupported_predicate_policy: UnsupportedPredicatePolicy,
    ) -> Self {
        self.unsupported_predicate_policy = unsupported_predicate_policy;
        self
    }
}

impl<R> MongoDbTranslator<R>
where
    R: PersistenceFieldResolver<String>,
{
    /// 翻译布尔表达式为 MongoDB filter document。
    /// Translate a boolean expression to a MongoDB filter document.
    pub fn translate_filter(
        &self,
        expression: &BooleanExpression<ExpressionValue>,
    ) -> Result<Value, MongoDbTranslationError> {
        self.translate_boolean(expression)
    }

    /// 翻译更新赋值为 `$set` / `$unset` document。
    /// Translate update assignments to `$set` / `$unset` document.
    pub fn translate_update(
        &self,
        assignments: &UpdateAssignments<ExpressionValue>,
    ) -> Result<Value, MongoDbTranslationError> {
        let mut set = Map::new();
        let mut unset = Map::new();
        for assignment in &assignments.items {
            match assignment {
                UpdateAssignment::SetValue(value) => {
                    set.insert(
                        self.resolve_field(&value.path)?,
                        expression_value_to_json(&value.value)?,
                    );
                }
                UpdateAssignment::SetNull(value) => {
                    unset.insert(self.resolve_field(&value.path)?, json!(""));
                }
                UpdateAssignment::SetFromExpression(_) => {
                    return Err(MongoDbTranslationError::UnsupportedPredicate(
                        "MongoDB update expression assignment is not supported".to_string(),
                    ));
                }
            }
        }
        let mut update = Map::new();
        if !set.is_empty() {
            update.insert("$set".to_string(), Value::Object(set));
        }
        if !unset.is_empty() {
            update.insert("$unset".to_string(), Value::Object(unset));
        }
        Ok(Value::Object(update))
    }

    fn translate_boolean(
        &self,
        expression: &BooleanExpression<ExpressionValue>,
    ) -> Result<Value, MongoDbTranslationError> {
        match expression {
            BooleanExpression::Constant(Trivalent::True) => Ok(json!({})),
            BooleanExpression::Constant(Trivalent::False | Trivalent::Unknown) => {
                Ok(json!({"$expr": {"$eq": [1, 0]}}))
            }
            BooleanExpression::Comparison {
                operator,
                left,
                right,
            } => self.translate_comparison(*operator, left, right),
            BooleanExpression::In {
                value,
                candidates,
                negated,
            } => {
                let (field, _) = self.reference_and_constant(value, None)?;
                let values = candidates
                    .iter()
                    .map(|candidate| self.scalar_constant(candidate))
                    .collect::<Result<Vec<_>, _>>()?;
                let operator = if *negated { "$nin" } else { "$in" };
                Ok(json!({field: {operator: values}}))
            }
            BooleanExpression::PatternMatch {
                value,
                pattern,
                mode,
                negated,
            } => {
                let (field, _) = self.reference_and_constant(value, None)?;
                let pattern = self.scalar_constant(pattern)?;
                let Some(pattern) = pattern.as_str() else {
                    return Err(MongoDbTranslationError::InvalidExpression(
                        "MongoDB pattern must be a string".to_string(),
                    ));
                };
                let regex = pattern_to_regex(pattern, *mode)?;
                let mut body = json!({"$regex": regex});
                if *negated {
                    body = json!({"$not": body});
                }
                Ok(json!({field: body}))
            }
            BooleanExpression::NullCheck {
                path,
                null_check_type,
            } => {
                let field = self.resolve_field(path)?;
                Ok(match null_check_type {
                    NullCheckType::IsNull => json!({field: null}),
                    NullCheckType::IsNotNull => json!({field: {"$ne": null}}),
                })
            }
            BooleanExpression::And(operands) => self.translate_logical("$and", operands),
            BooleanExpression::Or(operands) => self.translate_logical("$or", operands),
            BooleanExpression::Not(operand) => {
                Ok(json!({"$nor": [self.translate_boolean(operand)?]}))
            }
            BooleanExpression::Custom { description, .. } => {
                self.unsupported(description.as_deref().unwrap_or("custom MongoDB predicate"))
            }
        }
    }

    fn translate_comparison(
        &self,
        operator: ComparisonOperator,
        left: &ScalarExpression<ExpressionValue>,
        right: &ScalarExpression<ExpressionValue>,
    ) -> Result<Value, MongoDbTranslationError> {
        let (field, value) = self.reference_and_constant(left, Some(right))?;
        let value = value.ok_or_else(|| {
            MongoDbTranslationError::InvalidExpression("missing comparison value".to_string())
        })?;
        Ok(match operator {
            ComparisonOperator::Eq => json!({field: value}),
            ComparisonOperator::Ne => json!({field: {"$ne": value}}),
            ComparisonOperator::Lt => json!({field: {"$lt": value}}),
            ComparisonOperator::Le => json!({field: {"$lte": value}}),
            ComparisonOperator::Gt => json!({field: {"$gt": value}}),
            ComparisonOperator::Ge => json!({field: {"$gte": value}}),
        })
    }

    fn translate_logical(
        &self,
        operator: &str,
        operands: &[BooleanExpression<ExpressionValue>],
    ) -> Result<Value, MongoDbTranslationError> {
        let values = operands
            .iter()
            .map(|operand| self.translate_boolean(operand))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(json!({operator: values}))
    }

    fn reference_and_constant(
        &self,
        left: &ScalarExpression<ExpressionValue>,
        right: Option<&ScalarExpression<ExpressionValue>>,
    ) -> Result<(String, Option<Value>), MongoDbTranslationError> {
        let ScalarExpression::Reference(path) = left else {
            return Err(MongoDbTranslationError::UnsupportedPredicate(
                "MongoDB translator expects field reference on the left side".to_string(),
            ));
        };
        Ok((
            self.resolve_field(path)?,
            right.map(|right| self.scalar_constant(right)).transpose()?,
        ))
    }

    fn scalar_constant(
        &self,
        expression: &ScalarExpression<ExpressionValue>,
    ) -> Result<Value, MongoDbTranslationError> {
        let ScalarExpression::Constant(value) = expression else {
            return Err(MongoDbTranslationError::UnsupportedPredicate(
                "MongoDB translator supports constant scalar values only".to_string(),
            ));
        };
        expression_value_to_json(value)
    }

    fn resolve_field(
        &self,
        path: &ospf_rust_math::symbol::PropertyPath,
    ) -> Result<String, MongoDbTranslationError> {
        self.resolver
            .resolve_field(path)
            .ok_or_else(|| MongoDbTranslationError::UnresolvedField(path.value().to_string()))
    }

    fn unsupported(&self, reason: &str) -> Result<Value, MongoDbTranslationError> {
        match self.unsupported_predicate_policy {
            UnsupportedPredicatePolicy::AlwaysFalse => Ok(json!({"$expr": {"$eq": [1, 0]}})),
            UnsupportedPredicatePolicy::FailFast => Err(
                MongoDbTranslationError::UnsupportedPredicate(reason.to_string()),
            ),
            UnsupportedPredicatePolicy::ClientFilter => {
                Err(MongoDbTranslationError::UnsupportedPredicate(format!(
                    "ClientFilter is not implemented for MongoDB translator: {reason}"
                )))
            }
        }
    }
}

/// MongoDB 仓储操作文档构建器。
/// MongoDB repository operation document builder.
#[derive(Debug, Clone)]
pub struct MongoDbRepositoryAdapter<R> {
    collection: String,
    translator: MongoDbTranslator<R>,
}

impl<R> MongoDbRepositoryAdapter<R> {
    /// 创建 adapter。
    /// Create an adapter.
    pub fn new(collection: impl Into<String>, resolver: R) -> Self {
        Self {
            collection: collection.into(),
            translator: MongoDbTranslator::new(resolver),
        }
    }

    /// 创建插入操作。
    /// Create an insert operation.
    pub fn insert_document(&self, document: Value) -> MongoDbOperation {
        MongoDbOperation {
            collection: self.collection.clone(),
            filter: None,
            update: None,
            document: Some(document),
        }
    }
}

impl<R> MongoDbRepositoryAdapter<R>
where
    R: PersistenceFieldResolver<String>,
{
    /// 创建查询操作。
    /// Create a find operation.
    pub fn find(
        &self,
        where_expr: &BooleanExpression<ExpressionValue>,
    ) -> Result<MongoDbOperation, MongoDbTranslationError> {
        Ok(MongoDbOperation {
            collection: self.collection.clone(),
            filter: Some(self.translator.translate_filter(where_expr)?),
            update: None,
            document: None,
        })
    }

    /// 创建更新操作。
    /// Create an update operation.
    pub fn update(
        &self,
        where_expr: &BooleanExpression<ExpressionValue>,
        assignments: &UpdateAssignments<ExpressionValue>,
    ) -> Result<MongoDbOperation, MongoDbTranslationError> {
        Ok(MongoDbOperation {
            collection: self.collection.clone(),
            filter: Some(self.translator.translate_filter(where_expr)?),
            update: Some(self.translator.translate_update(assignments)?),
            document: None,
        })
    }
}

/// MongoDB 仓储操作。
/// MongoDB repository operation.
#[derive(Debug, Clone, PartialEq)]
pub struct MongoDbOperation {
    /// collection 名称 / Collection name
    pub collection: String,
    /// 过滤条件文档 / Filter document
    pub filter: Option<Value>,
    /// 更新操作文档 / Update document
    pub update: Option<Value>,
    /// 插入文档 / Insert document
    pub document: Option<Value>,
}

fn expression_value_to_json(value: &ExpressionValue) -> Result<Value, MongoDbTranslationError> {
    Ok(match value {
        ExpressionValue::Null => Value::Null,
        ExpressionValue::Boolean(value) => Value::Bool(*value),
        ExpressionValue::Number(value) => {
            Value::Number(Number::from_f64(*value).ok_or_else(|| {
                MongoDbTranslationError::InvalidExpression("non-finite number".to_string())
            })?)
        }
        ExpressionValue::String(value) => Value::String(value.clone()),
    })
}

fn pattern_to_regex(
    pattern: &str,
    mode: PatternMatchMode,
) -> Result<String, MongoDbTranslationError> {
    Ok(match mode {
        PatternMatchMode::Exact => format!("^{}$", escape_regex(pattern)),
        PatternMatchMode::Prefix => format!("^{}", escape_regex(pattern)),
        PatternMatchMode::Suffix => format!("{}$", escape_regex(pattern)),
        PatternMatchMode::Contains => escape_regex(pattern),
        PatternMatchMode::Like => pattern.to_string(),
        PatternMatchMode::Regex => pattern.to_string(),
    })
}

fn escape_regex(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        if ".+*?^$()[]{}|\\".contains(ch) {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::{UpdateAssignments, runtime_field};

    #[test]
    fn mongodb_translator_builds_filter_and_update_documents() {
        let adapter = MongoDbRepositoryAdapter::new("users", |path: &str| Some(path.to_string()));
        let operation = adapter
            .update(
                &(runtime_field("status").eq("active") & runtime_field("age").ge(18)),
                &UpdateAssignments::set("status", "inactive".into()).then_set_null("deleted_at"),
            )
            .unwrap();

        assert_eq!(operation.collection, "users");
        assert_eq!(
            operation.filter.unwrap(),
            json!({"$and": [{"status": "active"}, {"age": {"$gte": 18.0}}]})
        );
        assert_eq!(
            operation.update.unwrap(),
            json!({"$set": {"status": "inactive"}, "$unset": {"deleted_at": ""}})
        );
    }
}
