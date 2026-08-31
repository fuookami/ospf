//! BPP3D 建模适配层 / BPP3D modeling adapter
//!
//! 补齐 Kotlin DSL 到 Rust `MetaModel` 的承接层。
//! Bridges Kotlin DSL to Rust `MetaModel` for BPP3D domain modeling.
//!
//! # 核心组件 / Core Components
//!
//! - `VariableArray1`: 一维变量集合 / 1D variable array
//! - `VariableArray2`: 二维变量集合 / 2D variable array
//! - `ExpressionArray1`: 一维线性表达式集合 / 1D linear expression array
//! - `SolutionExtractor`: 结果提取辅助 / Solution extraction helper

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use ospf_rust_core::model::meta_model::MetaModel;
use ospf_rust_core::variable::variable_item::{BinaryVariableItem, ContinuousVariableItem};

// ============================================================================
// VariableArray1 - 一维变量集合 / 1D variable array
// ============================================================================

/// 一维索引变量集合 / One-dimensional indexed variable array
///
/// 将领域键映射到模型变量索引，支持注册和结果提取。
/// Maps domain keys to model variable indices, supporting registration and
/// solution extraction.
#[derive(Debug, Clone)]
pub struct VariableArray1<K, VT>
where
    K: Debug + Clone + Eq + Hash,
    VT: Debug + Clone,
{
    /// 变量名前缀 / Variable name prefix
    pub prefix: String,
    /// 键到模型索引的映射 / Key to model index mapping
    pub indices: HashMap<K, usize>,
    /// 键到变量类型的映射 / Key to variable type mapping
    _phantom: std::marker::PhantomData<VT>,
}

impl<K, VT> VariableArray1<K, VT>
where
    K: Debug + Clone + Eq + Hash,
    VT: Debug + Clone,
{
    /// 创建一维变量集合 / Create a 1D variable array
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            indices: HashMap::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// 获取变量数量 / Get number of variables
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    /// 是否为空 / Whether empty
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// 获取键对应的模型索引 / Get model index for key
    pub fn index(&self, key: &K) -> Option<usize> {
        self.indices.get(key).copied()
    }

    /// 生成变量名 / Generate variable name
    fn variable_name(&self, key: &K) -> String {
        format!("{}_{}", self.prefix, key_to_string(key))
    }
}

/// 辅助：将键转为字符串 / Helper: convert key to string
fn key_to_string<K: Debug>(key: &K) -> String {
    format!("{:?}", key)
}

impl<K> VariableArray1<K, BinaryVariableItem>
where
    K: Debug + Clone + Eq + Hash,
{
    /// 注册二值变量 / Register binary variables
    pub fn register_binary(&mut self, keys: &[K], model: &mut MetaModel<f64>) -> Result<(), String> {
        for key in keys {
            let name = self.variable_name(key);
            let var = BinaryVariableItem::auto(&name);
            let idx = model.register_variable(var)
                .map_err(|e| format!("Failed to register variable {}: {:?}", name, e))?;
            self.indices.insert(key.clone(), idx);
        }
        Ok(())
    }
}

impl<K> VariableArray1<K, ContinuousVariableItem>
where
    K: Debug + Clone + Eq + Hash,
{
    /// 注册连续变量 / Register continuous variables
    pub fn register_continuous(&mut self, keys: &[K], model: &mut MetaModel<f64>) -> Result<(), String> {
        for key in keys {
            let name = self.variable_name(key);
            let var = ContinuousVariableItem::auto(&name);
            let idx = model.register_variable(var)
                .map_err(|e| format!("Failed to register variable {}: {:?}", name, e))?;
            self.indices.insert(key.clone(), idx);
        }
        Ok(())
    }
}

// ============================================================================
// VariableArray2 - 二维变量集合 / 2D variable array
// ============================================================================

/// 二维索引变量集合 / Two-dimensional indexed variable array
#[derive(Debug, Clone)]
pub struct VariableArray2<K1, K2, VT>
where
    K1: Debug + Clone + Eq + Hash,
    K2: Debug + Clone + Eq + Hash,
    VT: Debug + Clone,
{
    /// 变量名前缀 / Variable name prefix
    pub prefix: String,
    /// (K1, K2) 到模型索引的映射 / (K1, K2) to model index mapping
    pub indices: HashMap<(K1, K2), usize>,
    _phantom: std::marker::PhantomData<VT>,
}

