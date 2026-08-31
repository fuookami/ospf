//! Gantt 建模适配器 / Gantt modeling adapters
//!
//! 提供一维/二维变量集合、线性表达式集合和结果提取的通用封装。
//! Provides generic 1D/2D variable arrays, linear expression arrays,
//! and solution extraction helpers for Gantt scheduling modeling.
//!
//! # 核心抽象 / Core Abstractions
//!
//! - [`IndexedVariableArray1`]: 一维变量集合（领域键索引）/ 1D variable array indexed by domain key
//! - [`IndexedVariableArray2`]: 二维变量集合（双领域键索引）/ 2D variable array indexed by two domain keys
//! - [`ModelComponent`]: 模型组件 trait / Model component trait
//! - [`extract_value`], [`extract_binary`]: 结果提取 / Solution extraction helpers

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::expression_symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::variable::{
    VariableCombination, VariableItem, VariableTypeTrait,
    Binary,
};
use ospf_rust_multiarray::shape::Shape;

use crate::GanttResult;
use crate::GanttError;

// ============================================================================
// 符号 ID 生成器 / Symbol ID Generator
// ============================================================================

/// 甘特排程专用中间符号 ID 生成器命名空间起点
/// Gantt scheduling-specific intermediate symbol ID namespace start
///
/// 使用较高命名空间（20 亿），避免与 core 框架的符号 ID 冲突：
/// - core 框架使用 `1_000_000_000` 起始命名空间
/// - 甘特排程使用 `2_000_000_000` 起始命名空间
///
/// Uses a high namespace (2B) to avoid collision with core framework's symbol IDs:
/// - Core framework uses `1_000_000_000` starting namespace
/// - Gantt scheduling uses `2_000_000_000` starting namespace
const GANTT_SYMBOL_ID_NAMESPACE: u64 = 2_000_000_000;

static NEXT_GANTT_SYMBOL_ID: AtomicU64 = AtomicU64::new(GANTT_SYMBOL_ID_NAMESPACE);

/// 生成唯一的甘特中间符号 ID / Generate unique Gantt intermediate symbol ID
pub fn next_gantt_symbol_id() -> u64 {
    NEXT_GANTT_SYMBOL_ID.fetch_add(1, Ordering::Relaxed)
}

// ============================================================================
// 一维索引变量集合 / 1D Indexed Variable Array
// ============================================================================

/// 一维索引变量集合 / 1D indexed variable array
///
/// 将领域键映射到 `VariableCombination` 中的变量项和模型注册索引。
/// Maps domain keys to variable items in `VariableCombination` and model registration indices.
///
/// # 类型参数 / Type Parameters
///
/// - `K`: 领域键类型 / Domain key type
/// - `VT`: 变量类型标记 / Variable type marker (Binary, UContinuous, etc.)
#[derive(Debug)]
pub struct IndexedVariableArray1<K, VT: VariableTypeTrait> {
    /// 集合名称 / Collection name
    pub name: String,
    /// 底层变量组合 / Underlying variable combination
    combination: VariableCombination<VT, Shape<1>>,
    /// 领域键 → 组合内线性索引 / Domain key → linear index in combination
    key_to_linear_index: HashMap<K, usize>,
    /// 组合内线性索引 → 模型注册索引 / Linear index in combination → model registration index
    linear_index_to_model_index: Vec<usize>,
    /// 变量类型标记 / Variable type marker
    _marker: std::marker::PhantomData<VT>,
}

