//! 变量项定义
//! Variable Item Definitions

use std::any::Any;
use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;
use ospf_rust_math::symbol::{DynSymbol, SymbolDynId};
use super::variable_type::{

    BalancedTernary, Binary, Continuous, Integer, Percentage, Ternary, UContinuous, UInteger,
};
use super::{VariableId, VariableRange, VariableType, VariableTypeTrait, new_standalone_id};

// ============================================================================
// VariableData - 泛型变量数据
// ============================================================================

/// 泛型变量数据 / Generic Variable Data
///
/// 使用类型参数指定变量类型，支持编译期类型检查。
/// Uses type parameter to specify variable type, supporting compile-time type checking.
///
/// # 类型参数 / Type Parameters
///
/// - `VT`: 变量类型标记（实现 `VariableTypeTrait`）/ Variable type marker (implements `VariableTypeTrait`)
///
/// # 示例 / Examples
///
/// ```rust
/// use ospf_rust_core::variable::{
///     VariableData, Binary, Continuous, VariableRange
/// };
///
/// // 创建二进制变量 / Create binary variable
/// let binary_var = VariableData::<Binary>::new(ospf_rust_core::variable::new_standalone_id(), "x");
///
/// // 创建连续变量（带自定义范围）/ Create continuous variable with custom range
/// let continuous_var = VariableData::<Continuous>::with_range(
///     ospf_rust_core::variable::new_standalone_id(),
///     "y",
///     VariableRange::bounded(-10.0, 10.0),
/// );
/// ```
#[derive(Debug)]
pub struct VariableData<VT: VariableTypeTrait> {
    /// 变量 ID / Variable ID
    pub id: VariableId,
    /// 索引 / Index
    pub index: usize,
    /// 名称 / Name
    pub name: String,
    /// 显示名称 / Display name
    pub display_name: Option<String>,
    /// 变量范围 / Variable range
    pub range: VariableRange<VT::Value>,
    /// 变量类型标记 / Variable type marker
    _marker: PhantomData<VT>,
}

impl<VT: VariableTypeTrait> VariableData<VT> {
    /// 创建新变量 / Create new variable
    pub fn new(id: VariableId, name: &str) -> Self {
        Self {
            id,
            index: id.index_in_group,
            name: name.to_string(),
            display_name: None,
            range: VT::default_range(),
            _marker: PhantomData,
        }
    }

    /// 创建带自定义范围的变量 / Create variable with custom range
    pub fn with_range(id: VariableId, name: &str, range: VariableRange<VT::Value>) -> Self {
        Self {
            id,
            index: id.index_in_group,
            name: name.to_string(),
            display_name: None,
            range,
            _marker: PhantomData,
        }
    }

    /// 创建带显示名称的变量 / Create variable with display name
    pub fn with_display_name(id: VariableId, name: &str, display_name: &str) -> Self {
        Self {
            id,
            index: id.index_in_group,
            name: name.to_string(),
            display_name: Some(display_name.to_string()),
            range: VT::default_range(),
            _marker: PhantomData,
        }
    }

    /// 创建完整的变量 / Create complete variable
    pub fn full(
        id: VariableId,
        name: &str,
        display_name: Option<String>,
        range: VariableRange<VT::Value>,
    ) -> Self {
        Self {
            id,
            index: id.index_in_group,
            name: name.to_string(),
            display_name,
            range,
            _marker: PhantomData,
        }
    }

    /// 获取变量类型枚举 / Get variable type enum
    pub fn var_type(&self) -> VariableType {
        VT::var_type()
    }

    /// 检查值是否有效 / Check if value is valid
    pub fn is_valid_value(&self, value: &VT::Value) -> bool {
        VT::is_valid_value(value) && self.range.contains(value)
    }

    /// 设置显示名称 / Set display name
    pub fn set_display_name(&mut self, name: &str) {
        self.display_name = Some(name.to_string());
    }

    /// 设置范围 / Set range
    pub fn set_range(&mut self, range: VariableRange<VT::Value>) {
        self.range = range;
    }
}

