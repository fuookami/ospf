//! Token 核心定义
//! Token Core Definitions

use std::fmt::Debug;
use std::sync::RwLock;

use num_traits::{FromPrimitive, ToPrimitive};

use crate::variable::{VariableId, VariableItem, VariableRange, VariableType, VariableTypeTrait};

// ============================================================================
// IntoValue - 值类型转换 Trait
// ============================================================================

/// 可转换的值类型 / Convertible Value Type
///
/// 约束变量类型的关联值类型可以转换到统一的值类型 `V`。
/// Constrains that the associated value type of a variable type can be converted to the unified value type `V`.
///
/// # 类型参数 / Type Parameters
///
/// - `V`: 目标值类型，由用户控制精度 / Target value type, precision controlled by user
///
/// # 示例 / Examples
///
/// ```rust
/// use ospf_rust_core::token::IntoValue;
///
/// // f64 -> f64 转换
/// let value: f64 = 1.5;
/// let converted: f64 = value.into_value();
/// assert_eq!(converted, 1.5);
/// ```
pub trait IntoValue<V>: Clone + Debug + PartialOrd + Send + Sync + 'static {
    /// 转换为目标值类型 / Convert to target value type
    fn into_value(self) -> V;

    /// 从目标值类型转换 / Convert from target value type
    fn from_value(value: V) -> Option<Self>;
}

// 扩展实现：f64 -> V / Extended implementation: f64 -> V
impl<V> IntoValue<V> for f64
where
    V: Clone + Debug + PartialOrd + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    fn into_value(self) -> V {
        V::from_f64(self).expect("failed to convert f64 into target value type")
    }

    fn from_value(value: V) -> Option<Self> {
        value.to_f64()
    }
}

// 扩展实现：i64 -> V / Extended implementation: i64 -> V
impl<V> IntoValue<V> for i64
where
    V: Clone + Debug + PartialOrd + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    fn into_value(self) -> V {
        V::from_i64(self).expect("failed to convert i64 into target value type")
    }

    fn from_value(value: V) -> Option<Self> {
        let value = value.to_f64()?;
        if value.is_finite()
            && value.fract() == 0.0
            && value >= i64::MIN as f64
            && value <= i64::MAX as f64
        {
            Some(value as i64)
        } else {
            None
        }
    }
}

// 扩展实现：f32 -> V / Extended implementation: f32 -> V
impl<V> IntoValue<V> for f32
where
    V: Clone + Debug + PartialOrd + Send + Sync + 'static + FromPrimitive + ToPrimitive,
{
    fn into_value(self) -> V {
        V::from_f32(self).expect("failed to convert f32 into target value type")
    }

    fn from_value(value: V) -> Option<Self> {
        value.to_f32()
    }
}

// ============================================================================
// VariableData - 变量数据（用于动态分发）
// ============================================================================

/// 变量数据 / Variable Data
///
/// 存储变量的核心信息，用于动态分发。
/// Stores core variable information for dynamic dispatch.
#[derive(Clone, Debug)]
pub struct VariableData<V = f64>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 变量 ID / Variable ID
    pub id: VariableId,
    /// 变量索引 / Variable index
    pub index: usize,
    /// 变量名称 / Variable name
    pub name: String,
    /// 显示名称 / Display name
    pub display_name: Option<String>,
    /// 变量类型 / Variable type
    pub var_type: VariableType,
    /// 下界 / Lower bound
    pub lower_bound: Option<V>,
    /// 上界 / Upper bound
    pub upper_bound: Option<V>,
}

impl<V> VariableData<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 从泛型变量项创建 / Create from generic variable item
    pub fn from_generic<VT: VariableTypeTrait>(item: &VariableItem<VT>) -> Self
    where
        VT::Value: IntoValue<V>,
    {
        let range = item.range().clone();
        Self {
            id: item.id(),
            index: item.index(),
            name: item.name().to_string(),
            display_name: item.display_name().map(|s| s.to_string()),
            var_type: VT::var_type(),
            lower_bound: range.lower_bound.map(|v| v.into_value()),
            upper_bound: range.upper_bound.map(|v| v.into_value()),
        }
    }

    /// 获取变量 ID / Get variable ID
    pub fn id(&self) -> VariableId {
        self.id
    }

    /// 获取变量索引 / Get variable index
    pub fn index(&self) -> usize {
        self.index
    }

    /// 获取变量名称 / Get variable name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取显示名称 / Get display name
    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    /// 获取变量类型 / Get variable type
    pub fn var_type(&self) -> VariableType {
        self.var_type
    }

    /// 检查值是否在有效范围内 / Check if value is valid
    pub fn is_valid_value(&self, value: &V) -> bool
    where
        V: PartialOrd,
    {
        let lower_ok = self.lower_bound.as_ref().is_none_or(|lb| value >= lb);
        let upper_ok = self.upper_bound.as_ref().is_none_or(|ub| value <= ub);
        lower_ok && upper_ok
    }

    /// 获取下界 / Get lower bound
    pub fn lower_bound(&self) -> Option<V> {
        self.lower_bound.clone()
    }

    /// 获取上界 / Get upper bound
    pub fn upper_bound(&self) -> Option<V> {
        self.upper_bound.clone()
    }

    /// 获取变量范围 / Get variable range
    pub fn range(&self) -> VariableRange<V> {
        VariableRange {
            lower_bound: self.lower_bound.clone(),
            upper_bound: self.upper_bound.clone(),
        }
    }

    /// 设置变量范围 / Set variable range
    pub fn set_range(&mut self, range: VariableRange<V>) {
        self.lower_bound = range.lower_bound;
        self.upper_bound = range.upper_bound;
    }
}

