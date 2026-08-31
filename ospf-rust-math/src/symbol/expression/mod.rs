//! 运行时表达式系统
//! Runtime expression system

use crate::Trivalent;
use crate::symbol::{DynSymbol, OwnedSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use std::ops::{BitAnd, BitOr, Not as StdNot};
use std::str::FromStr;

/// 属性路径解析错误。
/// Property path parse error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyPathParseError {
    text: String,
}

impl PropertyPathParseError {
    /// 创建属性路径解析错误。
    /// Create a property path parse error.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    /// 获取原始文本。
    /// Get original text.
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl Display for PropertyPathParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid property path: {}", self.text)
    }
}

impl std::error::Error for PropertyPathParseError {}

/// 属性路径。
/// Property path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PropertyPath {
    value: String,
}

impl PropertyPath {
    /// 空属性路径。
    /// Empty property path.
    pub const EMPTY: Self = Self {
        value: String::new(),
    };

    /// 直接创建属性路径，不校验标识符格式。
    /// Create a property path directly without validating identifier format.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }

    /// 从路径分段创建属性路径。
    /// Create a property path from segments.
    pub fn of<I, S>(segments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let value = segments
            .into_iter()
            .map(|segment| segment.as_ref().to_string())
            .collect::<Vec<_>>()
            .join(".");
        Self { value }
    }

    /// 解析属性路径，行为与 Kotlin `parse` 一致，仅裁剪空白。
    /// Parse property path like Kotlin `parse`, trimming whitespace only.
    pub fn parse(text: impl AsRef<str>) -> Self {
        Self::new(text.as_ref().trim())
    }

    /// 尝试解析属性路径，要求每个分段都是合法标识符。
    /// Try to parse property path, requiring every segment to be a valid identifier.
    pub fn parse_or_none(text: impl AsRef<str>) -> Option<Self> {
        let trimmed = text.as_ref().trim();
        if trimmed.is_empty() {
            return None;
        }
        let segments = trimmed.split('.');
        if segments.clone().all(Self::is_valid_identifier) {
            Some(Self::new(trimmed))
        } else {
            None
        }
    }

    /// 尝试解析属性路径，失败时返回错误。
    /// Try to parse property path, returning an error on failure.
    pub fn try_parse(text: impl AsRef<str>) -> Result<Self, PropertyPathParseError> {
        let text = text.as_ref();
        Self::parse_or_none(text).ok_or_else(|| PropertyPathParseError::new(text))
    }

    /// 获取路径字符串。
    /// Get path string.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// 获取路径分段。
    /// Get path segments.
    pub fn segments(&self) -> Vec<&str> {
        if self.value.is_empty() {
            Vec::new()
        } else {
            self.value.split('.').collect()
        }
    }

    /// 路径是否为空。
    /// Check whether the path is empty.
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// 路径是否非空。
    /// Check whether the path is non-empty.
    pub fn is_not_empty(&self) -> bool {
        !self.is_empty()
    }

    /// 获取路径深度。
    /// Get path depth.
    pub fn depth(&self) -> usize {
        self.segments().len()
    }

    /// 获取根分段。
    /// Get root segment.
    pub fn root(&self) -> Option<&str> {
        self.segments().first().copied()
    }

    /// 获取叶分段。
    /// Get leaf segment.
    pub fn leaf(&self) -> Option<&str> {
        self.segments().last().copied()
    }

    /// 获取父路径。
    /// Get parent path.
    pub fn parent(&self) -> Option<Self> {
        let segments = self.segments();
        if segments.len() <= 1 {
            None
        } else {
            Some(Self::of(&segments[..segments.len() - 1]))
        }
    }

    /// 获取子路径。
    /// Get child path.
    pub fn child(&self) -> Option<Self> {
        let segments = self.segments();
        if segments.len() <= 1 {
            None
        } else {
            Some(Self::of(&segments[1..]))
        }
    }

    /// 判断当前路径是否是另一个路径的子路径。
    /// Check whether this path is a sub-path of another path.
    pub fn is_sub_path_of(&self, other: &Self) -> bool {
        if self.is_empty() || other.is_empty() {
            return false;
        }
        let this_segments = self.segments();
        let other_segments = other.segments();
        this_segments.len() > other_segments.len()
            && this_segments
                .iter()
                .zip(other_segments.iter())
                .all(|(left, right)| left == right)
    }

    /// 判断当前路径是否是另一个路径的父路径。
    /// Check whether this path is a parent path of another path.
    pub fn is_parent_path_of(&self, other: &Self) -> bool {
        other.is_sub_path_of(self)
    }

    /// 拼接属性路径。
    /// Concatenate property paths.
    pub fn concat(&self, other: &Self) -> Self {
        if self.is_empty() {
            other.clone()
        } else if other.is_empty() {
            self.clone()
        } else {
            Self::new(format!("{}.{}", self.value, other.value))
        }
    }

    /// 拼接路径分段。
    /// Concatenate one path segment.
    pub fn concat_segment(&self, segment: impl AsRef<str>) -> Self {
        let segment = segment.as_ref();
        if self.is_empty() {
            Self::new(segment)
        } else {
            Self::new(format!("{}.{}", self.value, segment))
        }
    }

    /// 校验标识符是否有效。
    /// Validate whether an identifier is valid.
    pub fn is_valid_identifier(identifier: &str) -> bool {
        let mut chars = identifier.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        (first.is_alphabetic() || first == '_') && chars.all(|ch| ch.is_alphanumeric() || ch == '_')
    }
}

impl Display for PropertyPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl From<&str> for PropertyPath {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for PropertyPath {
    fn from(value: String) -> Self {
        Self::parse(value)
    }
}

impl From<PropertyPath> for String {
    fn from(value: PropertyPath) -> Self {
        value.value
    }
}

impl FromStr for PropertyPath {
    type Err = PropertyPathParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_parse(value)
    }
}

/// 路径符号。
/// Path symbol.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PathSymbol {
    path: PropertyPath,
    symbol_id: String,
}

impl PathSymbol {
    /// 创建路径符号。
    /// Create a path symbol.
    pub fn new(path: impl Into<PropertyPath>) -> Self {
        let path = path.into();
        let symbol_id = path_symbol_id(&path);
        Self { path, symbol_id }
    }

    /// 从路径创建路径符号。
    /// Create a path symbol from a path.
    pub fn from_path(path: PropertyPath) -> Self {
        Self::new(path)
    }

    /// 从路径字符串创建路径符号。
    /// Create a path symbol from a path string.
    pub fn from_path_str(path: impl AsRef<str>) -> Self {
        Self::new(PropertyPath::parse(path))
    }

    /// 从路径分段创建路径符号。
    /// Create a path symbol from path segments.
    pub fn of<I, S>(segments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self::new(PropertyPath::of(segments))
    }

    /// 获取属性路径。
    /// Get property path.
    pub fn path(&self) -> &PropertyPath {
        &self.path
    }

    /// 获取 Kotlin 兼容符号 id。
    /// Get Kotlin-compatible symbol id.
    pub fn symbol_id(&self) -> &str {
        &self.symbol_id
    }

    /// 转换为拥有权符号。
    /// Convert to owned symbol.
    pub fn into_owned_symbol(self) -> OwnedSymbol {
        OwnedSymbol::new(self)
    }

    fn stable_parent_id(&self) -> usize {
        stable_path_symbol_hash(self.symbol_id.as_bytes())
    }
}

impl Display for PathSymbol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

impl DynSymbol for PathSymbol {
    fn name(&self) -> &str {
        self.path.value()
    }

    fn display_name(&self) -> &str {
        self.path.value()
    }

    fn dyn_id(&self) -> SymbolDynId<'_> {
        SymbolDynId::component("PathSymbol", self.stable_parent_id(), 0)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Symbol for PathSymbol {
    type Id = String;

    fn id(&self) -> Self::Id {
        self.symbol_id.clone()
    }
}

/// 创建 Kotlin 兼容路径符号 id。
/// Create a Kotlin-compatible path symbol id.
pub fn path_symbol_id(path: &PropertyPath) -> String {
    format!("path:{}", path.value())
}

/// 创建路径符号。
/// Create a path symbol.
pub fn path_symbol(path: impl Into<PropertyPath>) -> PathSymbol {
    PathSymbol::new(path)
}

/// 创建路径符号并转换为拥有权符号。
/// Create a path symbol and convert it to an owned symbol.
pub fn path_owned_symbol(path: impl Into<PropertyPath>) -> OwnedSymbol {
    PathSymbol::new(path).into_owned_symbol()
}

/// 从动态符号提取属性路径。
/// Extract property path from a dynamic symbol.
pub fn property_path_from_symbol(symbol: &dyn DynSymbol) -> Option<&PropertyPath> {
    symbol
        .as_any()
        .downcast_ref::<PathSymbol>()
        .map(PathSymbol::path)
}

/// 从拥有权符号提取属性路径。
/// Extract property path from an owned symbol.
pub fn property_path_from_owned_symbol(symbol: &OwnedSymbol) -> Option<&PropertyPath> {
    property_path_from_symbol(symbol.as_ref())
}

fn stable_path_symbol_hash(bytes: &[u8]) -> usize {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash as usize
}

/// 标量一元操作符。
/// Scalar unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UnaryOperator {
    /// 负号 / Negation
    Negate,
    /// 正号 / Positive identity
    Positive,
    /// 绝对值 / Absolute value
    Abs,
}

impl UnaryOperator {
    /// 获取操作符符号。
    /// Get operator symbol.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Negate => "-",
            Self::Positive => "+",
            Self::Abs => "abs",
        }
    }
}

/// 标量二元操作符。
/// Scalar binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BinaryOperator {
    /// 加法 / Addition
    Add,
    /// 减法 / Subtraction
    Subtract,
    /// 乘法 / Multiplication
    Multiply,
    /// 除法 / Division
    Divide,
    /// 取模 / Modulo
    Modulo,
    /// 幂运算 / Power
    Power,
}

impl BinaryOperator {
    /// 获取操作符符号。
    /// Get operator symbol.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Modulo => "%",
            Self::Power => "^",
        }
    }
}

/// 比较操作符。
/// Comparison operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ComparisonOperator {
    /// 等于 / Equal
    Eq,
    /// 不等于 / Not equal
    Ne,
    /// 小于 / Less than
    Lt,
    /// 小于等于 / Less than or equal
    Le,
    /// 大于 / Greater than
    Gt,
    /// 大于等于 / Greater than or equal
    Ge,
}

impl ComparisonOperator {
    /// 获取操作符符号。
    /// Get operator symbol.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Ne => "<>",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
        }
    }

    /// 获取反向比较操作符。
    /// Get inverse comparison operator.
    pub const fn inverse(self) -> Self {
        match self {
            Self::Eq => Self::Ne,
            Self::Ne => Self::Eq,
            Self::Lt => Self::Gt,
            Self::Le => Self::Ge,
            Self::Gt => Self::Lt,
            Self::Ge => Self::Le,
        }
    }
}

/// 模式匹配模式。
/// Pattern match mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PatternMatchMode {
    /// 精确匹配 / Exact match
    Exact,
    /// 前缀匹配 / Prefix match
    Prefix,
    /// 后缀匹配 / Suffix match
    Suffix,
    /// 包含匹配 / Contains match
    Contains,
    /// SQL LIKE 风格通配符匹配 / SQL LIKE-style wildcard match
    Like,
    /// 正则匹配 / Regex match
    Regex,
}

/// 布尔操作符。
/// Boolean operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BooleanOperator {
    /// 逻辑与 / Logical AND
    And,
    /// 逻辑或 / Logical OR
    Or,
    /// 逻辑非 / Logical NOT
    Not,
}

impl BooleanOperator {
    /// 获取操作符符号。
    /// Get operator symbol.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::And => "and",
            Self::Or => "or",
            Self::Not => "not",
        }
    }
}

/// 空值检查类型。
/// Null check type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NullCheckType {
    /// 是空值 / Is null
    IsNull,
    /// 非空值 / Is not null
    IsNotNull,
}

impl NullCheckType {
    /// 获取操作符符号。
    /// Get operator symbol.
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::IsNull => "is null",
            Self::IsNotNull => "is not null",
        }
    }
}

/// 表达式解析器使用的标量字面量。
/// Scalar literal used by the expression parser.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExpressionValue {
    /// 空值 / Null value
    Null,
    /// 布尔值 / Boolean value
    Boolean(bool),
    /// 数字值 / Number value
    Number(f64),
    /// 字符串值 / String value
    String(String),
}

impl Display for ExpressionValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Boolean(value) => write!(f, "{}", value),
            Self::Number(value) => write!(f, "{}", value),
            Self::String(value) => write!(f, "{}", value),
        }
    }
}

impl From<bool> for ExpressionValue {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<f64> for ExpressionValue {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<f32> for ExpressionValue {
    fn from(value: f32) -> Self {
        Self::Number(f64::from(value))
    }
}

macro_rules! impl_expression_value_from_integer {
    ($($type:ty),* $(,)?) => {
        $(
            impl From<$type> for ExpressionValue {
                fn from(value: $type) -> Self {
                    Self::Number(value as f64)
                }
            }
        )*
    };
}

impl_expression_value_from_integer!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl From<&str> for ExpressionValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<String> for ExpressionValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

/// 解析后的标量表达式。
/// Parsed scalar expression.
pub type ParsedScalarExpression = ScalarExpression<ExpressionValue>;

/// 解析后的布尔表达式。
/// Parsed boolean expression.
pub type ParsedBooleanExpression = BooleanExpression<ExpressionValue>;

/// 标量表达式。
/// Scalar expression.
#[derive(Debug, Clone, PartialEq)]
pub enum ScalarExpression<T> {
    /// 标量常量 / Scalar constant
    Constant(T),
    /// 属性路径引用 / Property path reference
    Reference(PropertyPath),
    /// 普通符号引用 / Plain symbol reference
    SymbolReference(OwnedSymbol),
    /// 一元操作 / Unary operation
    Unary {
        /// 操作符 / Operator
        operator: UnaryOperator,
        /// 操作数 / Operand
        operand: Box<ScalarExpression<T>>,
    },
    /// 二元操作 / Binary operation
    Binary {
        /// 操作符 / Operator
        operator: BinaryOperator,
        /// 左操作数 / Left operand
        left: Box<ScalarExpression<T>>,
        /// 右操作数 / Right operand
        right: Box<ScalarExpression<T>>,
    },
    /// 函数调用 / Function call
    Function {
        /// 函数名 / Function name
        name: String,
        /// 参数列表 / Argument list
        arguments: Vec<ScalarExpression<T>>,
    },
    /// 自定义表达式 / Custom expression
    Custom {
        /// 自定义载荷 / Custom payload
        payload: String,
        /// 可选描述 / Optional description
        description: Option<String>,
    },
}

impl<T> ScalarExpression<T> {
    /// 创建常量表达式。
    /// Create constant expression.
    pub fn constant(value: T) -> Self {
        Self::Constant(value)
    }

    /// 创建路径引用表达式。
    /// Create path reference expression.
    pub fn reference(path: impl Into<PropertyPath>) -> Self {
        Self::Reference(path.into())
    }

    /// 创建符号引用表达式。
    /// Create symbol reference expression.
    pub fn symbol_reference(symbol: OwnedSymbol) -> Self {
        Self::SymbolReference(symbol)
    }

    /// 创建一元表达式。
    /// Create unary expression.
    pub fn unary(operator: UnaryOperator, operand: Self) -> Self {
        Self::Unary {
            operator,
            operand: Box::new(operand),
        }
    }

    /// 创建二元表达式。
    /// Create binary expression.
    pub fn binary(operator: BinaryOperator, left: Self, right: Self) -> Self {
        Self::Binary {
            operator,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// 创建函数调用表达式。
    /// Create function call expression.
    pub fn function(name: impl Into<String>, arguments: Vec<Self>) -> Self {
        Self::Function {
            name: name.into(),
            arguments,
        }
    }

    /// 创建自定义表达式。
    /// Create custom expression.
    pub fn custom(payload: impl Into<String>, description: Option<String>) -> Self {
        Self::Custom {
            payload: payload.into(),
            description,
        }
    }

    /// 创建加法表达式。
    /// Create addition expression.
    pub fn add_expr(left: Self, right: Self) -> Self {
        Self::binary(BinaryOperator::Add, left, right)
    }

    /// 创建减法表达式。
    /// Create subtraction expression.
    pub fn subtract_expr(left: Self, right: Self) -> Self {
        Self::binary(BinaryOperator::Subtract, left, right)
    }

    /// 创建乘法表达式。
    /// Create multiplication expression.
    pub fn multiply_expr(left: Self, right: Self) -> Self {
        Self::binary(BinaryOperator::Multiply, left, right)
    }

    /// 创建除法表达式。
    /// Create division expression.
    pub fn divide_expr(left: Self, right: Self) -> Self {
        Self::binary(BinaryOperator::Divide, left, right)
    }

    /// 获取表达式类型名。
    /// Get expression type name.
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Constant(_) => "Constant",
            Self::Reference(_) => "Reference",
            Self::SymbolReference(_) => "SymbolReference",
            Self::Unary { .. } => "Unary",
            Self::Binary { .. } => "Binary",
            Self::Function { .. } => "Function",
            Self::Custom { .. } => "Custom",
        }
    }