impl<K, VT: VariableTypeTrait> IndexedVariableArray1<K, VT>
where
    K: Hash + Eq + Clone + std::fmt::Debug,
    VT: Clone,
{
    /// 创建新的一维索引变量集合并注册到模型 / Create new 1D indexed variable array and register to model
    ///
    /// 为每个领域键创建一个 `VT` 类型变量，注册到 `MetaModel`，
    /// 并建立领域键到模型注册索引的双向映射。
    /// Creates a `VT` variable for each domain key, registers to `MetaModel`,
    /// and builds bidirectional mapping from domain keys to model indices.
    pub fn new<V>(name: &str, keys: &[K], model: &mut MetaModel<V>) -> GanttResult<Self>
    where
        V: Clone + std::fmt::Debug + Send + Sync + 'static,
        VT::Value: ospf_rust_core::token::IntoValue<V>,
    {
        let n = keys.len();
        let combination = VariableCombination::<VT, Shape<1>>::new(
            Shape::new([n]),
            name,
        );

        // 逐个注册变量到模型，建立映射
        // Register each variable to model, build mappings
        let mut key_to_linear_index = HashMap::new();
        let mut linear_index_to_model_index = Vec::with_capacity(n);

        for (linear_idx, key) in keys.iter().enumerate() {
            key_to_linear_index.insert(key.clone(), linear_idx);

            let var_item = combination[linear_idx].clone();
            let model_idx = model.register_variable(var_item)
                .map_err(|e| GanttError::Calculation {
                    message: format!(
                        "Failed to register variable {}_{}: {:?}",
                        name, linear_idx, e
                    ),
                })?;
            linear_index_to_model_index.push(model_idx);
        }

        Ok(Self {
            name: name.to_string(),
            combination,
            key_to_linear_index,
            linear_index_to_model_index,
            _marker: std::marker::PhantomData,
        })
    }

    /// 获取键对应的模型注册索引 / Get model registration index for key
    ///
    /// 返回 `register_variable` 返回的 `usize`，可用于约束注册和结果提取。
    /// Returns the `usize` from `register_variable`, usable in constraint registration and solution extraction.
    pub fn model_index(&self, key: &K) -> Option<usize> {
        self.key_to_linear_index.get(key)
            .and_then(|&linear_idx| self.linear_index_to_model_index.get(linear_idx))
            .copied()
    }

    /// 获取键对应的变量项 / Get variable item for key
    pub fn variable_item(&self, key: &K) -> Option<&VariableItem<VT>> {
        self.key_to_linear_index.get(key)
            .map(|&linear_idx| &self.combination[linear_idx])
    }

    /// 获取所有领域键 / Get all domain keys
    pub fn keys(&self) -> Vec<&K> {
        self.key_to_linear_index.keys().collect()
    }

    /// 获取底层变量组合 / Get underlying variable combination
    pub fn combination(&self) -> &VariableCombination<VT, Shape<1>> {
        &self.combination
    }

    /// 变量数量 / Number of variables
    pub fn len(&self) -> usize {
        self.key_to_linear_index.len()
    }

    /// 是否为空 / Whether empty
    pub fn is_empty(&self) -> bool {
        self.key_to_linear_index.is_empty()
    }

    /// 获取所有模型注册索引 / Get all model registration indices
    pub fn model_indices(&self) -> &[usize] {
        &self.linear_index_to_model_index
    }
}

// ============================================================================
// 二维索引变量集合 / 2D Indexed Variable Array
// ============================================================================

/// 二维索引变量集合 / 2D indexed variable array
///
/// 将两个领域键的组合映射到 `VariableCombination` 中的变量项和模型注册索引。
/// Maps pairs of domain keys to variable items in `VariableCombination` and model registration indices.
#[derive(Debug)]
pub struct IndexedVariableArray2<K1, K2, VT: VariableTypeTrait> {
    /// 集合名称 / Collection name
    pub name: String,
    /// 底层变量组合 / Underlying variable combination
    combination: VariableCombination<VT, Shape<2>>,
    /// 领域键对 → 组合内线性索引 / Domain key pair → linear index in combination
    key_to_linear_index: HashMap<(K1, K2), usize>,
    /// 组合内线性索引 → 领域键对（列移除时使用）/ Linear index → domain key pair (used in column removal)
    #[allow(dead_code)]
    linear_index_to_key: HashMap<usize, (K1, K2)>,
    /// 组合内线性索引 → 模型注册索引 / Linear index in combination → model registration index
    linear_index_to_model_index: Vec<usize>,
    /// 第一个维度的键列表 / First dimension keys
    pub keys1: Vec<K1>,
    /// 第二个维度的键列表 / Second dimension keys
    pub keys2: Vec<K2>,
    /// 变量类型标记 / Variable type marker
    _marker: std::marker::PhantomData<VT>,
}