impl<K1, K2, VT> VariableArray2<K1, K2, VT>
where
    K1: Debug + Clone + Eq + Hash,
    K2: Debug + Clone + Eq + Hash,
    VT: Debug + Clone,
{
    /// 创建二维变量集合 / Create a 2D variable array
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            indices: HashMap::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// 获取变量数量 / Get number of variables
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    /// 是否为空 / Whether empty
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// 获取键对应的模型索引 / Get model index for key pair
    pub fn index(&self, k1: &K1, k2: &K2) -> Option<usize> {
        self.indices.get(&(k1.clone(), k2.clone())).copied()
    }

    fn variable_name(&self, k1: &K1, k2: &K2) -> String {
        format!("{}_{}_{}", self.prefix, key_to_string(k1), key_to_string(k2))
    }
}

impl<K1, K2> VariableArray2<K1, K2, BinaryVariableItem>
where
    K1: Debug + Clone + Eq + Hash,
    K2: Debug + Clone + Eq + Hash,
{
    /// 注册二值变量 / Register binary variables
    pub fn register_binary(
        &mut self,
        keys1: &[K1],
        keys2: &[K2],
        model: &mut MetaModel<f64>,
    ) -> Result<(), String> {
        for k1 in keys1 {
            for k2 in keys2 {
                let name = self.variable_name(k1, k2);
                let var = BinaryVariableItem::auto(&name);
                let idx = model.register_variable(var)
                    .map_err(|e| format!("Failed to register variable {}: {:?}", name, e))?;
                self.indices.insert((k1.clone(), k2.clone()), idx);
            }
        }
        Ok(())
    }
}

impl<K1, K2> VariableArray2<K1, K2, ContinuousVariableItem>
where
    K1: Debug + Clone + Eq + Hash,
    K2: Debug + Clone + Eq + Hash,
{
    /// 注册连续变量 / Register continuous variables
    pub fn register_continuous(
        &mut self,
        keys1: &[K1],
        keys2: &[K2],
        model: &mut MetaModel<f64>,
    ) -> Result<(), String> {
        for k1 in keys1 {
            for k2 in keys2 {
                let name = self.variable_name(k1, k2);
                let var = ContinuousVariableItem::auto(&name);
                let idx = model.register_variable(var)
                    .map_err(|e| format!("Failed to register variable {}: {:?}", name, e))?;
                self.indices.insert((k1.clone(), k2.clone()), idx);
            }
        }
        Ok(())
    }
}

// ============================================================================
// ExpressionArray1 - 一维线性表达式集合 / 1D linear expression array
// ============================================================================

/// 一维线性表达式集合 / One-dimensional linear expression array
///
/// 存储中间线性表达式，用于约束和目标注册。
/// Stores intermediate linear expressions for constraint and objective registration.
#[derive(Debug, Clone)]
pub struct ExpressionArray1<K>
where
    K: Debug + Clone + Eq + Hash,
{
    /// 表达式名前缀 / Expression name prefix
    pub prefix: String,
    /// 键到 (变量索引, 系数) 列表的映射 / Key to (variable index, coefficient) list
    pub expressions: HashMap<K, Vec<(usize, f64)>>,
}

impl<K> ExpressionArray1<K>
where
    K: Debug + Clone + Eq + Hash,
{
    /// 创建一维表达式集合 / Create a 1D expression array
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            expressions: HashMap::new(),
        }
    }

    /// 添加表达式 / Add an expression
    pub fn insert(&mut self, key: K, terms: Vec<(usize, f64)>) {
        self.expressions.insert(key, terms);
    }

    /// 获取表达式 / Get an expression
    pub fn get(&self, key: &K) -> Option<&Vec<(usize, f64)>> {
        self.expressions.get(key)
    }

    /// 获取表达式数量 / Get number of expressions
    pub fn len(&self) -> usize {
        self.expressions.len()
    }

    /// 是否为空 / Whether empty
    pub fn is_empty(&self) -> bool {
        self.expressions.is_empty()
    }
}

// ============================================================================
// SolutionExtractor - 结果提取 / Solution extraction
// ============================================================================

/// 结果提取辅助 / Solution extraction helper
///
/// 从求解器输出中提取领域结果。
/// Extracts domain results from solver output.
pub struct SolutionExtractor;

impl SolutionExtractor {
    /// 从解向量中提取一维变量的值 / Extract 1D variable values from solution
    pub fn extract_values_1<K>(
        solution: &[f64],
        array: &VariableArray1<K, ContinuousVariableItem>,
    ) -> HashMap<K, f64>
    where
        K: Debug + Clone + Eq + Hash,
    {
        let mut result = HashMap::new();
        for (key, &idx) in &array.indices {
            if idx < solution.len() {
                result.insert(key.clone(), solution[idx]);
            }
        }
        result
    }

    /// 从解向量中提取一维二值变量 / Extract 1D binary variable values from solution
    pub fn extract_binary_1<K>(
        solution: &[f64],
        array: &VariableArray1<K, BinaryVariableItem>,
    ) -> HashMap<K, bool>
    where
        K: Debug + Clone + Eq + Hash,
    {
        let mut result = HashMap::new();
        for (key, &idx) in &array.indices {
            if idx < solution.len() {
                result.insert(key.clone(), solution[idx] > 0.5);
            }
        }
        result
    }