    /// 判断表达式是否是常量表达式。
    /// Check whether the expression is constant.
    pub fn is_constant(&self) -> bool {
        match self {
            Self::Constant(_) => true,
            Self::Reference(_) | Self::SymbolReference(_) | Self::Custom { .. } => false,
            Self::Unary { operand, .. } => operand.is_constant(),
            Self::Binary { left, right, .. } => left.is_constant() && right.is_constant(),
            Self::Function { arguments, .. } => arguments.iter().all(Self::is_constant),
        }
    }

    /// 判断表达式是否包含引用。
    /// Check whether the expression contains references.
    pub fn contains_reference(&self) -> bool {
        match self {
            Self::Constant(_) => false,
            Self::Reference(_) | Self::SymbolReference(_) | Self::Custom { .. } => true,
            Self::Unary { operand, .. } => operand.contains_reference(),
            Self::Binary { left, right, .. } => {
                left.contains_reference() || right.contains_reference()
            }
            Self::Function { arguments, .. } => arguments.iter().any(Self::contains_reference),
        }
    }

    /// 收集表达式中的属性路径引用。
    /// Collect property path references in the expression.
    pub fn collect_references(&self) -> HashSet<PropertyPath> {
        let mut references = HashSet::new();
        self.collect_references_into(&mut references);
        references
    }

    /// 将属性路径引用收集到给定集合。
    /// Collect property path references into the given set.
    pub fn collect_references_into(&self, references: &mut HashSet<PropertyPath>) {
        match self {
            Self::Reference(path) => {
                references.insert(path.clone());
            }
            Self::SymbolReference(symbol) => {
                if let Some(path) = property_path_from_owned_symbol(symbol) {
                    references.insert(path.clone());
                }
            }
            Self::Unary { operand, .. } => operand.collect_references_into(references),
            Self::Binary { left, right, .. } => {
                left.collect_references_into(references);
                right.collect_references_into(references);
            }
            Self::Function { arguments, .. } => {
                for argument in arguments {
                    argument.collect_references_into(references);
                }
            }
            Self::Constant(_) | Self::Custom { .. } => {}
        }
    }

    /// 获取标量表达式深度。
    /// Get scalar expression depth.
    pub fn depth(&self) -> usize {
        match self {
            Self::Constant(_)
            | Self::Reference(_)
            | Self::SymbolReference(_)
            | Self::Custom { .. } => 1,
            Self::Unary { operand, .. } => 1 + operand.depth(),
            Self::Binary { left, right, .. } => 1 + left.depth().max(right.depth()),
            Self::Function { arguments, .. } => {
                1 + arguments.iter().map(Self::depth).max().unwrap_or(0)
            }
        }
    }

    /// 获取结构键，用于表达式去重和排序。
    /// Get structural key for expression deduplication and sorting.
    pub fn structural_key(&self) -> String
    where
        T: Display,
    {
        scalar_structural_key(self)
    }
}

impl<T> From<T> for ScalarExpression<T> {
    fn from(value: T) -> Self {
        Self::constant(value)
    }
}

macro_rules! impl_runtime_scalar_from_literal {
    ($($type:ty),* $(,)?) => {
        $(
            impl From<$type> for ScalarExpression<ExpressionValue> {
                fn from(value: $type) -> Self {
                    Self::constant(ExpressionValue::from(value))
                }
            }
        )*
    };
}

impl_runtime_scalar_from_literal!(
    bool, f32, f64, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize
);

impl From<&str> for ScalarExpression<ExpressionValue> {
    fn from(value: &str) -> Self {
        Self::constant(ExpressionValue::from(value))
    }
}

impl From<String> for ScalarExpression<ExpressionValue> {
    fn from(value: String) -> Self {
        Self::constant(ExpressionValue::from(value))
    }
}

impl<T: Display> Display for ScalarExpression<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Constant(value) => write!(f, "{}", value),
            Self::Reference(path) => write!(f, "{}", path),
            Self::SymbolReference(symbol) => write!(f, "{}", symbol),
            Self::Unary { operator, operand } => write!(f, "{}({})", operator.symbol(), operand),
            Self::Binary {
                operator,
                left,
                right,
            } => write!(f, "({} {} {})", left, operator.symbol(), right),
            Self::Function { name, arguments } => {
                write!(f, "{}(", name)?;
                for (index, argument) in arguments.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", argument)?;
                }
                write!(f, ")")
            }
            Self::Custom {
                payload,
                description,
            } => write!(f, "{}", description.as_deref().unwrap_or(payload)),
        }
    }
}

/// 布尔表达式。
/// Boolean expression.
#[derive(Debug, Clone, PartialEq)]
pub enum BooleanExpression<T> {
    /// 布尔常量 / Boolean constant
    Constant(Trivalent),
    /// 比较表达式 / Comparison expression
    Comparison {
        /// 操作符 / Operator
        operator: ComparisonOperator,
        /// 左操作数 / Left operand
        left: ScalarExpression<T>,
        /// 右操作数 / Right operand
        right: ScalarExpression<T>,
    },
    /// 集合成员判断 / Set membership expression
    In {
        /// 被检查的值 / Checked value
        value: ScalarExpression<T>,
        /// 候选值列表 / Candidate values
        candidates: Vec<ScalarExpression<T>>,
        /// 是否取反 / Whether negated
        negated: bool,
    },
    /// 模式匹配 / Pattern match
    PatternMatch {
        /// 被匹配的值 / Matched value
        value: ScalarExpression<T>,
        /// 模式表达式 / Pattern expression
        pattern: ScalarExpression<T>,
        /// 匹配模式 / Match mode
        mode: PatternMatchMode,
        /// 是否取反 / Whether negated
        negated: bool,
    },
    /// 空值检查 / Null check
    NullCheck {
        /// 属性路径 / Property path
        path: PropertyPath,
        /// 空值检查类型 / Null check type
        null_check_type: NullCheckType,
    },
    /// 逻辑与 / Logical AND
    And(Vec<BooleanExpression<T>>),
    /// 逻辑或 / Logical OR
    Or(Vec<BooleanExpression<T>>),
    /// 逻辑非 / Logical NOT
    Not(Box<BooleanExpression<T>>),
    /// 自定义表达式 / Custom expression
    Custom {
        /// 自定义载荷 / Custom payload
        payload: String,
        /// 可选描述 / Optional description
        description: Option<String>,
    },
}

impl<T> BooleanExpression<T> {
    /// 创建布尔常量表达式。
    /// Create boolean constant expression.
    pub const fn constant(value: Trivalent) -> Self {
        Self::Constant(value)
    }

    /// 创建 true 常量表达式。
    /// Create true constant expression.
    pub const fn true_constant() -> Self {
        Self::Constant(Trivalent::True)
    }

    /// 创建 false 常量表达式。
    /// Create false constant expression.
    pub const fn false_constant() -> Self {
        Self::Constant(Trivalent::False)
    }

    /// 创建 unknown 常量表达式。
    /// Create unknown constant expression.
    pub const fn unknown_constant() -> Self {
        Self::Constant(Trivalent::Unknown)
    }

    /// 创建比较表达式。
    /// Create comparison expression.
    pub fn comparison(
        operator: ComparisonOperator,
        left: ScalarExpression<T>,
        right: ScalarExpression<T>,
    ) -> Self {
        Self::Comparison {
            operator,
            left,
            right,
        }
    }

    /// 创建相等比较表达式。
    /// Create equal comparison expression.
    pub fn eq(left: ScalarExpression<T>, right: ScalarExpression<T>) -> Self {
        Self::comparison(ComparisonOperator::Eq, left, right)
    }

    /// 创建不等比较表达式。
    /// Create not-equal comparison expression.
    pub fn ne(left: ScalarExpression<T>, right: ScalarExpression<T>) -> Self {
        Self::comparison(ComparisonOperator::Ne, left, right)
    }

    /// 创建小于比较表达式。
    /// Create less-than comparison expression.
    pub fn lt(left: ScalarExpression<T>, right: ScalarExpression<T>) -> Self {
        Self::comparison(ComparisonOperator::Lt, left, right)
    }

    /// 创建小于等于比较表达式。
    /// Create less-than-or-equal comparison expression.
    pub fn le(left: ScalarExpression<T>, right: ScalarExpression<T>) -> Self {
        Self::comparison(ComparisonOperator::Le, left, right)
    }

    /// 创建大于比较表达式。
    /// Create greater-than comparison expression.
    pub fn gt(left: ScalarExpression<T>, right: ScalarExpression<T>) -> Self {
        Self::comparison(ComparisonOperator::Gt, left, right)
    }

    /// 创建大于等于比较表达式。
    /// Create greater-than-or-equal comparison expression.
    pub fn ge(left: ScalarExpression<T>, right: ScalarExpression<T>) -> Self {
        Self::comparison(ComparisonOperator::Ge, left, right)
    }

    /// 创建集合成员判断表达式。
    /// Create set membership expression.
    pub fn in_expr(
        value: ScalarExpression<T>,
        candidates: Vec<ScalarExpression<T>>,
        negated: bool,
    ) -> Self {
        Self::In {
            value,
            candidates,
            negated,
        }
    }

    /// 创建模式匹配表达式。
    /// Create pattern match expression.
    pub fn pattern_match(
        value: ScalarExpression<T>,
        pattern: ScalarExpression<T>,
        mode: PatternMatchMode,
        negated: bool,
    ) -> Self {
        Self::PatternMatch {
            value,
            pattern,
            mode,
            negated,
        }
    }

    /// 创建空值检查表达式。
    /// Create null check expression.
    pub fn null_check(path: impl Into<PropertyPath>, null_check_type: NullCheckType) -> Self {
        Self::NullCheck {
            path: path.into(),
            null_check_type,
        }
    }

    /// 创建 is null 表达式。
    /// Create is-null expression.
    pub fn is_null(path: impl Into<PropertyPath>) -> Self {
        Self::null_check(path, NullCheckType::IsNull)
    }

    /// 创建 is not null 表达式。
    /// Create is-not-null expression.
    pub fn is_not_null(path: impl Into<PropertyPath>) -> Self {
        Self::null_check(path, NullCheckType::IsNotNull)
    }

    /// 创建逻辑与表达式。
    /// Create logical AND expression.
    pub fn and(operands: Vec<Self>) -> Self {
        assert!(
            !operands.is_empty(),
            "And expression requires at least one operand"
        );
        Self::And(operands)
    }

    /// 创建逻辑或表达式。
    /// Create logical OR expression.
    pub fn or(operands: Vec<Self>) -> Self {
        assert!(
            !operands.is_empty(),
            "Or expression requires at least one operand"
        );
        Self::Or(operands)
    }

    /// 创建逻辑非表达式。
    /// Create logical NOT expression.
    pub fn not_expr(operand: Self) -> Self {
        Self::Not(Box::new(operand))
    }

    /// 创建自定义表达式。
    /// Create custom expression.
    pub fn custom(payload: impl Into<String>, description: Option<String>) -> Self {
        Self::Custom {
            payload: payload.into(),
            description,
        }
    }

    /// 获取表达式类型名。
    /// Get expression type name.
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Constant(_) => "BooleanConstant",
            Self::Comparison { .. } => "Comparison",
            Self::In { negated, .. } => {
                if *negated {
                    "NotIn"
                } else {
                    "In"
                }
            }
            Self::PatternMatch { negated, .. } => {
                if *negated {
                    "NotPatternMatch"
                } else {
                    "PatternMatch"
                }
            }
            Self::NullCheck { .. } => "NullCheck",
            Self::And(_) => "And",
            Self::Or(_) => "Or",
            Self::Not(_) => "Not",
            Self::Custom { .. } => "Custom",
        }
    }

    /// 判断表达式是否是常量表达式。
    /// Check whether the expression is constant.
    pub fn is_constant(&self) -> bool {
        match self {
            Self::Constant(_) => true,
            Self::Comparison { left, right, .. } => left.is_constant() && right.is_constant(),
            Self::In { value, .. } | Self::PatternMatch { value, .. } => value.is_constant(),
            Self::NullCheck { .. } | Self::Custom { .. } => false,
            Self::And(operands) | Self::Or(operands) => operands.iter().all(Self::is_constant),
            Self::Not(operand) => operand.is_constant(),
        }
    }

    /// 判断表达式是否是纯逻辑表达式。
    /// Check whether the expression is pure logical.
    pub fn is_pure_logical(&self) -> bool {
        match self {
            Self::Constant(_) => true,
            Self::Comparison { .. }
            | Self::In { .. }
            | Self::PatternMatch { .. }
            | Self::NullCheck { .. }
            | Self::Custom { .. } => false,
            Self::And(operands) | Self::Or(operands) => operands.iter().all(Self::is_pure_logical),
            Self::Not(operand) => operand.is_pure_logical(),
        }
    }

    /// 收集表达式中的属性路径引用。
    /// Collect property path references in the expression.
    pub fn collect_references(&self) -> HashSet<PropertyPath> {
        let mut references = HashSet::new();
        self.collect_references_into(&mut references);
        references
    }

    /// 将属性路径引用收集到给定集合。
    /// Collect property path references into the given set.
    pub fn collect_references_into(&self, references: &mut HashSet<PropertyPath>) {
        match self {
            Self::Comparison { left, right, .. } => {
                left.collect_references_into(references);
                right.collect_references_into(references);
            }
            Self::In {
                value, candidates, ..
            } => {
                value.collect_references_into(references);
                for candidate in candidates {
                    candidate.collect_references_into(references);
                }
            }
            Self::PatternMatch { value, pattern, .. } => {
                value.collect_references_into(references);
                pattern.collect_references_into(references);
            }
            Self::NullCheck { path, .. } => {
                references.insert(path.clone());
            }
            Self::And(operands) | Self::Or(operands) => {
                for operand in operands {
                    operand.collect_references_into(references);
                }
            }
            Self::Not(operand) => operand.collect_references_into(references),
            Self::Constant(_) | Self::Custom { .. } => {}
        }
    }

    /// 获取逻辑操作符数量。
    /// Get logical operator count.
    pub fn logical_operator_count(&self) -> usize {
        match self {
            Self::Constant(_)
            | Self::Comparison { .. }
            | Self::In { .. }
            | Self::PatternMatch { .. }
            | Self::NullCheck { .. }
            | Self::Custom { .. } => 0,
            Self::And(operands) | Self::Or(operands) => {
                operands.len()
                    + operands
                        .iter()
                        .map(Self::logical_operator_count)
                        .sum::<usize>()
            }
            Self::Not(operand) => 1 + operand.logical_operator_count(),
        }
    }

    /// 获取表达式深度。
    /// Get expression depth.
    pub fn depth(&self) -> usize {
        match self {
            Self::Constant(_)
            | Self::Comparison { .. }
            | Self::In { .. }
            | Self::PatternMatch { .. }
            | Self::NullCheck { .. }
            | Self::Custom { .. } => 1,
            Self::And(operands) | Self::Or(operands) => {
                1 + operands.iter().map(Self::depth).max().unwrap_or(0)
            }
            Self::Not(operand) => 1 + operand.depth(),
        }
    }

    /// 获取结构键，用于表达式去重和排序。
    /// Get structural key for expression deduplication and sorting.
    pub fn structural_key(&self) -> String
    where
        T: Display,
    {
        boolean_structural_key(self)
    }

    /// 使用默认配置规范化布尔表达式。
    /// Normalize boolean expression with default configuration.
    pub fn normalize(&self) -> Self
    where
        T: Clone + Display,
    {
        normalize_boolean_expression(self, NormalizeConfig::default())
    }

    /// 使用指定配置规范化布尔表达式。
    /// Normalize boolean expression with the specified configuration.
    pub fn normalize_with_config(&self, config: NormalizeConfig) -> Self
    where
        T: Clone + Display,
    {
        normalize_boolean_expression(self, config)
    }
}

impl<T> From<Trivalent> for BooleanExpression<T> {
    fn from(value: Trivalent) -> Self {
        Self::constant(value)
    }
}

impl<T> From<bool> for BooleanExpression<T> {
    fn from(value: bool) -> Self {
        Self::constant(Trivalent::from(value))
    }
}

impl<T> From<Option<bool>> for BooleanExpression<T> {
    fn from(value: Option<bool>) -> Self {
        Self::constant(Trivalent::from(value))
    }
}

impl<T> BitAnd for BooleanExpression<T> {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        and_pair(self, rhs)
    }
}

impl<T> BitOr for BooleanExpression<T> {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        or_pair(self, rhs)
    }
}

impl<T> StdNot for BooleanExpression<T> {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self::not_expr(self)
    }
}

