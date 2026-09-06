//! 符号定义宏 / Symbol definition macros

/// 定义符号 / Define symbols
///
/// 用于快速创建 `OwnedSymbol` 变量。
/// Used to quickly create `OwnedSymbol` variables.
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::symbols;
/// use ospf_rust_math::symbol::OwnedSymbol;
///
/// // 自动生成 id
/// symbols!(x, y, z);
///
/// // 带显式 id
/// symbols!(a: 1, b: 2, c: 3);
/// ```
///
/// **注 / Note**: 此宏需要配合具体的 Symbol 实现使用。
/// 框架只提供 trait 定义，实际的 Symbol 类型由用户定义。
#[macro_export]
macro_rules! symbols {
    // 带显式 id
    ($($name:ident : $id:expr),* $(,)?) => {
        $(
            let $name = $crate::symbol::OwnedSymbol::new(
                $crate::symbol::test_utils::SimpleSymbol::with_id($id, stringify!($name))
            );
        )*
    };

    // 自动生成 id（空参数）
    ($($name:ident),* $(,)?) => {
        $(
            let $name = $crate::symbol::OwnedSymbol::new(
                $crate::symbol::test_utils::SimpleSymbol::new(stringify!($name))
            );
        )*
    };
}

/// 在测试中定义符号 / Define symbols in tests
///
/// 用于测试环境，使用测试专用的 SimpleSymbol 实现。
/// Used in test environment with test-specific SimpleSymbol implementation.
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::symbols_test;
/// use ospf_rust_math::symbol::OwnedSymbol;
///
/// symbols_test!(x, y, z);
///
/// // 现在可以使用 x, y, z 作为 OwnedSymbol
/// let _x: OwnedSymbol = x;
/// ```
#[macro_export]
macro_rules! symbols_test {
    ($($name:ident),* $(,)?) => {
        $(
            let $name = $crate::symbol::OwnedSymbol::new(
                $crate::symbol::test_utils::SimpleSymbol::new(stringify!($name))
            );
        )*
    };

    // 带显式 id
    ($($name:ident : $id:expr),* $(,)?) => {
        $(
            let $name = $crate::symbol::OwnedSymbol::new(
                $crate::symbol::test_utils::SimpleSymbol::with_id($id, stringify!($name))
            );
        )*
    };
}

/// 定义符号向量 / Define symbol vector
///
/// 创建一个包含多个符号的向量。
/// Creates a vector containing multiple symbols.
///
/// # 示例 / Examples
///
/// ```
/// use ospf_rust_math::symbols_vec;
/// use ospf_rust_math::symbol::OwnedSymbol;
///
/// let vars: Vec<OwnedSymbol> = symbols_vec![x, y, z];
/// assert_eq!(vars.len(), 3);
/// ```
#[macro_export]
macro_rules! symbols_vec {
    [$($name:ident),* $(,)?] => {
        vec![$($crate::symbol::OwnedSymbol::new(
            $crate::symbol::test_utils::SimpleSymbol::new(stringify!($name))
        )),*]
    };
}
