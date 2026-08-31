//! 变量组合模块
//! Variable Combination Module
//!
//! 提供多维变量组合类型，用于管理具有相同类型的一组变量。
//! Provides multi-dimensional variable combination types for managing a group of variables of the same type.
//!
//! # 核心概念 / Core Concepts
//!
//! - `VariableCombination`: 多维变量组合，在创建时自动分配唯一组 ID
//!   Multi-dimensional variable combination, automatically assigned unique group ID on creation
//! - 每个组合内的变量共享相同的组 ID，通过索引区分
//!   Variables within a combination share the same group ID, distinguished by index

use super::{GenericVariableItem, VariableId, VariableTypeTrait, new_group_id};
use ospf_rust_multiarray::{
    MultiArray, MultiArrayBuilder,
    shape::{AbstractShape, Shape},
};

// ============================================================================
// VariableCombination - 多维变量组合
// ============================================================================

/// 多维变量组合 / Multi-dimensional Variable Combination
///
/// 在创建时自动分配唯一组 ID 的多维变量数组。
/// Multi-dimensional variable array with automatically assigned unique group ID on creation.
///
/// # 设计说明 / Design Notes
///
/// - 组 ID 在创建时由全局 ID 生成器自动分配
/// - Group ID is automatically assigned by global ID generator on creation
/// - 每个变量的 `VariableId` 由组 ID 和数组索引组成
/// - Each variable's `VariableId` consists of group ID and array index
///
/// # 类型参数 / Type Parameters
///
/// - `VT`: 变量类型标记（实现 `VariableTypeTrait`）
///   Variable type marker (implements `VariableTypeTrait`)
/// - `S`: 形状类型（实现 `AbstractShape`）
///   Shape type (implements `AbstractShape`)
///
/// # 示例 / Examples
///
/// ```rust
/// use ospf_rust_core::variable::{VariableCombination, Binary, VariableId};
/// use ospf_rust_multiarray::Shape;
///
/// // 创建一个 5 元素的一维变量组合
/// // Create a 5-element 1D variable combination
/// let vars: VariableCombination<Binary, _> = VariableCombination::new(Shape::<1>::new([5]), "x");
/// assert_eq!(vars.len(), 5);
///
/// // 每个变量有唯一的 ID
/// // Each variable has a unique ID
/// let id0 = vars[0].id();
/// let id1 = vars[1].id();
/// assert_ne!(id0, id1);
/// assert_eq!(id0.group_id, id1.group_id); // 相同组 / Same group
/// ```
pub struct VariableCombination<VT: VariableTypeTrait, S: AbstractShape> {
    /// 变量数组 / Variable array
    variables: MultiArray<GenericVariableItem<VT>, S>,
    /// 组 ID / Group ID
    group_id: usize,
    /// 变量名称前缀 / Variable name prefix
    name_prefix: String,
}

impl<VT: VariableTypeTrait, S: AbstractShape> VariableCombination<VT, S> {
    /// 创建新的变量组合 / Create new variable combination
    ///
    /// # 参数 / Parameters
    ///
    /// - `shape`: 数组形状 / Array shape
    /// - `name_prefix`: 变量名称前缀 / Variable name prefix
    ///
    /// # 示例 / Examples
    ///
    /// ```rust
    /// use ospf_rust_core::variable::{VariableCombination, Binary};
    /// use ospf_rust_multiarray::Shape;
    ///
    /// let vars: VariableCombination<Binary, _> = VariableCombination::new(Shape::<1>::new([5]), "x");
    /// ```
    pub fn new(shape: S, name_prefix: &str) -> Self
    where
        VT: Clone,
    {
        let group_id = new_group_id();

        let variables = MultiArrayBuilder::new_by(shape, |index, _vector| {
            let id = VariableId::new(group_id, index);
            let name = format!("{}_{}", name_prefix, index);
            GenericVariableItem::create(id, &name)
        });

        Self {
            variables,
            group_id,
            name_prefix: name_prefix.to_string(),
        }
    }

    /// 使用自定义名称生成器创建变量组合 / Create variable combination with custom name generator
    ///
    /// # 参数 / Parameters
    ///
    /// - `shape`: 数组形状 / Array shape
    /// - `name_prefix`: 变量名称前缀 / Variable name prefix
    /// - `name_gen`: 名称生成函数，接收索引和向量坐标，返回变量名后缀
    ///   Name generator function, receives index and vector coordinates, returns variable name suffix
    pub fn with_name_generator<F>(shape: S, name_prefix: &str, name_gen: F) -> Self
    where
        VT: Clone,
        F: Fn(usize, &S::VectorType) -> String,
    {
        let group_id = new_group_id();

        let variables = MultiArrayBuilder::new_by(shape, |index, vector| {
            let id = VariableId::new(group_id, index);
            let suffix = name_gen(index, vector);
            let name = format!("{}_{}", name_prefix, suffix);
            GenericVariableItem::create(id, &name)
        });

        Self {
            variables,
            group_id,
            name_prefix: name_prefix.to_string(),
        }
    }

