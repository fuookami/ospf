//! 属性路径与路径符号
//! Property path and path symbol

use crate::symbol::{DynSymbol, OwnedSymbol, Symbol, SymbolDynId};
use std::any::Any;
use std::fmt::{Display, Formatter};
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

pub(super) fn stable_path_symbol_hash(bytes: &[u8]) -> usize {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash as usize
}

// ============================================================================
// 属性路径测试 / Property path tests
// ============================================================================

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::str::FromStr;

    use super::*;
    use crate::symbol::test_utils::SimpleSymbol;

    // ========================================================================
    // 路径解析 / Path parsing
    // ========================================================================

    #[test]
    fn parse_trims_surrounding_whitespace_only() {
        assert_eq!(PropertyPath::parse("  user.name  ").value(), "user.name");
        assert_eq!(PropertyPath::parse("user . name").value(), "user . name");
    }

    #[test]
    fn of_joins_segments_with_dots() {
        assert_eq!(PropertyPath::of(["a", "b", "c"]).value(), "a.b.c");
        assert_eq!(PropertyPath::of(["name"]).value(), "name");
        assert_eq!(PropertyPath::of(Vec::<String>::new()).value(), "");
    }

    #[test]
    fn parse_or_none_accepts_valid_identifiers() {
        assert_eq!(
            PropertyPath::parse_or_none("user.address_1"),
            Some(PropertyPath::parse("user.address_1"))
        );
        assert_eq!(
            PropertyPath::parse_or_none("_private.field2"),
            Some(PropertyPath::parse("_private.field2"))
        );
        assert_eq!(
            PropertyPath::parse_or_none("  user.name  "),
            Some(PropertyPath::parse("user.name"))
        );
    }

    #[test]
    fn parse_or_none_rejects_invalid_identifiers() {
        assert_eq!(PropertyPath::parse_or_none(""), None);
        assert_eq!(PropertyPath::parse_or_none("   "), None);
        assert_eq!(PropertyPath::parse_or_none("1user"), None);
        assert_eq!(PropertyPath::parse_or_none("user..name"), None);
        assert_eq!(PropertyPath::parse_or_none(".user"), None);
        assert_eq!(PropertyPath::parse_or_none("user."), None);
        assert_eq!(PropertyPath::parse_or_none("user.na me"), None);
        assert_eq!(PropertyPath::parse_or_none("user.na-me"), None);
    }

    #[test]
    fn try_parse_reports_error_with_original_text() {
        let error = PropertyPath::try_parse("1user").unwrap_err();
        assert_eq!(error.text(), "1user");
        assert_eq!(error.to_string(), "invalid property path: 1user");

        assert_eq!(
            PropertyPath::try_parse("user.name").unwrap(),
            PropertyPath::parse("user.name")
        );
    }

    #[test]
    fn from_str_requires_valid_identifier_segments() {
        assert_eq!(
            "user.name".parse::<PropertyPath>().unwrap(),
            PropertyPath::parse("user.name")
        );
        assert!("1user".parse::<PropertyPath>().is_err());
        assert!(PropertyPath::from_str("user.name").is_ok());
    }

    #[test]
    fn new_skips_identifier_validation() {
        // `new` 是低层入口，不做校验；校验入口是 `parse_or_none` / `try_parse`。
        // `new` is the low-level entry point without validation; validation lives in `parse_or_none` / `try_parse`.
        assert_eq!(PropertyPath::new("1user..name").value(), "1user..name");
        assert_eq!(PropertyPath::new("  padded  ").value(), "  padded  ");
    }

    #[test]
    fn is_valid_identifier_matches_documented_rule() {
        assert!(PropertyPath::is_valid_identifier("name"));
        assert!(PropertyPath::is_valid_identifier("_name"));
        assert!(PropertyPath::is_valid_identifier("name1"));
        assert!(PropertyPath::is_valid_identifier("名字"));

        assert!(!PropertyPath::is_valid_identifier(""));
        assert!(!PropertyPath::is_valid_identifier("1name"));
        assert!(!PropertyPath::is_valid_identifier("na me"));
        assert!(!PropertyPath::is_valid_identifier("na-me"));
        assert!(!PropertyPath::is_valid_identifier("na.me"));
    }

    // ========================================================================
    // 路径结构 / Path structure
    // ========================================================================

    #[test]
    fn segments_root_leaf_and_depth_describe_nested_path() {
        let path = PropertyPath::parse("user.address.city");

        assert_eq!(path.segments(), vec!["user", "address", "city"]);
        assert_eq!(path.depth(), 3);
        assert_eq!(path.root(), Some("user"));
        assert_eq!(path.leaf(), Some("city"));
        assert!(path.is_not_empty());
        assert!(!path.is_empty());
    }

    #[test]
    fn empty_path_has_no_segments_root_or_leaf() {
        let path = PropertyPath::EMPTY;

        assert!(path.is_empty());
        assert!(!path.is_not_empty());
        assert_eq!(path.depth(), 0);
        assert_eq!(path.segments(), Vec::<&str>::new());
        assert_eq!(path.root(), None);
        assert_eq!(path.leaf(), None);
        assert_eq!(path.parent(), None);
        assert_eq!(path.child(), None);
        assert_eq!(path, PropertyPath::default());
        assert_eq!(path, PropertyPath::parse(""));
    }

    #[test]
    fn single_segment_path_has_no_parent_or_child() {
        let path = PropertyPath::parse("name");

        assert_eq!(path.depth(), 1);
        assert_eq!(path.root(), Some("name"));
        assert_eq!(path.leaf(), Some("name"));
        assert_eq!(path.parent(), None);
        assert_eq!(path.child(), None);
    }

    #[test]
    fn parent_and_child_strip_opposite_ends() {
        let path = PropertyPath::parse("user.address.city");

        assert_eq!(path.parent(), Some(PropertyPath::parse("user.address")));
        assert_eq!(path.child(), Some(PropertyPath::parse("address.city")));
        assert_eq!(path.parent().unwrap().depth(), 2);
        assert_eq!(path.child().unwrap().depth(), 2);
    }

    // ========================================================================
    // 路径关系 / Path relations
    // ========================================================================

    #[test]
    fn sub_path_relation_is_strict_and_antisymmetric() {
        let parent = PropertyPath::parse("user.address");
        let child = PropertyPath::parse("user.address.city");

        assert!(child.is_sub_path_of(&parent));
        assert!(parent.is_parent_path_of(&child));
        assert!(!parent.is_sub_path_of(&child));
        assert!(!child.is_parent_path_of(&parent));
    }

    #[test]
    fn same_depth_paths_are_not_in_sub_path_relation() {
        let left = PropertyPath::parse("user.age");
        let right = PropertyPath::parse("user.name");

        assert!(!left.is_sub_path_of(&right));
        assert!(!right.is_sub_path_of(&left));
    }

    #[test]
    fn empty_path_participates_in_no_relation() {
        let empty = PropertyPath::EMPTY;
        let path = PropertyPath::parse("user.name");

        assert!(!empty.is_sub_path_of(&path));
        assert!(!path.is_sub_path_of(&empty));
        assert!(!path.is_sub_path_of(&path));
    }

    #[test]
    fn paths_are_ordered_lexicographically() {
        let mut paths = vec![
            PropertyPath::parse("user.name"),
            PropertyPath::parse("user.age"),
            PropertyPath::parse("account.id"),
        ];
        paths.sort();

        assert_eq!(
            paths,
            vec![
                PropertyPath::parse("account.id"),
                PropertyPath::parse("user.age"),
                PropertyPath::parse("user.name"),
            ]
        );
        assert!(PropertyPath::parse("a") < PropertyPath::parse("a.b"));
    }

    // ========================================================================
    // 路径拼接 / Path concatenation
    // ========================================================================

    #[test]
    fn concat_joins_two_non_empty_paths() {
        let left = PropertyPath::parse("user");
        let right = PropertyPath::parse("address.city");

        assert_eq!(left.concat(&right), PropertyPath::parse("user.address.city"));
    }

    #[test]
    fn concat_with_empty_path_is_neutral_on_both_sides() {
        let path = PropertyPath::parse("user.name");
        let empty = PropertyPath::EMPTY;

        assert_eq!(path.concat(&empty), path);
        assert_eq!(empty.concat(&path), path);
        assert_eq!(empty.concat(&empty), empty);
        assert!(empty.concat(&path).is_not_empty());
    }

    #[test]
    fn concat_segment_appends_one_level() {
        let path = PropertyPath::parse("user");

        assert_eq!(path.concat_segment("name"), PropertyPath::parse("user.name"));
        assert_eq!(
            path.concat_segment("address").concat_segment("city"),
            PropertyPath::parse("user.address.city")
        );
        assert_eq!(
            PropertyPath::EMPTY.concat_segment("name"),
            PropertyPath::parse("name")
        );
    }

    // ========================================================================
    // 显示与转换 / Display and conversions
    // ========================================================================

    #[test]
    fn display_and_string_conversions_round_trip() {
        let path = PropertyPath::parse("user.name");

        assert_eq!(path.to_string(), "user.name");
        assert_eq!(String::from(path.clone()), "user.name");
        assert_eq!(PropertyPath::from("user.name"), path);
        assert_eq!(PropertyPath::from("user.name".to_string()), path);
        assert_eq!(PropertyPath::from("  user.name  "), path);
    }

    #[test]
    fn hashing_and_equality_follow_the_path_text() {
        let mut set = HashSet::new();
        set.insert(PropertyPath::parse("user.name"));
        set.insert(PropertyPath::parse("  user.name  "));
        set.insert(PropertyPath::parse("user.age"));

        assert_eq!(set.len(), 2);
        assert!(set.contains(&PropertyPath::parse("user.name")));
    }

    // ========================================================================
    // 路径符号 / Path symbol
    // ========================================================================

    #[test]
    fn path_symbol_id_prefixes_the_path_value() {
        assert_eq!(path_symbol_id(&PropertyPath::parse("user.age")), "path:user.age");
        assert_eq!(path_symbol_id(&PropertyPath::EMPTY), "path:");
    }

    #[test]
    fn path_symbol_reports_path_name_and_symbol_id() {
        let symbol = PathSymbol::from_path(PropertyPath::parse("user.address.city"));

        assert_eq!(symbol.path(), &PropertyPath::parse("user.address.city"));
        assert_eq!(symbol.symbol_id(), "path:user.address.city");
        assert_eq!(symbol.name(), "user.address.city");
        assert_eq!(symbol.display_name(), "user.address.city");
        assert_eq!(symbol.to_string(), "user.address.city");
        assert_eq!(symbol.id(), "path:user.address.city".to_string());
        // 路径符号以 `PathSymbol` 作为父类型注册，index 为 0 表示父符号本身。
        // Path symbols register under the `PathSymbol` parent type, with index 0 meaning the parent itself.
        assert!(!symbol.dyn_id().is_standalone());
        assert_eq!(symbol.dyn_id().parent_type, "PathSymbol");
        assert_eq!(symbol.dyn_id().index, 0);
        assert!(symbol.dyn_id().is_parent());
    }

    #[test]
    fn path_symbol_constructors_agree_on_identity() {
        let from_path = PathSymbol::new(PropertyPath::parse("a.b.c"));
        let from_str = PathSymbol::from_path_str("a.b.c");
        let from_segments = PathSymbol::of(["a", "b", "c"]);
        let from_helper = path_symbol(PropertyPath::parse("a.b.c"));

        assert_eq!(from_path, from_str);
        assert_eq!(from_path, from_segments);
        assert_eq!(from_path, from_helper);
        assert_eq!(from_str.symbol_id(), "path:a.b.c");
        assert_eq!(from_path.dyn_id(), from_str.dyn_id());
    }

    #[test]
    fn path_symbol_equality_separates_distinct_paths() {
        let left = PathSymbol::from_path_str("user.address");
        let right = PathSymbol::from_path_str("user.name");
        let same = PathSymbol::from_path_str("user.address");

        assert_eq!(left, same);
        assert_ne!(left, right);
        assert_eq!(left, same.clone());
    }

    #[test]
    fn path_symbol_into_owned_symbol_keeps_display_and_path() {
        let symbol = PathSymbol::from_path(PropertyPath::parse("user.age"));
        let owned = symbol.clone().into_owned_symbol();

        assert_eq!(owned.name(), "user.age");
        assert_eq!(owned.to_string(), "user.age");
        assert_eq!(property_path_from_owned_symbol(&owned), Some(symbol.path()));
    }

    #[test]
    fn path_owned_symbol_helper_produces_owned_path_symbol() {
        let owned = path_owned_symbol(PropertyPath::parse("order.price"));

        assert_eq!(owned.name(), "order.price");
        assert_eq!(
            property_path_from_owned_symbol(&owned),
            Some(&PropertyPath::parse("order.price"))
        );
    }

    #[test]
    fn property_path_extraction_ignores_foreign_symbols() {
        let foreign = OwnedSymbol::new(SimpleSymbol::new("plain"));
        let path_symbol = PathSymbol::from_path_str("user.name");

        assert_eq!(property_path_from_owned_symbol(&foreign), None);
        assert_eq!(
            property_path_from_owned_symbol(&path_symbol.into_owned_symbol()),
            Some(&PropertyPath::parse("user.name"))
        );
        assert_eq!(property_path_from_symbol(&SimpleSymbol::new("plain")), None);
    }

    #[test]
    fn stable_path_symbol_hash_is_deterministic_and_content_sensitive() {
        assert_eq!(
            stable_path_symbol_hash(b"path:user.name"),
            stable_path_symbol_hash(b"path:user.name")
        );
        assert_ne!(
            stable_path_symbol_hash(b"path:user.name"),
            stable_path_symbol_hash(b"path:user.age")
        );
        assert_eq!(stable_path_symbol_hash(b""), 0xcbf29ce484222325_u64 as usize);
    }
}