impl<VT: VariableTypeTrait> Clone for VariableData<VT> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            index: self.index,
            name: self.name.clone(),
            display_name: self.display_name.clone(),
            range: self.range.clone(),
            _marker: PhantomData,
        }
    }
}

impl<VT: VariableTypeTrait> fmt::Display for VariableData<VT> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.display_name {
            Some(dn) => write!(f, "{} ({})", dn, self.name),
            None => write!(f, "{}", self.name),
        }
    }
}

// ============================================================================
// VariableItem - 泛型变量项
// ============================================================================

/// 泛型变量项 / Generic Variable Item
///
/// `Clone` 但共享底层数据（通过 `Arc`）。
/// `Clone` but shares underlying data (via `Arc`).
///
/// # 设计说明 / Design Notes
///
/// 使用 `Arc` 包装内部数据，使得克隆操作成本低廉，
/// 同时保持变量的唯一性（通过 `id` 标识）。
///
/// Uses `Arc` to wrap internal data, making clone operation cheap,
/// while maintaining variable uniqueness (identified by `id`).
#[derive(Debug)]
pub struct VariableItem<VT: VariableTypeTrait> {
    data: Arc<VariableData<VT>>,
}

impl<VT: VariableTypeTrait> VariableItem<VT> {
    /// 从数据创建变量项 / Create variable item from data
    pub fn new(data: VariableData<VT>) -> Self {
        Self {
            data: Arc::new(data),
        }
    }

    /// 创建新变量项 / Create new variable item
    pub fn create(id: VariableId, name: &str) -> Self {
        Self::new(VariableData::new(id, name))
    }

    /// 自动创建新变量项（使用全局递增 ID）/ Auto-create variable item with global incremental ID
    ///
    /// 该方法会调用全局变量 ID 生成器分配独立变量 ID，避免用户手动指定 `VariableId`。
    /// This method allocates a standalone variable ID from the global generator,
    /// avoiding manual `VariableId` assignment.
    pub fn auto(name: &str) -> Self {
        Self::new(VariableData::new(new_standalone_id(), name))
    }

    /// 创建带范围的变量项 / Create variable item with range
    pub fn with_range(id: VariableId, name: &str, range: VariableRange<VT::Value>) -> Self {
        Self::new(VariableData::with_range(id, name, range))
    }

    /// 自动创建带范围的变量项（使用全局递增 ID）/ Auto-create ranged variable item with global incremental ID
    pub fn auto_with_range(name: &str, range: VariableRange<VT::Value>) -> Self {
        Self::new(VariableData::with_range(
            new_standalone_id(),
            name,
            range,
        ))
    }

    /// 获取变量 ID / Get variable ID
    pub fn id(&self) -> VariableId {
        self.data.id
    }

    /// 获取索引 / Get index
    pub fn index(&self) -> usize {
        self.data.index
    }

    /// 获取名称 / Get name
    pub fn name(&self) -> &str {
        &self.data.name
    }

    /// 获取显示名称 / Get display name
    pub fn display_name(&self) -> Option<&str> {
        self.data.display_name.as_deref()
    }

    /// 获取变量范围 / Get variable range
    pub fn range(&self) -> &VariableRange<VT::Value> {
        &self.data.range
    }

    /// 获取变量类型枚举 / Get variable type enum
    pub fn var_type(&self) -> VariableType {
        self.data.var_type()
    }

    /// 检查值是否有效 / Check if value is valid
    pub fn is_valid_value(&self, value: &VT::Value) -> bool {
        self.data.is_valid_value(value)
    }

    /// 获取内部数据引用 / Get inner data reference
    pub fn data(&self) -> &VariableData<VT> {
        &self.data
    }
}

impl<VT: VariableTypeTrait> Clone for VariableItem<VT> {
    fn clone(&self) -> Self {
        Self {
            data: Arc::clone(&self.data),
        }
    }
}

impl<VT: VariableTypeTrait> PartialEq for VariableItem<VT> {
    fn eq(&self, other: &Self) -> bool {
        self.data.id == other.data.id
    }
}

impl<VT: VariableTypeTrait> Eq for VariableItem<VT> {}