impl<K1, K2, VT: VariableTypeTrait> IndexedVariableArray2<K1, K2, VT>
where
    K1: Hash + Eq + Clone + std::fmt::Debug,
    K2: Hash + Eq + Clone + std::fmt::Debug,
    VT: Clone,
{
    /// 创建新的二维索引变量集合并注册到模型 / Create new 2D indexed variable array and register to model
    pub fn new<V>(
        name: &str,
        keys1: &[K1],
        keys2: &[K2],
        model: &mut MetaModel<V>,
    ) -> GanttResult<Self>
    where
        V: Clone + std::fmt::Debug + Send + Sync + 'static,
        VT::Value: ospf_rust_core::token::IntoValue<V>,
    {
        let n1 = keys1.len();
        let n2 = keys2.len();
        let combination = VariableCombination::<VT, Shape<2>>::new(
            Shape::new([n1, n2]),
            name,
        );

        // 逐个注册变量到模型，建立映射
        // Register each variable to model, build mappings
        let mut key_to_linear_index = HashMap::new();
        let mut linear_index_to_key = HashMap::new();
        let mut linear_index_to_model_index = Vec::with_capacity(n1 * n2);

        for (i1, k1) in keys1.iter().enumerate() {
            for (i2, k2) in keys2.iter().enumerate() {
                let linear_idx = i1 * n2 + i2;
                key_to_linear_index.insert((k1.clone(), k2.clone()), linear_idx);
                linear_index_to_key.insert(linear_idx, (k1.clone(), k2.clone()));

                let var_item = combination[linear_idx].clone();
                let model_idx = model.register_variable(var_item)
                    .map_err(|e| GanttError::Calculation {
                        message: format!(
                            "Failed to register variable {}_{}_{}: {:?}",
                            name, i1, i2, e
                        ),
                    })?;
                linear_index_to_model_index.push(model_idx);
            }
        }

        Ok(Self {
            name: name.to_string(),
            combination,
            key_to_linear_index,
            linear_index_to_key,
            linear_index_to_model_index,
            keys1: keys1.to_vec(),
            keys2: keys2.to_vec(),
            _marker: std::marker::PhantomData,
        })
    }

    /// 获取键对对应的模型注册索引 / Get model registration index for key pair
    pub fn model_index(&self, k1: &K1, k2: &K2) -> Option<usize> {
        self.key_to_linear_index.get(&(k1.clone(), k2.clone()))
            .and_then(|&linear_idx| self.linear_index_to_model_index.get(linear_idx))
            .copied()
    }

    /// 获取键对对应的变量项 / Get variable item for key pair
    pub fn variable_item(&self, k1: &K1, k2: &K2) -> Option<&VariableItem<VT>> {
        self.key_to_linear_index.get(&(k1.clone(), k2.clone()))
            .map(|&linear_idx| &self.combination[linear_idx])
    }

    /// 获取第一个维度键的所有变量项 / Get all variable items for first dimension key
    ///
    /// 返回 `(k1, k2)` 中 `k1` 固定时的所有变量项。
    /// Returns all variable items where `k1` is fixed in the `(k1, k2)` pair.
    pub fn variable_items_for_key1(&self, k1: &K1) -> Vec<(&K2, &VariableItem<VT>)> {
        let n2 = self.keys2.len();
        let mut result = Vec::with_capacity(n2);
        for k2 in &self.keys2 {
            if let Some(item) = self.variable_item(k1, k2) {
                result.push((k2, item));
            }
        }
        result
    }

    /// 获取第二个维度键的所有变量项 / Get all variable items for second dimension key
    ///
    /// 返回 `(k1, k2)` 中 `k2` 固定时的所有变量项。
    /// Returns all variable items where `k2` is fixed in the `(k1, k2)` pair.
    pub fn variable_items_for_key2(&self, k2: &K2) -> Vec<(&K1, &VariableItem<VT>)> {
        let n1 = self.keys1.len();
        let mut result = Vec::with_capacity(n1);
        for k1 in &self.keys1 {
            if let Some(item) = self.variable_item(k1, k2) {
                result.push((k1, item));
            }
        }
        result
    }

    /// 变量数量 / Number of variables
    pub fn len(&self) -> usize {
        self.key_to_linear_index.len()
    }

    /// 是否为空 / Whether empty
    pub fn is_empty(&self) -> bool {
        self.key_to_linear_index.is_empty()
    }

    /// 获取所有模型注册索引 / Get all model registration indices
    pub fn model_indices(&self) -> &[usize] {
        &self.linear_index_to_model_index
    }

    /// 获取底层变量组合 / Get underlying variable combination
    pub fn combination(&self) -> &VariableCombination<VT, Shape<2>> {
        &self.combination
    }
}