    /// 从解向量中提取二维二值变量 / Extract 2D binary variable values from solution
    pub fn extract_binary_2<K1, K2>(
        solution: &[f64],
        array: &VariableArray2<K1, K2, BinaryVariableItem>,
    ) -> HashMap<(K1, K2), bool>
    where
        K1: Debug + Clone + Eq + Hash,
        K2: Debug + Clone + Eq + Hash,
    {
        let mut result = HashMap::new();
        for ((k1, k2), &idx) in &array.indices {
            if idx < solution.len() {
                result.insert((k1.clone(), k2.clone()), solution[idx] > 0.5);
            }
        }
        result
    }

    /// 提取单个变量的值 / Extract a single variable value
    pub fn extract_value(solution: &[f64], model_index: usize) -> Option<f64> {
        if model_index < solution.len() {
            Some(solution[model_index])
        } else {
            None
        }
    }

    /// 提取单个二值变量 / Extract a single binary variable
    pub fn extract_binary(solution: &[f64], model_index: usize) -> Option<bool> {
        Self::extract_value(solution, model_index).map(|v| v > 0.5)
    }
}

// ============================================================================
// Bpp3dModelComponent - BPP3D 模型组件 trait / BPP3D model component trait
// ============================================================================

/// BPP3D 模型组件 / BPP3D model component
///
/// 领域建模组件必须通过此 trait 注册到 `MetaModel`。
/// Domain modeling components must register to `MetaModel` through this trait.
pub trait Bpp3dModelComponent: Debug + Send + Sync {
    /// 组件名称 / Component name
    fn name(&self) -> &str;

    /// 注册变量和中间表达式到模型 / Register variables and intermediate expressions to model
    fn register(&mut self, model: &mut MetaModel<f64>) -> Result<(), String>;
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable_array1_new_and_len() {
        let array: VariableArray1<String, ContinuousVariableItem> = VariableArray1::new("x");
        assert_eq!(array.prefix, "x");
        assert!(array.is_empty());
    }

    #[test]
    fn variable_array2_new_and_len() {
        let array: VariableArray2<String, String, BinaryVariableItem> = VariableArray2::new("u");
        assert_eq!(array.prefix, "u");
        assert!(array.is_empty());
    }

    #[test]
    fn expression_array1_insert_and_get() {
        let mut exprs: ExpressionArray1<String> = ExpressionArray1::new("load");
        exprs.insert("demand_1".to_string(), vec![(0, 1.0), (1, 2.0)]);
        assert_eq!(exprs.len(), 1);

        let terms = exprs.get(&"demand_1".to_string()).unwrap();
        assert_eq!(terms.len(), 2);
        assert_eq!(terms[0], (0, 1.0));
    }

    #[test]
    fn solution_extractor_extract_value() {
        let solution = vec![0.0, 1.5, 0.8, 1.0];
        assert_eq!(SolutionExtractor::extract_value(&solution, 0), Some(0.0));
        assert_eq!(SolutionExtractor::extract_value(&solution, 1), Some(1.5));
        assert_eq!(SolutionExtractor::extract_value(&solution, 10), None);
    }

    #[test]
    fn solution_extractor_extract_binary() {
        let solution = vec![0.0, 1.0, 0.3, 0.8];
        assert_eq!(SolutionExtractor::extract_binary(&solution, 0), Some(false));
        assert_eq!(SolutionExtractor::extract_binary(&solution, 1), Some(true));
        assert_eq!(SolutionExtractor::extract_binary(&solution, 2), Some(false));
        assert_eq!(SolutionExtractor::extract_binary(&solution, 3), Some(true));
    }

    #[test]
    fn variable_array1_register_and_extract() {
        let mut model = MetaModel::<f64>::new("test_model");

        // 测试连续变量注册
        let mut array: VariableArray1<String, ContinuousVariableItem> = VariableArray1::new("x");
        let keys = vec!["layer_0".to_string(), "layer_1".to_string()];
        array.register_continuous(&keys, &mut model).unwrap();

        assert!(array.index(&"layer_0".to_string()).is_some());
        assert!(array.index(&"layer_1".to_string()).is_some());
        assert!(array.index(&"layer_2".to_string()).is_none());
        assert_eq!(array.len(), 2);

        // 测试二值变量注册
        let mut bin_array: VariableArray1<String, BinaryVariableItem> = VariableArray1::new("u");
        bin_array.register_binary(&keys, &mut model).unwrap();
        assert_eq!(bin_array.len(), 2);
    }
}