impl<VT: VariableTypeTrait> std::hash::Hash for VariableItem<VT> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.id.hash(state);
    }
}

impl<VT: VariableTypeTrait> fmt::Display for VariableItem<VT> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.fmt(f)
    }
}

// ============================================================================
// DynSymbol 实现 / DynSymbol Implementation
// ============================================================================

/// 为 VariableItem 实现 DynSymbol trait
/// Implement DynSymbol trait for VariableItem
///
/// 这允许变量项作为符号使用，参与多项式运算。
/// This allows variable items to be used as symbols in polynomial operations.
impl<VT: VariableTypeTrait> DynSymbol for VariableItem<VT> {
    /// 内部标识名 / Internal identifier name
    fn name(&self) -> &str {
        &self.data.name
    }

    /// 显示名称 / Display name
    fn display_name(&self) -> &str {
        self.data.display_name.as_deref().unwrap_or(&self.data.name)
    }

    /// 动态标识符 / Dynamic identifier
    ///
    /// 将 VariableId 映射到 SymbolDynId。
    /// Maps VariableId to SymbolDynId.
    fn dyn_id(&self) -> SymbolDynId<'_> {
        SymbolDynId::standalone(self.data.id.unique_id() as usize)
    }

    /// 转换为 Any 引用 / Convert to Any reference
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ============================================================================
// OwnedSymbol 转换 / OwnedSymbol Conversion
// ============================================================================

use ospf_rust_math::symbol::OwnedSymbol;

impl<VT: VariableTypeTrait> VariableItem<VT> {
    /// 转换为 OwnedSymbol / Convert to OwnedSymbol
    ///
    /// 用于参与多项式运算。
    /// Used to participate in polynomial operations.
    ///
    /// # 示例 / Examples
    ///
    /// ```rust
    /// use ospf_rust_core::variable::BinaryVariableItem;
    /// use ospf_rust_math::symbol::LinearMonomial;
    ///
    /// let x = BinaryVariableItem::auto("x");
    /// let symbol = x.to_owned_symbol();
    ///
    /// // 可以用于多项式运算
    /// let monomial: LinearMonomial<f64> = symbol * 2.0;
    /// ```
    pub fn to_owned_symbol(&self) -> OwnedSymbol {
        OwnedSymbol::new(self.clone())
    }
}

// ============================================================================
// 运算符重载 / Operator Overloading
// ============================================================================

use ospf_rust_math::symbol::{Linear, LinearMonomial, QuadraticMonomial};
use std::ops::{Add, Mul, Sub};

// O1: VariableItem * VariableItem → QuadraticMonomial<f64>
// 变量 × 变量 = 二次单项式
// Variable × Variable = Quadratic monomial
impl<VT: VariableTypeTrait> Mul for VariableItem<VT> {
    type Output = QuadraticMonomial<f64>;

    fn mul(self, rhs: Self) -> Self::Output {
        QuadraticMonomial::quadratic(1.0, self.to_owned_symbol(), rhs.to_owned_symbol())
    }
}

// O2: VariableItem * f64 → LinearMonomial<f64>
// 变量 × 标量 = 线性单项式
// Variable × Scalar = Linear monomial
impl<VT: VariableTypeTrait> Mul<f64> for VariableItem<VT> {
    type Output = LinearMonomial<f64>;

    fn mul(self, rhs: f64) -> Self::Output {
        LinearMonomial::new(rhs, self.to_owned_symbol())
    }
}

// O3: f64 * VariableItem → LinearMonomial<f64>
// 标量 × 变量 = 线性单项式
// Scalar × Variable = Linear monomial
impl<VT: VariableTypeTrait> Mul<VariableItem<VT>> for f64 {
    type Output = LinearMonomial<f64>;

    fn mul(self, rhs: VariableItem<VT>) -> Self::Output {
        LinearMonomial::new(self, rhs.to_owned_symbol())
    }
}

// O4: VariableItem + VariableItem → Linear<f64>
// 变量 + 变量 = 线性多项式
// Variable + Variable = Linear polynomial
impl<VT: VariableTypeTrait> Add for VariableItem<VT> {
    type Output = Linear<f64>;