// ============================================================================
// 三维索引变量集合 / 3D Indexed Variable Array
// ============================================================================

/// 三维索引变量集合 / 3D indexed variable array
///
/// 管理三维变量 `x[k1, k2, k3]` 到 `MetaModel` 的注册和索引映射。
/// 三维变量用于产能排程中的 `x[action, slot, order]` 等场景。
///
/// Manages 3D variable `x[k1, k2, k3]` registration and index mapping to `MetaModel`.
/// 3D variables are used in capacity scheduling for `x[action, slot, order]` etc.
pub struct IndexedVariableArray3<K1, K2, K3, VT: VariableTypeTrait> {
    /// 集合名称 / Collection name
    pub name: String,
    /// 底层变量组合 / Underlying variable combination
    combination: VariableCombination<VT, Shape<3>>,
    /// 领域键三元组 → 组合内线性索引 / Domain key triple → linear index in combination
    key_to_linear_index: HashMap<(K1, K2, K3), usize>,
    /// 组合内线性索引 → 领域键三元组 / Linear index → domain key triple
    #[allow(dead_code)]
    linear_index_to_key: HashMap<usize, (K1, K2, K3)>,
    /// 组合内线性索引 → 模型注册索引 / Linear index in combination → model registration index
    linear_index_to_model_index: Vec<usize>,
    /// 第一个维度的键列表 / First dimension keys
    pub keys1: Vec<K1>,
    /// 第二个维度的键列表 / Second dimension keys
    pub keys2: Vec<K2>,
    /// 第三个维度的键列表 / Third dimension keys
    pub keys3: Vec<K3>,
    /// 变量类型标记 / Variable type marker
    _marker: std::marker::PhantomData<VT>,
}