// ============================================================================
// AnyVariable - 任意类型变量包装器
// ============================================================================

/// 任意类型变量包装器 / Any-Type Variable Wrapper
///
/// 包装任意类型的变量，统一值类型为 `V`。
/// Wraps variables of any type, unifying value type to `V`.
///
/// # 类型参数 / Type Parameters
///
/// - `V`: 统一值类型 / Unified value type
#[derive(Clone, Debug)]
pub struct AnyVariable<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    data: VariableData<V>,
}

impl<V: Clone + Debug + Send + Sync + 'static> AnyVariable<V> {
    /// 从变量数据创建 / Create from variable data
    pub fn new(data: VariableData<V>) -> Self {
        Self { data }
    }

    /// 从 VariableItem 创建 / Create from VariableItem
    pub fn from_generic<VT>(item: VariableItem<VT>) -> Self
    where
        VT: VariableTypeTrait,
        VT::Value: IntoValue<V>,
    {
        Self::new(VariableData::<V>::from_generic(&item))
    }

    /// 获取变量 ID / Get variable ID
    pub fn id(&self) -> VariableId {
        self.data.id
    }

    /// 获取变量索引 / Get variable index
    pub fn index(&self) -> usize {
        self.data.index
    }

    /// 获取变量名称 / Get variable name
    pub fn name(&self) -> &str {
        &self.data.name
    }

    /// 获取显示名称 / Get display name
    pub fn display_name(&self) -> Option<&str> {
        self.data.display_name.as_deref()
    }

    /// 获取变量类型 / Get variable type
    pub fn var_type(&self) -> VariableType {
        self.data.var_type
    }

    /// 检查值是否在有效范围内 / Check if value is valid
    pub fn is_valid_value(&self, value: V) -> bool
    where
        V: PartialOrd,
    {
        self.data.is_valid_value(&value)
    }

    /// 获取下界 / Get lower bound
    pub fn lower_bound(&self) -> Option<V> {
        self.data.lower_bound.clone()
    }

    /// 获取上界 / Get upper bound
    pub fn upper_bound(&self) -> Option<V> {
        self.data.upper_bound.clone()
    }

    /// 获取变量范围 / Get variable range
    pub fn range(&self) -> VariableRange<V> {
        VariableRange {
            lower_bound: self.lower_bound(),
            upper_bound: self.upper_bound(),
        }
    }

    /// 设置变量范围 / Set variable range
    pub fn set_range(&mut self, range: VariableRange<V>) {
        self.data.set_range(range);
    }

    /// 获取内部数据引用 / Get inner data reference
    pub fn data(&self) -> &VariableData<V> {
        &self.data
    }
}

// ============================================================================
// Token - 变量在求解器中的表示
// ============================================================================

/// Token - 变量在求解器中的表示 / Token - Variable Representation in Solver
///
/// 统一的 Token 类型，不再区分变量类型。
/// Unified Token type, no longer distinguishing variable types.
///
/// # 类型参数 / Type Parameters
///
/// - `V`: 统一值类型 / Unified value type
///
/// # 示例 / Examples
///
/// ```rust
/// use ospf_rust_core::token::{Token, AnyVariable};
/// use ospf_rust_core::variable::{Binary, VariableItem};
///
/// // 创建二进制变量的 Token
/// let var = VariableItem::<Binary>::auto("x");
/// let token = Token::from_generic(var, 0);
///
/// // 设置求解结果
/// token.set_result(1.0);
/// assert_eq!(token.get_result(), Some(1.0));
/// ```
#[derive(Debug)]
pub struct Token<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 变量引用（统一类型） / Variable reference (unified type)
    pub variable: AnyVariable<V>,
    /// 求解器索引 / Solver index
    pub solver_index: usize,
    /// 求解结果 / Solution result
    pub result: RwLock<Option<V>>,
}