impl<T: Display> Display for BooleanExpression<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Constant(value) => match value {
                Trivalent::True => write!(f, "true"),
                Trivalent::False => write!(f, "false"),
                Trivalent::Unknown => write!(f, "unknown"),
            },
            Self::Comparison {
                operator,
                left,
                right,
            } => write!(f, "{} {} {}", left, operator.symbol(), right),
            Self::In {
                value,
                candidates,
                negated,
            } => {
                write!(f, "{} {}in (", value, if *negated { "not " } else { "" })?;
                for (index, candidate) in candidates.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", candidate)?;
                }
                write!(f, ")")
            }
            Self::PatternMatch {
                value,
                pattern,
                mode,
                negated,
            } => write!(
                f,
                "{} {}{:?} {}",
                value,
                if *negated { "not " } else { "" },
                mode,
                pattern
            ),
            Self::NullCheck {
                path,
                null_check_type,
            } => write!(f, "{} {}", path, null_check_type.symbol()),
            Self::And(operands) => write_boolean_operands(f, operands, BooleanOperator::And),
            Self::Or(operands) => write_boolean_operands(f, operands, BooleanOperator::Or),
            Self::Not(operand) => write!(f, "not ({})", operand),
            Self::Custom {
                payload,
                description,
            } => write!(f, "{}", description.as_deref().unwrap_or(payload)),
        }
    }
}

fn write_boolean_operands<T: Display>(
    f: &mut Formatter<'_>,
    operands: &[BooleanExpression<T>],
    operator: BooleanOperator,
) -> std::fmt::Result {
    for (index, operand) in operands.iter().enumerate() {
        if index > 0 {
            write!(f, " {} ", operator.symbol())?;
        }
        write!(f, "({})", operand)?;
    }
    Ok(())
}

/// 标准标量函数名称。
/// Standard scalar function names.
pub struct ScalarFunctionNames;

impl ScalarFunctionNames {
    /// 绝对值函数 / Absolute value function
    pub const ABS: &'static str = "abs";
    /// 小写函数 / Lowercase function
    pub const LOWER: &'static str = "lower";
    /// 大写函数 / Uppercase function
    pub const UPPER: &'static str = "upper";
    /// 裁剪函数 / Trim function
    pub const TRIM: &'static str = "trim";
    /// 长度函数 / Length function
    pub const LENGTH: &'static str = "length";
    /// 合并空值函数 / Coalesce function
    pub const COALESCE: &'static str = "coalesce";
}

/// 标量表达式构建扩展。
/// Scalar expression builder extension.
pub trait ScalarExpressionDsl<T>: Sized {
    /// 创建标量比较表达式。
    /// Create scalar comparison expression.
    fn compare_expr(
        self,
        operator: ComparisonOperator,
        value: impl Into<ScalarExpression<T>>,
    ) -> BooleanExpression<T>;

    /// 创建相等比较表达式。
    /// Create equal comparison expression.
    fn eq_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T>;

    /// 创建不等比较表达式。
    /// Create not-equal comparison expression.
    fn ne_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T>;

    /// 创建小于比较表达式。
    /// Create less-than comparison expression.
    fn lt_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T>;

    /// 创建小于等于比较表达式。
    /// Create less-than-or-equal comparison expression.
    fn le_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T>;

    /// 创建大于比较表达式。
    /// Create greater-than comparison expression.
    fn gt_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T>;

    /// 创建大于等于比较表达式。
    /// Create greater-than-or-equal comparison expression.
    fn ge_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T>;
}

impl<T> ScalarExpressionDsl<T> for ScalarExpression<T> {
    fn compare_expr(
        self,
        operator: ComparisonOperator,
        value: impl Into<ScalarExpression<T>>,
    ) -> BooleanExpression<T> {
        BooleanExpression::comparison(operator, self, value.into())
    }

    fn eq_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare_expr(ComparisonOperator::Eq, value)
    }

    fn ne_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare_expr(ComparisonOperator::Ne, value)
    }

    fn lt_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare_expr(ComparisonOperator::Lt, value)
    }

    fn le_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare_expr(ComparisonOperator::Le, value)
    }

    fn gt_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare_expr(ComparisonOperator::Gt, value)
    }

    fn ge_expr(self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare_expr(ComparisonOperator::Ge, value)
    }
}

/// 路径表达式构建器。
/// Path expression builder.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathBuilder<T = ExpressionValue> {
    path: PropertyPath,
    _marker: PhantomData<fn() -> T>,
}

impl<T> PathBuilder<T> {
    /// 创建路径表达式构建器。
    /// Create a path expression builder.
    pub fn new(path: impl Into<PropertyPath>) -> Self {
        Self {
            path: path.into(),
            _marker: PhantomData,
        }
    }

    /// 解析路径表达式构建器。
    /// Parse a path expression builder.
    pub fn parse(path: impl AsRef<str>) -> Self {
        Self::new(PropertyPath::parse(path))
    }

    /// 切换为另一个标量值类型。
    /// Switch to another scalar value type.
    pub fn typed<U>(&self) -> PathBuilder<U> {
        PathBuilder::new(self.path.clone())
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

    /// 转换为标量引用表达式。
    /// Convert to scalar reference expression.
    pub fn as_scalar(&self) -> ScalarExpression<T> {
        ScalarExpression::reference(self.path.clone())
    }

    /// 创建比较表达式。
    /// Create comparison expression.
    pub fn compare(
        &self,
        operator: ComparisonOperator,
        value: impl Into<ScalarExpression<T>>,
    ) -> BooleanExpression<T> {
        BooleanExpression::comparison(operator, self.as_scalar(), value.into())
    }

    /// 创建相等比较表达式。
    /// Create equal comparison expression.
    pub fn eq(&self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare(ComparisonOperator::Eq, value)
    }

    /// 创建不等比较表达式。
    /// Create not-equal comparison expression.
    pub fn ne(&self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare(ComparisonOperator::Ne, value)
    }

    /// 创建小于比较表达式。
    /// Create less-than comparison expression.
    pub fn lt(&self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare(ComparisonOperator::Lt, value)
    }

    /// 创建小于等于比较表达式。
    /// Create less-than-or-equal comparison expression.
    pub fn le(&self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare(ComparisonOperator::Le, value)
    }

    /// 创建大于比较表达式。
    /// Create greater-than comparison expression.
    pub fn gt(&self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare(ComparisonOperator::Gt, value)
    }

    /// 创建大于等于比较表达式。
    /// Create greater-than-or-equal comparison expression.
    pub fn ge(&self, value: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.compare(ComparisonOperator::Ge, value)
    }

    /// 创建集合成员判断表达式。
    /// Create set membership expression.
    pub fn in_values<I, V>(&self, values: I) -> BooleanExpression<T>
    where
        I: IntoIterator<Item = V>,
        V: Into<ScalarExpression<T>>,
    {
        self.set_membership(values, false)
    }

    /// 创建非集合成员判断表达式。
    /// Create negated set membership expression.
    pub fn not_in_values<I, V>(&self, values: I) -> BooleanExpression<T>
    where
        I: IntoIterator<Item = V>,
        V: Into<ScalarExpression<T>>,
    {
        self.set_membership(values, true)
    }

    /// 创建空值检查表达式。
    /// Create is-null expression.
    pub fn is_null(&self) -> BooleanExpression<T> {
        BooleanExpression::is_null(self.path.clone())
    }

    /// 创建非空检查表达式。
    /// Create is-not-null expression.
    pub fn is_not_null(&self) -> BooleanExpression<T> {
        BooleanExpression::is_not_null(self.path.clone())
    }

    /// 创建模式匹配表达式。
    /// Create pattern match expression.
    pub fn pattern_match(
        &self,
        pattern: impl Into<ScalarExpression<T>>,
        mode: PatternMatchMode,
        negated: bool,
    ) -> BooleanExpression<T> {
        BooleanExpression::pattern_match(self.as_scalar(), pattern.into(), mode, negated)
    }

    /// 创建 LIKE 模式匹配表达式。
    /// Create LIKE pattern match expression.
    pub fn like(&self, pattern: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.pattern_match(pattern, PatternMatchMode::Like, false)
    }

    /// 创建精确模式匹配表达式。
    /// Create exact pattern match expression.
    pub fn like_exact(&self, pattern: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.pattern_match(pattern, PatternMatchMode::Exact, false)
    }

    /// 创建前缀模式匹配表达式。
    /// Create prefix pattern match expression.
    pub fn like_prefix(&self, pattern: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.pattern_match(pattern, PatternMatchMode::Prefix, false)
    }

    /// 创建后缀模式匹配表达式。
    /// Create suffix pattern match expression.
    pub fn like_suffix(&self, pattern: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.pattern_match(pattern, PatternMatchMode::Suffix, false)
    }

    /// 创建包含模式匹配表达式。
    /// Create contains pattern match expression.
    pub fn like_contains(&self, pattern: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.pattern_match(pattern, PatternMatchMode::Contains, false)
    }

    /// 创建正则模式匹配表达式。
    /// Create regex pattern match expression.
    pub fn regex(&self, pattern: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.pattern_match(pattern, PatternMatchMode::Regex, false)
    }

    /// 创建否定 LIKE 模式匹配表达式。
    /// Create negated LIKE pattern match expression.
    pub fn not_like(&self, pattern: impl Into<ScalarExpression<T>>) -> BooleanExpression<T> {
        self.pattern_match(pattern, PatternMatchMode::Like, true)
    }

    fn set_membership<I, V>(&self, values: I, negated: bool) -> BooleanExpression<T>
    where
        I: IntoIterator<Item = V>,
        V: Into<ScalarExpression<T>>,
    {
        BooleanExpression::in_expr(
            self.as_scalar(),
            values.into_iter().map(Into::into).collect(),
            negated,
        )
    }
}

impl<T> From<PathBuilder<T>> for ScalarExpression<T> {
    fn from(value: PathBuilder<T>) -> Self {
        ScalarExpression::reference(value.path)
    }
}

impl<T> From<&PathBuilder<T>> for ScalarExpression<T> {
    fn from(value: &PathBuilder<T>) -> Self {
        value.as_scalar()
    }
}

/// 布尔表达式组合扩展。
/// Boolean expression composition extension.
pub trait BooleanExpressionDsl<T>: Sized {
    /// 逻辑与组合。
    /// Compose with logical AND.
    fn and_expr(self, other: BooleanExpression<T>) -> BooleanExpression<T>;

    /// 逻辑或组合。
    /// Compose with logical OR.
    fn or_expr(self, other: BooleanExpression<T>) -> BooleanExpression<T>;

    /// 逻辑非组合。
    /// Compose with logical NOT.
    fn not_expr(self) -> BooleanExpression<T>;
}

impl<T> BooleanExpressionDsl<T> for BooleanExpression<T> {
    fn and_expr(self, other: BooleanExpression<T>) -> BooleanExpression<T> {
        and_pair(self, other)
    }

    fn or_expr(self, other: BooleanExpression<T>) -> BooleanExpression<T> {
        or_pair(self, other)
    }

    fn not_expr(self) -> BooleanExpression<T> {
        BooleanExpression::not_expr(self)
    }
}

/// 创建默认运行时路径构建器。
/// Create a default runtime path builder.
pub fn path(path: impl AsRef<str>) -> PathBuilder<ExpressionValue> {
    PathBuilder::parse(path)
}

/// 创建类型化路径构建器。
/// Create a typed path builder.
pub fn typed_path<T>(path: impl AsRef<str>) -> PathBuilder<T> {
    PathBuilder::parse(path)
}

/// 创建类型化标量路径引用。
/// Create a typed scalar path reference.
pub fn scalar_path<T>(path: impl Into<PropertyPath>) -> ScalarExpression<T> {
    ScalarExpression::reference(path)
}

/// 创建布尔常量表达式。
/// Create boolean constant expression.
pub fn bool_expr<T>(value: bool) -> BooleanExpression<T> {
    BooleanExpression::constant(Trivalent::from(value))
}

/// 创建三值布尔常量表达式。
/// Create trivalent boolean constant expression.
pub fn trivalent_expr<T>(value: impl Into<Trivalent>) -> BooleanExpression<T> {
    BooleanExpression::constant(value.into())
}

/// 使用闭包创建布尔表达式。
/// Create a boolean expression with a closure.
pub fn boolean_expression<T>(block: impl FnOnce() -> BooleanExpression<T>) -> BooleanExpression<T> {
    block()
}

/// 创建指定名称的标量函数表达式。
/// Create a scalar function expression with the given name.
pub fn scalar_function<T>(
    name: impl Into<String>,
    arguments: impl IntoIterator<Item = ScalarExpression<T>>,
) -> ScalarExpression<T> {
    ScalarExpression::function(name, arguments.into_iter().collect())
}

/// 创建绝对值函数表达式。
/// Create absolute-value function expression.
pub fn abs<T>(expression: impl Into<ScalarExpression<T>>) -> ScalarExpression<T> {
    scalar_function(ScalarFunctionNames::ABS, [expression.into()])
}

/// 创建小写函数表达式。
/// Create lowercase function expression.
pub fn lower<T>(expression: impl Into<ScalarExpression<T>>) -> ScalarExpression<T> {
    scalar_function(ScalarFunctionNames::LOWER, [expression.into()])
}

/// 创建大写函数表达式。
/// Create uppercase function expression.
pub fn upper<T>(expression: impl Into<ScalarExpression<T>>) -> ScalarExpression<T> {
    scalar_function(ScalarFunctionNames::UPPER, [expression.into()])
}

/// 创建裁剪函数表达式。
/// Create trim function expression.
pub fn trim<T>(expression: impl Into<ScalarExpression<T>>) -> ScalarExpression<T> {
    scalar_function(ScalarFunctionNames::TRIM, [expression.into()])
}

/// 创建长度函数表达式。
/// Create length function expression.
pub fn length<T>(expression: impl Into<ScalarExpression<T>>) -> ScalarExpression<T> {
    scalar_function(ScalarFunctionNames::LENGTH, [expression.into()])
}

/// 创建合并空值函数表达式。
/// Create coalesce function expression.
pub fn coalesce<T>(
    expressions: impl IntoIterator<Item = ScalarExpression<T>>,
) -> ScalarExpression<T> {
    scalar_function(ScalarFunctionNames::COALESCE, expressions)
}

/// 快速创建比较表达式。
/// Quickly create comparison expression.
pub fn compare<T>(
    path: impl Into<PropertyPath>,
    operator: ComparisonOperator,
    value: impl Into<ScalarExpression<T>>,
) -> BooleanExpression<T> {
    BooleanExpression::comparison(operator, ScalarExpression::reference(path), value.into())
}

/// 快速创建相等比较表达式。
/// Quickly create equal comparison expression.
pub fn eq<T>(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<T>>,
) -> BooleanExpression<T> {
    compare(path, ComparisonOperator::Eq, value)
}

/// 快速创建不等比较表达式。
/// Quickly create not-equal comparison expression.
pub fn ne<T>(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<T>>,
) -> BooleanExpression<T> {
    compare(path, ComparisonOperator::Ne, value)
}

/// 快速创建小于比较表达式。
/// Quickly create less-than comparison expression.
pub fn lt<T>(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<T>>,
) -> BooleanExpression<T> {
    compare(path, ComparisonOperator::Lt, value)
}

/// 快速创建小于等于比较表达式。
/// Quickly create less-than-or-equal comparison expression.
pub fn le<T>(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<T>>,
) -> BooleanExpression<T> {
    compare(path, ComparisonOperator::Le, value)
}

/// 快速创建大于比较表达式。
/// Quickly create greater-than comparison expression.
pub fn gt<T>(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<T>>,
) -> BooleanExpression<T> {
    compare(path, ComparisonOperator::Gt, value)
}

/// 快速创建大于等于比较表达式。
/// Quickly create greater-than-or-equal comparison expression.
pub fn ge<T>(
    path: impl Into<PropertyPath>,
    value: impl Into<ScalarExpression<T>>,
) -> BooleanExpression<T> {
    compare(path, ComparisonOperator::Ge, value)
}

/// 快速创建集合成员判断表达式。
/// Quickly create set membership expression.
pub fn in_expr<T, I, V>(path: impl Into<PropertyPath>, values: I) -> BooleanExpression<T>
where
    I: IntoIterator<Item = V>,
    V: Into<ScalarExpression<T>>,
{
    BooleanExpression::in_expr(
        ScalarExpression::reference(path),
        values.into_iter().map(Into::into).collect(),
        false,
    )
}

/// 快速创建非集合成员判断表达式。
/// Quickly create negated set membership expression.
pub fn not_in_expr<T, I, V>(path: impl Into<PropertyPath>, values: I) -> BooleanExpression<T>
where
    I: IntoIterator<Item = V>,
    V: Into<ScalarExpression<T>>,
{
    BooleanExpression::in_expr(
        ScalarExpression::reference(path),
        values.into_iter().map(Into::into).collect(),
        true,
    )
}

/// 快速创建默认运行时空值检查表达式。
/// Quickly create default runtime is-null expression.
pub fn is_null(path: impl Into<PropertyPath>) -> ParsedBooleanExpression {
    BooleanExpression::is_null(path)
}

/// 快速创建默认运行时非空检查表达式。
/// Quickly create default runtime is-not-null expression.
pub fn is_not_null(path: impl Into<PropertyPath>) -> ParsedBooleanExpression {
    BooleanExpression::is_not_null(path)
}

/// 快速创建逻辑与表达式。
/// Quickly create logical AND expression.
pub fn and<T, I>(expressions: I) -> BooleanExpression<T>
where
    I: IntoIterator<Item = BooleanExpression<T>>,
{
    let mut operands = Vec::new();
    for expression in expressions {
        if let BooleanExpression::And(items) = expression {
            operands.extend(items);
        } else {
            operands.push(expression);
        }
    }
    BooleanExpression::and(operands)
}

/// 快速创建逻辑或表达式。
/// Quickly create logical OR expression.
pub fn or<T, I>(expressions: I) -> BooleanExpression<T>
where
    I: IntoIterator<Item = BooleanExpression<T>>,
{
    let mut operands = Vec::new();
    for expression in expressions {
        if let BooleanExpression::Or(items) = expression {
            operands.extend(items);
        } else {
            operands.push(expression);
        }
    }
    BooleanExpression::or(operands)
}

/// 快速创建逻辑非表达式。
/// Quickly create logical NOT expression.
pub fn not_expr<T>(expression: BooleanExpression<T>) -> BooleanExpression<T> {
    BooleanExpression::not_expr(expression)
}

fn and_pair<T>(left: BooleanExpression<T>, right: BooleanExpression<T>) -> BooleanExpression<T> {
    let mut operands = Vec::new();
    if let BooleanExpression::And(items) = left {
        operands.extend(items);
    } else {
        operands.push(left);
    }
    if let BooleanExpression::And(items) = right {
        operands.extend(items);
    } else {
        operands.push(right);
    }
    BooleanExpression::And(operands)
}

fn or_pair<T>(left: BooleanExpression<T>, right: BooleanExpression<T>) -> BooleanExpression<T> {
    let mut operands = Vec::new();
    if let BooleanExpression::Or(items) = left {
        operands.extend(items);
    } else {
        operands.push(left);
    }
    if let BooleanExpression::Or(items) = right {
        operands.extend(items);
    } else {
        operands.push(right);
    }
    BooleanExpression::Or(operands)
}

/// 运行时表达式求值上下文。
/// Runtime expression evaluation context.
pub trait EvaluationContext {
    /// 获取指定属性路径的值。
    /// Get value at the specified property path.
    fn get(&self, path: &PropertyPath) -> Option<&ExpressionValue>;