impl<K1, K2, K3, VT: VariableTypeTrait> IndexedVariableArray3<K1, K2, K3, VT>
where
    K1: Hash + Eq + Clone + std::fmt::Debug,
    K2: Hash + Eq + Clone + std::fmt::Debug,
    K3: Hash + Eq + Clone + std::fmt::Debug,
    VT: Clone,
{
    /// 创建新的三维索引变量集合并注册到模型 / Create new 3D indexed variable array and register to model
    pub fn new<V>(
        name: &str,
        keys1: &[K1],
        keys2: &[K2],
        keys3: &[K3],
        model: &mut MetaModel<V>,
    ) -> GanttResult<Self>
    where
        V: Clone + std::fmt::Debug + Send + Sync + 'static,
        VT::Value: ospf_rust_core::token::IntoValue<V>,
    {
        let n1 = keys1.len();
        let n2 = keys2.len();
        let n3 = keys3.len();
        let combination = VariableCombination::<VT, Shape<3>>::new(
            Shape::new([n1, n2, n3]),
            name,
        );

        let mut key_to_linear_index = HashMap::new();
        let mut linear_index_to_key = HashMap::new();
        let mut linear_index_to_model_index = Vec::with_capacity(n1 * n2 * n3);

        for (i1, k1) in keys1.iter().enumerate() {
            for (i2, k2) in keys2.iter().enumerate() {
                for (i3, k3) in keys3.iter().enumerate() {
                    let linear_idx = i1 * n2 * n3 + i2 * n3 + i3;
                    key_to_linear_index.insert((k1.clone(), k2.clone(), k3.clone()), linear_idx);
                    linear_index_to_key.insert(linear_idx, (k1.clone(), k2.clone(), k3.clone()));

                    let var_item = combination[linear_idx].clone();
                    let model_idx = model.register_variable(var_item)
                        .map_err(|e| GanttError::Calculation {
                            message: format!(
                                "Failed to register variable {}_{}_{}_{}: {:?}",
                                name, i1, i2, i3, e
                            ),
                        })?;
                    linear_index_to_model_index.push(model_idx);
                }
            }
        }

        Ok(Self {
            name: name.to_string(),
            combination,
            key_to_linear_index,
            linear_index_to_key,
            linear_index_to_model_index,
            keys1: keys1.to_vec(),
            keys2: keys2.to_vec(),
            keys3: keys3.to_vec(),
            _marker: std::marker::PhantomData,
        })
    }

    /// 获取键三元组对应的模型注册索引 / Get model registration index for key triple
    pub fn model_index(&self, k1: &K1, k2: &K2, k3: &K3) -> Option<usize> {
        self.key_to_linear_index.get(&(k1.clone(), k2.clone(), k3.clone()))
            .and_then(|&linear_idx| self.linear_index_to_model_index.get(linear_idx))
            .copied()
    }

    /// 获取键三元组对应的变量项 / Get variable item for key triple
    pub fn variable_item(&self, k1: &K1, k2: &K2, k3: &K3) -> Option<&VariableItem<VT>> {
        self.key_to_linear_index.get(&(k1.clone(), k2.clone(), k3.clone()))
            .map(|&linear_idx| &self.combination[linear_idx])
    }

    /// 变量数量 / Number of variables
    pub fn len(&self) -> usize {
        self.key_to_linear_index.len()
    }

    /// 是否为空 / Whether empty
    pub fn is_empty(&self) -> bool {
        self.key_to_linear_index.is_empty()
    }

    /// 获取所有模型注册索引 / Get all model registration indices
    pub fn model_indices(&self) -> &[usize] {
        &self.linear_index_to_model_index
    }

    /// 获取底层变量组合 / Get underlying variable combination
    pub fn combination(&self) -> &VariableCombination<VT, Shape<3>> {
        &self.combination
    }
}

// ============================================================================
// 表达式构建辅助 / Expression Building Helpers
// ============================================================================

/// 从变量项列表构建线性求和表达式 / Build linear sum expression from variable items
///
/// 每个变量项的系数为 1.0，常数项为 0.0。
/// Each variable item has coefficient 1.0, constant term 0.0.
///
/// 注意：此函数构建 `Linear<f64>` 多项式，其中单项式的 `var_index` 为模型注册索引。
/// Note: This builds a `Linear<f64>` polynomial where monomials use model registration indices.
pub fn sum_to_linear(model_indices_and_coeffs: &[(usize, f64)]) -> Linear<f64> {
    let monomials: Vec<LinearMonomial<f64>> = model_indices_and_coeffs
        .iter()
        .map(|(idx, coeff)| LinearMonomial::new(*coeff, *idx))
        .collect();
    Linear::new(monomials, 0.0)
}

