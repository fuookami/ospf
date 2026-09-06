// ============================================================================
// FixtureFilter - Fixture 过滤器 / Fixture filter
// ============================================================================

/// Fixture 过滤器 / Fixture filter
///
/// 支持按 tag、group、backend smoke 状态选择运行 fixture，
/// 便于把重型回归从普通 lib test 中拆出来。
/// Supports selecting fixtures by tag, group, and backend smoke status,
/// making it easy to separate heavy regression from regular lib tests.
#[derive(Debug, Clone, Default)]
pub struct FixtureFilter {
    /// 按标签过滤（包含任一即匹配）/ Filter by tag (any match)
    pub tags: Vec<String>,
    /// 按分组过滤 / Filter by group
    pub group: Option<String>,
    /// 只包含 backend smoke / Only include backend smoke fixtures
    pub backend_smoke_only: bool,
    /// 排除 backend smoke / Exclude backend smoke fixtures
    pub exclude_backend_smoke: bool,
    /// 按名称前缀过滤 / Filter by name prefix
    pub name_prefix: Option<String>,
}

impl FixtureFilter {
    /// 创建无过滤器 / Create no-op filter
    pub fn all() -> Self {
        Self::default()
    }

    /// 按标签过滤 / Filter by tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// 按分组过滤 / Filter by group
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    /// 只包含 backend smoke / Only include backend smoke
    pub fn backend_smoke_only(mut self) -> Self {
        self.backend_smoke_only = true;
        self
    }

    /// 排除 backend smoke / Exclude backend smoke
    pub fn exclude_backend_smoke(mut self) -> Self {
        self.exclude_backend_smoke = true;
        self
    }

    /// 按名称前缀过滤 / Filter by name prefix
    pub fn with_name_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.name_prefix = Some(prefix.into());
        self
    }

    /// 判断 fixture 是否匹配过滤条件 / Check whether fixture matches filter
    pub fn matches(
        &self,
        name: &str,
        group: &Option<String>,
        tags: &[String],
        backend_smoke: bool,
    ) -> bool {
        if self.backend_smoke_only && !backend_smoke {
            return false;
        }
        if self.exclude_backend_smoke && backend_smoke {
            return false;
        }
        if let Some(ref filter_group) = self.group {
            match group {
                Some(g) if g == filter_group => {}
                _ => return false,
            }
        }
        if !self.tags.is_empty() {
            let has_match = self.tags.iter().any(|tag| tags.contains(tag));
            if !has_match {
                return false;
            }
        }
        if let Some(ref prefix) = self.name_prefix {
            if !name.starts_with(prefix.as_str()) {
                return false;
            }
        }
        true
    }
}

#[cfg(feature = "serde")]
impl FixtureFilter {
    /// 过滤 fixture 列表 / Filter fixture list
    pub fn filter_fixtures<'a>(&self, fixtures: &'a [super::SolverDatasetFixture]) -> Vec<&'a super::SolverDatasetFixture> {
        fixtures
            .iter()
            .filter(|f| self.matches(&f.name, &f.group, &f.tags, f.backend_smoke))
            .collect()
    }
}

