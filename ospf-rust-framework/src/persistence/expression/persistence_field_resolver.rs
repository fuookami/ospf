//! 持久化字段解析器
//! Persistence field resolver

use ospf_rust_math::symbol::PropertyPath;

/// 持久化字段解析器。
/// Persistence field resolver.
pub trait PersistenceFieldResolver<C> {
    /// 将领域属性路径解析为后端字段表示。
    /// Resolve a domain property path into a backend field representation.
    fn resolve_field(&self, path: &PropertyPath) -> Option<C>;
}

/// 持久化字段解析结果。
/// Persistence field resolution result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistenceFieldResolution<C> {
    /// 已解析字段 / Resolved field
    Resolved(C),
    /// 字段不存在 / Missing field
    Missing {
        /// 未解析的字段路径 / Unresolved field path
        path: String,
    },
    /// 字段存在多个候选 / Ambiguous field
    Ambiguous {
        /// 产生歧义的字段路径 / Ambiguous field path
        path: String,
        /// 候选字段 / Candidate fields
        candidates: Vec<String>,
    },
    /// 解析器配置非法 / Invalid resolver configuration
    InvalidConfiguration {
        /// 配置错误原因 / Configuration error reason
        reason: String,
    },
}

/// 带诊断信息的持久化字段解析器。
/// Persistence field resolver with diagnostics.
///
/// 详细解析接口提供与旧可空解析器等价的默认方法，因此实现者只需实现详细结果。
/// The detailed interface provides a nullable compatibility method, so implementers only need to provide detailed results.
pub trait DiagnosticPersistenceFieldResolver<C> {
    /// 解析字段并保留缺失、歧义和配置错误。
    /// Resolve a field while preserving missing, ambiguous, and configuration failures.
    fn resolve_detailed(&self, path: &PropertyPath) -> PersistenceFieldResolution<C>;

    /// 将详细结果转换为兼容的可空字段结果。
    /// Convert the detailed result into a nullable-compatible field result.
    fn resolve_field(&self, path: &PropertyPath) -> Option<C> {
        match self.resolve_detailed(path) {
            PersistenceFieldResolution::Resolved(field) => Some(field),
            PersistenceFieldResolution::Missing { .. }
            | PersistenceFieldResolution::Ambiguous { .. }
            | PersistenceFieldResolution::InvalidConfiguration { .. } => None,
        }
    }
}

impl<C, F> PersistenceFieldResolver<C> for F
where
    F: Fn(&str) -> Option<C>,
{
    fn resolve_field(&self, path: &PropertyPath) -> Option<C> {
        self(path.value())
    }
}

/// 字符串字段名解析器。
/// String field-name resolver.
pub type FieldNameResolver = dyn PersistenceFieldResolver<String>;

/// 使用 resolver 解析字段名。
/// Resolve a field name with a resolver.
pub fn resolve_field_name<R>(resolver: &R, path: impl Into<PropertyPath>) -> Option<String>
where
    R: PersistenceFieldResolver<String> + ?Sized,
{
    resolver.resolve_field(&path.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closure_resolver_maps_property_path_to_backend_field() {
        let resolver = |path: &str| match path {
            "status" => Some("user_status".to_string()),
            _ => None,
        };

        assert_eq!(
            resolve_field_name(&resolver, "status"),
            Some("user_status".to_string())
        );
        assert_eq!(resolve_field_name(&resolver, "missing"), None);
    }

    #[test]
    fn diagnostic_resolver_preserves_missing_and_ambiguous_results() {
        #[derive(Clone)]
        struct Resolver;

        impl PersistenceFieldResolver<String> for Resolver {
            fn resolve_field(&self, path: &PropertyPath) -> Option<String> {
                (path.value() == "id").then(|| "users.id".to_string())
            }
        }

        impl DiagnosticPersistenceFieldResolver<String> for Resolver {
            fn resolve_detailed(&self, path: &PropertyPath) -> PersistenceFieldResolution<String> {
                match path.value() {
                    "id" => PersistenceFieldResolution::Ambiguous {
                        path: path.value().to_string(),
                        candidates: vec!["users.id".to_string(), "legacy.id".to_string()],
                    },
                    _ => PersistenceFieldResolution::Missing {
                        path: path.value().to_string(),
                    },
                }
            }
        }

        let resolver = Resolver;
        assert!(matches!(
            resolver.resolve_detailed(&PropertyPath::parse("id")),
            PersistenceFieldResolution::Ambiguous { .. }
        ));
        assert_eq!(
            PersistenceFieldResolver::resolve_field(&resolver, &PropertyPath::parse("id")),
            Some("users.id".to_string())
        );
    }

    #[test]
    fn diagnostic_resolver_can_provide_only_detailed_results() {
        struct Resolver;

        impl DiagnosticPersistenceFieldResolver<String> for Resolver {
            fn resolve_detailed(&self, path: &PropertyPath) -> PersistenceFieldResolution<String> {
                PersistenceFieldResolution::Resolved(format!("users.{}", path.value()))
            }
        }

        let resolver = Resolver;

        assert_eq!(
            DiagnosticPersistenceFieldResolver::resolve_field(
                &resolver,
                &PropertyPath::parse("id")
            ),
            Some("users.id".to_string())
        );
    }
}