    fn add(self, rhs: Self) -> Self::Output {
        Linear::new(
            vec![
                LinearMonomial::new(1.0, self.to_owned_symbol()),
                LinearMonomial::new(1.0, rhs.to_owned_symbol()),
            ],
            0.0,
        )
    }
}

// O5: VariableItem + f64 → Linear<f64>
// 变量 + 标量 = 线性多项式
// Variable + Scalar = Linear polynomial
impl<VT: VariableTypeTrait> Add<f64> for VariableItem<VT> {
    type Output = Linear<f64>;

    fn add(self, rhs: f64) -> Self::Output {
        Linear::new(vec![LinearMonomial::new(1.0, self.to_owned_symbol())], rhs)
    }
}

// O6: f64 + VariableItem → Linear<f64>
// 标量 + 变量 = 线性多项式
// Scalar + Variable = Linear polynomial
impl<VT: VariableTypeTrait> Add<VariableItem<VT>> for f64 {
    type Output = Linear<f64>;

    fn add(self, rhs: VariableItem<VT>) -> Self::Output {
        Linear::new(vec![LinearMonomial::new(1.0, rhs.to_owned_symbol())], self)
    }
}

// O7: VariableItem - VariableItem → Linear<f64>
// 变量 - 变量 = 线性多项式
// Variable - Variable = Linear polynomial
impl<VT: VariableTypeTrait> Sub for VariableItem<VT> {
    type Output = Linear<f64>;

    fn sub(self, rhs: Self) -> Self::Output {
        Linear::new(
            vec![
                LinearMonomial::new(1.0, self.to_owned_symbol()),
                LinearMonomial::new(-1.0, rhs.to_owned_symbol()),
            ],
            0.0,
        )
    }
}

// O8: VariableItem - f64 → Linear<f64>
// 变量 - 标量 = 线性多项式
// Variable - Scalar = Linear polynomial
impl<VT: VariableTypeTrait> Sub<f64> for VariableItem<VT> {
    type Output = Linear<f64>;

    fn sub(self, rhs: f64) -> Self::Output {
        Linear::new(vec![LinearMonomial::new(1.0, self.to_owned_symbol())], -rhs)
    }
}

// O9: f64 - VariableItem → Linear<f64>
// 标量 - 变量 = 线性多项式
// Scalar - Variable = Linear polynomial
impl<VT: VariableTypeTrait> Sub<VariableItem<VT>> for f64 {
    type Output = Linear<f64>;

    fn sub(self, rhs: VariableItem<VT>) -> Self::Output {
        Linear::new(vec![LinearMonomial::new(-1.0, rhs.to_owned_symbol())], self)
    }
}

// O10: &VariableItem * f64 → LinearMonomial<f64>
// 变量引用 × 标量 = 线性单项式
// Variable reference × Scalar = Linear monomial
impl<VT: VariableTypeTrait> Mul<f64> for &VariableItem<VT> {
    type Output = LinearMonomial<f64>;

    fn mul(self, rhs: f64) -> Self::Output {
        LinearMonomial::new(rhs, self.to_owned_symbol())
    }
}

// O11: f64 * &VariableItem → LinearMonomial<f64>
// 标量 × 变量引用 = 线性单项式
// Scalar × Variable reference = Linear monomial
impl<VT: VariableTypeTrait> Mul<&VariableItem<VT>> for f64 {
    type Output = LinearMonomial<f64>;

    fn mul(self, rhs: &VariableItem<VT>) -> Self::Output {
        LinearMonomial::new(self, rhs.to_owned_symbol())
    }
}

// ============================================================================
// 类型别名 / Type Aliases
// ============================================================================

/// 二进制变量数据 / Binary variable data
pub type BinaryVariableData = VariableData<Binary>;

/// 三元变量数据 / Ternary variable data
pub type TernaryVariableData = VariableData<Ternary>;

/// 平衡三元变量数据 / Balanced ternary variable data
pub type BalancedTernaryVariableData = VariableData<BalancedTernary>;

/// 百分比变量数据 / Percentage variable data
pub type PercentageVariableData = VariableData<Percentage>;