/// 从变量项和系数列表构建 LinearExpressionSymbol / Build LinearExpressionSymbol from variables and coefficients
///
/// 创建中间表达式符号，用于注册到 `MetaModel.add_symbol()`。
/// Creates intermediate expression symbol for registration to `MetaModel.add_symbol()`.
pub fn build_linear_expression_symbol(
    name: &str,
    model_indices_and_coeffs: &[(usize, f64)],
    constant: f64,
) -> Arc<LinearExpressionSymbol<f64>> {
    let id = next_gantt_symbol_id();
    let monomials: Vec<LinearMonomial<f64>> = model_indices_and_coeffs
        .iter()
        .map(|(idx, coeff)| LinearMonomial::new(*coeff, *idx))
        .collect();
    Arc::new(LinearExpressionSymbol::new(id, name, monomials, constant))
}

// ============================================================================
// 模型组件 trait / Model component trait
// ============================================================================

/// 模型组件 trait / Model component trait
///
/// 定义向 `MetaModel` 注册变量、表达式和中间符号的接口。
/// Defines the interface for registering variables, expressions, and
/// intermediate symbols to `MetaModel`.
///
/// 注意：`register` 方法使用 `&mut self`，因为组件在注册过程中需要存储变量引用。
/// Note: `register` uses `&mut self` because components store variable references during registration.
pub trait ModelComponent<V>: Send + Sync + std::fmt::Debug + 'static
where
    V: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    /// 组件名称 / Component name
    fn name(&self) -> &str;

    /// 注册到模型 / Register to model
    ///
    /// 创建变量、中间表达式并注册到 `MetaModel`。
    /// Creates variables, intermediate expressions and registers them to `MetaModel`.
    fn register(&mut self, model: &mut MetaModel<V>) -> GanttResult<()>;
}

// ============================================================================
// 结果提取 / Solution Extraction
// ============================================================================

/// 从求解器解向量中提取变量值 / Extract variable value from solver solution vector
///
/// 根据模型注册索引从解向量中获取对应的值。
/// Gets the value corresponding to a model registration index from the solution vector.
pub fn extract_value(solution: &[f64], model_index: usize) -> Option<f64> {
    if model_index < solution.len() {
        Some(solution[model_index])
    } else {
        None
    }
}

/// 从求解器解向量中提取二元变量的布尔值 / Extract binary variable boolean value
pub fn extract_binary(solution: &[f64], model_index: usize) -> Option<bool> {
    extract_value(solution, model_index).map(|v| v > 0.5)
}

/// 从一维索引变量集合中提取所有布尔值 / Extract all boolean values from 1D indexed variable array
pub fn extract_binary_values_1<K>(
    solution: &[f64],
    array: &IndexedVariableArray1<K, Binary>,
) -> HashMap<K, bool>
where
    K: Hash + Eq + Clone,
{
    array.key_to_linear_index
        .iter()
        .filter_map(|(k, &linear_idx)| {
            let model_idx = array.linear_index_to_model_index.get(linear_idx)?;
            extract_binary(solution, *model_idx).map(|v| (k.clone(), v))
        })
        .collect()
}

/// 从二维索引变量集合中提取所有布尔值 / Extract all boolean values from 2D indexed variable array
pub fn extract_binary_values_2<K1, K2>(
    solution: &[f64],
    array: &IndexedVariableArray2<K1, K2, Binary>,
) -> HashMap<(K1, K2), bool>
where
    K1: Hash + Eq + Clone,
    K2: Hash + Eq + Clone,
{
    array.key_to_linear_index
        .iter()
        .filter_map(|((k1, k2), &linear_idx)| {
            let model_idx = array.linear_index_to_model_index.get(linear_idx)?;
            extract_binary(solution, *model_idx).map(|v| ((k1.clone(), k2.clone()), v))
        })
        .collect()
}

