//! 谓词 schema
//! Predicate schema

use std::collections::HashMap;
use std::marker::PhantomData;

use ospf_rust_math::symbol::{
    BooleanExpression, ComparisonOperator, ExpressionValue, PathBuilder, PatternMatchMode,
    PropertyPath, ScalarExpression,
};

use super::PersistenceFieldResolver;

/// 字段路径。
/// Field path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FieldPath<T = ExpressionValue> {
    path: PropertyPath,
    _marker: PhantomData<fn() -> T>,
}

impl<T> FieldPath<T> {
    /// 创建字段路径。
    /// Create a field path.
    pub fn new(path: impl Into<PropertyPath>) -> Self {
        Self {
            path: path.into(),
            _marker: PhantomData,
        }
    }

    /// 解析字段路径。
    /// Parse a field path.
    pub fn parse(path: impl AsRef<str>) -> Self {
        Self::new(PropertyPath::parse(path))
    }

    /// 获取属性路径。
    /// Get property path.
    pub fn path(&self) -> &PropertyPath {
        &self.path
    }

    /// 转换为属性路径。
    /// Convert into property path.
    pub fn into_path(self) -> PropertyPath {
        self.path
    }

    /// 转换为 math 层路径构建器。
    /// Convert to the math-layer path builder.
    pub fn as_path_builder(&self) -> PathBuilder<T> {
        PathBuilder::new(self.path.clone())
    }

    /// 转换为标量表达式。
    /// Convert to a scalar expression.
    pub fn as_scalar(&self) -> ScalarExpression<T> {
        ScalarExpression::reference(self.path.clone())
    }

    /// 转为其他值类型的字段路径。
    /// Convert to a field path with another value type.
    pub fn typed<U>(&self) -> FieldPath<U> {
        FieldPath::new(self.path.clone())
    }

    /// 创建比较表达式。
    /// Create a comparison expression.
    pub fn compare(
        &self,
        operator: ComparisonOperator,
        value: impl Into<ScalarExpression<T>>,
    ) -> BooleanExpression<T> {
        self.as_path_builder().compare(operator, value)
    }

    /// 等于。
    /// Equal.
    pub fn eq(&self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.as_path_builder().eq(value)
    }

    /// 不等于。
    /// Not equal.
    pub fn ne(&self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.as_path_builder().ne(value)
    }

    /// 小于。
    /// Less than.
    pub fn lt(&self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.as_path_builder().lt(value)
    }

    /// 小于等于。
    /// Less than or equal.
    pub fn le(&self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.as_path_builder().le(value)
    }

    /// 大于。
    /// Greater than.
    pub fn gt(&self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.as_path_builder().gt(value)
    }

    /// 大于等于。
    /// Greater than or equal.
    pub fn ge(&self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.as_path_builder().ge(value)
    }

    /// IN 谓词。
    /// IN predicate.
    pub fn in_values<I, V>(&self, values: I) -> BooleanExpression<T>
    where
        I: IntoIterator<Item = V>,
        V: Into<ScalarExpression<T>>,
    {
        self.as_path_builder().in_values(values)
    }

    /// NOT IN 谓词。
    /// NOT IN predicate.
    pub fn not_in_values<I, V>(&self, values: I) -> BooleanExpression<T>
    where
        I: IntoIterator<Item = V>,
        V: Into<ScalarExpression<T>>,
    {
        self.as_path_builder().not_in_values(values)
    }

    /// 空值判断。
    /// Null check.
    pub fn is_null(&self) -> BooleanExpression<T> {
        self.as_path_builder().is_null()
    }

    /// 非空判断。
    /// Not-null check.
    pub fn is_not_null(&self) -> BooleanExpression<T> {
        self.as_path_builder().is_not_null()
    }

    /// 模式匹配。
    /// Pattern match.
    pub fn pattern_match(
        &self,
        pattern: impl Into<ScalarExpression<T>>,
        mode: PatternMatchMode,
        negated: bool,
    ) -> BooleanExpression<T> {
        self.as_path_builder().pattern_match(pattern, mode, negated)
    }

    /// SQL LIKE 风格匹配。
    /// SQL LIKE-style match.
    pub fn like(&self, pattern: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.as_path_builder().like(pattern)
    }

    /// SQL LIKE 风格不匹配。
    /// SQL LIKE-style negative match.
    pub fn not_like(&self, pattern: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.as_path_builder().not_like(pattern)
    }

    /// 正则匹配。
    /// Regex match.
    pub fn regex(&self, pattern: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.as_path_builder().regex(pattern)
    }
}

impl<T> From<FieldPath<T>> for PropertyPath {
    fn from(value: FieldPath<T>) -> Self {
        value.path
    }
}

impl<T> From<&FieldPath<T>> for PropertyPath {
    fn from(value: &FieldPath<T>) -> Self {
        value.path.clone()
    }
}

impl<T> From<FieldPath<T>> for ScalarExpression<T> {
    fn from(value: FieldPath<T>) -> Self {
        ScalarExpression::reference(value.path)
    }
}

impl<T> From<&FieldPath<T>> for ScalarExpression<T> {
    fn from(value: &FieldPath<T>) -> Self {
        value.as_scalar()
    }
}

/// 创建字段路径。
/// Create a field path.
pub fn field<T>(path: impl AsRef<str>) -> FieldPath<T> {
    FieldPath::parse(path)
}

/// 创建运行时值字段路径。
/// Create a runtime-value field path.
pub fn runtime_field(path: impl AsRef<str>) -> FieldPath<ExpressionValue> {
    FieldPath::parse(path)
}