    /// 检查指定属性路径是否存在。
    /// Check whether the specified property path exists.
    fn contains(&self, path: &PropertyPath) -> bool {
        self.get(path).is_some()
    }
}

/// 基于哈希表的运行时表达式求值上下文。
/// HashMap-based runtime expression evaluation context.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MapEvaluationContext {
    values: HashMap<PropertyPath, ExpressionValue>,
}

impl MapEvaluationContext {
    /// 从属性路径映射创建求值上下文。
    /// Create an evaluation context from a property-path map.
    pub fn from_path_map(values: HashMap<PropertyPath, ExpressionValue>) -> Self {
        Self { values }
    }

    /// 从字符串路径映射创建求值上下文。
    /// Create an evaluation context from a string-path map.
    pub fn from_string_map<I, K>(values: I) -> Self
    where
        I: IntoIterator<Item = (K, ExpressionValue)>,
        K: AsRef<str>,
    {
        Self {
            values: values
                .into_iter()
                .map(|(key, value)| (PropertyPath::parse(key.as_ref()), value))
                .collect(),
        }
    }

    /// 插入或替换一个路径值。
    /// Insert or replace a path value.
    pub fn insert(&mut self, path: impl Into<PropertyPath>, value: impl Into<ExpressionValue>) {
        self.values.insert(path.into(), value.into());
    }

    /// 获取底层值映射。
    /// Get underlying value map.
    pub fn values(&self) -> &HashMap<PropertyPath, ExpressionValue> {
        &self.values
    }
}

impl EvaluationContext for MapEvaluationContext {
    fn get(&self, path: &PropertyPath) -> Option<&ExpressionValue> {
        self.values.get(path)
    }
}

impl EvaluationContext for HashMap<PropertyPath, ExpressionValue> {
    fn get(&self, path: &PropertyPath) -> Option<&ExpressionValue> {
        self.get(path)
    }
}

/// 空运行时表达式求值上下文。
/// Empty runtime expression evaluation context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EmptyEvaluationContext;

impl EvaluationContext for EmptyEvaluationContext {
    fn get(&self, _path: &PropertyPath) -> Option<&ExpressionValue> {
        None
    }
}

/// 运行时表达式求值结果。
/// Runtime expression evaluation result.
pub type EvaluationResult = Trivalent;

/// 求值运行时布尔表达式。
/// Evaluate a runtime boolean expression.
pub fn evaluate_boolean(
    expression: &ParsedBooleanExpression,
    context: &impl EvaluationContext,
) -> EvaluationResult {
    match expression {
        BooleanExpression::Constant(value) => *value,
        BooleanExpression::Comparison {
            operator,
            left,
            right,
        } => {
            let Some(left) = evaluate_scalar(left, context) else {
                return Trivalent::Unknown;
            };
            let Some(right) = evaluate_scalar(right, context) else {
                return Trivalent::Unknown;
            };
            Trivalent::from(compare_expression_values(&left, &right, *operator))
        }
        BooleanExpression::In {
            value,
            candidates,
            negated,
        } => {
            let Some(value) = evaluate_scalar(value, context) else {
                return Trivalent::Unknown;
            };
            if value == ExpressionValue::Null {
                return Trivalent::Unknown;
            }

            let contains = candidates.iter().any(|candidate| {
                evaluate_scalar(candidate, context)
                    .filter(|candidate| *candidate != ExpressionValue::Null)
                    .is_some_and(|candidate| expression_values_equal(&value, &candidate))
            });
            Trivalent::from(if *negated { !contains } else { contains })
        }
        BooleanExpression::PatternMatch {
            value,
            pattern,
            mode,
            negated,
        } => {
            let Some(value) = evaluate_scalar(value, context) else {
                return Trivalent::Unknown;
            };
            let Some(pattern) = evaluate_scalar(pattern, context) else {
                return Trivalent::Unknown;
            };
            let Some(matches) = evaluate_pattern_match(&value, &pattern, *mode) else {
                return Trivalent::Unknown;
            };
            Trivalent::from(if *negated { !matches } else { matches })
        }
        BooleanExpression::NullCheck {
            path,
            null_check_type,
        } => {
            if !context.contains(path) {
                return Trivalent::Unknown;
            }
            let is_null = context
                .get(path)
                .is_none_or(|value| *value == ExpressionValue::Null);
            Trivalent::from(match null_check_type {
                NullCheckType::IsNull => is_null,
                NullCheckType::IsNotNull => !is_null,
            })
        }
        BooleanExpression::And(operands) => {
            let mut has_unknown = false;
            for operand in operands {
                match evaluate_boolean(operand, context) {
                    Trivalent::False => return Trivalent::False,
                    Trivalent::Unknown => has_unknown = true,
                    Trivalent::True => {}
                }
            }
            if has_unknown {
                Trivalent::Unknown
            } else {
                Trivalent::True
            }
        }
        BooleanExpression::Or(operands) => {
            let mut has_unknown = false;
            for operand in operands {
                match evaluate_boolean(operand, context) {
                    Trivalent::True => return Trivalent::True,
                    Trivalent::Unknown => has_unknown = true,
                    Trivalent::False => {}
                }
            }
            if has_unknown {
                Trivalent::Unknown
            } else {
                Trivalent::False
            }
        }
        BooleanExpression::Not(operand) => match evaluate_boolean(operand, context) {
            Trivalent::True => Trivalent::False,
            Trivalent::False => Trivalent::True,
            Trivalent::Unknown => Trivalent::Unknown,
        },
        BooleanExpression::Custom { .. } => Trivalent::Unknown,
    }
}

/// 求值运行时布尔表达式，并以可空布尔值返回结果。
/// Evaluate a runtime boolean expression and return a nullable boolean result.
pub fn evaluate_boolean_or_none(
    expression: &ParsedBooleanExpression,
    context: &impl EvaluationContext,
) -> Option<bool> {
    evaluate_boolean(expression, context).into()
}

/// 运行时布尔表达式求值扩展。
/// Runtime boolean expression evaluation extension.
pub trait EvaluateBoolean {
    /// 求值表达式。
    /// Evaluate expression.
    fn evaluate_with(&self, context: &impl EvaluationContext) -> Trivalent;

    /// 求值表达式，并以可空布尔值返回结果。
    /// Evaluate expression and return a nullable boolean result.
    fn evaluate_with_or_none(&self, context: &impl EvaluationContext) -> Option<bool>;
}

impl EvaluateBoolean for ParsedBooleanExpression {
    fn evaluate_with(&self, context: &impl EvaluationContext) -> Trivalent {
        evaluate_boolean(self, context)
    }

    fn evaluate_with_or_none(&self, context: &impl EvaluationContext) -> Option<bool> {
        evaluate_boolean_or_none(self, context)
    }
}

fn evaluate_scalar(
    expression: &ParsedScalarExpression,
    context: &impl EvaluationContext,
) -> Option<ExpressionValue> {
    match expression {
        ScalarExpression::Constant(value) => Some(value.clone()),
        ScalarExpression::Reference(path) => context.get(path).cloned(),
        ScalarExpression::SymbolReference(symbol) => {
            property_path_from_owned_symbol(symbol).and_then(|path| context.get(path).cloned())
        }
        ScalarExpression::Unary { operator, operand } => {
            let operand = evaluate_scalar(operand, context)?;
            evaluate_unary_expression_value(*operator, operand)
        }
        ScalarExpression::Binary {
            operator,
            left,
            right,
        } => {
            let left = evaluate_scalar(left, context)?;
            let right = evaluate_scalar(right, context)?;
            evaluate_binary_expression_value(*operator, left, right)
        }
        ScalarExpression::Function { name, arguments } => {
            let arguments = arguments
                .iter()
                .map(|argument| evaluate_scalar(argument, context))
                .collect::<Vec<_>>();
            evaluate_scalar_function(name, &arguments)
        }
        ScalarExpression::Custom { .. } => None,
    }
}

fn evaluate_unary_expression_value(
    operator: UnaryOperator,
    value: ExpressionValue,
) -> Option<ExpressionValue> {
    match operator {
        UnaryOperator::Positive => Some(value),
        UnaryOperator::Negate => Some(ExpressionValue::Number(-expression_number(&value)?)),
        UnaryOperator::Abs => Some(ExpressionValue::Number(expression_number(&value)?.abs())),
    }
}

fn evaluate_binary_expression_value(
    operator: BinaryOperator,
    left: ExpressionValue,
    right: ExpressionValue,
) -> Option<ExpressionValue> {
    let left = expression_number(&left)?;
    let right = expression_number(&right)?;
    let value = match operator {
        BinaryOperator::Add => left + right,
        BinaryOperator::Subtract => left - right,
        BinaryOperator::Multiply => left * right,
        BinaryOperator::Divide => {
            if right == 0.0 {
                return None;
            }
            left / right
        }
        BinaryOperator::Modulo => {
            if right == 0.0 {
                return None;
            }
            left % right
        }
        BinaryOperator::Power => left.powf(right),
    };
    value.is_finite().then_some(ExpressionValue::Number(value))
}

fn evaluate_scalar_function(
    name: &str,
    arguments: &[Option<ExpressionValue>],
) -> Option<ExpressionValue> {
    match name.to_ascii_lowercase().as_str() {
        ScalarFunctionNames::ABS => {
            let value = arguments.first()?.as_ref()?;
            Some(ExpressionValue::Number(expression_number(value)?.abs()))
        }
        ScalarFunctionNames::LOWER => evaluate_string_unary(arguments, |value| {
            ExpressionValue::String(value.to_ascii_lowercase())
        }),
        ScalarFunctionNames::UPPER => evaluate_string_unary(arguments, |value| {
            ExpressionValue::String(value.to_ascii_uppercase())
        }),
        ScalarFunctionNames::TRIM => evaluate_string_unary(arguments, |value| {
            ExpressionValue::String(value.trim().to_string())
        }),
        ScalarFunctionNames::LENGTH => evaluate_string_unary(arguments, |value| {
            ExpressionValue::Number(value.chars().count() as f64)
        }),
        ScalarFunctionNames::COALESCE => arguments
            .iter()
            .flatten()
            .find(|value| **value != ExpressionValue::Null)
            .cloned(),
        _ => None,
    }
}

fn evaluate_string_unary(
    arguments: &[Option<ExpressionValue>],
    operation: impl FnOnce(&str) -> ExpressionValue,
) -> Option<ExpressionValue> {
    let value = arguments.first()?.as_ref()?;
    match value {
        ExpressionValue::String(value) => Some(operation(value)),
        _ => None,
    }
}

fn expression_number(value: &ExpressionValue) -> Option<f64> {
    match value {
        ExpressionValue::Number(value) if value.is_finite() => Some(*value),
        _ => None,
    }
}

fn compare_expression_values(
    left: &ExpressionValue,
    right: &ExpressionValue,
    operator: ComparisonOperator,
) -> Option<bool> {
    match operator {
        ComparisonOperator::Eq => Some(expression_values_equal(left, right)),
        ComparisonOperator::Ne => Some(!expression_values_equal(left, right)),
        ComparisonOperator::Lt => compare_expression_order(left, right).map(|order| order < 0),
        ComparisonOperator::Le => compare_expression_order(left, right).map(|order| order <= 0),
        ComparisonOperator::Gt => compare_expression_order(left, right).map(|order| order > 0),
        ComparisonOperator::Ge => compare_expression_order(left, right).map(|order| order >= 0),
    }
}

fn compare_expression_order(left: &ExpressionValue, right: &ExpressionValue) -> Option<i8> {
    match (left, right) {
        (ExpressionValue::Number(left), ExpressionValue::Number(right))
            if left.is_finite() && right.is_finite() =>
        {
            left.partial_cmp(right)
                .map(|order| order as i8)
                .or_else(|| Some(0))
        }
        (ExpressionValue::String(left), ExpressionValue::String(right)) => {
            Some(left.cmp(right) as i8)
        }
        (ExpressionValue::Boolean(left), ExpressionValue::Boolean(right)) => {
            Some(left.cmp(right) as i8)
        }
        _ => None,
    }
}

fn expression_values_equal(left: &ExpressionValue, right: &ExpressionValue) -> bool {
    match (left, right) {
        (ExpressionValue::Number(left), ExpressionValue::Number(right)) => left == right,
        _ => left == right,
    }
}

fn evaluate_pattern_match(
    value: &ExpressionValue,
    pattern: &ExpressionValue,
    mode: PatternMatchMode,
) -> Option<bool> {
    if *value == ExpressionValue::Null || *pattern == ExpressionValue::Null {
        return None;
    }
    let value = expression_value_to_match_text(value);
    let pattern = expression_value_to_match_text(pattern);
    match mode {
        PatternMatchMode::Exact => Some(value == pattern),
        PatternMatchMode::Prefix => Some(value.starts_with(&pattern)),
        PatternMatchMode::Suffix => Some(value.ends_with(&pattern)),
        PatternMatchMode::Contains => Some(value.contains(&pattern)),
        PatternMatchMode::Like => Some(match_like(&value, &pattern)),
        PatternMatchMode::Regex => regex::Regex::new(&pattern)
            .ok()
            .map(|regex| regex.is_match(&value)),
    }
}

fn expression_value_to_match_text(value: &ExpressionValue) -> String {
    match value {
        ExpressionValue::Null => String::new(),
        ExpressionValue::Boolean(value) => value.to_string(),
        ExpressionValue::Number(value) => value.to_string(),
        ExpressionValue::String(value) => value.clone(),
    }
}

fn match_like(value: &str, pattern: &str) -> bool {
    let mut regex_pattern = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '%' => regex_pattern.push_str(".*"),
            '_' => regex_pattern.push('.'),
            _ => regex_pattern.push_str(&regex::escape(&ch.to_string())),
        }
    }
    regex_pattern.push('$');
    regex::Regex::new(&regex_pattern)
        .map(|regex| regex.is_match(value))
        .unwrap_or(false)
}

/// 布尔表达式规范化配置。
/// Boolean expression normalization configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormalizeConfig {
    /// 是否扁平化 And/Or。
    /// Whether to flatten And/Or.
    pub flatten: bool,
    /// 是否进行常量折叠。
    /// Whether to perform constant folding.
    pub constant_folding: bool,
    /// 是否去重。
    /// Whether to deduplicate.
    pub deduplicate: bool,
    /// 是否消除双重否定。
    /// Whether to eliminate double negation.
    pub eliminate_double_negation: bool,
    /// 是否应用德摩根定律。
    /// Whether to apply De Morgan's laws.
    pub apply_de_morgan: bool,
    /// 是否按结构键排序操作数。
    /// Whether to sort operands by structural key.
    pub sort_operands: bool,
}

impl Default for NormalizeConfig {
    fn default() -> Self {
        Self {
            flatten: true,
            constant_folding: true,
            deduplicate: true,
            eliminate_double_negation: true,
            apply_de_morgan: false,
            sort_operands: false,
        }
    }
}

/// 规范化布尔表达式。
/// Normalize boolean expression.
pub fn normalize_boolean_expression<T>(
    expression: &BooleanExpression<T>,
    config: NormalizeConfig,
) -> BooleanExpression<T>
where
    T: Clone + Display,
{
    let mut result = expression.clone();
    if config.flatten {
        result = flatten_boolean_expression(&result);
    }
    if config.eliminate_double_negation {
        result = eliminate_double_negation(&result);
    }
    if config.apply_de_morgan {
        result = apply_de_morgan(&result);
    }
    if config.constant_folding {
        result = constant_fold_boolean_expression(&result);
    }
    if config.deduplicate {
        result = deduplicate_boolean_expression(&result);
    }
    if config.sort_operands {
        result = sort_boolean_operands(&result);
    }
    result = normalize_boolean_children(&result, config);
    simplify_single_boolean_operand(&result)
}

