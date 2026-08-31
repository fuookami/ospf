//! 类型级别索引定义
//! Type-level index definitions
//!
//! 提供编译期索引标记，用于爱因斯坦表示法的类型安全操作。
//! Provides compile-time index labels for type-safe Einstein notation operations.

use std::marker::PhantomData;

// ============================================================================
// IndexLabel trait - 索引标记 trait
// ============================================================================

/// 索引标记 trait / Index label trait
///
/// 用于在编译期标识张量的索引。
/// Used to identify tensor indices at compile time.
///
/// # 类型参数约束 / Type Parameter Constraints
///
/// - `Clone`: 支持克隆
/// - `Copy`: 支持复制（零大小类型）
/// - `Default`: 支持默认值
/// - `'static`: 编译期已知
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_multiarray::einsum::{IndexLabel, I, J};
///
/// // 使用内置索引类型
/// // Use built-in index types
/// fn process<I: IndexLabel, J: IndexLabel>() {
///     println!("Processing indices: {}, {}", I::NAME, J::NAME);
/// }
///
/// // 调用示例 / Call example
/// process::<I, J>();
/// ```
pub trait IndexLabel: Clone + Copy + Default + 'static {
    /// 索引名称（用于调试和显示）
    /// Index name (for debugging and display)
    const NAME: &'static str;
    
    /// 索引的唯一标识符
    /// Unique identifier for the index
    ///
    /// 用于在运行时比较索引是否相同。
    /// Used to compare indices at runtime.
    const ID: usize;
}

// ============================================================================
// 内置索引类型 / Built-in Index Types
// ============================================================================

/// 索引 i / Index i
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub struct I;

/// 索引 j / Index j
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub struct J;

/// 索引 k / Index k
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub struct K;

/// 索引 l / Index l
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub struct L;

/// 索引 m / Index m
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub struct M;

/// 索引 n / Index n
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub struct N;

impl IndexLabel for I {
    const NAME: &'static str = "i";
    const ID: usize = 0;
}

impl IndexLabel for J {
    const NAME: &'static str = "j";
    const ID: usize = 1;
}

impl IndexLabel for K {
    const NAME: &'static str = "k";
    const ID: usize = 2;
}

impl IndexLabel for L {
    const NAME: &'static str = "l";
    const ID: usize = 3;
}

impl IndexLabel for M {
    const NAME: &'static str = "m";
    const ID: usize = 4;
}

impl IndexLabel for N {
    const NAME: &'static str = "n";
    const ID: usize = 5;
}

// ============================================================================
// IndexList trait - 类型级别索引列表
// ============================================================================

/// 类型级别索引列表 trait / Type-level index list trait
///
/// 表示一组索引的有序集合，在编译期确定。
/// Represents an ordered collection of indices, determined at compile time.
///
/// # 示例 / Example
///
/// ```
/// use ospf_rust_multiarray::einsum::{IndexList, I, J, Nil, Cons, IL, IL2};
///
/// // 单索引列表
/// // Single index list
/// type Single = Cons<I, Nil>;
/// assert_eq!(Single::LEN, 1);
///
/// // 双索引列表
/// // Double index list
/// type Double = Cons<I, Cons<J, Nil>>;
/// assert_eq!(Double::LEN, 2);
///
/// // 使用便捷类型别名
/// // Using convenience type aliases
/// assert_eq!(IL::<I>::LEN, 1);
/// assert_eq!(IL2::<I, J>::LEN, 2);
/// ```
pub trait IndexList: Clone + Default + 'static {
    /// 列表长度
    /// List length
    const LEN: usize;
    
    /// 将索引列表转换为索引 ID 切片
    /// Convert index list to slice of index IDs
    fn to_ids() -> Vec<usize>;
    
    /// 将索引列表转换为名称字符串
    /// Convert index list to name string
    fn to_names() -> String;
}

// ============================================================================
// 空列表和列表节点 / Empty List and List Node
// ============================================================================

/// 空列表 / Empty list
///
/// 表示没有索引的列表，作为索引列表的终止符。
/// Represents an empty list of indices, serving as the terminator for index lists.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub struct Nil;

impl IndexList for Nil {
    const LEN: usize = 0;
    
    fn to_ids() -> Vec<usize> {
        Vec::new()
    }
    
    fn to_names() -> String {
        String::new()
    }
}

/// 列表节点 / List node
///
/// 表示一个非空的索引列表，由头元素和尾列表组成。
/// Represents a non-empty index list, consisting of a head element and a tail list.
///
/// # 类型参数 / Type Parameters
///
/// - `H`: 头元素（索引类型）
/// - `T`: 尾元素（索引列表类型）
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub struct Cons<H: IndexLabel, T: IndexList> {
    _head: PhantomData<H>,
    _tail: PhantomData<T>,
}

impl<H: IndexLabel, T: IndexList> IndexList for Cons<H, T> {
    const LEN: usize = 1 + T::LEN;
    
