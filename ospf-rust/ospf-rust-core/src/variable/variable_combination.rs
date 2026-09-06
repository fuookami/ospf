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

use super::{VariableId, VariableItem, VariableRange, VariableTypeTrait, new_group_id};
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
    variables: MultiArray<VariableItem<VT>, S>,
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
            VariableItem::create(id, &name)
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
            VariableItem::create(id, &name)
        });

        Self {
            variables,
            group_id,
            name_prefix: name_prefix.to_string(),
        }
    }

    /// 使用自定义名称和范围生成器创建变量组合。
    /// Create variable combination with custom name and range generators.
    ///
    /// # 参数 / Parameters
    ///
    /// - `shape`: 数组形状 / Array shape
    /// - `name_prefix`: 变量名称前缀 / Variable name prefix
    /// - `name_gen`: 名称生成函数，接收索引和向量坐标，返回变量名后缀
    ///   Name generator, receives linear index and vector coordinate, returns variable name suffix
    /// - `range_gen`: 范围生成函数，接收索引和向量坐标，返回变量范围
    ///   Range generator, receives linear index and vector coordinate, returns variable range
    pub fn with_name_and_range_generator<N, R>(
        shape: S,
        name_prefix: &str,
        name_gen: N,
        range_gen: R,
    ) -> Self
    where
        VT: Clone,
        N: Fn(usize, &S::VectorType) -> String,
        R: Fn(usize, &S::VectorType) -> VariableRange<VT::Value>,
    {
        let group_id = new_group_id();

        let variables = MultiArrayBuilder::new_by(shape, |index, vector| {
            let id = VariableId::new(group_id, index);
            let suffix = name_gen(index, vector);
            let name = format!("{}_{}", name_prefix, suffix);
            VariableItem::with_range(id, &name, range_gen(index, vector))
        });

        Self {
            variables,
            group_id,
            name_prefix: name_prefix.to_string(),
        }
    }

    /// 使用自定义范围生成器创建变量组合。
    /// Create variable combination with custom range generator.
    pub fn with_range_generator<R>(shape: S, name_prefix: &str, range_gen: R) -> Self
    where
        VT: Clone,
        R: Fn(usize, &S::VectorType) -> VariableRange<VT::Value>,
    {
        Self::with_name_and_range_generator(
            shape,
            name_prefix,
            |index, _vector| index.to_string(),
            range_gen,
        )
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
            VariableItem::create(id, &name)
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
    pub fn as_array(&self) -> &MultiArray<VariableItem<VT>, S> {
        &self.variables
    }

    /// 消耗 self，返回变量数组 / Consume self, return variable array
    pub fn into_array(self) -> MultiArray<VariableItem<VT>, S> {
        self.variables
    }

    /// 获取变量迭代器 / Get variable iterator
    pub fn iter(&self) -> impl Iterator<Item = &VariableItem<VT>> {
        self.variables.iter()
    }

    /// 对固定前缀索引的某维度求和，返回模型级 `Linear<V>` 多项式。
    /// Sum across a dimension, returning model-level `Linear<V>` polynomial.
    ///
    /// 使用 `index_array`（由 `register_combination` 返回）将每个变量映射到模型 token 索引，
    /// 构造 `LinearMonomial<V>` 的 `var_index`。
    ///
    /// Uses `index_array` (returned by `register_combination`) to map each variable
    /// to its model token index for `LinearMonomial<V>::var_index`.
    ///
    /// # 参数 / Parameters
    ///
    /// - `fixed_indices`: 固定维度的索引值，长度必须等于 `S::DIMENSION - 1`
    ///   Fixed dimension index values, length must equal `S::DIMENSION - 1`
    /// - `dim`: 要求和的维度（0-based）
    ///   Dimension to sum across (0-based)
    /// - `index_array`: 模型索引数组，由 `register_combination` 返回
    ///   Model index array, returned by `register_combination`
    /// - `coefficient`: 每个单项式的系数，通常为 `1.0`
    ///   Coefficient for each monomial, typically `1.0`
    /// - `zero`: 常数项，通常为 `0.0`
    ///   Constant term, typically `0.0`
    ///
    /// # 示例 / Examples
    ///
    /// ```rust
    /// use ospf_rust_core::variable::{VariableCombination, UContinuous};
    /// use ospf_rust_core::model::MetaModel;
    /// use ospf_rust_multiarray::Shape;
    ///
    /// let mut model = MetaModel::<f64>::new("test");
    /// let vars: VariableCombination<UContinuous, _> =
    ///     VariableCombination::new(Shape::<2>::new([3, 4]), "y");
    /// let y_idx = model.register_combination(&vars).unwrap();
    /// // 对第 1 行（固定 dim 0 = 1）的 dim 1 求和
    /// let poly = vars.sum_along_dimension(&[1], 1, &y_idx, 1.0_f64, 0.0_f64);
    /// assert_eq!(poly.monomials().len(), 4);
    /// ```
    pub fn sum_along_dimension<V>(
        &self,
        fixed_indices: &[usize],
        dim: usize,
        index_array: &MultiArray<usize, S>,
        coefficient: V,
        zero: V,
    ) -> crate::symbol::flatten::Linear<V>
    where
        V: Clone + std::fmt::Debug + Send + Sync + 'static,
        VT: Clone,
    {
        let ndim = self.variables.shape.dimension();
        assert_eq!(
            fixed_indices.len(),
            ndim - 1,
            "fixed_indices length must be S::DIMENSION - 1 (expected {}, got {})",
            ndim - 1,
            fixed_indices.len()
        );
        assert!(dim < ndim, "dim {} out of range (ndim = {})", dim, ndim);

        use crate::symbol::flatten::{
            Linear as ModelLinear, LinearMonomial as ModelLinearMonomial,
        };

        let mut monomials = Vec::new();
        for (linear, vector, _item) in self.variables.enumerate() {
            // 检查除 dim 维度外的所有坐标是否与 fixed_indices 匹配
            let mut fixed_idx = 0;
            let matches = (0..ndim).all(|d| {
                if d == dim {
                    true
                } else {
                    let expected = fixed_indices[fixed_idx];
                    fixed_idx += 1;
                    vector[d] == expected
                }
            });

            if matches {
                let model_index = index_array[linear];
                monomials.push(ModelLinearMonomial::new(coefficient.clone(), model_index));
            }
        }

        ModelLinear::new(monomials, zero)
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
    type Target = MultiArray<VariableItem<VT>, S>;

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

/// 四维变量组合 / 4D Variable Combination
pub type VariableCombination4D<VT> = VariableCombination<VT, Shape<4>>;

// ============================================================================
// 特定类型的别名 / Type-specific Aliases
// ============================================================================

use super::variable_type::{
    BalancedTernary, Binary, Continuous, Integer, Percentage, Ternary, UContinuous, UInteger,
};

/// 二进制变量组合 / Binary variable combination
pub type BinaryCombination<S> = VariableCombination<Binary, S>;

/// 二进制变量组合（Kotlin 概念对齐命名）/ Binary variable combination (Kotlin-aligned concept name)
pub type BinaryVariable<S> = VariableCombination<Binary, S>;

/// 三元变量组合 / Ternary variable combination
pub type TernaryCombination<S> = VariableCombination<Ternary, S>;

/// 三元变量组合（Kotlin 概念对齐命名）/ Ternary variable combination (Kotlin-aligned concept name)
pub type TernaryVariable<S> = VariableCombination<Ternary, S>;

/// 平衡三元变量组合 / Balanced ternary variable combination
pub type BalancedTernaryCombination<S> = VariableCombination<BalancedTernary, S>;

/// 平衡三元变量组合（Kotlin 概念对齐命名）/ Balanced ternary variable combination (Kotlin-aligned concept name)
pub type BalancedTernaryVariable<S> = VariableCombination<BalancedTernary, S>;

/// 百分比变量组合 / Percentage variable combination
pub type PercentageCombination<S> = VariableCombination<Percentage, S>;

/// 百分比变量组合（Kotlin 概念对齐命名）/ Percentage variable combination (Kotlin-aligned concept name)
pub type PercentageVariable<S> = VariableCombination<Percentage, S>;

/// 整数变量组合 / Integer variable combination
pub type IntegerCombination<S> = VariableCombination<Integer, S>;

/// 整数变量组合（Kotlin 概念对齐命名）/ Integer variable combination (Kotlin-aligned concept name)
pub type IntegerVariable<S> = VariableCombination<Integer, S>;

/// 无符号整数变量组合 / Unsigned integer variable combination
pub type UIntegerCombination<S> = VariableCombination<UInteger, S>;

/// 无符号整数变量组合（Kotlin 概念对齐命名）/ Unsigned integer variable combination (Kotlin-aligned concept name)
pub type UIntegerVariable<S> = VariableCombination<UInteger, S>;

/// 连续变量组合 / Continuous variable combination
pub type ContinuousCombination<S> = VariableCombination<Continuous, S>;

/// 连续变量组合（Kotlin 概念对齐命名）/ Continuous variable combination (Kotlin-aligned concept name)
pub type ContinuousVariable<S> = VariableCombination<Continuous, S>;

/// 无符号连续变量组合 / Unsigned continuous variable combination
pub type UContinuousCombination<S> = VariableCombination<UContinuous, S>;

/// 无符号连续变量组合（Kotlin 概念对齐命名）/ Unsigned continuous variable combination (Kotlin-aligned concept name)
pub type UContinuousVariable<S> = VariableCombination<UContinuous, S>;

// ---------------------------------------------------------------------------
// 一维特定类型别名 / 1D Type-specific Aliases
// ---------------------------------------------------------------------------

/// 一维二进制变量组合 / 1D Binary variable combination
pub type BinaryCombination1D = BinaryCombination<Shape<1>>;

/// 一维二进制变量组合 / 1D Binary variable combination
pub type BinaryVariable1D = BinaryVariable<Shape<1>>;

/// 二维二进制变量组合 / 2D Binary variable combination
pub type BinaryVariable2D = BinaryVariable<Shape<2>>;

/// 三维二进制变量组合 / 3D Binary variable combination
pub type BinaryVariable3D = BinaryVariable<Shape<3>>;

/// 四维二进制变量组合 / 4D Binary variable combination
pub type BinaryVariable4D = BinaryVariable<Shape<4>>;

/// 一维三元变量组合 / 1D Ternary variable combination
pub type TernaryVariable1D = TernaryVariable<Shape<1>>;

/// 二维三元变量组合 / 2D Ternary variable combination
pub type TernaryVariable2D = TernaryVariable<Shape<2>>;

/// 三维三元变量组合 / 3D Ternary variable combination
pub type TernaryVariable3D = TernaryVariable<Shape<3>>;

/// 四维三元变量组合 / 4D Ternary variable combination
pub type TernaryVariable4D = TernaryVariable<Shape<4>>;

/// 一维平衡三元变量组合 / 1D Balanced ternary variable combination
pub type BalancedTernaryVariable1D = BalancedTernaryVariable<Shape<1>>;

/// 二维平衡三元变量组合 / 2D Balanced ternary variable combination
pub type BalancedTernaryVariable2D = BalancedTernaryVariable<Shape<2>>;

/// 三维平衡三元变量组合 / 3D Balanced ternary variable combination
pub type BalancedTernaryVariable3D = BalancedTernaryVariable<Shape<3>>;

/// 四维平衡三元变量组合 / 4D Balanced ternary variable combination
pub type BalancedTernaryVariable4D = BalancedTernaryVariable<Shape<4>>;

/// 一维百分比变量组合 / 1D Percentage variable combination
pub type PercentageVariable1D = PercentageVariable<Shape<1>>;

/// 二维百分比变量组合 / 2D Percentage variable combination
pub type PercentageVariable2D = PercentageVariable<Shape<2>>;

/// 三维百分比变量组合 / 3D Percentage variable combination
pub type PercentageVariable3D = PercentageVariable<Shape<3>>;

/// 四维百分比变量组合 / 4D Percentage variable combination
pub type PercentageVariable4D = PercentageVariable<Shape<4>>;

/// 一维连续变量组合 / 1D Continuous variable combination
pub type ContinuousCombination1D = ContinuousCombination<Shape<1>>;

/// 一维连续变量组合 / 1D Continuous variable combination
pub type ContinuousVariable1D = ContinuousVariable<Shape<1>>;

/// 二维连续变量组合 / 2D Continuous variable combination
pub type ContinuousVariable2D = ContinuousVariable<Shape<2>>;

/// 三维连续变量组合 / 3D Continuous variable combination
pub type ContinuousVariable3D = ContinuousVariable<Shape<3>>;

/// 四维连续变量组合 / 4D Continuous variable combination
pub type ContinuousVariable4D = ContinuousVariable<Shape<4>>;

/// 一维整数变量组合 / 1D Integer variable combination
pub type IntegerCombination1D = IntegerCombination<Shape<1>>;

/// 一维整数变量组合 / 1D Integer variable combination
pub type IntegerVariable1D = IntegerVariable<Shape<1>>;

/// 二维整数变量组合 / 2D Integer variable combination
pub type IntegerVariable2D = IntegerVariable<Shape<2>>;

/// 三维整数变量组合 / 3D Integer variable combination
pub type IntegerVariable3D = IntegerVariable<Shape<3>>;

/// 四维整数变量组合 / 4D Integer variable combination
pub type IntegerVariable4D = IntegerVariable<Shape<4>>;

/// 一维无符号整数变量组合 / 1D Unsigned integer variable combination
pub type UIntegerVariable1D = UIntegerVariable<Shape<1>>;

/// 二维无符号整数变量组合 / 2D Unsigned integer variable combination
pub type UIntegerVariable2D = UIntegerVariable<Shape<2>>;

/// 三维无符号整数变量组合 / 3D Unsigned integer variable combination
pub type UIntegerVariable3D = UIntegerVariable<Shape<3>>;

/// 四维无符号整数变量组合 / 4D Unsigned integer variable combination
pub type UIntegerVariable4D = UIntegerVariable<Shape<4>>;

/// 一维无符号连续变量组合 / 1D Unsigned continuous variable combination
pub type UContinuousVariable1D = UContinuousVariable<Shape<1>>;

/// 二维无符号连续变量组合 / 2D Unsigned continuous variable combination
pub type UContinuousVariable2D = UContinuousVariable<Shape<2>>;

/// 三维无符号连续变量组合 / 3D Unsigned continuous variable combination
pub type UContinuousVariable3D = UContinuousVariable<Shape<3>>;

/// 四维无符号连续变量组合 / 4D Unsigned continuous variable combination
pub type UContinuousVariable4D = UContinuousVariable<Shape<4>>;

// ============================================================================
// 便捷构造函数 / Convenience Constructors
// ============================================================================

/// 创建二进制变量组合 / Create a binary variable combination
pub fn binary_variables<S>(shape: S, name: &str) -> BinaryVariable<S>
where
    S: AbstractShape,
{
    VariableCombination::new(shape, name)
}

/// 创建三元变量组合 / Create a ternary variable combination
pub fn ternary_variables<S>(shape: S, name: &str) -> TernaryVariable<S>
where
    S: AbstractShape,
{
    VariableCombination::new(shape, name)
}

/// 创建平衡三元变量组合 / Create a balanced ternary variable combination
pub fn balanced_ternary_variables<S>(shape: S, name: &str) -> BalancedTernaryVariable<S>
where
    S: AbstractShape,
{
    VariableCombination::new(shape, name)
}

/// 创建百分比变量组合 / Create a percentage variable combination
pub fn percentage_variables<S>(shape: S, name: &str) -> PercentageVariable<S>
where
    S: AbstractShape,
{
    VariableCombination::new(shape, name)
}

/// 创建整数变量组合 / Create an integer variable combination
pub fn integer_variables<S>(shape: S, name: &str) -> IntegerVariable<S>
where
    S: AbstractShape,
{
    VariableCombination::new(shape, name)
}

/// 创建无符号整数变量组合 / Create an unsigned integer variable combination
pub fn unsigned_integer_variables<S>(shape: S, name: &str) -> UIntegerVariable<S>
where
    S: AbstractShape,
{
    VariableCombination::new(shape, name)
}

/// 创建连续变量组合 / Create a continuous variable combination
pub fn continuous_variables<S>(shape: S, name: &str) -> ContinuousVariable<S>
where
    S: AbstractShape,
{
    VariableCombination::new(shape, name)
}

/// 创建无符号连续变量组合 / Create an unsigned continuous variable combination
pub fn unsigned_continuous_variables<S>(shape: S, name: &str) -> UContinuousVariable<S>
where
    S: AbstractShape,
{
    VariableCombination::new(shape, name)
}

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
        for (i, index) in indices.iter().enumerate() {
            assert_eq!(*index, i);
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
    fn test_kotlin_aligned_aliases_and_helpers() {
        let binary: BinaryVariable2D = binary_variables(Shape::new([2, 3]), "x");
        let continuous: ContinuousVariable3D = continuous_variables(Shape::new([2, 2, 2]), "y");
        let integer: IntegerVariable4D = integer_variables(Shape::new([1, 1, 1, 2]), "z");

        assert_eq!(binary.len(), 6);
        assert_eq!(binary[&[1, 2]].name(), "x_5");
        assert_eq!(continuous.len(), 8);
        assert_eq!(integer.len(), 2);
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
    fn test_with_name_and_range_generator() {
        let vars: VariableCombination2D<Binary> =
            VariableCombination::with_name_and_range_generator(
                Shape::new([2, 2]),
                "arc",
                |_index, vec| format!("{}_{}", vec[0], vec[1]),
                |_index, vec| {
                    if vec[0] == vec[1] {
                        VariableRange::fixed(0.0)
                    } else {
                        Binary::default_range()
                    }
                },
            );

        assert_eq!(vars[&[0, 1]].name(), "arc_0_1");
        assert_eq!(vars[&[0, 0]].range(), &VariableRange::fixed(0.0));
        assert_eq!(vars[&[1, 0]].range(), &Binary::default_range());
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