impl<V: Clone + Debug + Send + Sync + 'static> Token<V> {
    /// 创建新 Token / Create new token
    pub fn new(variable: AnyVariable<V>, solver_index: usize) -> Self {
        Self {
            variable,
            solver_index,
            result: RwLock::new(None),
        }
    }

    /// 从泛型变量创建 Token / Create token from generic variable
    pub fn from_generic<VT>(variable: VariableItem<VT>, solver_index: usize) -> Self
    where
        VT: VariableTypeTrait,
        VT::Value: IntoValue<V>,
    {
        Self::new(AnyVariable::from_generic(variable), solver_index)
    }

    /// 设置求解结果 / Set solution result
    pub fn set_result(&self, value: V) {
        *ospf_rust_base::write_unwrap!(&self.result) = Some(value);
    }

    /// 获取求解结果 / Get solution result
    pub fn get_result(&self) -> Option<V> {
        ospf_rust_base::read_unwrap!(&self.result).clone()
    }

    /// 清除求解结果 / Clear solution result
    pub fn clear_result(&self) {
        *ospf_rust_base::write_unwrap!(&self.result) = None;
    }

    /// 获取变量类型 / Get variable type
    pub fn var_type(&self) -> VariableType {
        self.variable.var_type()
    }

    /// 获取变量 ID / Get variable ID
    pub fn id(&self) -> VariableId {
        self.variable.id()
    }

    /// 获取变量名称 / Get variable name
    pub fn name(&self) -> &str {
        self.variable.name()
    }

    /// 检查是否有结果 / Check if has result
    pub fn has_result(&self) -> bool {
        ospf_rust_base::read_unwrap!(&self.result).is_some()
    }

    /// 获取变量范围 / Get variable range
    pub fn range(&self) -> VariableRange<V> {
        self.variable.range()
    }

    /// 设置变量范围 / Set variable range
    pub fn set_range(&mut self, range: VariableRange<V>) {
        self.variable.set_range(range);
    }
}

impl<V: Clone + Debug + Send + Sync + 'static> Clone for Token<V> {
    fn clone(&self) -> Self {
        Self {
            variable: self.variable.clone(),
            solver_index: self.solver_index,
            result: RwLock::new(self.get_result()),
        }
    }
}

impl<V: Clone + Debug + Send + Sync + 'static> PartialEq for Token<V> {
    fn eq(&self, other: &Self) -> bool {
        self.variable.id() == other.variable.id()
    }
}

impl<V: Clone + Debug + Send + Sync + 'static> Eq for Token<V> {}

impl<V: Clone + Debug + Send + Sync + 'static> std::hash::Hash for Token<V> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.variable.id().hash(state);
    }
}

// ============================================================================
// 类型别名 / Type Aliases
// ============================================================================

/// f64 精度的 Token / Token with f64 precision
pub type TokenF64 = Token<f64>;

/// f64 精度的 AnyVariable / AnyVariable with f64 precision
pub type AnyVariableF64 = AnyVariable<f64>;

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::variable::{Binary, Continuous};

    #[test]
    fn test_into_value_f64() {
        let value: f64 = 1.5;
        let converted: f64 = value.into_value();
        assert_eq!(converted, 1.5);

        let back = f64::from_value(1.5);
        assert_eq!(back, Some(1.5));
    }

    #[test]
    fn test_into_value_f32_roundtrip() {
        let value: f64 = 1.25;
        let converted: f32 = value.into_value();
        assert!((converted - 1.25_f32).abs() <= f32::EPSILON);

        let back = f32::from_value(2.5_f64);
        assert_eq!(back, Some(2.5_f32));
    }

    #[test]
    fn test_into_value_i64_requires_integer_and_range() {
        let valid = i64::from_value(42.0_f64);
        assert_eq!(valid, Some(42));

        let non_integer = i64::from_value(42.5_f64);
        assert_eq!(non_integer, None);

        let out_of_range = i64::from_value((i64::MAX as f64) * 2.0);
        assert_eq!(out_of_range, None);
    }

    #[test]
    fn test_any_variable() {
        let var = VariableItem::<Binary>::auto("x");
        let any_var: AnyVariableF64 = AnyVariable::from_generic(var);

        assert_eq!(any_var.name(), "x");
        assert_eq!(any_var.var_type(), VariableType::Binary);
    }

    #[test]
    fn test_token_creation() {
        let var = VariableItem::<Continuous>::auto("y");
        let token = TokenF64::from_generic(var, 0);

        assert_eq!(token.name(), "y");
        assert_eq!(token.solver_index, 0);
        assert_eq!(token.var_type(), VariableType::Continuous);
        assert!(!token.has_result());
    }

    #[test]
    fn test_token_result() {
        let var = VariableItem::<Binary>::auto("x");
        let token = TokenF64::from_generic(var, 0);

        token.set_result(1.0);
        assert!(token.has_result());
        assert_eq!(token.get_result(), Some(1.0));

        token.clear_result();
        assert!(!token.has_result());
        assert_eq!(token.get_result(), None);
    }

    #[test]
    fn test_token_clone() {
        let var = VariableItem::<Binary>::auto("x");
        let token1 = TokenF64::from_generic(var, 0);
        token1.set_result(1.0);

        let token2 = token1.clone();
        assert_eq!(token2.get_result(), Some(1.0));
        assert_eq!(token2, token1);
    }
}
