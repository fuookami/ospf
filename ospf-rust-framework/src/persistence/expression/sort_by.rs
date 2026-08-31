//! 排序描述
//! Sort descriptor

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NullsOrder {
    NullsFirst,
    NullsLast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullsOrderSupport {
    Auto,
    Always,
    Never,
    OnlyAsc,
}

impl NullsOrderSupport {
    /// 检查后端是否支持指定排序项的空值排序语法。
    /// Check whether the backend supports null ordering for the given item.
    pub const fn is_supported(self, item: &SortItem) -> bool {
        match self {
            Self::Auto | Self::Always => true,
            Self::Never => false,
            Self::OnlyAsc => matches!(item.direction, SortDirection::Asc),
        }
    }
}

impl Default for NullsOrderSupport {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortItem {
    pub path: ospf_rust_math::symbol::PropertyPath,
    pub direction: SortDirection,
    pub nulls: Option<NullsOrder>,
}

impl SortItem {
    /// 创建排序项。
    /// Create a sort item.
    pub fn new(
        path: impl Into<ospf_rust_math::symbol::PropertyPath>,
        direction: SortDirection,
        nulls: Option<NullsOrder>,
    ) -> Self {
        Self {
            path: path.into(),
            direction,
            nulls,
        }
    }

    /// 创建升序排序项。
    /// Create an ascending sort item.
    pub fn asc(path: impl Into<ospf_rust_math::symbol::PropertyPath>) -> Self {
        Self::new(path, SortDirection::Asc, None)
    }

    /// 创建降序排序项。
    /// Create a descending sort item.
    pub fn desc(path: impl Into<ospf_rust_math::symbol::PropertyPath>) -> Self {
        Self::new(path, SortDirection::Desc, None)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SortBy {
    pub items: Vec<SortItem>,
}

impl SortBy {
    /// 创建空排序。
    /// Create an empty sort descriptor.
    pub fn empty() -> Self {
        Self { items: Vec::new() }
    }

    /// 创建单字段升序排序。
    /// Create a single-field ascending sort.
    pub fn asc(path: impl Into<ospf_rust_math::symbol::PropertyPath>) -> Self {
        Self {
            items: vec![SortItem::asc(path)],
        }
    }

    /// 创建单字段降序排序。
    /// Create a single-field descending sort.
    pub fn desc(path: impl Into<ospf_rust_math::symbol::PropertyPath>) -> Self {
        Self {
            items: vec![SortItem::desc(path)],
        }
    }

    /// 创建带空值顺序的升序排序。
    /// Create an ascending sort with null ordering.
    pub fn asc_nulls(
        path: impl Into<ospf_rust_math::symbol::PropertyPath>,
        nulls: NullsOrder,
    ) -> Self {
        Self {
            items: vec![SortItem::new(path, SortDirection::Asc, Some(nulls))],
        }
    }

    /// 创建带空值顺序的降序排序。
    /// Create a descending sort with null ordering.
    pub fn desc_nulls(
        path: impl Into<ospf_rust_math::symbol::PropertyPath>,
        nulls: NullsOrder,
    ) -> Self {
        Self {
            items: vec![SortItem::new(path, SortDirection::Desc, Some(nulls))],
        }
    }

    /// 从多个路径创建升序排序。
    /// Create ascending sort from multiple paths.
    pub fn asc_many<I, P>(paths: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<ospf_rust_math::symbol::PropertyPath>,
    {
        Self {
            items: paths.into_iter().map(SortItem::asc).collect(),
        }
    }

    /// 从多个路径创建降序排序。
    /// Create descending sort from multiple paths.
    pub fn desc_many<I, P>(paths: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<ospf_rust_math::symbol::PropertyPath>,
    {
        Self {
            items: paths.into_iter().map(SortItem::desc).collect(),
        }
    }

    /// 组合排序。
    /// Combine sort descriptors.
    pub fn then(mut self, other: Self) -> Self {
        self.items.extend(other.items);
        self
    }

    /// 添加升序排序项。
    /// Add an ascending sort item.
    pub fn then_asc(mut self, path: impl Into<ospf_rust_math::symbol::PropertyPath>) -> Self {
        self.items.push(SortItem::asc(path));
        self
    }

    /// 添加降序排序项。
    /// Add a descending sort item.
    pub fn then_desc(mut self, path: impl Into<ospf_rust_math::symbol::PropertyPath>) -> Self {
        self.items.push(SortItem::desc(path));
        self
    }

    /// 添加带空值顺序的升序排序项。
    /// Add an ascending sort item with null ordering.
    pub fn then_asc_nulls(
        mut self,
        path: impl Into<ospf_rust_math::symbol::PropertyPath>,
        nulls: NullsOrder,
    ) -> Self {
        self.items
            .push(SortItem::new(path, SortDirection::Asc, Some(nulls)));
        self
    }

    /// 添加带空值顺序的降序排序项。
    /// Add a descending sort item with null ordering.
    pub fn then_desc_nulls(
        mut self,
        path: impl Into<ospf_rust_math::symbol::PropertyPath>,
        nulls: NullsOrder,
    ) -> Self {
        self.items
            .push(SortItem::new(path, SortDirection::Desc, Some(nulls)));
        self
    }

    /// 判断排序是否为空。
    /// Check whether the sort descriptor is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 判断排序是否非空。
    /// Check whether the sort descriptor is non-empty.
    pub fn is_not_empty(&self) -> bool {
        !self.is_empty()
    }
}

impl std::ops::Add for SortBy {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.then(rhs)
    }
}

/// 兼容旧命名。
/// Compatibility alias for the old name.
pub type SortOrder = SortDirection;

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_math::symbol::PropertyPath;

    #[test]
    fn sort_by_builds_multi_field_sort() {
        let sort = SortBy::desc_nulls("created_at", NullsOrder::NullsLast).then_asc("id")
            + SortBy::asc_many(["name", "status"]);

        assert_eq!(sort.items.len(), 4);
        assert_eq!(sort.items[0].path, PropertyPath::parse("created_at"));
        assert_eq!(sort.items[0].direction, SortDirection::Desc);
        assert_eq!(sort.items[0].nulls, Some(NullsOrder::NullsLast));
        assert_eq!(sort.items[1].direction, SortDirection::Asc);
        assert!(sort.is_not_empty());
    }

    #[test]
    fn nulls_order_support_checks_direction() {
        let asc = SortItem::asc("name");
        let desc = SortItem::desc("name");

        assert!(NullsOrderSupport::OnlyAsc.is_supported(&asc));
        assert!(!NullsOrderSupport::OnlyAsc.is_supported(&desc));
        assert!(!NullsOrderSupport::Never.is_supported(&asc));
    }
}