/// 从一维索引变量集合中提取所有连续值 / Extract all continuous values from 1D indexed variable array
pub fn extract_values_1<K, VT: VariableTypeTrait>(
    solution: &[f64],
    array: &IndexedVariableArray1<K, VT>,
) -> HashMap<K, f64>
where
    K: Hash + Eq + Clone,
{
    array.key_to_linear_index
        .iter()
        .filter_map(|(k, &linear_idx)| {
            let model_idx = array.linear_index_to_model_index.get(linear_idx)?;
            extract_value(solution, *model_idx).map(|v| (k.clone(), v))
        })
        .collect()
}

/// 从二维索引变量集合中提取所有连续值 / Extract all continuous values from 2D indexed variable array
pub fn extract_values_2<K1, K2, VT: VariableTypeTrait>(
    solution: &[f64],
    array: &IndexedVariableArray2<K1, K2, VT>,
) -> HashMap<(K1, K2), f64>
where
    K1: Hash + Eq + Clone,
    K2: Hash + Eq + Clone,
{
    array.key_to_linear_index
        .iter()
        .filter_map(|((k1, k2), &linear_idx)| {
            let model_idx = array.linear_index_to_model_index.get(linear_idx)?;
            extract_value(solution, *model_idx).map(|v| ((k1.clone(), k2.clone()), v))
        })
        .collect()
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_core::model::MetaModel;
    use ospf_rust_core::variable::Binary;

    #[test]
    fn test_indexed_variable_array_1d() {
        let mut model = MetaModel::<f64>::new("test_1d");
        let keys: Vec<usize> = vec![0, 1, 2, 3, 4];

        let array: IndexedVariableArray1<usize, Binary> =
            IndexedVariableArray1::new("x", &keys, &mut model).unwrap();

        assert_eq!(array.len(), 5);
        assert!(!array.is_empty());
        assert_eq!(array.name, "x");

        // 每个键都有模型索引
        // Each key has a model index
        for k in &keys {
            assert!(array.model_index(k).is_some());
            assert!(array.variable_item(k).is_some());
        }

        // 不存在的键返回 None
        // Non-existent key returns None
        assert!(array.model_index(&99).is_none());
        assert!(array.variable_item(&99).is_none());
    }

    #[test]
    fn test_indexed_variable_array_2d() {
        let mut model = MetaModel::<f64>::new("test_2d");
        let keys1: Vec<usize> = vec![0, 1, 2];
        let keys2: Vec<usize> = vec![0, 1];

        let array: IndexedVariableArray2<usize, usize, Binary> =
            IndexedVariableArray2::new("x", &keys1, &keys2, &mut model).unwrap();

        assert_eq!(array.len(), 6);  // 3 * 2 = 6
        assert_eq!(array.keys1.len(), 3);
        assert_eq!(array.keys2.len(), 2);

        // 每个键对都有模型索引
        // Each key pair has a model index
        for k1 in &keys1 {
            for k2 in &keys2 {
                assert!(array.model_index(k1, k2).is_some());
                assert!(array.variable_item(k1, k2).is_some());
            }
        }
    }

    #[test]
    fn test_model_index_correctness() {
        let mut model = MetaModel::<f64>::new("test_index");
        let keys: Vec<usize> = vec![10, 20, 30];

        // 先注册一些独立变量，然后再注册 array
        // Register standalone variables first, then array
        let v0 = ospf_rust_core::variable::BinaryVariableItem::auto("v0");
        let idx0 = model.register_variable(v0).unwrap();

        let array: IndexedVariableArray1<usize, Binary> =
            IndexedVariableArray1::new("x", &keys, &mut model).unwrap();

        // array 内的变量索引应该大于 idx0
        // Array variables' indices should be greater than idx0
        for k in &keys {
            let arr_idx = array.model_index(k).unwrap();
            assert!(arr_idx > idx0);
        }

        // 不同键应该映射到不同索引
        // Different keys should map to different indices
        let idx10 = array.model_index(&10).unwrap();
        let idx20 = array.model_index(&20).unwrap();
        let idx30 = array.model_index(&30).unwrap();
        assert_ne!(idx10, idx20);
        assert_ne!(idx20, idx30);
    }

    #[test]
    fn test_sum_to_linear() {
        let terms = vec![(0, 1.0), (1, 2.0), (2, -1.0)];
        let linear = sum_to_linear(&terms);

        assert_eq!(linear.monomials().len(), 3);
        assert_eq!(linear.constant_term(), &0.0);
    }

    #[test]
    fn test_build_linear_expression_symbol() {
        let terms = vec![(0, 1.0), (1, 1.0)];
        let symbol = build_linear_expression_symbol("test_expr", &terms, 0.0);

        // 验证符号名称通过 Display trait 可读
        // Verify symbol name is readable via Display trait
        assert!(format!("{}", symbol).contains("test_expr"));
    }

    #[test]
    fn test_extract_value() {
        let solution = vec![0.0, 1.0, 0.5, 3.0];

        assert_eq!(extract_value(&solution, 0), Some(0.0));
        assert_eq!(extract_value(&solution, 1), Some(1.0));
        assert_eq!(extract_value(&solution, 2), Some(0.5));
        assert_eq!(extract_value(&solution, 99), None);
    }

    #[test]
    fn test_extract_binary() {
        let solution = vec![0.0, 1.0, 0.3, 0.7];

        assert_eq!(extract_binary(&solution, 0), Some(false));
        assert_eq!(extract_binary(&solution, 1), Some(true));
        assert_eq!(extract_binary(&solution, 2), Some(false));  // 0.3 < 0.5
        assert_eq!(extract_binary(&solution, 3), Some(true));   // 0.7 > 0.5
    }

    #[test]
    fn test_extract_binary_values_1() {
        let mut model = MetaModel::<f64>::new("test_extract_1");
        let keys: Vec<usize> = vec![0, 1, 2];
        let array: IndexedVariableArray1<usize, Binary> =
            IndexedVariableArray1::new("x", &keys, &mut model).unwrap();

        // 构造模拟解向量（按模型注册索引排列）
        // Build simulated solution vector (indexed by model registration indices)
        let n_vars = model.register_auto_variable::<Binary>("padding").unwrap() + 1;
        let mut solution = vec![0.0; n_vars];
        // 手动设置 array 中的变量值为 1.0
        // Set array variables to 1.0
        for k in &keys {
            let idx = array.model_index(k).unwrap();
            solution[idx] = 1.0;
        }

        let result = extract_binary_values_1(&solution, &array);
        assert_eq!(result.len(), 3);
        for k in &keys {
            assert_eq!(result.get(k), Some(&true));
        }
    }

    #[test]
    fn test_extract_binary_values_2() {
        let mut model = MetaModel::<f64>::new("test_extract_2");
        let keys1: Vec<usize> = vec![0, 1];
        let keys2: Vec<usize> = vec![0, 1];
        let array: IndexedVariableArray2<usize, usize, Binary> =
            IndexedVariableArray2::new("x", &keys1, &keys2, &mut model).unwrap();

        let n_vars = model.register_auto_variable::<Binary>("padding").unwrap() + 1;
        let mut solution = vec![0.0; n_vars];
        // 只有 x[0,1] 和 x[1,0] 为 1.0
        // Only x[0,1] and x[1,0] are 1.0
        if let Some(idx) = array.model_index(&0, &1) {
            solution[idx] = 1.0;
        }
        if let Some(idx) = array.model_index(&1, &0) {
            solution[idx] = 1.0;
        }

        let result = extract_binary_values_2(&solution, &array);
        assert_eq!(result.get(&(0, 1)), Some(&true));
        assert_eq!(result.get(&(1, 0)), Some(&true));
        assert_eq!(result.get(&(0, 0)), Some(&false));
        assert_eq!(result.get(&(1, 1)), Some(&false));
    }
}