/// 字段映射。
/// Field mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldMapping<C = String> {
    pub path: PropertyPath,
    pub backend_field: C,
}

impl<C> FieldMapping<C> {
    /// 创建字段映射。
    /// Create a field mapping.
    pub fn new(path: impl Into<PropertyPath>, backend_field: C) -> Self {
        Self {
            path: path.into(),
            backend_field,
        }
    }
}

/// 谓词 schema。
/// Predicate schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateSchema<C = String> {
    name: String,
    fields: Vec<FieldMapping<C>>,
}

impl<C> PredicateSchema<C> {
    /// 创建空 schema。
    /// Create an empty schema.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fields: Vec::new(),
        }
    }

    /// 获取 schema 名称。
    /// Get schema name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取字段映射列表。
    /// Get field mappings.
    pub fn fields(&self) -> &[FieldMapping<C>] {
        &self.fields
    }

    /// 添加字段映射。
    /// Add a field mapping.
    pub fn with_field(mut self, path: impl Into<PropertyPath>, backend_field: C) -> Self {
        self.fields.push(FieldMapping::new(path, backend_field));
        self
    }

    /// 创建字段路径。
    /// Create a field path.
    pub fn field<T>(&self, path: impl Into<PropertyPath>) -> FieldPath<T> {
        FieldPath::new(path)
    }
}

impl PredicateSchema<String> {
    /// 添加同名字段映射。
    /// Add a field mapping whose backend field has the same name.
    pub fn with_same_field(self, path: impl Into<PropertyPath>) -> Self {
        let path = path.into();
        let backend_field = path.value().to_string();
        self.with_field(path, backend_field)
    }

    /// 添加字段名映射。
    /// Add a field-name mapping.
    pub fn with_field_name(
        self,
        path: impl Into<PropertyPath>,
        backend_field: impl Into<String>,
    ) -> Self {
        self.with_field(path, backend_field.into())
    }
}

impl<C> PredicateSchema<C>
where
    C: Clone,
{
    /// 解析后端字段。
    /// Resolve a backend field.
    pub fn resolve(&self, path: impl Into<PropertyPath>) -> Option<C> {
        let path = path.into();
        self.fields
            .iter()
            .find(|mapping| mapping.path == path)
            .map(|mapping| mapping.backend_field.clone())
    }

    /// 创建字符串 path resolver。
    /// Create a string-path resolver.
    pub fn resolver(&self) -> impl Fn(&str) -> Option<C> + '_ {
        move |path| self.resolve(PropertyPath::parse(path))
    }

    /// 转换为字段映射表。
    /// Convert to a field mapping table.
    pub fn to_field_map(&self) -> HashMap<PropertyPath, C> {
        self.fields
            .iter()
            .map(|mapping| (mapping.path.clone(), mapping.backend_field.clone()))
            .collect()
    }
}

impl<C> PersistenceFieldResolver<C> for PredicateSchema<C>
where
    C: Clone,
{
    fn resolve_field(&self, path: &PropertyPath) -> Option<C> {
        self.resolve(path.clone())
    }
}

/// 谓词 schema 构建器。
/// Predicate schema builder.
pub type PredicateSchemaBuilder<C = String> = PredicateSchema<C>;

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_math::symbol::{BooleanExpression, ExpressionValue};

    #[test]
    fn field_path_builds_runtime_predicates() {
        let status = runtime_field("status");
        let age = runtime_field("age");
        let expression = status.eq("active") & age.gt(18);

        let BooleanExpression::And(operands) = expression else {
            panic!("expected AND expression");
        };
        assert_eq!(operands.len(), 2);
        assert!(
            operands[0]
                .collect_references()
                .contains(&PropertyPath::parse("status"))
        );
        assert!(
            operands[1]
                .collect_references()
                .contains(&PropertyPath::parse("age"))
        );
    }

    #[test]
    fn field_path_can_be_typed() {
        let age = field::<i32>("age");
        let expression = age.gt(18);

        assert!(
            expression
                .collect_references()
                .contains(&PropertyPath::parse("age"))
        );
    }

    #[test]
    fn predicate_schema_resolves_backend_fields() {
        let schema = PredicateSchema::new("Users")
            .with_field_name("id", "user_id")
            .with_same_field("age");

        assert_eq!(schema.name(), "Users");
        assert_eq!(schema.resolve("id"), Some("user_id".to_string()));
        assert_eq!(schema.resolve("age"), Some("age".to_string()));
        assert_eq!(schema.resolve("missing"), None);

        let resolver = schema.resolver();
        assert_eq!(resolver("id"), Some("user_id".to_string()));
    }

    #[test]
    fn predicate_schema_implements_field_resolver() {
        let schema = PredicateSchema::new("Users").with_field_name("status", "user_status");

        assert_eq!(
            schema.resolve_field(&PropertyPath::parse("status")),
            Some("user_status".to_string())
        );
    }

    #[test]
    fn predicate_schema_supports_non_string_backend_fields() {
        let schema = PredicateSchema::new("Users").with_field("id", 7_u32);

        assert_eq!(schema.resolve("id"), Some(7_u32));
    }

    #[test]
    fn field_path_scalar_conversion_uses_same_path() {
        let status = runtime_field("status");
        let scalar: ScalarExpression<ExpressionValue> = (&status).into();

        assert_eq!(scalar, ScalarExpression::reference("status"));
    }
}