    /// 使用指定组 ID 创建变量组合 / Create variable combination with specified group ID
    ///
    /// # 参数 / Parameters
    ///
    /// - `shape`: 数组形状 / Array shape
    /// - `name_prefix`: 变量名称前缀 / Variable name prefix
    /// - `group_id`: 指定的组 ID / Specified group ID
    pub fn with_group_id(shape: S, name_prefix: &str, group_id: usize) -> Self
    where
        VT: Clone,
    {
        let variables = MultiArrayBuilder::new_by(shape, |index, _vector| {
            let id = VariableId::new(group_id, index);
            let name = format!("{}_{}", name_prefix, index);
            GenericVariableItem::create(id, &name)
        });

        Self {
            variables,
            group_id,
            name_prefix: name_prefix.to_string(),
        }
    }

    /// 获取组 ID / Get group ID
    pub fn group_id(&self) -> usize {
        self.group_id
    }

    /// 获取变量数量 / Get variable count
    pub fn len(&self) -> usize {
        self.variables.len()
    }

    /// 检查是否为空 / Check if empty
    pub fn is_empty(&self) -> bool {
        self.variables.is_empty()
    }

    /// 获取名称前缀 / Get name prefix
    pub fn name_prefix(&self) -> &str {
        &self.name_prefix
    }

    /// 获取形状引用 / Get shape reference
    pub fn shape(&self) -> &S {
        &self.variables.shape
    }

    /// 获取所有变量 ID / Get all variable IDs
    pub fn ids(&self) -> Vec<VariableId> {
        self.variables.iter().map(|v| v.id()).collect()
    }

    /// 获取变量数组引用 / Get variable array reference
    pub fn as_array(&self) -> &MultiArray<GenericVariableItem<VT>, S> {
        &self.variables
    }

    /// 消耗 self，返回变量数组 / Consume self, return variable array
    pub fn into_array(self) -> MultiArray<GenericVariableItem<VT>, S> {
        self.variables
    }

    /// 获取变量迭代器 / Get variable iterator
    pub fn iter(&self) -> impl Iterator<Item = &GenericVariableItem<VT>> {
        self.variables.iter()
    }
}

impl<VT: VariableTypeTrait, S: AbstractShape + Clone> Clone for VariableCombination<VT, S>
where
    VT: Clone,
{
    fn clone(&self) -> Self {
        Self {
            variables: MultiArray::clone(&self.variables),
            group_id: self.group_id,
            name_prefix: self.name_prefix.clone(),
        }
    }
}

impl<VT: VariableTypeTrait, S: AbstractShape> std::fmt::Debug for VariableCombination<VT, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VariableCombination")
            .field("group_id", &self.group_id)
            .field("name_prefix", &self.name_prefix)
            .field("len", &self.variables.len())
            .finish()
    }
}

impl<VT: VariableTypeTrait, S: AbstractShape> std::ops::Deref for VariableCombination<VT, S> {
    type Target = MultiArray<GenericVariableItem<VT>, S>;

    fn deref(&self) -> &Self::Target {
        &self.variables
    }
}

impl<VT: VariableTypeTrait, S: AbstractShape> std::ops::DerefMut for VariableCombination<VT, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.variables
    }
}

// ============================================================================
// 便捷类型别名 / Convenience Type Aliases
// ============================================================================

/// 一维变量组合 / 1D Variable Combination
pub type VariableCombination1D<VT> = VariableCombination<VT, Shape<1>>;

/// 二维变量组合 / 2D Variable Combination
pub type VariableCombination2D<VT> = VariableCombination<VT, Shape<2>>;

/// 三维变量组合 / 3D Variable Combination
pub type VariableCombination3D<VT> = VariableCombination<VT, Shape<3>>;

// ============================================================================
// 特定类型的别名 / Type-specific Aliases
// ============================================================================

use super::variable_type::{
    BalancedTernary, Binary, Continuous, Integer, Percentage, Ternary, UContinuous, UInteger,
};

/// 二进制变量组合 / Binary variable combination
pub type BinaryCombination<S> = VariableCombination<Binary, S>;

/// 三元变量组合 / Ternary variable combination
pub type TernaryCombination<S> = VariableCombination<Ternary, S>;

/// 平衡三元变量组合 / Balanced ternary variable combination
pub type BalancedTernaryCombination<S> = VariableCombination<BalancedTernary, S>;

/// 百分比变量组合 / Percentage variable combination
pub type PercentageCombination<S> = VariableCombination<Percentage, S>;

/// 整数变量组合 / Integer variable combination
pub type IntegerCombination<S> = VariableCombination<Integer, S>;