/// 扁平化布尔表达式中的 And/Or。
/// Flatten And/Or in a boolean expression.
pub fn flatten_boolean_expression<T>(expression: &BooleanExpression<T>) -> BooleanExpression<T>
where
    T: Clone,
{
    match expression {
        BooleanExpression::And(operands) => {
            let mut flattened = Vec::new();
            for operand in operands {
                match flatten_boolean_expression(operand) {
                    BooleanExpression::And(items) => flattened.extend(items),
                    item => flattened.push(item),
                }
            }
            BooleanExpression::And(flattened)
        }
        BooleanExpression::Or(operands) => {
            let mut flattened = Vec::new();
            for operand in operands {
                match flatten_boolean_expression(operand) {
                    BooleanExpression::Or(items) => flattened.extend(items),
                    item => flattened.push(item),
                }
            }
            BooleanExpression::Or(flattened)
        }
        BooleanExpression::Not(operand) => {
            BooleanExpression::Not(Box::new(flatten_boolean_expression(operand)))
        }
        _ => expression.clone(),
    }
}

/// 常量折叠布尔表达式。
/// Constant-fold boolean expression.
pub fn constant_fold_boolean_expression<T>(
    expression: &BooleanExpression<T>,
) -> BooleanExpression<T>
where
    T: Clone,
{
    match expression {
        BooleanExpression::Constant(_) => expression.clone(),
        BooleanExpression::And(operands) => {
            let operands = operands
                .iter()
                .map(constant_fold_boolean_expression)
                .collect::<Vec<_>>();
            if operands
                .iter()
                .any(|operand| matches!(operand, BooleanExpression::Constant(Trivalent::False)))
            {
                return BooleanExpression::Constant(Trivalent::False);
            }
            let filtered = operands
                .into_iter()
                .filter(|operand| !matches!(operand, BooleanExpression::Constant(Trivalent::True)))
                .collect::<Vec<_>>();
            match filtered.len() {
                0 => BooleanExpression::Constant(Trivalent::True),
                1 => filtered.into_iter().next().unwrap(),
                _ => BooleanExpression::And(filtered),
            }
        }
        BooleanExpression::Or(operands) => {
            let operands = operands
                .iter()
                .map(constant_fold_boolean_expression)
                .collect::<Vec<_>>();
            if operands
                .iter()
                .any(|operand| matches!(operand, BooleanExpression::Constant(Trivalent::True)))
            {
                return BooleanExpression::Constant(Trivalent::True);
            }
            let filtered = operands
                .into_iter()
                .filter(|operand| !matches!(operand, BooleanExpression::Constant(Trivalent::False)))
                .collect::<Vec<_>>();
            match filtered.len() {
                0 => BooleanExpression::Constant(Trivalent::False),
                1 => filtered.into_iter().next().unwrap(),
                _ => BooleanExpression::Or(filtered),
            }
        }
        BooleanExpression::Not(operand) => match constant_fold_boolean_expression(operand) {
            BooleanExpression::Constant(value) => BooleanExpression::Constant(match value {
                Trivalent::True => Trivalent::False,
                Trivalent::False => Trivalent::True,
                Trivalent::Unknown => Trivalent::Unknown,
            }),
            operand => BooleanExpression::Not(Box::new(operand)),
        },
        _ => expression.clone(),
    }
}

/// 对布尔表达式操作数去重。
/// Deduplicate boolean expression operands.
pub fn deduplicate_boolean_expression<T>(expression: &BooleanExpression<T>) -> BooleanExpression<T>
where
    T: Clone + Display,
{
    match expression {
        BooleanExpression::And(operands) => {
            BooleanExpression::And(deduplicate_boolean_operands(operands))
        }
        BooleanExpression::Or(operands) => {
            BooleanExpression::Or(deduplicate_boolean_operands(operands))
        }
        BooleanExpression::Not(operand) => {
            BooleanExpression::Not(Box::new(deduplicate_boolean_expression(operand)))
        }
        _ => expression.clone(),
    }
}

/// 消除布尔表达式中的双重否定。
/// Eliminate double negation in a boolean expression.
pub fn eliminate_double_negation<T>(expression: &BooleanExpression<T>) -> BooleanExpression<T>
where
    T: Clone,
{
    match expression {
        BooleanExpression::Not(operand) => match eliminate_double_negation(operand) {
            BooleanExpression::Not(inner) => *inner,
            operand => BooleanExpression::Not(Box::new(operand)),
        },
        BooleanExpression::And(operands) => {
            BooleanExpression::And(operands.iter().map(eliminate_double_negation).collect())
        }
        BooleanExpression::Or(operands) => {
            BooleanExpression::Or(operands.iter().map(eliminate_double_negation).collect())
        }
        _ => expression.clone(),
    }
}

/// 对布尔表达式应用德摩根定律。
/// Apply De Morgan's laws to a boolean expression.
pub fn apply_de_morgan<T>(expression: &BooleanExpression<T>) -> BooleanExpression<T>
where
    T: Clone,
{
    match expression {
        BooleanExpression::Not(operand) => match apply_de_morgan(operand) {
            BooleanExpression::And(operands) => BooleanExpression::Or(
                operands
                    .into_iter()
                    .map(|operand| BooleanExpression::Not(Box::new(operand)))
                    .collect(),
            ),
            BooleanExpression::Or(operands) => BooleanExpression::And(
                operands
                    .into_iter()
                    .map(|operand| BooleanExpression::Not(Box::new(operand)))
                    .collect(),
            ),
            operand => BooleanExpression::Not(Box::new(operand)),
        },
        BooleanExpression::And(operands) => {
            BooleanExpression::And(operands.iter().map(apply_de_morgan).collect())
        }
        BooleanExpression::Or(operands) => {
            BooleanExpression::Or(operands.iter().map(apply_de_morgan).collect())
        }
        _ => expression.clone(),
    }
}

/// 按结构键排序布尔表达式操作数。
/// Sort boolean expression operands by structural key.
pub fn sort_boolean_operands<T>(expression: &BooleanExpression<T>) -> BooleanExpression<T>
where
    T: Clone + Display,
{
    match expression {
        BooleanExpression::And(operands) => {
            let mut operands = operands
                .iter()
                .map(sort_boolean_operands)
                .collect::<Vec<_>>();
            operands.sort_by_key(boolean_structural_key);
            BooleanExpression::And(operands)
        }
        BooleanExpression::Or(operands) => {
            let mut operands = operands
                .iter()
                .map(sort_boolean_operands)
                .collect::<Vec<_>>();
            operands.sort_by_key(boolean_structural_key);
            BooleanExpression::Or(operands)
        }
        BooleanExpression::Not(operand) => {
            BooleanExpression::Not(Box::new(sort_boolean_operands(operand)))
        }
        _ => expression.clone(),
    }
}

fn deduplicate_boolean_operands<T>(operands: &[BooleanExpression<T>]) -> Vec<BooleanExpression<T>>
where
    T: Clone + Display,
{
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for operand in operands {
        let operand = deduplicate_boolean_expression(operand);
        if seen.insert(boolean_structural_key(&operand)) {
            result.push(operand);
        }
    }
    result
}

fn normalize_boolean_children<T>(
    expression: &BooleanExpression<T>,
    config: NormalizeConfig,
) -> BooleanExpression<T>
where
    T: Clone + Display,
{
    match expression {
        BooleanExpression::And(operands) => BooleanExpression::And(
            operands
                .iter()
                .map(|operand| normalize_boolean_expression(operand, config))
                .collect(),
        ),
        BooleanExpression::Or(operands) => BooleanExpression::Or(
            operands
                .iter()
                .map(|operand| normalize_boolean_expression(operand, config))
                .collect(),
        ),
        BooleanExpression::Not(operand) => {
            BooleanExpression::Not(Box::new(normalize_boolean_expression(operand, config)))
        }
        _ => expression.clone(),
    }
}

fn simplify_single_boolean_operand<T>(expression: &BooleanExpression<T>) -> BooleanExpression<T>
where
    T: Clone,
{
    match expression {
        BooleanExpression::And(operands) => match operands.as_slice() {
            [] => BooleanExpression::Constant(Trivalent::True),
            [operand] => simplify_single_boolean_operand(operand),
            _ => BooleanExpression::And(
                operands
                    .iter()
                    .map(simplify_single_boolean_operand)
                    .collect(),
            ),
        },
        BooleanExpression::Or(operands) => match operands.as_slice() {
            [] => BooleanExpression::Constant(Trivalent::False),
            [operand] => simplify_single_boolean_operand(operand),
            _ => BooleanExpression::Or(
                operands
                    .iter()
                    .map(simplify_single_boolean_operand)
                    .collect(),
            ),
        },
        BooleanExpression::Not(operand) => {
            BooleanExpression::Not(Box::new(simplify_single_boolean_operand(operand)))
        }
        _ => expression.clone(),
    }
}

fn boolean_structural_key<T>(expression: &BooleanExpression<T>) -> String
where
    T: Display,
{
    match expression {
        BooleanExpression::Constant(value) => format!("Const:{value:?}"),
        BooleanExpression::Comparison {
            operator,
            left,
            right,
        } => format!(
            "Cmp:{operator:?}:{}:{}",
            scalar_structural_key(left),
            scalar_structural_key(right)
        ),
        BooleanExpression::In {
            value,
            candidates,
            negated,
        } => format!(
            "In:{negated}:{}:{}",
            scalar_structural_key(value),
            candidates
                .iter()
                .map(scalar_structural_key)
                .collect::<Vec<_>>()
                .join(",")
        ),
        BooleanExpression::PatternMatch {
            value,
            pattern,
            mode,
            negated,
        } => format!(
            "Match:{mode:?}:{negated}:{}:{}",
            scalar_structural_key(value),
            scalar_structural_key(pattern)
        ),
        BooleanExpression::NullCheck {
            path,
            null_check_type,
        } => format!("Null:{null_check_type:?}:{path}"),
        BooleanExpression::And(operands) => format!(
            "And:{}",
            operands
                .iter()
                .map(boolean_structural_key)
                .collect::<Vec<_>>()
                .join(",")
        ),
        BooleanExpression::Or(operands) => format!(
            "Or:{}",
            operands
                .iter()
                .map(boolean_structural_key)
                .collect::<Vec<_>>()
                .join(",")
        ),
        BooleanExpression::Not(operand) => format!("Not:{}", boolean_structural_key(operand)),
        BooleanExpression::Custom {
            payload,
            description,
        } => format!("Custom:{}", description.as_deref().unwrap_or(payload)),
    }
}

fn scalar_structural_key<T>(expression: &ScalarExpression<T>) -> String
where
    T: Display,
{
    match expression {
        ScalarExpression::Constant(value) => format!("Const:{value}"),
        ScalarExpression::Reference(path) => format!("Ref:{path}"),
        ScalarExpression::SymbolReference(symbol) => format!("SymRef:{}", symbol.name()),
        ScalarExpression::Unary { operator, operand } => {
            format!("Unary:{operator:?}:{}", scalar_structural_key(operand))
        }
        ScalarExpression::Binary {
            operator,
            left,
            right,
        } => format!(
            "Bin:{operator:?}:{}:{}",
            scalar_structural_key(left),
            scalar_structural_key(right)
        ),
        ScalarExpression::Function { name, arguments } => format!(
            "Func:{name}:{}",
            arguments
                .iter()
                .map(scalar_structural_key)
                .collect::<Vec<_>>()
                .join(",")
        ),
        ScalarExpression::Custom {
            payload,
            description,
        } => format!("Custom:{}", description.as_deref().unwrap_or(payload)),
    }
}

#[cfg(feature = "parser")]
mod parser_support {
    use super::*;