    fn to_ids() -> Vec<usize> {
        let mut ids = vec![H::ID];
        ids.extend(T::to_ids());
        ids
    }
    
    fn to_names() -> String {
        let mut names = H::NAME.to_string();
        let tail_names = T::to_names();
        if !tail_names.is_empty() {
            names.push_str(", ");
            names.push_str(&tail_names);
        }
        names
    }
}

// ============================================================================
// 便捷类型别名 / Convenience Type Aliases
// ============================================================================

/// 单索引列表类型别名 / Single index list type alias
pub type IL<I1> = Cons<I1, Nil>;

/// 双索引列表类型别名 / Double index list type alias
pub type IL2<I1, I2> = Cons<I1, Cons<I2, Nil>>;

/// 三索引列表类型别名 / Triple index list type alias
pub type IL3<I1, I2, I3> = Cons<I1, Cons<I2, Cons<I3, Nil>>>;

/// 四索引列表类型别名 / Four index list type alias
pub type IL4<I1, I2, I3, I4> = Cons<I1, Cons<I2, Cons<I3, Cons<I4, Nil>>>>;

/// 五索引列表类型别名 / Five index list type alias
pub type IL5<I1, I2, I3, I4, I5> = Cons<I1, Cons<I2, Cons<I3, Cons<I4, Cons<I5, Nil>>>>>;

/// 六索引列表类型别名 / Six index list type alias
pub type IL6<I1, I2, I3, I4, I5, I6> = Cons<I1, Cons<I2, Cons<I3, Cons<I4, Cons<I5, Cons<I6, Nil>>>>>>;

// ============================================================================
// 辅助函数 / Helper Functions
// ============================================================================

/// 查找两个索引列表的公共索引（求和索引）
/// Find common indices (summation indices) between two index lists
///
/// # 参数 / Arguments
///
/// - `lhs_ids`: 左操作数的索引 ID 列表
/// - `rhs_ids`: 右操作数的索引 ID 列表
///
/// # 返回 / Returns
///
/// 返回两个列表中都出现的索引 ID。
/// Returns the index IDs that appear in both lists.
pub fn find_common_indices(lhs_ids: &[usize], rhs_ids: &[usize]) -> Vec<usize> {
    let mut common = Vec::new();
    for &id in lhs_ids {
        if rhs_ids.contains(&id) && !common.contains(&id) {
            common.push(id);
        }
    }
    common
}

/// 从索引列表中移除指定索引
/// Remove specified indices from an index list
///
/// # 参数 / Arguments
///
/// - `ids`: 原索引 ID 列表
/// - `to_remove`: 要移除的索引 ID 列表
///
/// # 返回 / Returns
///
/// 返回移除指定索引后的新列表。
/// Returns the new list after removing specified indices.
pub fn remove_indices(ids: &[usize], to_remove: &[usize]) -> Vec<usize> {
    ids.iter()
        .filter(|&&id| !to_remove.contains(&id))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_label_properties() {
        assert_eq!(I::NAME, "i");
        assert_eq!(J::NAME, "j");
        assert_eq!(K::NAME, "k");
        
        assert_eq!(I::ID, 0);
        assert_eq!(J::ID, 1);
        assert_eq!(K::ID, 2);
    }

    #[test]
    fn test_index_list_length() {
        assert_eq!(Nil::LEN, 0);
        assert_eq!(IL::<I>::LEN, 1);
        assert_eq!(IL2::<I, J>::LEN, 2);
        assert_eq!(IL3::<I, J, K>::LEN, 3);
    }

    #[test]
    fn test_index_list_to_ids() {
        assert!(Nil::to_ids().is_empty());
        
        let single = IL::<I>::to_ids();
        assert_eq!(single, vec![0]);
        
        let double = IL2::<I, J>::to_ids();
        assert_eq!(double, vec![0, 1]);
        
        let triple = IL3::<I, J, K>::to_ids();
        assert_eq!(triple, vec![0, 1, 2]);
    }

    #[test]
    fn test_index_list_to_names() {
        assert!(Nil::to_names().is_empty());
        
        let single = IL::<I>::to_names();
        assert_eq!(single, "i");
        
        let double = IL2::<I, J>::to_names();
        assert_eq!(double, "i, j");
    }

    #[test]
    fn test_find_common_indices() {
        let lhs = vec![0, 1, 2];  // i, j, k
        let rhs = vec![1, 2, 3];  // j, k, l
        
        let common = find_common_indices(&lhs, &rhs);
        assert_eq!(common, vec![1, 2]);  // j, k
    }

    #[test]
    fn test_remove_indices() {
        let ids = vec![0, 1, 2, 3];  // i, j, k, l
        let to_remove = vec![1, 3];  // j, l
        
        let result = remove_indices(&ids, &to_remove);
        assert_eq!(result, vec![0, 2]);  // i, k
    }
}