/// 整数变量数据 / Integer variable data
pub type IntegerVariableData = VariableData<Integer>;

/// 无符号整数变量数据 / Unsigned integer variable data
pub type UIntegerVariableData = VariableData<UInteger>;

/// 连续变量数据 / Continuous variable data
pub type ContinuousVariableData = VariableData<Continuous>;

/// 无符号连续变量数据 / Unsigned continuous variable data
pub type UContinuousVariableData = VariableData<UContinuous>;

// ---------------------------------------------------------------------------

/// 二进制变量项 / Binary variable item
pub type BinaryVariableItem = VariableItem<Binary>;

/// 三元变量项 / Ternary variable item
pub type TernaryVariableItem = VariableItem<Ternary>;

/// 平衡三元变量项 / Balanced ternary variable item
pub type BalancedTernaryVariableItem = VariableItem<BalancedTernary>;

/// 百分比变量项 / Percentage variable item
pub type PercentageVariableItem = VariableItem<Percentage>;

/// 整数变量项 / Integer variable item
pub type IntegerVariableItem = VariableItem<Integer>;

/// 无符号整数变量项 / Unsigned integer variable item
pub type UIntegerVariableItem = VariableItem<UInteger>;

/// 连续变量项 / Continuous variable item
pub type ContinuousVariableItem = VariableItem<Continuous>;

/// 无符号连续变量项 / Unsigned continuous variable item
pub type UContinuousVariableItem = VariableItem<UContinuous>;

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_variable_creation() {
        let id = VariableId::standalone(0);
        let data = BinaryVariableData::new(id, "x");
        assert_eq!(data.id, id);
        assert_eq!(data.name, "x");
        assert_eq!(data.var_type(), VariableType::Binary);
        assert_eq!(data.range.lower_bound, Some(0.0));
        assert_eq!(data.range.upper_bound, Some(1.0));
    }

    #[test]
    fn test_continuous_variable_with_range() {
        let id = VariableId::standalone(1);
        let range = VariableRange::bounded(-10.0, 10.0);
        let data = ContinuousVariableData::with_range(id, "y", range.clone());
        assert_eq!(data.range, range);
    }

    #[test]
    fn test_variable_item_cloning() {
        let id = VariableId::standalone(0);
        let item1 = BinaryVariableItem::create(id, "x");
        let item2 = item1.clone();
        assert_eq!(item1, item2);
        assert_eq!(item1.id(), item2.id());
    }

    #[test]
    fn test_variable_item_hash() {
        use std::collections::HashSet;

        let id1 = VariableId::standalone(0);
        let id2 = VariableId::standalone(0);
        let id3 = VariableId::standalone(1);

        let item1 = BinaryVariableItem::create(id1, "x");
        let item2 = BinaryVariableItem::create(id2, "x");
        let item3 = BinaryVariableItem::create(id3, "y");

        let mut set = HashSet::new();
        set.insert(item1.clone());
        assert!(set.contains(&item2)); // 相同 ID
        assert!(!set.contains(&item3)); // 不同 ID
    }

    #[test]
    fn test_is_valid_value() {
        let id = VariableId::standalone(0);
        let item = BinaryVariableItem::create(id, "x");

        assert!(item.is_valid_value(&0.0));
        assert!(item.is_valid_value(&1.0));
        assert!(item.is_valid_value(&0.5));
        assert!(!item.is_valid_value(&2.0));
        assert!(!item.is_valid_value(&-1.0));
    }

    #[test]
    fn test_auto_variable_creation_uses_unique_global_ids() {
        let x = BinaryVariableItem::auto("x_auto");
        let y = BinaryVariableItem::auto("y_auto");
        assert_ne!(x.id(), y.id());
        assert!(x.id().is_standalone());
        assert!(y.id().is_standalone());
    }

    #[test]
    fn test_auto_with_range_preserves_bounds() {
        let item =
            ContinuousVariableItem::auto_with_range("z_auto", VariableRange::bounded(-3.0, 7.0));
        assert_eq!(item.range().lower_bound, Some(-3.0));
        assert_eq!(item.range().upper_bound, Some(7.0));
    }
}