    /// 表达式解析错误。
    /// Expression parse error.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ExpressionParseError {
        message: String,
        position: usize,
    }

    impl ExpressionParseError {
        /// 创建表达式解析错误。
        /// Create an expression parse error.
        pub fn new(message: impl Into<String>, position: usize) -> Self {
            Self {
                message: message.into(),
                position,
            }
        }

        /// 获取错误消息。
        /// Get error message.
        pub fn message(&self) -> &str {
            &self.message
        }

        /// 获取错误位置。
        /// Get error position.
        pub fn position(&self) -> usize {
            self.position
        }
    }

    impl Display for ExpressionParseError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{} at {}", self.message, self.position)
        }
    }

    impl std::error::Error for ExpressionParseError {}

    /// 表达式词法单元类型。
    /// Expression token type.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum ExpressionTokenType {
        /// 真常量 / True constant
        True,
        /// 假常量 / False constant
        False,
        /// 空值 / Null value
        Null,
        /// 字符串字面量 / String literal
        String,
        /// 数字字面量 / Number literal
        Number,
        /// 标识符或属性路径 / Identifier or property path
        Identifier,
        /// 逻辑与 / Logical AND
        And,
        /// 逻辑或 / Logical OR
        Or,
        /// 逻辑非 / Logical NOT
        Not,
        /// 集合成员判断 / Set membership
        In,
        /// 空值判断关键字 / Null-check keyword
        Is,
        /// LIKE 模式匹配 / LIKE pattern match
        Like,
        /// 包含匹配 / Contains match
        Contains,
        /// 前缀匹配 / Prefix match
        Prefix,
        /// 后缀匹配 / Suffix match
        Suffix,
        /// 正则匹配 / Regex match
        Regex,
        /// 精确匹配 / Exact match
        Exact,
        /// 等于 / Equal
        Eq,
        /// 不等于 / Not equal
        Ne,
        /// 小于 / Less than
        Lt,
        /// 小于等于 / Less than or equal
        Le,
        /// 大于 / Greater than
        Gt,
        /// 大于等于 / Greater than or equal
        Ge,
        /// 左括号 / Left parenthesis
        LParen,
        /// 右括号 / Right parenthesis
        RParen,
        /// 逗号 / Comma
        Comma,
        /// 文件结束 / End of file
        Eof,
        /// 未知词法单元 / Unknown token
        Unknown,
    }

    impl ExpressionTokenType {
        fn comparison_operator(self) -> Option<ComparisonOperator> {
            match self {
                Self::Eq => Some(ComparisonOperator::Eq),
                Self::Ne => Some(ComparisonOperator::Ne),
                Self::Lt => Some(ComparisonOperator::Lt),
                Self::Le => Some(ComparisonOperator::Le),
                Self::Gt => Some(ComparisonOperator::Gt),
                Self::Ge => Some(ComparisonOperator::Ge),
                _ => None,
            }
        }

        fn pattern_match_mode(self) -> Option<PatternMatchMode> {
            match self {
                Self::Like => Some(PatternMatchMode::Like),
                Self::Contains => Some(PatternMatchMode::Contains),
                Self::Prefix => Some(PatternMatchMode::Prefix),
                Self::Suffix => Some(PatternMatchMode::Suffix),
                Self::Regex => Some(PatternMatchMode::Regex),
                Self::Exact => Some(PatternMatchMode::Exact),
                _ => None,
            }
        }

        fn is_comparison_operator(self) -> bool {
            self.comparison_operator().is_some()
        }

        fn is_pattern_operator(self) -> bool {
            self.pattern_match_mode().is_some()
        }
    }

    /// 表达式词法单元。
    /// Expression token.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ExpressionToken {
        /// 词法单元类型 / Token type
        pub token_type: ExpressionTokenType,
        /// 词法单元值 / Token value
        pub value: String,
        /// 词法单元位置 / Token position
        pub position: usize,
    }

    impl ExpressionToken {
        fn new(token_type: ExpressionTokenType, value: impl Into<String>, position: usize) -> Self {
            Self {
                token_type,
                value: value.into(),
                position,
            }
        }

        fn eof(position: usize) -> Self {
            Self::new(ExpressionTokenType::Eof, "", position)
        }

        fn unknown(value: impl Into<String>, position: usize) -> Self {
            Self::new(ExpressionTokenType::Unknown, value, position)
        }
    }

    /// 表达式词法分析器。
    /// Expression lexer.
    pub struct ExpressionLexer {
        input: Vec<char>,
        position: usize,
    }

    impl ExpressionLexer {
        /// 创建表达式词法分析器。
        /// Create an expression lexer.
        pub fn new(input: impl AsRef<str>) -> Self {
            Self {
                input: input.as_ref().chars().collect(),
                position: 0,
            }
        }

        /// 分析完整输入并返回词法单元列表。
        /// Tokenize the whole input and return token list.
        pub fn tokenize(&mut self) -> Vec<ExpressionToken> {
            let mut tokens = Vec::new();
            loop {
                let token = self.next_token();
                let is_eof = token.token_type == ExpressionTokenType::Eof;
                tokens.push(token);
                if is_eof {
                    break;
                }
            }
            tokens
        }

        /// 获取下一个词法单元。
        /// Get next token.
        pub fn next_token(&mut self) -> ExpressionToken {
            self.skip_whitespace();
            let start = self.position;
            let Some(current) = self.current_char() else {
                return ExpressionToken::eof(start);
            };

            if current == '\'' || current == '"' {
                return self.read_string(start);
            }
            if current.is_ascii_digit()
                || (current == '-'
                    && self
                        .peek_char(1)
                        .map(|ch| ch.is_ascii_digit())
                        .unwrap_or(false))
            {
                return self.read_number(start);
            }
            if current.is_alphabetic() || current == '_' {
                return self.read_identifier_or_keyword(start);
            }

            match current {
                '(' => {
                    self.advance();
                    ExpressionToken::new(ExpressionTokenType::LParen, "(", start)
                }
                ')' => {
                    self.advance();
                    ExpressionToken::new(ExpressionTokenType::RParen, ")", start)
                }
                ',' => {
                    self.advance();
                    ExpressionToken::new(ExpressionTokenType::Comma, ",", start)
                }
                '=' => {
                    self.advance();
                    ExpressionToken::new(ExpressionTokenType::Eq, "=", start)
                }
                '!' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        ExpressionToken::new(ExpressionTokenType::Ne, "!=", start)
                    } else {
                        ExpressionToken::unknown("!", start)
                    }
                }
                '<' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        ExpressionToken::new(ExpressionTokenType::Le, "<=", start)
                    } else if self.current_char() == Some('>') {
                        self.advance();
                        ExpressionToken::new(ExpressionTokenType::Ne, "<>", start)
                    } else {
                        ExpressionToken::new(ExpressionTokenType::Lt, "<", start)
                    }
                }
                '>' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        ExpressionToken::new(ExpressionTokenType::Ge, ">=", start)
                    } else {
                        ExpressionToken::new(ExpressionTokenType::Gt, ">", start)
                    }
                }
                _ => {
                    self.advance();
                    ExpressionToken::unknown(current.to_string(), start)
                }
            }
        }

        fn current_char(&self) -> Option<char> {
            self.input.get(self.position).copied()
        }

        fn peek_char(&self, offset: usize) -> Option<char> {
            self.input.get(self.position + offset).copied()
        }

        fn advance(&mut self) {
            self.position += 1;
        }

        fn skip_whitespace(&mut self) {
            while self
                .current_char()
                .map(|ch| ch.is_whitespace())
                .unwrap_or(false)
            {
                self.advance();
            }
        }

        fn read_string(&mut self, start: usize) -> ExpressionToken {
            let quote = self.current_char().unwrap_or('"');
            self.advance();
            let mut value = String::new();
            while let Some(current) = self.current_char() {
                if current == quote {
                    self.advance();
                    break;
                }
                if current == '\\' {
                    self.advance();
                    match self.current_char() {
                        Some('n') => value.push('\n'),
                        Some('t') => value.push('\t'),
                        Some('r') => value.push('\r'),
                        Some('\\') => value.push('\\'),
                        Some('\'') => value.push('\''),
                        Some('"') => value.push('"'),
                        Some(other) => {
                            value.push('\\');
                            value.push(other);
                        }
                        None => value.push('\\'),
                    }
                } else {
                    value.push(current);
                }
                self.advance();
            }
            ExpressionToken::new(ExpressionTokenType::String, value, start)
        }

        fn read_number(&mut self, start: usize) -> ExpressionToken {
            let mut value = String::new();
            if self.current_char() == Some('-') {
                value.push('-');
                self.advance();
            }
            while self
                .current_char()
                .map(|ch| ch.is_ascii_digit())
                .unwrap_or(false)
            {
                value.push(self.current_char().unwrap_or_default());
                self.advance();
            }
            if self.current_char() == Some('.')
                && self
                    .peek_char(1)
                    .map(|ch| ch.is_ascii_digit())
                    .unwrap_or(false)
            {
                value.push('.');
                self.advance();
                while self
                    .current_char()
                    .map(|ch| ch.is_ascii_digit())
                    .unwrap_or(false)
                {
                    value.push(self.current_char().unwrap_or_default());
                    self.advance();
                }
            }
            if matches!(self.current_char(), Some('e') | Some('E')) {
                value.push(self.current_char().unwrap_or_default());
                self.advance();
                if matches!(self.current_char(), Some('+') | Some('-')) {
                    value.push(self.current_char().unwrap_or_default());
                    self.advance();
                }
                while self
                    .current_char()
                    .map(|ch| ch.is_ascii_digit())
                    .unwrap_or(false)
                {
                    value.push(self.current_char().unwrap_or_default());
                    self.advance();
                }
            }
            ExpressionToken::new(ExpressionTokenType::Number, value, start)
        }

        fn read_identifier_or_keyword(&mut self, start: usize) -> ExpressionToken {
            let mut value = String::new();
            while self
                .current_char()
                .map(|ch| ch.is_alphanumeric() || ch == '_')
                .unwrap_or(false)
            {
                value.push(self.current_char().unwrap_or_default());
                self.advance();
            }
            while self.current_char() == Some('.')
                && self
                    .peek_char(1)
                    .map(|ch| ch.is_alphabetic() || ch == '_')
                    .unwrap_or(false)
            {
                value.push('.');
                self.advance();
                while self
                    .current_char()
                    .map(|ch| ch.is_alphanumeric() || ch == '_')
                    .unwrap_or(false)
                {
                    value.push(self.current_char().unwrap_or_default());
                    self.advance();
                }
            }

            let token_type = match value.to_ascii_lowercase().as_str() {
                "and" => ExpressionTokenType::And,
                "or" => ExpressionTokenType::Or,
                "not" => ExpressionTokenType::Not,
                "in" => ExpressionTokenType::In,
                "is" => ExpressionTokenType::Is,
                "like" => ExpressionTokenType::Like,
                "contains" => ExpressionTokenType::Contains,
                "prefix" => ExpressionTokenType::Prefix,
                "suffix" => ExpressionTokenType::Suffix,
                "regex" | "matches" => ExpressionTokenType::Regex,
                "exact" => ExpressionTokenType::Exact,
                "null" => ExpressionTokenType::Null,
                "true" => ExpressionTokenType::True,
                "false" => ExpressionTokenType::False,
                _ => ExpressionTokenType::Identifier,
            };
            ExpressionToken::new(token_type, value, start)
        }
    }

    /// 表达式解析器。
    /// Expression parser.
    pub struct ExpressionParser {
        tokens: Vec<ExpressionToken>,
        position: usize,
    }

    impl ExpressionParser {
        /// 创建表达式解析器。
        /// Create an expression parser.
        pub fn new(tokens: Vec<ExpressionToken>) -> Self {
            Self {
                tokens,
                position: 0,
            }
        }

        /// 解析布尔表达式。
        /// Parse a boolean expression.
        pub fn parse(&mut self) -> Result<ParsedBooleanExpression, ExpressionParseError> {
            if self.tokens.is_empty()
                || (self.tokens.len() == 1
                    && self.current_token().token_type == ExpressionTokenType::Eof)
            {
                return Err(ExpressionParseError::new("empty expression", 0));
            }
            let expression = self.parse_or_expression()?;
            if self.current_token().token_type != ExpressionTokenType::Eof {
                return Err(ExpressionParseError::new(
                    format!("unexpected token: {}", self.current_token().value),
                    self.current_token().position,
                ));
            }
            Ok(expression)
        }

        fn parse_or_expression(&mut self) -> Result<ParsedBooleanExpression, ExpressionParseError> {
            let mut left = self.parse_and_expression()?;
            while self.current_token().token_type == ExpressionTokenType::Or {
                self.advance();
                let right = self.parse_and_expression()?;
                left = merge_or(left, right);
            }
            Ok(left)
        }

        fn parse_and_expression(
            &mut self,
        ) -> Result<ParsedBooleanExpression, ExpressionParseError> {
            let mut left = self.parse_not_expression()?;
            while self.current_token().token_type == ExpressionTokenType::And {
                self.advance();
                let right = self.parse_not_expression()?;
                left = merge_and(left, right);
            }
            Ok(left)
        }

        fn parse_not_expression(
            &mut self,
        ) -> Result<ParsedBooleanExpression, ExpressionParseError> {
            if self.current_token().token_type == ExpressionTokenType::Not {
                self.advance();
                return Ok(BooleanExpression::not_expr(self.parse_not_expression()?));
            }
            self.parse_primary_expression()
        }

        fn parse_primary_expression(
            &mut self,
        ) -> Result<ParsedBooleanExpression, ExpressionParseError> {
            match self.current_token().token_type {
                ExpressionTokenType::LParen => {
                    self.advance();
                    let expression = self.parse_or_expression()?;
                    self.expect(ExpressionTokenType::RParen, "expected ')'")?;
                    Ok(expression)
                }
                ExpressionTokenType::True => {
                    self.advance();
                    Ok(BooleanExpression::true_constant())
                }
                ExpressionTokenType::False => {
                    self.advance();
                    Ok(BooleanExpression::false_constant())
                }
                ExpressionTokenType::Identifier => self.parse_path_expression(),
                _ => Err(ExpressionParseError::new(
                    format!("unexpected token: {}", self.current_token().value),
                    self.current_token().position,
                )),
            }
        }

        fn parse_path_expression(
            &mut self,
        ) -> Result<ParsedBooleanExpression, ExpressionParseError> {
            let path = self.parse_path()?;
            match self.current_token().token_type {
                ExpressionTokenType::Is => self.parse_null_check(path),
                ExpressionTokenType::Not => {
                    self.advance();
                    if self.current_token().token_type == ExpressionTokenType::In {
                        self.advance();
                        self.parse_in_expression(path, true)
                    } else if self.current_token().token_type.is_pattern_operator() {
                        let mode = self
                            .current_token()
                            .token_type
                            .pattern_match_mode()
                            .unwrap();
                        self.advance();
                        self.parse_pattern_match(path, mode, true)
                    } else {
                        Err(ExpressionParseError::new(
                            "expected 'in' or pattern operator after 'not'",
                            self.current_token().position,
                        ))
                    }
                }
                ExpressionTokenType::In => {
                    self.advance();
                    self.parse_in_expression(path, false)
                }
                token_type if token_type.is_pattern_operator() => {
                    let mode = token_type.pattern_match_mode().unwrap();
                    self.advance();
                    self.parse_pattern_match(path, mode, false)
                }
                token_type if token_type.is_comparison_operator() => self.parse_comparison(path),
                _ => Err(ExpressionParseError::new(
                    format!("expected comparison operator, 'in', or 'is' after '{path}'"),
                    self.current_token().position,
                )),
            }
        }

        fn parse_null_check(
            &mut self,
            path: PropertyPath,
        ) -> Result<ParsedBooleanExpression, ExpressionParseError> {
            self.advance();
            let null_check_type = if self.current_token().token_type == ExpressionTokenType::Not {
                self.advance();
                NullCheckType::IsNotNull
            } else {
                NullCheckType::IsNull
            };
            self.expect(ExpressionTokenType::Null, "expected 'null' after 'is'")?;
            Ok(BooleanExpression::null_check(path, null_check_type))
        }

        fn parse_in_expression(
            &mut self,
            path: PropertyPath,
            negated: bool,
        ) -> Result<ParsedBooleanExpression, ExpressionParseError> {
            self.expect(ExpressionTokenType::LParen, "expected '(' after 'in'")?;
            let mut candidates = Vec::new();
            loop {
                candidates.push(self.parse_scalar_value()?);
                if self.current_token().token_type != ExpressionTokenType::Comma {
                    break;
                }
                self.advance();
            }
            self.expect(ExpressionTokenType::RParen, "expected ')' after 'in' list")?;
            Ok(BooleanExpression::in_expr(
                ScalarExpression::reference(path),
                candidates,
                negated,
            ))
        }

        fn parse_pattern_match(
            &mut self,
            path: PropertyPath,
            mode: PatternMatchMode,
            negated: bool,
        ) -> Result<ParsedBooleanExpression, ExpressionParseError> {
            let pattern = self.parse_scalar_value()?;
            Ok(BooleanExpression::pattern_match(
                ScalarExpression::reference(path),
                pattern,
                mode,
                negated,
            ))
        }

        fn parse_comparison(
            &mut self,
            left_path: PropertyPath,
        ) -> Result<ParsedBooleanExpression, ExpressionParseError> {
            let operator = self
                .current_token()
                .token_type
                .comparison_operator()
                .ok_or_else(|| {
                    ExpressionParseError::new(
                        "expected comparison operator",
                        self.current_token().position,
                    )
                })?;
            self.advance();
            let right = self.parse_scalar_value()?;
            Ok(BooleanExpression::comparison(
                operator,
                ScalarExpression::reference(left_path),
                right,
            ))
        }

        fn parse_scalar_value(&mut self) -> Result<ParsedScalarExpression, ExpressionParseError> {
            match self.current_token().token_type {
                ExpressionTokenType::String => {
                    let value = self.current_token().value.clone();
                    self.advance();
                    Ok(ScalarExpression::constant(ExpressionValue::String(value)))
                }
                ExpressionTokenType::Number => {
                    let token = self.current_token();
                    self.advance();
                    let value = token.value.parse::<f64>().map_err(|_| {
                        ExpressionParseError::new("invalid number literal", token.position)
                    })?;
                    Ok(ScalarExpression::constant(ExpressionValue::Number(value)))
                }
                ExpressionTokenType::True => {
                    self.advance();
                    Ok(ScalarExpression::constant(ExpressionValue::Boolean(true)))
                }
                ExpressionTokenType::False => {
                    self.advance();
                    Ok(ScalarExpression::constant(ExpressionValue::Boolean(false)))
                }
                ExpressionTokenType::Null => {
                    self.advance();
                    Ok(ScalarExpression::constant(ExpressionValue::Null))
                }
                ExpressionTokenType::Identifier => {
                    Ok(ScalarExpression::reference(self.parse_path()?))
                }
                _ => Err(ExpressionParseError::new(
                    format!("expected scalar value, got: {}", self.current_token().value),
                    self.current_token().position,
                )),
            }
        }

        fn parse_path(&mut self) -> Result<PropertyPath, ExpressionParseError> {
            if self.current_token().token_type != ExpressionTokenType::Identifier {
                return Err(ExpressionParseError::new(
                    "expected identifier",
                    self.current_token().position,
                ));
            }
            let path = PropertyPath::parse(self.current_token().value.as_str());
            self.advance();
            Ok(path)
        }

        fn current_token(&self) -> ExpressionToken {
            self.tokens
                .get(self.position)
                .cloned()
                .unwrap_or_else(|| ExpressionToken::eof(self.position))
        }

        fn advance(&mut self) -> ExpressionToken {
            let token = self.current_token();
            self.position += 1;
            token
        }

        fn expect(
            &mut self,
            token_type: ExpressionTokenType,
            message: &'static str,
        ) -> Result<ExpressionToken, ExpressionParseError> {
            if self.current_token().token_type != token_type {
                return Err(ExpressionParseError::new(
                    message,
                    self.current_token().position,
                ));
            }
            Ok(self.advance())
        }
    }

    /// 将输入字符串切分为表达式词法单元。
    /// Tokenize input string into expression tokens.
    pub fn tokenize_expression(input: impl AsRef<str>) -> Vec<ExpressionToken> {
        ExpressionLexer::new(input).tokenize()
    }

    /// 解析布尔表达式字符串。
    /// Parse a boolean expression string.
    pub fn parse_boolean_expression(
        input: impl AsRef<str>,
    ) -> Result<ParsedBooleanExpression, ExpressionParseError> {
        let tokens = tokenize_expression(input);
        ExpressionParser::new(tokens).parse()
    }

    /// 尝试解析布尔表达式字符串。
    /// Try to parse a boolean expression string.
    pub fn parse_boolean_expression_or_none(
        input: impl AsRef<str>,
    ) -> Option<ParsedBooleanExpression> {
        parse_boolean_expression(input).ok()
    }

    fn merge_or(
        left: ParsedBooleanExpression,
        right: ParsedBooleanExpression,
    ) -> ParsedBooleanExpression {
        let mut operands = Vec::new();
        if let BooleanExpression::Or(items) = left {
            operands.extend(items);
        } else {
            operands.push(left);
        }
        if let BooleanExpression::Or(items) = right {
            operands.extend(items);
        } else {
            operands.push(right);
        }
        BooleanExpression::Or(operands)
    }

    fn merge_and(
        left: ParsedBooleanExpression,
        right: ParsedBooleanExpression,
    ) -> ParsedBooleanExpression {
        let mut operands = Vec::new();
        if let BooleanExpression::And(items) = left {
            operands.extend(items);
        } else {
            operands.push(left);
        }
        if let BooleanExpression::And(items) = right {
            operands.extend(items);
        } else {
            operands.push(right);
        }
        BooleanExpression::And(operands)
    }
}

#[cfg(feature = "parser")]
pub use parser_support::{
    ExpressionLexer, ExpressionParseError, ExpressionParser, ExpressionToken, ExpressionTokenType,
    parse_boolean_expression, parse_boolean_expression_or_none, tokenize_expression,
};

#[cfg(feature = "serde")]
mod serde_support {
    use super::*;
    use serde::de::DeserializeOwned;
    use serde::{Deserialize, Serialize};

