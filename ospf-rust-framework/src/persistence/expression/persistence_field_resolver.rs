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
}