/// 无符号整数变量组合 / Unsigned integer variable combination
pub type UIntegerCombination<S> = VariableCombination<UInteger, S>;

/// 连续变量组合 / Continuous variable combination
pub type ContinuousCombination<S> = VariableCombination<Continuous, S>;

/// 无符号连续变量组合 / Unsigned continuous variable combination
pub type UContinuousCombination<S> = VariableCombination<UContinuous, S>;

// ---------------------------------------------------------------------------
// 一维特定类型别名 / 1D Type-specific Aliases
// ---------------------------------------------------------------------------

/// 一维二进制变量组合 / 1D Binary variable combination
pub type BinaryCombination1D = BinaryCombination<Shape<1>>;

/// 一维连续变量组合 / 1D Continuous variable combination
pub type ContinuousCombination1D = ContinuousCombination<Shape<1>>;

/// 一维整数变量组合 / 1D Integer variable combination
pub type IntegerCombination1D = IntegerCombination<Shape<1>>;

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variable::BinaryVariableItem;
    use ospf_rust_multiarray::Shape;

    #[test]
    fn test_variable_combination_creation() {
        let vars: BinaryCombination1D = VariableCombination::new(Shape::new([5]), "x");

        assert_eq!(vars.len(), 5);
        assert!(!vars.is_empty());
        assert_eq!(vars.name_prefix(), "x");
    }

    #[test]
    fn test_unique_ids() {
        let vars: BinaryCombination1D = VariableCombination::new(Shape::new([5]), "x");

        // 所有变量应该有相同的组 ID / All variables should have the same group ID
        let group_ids: Vec<usize> = vars.iter().map(|v| v.id().group_id).collect();
        assert!(group_ids.windows(2).all(|w| w[0] == w[1]));

        // 所有变量应该有不同的索引 / All variables should have different indices
        let indices: Vec<usize> = vars.iter().map(|v| v.id().index_in_group).collect();
        for i in 0..5 {
            assert_eq!(indices[i], i);
        }
    }

    #[test]
    fn test_different_combinations_have_different_group_ids() {
        let vars1: BinaryCombination1D = VariableCombination::new(Shape::new([3]), "x");
        let vars2: BinaryCombination1D = VariableCombination::new(Shape::new([3]), "y");

        // 不同的组合应该有不同的组 ID / Different combinations should have different group IDs
        assert_ne!(vars1.group_id(), vars2.group_id());
    }

    #[test]
    fn test_variable_names() {
        let vars: BinaryCombination1D = VariableCombination::new(Shape::new([3]), "select");

        assert_eq!(vars[0].name(), "select_0");
        assert_eq!(vars[1].name(), "select_1");
        assert_eq!(vars[2].name(), "select_2");
    }

    #[test]
    fn test_2d_combination() {
        let vars: VariableCombination2D<Binary> =
            VariableCombination::new(Shape::new([2, 3]), "matrix");

        assert_eq!(vars.len(), 6);
        assert_eq!(vars.shape().shape(), [2, 3].as_slice());
    }

    #[test]
    fn test_with_name_generator() {
        let vars: BinaryCombination1D =
            VariableCombination::with_name_generator(Shape::new([3]), "company", |index, _vec| {
                format!("C{}", index + 1)
            });

        assert_eq!(vars[0].name(), "company_C1");
        assert_eq!(vars[1].name(), "company_C2");
        assert_eq!(vars[2].name(), "company_C3");
    }

    #[test]
    fn test_ids_method() {
        let vars: BinaryCombination1D = VariableCombination::new(Shape::new([3]), "x");
        let ids = vars.ids();

        assert_eq!(ids.len(), 3);
        for (i, id) in ids.iter().enumerate() {
            assert_eq!(id.index_in_group, i);
        }
    }

    #[test]
    fn test_deref_access() {
        let vars: BinaryCombination1D = VariableCombination::new(Shape::new([3]), "x");

        // 通过解引用访问 / Access via deref
        let first_var = &vars[0];
        assert_eq!(first_var.name(), "x_0");
    }

    #[test]
    fn test_single_and_combination_share_global_id_generator() {
        let single_before = BinaryVariableItem::auto("single_before");
        let combination: BinaryCombination1D = VariableCombination::new(Shape::new([2]), "combo");
        let single_after = BinaryVariableItem::auto("single_after");

        let single_before_group = single_before.id().group_id;
        let combination_group = combination.group_id();
        let single_after_group = single_after.id().group_id;

        // 三者来自同一全局生成器，因此按创建顺序严格递增。
        // The three IDs come from the same global generator and strictly increase by creation order.
        assert!(single_before_group < combination_group);
        assert!(combination_group < single_after_group);

        // 组合内变量应与组合共享组 ID。
        // Variables inside the combination must share the combination group ID.
        assert!(
            combination
                .iter()
                .all(|variable| variable.id().group_id == combination_group)
        );
    }
}