    /// 表达式 JSON 错误。
    /// Expression JSON error.
    #[derive(Debug)]
    pub enum ExpressionJsonError {
        /// JSON 编码或解码失败。
        /// JSON encoding or decoding failed.
        Json(serde_json::Error),
        /// 操作符名称无效。
        /// Operator name is invalid.
        InvalidOperator {
            /// 操作符类别 / Operator kind
            kind: &'static str,
            /// 操作符名称 / Operator name
            value: String,
        },
    }

    impl Display for ExpressionJsonError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Json(error) => write!(f, "json error: {error}"),
                Self::InvalidOperator { kind, value } => {
                    write!(f, "invalid {kind} operator: {value}")
                }
            }
        }
    }

    impl std::error::Error for ExpressionJsonError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Self::Json(error) => Some(error),
                Self::InvalidOperator { .. } => None,
            }
        }
    }

    impl From<serde_json::Error> for ExpressionJsonError {
        fn from(error: serde_json::Error) -> Self {
            Self::Json(error)
        }
    }

    /// 轻量符号，用于恢复非路径符号引用。
    /// Lightweight symbol used to restore non-path symbol references.
    #[derive(Debug, Clone)]
    struct PlainExpressionSymbol {
        name: String,
        id: usize,
    }

    impl PlainExpressionSymbol {
        fn new(name: impl Into<String>) -> Self {
            let name = name.into();
            let id = stable_path_symbol_hash(name.as_bytes());
            Self { name, id }
        }
    }

    impl Display for PlainExpressionSymbol {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.name)
        }
    }

    impl DynSymbol for PlainExpressionSymbol {
        fn name(&self) -> &str {
            &self.name
        }

        fn display_name(&self) -> &str {
            &self.name
        }

        fn dyn_id(&self) -> SymbolDynId<'_> {
            SymbolDynId::standalone(self.id)
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "type")]
    enum ScalarExpressionData<T> {
        #[serde(rename = "Constant")]
        Constant { value: T },
        #[serde(rename = "Reference")]
        Reference { path: String },
        #[serde(rename = "SymbolReference")]
        SymbolReference { identifier: String },
        #[serde(rename = "Unary")]
        Unary {
            operator: String,
            operand: Box<ScalarExpressionData<T>>,
        },
        #[serde(rename = "Binary")]
        Binary {
            operator: String,
            left: Box<ScalarExpressionData<T>>,
            right: Box<ScalarExpressionData<T>>,
        },
        #[serde(rename = "Function")]
        Function {
            name: String,
            arguments: Vec<ScalarExpressionData<T>>,
        },
        #[serde(rename = "Custom")]
        Custom {
            payload: Option<String>,
            description: Option<String>,
        },
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "type")]
    enum BooleanExpressionData<T> {
        #[serde(rename = "BooleanConstant")]
        BooleanConstant { value: String },
        #[serde(rename = "Comparison")]
        Comparison {
            operator: String,
            left: ScalarExpressionData<T>,
            right: ScalarExpressionData<T>,
        },
        #[serde(rename = "In")]
        In {
            value: ScalarExpressionData<T>,
            candidates: Vec<ScalarExpressionData<T>>,
            #[serde(default)]
            negated: bool,
        },
        #[serde(rename = "PatternMatch")]
        PatternMatch {
            value: ScalarExpressionData<T>,
            pattern: ScalarExpressionData<T>,
            mode: String,
            #[serde(default)]
            negated: bool,
        },
        #[serde(rename = "NullCheck")]
        NullCheck {
            path: String,
            #[serde(rename = "nullCheckType")]
            null_check_type: String,
        },
        #[serde(rename = "And")]
        And {
            operands: Vec<BooleanExpressionData<T>>,
        },
        #[serde(rename = "Or")]
        Or {
            operands: Vec<BooleanExpressionData<T>>,
        },
        #[serde(rename = "Not")]
        Not {
            operand: Box<BooleanExpressionData<T>>,
        },
        #[serde(rename = "Custom")]
        Custom {
            payload: Option<String>,
            description: Option<String>,
        },
    }

    impl<T: Clone> From<&ScalarExpression<T>> for ScalarExpressionData<T> {
        fn from(value: &ScalarExpression<T>) -> Self {
            match value {
                ScalarExpression::Constant(value) => Self::Constant {
                    value: value.clone(),
                },
                ScalarExpression::Reference(path) => Self::Reference {
                    path: path.value().to_string(),
                },
                ScalarExpression::SymbolReference(symbol) => Self::SymbolReference {
                    identifier: symbol_identifier(symbol),
                },
                ScalarExpression::Unary { operator, operand } => Self::Unary {
                    operator: unary_operator_name(*operator).to_string(),
                    operand: Box::new(ScalarExpressionData::from(operand.as_ref())),
                },
                ScalarExpression::Binary {
                    operator,
                    left,
                    right,
                } => Self::Binary {
                    operator: binary_operator_name(*operator).to_string(),
                    left: Box::new(ScalarExpressionData::from(left.as_ref())),
                    right: Box::new(ScalarExpressionData::from(right.as_ref())),
                },
                ScalarExpression::Function { name, arguments } => Self::Function {
                    name: name.clone(),
                    arguments: arguments.iter().map(ScalarExpressionData::from).collect(),
                },
                ScalarExpression::Custom {
                    payload,
                    description,
                } => Self::Custom {
                    payload: Some(payload.clone()),
                    description: description.clone(),
                },
            }
        }
    }

    impl<T> TryFrom<ScalarExpressionData<T>> for ScalarExpression<T> {
        type Error = ExpressionJsonError;

        fn try_from(value: ScalarExpressionData<T>) -> Result<Self, Self::Error> {
            match value {
                ScalarExpressionData::Constant { value } => Ok(Self::Constant(value)),
                ScalarExpressionData::Reference { path } => {
                    Ok(Self::Reference(PropertyPath::parse(path)))
                }
                ScalarExpressionData::SymbolReference { identifier } => {
                    Ok(Self::SymbolReference(symbol_from_identifier(&identifier)))
                }
                ScalarExpressionData::Unary { operator, operand } => Ok(Self::Unary {
                    operator: parse_unary_operator(&operator)?,
                    operand: Box::new(ScalarExpression::try_from(*operand)?),
                }),
                ScalarExpressionData::Binary {
                    operator,
                    left,
                    right,
                } => Ok(Self::Binary {
                    operator: parse_binary_operator(&operator)?,
                    left: Box::new(ScalarExpression::try_from(*left)?),
                    right: Box::new(ScalarExpression::try_from(*right)?),
                }),
                ScalarExpressionData::Function { name, arguments } => Ok(Self::Function {
                    name,
                    arguments: arguments
                        .into_iter()
                        .map(ScalarExpression::try_from)
                        .collect::<Result<Vec<_>, _>>()?,
                }),
                ScalarExpressionData::Custom {
                    payload,
                    description,
                } => Ok(Self::Custom {
                    payload: payload.unwrap_or_default(),
                    description,
                }),
            }
        }
    }

    impl<T: Clone> From<&BooleanExpression<T>> for BooleanExpressionData<T> {
        fn from(value: &BooleanExpression<T>) -> Self {
            match value {
                BooleanExpression::Constant(value) => Self::BooleanConstant {
                    value: trivalent_name(*value).to_string(),
                },
                BooleanExpression::Comparison {
                    operator,
                    left,
                    right,
                } => Self::Comparison {
                    operator: comparison_operator_name(*operator).to_string(),
                    left: ScalarExpressionData::from(left),
                    right: ScalarExpressionData::from(right),
                },
                BooleanExpression::In {
                    value,
                    candidates,
                    negated,
                } => Self::In {
                    value: ScalarExpressionData::from(value),
                    candidates: candidates.iter().map(ScalarExpressionData::from).collect(),
                    negated: *negated,
                },
                BooleanExpression::PatternMatch {
                    value,
                    pattern,
                    mode,
                    negated,
                } => Self::PatternMatch {
                    value: ScalarExpressionData::from(value),
                    pattern: ScalarExpressionData::from(pattern),
                    mode: pattern_match_mode_name(*mode).to_string(),
                    negated: *negated,
                },
                BooleanExpression::NullCheck {
                    path,
                    null_check_type,
                } => Self::NullCheck {
                    path: path.value().to_string(),
                    null_check_type: null_check_type_name(*null_check_type).to_string(),
                },
                BooleanExpression::And(operands) => Self::And {
                    operands: operands.iter().map(BooleanExpressionData::from).collect(),
                },
                BooleanExpression::Or(operands) => Self::Or {
                    operands: operands.iter().map(BooleanExpressionData::from).collect(),
                },
                BooleanExpression::Not(operand) => Self::Not {
                    operand: Box::new(BooleanExpressionData::from(operand.as_ref())),
                },
                BooleanExpression::Custom {
                    payload,
                    description,
                } => Self::Custom {
                    payload: Some(payload.clone()),
                    description: description.clone(),
                },
            }
        }
    }

    impl<T> TryFrom<BooleanExpressionData<T>> for BooleanExpression<T> {
        type Error = ExpressionJsonError;

        fn try_from(value: BooleanExpressionData<T>) -> Result<Self, Self::Error> {
            match value {
                BooleanExpressionData::BooleanConstant { value } => {
                    Ok(Self::Constant(parse_trivalent(&value)))
                }
                BooleanExpressionData::Comparison {
                    operator,
                    left,
                    right,
                } => Ok(Self::Comparison {
                    operator: parse_comparison_operator(&operator)?,
                    left: ScalarExpression::try_from(left)?,
                    right: ScalarExpression::try_from(right)?,
                }),
                BooleanExpressionData::In {
                    value,
                    candidates,
                    negated,
                } => Ok(Self::In {
                    value: ScalarExpression::try_from(value)?,
                    candidates: candidates
                        .into_iter()
                        .map(ScalarExpression::try_from)
                        .collect::<Result<Vec<_>, _>>()?,
                    negated,
                }),
                BooleanExpressionData::PatternMatch {
                    value,
                    pattern,
                    mode,
                    negated,
                } => Ok(Self::PatternMatch {
                    value: ScalarExpression::try_from(value)?,
                    pattern: ScalarExpression::try_from(pattern)?,
                    mode: parse_pattern_match_mode(&mode)?,
                    negated,
                }),
                BooleanExpressionData::NullCheck {
                    path,
                    null_check_type,
                } => Ok(Self::NullCheck {
                    path: PropertyPath::parse(path),
                    null_check_type: parse_null_check_type(&null_check_type)?,
                }),
                BooleanExpressionData::And { operands } => Ok(Self::and(
                    operands
                        .into_iter()
                        .map(BooleanExpression::try_from)
                        .collect::<Result<Vec<_>, _>>()?,
                )),
                BooleanExpressionData::Or { operands } => Ok(Self::or(
                    operands
                        .into_iter()
                        .map(BooleanExpression::try_from)
                        .collect::<Result<Vec<_>, _>>()?,
                )),
                BooleanExpressionData::Not { operand } => {
                    Ok(Self::not_expr(BooleanExpression::try_from(*operand)?))
                }
                BooleanExpressionData::Custom {
                    payload,
                    description,
                } => Ok(Self::Custom {
                    payload: payload.unwrap_or_default(),
                    description,
                }),
            }
        }
    }

    impl<T: Clone + Serialize> ScalarExpression<T> {
        /// 转换为紧凑 JSON 字符串。
        /// Convert to a compact JSON string.
        pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
            serde_json::to_string(&ScalarExpressionData::from(self))
        }

        /// 转换为格式化 JSON 字符串。
        /// Convert to a pretty JSON string.
        pub fn to_json_string_pretty(&self) -> Result<String, serde_json::Error> {
            serde_json::to_string_pretty(&ScalarExpressionData::from(self))
        }
    }

    impl<T: Clone + Serialize> BooleanExpression<T> {
        /// 转换为紧凑 JSON 字符串。
        /// Convert to a compact JSON string.
        pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
            serde_json::to_string(&BooleanExpressionData::from(self))
        }

        /// 转换为格式化 JSON 字符串。
        /// Convert to a pretty JSON string.
        pub fn to_json_string_pretty(&self) -> Result<String, serde_json::Error> {
            serde_json::to_string_pretty(&BooleanExpressionData::from(self))
        }
    }

    /// 从 JSON 字符串反序列化标量表达式。
    /// Deserialize a scalar expression from a JSON string.
    pub fn scalar_expression_from_json<T>(
        json: &str,
    ) -> Result<ScalarExpression<T>, ExpressionJsonError>
    where
        T: DeserializeOwned,
    {
        let data: ScalarExpressionData<T> = serde_json::from_str(json)?;
        ScalarExpression::try_from(data)
    }

    /// 尝试从 JSON 字符串反序列化标量表达式。
    /// Try to deserialize a scalar expression from a JSON string.
    pub fn scalar_expression_from_json_or_none<T>(json: &str) -> Option<ScalarExpression<T>>
    where
        T: DeserializeOwned,
    {
        scalar_expression_from_json(json).ok()
    }

    /// 从 JSON 字符串反序列化布尔表达式。
    /// Deserialize a boolean expression from a JSON string.
    pub fn boolean_expression_from_json<T>(
        json: &str,
    ) -> Result<BooleanExpression<T>, ExpressionJsonError>
    where
        T: DeserializeOwned,
    {
        let data: BooleanExpressionData<T> = serde_json::from_str(json)?;
        BooleanExpression::try_from(data)
    }

    /// 尝试从 JSON 字符串反序列化布尔表达式。
    /// Try to deserialize a boolean expression from a JSON string.
    pub fn boolean_expression_from_json_or_none<T>(json: &str) -> Option<BooleanExpression<T>>
    where
        T: DeserializeOwned,
    {
        boolean_expression_from_json(json).ok()
    }

    fn symbol_identifier(symbol: &OwnedSymbol) -> String {
        if let Some(path) = property_path_from_owned_symbol(symbol) {
            path_symbol_id(path)
        } else {
            symbol.name().to_string()
        }
    }

    fn symbol_from_identifier(identifier: &str) -> OwnedSymbol {
        if let Some(path) = identifier.strip_prefix("path:") {
            path_owned_symbol(path)
        } else {
            OwnedSymbol::new(PlainExpressionSymbol::new(identifier))
        }
    }

    fn invalid_operator(kind: &'static str, value: &str) -> ExpressionJsonError {
        ExpressionJsonError::InvalidOperator {
            kind,
            value: value.to_string(),
        }
    }

    fn unary_operator_name(operator: UnaryOperator) -> &'static str {
        match operator {
            UnaryOperator::Negate => "Negate",
            UnaryOperator::Positive => "Positive",
            UnaryOperator::Abs => "Abs",
        }
    }

    fn parse_unary_operator(value: &str) -> Result<UnaryOperator, ExpressionJsonError> {
        match value {
            "Negate" => Ok(UnaryOperator::Negate),
            "Positive" => Ok(UnaryOperator::Positive),
            "Abs" => Ok(UnaryOperator::Abs),
            _ => Err(invalid_operator("unary", value)),
        }
    }

    fn binary_operator_name(operator: BinaryOperator) -> &'static str {
        match operator {
            BinaryOperator::Add => "Add",
            BinaryOperator::Subtract => "Subtract",
            BinaryOperator::Multiply => "Multiply",
            BinaryOperator::Divide => "Divide",
            BinaryOperator::Modulo => "Modulo",
            BinaryOperator::Power => "Power",
        }
    }

    fn parse_binary_operator(value: &str) -> Result<BinaryOperator, ExpressionJsonError> {
        match value {
            "Add" => Ok(BinaryOperator::Add),
            "Subtract" => Ok(BinaryOperator::Subtract),
            "Multiply" => Ok(BinaryOperator::Multiply),
            "Divide" => Ok(BinaryOperator::Divide),
            "Modulo" => Ok(BinaryOperator::Modulo),
            "Power" => Ok(BinaryOperator::Power),
            _ => Err(invalid_operator("binary", value)),
        }
    }

    fn comparison_operator_name(operator: ComparisonOperator) -> &'static str {
        match operator {
            ComparisonOperator::Eq => "Eq",
            ComparisonOperator::Ne => "Ne",
            ComparisonOperator::Lt => "Lt",
            ComparisonOperator::Le => "Le",
            ComparisonOperator::Gt => "Gt",
            ComparisonOperator::Ge => "Ge",
        }
    }

    fn parse_comparison_operator(value: &str) -> Result<ComparisonOperator, ExpressionJsonError> {
        match value {
            "Eq" => Ok(ComparisonOperator::Eq),
            "Ne" => Ok(ComparisonOperator::Ne),
            "Lt" => Ok(ComparisonOperator::Lt),
            "Le" => Ok(ComparisonOperator::Le),
            "Gt" => Ok(ComparisonOperator::Gt),
            "Ge" => Ok(ComparisonOperator::Ge),
            _ => Err(invalid_operator("comparison", value)),
        }
    }

    fn pattern_match_mode_name(mode: PatternMatchMode) -> &'static str {
        match mode {
            PatternMatchMode::Exact => "Exact",
            PatternMatchMode::Prefix => "Prefix",
            PatternMatchMode::Suffix => "Suffix",
            PatternMatchMode::Contains => "Contains",
            PatternMatchMode::Like => "Like",
            PatternMatchMode::Regex => "Regex",
        }
    }

    fn parse_pattern_match_mode(value: &str) -> Result<PatternMatchMode, ExpressionJsonError> {
        match value {
            "Exact" => Ok(PatternMatchMode::Exact),
            "Prefix" => Ok(PatternMatchMode::Prefix),
            "Suffix" => Ok(PatternMatchMode::Suffix),
            "Contains" => Ok(PatternMatchMode::Contains),
            "Like" => Ok(PatternMatchMode::Like),
            "Regex" => Ok(PatternMatchMode::Regex),
            _ => Err(invalid_operator("pattern match", value)),
        }
    }

    fn null_check_type_name(null_check_type: NullCheckType) -> &'static str {
        match null_check_type {
            NullCheckType::IsNull => "IsNull",
            NullCheckType::IsNotNull => "IsNotNull",
        }
    }

    fn parse_null_check_type(value: &str) -> Result<NullCheckType, ExpressionJsonError> {
        match value {
            "IsNull" => Ok(NullCheckType::IsNull),
            "IsNotNull" => Ok(NullCheckType::IsNotNull),
            _ => Err(invalid_operator("null check", value)),
        }
    }

    fn trivalent_name(value: Trivalent) -> &'static str {
        match value {
            Trivalent::True => "true",
            Trivalent::False => "false",
            Trivalent::Unknown => "unknown",
        }
    }

    fn parse_trivalent(value: &str) -> Trivalent {
        match value.to_ascii_lowercase().as_str() {
            "true" => Trivalent::True,
            "false" => Trivalent::False,
            _ => Trivalent::Unknown,
        }
    }
}

