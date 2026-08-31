//! 属性路径与路径符号
//! Property path and path symbol

use std::any::Any;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use crate::symbol::{DynSymbol, OwnedSymbol, Symbol, SymbolDynId};

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

pub(super) fn stable_path_symbol_hash(bytes: &[u8]) -> usize {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash as usize
}
