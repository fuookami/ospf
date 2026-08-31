//! Cornucopia 持久化后端
//! Cornucopia persistence backend

/// Cornucopia 后端标记类型。
/// Cornucopia backend marker type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CornucopiaBackend;

/// Cornucopia 生成查询绑定。
/// Cornucopia generated query binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CornucopiaQueryBinding {
    pub query_name: String,
    pub sql_file: Option<String>,
}

impl CornucopiaQueryBinding {
    /// 创建查询绑定。
    /// Create a query binding.
    pub fn new(query_name: impl Into<String>) -> Self {
        Self {
            query_name: query_name.into(),
            sql_file: None,
        }
    }

    /// 设置 SQL 文件。
    /// Set SQL file.
    pub fn with_sql_file(mut self, sql_file: impl Into<String>) -> Self {
        self.sql_file = Some(sql_file.into());
        self
    }
}

/// Cornucopia 仓储函数绑定集合。
/// Cornucopia repository function bindings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CornucopiaRepositoryBindings {
    pub find: Option<CornucopiaQueryBinding>,
    pub count: Option<CornucopiaQueryBinding>,
    pub update: Option<CornucopiaQueryBinding>,
    pub delete: Option<CornucopiaQueryBinding>,
}

impl CornucopiaRepositoryBindings {
    /// 创建空绑定集合。
    /// Create empty bindings.
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置 find 绑定。
    /// Set find binding.
    pub fn with_find(mut self, binding: CornucopiaQueryBinding) -> Self {
        self.find = Some(binding);
        self
    }

    /// 设置 count 绑定。
    /// Set count binding.
    pub fn with_count(mut self, binding: CornucopiaQueryBinding) -> Self {
        self.count = Some(binding);
        self
    }

    /// 设置 update 绑定。
    /// Set update binding.
    pub fn with_update(mut self, binding: CornucopiaQueryBinding) -> Self {
        self.update = Some(binding);
        self
    }

    /// 设置 delete 绑定。
    /// Set delete binding.
    pub fn with_delete(mut self, binding: CornucopiaQueryBinding) -> Self {
        self.delete = Some(binding);
        self
    }

    /// 判断是否包含完整仓储绑定。
    /// Check whether all repository bindings are present.
    pub fn is_complete(&self) -> bool {
        self.find.is_some()
            && self.count.is_some()
            && self.update.is_some()
            && self.delete.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cornucopia_bindings_track_generated_query_functions() {
        let bindings = CornucopiaRepositoryBindings::new()
            .with_find(CornucopiaQueryBinding::new("find_users").with_sql_file("users.sql"))
            .with_count(CornucopiaQueryBinding::new("count_users"))
            .with_update(CornucopiaQueryBinding::new("update_users"))
            .with_delete(CornucopiaQueryBinding::new("delete_users"));

        assert!(bindings.is_complete());
        assert_eq!(
            bindings.find.unwrap().sql_file,
            Some("users.sql".to_string())
        );
    }
}