#[cfg(feature = "serde")]
pub use serde_support::{
    ExpressionJsonError, boolean_expression_from_json, boolean_expression_from_json_or_none,
    scalar_expression_from_json, scalar_expression_from_json_or_none,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_path_matches_kotlin_shape() {
        let path = PropertyPath::parse(" user.address.city ");

        assert_eq!(path.value(), "user.address.city");
        assert_eq!(path.segments(), vec!["user", "address", "city"]);
        assert_eq!(path.depth(), 3);
        assert_eq!(path.root(), Some("user"));
        assert_eq!(path.leaf(), Some("city"));
        assert_eq!(path.parent(), Some(PropertyPath::parse("user.address")));
        assert_eq!(path.child(), Some(PropertyPath::parse("address.city")));
        assert!(path.is_sub_path_of(&PropertyPath::parse("user.address")));
        assert!(PropertyPath::parse("user").is_parent_path_of(&path));
        assert_eq!(
            PropertyPath::parse("user").concat_segment("name"),
            PropertyPath::parse("user.name")
        );
    }

    #[test]
    fn property_path_validates_identifiers() {
        assert_eq!(
            PropertyPath::parse_or_none("user.address_1"),
            Some(PropertyPath::parse("user.address_1"))
        );
        assert_eq!(PropertyPath::parse_or_none("1user.address"), None);
        assert_eq!(PropertyPath::parse_or_none("user..address"), None);
        assert_eq!(PropertyPath::parse_or_none(""), None);
    }

    #[test]
    fn path_symbol_bridges_to_owned_symbol() {
        let path = PropertyPath::parse("user.age");
        let symbol = PathSymbol::from_path(path.clone());
        let owned = symbol.clone().into_owned_symbol();

        assert_eq!(symbol.name(), "user.age");
        assert_eq!(symbol.display_name(), "user.age");
        assert_eq!(symbol.symbol_id(), "path:user.age");
        assert_eq!(path_symbol_id(&path), "path:user.age");
        assert_eq!(property_path_from_owned_symbol(&owned), Some(&path));
        assert_eq!(owned.name(), "user.age");
    }

    #[test]
    fn scalar_expression_reports_references_and_depth() {
        let expression = ScalarExpression::<i32>::add_expr(
            ScalarExpression::reference("user.age"),
            ScalarExpression::function(
                ScalarFunctionNames::ABS,
                vec![ScalarExpression::symbol_reference(path_owned_symbol(
                    "order.price",
                ))],
            ),
        );

        let references = expression.collect_references();
        assert_eq!(expression.type_name(), "Binary");
        assert!(!expression.is_constant());
        assert!(expression.contains_reference());
        assert_eq!(expression.depth(), 3);
        assert!(references.contains(&PropertyPath::parse("user.age")));
        assert!(references.contains(&PropertyPath::parse("order.price")));
    }

    #[test]
    fn boolean_expression_reports_structure() {
        let age = ScalarExpression::<i32>::reference("user.age");
        let adult = BooleanExpression::ge(age, ScalarExpression::constant(18));
        let named = BooleanExpression::is_not_null("user.name");
        let expression = BooleanExpression::and(vec![adult, BooleanExpression::not_expr(named)]);

        assert_eq!(expression.type_name(), "And");
        assert_eq!(expression.logical_operator_count(), 3);
        assert_eq!(expression.depth(), 3);
        assert!(!expression.is_constant());
        assert!(!expression.is_pure_logical());

        let references = expression.collect_references();
        assert!(references.contains(&PropertyPath::parse("user.age")));
        assert!(references.contains(&PropertyPath::parse("user.name")));
    }

    #[test]
    fn operator_symbols_and_inverse_are_kotlin_compatible() {
        assert_eq!(UnaryOperator::Abs.symbol(), "abs");
        assert_eq!(BinaryOperator::Power.symbol(), "^");
        assert_eq!(ComparisonOperator::Le.symbol(), "<=");
        assert_eq!(ComparisonOperator::Lt.inverse(), ComparisonOperator::Gt);
        assert_eq!(BooleanOperator::And.symbol(), "and");
        assert_eq!(NullCheckType::IsNotNull.symbol(), "is not null");
    }

    #[test]
    fn path_builder_builds_runtime_expression() {
        let expression = boolean_expression(|| {
            path("age").ge(18) & path("status").eq("active") & !path("deleted_at").is_null()
        });

        let BooleanExpression::And(operands) = expression else {
            panic!("expected AND expression");
        };
        assert_eq!(operands.len(), 3);
        assert_eq!(
            operands[0],
            BooleanExpression::ge(
                ScalarExpression::reference("age"),
                ScalarExpression::constant(ExpressionValue::Number(18.0)),
            )
        );
        assert_eq!(
            operands[1],
            BooleanExpression::eq(
                ScalarExpression::reference("status"),
                ScalarExpression::constant(ExpressionValue::String("active".to_string())),
            )
        );
        assert!(matches!(operands[2], BooleanExpression::Not(_)));
    }

    #[test]
    fn path_builder_builds_typed_expression() {
        let expression = typed_path::<i32>("age").ge(18)
            & typed_path::<i32>("score").lt(typed_path::<i32>("limit").as_scalar());

        let references = expression.collect_references();
        assert!(references.contains(&PropertyPath::parse("age")));
        assert!(references.contains(&PropertyPath::parse("score")));
        assert!(references.contains(&PropertyPath::parse("limit")));
        assert!(matches!(expression, BooleanExpression::And(_)));
    }

    #[test]
    fn quick_expression_constructors_use_runtime_values() {
        let expression: ParsedBooleanExpression = eq("age", 18)
            .and_expr(in_expr("status", ["active", "pending"]))
            .and_expr(is_not_null("profile.email"));

        let BooleanExpression::And(operands) = expression else {
            panic!("expected AND expression");
        };
        assert_eq!(operands.len(), 3);
        assert!(matches!(operands[1], BooleanExpression::In { .. }));
        assert!(matches!(operands[2], BooleanExpression::NullCheck { .. }));
    }

    #[test]
    fn scalar_function_dsl_builds_comparable_expression() {
        let expression = abs(path("delta")).gt_expr(0);

        let BooleanExpression::Comparison {
            operator,
            left,
            right,
        } = expression
        else {
            panic!("expected comparison expression");
        };
        assert_eq!(operator, ComparisonOperator::Gt);
        assert_eq!(
            left,
            ScalarExpression::function(
                ScalarFunctionNames::ABS,
                vec![ScalarExpression::reference("delta")],
            )
        );
        assert_eq!(
            right,
            ScalarExpression::constant(ExpressionValue::Number(0.0))
        );
    }

    #[test]
    fn runtime_boolean_expression_evaluates_with_context() {
        let expression = path("age").ge(18)
            & lower(path("status").as_scalar()).eq_expr("active")
            & path("name").like("A%")
            & abs(path("delta").as_scalar()).ge_expr(3);
        let context = MapEvaluationContext::from_string_map([
            ("age", ExpressionValue::from(20)),
            ("status", ExpressionValue::from("ACTIVE")),
            ("name", ExpressionValue::from("Alice")),
            ("delta", ExpressionValue::from(-3)),
        ]);

        assert_eq!(evaluate_boolean(&expression, &context), Trivalent::True);
        assert_eq!(expression.evaluate_with_or_none(&context), Some(true));
    }

    #[test]
    fn runtime_boolean_expression_preserves_unknown_semantics() {
        let expression = path("deleted_at").is_null() | path("score").gt(90);
        let context = MapEvaluationContext::from_string_map([("score", ExpressionValue::from(80))]);

        assert_eq!(evaluate_boolean(&expression, &context), Trivalent::Unknown);

        let expression = path("code").regex("^A[0-9]+$");
        let context =
            MapEvaluationContext::from_string_map([("code", ExpressionValue::from("A12"))]);
        assert_eq!(evaluate_boolean(&expression, &context), Trivalent::True);
    }

    #[test]
    fn boolean_expression_normalizes_like_kotlin_operation_layer() {
        let adult = path("age").ge(18);
        let duplicated = BooleanExpression::and(vec![
            BooleanExpression::true_constant(),
            adult.clone(),
            BooleanExpression::and(vec![adult.clone(), BooleanExpression::true_constant()]),
        ]);

        assert_eq!(duplicated.normalize(), adult);

        let demorgan = BooleanExpression::not_expr(BooleanExpression::and(vec![
            path("a").eq(1),
            path("b").eq(2),
        ]));
        let normalized = demorgan.normalize_with_config(NormalizeConfig {
            apply_de_morgan: true,
            ..NormalizeConfig::default()
        });
        assert!(matches!(normalized, BooleanExpression::Or(_)));
    }

    #[cfg(feature = "parser")]
    #[test]
    fn expression_lexer_tokenizes_paths_literals_and_keywords() {
        let tokens = tokenize_expression(r#"user.name like "A\n%" and score >= -1.25e+2"#);
        let token_types = tokens
            .iter()
            .map(|token| token.token_type)
            .collect::<Vec<_>>();

        assert_eq!(
            token_types,
            vec![
                ExpressionTokenType::Identifier,
                ExpressionTokenType::Like,
                ExpressionTokenType::String,
                ExpressionTokenType::And,
                ExpressionTokenType::Identifier,
                ExpressionTokenType::Ge,
                ExpressionTokenType::Number,
                ExpressionTokenType::Eof,
            ]
        );
        assert_eq!(tokens[0].value, "user.name");
        assert_eq!(tokens[2].value, "A\n%");
        assert_eq!(tokens[6].value, "-1.25e+2");
    }

    #[cfg(feature = "parser")]
    #[test]
    fn expression_parser_parses_comparison() {
        let expression = parse_boolean_expression("age >= 18").unwrap();

        assert_eq!(
            expression,
            BooleanExpression::ge(
                ScalarExpression::reference("age"),
                ScalarExpression::constant(ExpressionValue::Number(18.0)),
            )
        );
    }

    #[cfg(feature = "parser")]
    #[test]
    fn expression_parser_preserves_logic_precedence() {
        let expression =
            parse_boolean_expression("age >= 18 and status = 'active' or vip = true").unwrap();

        let BooleanExpression::Or(or_operands) = expression else {
            panic!("expected top-level OR expression");
        };
        assert_eq!(or_operands.len(), 2);
        assert!(matches!(or_operands[0], BooleanExpression::And(_)));
        assert_eq!(
            or_operands[1],
            BooleanExpression::eq(
                ScalarExpression::reference("vip"),
                ScalarExpression::constant(ExpressionValue::Boolean(true)),
            )
        );
    }

    #[cfg(feature = "parser")]
    #[test]
    fn expression_parser_parses_parentheses_and_not() {
        let expression = parse_boolean_expression("not (age < 18 or banned = true)").unwrap();

        let BooleanExpression::Not(inner) = expression else {
            panic!("expected NOT expression");
        };
        let BooleanExpression::Or(operands) = inner.as_ref() else {
            panic!("expected grouped OR expression");
        };
        assert_eq!(operands.len(), 2);
        assert_eq!(
            operands[0],
            BooleanExpression::lt(
                ScalarExpression::reference("age"),
                ScalarExpression::constant(ExpressionValue::Number(18.0)),
            )
        );
    }

    #[cfg(feature = "parser")]
    #[test]
    fn expression_parser_parses_in_and_not_in() {
        let expression = parse_boolean_expression("status in ('active', 'pending')").unwrap();
        let BooleanExpression::In {
            value,
            candidates,
            negated,
        } = expression
        else {
            panic!("expected IN expression");
        };
        assert_eq!(value, ScalarExpression::reference("status"));
        assert_eq!(
            candidates,
            vec![
                ScalarExpression::constant(ExpressionValue::String("active".to_string())),
                ScalarExpression::constant(ExpressionValue::String("pending".to_string())),
            ]
        );
        assert!(!negated);

        let expression = parse_boolean_expression("status not in ('archived')").unwrap();
        assert!(matches!(
            expression,
            BooleanExpression::In { negated: true, .. }
        ));
    }

    #[cfg(feature = "parser")]
    #[test]
    fn expression_parser_parses_null_checks() {
        assert_eq!(
            parse_boolean_expression("deleted_at is null").unwrap(),
            BooleanExpression::is_null("deleted_at")
        );
        assert_eq!(
            parse_boolean_expression("profile.email is not null").unwrap(),
            BooleanExpression::is_not_null("profile.email")
        );
    }

    #[cfg(feature = "parser")]
    #[test]
    fn expression_parser_parses_pattern_matches() {
        let expression = parse_boolean_expression("name like 'A%'").unwrap();
        assert_eq!(
            expression,
            BooleanExpression::pattern_match(
                ScalarExpression::reference("name"),
                ScalarExpression::constant(ExpressionValue::String("A%".to_string())),
                PatternMatchMode::Like,
                false,
            )
        );

        let expression = parse_boolean_expression("name not regex '^A'").unwrap();
        assert_eq!(
            expression,
            BooleanExpression::pattern_match(
                ScalarExpression::reference("name"),
                ScalarExpression::constant(ExpressionValue::String("^A".to_string())),
                PatternMatchMode::Regex,
                true,
            )
        );
    }

    #[cfg(feature = "parser")]
    #[test]
    fn expression_parser_reports_invalid_input() {
        let error = parse_boolean_expression("age >").unwrap_err();

        assert!(error.message().contains("expected scalar value"));
        assert!(parse_boolean_expression_or_none("age >").is_none());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn scalar_expression_json_matches_kotlin_shape() {
        let expression = ScalarExpression::<i32>::add_expr(
            ScalarExpression::reference("user.age"),
            ScalarExpression::symbol_reference(path_owned_symbol("order.price")),
        );

        let json = expression.to_json_string().unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "Binary");
        assert_eq!(value["operator"], "Add");
        assert_eq!(value["left"]["type"], "Reference");
        assert_eq!(value["left"]["path"], "user.age");
        assert_eq!(value["right"]["type"], "SymbolReference");
        assert_eq!(value["right"]["identifier"], "path:order.price");

        let restored = scalar_expression_from_json::<i32>(&json).unwrap();
        let references = restored.collect_references();
        assert!(references.contains(&PropertyPath::parse("user.age")));
        assert!(references.contains(&PropertyPath::parse("order.price")));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn boolean_expression_json_round_trips() {
        let age = ScalarExpression::<i32>::reference("user.age");
        let adult = BooleanExpression::ge(age, ScalarExpression::constant(18));
        let named = BooleanExpression::is_not_null("user.name");
        let expression = BooleanExpression::and(vec![adult, BooleanExpression::not_expr(named)]);

        let json = expression.to_json_string().unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "And");
        assert_eq!(value["operands"][0]["type"], "Comparison");
        assert_eq!(value["operands"][0]["operator"], "Ge");
        assert_eq!(value["operands"][1]["type"], "Not");
        assert_eq!(value["operands"][1]["operand"]["type"], "NullCheck");
        assert_eq!(
            value["operands"][1]["operand"]["nullCheckType"],
            "IsNotNull"
        );

        let restored = boolean_expression_from_json::<i32>(&json).unwrap();
        assert_eq!(restored.logical_operator_count(), 3);
        assert_eq!(restored.depth(), 3);
        assert!(
            restored
                .collect_references()
                .contains(&PropertyPath::parse("user.name"))
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn boolean_expression_json_rejects_invalid_operator() {
        let json = r#"{"type":"Comparison","operator":"Bad","left":{"type":"Constant","value":1},"right":{"type":"Constant","value":2}}"#;
        assert!(boolean_expression_from_json::<i32>(json).is_err());
        assert!(boolean_expression_from_json_or_none::<i32>(json).is_none());
    }
}
