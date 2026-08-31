# OSPF Rust Core - 运筹建模框架设计文档
# OSPF Rust Core - Operations Research Modeling Framework Design Document

[中文](#中文) | [English](#english)

---

<a name="中文"></a>
## 中文

### 一、概述

本模块实现一个运筹学建模框架，支持线性规划 (LP)、混合整数规划 (MIP)、二次规划 (QP) 等优化问题的建模与求解。

#### 设计目标

1. **类型安全**: 利用 Rust 类型系统保证编译期正确性
2. **零成本抽象**: 泛型设计避免运行时开销
3. **易用性**: 提供声明式建模 API
4. **可扩展**: 支持自定义变量类型、约束和求解器

---

### 二、架构设计

框架采用**分层架构**设计：

```
┌─────────────────────────────────────────────────────────────┐
│                     用户建模层 (MetaModel)                     │
│   - 用户友好的 API，支持约束和目标函数的声明式添加                 │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                   机制模型层 (MechanismModel)                  │
│   - 展开 Symbol，生成实际的约束和变量                           │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                  中间模型层 (IntermediateModel)                │
│   - 转换为求解器可理解的格式 (如 LP/QP 标准形式)                  │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                      求解器层 (Solver)                         │
│   - 对接 Gurobi, COPT, SCIP 等商业/开源求解器                   │
└─────────────────────────────────────────────────────────────┘
```

---

### 三、模块设计

#### 3.1 变量系统 (Variable System)

**目录结构**:
```
src/variable/
├── mod.rs
├── variable_type.rs      # 变量类型枚举
├── variable_range.rs     # 变量范围
├── variable_item.rs      # 单个变量
└── variable_combination.rs  # 变量组合（多维变量）
```

**核心类型**:

```rust
/// 变量类型枚举 / Variable Type Enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableType {
    /// 二进制变量 (0 或 1) / Binary variable (0 or 1)
    Binary,
    /// 三元变量 (0, 1, 或 2) / Ternary variable (0, 1, or 2)
    Ternary,
    /// 平衡三元变量 (-1, 0, 或 1) / Balanced ternary variable (-1, 0, or 1)
    BalancedTernary,
    /// 百分比变量 [0, 1] / Percentage variable [0, 1]
    Percentage,
    /// 整数变量 / Integer variable
    Integer,
    /// 无符号整数变量 / Unsigned integer variable
    UInteger,
    /// 连续变量 / Continuous variable
    Continuous,
    /// 无符号连续变量 / Unsigned continuous variable
    UContinuous,
}

// ============================================================================
// 变量类型标记类型 / Variable Type Marker Types
// ============================================================================

/// 变量类型 trait / Variable Type Trait
/// 
/// 为变量类型提供元信息，包括值类型、边界、默认范围等。
/// Provides metadata for variable types, including value type, bounds, default range, etc.
/// 
/// # 类型参数 / Type Parameters
/// - `V`: 值类型（如 f64, i64）/ Value type (e.g., f64, i64)
/// 
/// # 示例 / Examples
/// 
/// ```rust
/// // 获取二进制变量的默认范围
/// let range = Binary::default_range();
/// assert_eq!(range.lower_bound, Some(0.0));
/// assert_eq!(range.upper_bound, Some(1.0));
/// 
/// // 检查值是否在范围内
/// assert!(Binary::is_valid_value(0.5));
/// assert!(!Binary::is_valid_value(2.0));
/// ```
pub trait VariableTypeTrait: Clone + Debug + Default + Send + Sync + 'static {
    /// 值类型 / Value type
    type Value: Clone + Debug + PartialOrd + Send + Sync + 'static;
    
    /// 获取变量类型枚举 / Get variable type enum
    fn var_type() -> VariableType;
    
    /// 获取最小值 / Get minimum value
    /// 
    /// 返回 `None` 表示无下界。
    /// Returns `None` for no lower bound.
    fn min_value() -> Option<Self::Value>;
    
    /// 获取最大值 / Get maximum value
    /// 
    /// 返回 `None` 表示无上界。
    /// Returns `None` for no upper bound.
    fn max_value() -> Option<Self::Value>;
    
    /// 获取默认取值范围 / Get default range
    /// 
    /// 默认实现基于 `min_value` 和 `max_value`。
    /// Default implementation based on `min_value` and `max_value`.
    fn default_range() -> VariableRange<Self::Value> {
        VariableRange {
            lower_bound: Self::min_value(),
            upper_bound: Self::max_value(),
        }
    }
    
    /// 检查值是否有效 / Check if value is valid
    /// 
    /// 检查值是否在允许的取值范围内。
    /// Checks if value is within allowed range.
    fn is_valid_value(value: &Self::Value) -> bool {
        let range = Self::default_range();
        let lower_ok = range.lower_bound.as_ref().map_or(true, |lb| value >= lb);
        let upper_ok = range.upper_bound.as_ref().map_or(true, |ub| value <= ub);
        lower_ok && upper_ok
    }
    
    /// 是否为整数变量 / Whether it's an integer variable
    fn is_integer() -> bool {
        false
    }
    
    /// 是否为有符号变量 / Whether it's a signed variable
    fn is_signed() -> bool {
        true
    }
    
    /// 获取类型名称 / Get type name
    fn type_name() -> &'static str;
}

/// 二进制变量标记类型 / Binary variable marker type
/// 
/// 取值范围: {0, 1}
/// Value range: {0, 1}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Binary;

impl VariableTypeTrait for Binary {
    type Value = f64;
    
    fn var_type() -> VariableType { VariableType::Binary }
    fn min_value() -> Option<Self::Value> { Some(0.0) }
    fn max_value() -> Option<Self::Value> { Some(1.0) }
    fn is_integer() -> bool { true }
    fn type_name() -> &'static str { "Binary" }
}

/// 三元变量标记类型 / Ternary variable marker type
/// 
/// 取值范围: {0, 1, 2}
/// Value range: {0, 1, 2}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Ternary;

impl VariableTypeTrait for Ternary {
    type Value = f64;
    
    fn var_type() -> VariableType { VariableType::Ternary }
    fn min_value() -> Option<Self::Value> { Some(0.0) }
    fn max_value() -> Option<Self::Value> { Some(2.0) }
    fn is_integer() -> bool { true }
    fn type_name() -> &'static str { "Ternary" }
}

/// 平衡三元变量标记类型 / Balanced ternary variable marker type
/// 
/// 取值范围: {-1, 0, 1}
/// Value range: {-1, 0, 1}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BalancedTernary;

impl VariableTypeTrait for BalancedTernary {
    type Value = f64;
    
    fn var_type() -> VariableType { VariableType::BalancedTernary }
    fn min_value() -> Option<Self::Value> { Some(-1.0) }
    fn max_value() -> Option<Self::Value> { Some(1.0) }
    fn is_integer() -> bool { true }
    fn type_name() -> &'static str { "BalancedTernary" }
}

/// 百分比变量标记类型 / Percentage variable marker type
/// 
/// 取值范围: [0, 1]
/// Value range: [0, 1]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Percentage;

impl VariableTypeTrait for Percentage {
    type Value = f64;
    
    fn var_type() -> VariableType { VariableType::Percentage }
    fn min_value() -> Option<Self::Value> { Some(0.0) }
    fn max_value() -> Option<Self::Value> { Some(1.0) }
    fn is_integer() -> bool { false }
    fn type_name() -> &'static str { "Percentage" }
}

/// 整数变量标记类型 / Integer variable marker type
/// 
/// 取值范围: (-∞, +∞)，默认范围可自定义
/// Value range: (-∞, +∞), default range can be customized
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Integer;

impl VariableTypeTrait for Integer {
    type Value = f64;
    
    fn var_type() -> VariableType { VariableType::Integer }
    fn min_value() -> Option<Self::Value> { None }
    fn max_value() -> Option<Self::Value> { None }
    fn is_integer() -> bool { true }
    fn type_name() -> &'static str { "Integer" }
}

/// 无符号整数变量标记类型 / Unsigned integer variable marker type
/// 
/// 取值范围: [0, +∞)
/// Value range: [0, +∞)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct UInteger;

impl VariableTypeTrait for UInteger {
    type Value = f64;
    
    fn var_type() -> VariableType { VariableType::UInteger }
    fn min_value() -> Option<Self::Value> { Some(0.0) }
    fn max_value() -> Option<Self::Value> { None }
    fn is_integer() -> bool { true }
    fn is_signed() -> bool { false }
    fn type_name() -> &'static str { "UInteger" }
}

/// 连续变量标记类型 / Continuous variable marker type
/// 
/// 取值范围: (-∞, +∞)
/// Value range: (-∞, +∞)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Continuous;

impl VariableTypeTrait for Continuous {
    type Value = f64;
    
    fn var_type() -> VariableType { VariableType::Continuous }
    fn min_value() -> Option<Self::Value> { None }
    fn max_value() -> Option<Self::Value> { None }
    fn is_integer() -> bool { false }
    fn type_name() -> &'static str { "Continuous" }
}

/// 无符号连续变量标记类型 / Unsigned continuous variable marker type
/// 
/// 取值范围: [0, +∞)
/// Value range: [0, +∞)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct UContinuous;

impl VariableTypeTrait for UContinuous {
    type Value = f64;
    
    fn var_type() -> VariableType { VariableType::UContinuous }
    fn min_value() -> Option<Self::Value> { Some(0.0) }
    fn max_value() -> Option<Self::Value> { None }
    fn is_integer() -> bool { false }
    fn is_signed() -> bool { false }
    fn type_name() -> &'static str { "UContinuous" }
}

// ============================================================================
// 泛型变量数据 / Generic Variable Data
// ============================================================================

/// 泛型变量数据 / Generic Variable Data
/// 
/// 使用类型参数指定变量类型，支持编译期类型检查。
/// Uses type parameter to specify variable type, supporting compile-time type checking.
/// 
/// # 类型参数 / Type Parameters
/// - `VT`: 变量类型标记（实现 `VariableTypeTrait`）/ Variable type marker (implements `VariableTypeTrait`)
/// 
/// # 示例 / Examples
/// 
/// ```rust
/// // 创建二进制变量
/// let binary_var = GenericVariableData::<Binary>::new(
///     VariableId::standalone(0),
///     "x",
/// );
/// 
/// // 创建连续变量（带自定义范围）
/// let continuous_var = GenericVariableData::<Continuous>::with_range(
///     VariableId::standalone(1),
///     "y",
///     VariableRange::new(Some(-10.0), Some(10.0)),
/// );
/// ```
#[derive(Debug)]
pub struct GenericVariableData<VT: VariableTypeTrait> {
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

impl<VT: VariableTypeTrait> GenericVariableData<VT> {
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
    
    /// 获取变量类型枚举 / Get variable type enum
    pub fn var_type(&self) -> VariableType {
        VT::var_type()
    }
    
    /// 检查值是否有效 / Check if value is valid
    pub fn is_valid_value(&self, value: &VT::Value) -> bool {
        VT::is_valid_value(value)
    }
}

// ============================================================================
// 类型别名 / Type Aliases
// ============================================================================

/// 二进制变量项 / Binary variable item
pub type BinaryVariableItem = GenericVariableItem<Binary>;

/// 三元变量项 / Ternary variable item
pub type TernaryVariableItem = GenericVariableItem<Ternary>;

/// 平衡三元变量项 / Balanced ternary variable item
pub type BalancedTernaryVariableItem = GenericVariableItem<BalancedTernary>;

/// 百分比变量项 / Percentage variable item
pub type PercentageVariableItem = GenericVariableItem<Percentage>;

/// 整数变量项 / Integer variable item
pub type IntegerVariableItem = GenericVariableItem<Integer>;

/// 无符号整数变量项 / Unsigned integer variable item
pub type UIntegerVariableItem = GenericVariableItem<UInteger>;

/// 连续变量项 / Continuous variable item
pub type ContinuousVariableItem = GenericVariableItem<Continuous>;

/// 无符号连续变量项 / Unsigned continuous variable item
pub type UContinuousVariableItem = GenericVariableItem<UContinuous>;

/// 泛型变量项 / Generic Variable Item
/// 
/// `Clone` 但共享底层数据（通过 `Arc`）。
/// `Clone` but shares underlying data (via `Arc`).
#[derive(Debug, Clone)]
pub struct GenericVariableItem<VT: VariableTypeTrait> {
    data: Arc<GenericVariableData<VT>>,
}

impl<VT: VariableTypeTrait> GenericVariableItem<VT> {
    pub fn id(&self) -> VariableId { self.data.id }
    pub fn index(&self) -> usize { self.data.index }
    pub fn name(&self) -> &str { &self.data.name }
    pub fn display_name(&self) -> Option<&str> { self.data.display_name.as_deref() }
    pub fn range(&self) -> &VariableRange<VT::Value> { &self.data.range }
    pub fn var_type(&self) -> VariableType { VT::var_type() }
}

/// 变量范围 / Variable Range
#[derive(Debug, Clone, PartialEq)]
pub struct VariableRange<T> {
    pub lower_bound: Option<T>,
    pub upper_bound: Option<T>,
}

/// 变量 ID / Variable ID
/// 
/// 变量的唯一标识符，实现 `SymbolId` trait。
/// Unique identifier for a variable, implements `SymbolId` trait.
/// 
/// 由两部分组成：
/// - `group_id`: 由 ID 生成器递增生成的组 ID
/// - `index_in_group`: 在变量组中的序号（独立变量为 0）
/// 
/// Composed of two parts:
/// - `group_id`: Group ID incremented by ID generator
/// - `index_in_group`: Index within the variable group (0 for standalone variables)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VariableId {
    /// 组 ID（由 ID 生成器递增生成）/ Group ID (incremented by ID generator)
    pub group_id: usize,
    /// 组内索引 / Index within group
    pub index_in_group: usize,
}

impl VariableId {
    /// 创建新的变量 ID / Create new variable ID
    pub fn new(group_id: usize, index_in_group: usize) -> Self {
        Self { group_id, index_in_group }
    }
    
    /// 创建独立变量 ID / Create standalone variable ID
    /// 
    /// 独立变量的 `index_in_group` 恒为 0。
    /// Standalone variables always have `index_in_group` equal to 0.
    pub fn standalone(group_id: usize) -> Self {
        Self { group_id, index_in_group: 0 }
    }
}

/// 实现 SymbolId trait / Implement SymbolId trait
impl SymbolId for VariableId {
    type Id = Self;
    
    fn id(&self) -> Self::Id {
        *self
    }
}

/// 变量 ID 生成器 / Variable ID Generator
/// 
/// 用于生成唯一的变量组 ID。
/// Used to generate unique variable group IDs.
#[derive(Debug, Default)]
pub struct VariableIdGenerator {
    next_id: AtomicUsize,
}

impl VariableIdGenerator {
    /// 创建新的 ID 生成器 / Create new ID generator
    pub fn new() -> Self {
        Self {
            next_id: AtomicUsize::new(0),
        }
    }
    
    /// 创建带起始 ID 的生成器 / Create generator with starting ID
    pub fn with_start(start: usize) -> Self {
        Self {
            next_id: AtomicUsize::new(start),
        }
    }
    
    /// 生成下一个组 ID / Generate next group ID
    /// 
    /// 返回一个新的唯一组 ID。
    /// Returns a new unique group ID.
    pub fn next_group_id(&self) -> usize {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }
    
    /// 生成独立变量 ID / Generate standalone variable ID
    pub fn next_standalone_id(&self) -> VariableId {
        VariableId::standalone(self.next_group_id())
    }
    
    /// 生成变量组 ID 列表 / Generate variable group ID list
    /// 
    /// 生成一个变量组中所有变量的 ID。
    /// Generates IDs for all variables in a group.
    pub fn next_group_ids(&self, count: usize) -> Vec<VariableId> {
        let group_id = self.next_group_id();
        (0..count).map(|i| VariableId::new(group_id, i)).collect()
    }
}

/// 全局变量 ID 生成器 / Global variable ID generator
/// 
/// 用于全局变量 ID 分配。
/// Used for global variable ID allocation.
pub static GLOBAL_VARIABLE_ID_GENERATOR: Lazy<VariableIdGenerator> = Lazy::new(VariableIdGenerator::new);

/// 变量数据 / Variable Data
/// 
/// 变量的实际数据存储。
/// Actual data storage for a variable.
/// 
/// 该类型不直接使用，而是通过 `VariableItem` 的 `Arc` 指针引用。
/// This type is not used directly, but referenced through `Arc` pointer in `VariableItem`.
#[derive(Debug)]
pub struct VariableData {
    /// 唯一标识符 / Unique identifier
    pub id: u64,
    /// 索引 / Index
    pub index: usize,
    /// 名称 / Name
    pub name: String,
    /// 显示名称 / Display name
    pub display_name: Option<String>,
    /// 变量类型 / Variable type
    pub var_type: VariableType,
    /// 变量范围 / Variable range
    pub range: VariableRange<f64>,
}

/// 变量项 / Variable Item
/// 
/// 单个决策变量，实现 `DynSymbol` trait。
/// A single decision variable, implements `DynSymbol` trait.
/// 
/// `VariableItem` 是 `Clone` 的，克隆时共享底层数据（通过 `Arc`）。
/// `VariableItem` is `Clone`, cloning shares the underlying data (via `Arc`).
#[derive(Debug, Clone)]
pub struct VariableItem {
    /// 指向实际数据的 Arc 指针 / Arc pointer to actual data
    data: Arc<VariableData>,
}

impl VariableItem {
    /// 获取唯一标识符 / Get unique identifier
    pub fn id(&self) -> u64 {
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
    
    /// 获取变量类型 / Get variable type
    pub fn var_type(&self) -> VariableType {
        self.data.var_type
    }
    
    /// 获取变量范围 / Get variable range
    pub fn range(&self) -> &VariableRange<f64> {
        &self.data.range
    }
}

/// 变量组合 / Variable Combination
/// 
/// 使用 `MultiArray` 组织多维变量。
/// Uses `MultiArray` to organize multi-dimensional variables.
/// 
/// # 示例 / Examples
/// 
/// ```rust
/// // 一维变量 / 1D variables
/// let vars_1d: VariableCombination<Shape1> = VariableCombination::new(
///     "x",
///     Shape::new([10]),
///     VariableType::Continuous,
/// );
/// 
/// // 二维变量 / 2D variables
/// let vars_2d: VariableCombination<Shape2> = VariableCombination::new(
///     "y",
///     Shape::new([5, 10]),
///     VariableType::Binary,
/// );
/// ```
pub struct VariableCombination<S: AbstractShape> {
    pub id: u64,
    pub name: String,
    pub items: MultiArray<VariableItem, S>,
}

// 类型别名 / Type aliases
pub type Variable1D = VariableCombination<Shape1>;
pub type Variable2D = VariableCombination<Shape2>;
pub type Variable3D = VariableCombination<Shape3>;
```

**设计要点**:
- `VariableItem` 实现 `DynSymbol`，可与现有符号系统无缝集成
- 变量组合支持多维数组索引
- 范围支持动态设置和查询

**内存池优化 / Memory Pool Optimization**:

为了优化多维变量的内存分配性能，使用 `typed_arena` Arena 分配器：

```rust
use typed_arena::Arena;
use std::cell::RefCell;

/// 变量 Arena / Variable Arena
/// 
/// 用于批量分配变量数据，减少内存碎片和分配开销。
/// Used for batch allocation of variable data, reducing memory fragmentation and allocation overhead.
/// 
/// `typed_arena` 提供类型化的 Arena 分配，比通用 Arena 更高效。
/// `typed_arena` provides typed arena allocation, more efficient than generic arenas.
/// 
/// # 示例 / Examples
/// 
/// ```rust
/// let arena = VariableArena::new();
/// 
/// // 批量创建变量 / Batch create variables
/// let vars: Vec<VariableItem> = (0..1000)
///     .map(|i| arena.alloc(VariableData {
///         id: VariableId::standalone(i),
///         index: i,
///         name: format!("x_{}", i),
///         display_name: None,
///         var_type: VariableType::Continuous,
///         range: VariableRange::new(Some(0.0), Some(100.0)),
///     }))
///     .collect();
/// ```
pub struct VariableArena {
    /// 类型化 Arena 分配器 / Typed arena allocator
    arena: Arena<VariableData>,
}

impl VariableArena {
    /// 创建新的 Arena / Create new arena
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
        }
    }
    
    /// 创建带预留容量的 Arena / Create arena with reserved capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            arena: Arena::with_capacity(capacity),
        }
    }
    
    /// 在 Arena 中分配变量数据 / Allocate variable data in arena
    /// 
    /// 返回的 `VariableItem` 持有 Arena 中数据的引用。
    /// The returned `VariableItem` holds a reference to data in the arena.
    pub fn alloc(&self, data: VariableData) -> VariableItem {
        VariableItem {
            data: Arc::new(self.arena.alloc(data).clone()),
        }
    }
    
    /// 批量分配变量 / Batch allocate variables
    /// 
    /// 使用 Arena 分配器批量创建变量，性能更优。
    /// Batch create variables using arena allocator for better performance.
    pub fn alloc_iter<I: IntoIterator<Item = VariableData>>(&self, iter: I) -> Vec<VariableItem> {
        iter.into_iter()
            .map(|data| self.alloc(data))
            .collect()
    }
    
    /// 预留容量 / Reserve capacity
    /// 
    /// 预留指定数量的元素空间。
    /// Reserves space for the specified number of elements.
    pub fn reserve(&mut self, additional: usize) {
        self.arena.reserve(additional);
    }
    
    /// 获取已分配数量 / Get allocated count
    pub fn len(&self) -> usize {
        self.arena.len()
    }
    
    /// 检查是否为空 / Check if empty
    pub fn is_empty(&self) -> bool {
        self.arena.is_empty()
    }
}

impl Default for VariableArena {
    fn default() -> Self {
        Self::new()
    }
}

/// 泛型变量 Arena / Generic Variable Arena
/// 
/// 支持任意类型变量的 Arena 分配器。
/// Arena allocator supporting arbitrary variable types.
/// 
/// # 类型参数 / Type Parameters
/// - `VT`: 变量类型标记（实现 `VariableTypeTrait`）
/// 
/// # 示例 / Examples
/// 
/// ```rust
/// let arena: GenericVariableArena<Binary> = GenericVariableArena::new();
/// 
/// // 创建二进制变量
/// let binary_var = arena.alloc(GenericVariableData::<Binary>::new(
///     VariableId::standalone(0),
///     "x",
/// ));
/// ```
pub struct GenericVariableArena<VT: VariableTypeTrait> {
    arena: Arena<GenericVariableData<VT>>,
    _marker: PhantomData<VT>,
}

impl<VT: VariableTypeTrait> GenericVariableArena<VT> {
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
            _marker: PhantomData,
        }
    }
    
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            arena: Arena::with_capacity(capacity),
            _marker: PhantomData,
        }
    }
    
    pub fn alloc(&self, data: GenericVariableData<VT>) -> GenericVariableItem<VT> {
        GenericVariableItem {
            data: Arc::new(self.arena.alloc(data).clone()),
        }
    }
    
    pub fn alloc_iter<I: IntoIterator<Item = GenericVariableData<VT>>>(&self, iter: I) -> Vec<GenericVariableItem<VT>> {
        iter.into_iter()
            .map(|data| self.alloc(data))
            .collect()
    }
    
    pub fn reserve(&mut self, additional: usize) {
        self.arena.reserve(additional);
    }
}

impl<VT: VariableTypeTrait> Default for GenericVariableArena<VT> {
    fn default() -> Self {
        Self::new()
    }
}

/// 线程安全的变量 Arena / Thread-safe variable arena
/// 
/// 支持多线程并发分配（内部使用 `RefCell`）。
/// Supports multi-threaded concurrent allocation (uses `RefCell` internally).
pub struct ConcurrentVariableArena {
    arena: RefCell<Arena<VariableData>>,
}

impl ConcurrentVariableArena {
    pub fn new() -> Self {
        Self {
            arena: RefCell::new(Arena::new()),
        }
    }
    
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            arena: RefCell::new(Arena::with_capacity(capacity)),
        }
    }
    
    pub fn alloc(&self, data: VariableData) -> VariableItem {
        VariableItem {
            data: Arc::new(self.arena.borrow_mut().alloc(data).clone()),
        }
    }
    
    pub fn alloc_iter<I: IntoIterator<Item = VariableData>>(&self, iter: I) -> Vec<VariableItem> {
        iter.into_iter()
            .map(|data| self.alloc(data))
            .collect()
    }
}

impl Default for ConcurrentVariableArena {
    fn default() -> Self {
        Self::new()
    }
}

// 类型别名 / Type aliases for common variable arenas
pub type BinaryArena = GenericVariableArena<Binary>;
pub type ContinuousArena = GenericVariableArena<Continuous>;
pub type IntegerArena = GenericVariableArena<Integer>;
```

**性能优势 / Performance Benefits**:

| 特性 | 传统分配 | typed_arena 分配 |
|------|---------|-----------------|
| 分配速度 | O(n) 每次分配 | O(1) 均摊 |
| 内存碎片 | 高 | 低 |
| 缓存友好性 | 差 | 好 |
| 类型安全 | 运行时 | 编译期 |
| 批量释放 | 不支持 | 自动 (drop) |

**typed_arena vs bumpalo 对比 / Comparison**:

| 特性 | typed_arena | bumpalo |
|------|-------------|---------|
| 类型安全 | ✅ 编译期检查 | ❌ 运行时检查 |
| 分配速度 | 更快 (单类型优化) | 快 |
| 内存对齐 | 自动处理 | 需要手动处理 |
| API 简洁度 | 简单 | 较复杂 |
| 适用场景 | 单类型批量分配 | 多类型混合分配 |

**依赖 / Dependencies**:
```toml
[dependencies]
typed-arena = "2"
```

---

#### 3.2 Token 系统

**目录结构**:
```
src/token/
├── mod.rs
├── token.rs              # Token 定义
├── token_list.rs         # TokenList (Token 集合)
└── token_table.rs        # TokenTable (管理变量和符号)
```

**核心类型**:

```rust
use std::sync::{Arc, RwLock};
use ospf_rust_math::symbol::DynSymbol;

/// Token - 变量在求解器中的表示
/// Token - Variable representation in solver
/// 
/// # 类型参数 / Type Parameters
/// - `VT`: 变量类型标记（实现 `VariableTypeTrait`）
#[derive(Debug)]
pub struct Token<VT: VariableTypeTrait = Continuous> {
    /// 变量引用 / Variable reference
    pub variable: GenericVariableItem<VT>,
    /// 求解器索引 / Solver index
    pub solver_index: usize,
    /// 求解结果 / Solution result
    pub result: RwLock<Option<VT::Value>>,
}

impl<VT: VariableTypeTrait> Token<VT> {
    /// 创建新 Token / Create new token
    pub fn new(variable: GenericVariableItem<VT>, solver_index: usize) -> Self {
        Self {
            variable,
            solver_index,
            result: RwLock::new(None),
        }
    }
    
    /// 设置求解结果 / Set solution result
    pub fn set_result(&self, value: VT::Value) {
        *self.result.write().unwrap() = Some(value);
    }
    
    /// 获取求解结果 / Get solution result
    pub fn get_result(&self) -> Option<VT::Value> {
        self.result.read().unwrap().clone()
    }
    
    /// 清除求解结果 / Clear solution result
    pub fn clear_result(&self) {
        *self.result.write().unwrap() = None;
    }
}

/// 泛型 Token 列表 trait / Generic Token List trait
pub trait TokenList<VT: VariableTypeTrait = Continuous>: Send + Sync {
    /// 获取所有 Token / Get all tokens
    fn tokens(&self) -> &Vec<Token<VT>>;
    
    /// 获取求解器中的 Token / Get tokens in solver
    fn tokens_in_solver(&self) -> Vec<&Token<VT>>;
    
    /// 查找变量对应的 Token / Find token by variable
    fn find(&self, variable: &GenericVariableItem<VT>) -> Option<&Token<VT>>;
    
    /// 通过 ID 查找 Token / Find token by ID
    fn find_by_id(&self, id: VariableId) -> Option<&Token<VT>>;
    
    /// 通过索引查找 Token / Find token by index
    fn find_by_index(&self, index: usize) -> Option<&Token<VT>>;
    
    /// 设置求解结果 / Set solution
    fn set_solution(&self, solution: &[VT::Value]);
    
    /// 清除求解结果 / Clear solution
    fn clear_solution(&self);
}

/// TokenTable - 管理变量和中间符号
/// TokenTable - Manages variables and intermediate symbols
pub trait TokenTable: TokenList {
    /// 类别 / Category
    fn category(&self) -> Category;
    
    /// 获取所有中间符号 / Get all intermediate symbols
    fn symbols(&self) -> &Vec<Arc<dyn IntermediateSymbol>>;
    
    /// 添加变量 / Add variable
    fn add_variable<VT: VariableTypeTrait>(&mut self, variable: GenericVariableItem<VT>) -> Result<()>;
    
    /// 添加中间符号 / Add intermediate symbol
    fn add_symbol(&mut self, symbol: Arc<dyn IntermediateSymbol>) -> Result<()>;
    
    /// 缓存符号值 / Cache symbol value
    fn cache_value(&self, symbol: &dyn IntermediateSymbol, value: f64);
    
    /// 获取缓存的符号值 / Get cached symbol value
    fn cached_value(&self, symbol: &dyn IntermediateSymbol) -> Option<f64>;
}

/// 可添加 Token 的集合 / Addable Token Collection
pub trait AddableTokenCollection {
    /// 添加变量 / Add variable
    fn add_variable<VT: VariableTypeTrait>(&mut self, variable: GenericVariableItem<VT>) -> Result<()>;
    
    /// 批量添加变量 / Add variables
    fn add_variables<VT: VariableTypeTrait, I: IntoIterator<Item = GenericVariableItem<VT>>>(
        &mut self, 
        variables: I
    ) -> Result<()> {
        for var in variables {
            self.add_variable(var)?;
        }
        Ok(())
    }
}
```

**设计要点**:
- `Token` 支持泛型变量类型，自动匹配值类型
- `TokenList` 和 `TokenTable` 支持泛型参数
- 使用 `VariableId` 进行变量查找和索引
- 支持线程安全的并发访问

---

#### 3.3 泛型值类型参数系统 (Generic Value Type System)

**设计目标**:
- 用户可控制精度（如 `f64`、`BigDecimal`）
- 值类型参数贯穿整个系统
- 变量类型的关联值类型可转换到统一值类型

**核心类型**:

```rust
use std::marker::PhantomData;

// ============================================================================
// 值类型约束 / Value Type Constraint
// ============================================================================

/// 可转换的值类型 / Convertible Value Type
/// 
/// 约束变量类型的关联值类型可以转换到统一的值类型 `V`。
/// Constrains that the associated value type of a variable type can be converted to the unified value type `V`.
/// 
/// # 类型参数 / Type Parameters
/// - `V`: 目标值类型，由用户控制精度 / Target value type, precision controlled by user
pub trait IntoValue<V>: Clone + PartialOrd + Send + Sync + 'static {
    /// 转换为目标值类型 / Convert to target value type
    fn into_value(self) -> V;
    
    /// 从目标值类型转换 / Convert from target value type
    fn from_value(value: V) -> Option<Self>;
}

// 标准实现：f64 -> f64 / Standard implementation: f64 -> f64
impl IntoValue<f64> for f64 {
    fn into_value(self) -> f64 { self }
    fn from_value(value: f64) -> Option<Self> { Some(value) }
}

// BigDecimal 支持（需要启用 feature）/ BigDecimal support (requires feature)
#[cfg(feature = "bigdecimal")]
use bigdecimal::BigDecimal;

#[cfg(feature = "bigdecimal")]
impl IntoValue<BigDecimal> for f64 {
    fn into_value(self) -> BigDecimal { BigDecimal::try_from(self).unwrap_or_default() }
    fn from_value(value: BigDecimal) -> Option<Self> { 
        value.try_into().ok() 
    }
}

// ============================================================================
// 变量包装 trait（带值类型参数）/ Variable Wrapper Trait (with Value Type Parameter)
// ============================================================================

/// 变量包装 trait / Variable Wrapper Trait
/// 
/// 为不同类型的变量提供统一的接口。
/// Provides unified interface for variables of different types.
/// 
/// # 类型参数 / Type Parameters
/// - `V`: 统一值类型，由用户控制精度 / Unified value type, precision controlled by user
pub trait Variable<V>: Clone + Debug + Send + Sync + 'static {
    /// 获取变量 ID / Get variable ID
    fn id(&self) -> VariableId;
    
    /// 获取变量索引 / Get variable index
    fn index(&self) -> usize;
    
    /// 获取变量名称 / Get variable name
    fn name(&self) -> &str;
    
    /// 获取变量类型 / Get variable type
    fn var_type(&self) -> VariableType;
    
    /// 检查值是否在有效范围内 / Check if value is valid
    fn is_valid_value(&self, value: V) -> bool;
    
    /// 获取下界 / Get lower bound
    fn lower_bound(&self) -> Option<V>;
    
    /// 获取上界 / Get upper bound
    fn upper_bound(&self) -> Option<V>;
}

// ============================================================================
// 泛型变量包装器 / Generic Variable Wrapper
// ============================================================================

/// 泛型变量包装器 / Generic Variable Wrapper
/// 
/// 包装任意类型的变量，统一值类型为 `V`。
/// Wraps variables of any type, unifying value type to `V`.
pub struct AnyVariable<V> {
    inner: Arc<dyn Variable<V>>,
}

impl<V: Clone + Debug + Send + Sync + 'static> Clone for AnyVariable<V> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<V: Clone + Debug + Send + Sync + 'static> Debug for AnyVariable<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyVariable")
            .field("name", &self.inner.name())
            .field("var_type", &self.inner.var_type())
            .finish()
    }
}

impl<V: Clone + Debug + Send + Sync + 'static> Variable<V> for AnyVariable<V> {
    fn id(&self) -> VariableId { self.inner.id() }
    fn index(&self) -> usize { self.inner.index() }
    fn name(&self) -> &str { self.inner.name() }
    fn var_type(&self) -> VariableType { self.inner.var_type() }
    fn is_valid_value(&self, value: V) -> bool { self.inner.is_valid_value(value) }
    fn lower_bound(&self) -> Option<V> { self.inner.lower_bound() }
    fn upper_bound(&self) -> Option<V> { self.inner.upper_bound() }
}

impl<V> AnyVariable<V> {
    /// 从泛型变量创建 / Create from generic variable
    pub fn new(var: impl Variable<V>) -> Self {
        Self { inner: Arc::new(var) }
    }
    
    /// 从 GenericVariableItem 创建 / Create from GenericVariableItem
    pub fn from_generic<VT>(item: GenericVariableItem<VT>) -> Self 
    where 
        VT: VariableTypeTrait,
        VT::Value: IntoValue<V>,
    {
        Self::new(GenericVariableWrapper::<VT, V>::new(item))
    }
}

// ============================================================================
// 泛型变量包装器实现 / Generic Variable Wrapper Implementation
// ============================================================================

/// 泛型变量包装器实现 / Generic Variable Wrapper Implementation
/// 
/// 用于将 `GenericVariableItem<VT>` 包装为实现 `Variable<V>` trait。
struct GenericVariableWrapper<VT, V> 
where 
    VT: VariableTypeTrait,
    VT::Value: IntoValue<V>,
{
    item: GenericVariableItem<VT>,
    _value: PhantomData<V>,
}

impl<VT, V> GenericVariableWrapper<VT, V>
where
    VT: VariableTypeTrait,
    VT::Value: IntoValue<V>,
{
    fn new(item: GenericVariableItem<VT>) -> Self {
        Self { item, _value: PhantomData }
    }
}

impl<VT, V> Clone for GenericVariableWrapper<VT, V>
where
    VT: VariableTypeTrait,
    VT::Value: IntoValue<V>,
{
    fn clone(&self) -> Self {
        Self { item: self.item.clone(), _value: PhantomData }
    }
}

impl<VT, V> Debug for GenericVariableWrapper<VT, V>
where
    VT: VariableTypeTrait,
    VT::Value: IntoValue<V>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenericVariableWrapper")
            .field("name", &self.item.name())
            .finish()
    }
}

impl<VT, V> Variable<V> for GenericVariableWrapper<VT, V>
where
    VT: VariableTypeTrait,
    VT::Value: IntoValue<V>,
{
    fn id(&self) -> VariableId { self.item.id() }
    fn index(&self) -> usize { self.item.index() }
    fn name(&self) -> &str { self.item.name() }
    fn var_type(&self) -> VariableType { VT::var_type() }
    
    fn is_valid_value(&self, value: V) -> bool {
        VT::Value::from_value(value)
            .map(|v| VT::is_valid_value(&v))
            .unwrap_or(false)
    }
    
    fn lower_bound(&self) -> Option<V> {
        VT::min_value().map(|v| v.into_value())
    }
    
    fn upper_bound(&self) -> Option<V> {
        VT::max_value().map(|v| v.into_value())
    }
}

// ============================================================================
// 统一 Token 类型 / Unified Token Type
// ============================================================================

/// Token - 变量在求解器中的表示 / Token - Variable representation in solver
/// 
/// 统一的 Token 类型，不再区分变量类型。
/// Unified Token type, no longer distinguishing variable types.
/// 
/// # 类型参数 / Type Parameters
/// - `V`: 统一值类型 / Unified value type
#[derive(Debug)]
pub struct Token<V> {
    /// 变量引用（统一类型）/ Variable reference (unified type)
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
    pub fn from_generic<VT>(variable: GenericVariableItem<VT>, solver_index: usize) -> Self 
    where
        VT: VariableTypeTrait,
        VT::Value: IntoValue<V>,
    {
        Self::new(AnyVariable::from_generic(variable), solver_index)
    }
    
    /// 设置求解结果 / Set solution result
    pub fn set_result(&self, value: V) {
        *self.result.write().unwrap() = Some(value);
    }
    
    /// 获取求解结果 / Get solution result
    pub fn get_result(&self) -> Option<V> {
        self.result.read().unwrap().clone()
    }
    
    /// 清除求解结果 / Clear solution result
    pub fn clear_result(&self) {
        *self.result.write().unwrap() = None;
    }
    
    /// 获取变量类型 / Get variable type
    pub fn var_type(&self) -> VariableType {
        self.variable.var_type()
    }
}

// ============================================================================
// TokenList trait（带值类型参数）/ TokenList trait (with value type parameter)
// ============================================================================

/// Token 列表 trait / Token List trait
pub trait TokenList<V>: Send + Sync {
    /// 获取所有 Token / Get all tokens
    fn tokens(&self) -> &Vec<Token<V>>;
    
    /// 获取求解器中的 Token / Get tokens in solver
    fn tokens_in_solver(&self) -> Vec<&Token<V>>;
    
    /// 通过变量查找 Token / Find token by variable
    fn find(&self, variable: &AnyVariable<V>) -> Option<&Token<V>>;
    
    /// 通过 ID 查找 Token / Find token by ID
    fn find_by_id(&self, id: VariableId) -> Option<&Token<V>>;
    
    /// 通过索引查找 Token / Find token by index
    fn find_by_index(&self, index: usize) -> Option<&Token<V>>;
    
    /// 设置求解结果 / Set solution
    fn set_solution(&self, solution: &[V]);
    
    /// 清除求解结果 / Clear solution
    fn clear_solution(&self);
}

// ============================================================================
// 类型别名 / Type Aliases
// ============================================================================

/// f64 精度的 Token / Token with f64 precision
pub type TokenF64 = Token<f64>;

/// BigDecimal 精度的 Token / Token with BigDecimal precision
#[cfg(feature = "bigdecimal")]
pub type TokenBigDecimal = Token<BigDecimal>;
```

**值类型参数贯穿层级**:

| 层级 | 值类型参数位置 |
|-----|---------------|
| `VariableTypeTrait` | `type Value` (原始类型) |
| `IntoValue<V>` | 转换约束 |
| `Variable<V>` | trait 参数 |
| `AnyVariable<V>` | 结构体参数 |
| `Token<V>` | 结构体参数 |
| `TokenList<V>` | trait 参数 |
| `MetaModel<V>` | 结构体参数 |

---

#### 3.4 表达式平展系统 (Expression Flatten System)

**目录结构**:
```
src/flatten/
├── mod.rs
├── context.rs           # FlattenContext trait
├── callback.rs          # FlattenCallback trait
├── flattenable.rs       # Flattenable trait
└── lazy_context.rs      # LazyLinearFlattenContext 等
```

**核心设计**:

```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::cell::OnceCell;

// ============================================================================
// 缓存 Key 策略 / Cache Key Strategy
// ============================================================================

/// 缓存 Key / Cache Key
/// 
/// 用于标识平展结果的缓存键。
/// Cache key for identifying flattened results.
/// 
/// # 设计说明 / Design Notes
/// 
/// - **单项式/多项式**: 使用 `Box<Inner>` 设计，缓存 Key 使用堆地址
/// - **中间符号**: 使用 `Arc<Inner>` 设计（需要共享），缓存 Key 使用 Arc 内部地址
/// 
/// - **Monomial/Polynomial**: Uses `Box<Inner>` design, cache key uses heap address
/// - **Intermediate Symbol**: Uses `Arc<Inner>` design (needs sharing), cache key uses Arc inner address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// Key 值（内存地址）/ Key value (memory address)
    pub value: u64,
}

impl CacheKey {
    /// 从 Arc 指针地址创建（用于中间符号）/ Create from Arc pointer address (for intermediate symbols)
    pub fn from_arc<T>(ptr: &Arc<T>) -> Self {
        Self {
            // 使用 Arc 内部指针的地址作为 key
            // Use the address of the inner pointer in Arc as key
            value: Arc::as_ptr(ptr) as u64,
        }
    }
    
    /// 从 Box 内部地址创建（用于单项式/多项式）/ Create from Box inner address (for monomials/polynomials)
    pub fn from_box<T>(inner: &T) -> Self {
        Self {
            // 使用 Box 内部数据的地址作为 key
            // Use the address of the inner Box data as key
            value: inner as *const T as u64,
        }
    }
    
    /// 从原始地址创建 / Create from raw address
    pub fn from_raw(addr: u64) -> Self {
        Self { value: addr }
    }
}

// ============================================================================
// 可缓存 trait / Cacheable Trait
// ============================================================================

/// 可缓存的 trait / Cacheable Trait
/// 
/// 拥有稳定内存地址的对象可实现此 trait，用于缓存。
/// Objects with stable memory addresses can implement this trait for caching.
pub trait Cacheable {
    /// 获取缓存 Key / Get cache key
    fn cache_key(&self) -> CacheKey;
}

// ============================================================================
// Inner Box 设计 - 单项式 / Inner Box Design - Monomial
// ============================================================================

/// 线性单项式内部数据 / Linear Monomial Inner Data
/// 
/// 实际的单项式数据，存储在 Box 中以获得稳定地址，Clone 为深拷贝。
/// Actual monomial data, stored in Box for stable address, Clone performs deep copy.
#[derive(Debug, Clone)]
pub struct LinearMonomialInner<V> {
    /// 系数 / Coefficient
    pub coefficient: V,
    /// 符号 / Symbol
    pub symbol: OwnedSymbol,
}

/// 线性单项式 / Linear Monomial
/// 
/// 包装 Inner Box 的单项式，拥有稳定内存地址，Clone 为深拷贝。
/// Monomial wrapping Inner Box, having stable memory address, Clone performs deep copy.
/// 
/// # 设计说明 / Design Notes
/// 
/// - 使用 `Box<Inner>` 而非 `Arc<Inner>`，确保深拷贝语义
/// - 每个实例拥有唯一的堆地址，可作为缓存 Key
/// - 无引用计数开销，更轻量
#[derive(Debug, Clone)]
pub struct LinearMonomial<V> {
    inner: Box<LinearMonomialInner<V>>,
}

impl<V> LinearMonomial<V> {
    /// 创建新单项式 / Create new monomial
    pub fn new(coefficient: V, symbol: OwnedSymbol) -> Self {
        Self {
            inner: Box::new(LinearMonomialInner { coefficient, symbol }),
        }
    }
    
    /// 获取系数 / Get coefficient
    pub fn coefficient(&self) -> &V {
        &self.inner.coefficient
    }
    
    /// 获取符号 / Get symbol
    pub fn symbol(&self) -> &OwnedSymbol {
        &self.inner.symbol
    }
    
    /// 获取内部 Box 引用 / Get inner Box reference
    pub fn inner(&self) -> &LinearMonomialInner<V> {
        &self.inner
    }
}

impl<V> Cacheable for LinearMonomial<V> {
    fn cache_key(&self) -> CacheKey {
        // 使用 Box 内部指针地址作为 Key
        // Use the address of the inner Box pointer as key
        CacheKey::from_raw(&*self.inner as *const _ as u64)
    }
}

// ============================================================================
// Inner Box 设计 - 多项式 / Inner Box Design - Polynomial
// ============================================================================

/// 线性多项式内部数据 / Linear Polynomial Inner Data
/// 
/// 实际的多项式数据，存储在 Box 中以获得稳定地址，Clone 为深拷贝。
/// Actual polynomial data, stored in Box for stable address, Clone performs deep copy.
#[derive(Debug, Clone)]
pub struct LinearInner<V> {
    /// 单项式列表 / Monomial list
    pub monomials: Vec<LinearMonomial<V>>,
    /// 常数项 / Constant term
    pub constant: V,
}

/// 线性多项式 / Linear Polynomial
/// 
/// 包装 Inner Box 的多项式，拥有稳定内存地址，Clone 为深拷贝。
/// Polynomial wrapping Inner Box, having stable memory address, Clone performs deep copy.
/// 
/// # 设计优势 / Design Benefits
/// 
/// 1. **稳定地址**: 每个 `Linear<V>` 实例都有唯一的内存地址
/// 2. **高效缓存**: 缓存 Key 直接使用地址，无需计算哈希
/// 3. **深拷贝语义**: 每次克隆都是独立的新实例
/// 4. **比较高效**: 地址比较比内容比较更快
#[derive(Debug, Clone)]
pub struct Linear<V> {
    inner: Box<LinearInner<V>>,
}

impl<V> Linear<V> {
    /// 创建新多项式 / Create new polynomial
    pub fn new(monomials: Vec<LinearMonomial<V>>, constant: V) -> Self {
        Self {
            inner: Box::new(LinearInner { monomials, constant }),
        }
    }
    
    /// 创建空多项式 / Create empty polynomial
    pub fn zero(constant: V) -> Self 
    where 
        V: Default,
    {
        Self {
            inner: Box::new(LinearInner { 
                monomials: Vec::new(), 
                constant: constant,
            }),
        }
    }
    
    /// 获取单项式列表 / Get monomial list
    pub fn monomials(&self) -> &[LinearMonomial<V>] {
        &self.inner.monomials
    }
    
    /// 获取常数项 / Get constant
    pub fn constant(&self) -> &V {
        &self.inner.constant
    }
    
    /// 获取内部 Box 引用 / Get inner Box reference
    pub fn inner(&self) -> &LinearInner<V> {
        &self.inner
    }
}

impl<V> Cacheable for Linear<V> {
    fn cache_key(&self) -> CacheKey {
        // 使用 Box 内部指针地址作为 Key
        // Use the address of the inner Box pointer as key
        CacheKey::from_raw(&*self.inner as *const _ as u64)
    }
}

// ============================================================================
// 二次多项式 Inner Box 设计 / Quadratic Polynomial Inner Box Design
// ============================================================================

/// 二次单项式内部数据 / Quadratic Monomial Inner Data
#[derive(Debug, Clone)]
pub struct QuadraticMonomialInner<V> {
    pub coefficient: V,
    pub symbol1: OwnedSymbol,
    pub symbol2: Option<OwnedSymbol>,
}

/// 二次单项式 / Quadratic Monomial
/// 
/// 包装 Inner Box，Clone 为深拷贝。
/// Wrapping Inner Box, Clone performs deep copy.
#[derive(Debug, Clone)]
pub struct QuadraticMonomial<V> {
    inner: Box<QuadraticMonomialInner<V>>,
}

impl<V> QuadraticMonomial<V> {
    pub fn new_quadratic(coefficient: V, symbol1: OwnedSymbol, symbol2: OwnedSymbol) -> Self {
        Self {
            inner: Box::new(QuadraticMonomialInner {
                coefficient,
                symbol1,
                symbol2: Some(symbol2),
            }),
        }
    }
    
    pub fn new_linear(coefficient: V, symbol: OwnedSymbol) -> Self {
        Self {
            inner: Box::new(QuadraticMonomialInner {
                coefficient,
                symbol1: symbol,
                symbol2: None,
            }),
        }
    }
    
    pub fn cache_key(&self) -> CacheKey {
        CacheKey::from_raw(&*self.inner as *const _ as u64)
    }
}

/// 二次多项式内部数据 / Quadratic Polynomial Inner Data
#[derive(Debug, Clone)]
pub struct QuadraticInner<V> {
    pub monomials: Vec<QuadraticMonomial<V>>,
    pub constant: V,
}

/// 二次多项式 / Quadratic Polynomial
/// 
/// 包装 Inner Box，Clone 为深拷贝。
/// Wrapping Inner Box, Clone performs deep copy.
#[derive(Debug, Clone)]
pub struct Quadratic<V> {
    inner: Box<QuadraticInner<V>>,
}

impl<V> Quadratic<V> {
    pub fn new(monomials: Vec<QuadraticMonomial<V>>, constant: V) -> Self {
        Self {
            inner: Box::new(QuadraticInner { monomials, constant }),
        }
    }
    
    pub fn cache_key(&self) -> CacheKey {
        CacheKey::from_raw(&*self.inner as *const _ as u64)
    }
}

// ============================================================================
// 标准多项式 Inner Box 设计 / Canonical Polynomial Inner Box Design
// ============================================================================

/// 标准单项式内部数据 / Canonical Monomial Inner Data
#[derive(Debug, Clone)]
pub struct CanonicalMonomialInner<V> {
    pub coefficient: V,
    /// 符号到幂次的映射 / Symbol to power mapping
    pub powers: HashMap<OwnedSymbol, i32>,
}

/// 标准单项式 / Canonical Monomial
/// 
/// 包装 Inner Box，Clone 为深拷贝。
/// Wrapping Inner Box, Clone performs deep copy.
#[derive(Debug, Clone)]
pub struct CanonicalMonomial<V> {
    inner: Box<CanonicalMonomialInner<V>>,
}

impl<V> CanonicalMonomial<V> {
    pub fn cache_key(&self) -> CacheKey {
        CacheKey::from_raw(&*self.inner as *const _ as u64)
    }
}

/// 标准多项式内部数据 / Canonical Polynomial Inner Data
#[derive(Debug, Clone)]
pub struct CanonicalInner<V> {
    pub monomials: Vec<CanonicalMonomial<V>>,
    pub constant: V,
}

/// 标准多项式 / Canonical Polynomial
/// 
/// 包装 Inner Box，Clone 为深拷贝。
/// Wrapping Inner Box, Clone performs deep copy.
#[derive(Debug, Clone)]
pub struct Canonical<V> {
    inner: Box<CanonicalInner<V>>,
}

impl<V> Canonical<V> {
    pub fn new(monomials: Vec<CanonicalMonomial<V>>, constant: V) -> Self {
        Self {
            inner: Box::new(CanonicalInner { monomials, constant }),
        }
    }
    
    pub fn cache_key(&self) -> CacheKey {
        CacheKey::from_raw(&*self.inner as *const _ as u64)
    }
}

// ============================================================================
// 平展结果类型 / Flatten Result Types
// ============================================================================

/// 平展结果 / Flatten Result
/// 
/// 所有平展操作的结果都是多项式，不区分单项式/多项式/中间符号。
/// Results of all flatten operations are polynomials, no distinction between monomial/polynomial/symbol.
/// 
/// # 设计说明 / Design Notes
/// 
/// 统一使用多项式作为平展结果的原因：
/// 1. 单项式可以视为只有一项的多项式
/// 2. 中间符号平展后本质也是多项式
/// 3. 简化缓存设计，避免多种结果类型
/// 
/// Reasons for unified polynomial result:
/// 1. Monomial can be viewed as polynomial with single term
/// 2. Intermediate symbol flatten result is essentially polynomial
/// 3. Simplifies cache design, avoids multiple result types
#[derive(Debug, Clone)]
pub struct FlattenResult<P> {
    /// 平展后的多项式 / Flattened polynomial
    pub polynomial: P,
}

// 类型别名 / Type aliases
pub type LinearFlattenResult<V> = FlattenResult<Linear<V>>;
pub type QuadraticFlattenResult<V> = FlattenResult<Quadratic<V>>;
pub type CanonicalFlattenResult<V> = FlattenResult<Canonical<V>>;

// ============================================================================
// 计算值缓存 / Computed Value Cache
// ============================================================================

/// 计算值缓存 Key / Computed Value Cache Key
/// 
/// 用于缓存基于解或 TokenTable 计算得到的值。
/// Used for caching values computed from solution or TokenTable.
/// 
/// # 设计说明 / Design Notes
/// 
/// 计算值缓存用于以下场景：
/// 1. 从部分初始解计算全部初始解
/// 2. 中间符号的值计算
/// 3. 目标函数值计算
/// 
/// Computed value cache is used for:
/// 1. Computing full initial solution from partial initial solution
/// 2. Intermediate symbol value computation
/// 3. Objective function value computation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueCacheKey {
    /// 缓存类型 / Cache type
    pub kind: ValueCacheKind,
    /// 对象 ID（多项式地址或符号 ID）/ Object ID (polynomial address or symbol ID)
    pub object_id: u64,
}

/// 计算值缓存类型 / Computed Value Cache Kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueCacheKind {
    /// 多项式求值 / Polynomial evaluation
    Polynomial,
    /// 中间符号值 / Intermediate symbol value
    IntermediateSymbol,
    /// 目标函数值 / Objective function value
    Objective,
}

impl ValueCacheKey {
    /// 从多项式创建 / Create from polynomial
    pub fn from_polynomial<P: Cacheable>(poly: &P) -> Self {
        Self {
            kind: ValueCacheKind::Polynomial,
            object_id: poly.cache_key().value,
        }
    }
    
    /// 从中间符号创建 / Create from intermediate symbol
    pub fn from_symbol(identifier: u64) -> Self {
        Self {
            kind: ValueCacheKind::IntermediateSymbol,
            object_id: identifier,
        }
    }
    
    /// 从目标函数创建 / Create from objective
    pub fn from_objective(id: u64) -> Self {
        Self {
            kind: ValueCacheKind::Objective,
            object_id: id,
        }
    }
}

/// 计算值缓存上下文 trait / Computed Value Cache Context Trait
/// 
/// 类似于 FlattenContextTrait，提供值缓存的统一接口。
/// Similar to FlattenContextTrait, provides unified interface for value caching.
/// 
/// # 设计说明 / Design Notes
/// 
/// - 生命周期与 MetaModel 一致，延迟初始化直到 TokenTable 可用
/// - 支持基于解或 TokenTable 计算得到的值缓存
/// - 可按类型或对象清除缓存
/// 
/// - Lifecycle matches MetaModel, lazy initialization until TokenTable is available
/// - Supports caching values computed from solution or TokenTable
/// - Can clear cache by type or object
pub trait ValueCacheContextTrait<V>: Send + Sync {
    /// 获取 TokenTable 引用 / Get TokenTable reference
    fn token_table(&self) -> &dyn TokenList<V>;
    
    /// 缓存映射 / Cache map
    fn cache(&self) -> &HashMap<ValueCacheKey, V>;
    fn cache_mut(&mut self) -> &mut HashMap<ValueCacheKey, V>;
    
    /// 获取缓存值 / Get cached value
    fn get(&self, key: ValueCacheKey) -> Option<&V> {
        self.cache().get(&key)
    }
    
    /// 设置缓存值 / Set cached value
    fn set(&mut self, key: ValueCacheKey, value: V) {
        self.cache_mut().insert(key, value);
    }
    
    /// 获取或计算 / Get or compute
    fn get_or_compute<F>(&mut self, key: ValueCacheKey, f: F) -> &V
    where
        F: FnOnce() -> V,
    {
        self.cache_mut().entry(key).or_insert_with(f)
    }
    
    /// 清除所有缓存 / Clear all cache
    fn clear(&mut self) {
        self.cache_mut().clear();
    }
    
    /// 清除指定类型的缓存 / Clear cache by kind
    fn clear_kind(&mut self, kind: ValueCacheKind) {
        self.cache_mut().retain(|k, _| k.kind != kind);
    }
    
    /// 清除指定对象的缓存 / Clear cache by object
    fn clear_object(&mut self, key: ValueCacheKey) -> bool {
        self.cache_mut().remove(&key).is_some()
    }
    
    /// 缓存大小 / Cache size
    fn len(&self) -> usize {
        self.cache().len()
    }
    
    /// 是否为空 / Is empty
    fn is_empty(&self) -> bool {
        self.cache().is_empty()
    }
    
    /// 检查变量是否已注册 / Check if variable is registered
    fn is_registered(&self, var_id: VariableId) -> bool {
        self.token_table().find_by_id(var_id).is_some()
    }
    
    /// 通过 ID 查找 Token / Find token by ID
    fn find_token(&self, var_id: VariableId) -> Option<&Token<V>> {
        self.token_table().find_by_id(var_id)
    }
}

// ============================================================================
// 懒加载值缓存上下文 / Lazy Value Cache Context
// ============================================================================

/// 懒加载的值缓存上下文 / Lazy Value Cache Context
/// 
/// 生命周期与 MetaModel 一致，但延迟初始化直到 token_table 可用。
/// Lifecycle matches MetaModel, but lazy initialization until token_table is available.
/// 
/// # 设计说明 / Design Notes
/// 
/// 与 `LazyLinearFlattenContext` 类似的设计：
/// - 使用 `OnceCell` 延迟初始化 TokenTable 引用
/// - 存储计算值的缓存
/// - 支持按类型或对象清除缓存
pub struct LazyValueCacheContext<V, T: TokenList<V>> {
    /// TokenTable 引用（延迟初始化）/ TokenTable reference (lazy initialization)
    token_table: OnceCell<Arc<T>>,
    /// 缓存映射 / Cache map
    cache: HashMap<ValueCacheKey, V>,
    _value: PhantomData<V>,
}

impl<V, T: TokenList<V>> LazyValueCacheContext<V, T> {
    /// 创建未初始化的上下文 / Create uninitialized context
    pub fn new() -> Self {
        Self {
            token_table: OnceCell::new(),
            cache: HashMap::new(),
            _value: PhantomData,
        }
    }
    
    /// 初始化（设置 token_table 引用）/ Initialize (set token_table reference)
    /// 
    /// 只能调用一次，后续调用将被忽略。
    /// Can only be called once, subsequent calls will be ignored.
    pub fn init(&self, token_table: Arc<T>) {
        let _ = self.token_table.set(token_table);
    }
    
    /// 检查是否已初始化 / Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.token_table.get().is_some()
    }
    
    /// 获取 token_table（需确保已初始化）/ Get token_table (must be initialized)
    pub fn token_table_ref(&self) -> Option<&T> {
        self.token_table.get().map(|arc| arc.as_ref())
    }
}

impl<V: Clone + Debug + Send + Sync + 'static, T: TokenList<V>> ValueCacheContextTrait<V> 
    for LazyValueCacheContext<V, T> 
{
    fn token_table(&self) -> &dyn TokenList<V> {
        self.token_table.get()
            .expect("ValueCacheContext not initialized. Call init() first.")
            .as_ref()
    }
    
    fn cache(&self) -> &HashMap<ValueCacheKey, V> {
        &self.cache
    }
    
    fn cache_mut(&mut self) -> &mut HashMap<ValueCacheKey, V> {
        &mut self.cache
    }
}

impl<V, T: TokenList<V>> Default for LazyValueCacheContext<V, T> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 类型别名 / Type Aliases
// ============================================================================

/// f64 精度的值缓存上下文 / Value cache context with f64 precision
pub type F64ValueCacheContext<T> = LazyValueCacheContext<f64, T>;

/// BigDecimal 精度的值缓存上下文 / Value cache context with BigDecimal precision
#[cfg(feature = "bigdecimal")]
pub type BigDecimalValueCacheContext<T> = LazyValueCacheContext<BigDecimal, T>;

// ============================================================================
// 范围缓存 / Range Cache
// ============================================================================

/// 范围缓存 Key / Range Cache Key
///
/// 用于缓存中间符号或表达式的取值范围。
/// Used for caching ranges of intermediate symbols or expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RangeCacheKey {
    /// 对象 ID（表达式地址或符号 ID）/ Object ID (expression address or symbol ID)
    pub object_id: u64,
}

impl RangeCacheKey {
    /// 从可缓存对象创建 / Create from cacheable object
    pub fn from_cacheable<T: Cacheable>(object: &T) -> Self {
        Self { object_id: object.cache_key().value }
    }
    
    /// 从中间符号创建 / Create from intermediate symbol
    pub fn from_symbol(identifier: u64) -> Self {
        Self { object_id: identifier }
    }
}

/// 范围缓存上下文 trait / Range Cache Context Trait
///
/// 生命周期与 MetaModel 一致，延迟初始化直到 TokenTable 可用。
/// Lifecycle matches MetaModel, lazy initialization until TokenTable is available.
///
/// # 设计说明 / Design Notes
///
/// - 支持 Big-M 推导、范围传播、约束紧化中的范围复用
/// - 与 `ValueCacheContextTrait` 的失效策略保持一致
/// - 缓存值使用 `VariableRange<V>`
///
/// - Supports range reuse for Big-M derivation, bound propagation, and constraint tightening
/// - Keeps invalidation strategy consistent with `ValueCacheContextTrait`
/// - Uses `VariableRange<V>` as cache value
pub trait RangeCacheContextTrait<V>: Send + Sync {
    /// 获取 TokenTable 引用 / Get TokenTable reference
    fn token_table(&self) -> &dyn TokenList<V>;
    
    /// 缓存映射 / Cache map
    fn cache(&self) -> &HashMap<RangeCacheKey, VariableRange<V>>;
    fn cache_mut(&mut self) -> &mut HashMap<RangeCacheKey, VariableRange<V>>;
    
    /// 获取缓存值 / Get cached range
    fn get(&self, key: RangeCacheKey) -> Option<&VariableRange<V>> {
        self.cache().get(&key)
    }
    
    /// 设置缓存值 / Set cached range
    fn set(&mut self, key: RangeCacheKey, value: VariableRange<V>) {
        self.cache_mut().insert(key, value);
    }
    
    /// 获取或计算 / Get or compute
    fn get_or_compute<F>(&mut self, key: RangeCacheKey, f: F) -> &VariableRange<V>
    where
        F: FnOnce() -> VariableRange<V>,
    {
        self.cache_mut().entry(key).or_insert_with(f)
    }
    
    /// 清除所有缓存 / Clear all cache
    fn clear(&mut self) {
        self.cache_mut().clear();
    }
    
    /// 清除指定对象的缓存 / Clear cache by object
    fn clear_object(&mut self, key: RangeCacheKey) -> bool {
        self.cache_mut().remove(&key).is_some()
    }
}

// ============================================================================
// 懒加载范围缓存上下文 / Lazy Range Cache Context
// ============================================================================

/// 懒加载的范围缓存上下文 / Lazy Range Cache Context
pub struct LazyRangeCacheContext<V, T: TokenList<V>> {
    /// TokenTable 引用（延迟初始化）/ TokenTable reference (lazy initialization)
    token_table: OnceCell<Arc<T>>,
    /// 缓存映射 / Cache map
    cache: HashMap<RangeCacheKey, VariableRange<V>>,
    _value: PhantomData<V>,
}

impl<V, T: TokenList<V>> LazyRangeCacheContext<V, T> {
    /// 创建未初始化的上下文 / Create uninitialized context
    pub fn new() -> Self {
        Self {
            token_table: OnceCell::new(),
            cache: HashMap::new(),
            _value: PhantomData,
        }
    }
    
    /// 初始化（设置 token_table 引用）/ Initialize (set token_table reference)
    pub fn init(&self, token_table: Arc<T>) {
        let _ = self.token_table.set(token_table);
    }
    
    /// 检查是否已初始化 / Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.token_table.get().is_some()
    }
}

impl<V: Clone + Debug + Send + Sync + 'static, T: TokenList<V>> RangeCacheContextTrait<V>
    for LazyRangeCacheContext<V, T>
{
    fn token_table(&self) -> &dyn TokenList<V> {
        self.token_table.get()
            .expect("RangeCacheContext not initialized. Call init() first.")
            .as_ref()
    }
    
    fn cache(&self) -> &HashMap<RangeCacheKey, VariableRange<V>> {
        &self.cache
    }
    
    fn cache_mut(&mut self) -> &mut HashMap<RangeCacheKey, VariableRange<V>> {
        &mut self.cache
    }
}

impl<V, T: TokenList<V>> Default for LazyRangeCacheContext<V, T> {
    fn default() -> Self {
        Self::new()
    }
}

/// f64 精度的范围缓存上下文 / Range cache context with f64 precision
pub type F64RangeCacheContext<T> = LazyRangeCacheContext<f64, T>;

/// BigDecimal 精度的范围缓存上下文 / Range cache context with BigDecimal precision
#[cfg(feature = "bigdecimal")]
pub type BigDecimalRangeCacheContext<T> = LazyRangeCacheContext<BigDecimal, T>;

// ============================================================================
// 平展上下文 trait / Flatten Context Trait
// ============================================================================

/// 平展上下文 trait / Flatten Context Trait
/// 
/// 根据多项式类型区分不同的上下文，携带 TokenTable 引用用于查询注册的变量。
/// Distinguishes different contexts by polynomial type, carries TokenTable reference for querying registered variables.
/// 
/// # 类型参数 / Type Parameters
/// - `V`: 统一值类型 / Unified value type
pub trait FlattenContextTrait<V>: Send + Sync {
    /// 多项式类型 / Polynomial type
    type Polynomial;
    /// 单项式类型 / Monomial type
    type Monomial;
    
    /// 获取 TokenTable 引用 / Get TokenTable reference
    fn token_table(&self) -> &dyn TokenList<V>;
    
    /// 单项式缓存 / Monomial cache
    fn monomial_cache(&self) -> &HashMap<u64, FlattenedMonomial<Self::Monomial>>;
    fn monomial_cache_mut(&mut self) -> &mut HashMap<u64, FlattenedMonomial<Self::Monomial>>;
    
    /// 多项式缓存 / Polynomial cache
    fn polynomial_cache(&self) -> &HashMap<u64, FlattenedPolynomial<Self::Polynomial>>;
    fn polynomial_cache_mut(&mut self) -> &mut HashMap<u64, FlattenedPolynomial<Self::Polynomial>>;
    
    /// 中间符号缓存 / Intermediate symbol cache
    fn symbol_cache(&self) -> &HashMap<u64, FlattenedSymbol<Self::Polynomial>>;
    fn symbol_cache_mut(&mut self) -> &mut HashMap<u64, FlattenedSymbol<Self::Polynomial>>;
    
    /// 检查变量是否已注册 / Check if variable is registered
    fn is_registered(&self, var_id: VariableId) -> bool {
        self.token_table().find_by_id(var_id).is_some()
    }
    
    /// 通过 ID 查找 Token / Find token by ID
    fn find_token(&self, var_id: VariableId) -> Option<&Token<V>> {
        self.token_table().find_by_id(var_id)
    }
    
    /// 清除所有缓存 / Clear all caches
    fn clear(&mut self) {
        self.monomial_cache_mut().clear();
        self.polynomial_cache_mut().clear();
        self.symbol_cache_mut().clear();
    }
    
    /// 清除指定单项式的缓存 / Clear cache for specific monomial
    fn clear_monomial(&mut self, id: u64) -> bool {
        self.monomial_cache_mut().remove(&id).is_some()
    }
    
    /// 清除指定多项式的缓存 / Clear cache for specific polynomial
    fn clear_polynomial(&mut self, id: u64) -> bool {
        self.polynomial_cache_mut().remove(&id).is_some()
    }
    
    /// 清除指定中间符号的缓存 / Clear cache for specific intermediate symbol
    fn clear_symbol(&mut self, id: u64) -> bool {
        self.symbol_cache_mut().remove(&id).is_some()
    }
    
    /// 批量清除指定项的缓存 / Clear cache for multiple items
    fn clear_items(&mut self, ids: &[u64]) {
        for id in ids {
            self.monomial_cache_mut().remove(id);
            self.polynomial_cache_mut().remove(id);
            self.symbol_cache_mut().remove(id);
        }
    }
}

// ============================================================================
// 懒加载平展上下文 / Lazy Flatten Context
// ============================================================================

/// 懒加载的线性平展上下文 / Lazy Linear Flatten Context
/// 
/// 生命周期与 MetaModel 一致，但延迟初始化直到 token_table 可用。
/// Lifecycle matches MetaModel, but lazy initialization until token_table is available.
pub struct LazyLinearFlattenContext<V, T: TokenList<V>> {
    /// TokenTable 引用（延迟初始化）/ TokenTable reference (lazy initialization)
    token_table: OnceCell<Arc<T>>,
    /// 单项式缓存 / Monomial cache
    monomial_cache: HashMap<u64, FlattenedMonomial<LinearMonomial<V>>>,
    /// 多项式缓存 / Polynomial cache
    polynomial_cache: HashMap<u64, FlattenedPolynomial<Linear<V>>>,
    /// 中间符号缓存 / Intermediate symbol cache
    symbol_cache: HashMap<u64, FlattenedSymbol<Linear<V>>>,
    _value: PhantomData<V>,
}

impl<V, T: TokenList<V>> LazyLinearFlattenContext<V, T> {
    /// 创建未初始化的上下文 / Create uninitialized context
    pub fn new() -> Self {
        Self {
            token_table: OnceCell::new(),
            monomial_cache: HashMap::new(),
            polynomial_cache: HashMap::new(),
            symbol_cache: HashMap::new(),
            _value: PhantomData,
        }
    }
    
    /// 初始化（设置 token_table 引用）/ Initialize (set token_table reference)
    /// 
    /// 只能调用一次，后续调用将被忽略。
    /// Can only be called once, subsequent calls will be ignored.
    pub fn init(&self, token_table: Arc<T>) {
        let _ = self.token_table.set(token_table);
    }
    
    /// 检查是否已初始化 / Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.token_table.get().is_some()
    }
    
    /// 获取 token_table（需确保已初始化）/ Get token_table (must be initialized)
    pub fn token_table(&self) -> Option<&T> {
        self.token_table.get().map(|arc| arc.as_ref())
    }
}

impl<V: Clone + Debug + Send + Sync + 'static, T: TokenList<V>> FlattenContextTrait<V> 
    for LazyLinearFlattenContext<V, T> 
{
    type Polynomial = Linear<V>;
    type Monomial = LinearMonomial<V>;
    
    fn token_table(&self) -> &dyn TokenList<V> {
        self.token_table.get()
            .expect("FlattenContext not initialized. Call init() first.")
            .as_ref()
    }
    
    fn monomial_cache(&self) -> &HashMap<u64, FlattenedMonomial<Self::Monomial>> {
        &self.monomial_cache
    }
    fn monomial_cache_mut(&mut self) -> &mut HashMap<u64, FlattenedMonomial<Self::Monomial>> {
        &mut self.monomial_cache
    }
    fn polynomial_cache(&self) -> &HashMap<u64, FlattenedPolynomial<Self::Polynomial>> {
        &self.polynomial_cache
    }
    fn polynomial_cache_mut(&mut self) -> &mut HashMap<u64, FlattenedPolynomial<Self::Polynomial>> {
        &mut self.polynomial_cache
    }
    fn symbol_cache(&self) -> &HashMap<u64, FlattenedSymbol<Self::Polynomial>> {
        &self.symbol_cache
    }
    fn symbol_cache_mut(&mut self) -> &mut HashMap<u64, FlattenedSymbol<Self::Polynomial>> {
        &mut self.symbol_cache
    }
}

// ============================================================================
// Flattenable Trait
// ============================================================================

/// 可标识的 trait / Identified trait
pub trait Identified {
    /// 获取唯一标识符 / Get unique identifier
    fn identifier(&self) -> u64;
}

/// 可平展的 trait / Flattenable trait
/// 
/// 平展操作将中间符号展开为纯变量表达式。
/// Flatten operation expands intermediate symbols into pure variable expressions.
/// 
/// 注意：函数中间符号的辅助变量和约束注册由 `FunctionSymbol` trait 处理，
/// 不在 `Flattenable` 中进行回调。
/// 
/// Note: Auxiliary variable and constraint registration for function symbols
/// is handled by `FunctionSymbol` trait, not via callbacks in `Flattenable`.
pub trait Flattenable<C: FlattenContextTrait<V>, V> {
    /// 平展为纯变量表达式 / Flatten to pure variable expression
    fn flatten(&self, ctx: &mut C) -> C::Polynomial;
    
    /// 带缓存的平展 / Flatten with cache
    fn flatten_cached(&self, ctx: &mut C) -> C::Polynomial
    where
        Self: Identified,
    {
        let id = self.identifier();
        if let Some(cached) = ctx.polynomial_cache().get(&id) {
            return cached.polynomial.clone();
        }
        let result = self.flatten(ctx);
        ctx.polynomial_cache_mut().insert(id, FlattenedPolynomial { polynomial: result.clone() });
        result
    }
}
```

**三种 MetaModel 对应三种 Context**:

| MetaModel 类型 | Context 类型 | Polynomial | Monomial |
|---------------|-------------|------------|----------|
| `LinearMetaModel<V>` | `LazyLinearFlattenContext<V, T>` | `Linear<V>` | `LinearMonomial<V>` |
| `QuadraticMetaModel<V>` | `LazyQuadraticFlattenContext<V, T>` | `Quadratic<V>` | `QuadraticMonomial<V>` |
| `CanonicalMetaModel<V>` | `LazyCanonicalFlattenContext<V, T>` | `Canonical<V>` | `CanonicalMonomial<V>` |

注：`ValueCacheContext` 与 `RangeCacheContext` 不区分多项式阶次，按统一值类型 `V` 管理。
Note: `ValueCacheContext` and `RangeCacheContext` are polynomial-order agnostic and managed by unified value type `V`.

**初始化流程**:

```
MetaModel::new()
    ↓
tokens 创建
    ↓
flatten_ctx / value_cache_ctx / range_cache_ctx 未初始化
    ↓
首次调用 flatten_*() / eval_*() / range_*()
    ↓
ensure_flatten_context()
ensure_value_cache_context()
ensure_range_cache_context()
    ↓
flatten_ctx.init(tokens.clone())
value_cache_ctx.init(tokens.clone())
range_cache_ctx.init(tokens.clone())
    ↓
三类 context 可用
```

---

#### 3.5 中间符号系统 (Intermediate Symbol System)

**目录结构**:
```
src/symbol/
├── mod.rs
├── intermediate_symbol.rs    # IntermediateSymbol trait
├── expression_symbol.rs      # ExpressionSymbol
├── function_symbol.rs        # FunctionSymbol trait
└── functions/
    ├── mod.rs
    ├── min_max.rs            # Min, Max, MaxMin, MinMax
    ├── abs.rs                # Abs
    ├── floor_ceiling.rs      # Floor, Ceiling
    ├── if_then.rs            # If, IfThen, IfIn
    ├── piecewise.rs          # Univariate/BivariateLinearPiecewise
    ├── logic.rs              # And, Or, Not, Xor
    ├── sigmoid.rs            # Sigmoid
    ├── slack.rs              # Slack, SlackRange
    └── masking.rs            # Masking, MaskingRange
```

**核心 Trait**:

```rust
/// 中间符号标识符 / Intermediate Symbol Identifier
/// 
/// 用于唯一标识中间符号。
/// Used to uniquely identify intermediate symbols.
/// 
/// 自动实现 `ospf_rust_math::symbol::SymbolId`。
/// Automatically implements `ospf_rust_math::symbol::SymbolId`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IntermediateSymbolId {
    /// 符号唯一编号 / Symbol unique number
    pub id: u64,
    /// 符号名称 / Symbol name
    pub name: String,
}

/// 中间符号 / Intermediate Symbol
/// 
/// 继承自 `ospf_rust_math::symbol::Symbol`，表示一个可能需要展开的复合表达式。
/// Inherits from `ospf_rust_math::symbol::Symbol`, represents a composite expression that may need to be expanded.
pub trait IntermediateSymbol<V = f64>: Symbol<Id = IntermediateSymbolId> + Send + Sync
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 符号类别 (Linear, Quadratic, etc.) / Symbol category
    fn category(&self) -> Category;
    
    /// 操作类别 / Operation category
    fn operation_category(&self) -> Category {
        self.category()
    }
    
    /// 是否已缓存 / Whether cached
    fn cached(&self) -> bool;
    
    /// 父符号 / Parent symbol
    fn parent(&self) -> Option<&dyn IntermediateSymbol<V>> {
        None
    }
    
    /// 依赖的其他符号 / Dependencies on other symbols
    fn dependencies(&self) -> HashSet<Arc<dyn IntermediateSymbol<V>>>;
    
    /// 刷新缓存 / Flush cache
    fn flush(&self, force: bool);
    
    /// 准备求值 / Prepare for evaluation
    fn prepare(&self, values: &HashMap<usize, V>) -> Option<V>;

    /// 带缓存上下文求值 / Evaluate with value cache context
    fn evaluate_with_ctx(
        &self,
        values: &HashMap<usize, V>,
        ctx: &mut dyn ValueCacheContextTrait<V>,
    ) -> Option<V>;

    /// 计算符号范围 / Compute symbol range
    fn range(&self) -> Option<VariableRange<V>>;

    /// 带缓存上下文求范围 / Compute range with range cache context
    fn range_with_ctx(
        &self,
        ctx: &mut dyn RangeCacheContextTrait<V>,
    ) -> Option<VariableRange<V>>;
    
    /// 转换为原始字符串 / Convert to raw string
    fn to_raw_string(&self, unfold: u64) -> String;
}

/// 线性中间符号 / Linear Intermediate Symbol
pub trait LinearIntermediateSymbol<V = f64>: IntermediateSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 线性单元格列表 / Linear cell list
    fn cells(&self) -> Vec<LinearMonomialCell>;
    
    /// 转换为线性多项式 / Convert to linear polynomial
    fn to_linear_polynomial(&self) -> Linear<V>;
    
    /// 转换为二次多项式 / Convert to quadratic polynomial
    fn to_quadratic_polynomial(&self) -> Quadratic<V>;
}

/// 二次中间符号 / Quadratic Intermediate Symbol
pub trait QuadraticIntermediateSymbol<V = f64>: IntermediateSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 二次单元格列表 / Quadratic cell list
    fn cells(&self) -> Vec<QuadraticMonomialCell>;
    
    /// 转换为二次多项式 / Convert to quadratic polynomial
    fn to_quadratic_polynomial(&self) -> Quadratic<V>;
}

/// 函数符号 / Function Symbol
/// 
/// 需要向模型注册额外变量和约束的符号。
/// Symbols that need to register additional variables and constraints to the model.
pub trait FunctionSymbol<V = f64>: IntermediateSymbol<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    /// 注册到 Token 集合 / Register to token collection
    fn register(&self, tokens: &mut dyn AddableTokenCollection) -> Result<()>;
    
    /// 注册到模型 / Register to model
    fn register_to_model(&self, model: &mut dyn MechanismModel) -> Result<()>;
    
    /// 计算值 / Calculate value
    fn calculate_value(&self, token_table: &dyn TokenTable<V>, zero_if_none: bool) -> Option<V>;
}
```

**函数符号示例 - Min**:

```rust
/// 最小值函数 / Minimum Function
/// 
/// 表示 min(p1, p2, ..., pn)，其中 pi 是线性多项式。
/// Represents min(p1, p2, ..., pn) where pi are linear polynomials.
pub struct MinFunction {
    id: u64,
    name: String,
    polynomials: Vec<Linear>,
    maxmin_var: VariableItem,  // 辅助变量
    u_vars: Option<Vec<VariableItem>>,  // 二进制辅助变量（精确模式）
    exact: bool,
}

impl FunctionSymbol for MinFunction {
    fn register(&self, tokens: &mut dyn AddableTokenCollection) -> Result<()> {
        tokens.add_variable(self.maxmin_var.clone())?;
        if let Some(ref u_vars) = self.u_vars {
            tokens.add_variables(u_vars.iter().cloned())?;
        }
        Ok(())
    }
    
    fn register_to_model(&self, model: &mut dyn MechanismModel) -> Result<()> {
        // 添加约束: maxmin <= pi for all i
        for (i, poly) in self.polynomials.iter().enumerate() {
            model.add_constraint(self.maxmin_var.clone() <= poly.clone())?;
        }
        
        // 精确模式：添加额外约束
        if self.exact {
            // maxmin >= pi - M * (1 - ui)
            // sum(ui) = 1
            // ...
        }
        
        Ok(())
    }
}
```

**支持的函数符号**:

| 符号 | 描述 | 线性/二次 |
|------|------|----------|
| `Min` | 最小值 | 线性 |
| `Max` | 最大值 | 线性 |
| `MaxMin` | 最大最小值（精确） | 线性 |
| `MinMax` | 最小最大值（精确） | 线性 |
| `Abs` | 绝对值 | 线性 |
| `Floor` | 向下取整 | 线性 |
| `Ceiling` | 向上取整 | 线性 |
| `If` | 条件判断 | 线性 |
| `IfThen` | 条件选择 | 线性 |
| `UnivariateLinearPiecewise` | 一维分段线性 | 线性 |
| `BivariateLinearPiecewise` | 二维分段线性 | 线性 |
| `And` | 逻辑与 | 线性 |
| `Or` | 逻辑或 | 线性 |
| `Not` | 逻辑非 | 线性 |
| `Xor` | 逻辑异或 | 线性 |
| `Sigmoid` | Sigmoid 函数 | 线性 |
| `Slack` | 松弛变量 | 线性 |
| `Masking` | 掩码 | 线性 |

---

#### 3.4 约束系统 (Constraint System)

**目录结构**:
```
src/constraint/
├── mod.rs
├── constraint.rs         # Constraint
├── meta_constraint.rs    # MetaConstraint
└── constraint_group.rs   # MetaConstraintGroup
```

**核心类型**:

```rust
/// 约束 / Constraint
/// 
/// 单个约束条件，包含不等式和元数据。
/// A single constraint, containing inequality and metadata.
#[derive(Debug, Clone)]
pub struct Constraint<P: Polynomial> {
    /// 不等式 / Inequality
    pub inequality: Inequality<P>,
    /// 约束名称 / Constraint name
    pub name: String,
    /// 来源符号 / Source symbol
    pub from: Option<Arc<dyn IntermediateSymbol>>,
}

/// 元约束 / Meta Constraint
/// 
/// 支持延迟求值和分组的约束。
/// Constraint supporting lazy evaluation and grouping.
#[derive(Debug, Clone)]
pub struct MetaConstraint<I: InequalityTrait> {
    /// 约束 / Constraint
    pub constraint: I,
    /// 约束组 / Constraint group
    pub group: Option<Arc<MetaConstraintGroup>>,
    /// 是否延迟求值 / Whether lazy evaluation
    pub lazy: bool,
    /// 额外参数 / Additional arguments
    pub args: Option<Arc<dyn Any>>,
}

/// 约束组 / Constraint Group
/// 
/// 用于批量管理约束。
/// For batch management of constraints.
#[derive(Debug)]
pub struct MetaConstraintGroup {
    pub id: u64,
    pub name: String,
}
```

---

#### 3.5 模型系统 (Model System)

**目录结构**:
```
src/model/
├── mod.rs
├── basic_model.rs        # BasicModel (基本模型层)
├── meta_model.rs         # MetaModel 主实现（用户建模层）
├── mechanism/
│   ├── mod.rs
│   ├── basic_mechanism_model.rs  # BasicMechanismModel (基本机理模型层)
│   ├── mechanism_model.rs        # MechanismModel
│   └── meta_model.rs             # 兼容层：re-export 到 model::meta_model
├── object.rs             # Object (目标函数)
└── configuration.rs      # Configuration
```

**核心类型**:

```rust
// ============================================================================
// 基本模型层 / Basic Model Layer
// ============================================================================

/// 基本模型 / Basic Model
/// 
/// 只包含变量和约束的基本模型层，用于支持：
/// - 对偶模型创建
/// - 多目标模型组合
/// - 模型复用
/// 
/// Basic model layer containing only variables and constraints, supporting:
/// - Dual model creation
/// - Multi-objective model composition
/// - Model reuse
/// 
/// # 设计说明 / Design Notes
/// 
/// 基本模型不包含目标函数，允许：
/// 1. 同一组约束用于不同的优化目标
/// 2. 创建对偶模型时复用约束结构
/// 3. 多目标优化时组合多个基本模型
/// 
/// Basic model doesn't contain objective, allowing:
/// 1. Same constraint set for different optimization objectives
/// 2. Reusing constraint structure when creating dual models
/// 3. Combining multiple basic models in multi-objective optimization
pub struct BasicModel<V> {
    /// 模型名称 / Model name
    pub name: String,
    /// Token 表 / Token table
    tokens: Arc<RwLock<Box<dyn TokenTable<V>>>>,
    /// 约束列表 / Constraints
    constraints: Vec<MetaConstraint<LinearInequality<V>>>,
    /// 平展上下文 / Flatten context
    flatten_ctx: LazyLinearFlattenContext<V, Box<dyn TokenTable<V>>>,
    /// 值缓存上下文 / Value cache context
    value_cache_ctx: LazyValueCacheContext<V, Box<dyn TokenTable<V>>>,
    /// 范围缓存上下文 / Range cache context
    range_cache_ctx: LazyRangeCacheContext<V, Box<dyn TokenTable<V>>>,
    /// 配置 / Configuration
    config: BasicModelConfiguration,
}

impl<V: Clone + Debug + Send + Sync + 'static> BasicModel<V> {
    /// 创建新模型 / Create new model
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tokens: Arc::new(RwLock::new(Box::new(TokenTableImpl::new()))),
            constraints: Vec::new(),
            flatten_ctx: LazyLinearFlattenContext::new(),
            value_cache_ctx: LazyValueCacheContext::new(),
            range_cache_ctx: LazyRangeCacheContext::new(),
            config: BasicModelConfiguration::default(),
        }
    }
    
    /// 添加变量 / Add variable
    pub fn add_variable(&mut self, variable: impl Into<AnyVariable<V>>) -> Result<()>;
    
    /// 添加变量组 / Add variables
    pub fn add_variables<I: IntoIterator<Item = impl Into<AnyVariable<V>>>>(
        &mut self, 
        variables: I
    ) -> Result<()>;
    
    /// 添加中间符号 / Add intermediate symbol
    pub fn add_symbol(&mut self, symbol: Arc<dyn IntermediateSymbol>) -> Result<()>;
    
    /// 添加约束 / Add constraint
    pub fn add_constraint(&mut self, constraint: LinearInequality<V>) -> Result<()>;
    
    /// 设置解 / Set solution
    pub fn set_solution(&mut self, solution: &HashMap<VariableId, V>);
    
    /// 获取所有变量 / Get all variables
    pub fn variables(&self) -> Vec<&AnyVariable<V>>;
    
    /// 获取所有约束 / Get all constraints
    pub fn constraints(&self) -> &[MetaConstraint<LinearInequality<V>>];
    
    /// 克隆约束结构（用于对偶模型）/ Clone constraint structure (for dual model)
    pub fn clone_constraints(&self) -> Vec<MetaConstraint<LinearInequality<V>>>;
    
    /// 合并另一个基本模型的约束 / Merge constraints from another basic model
    pub fn merge_constraints(&mut self, other: &BasicModel<V>) -> Result<()>;
}

// ============================================================================
// 元模型（带目标函数）/ Meta Model (with Objective)
// ============================================================================

/// 元模型 / Meta Model
/// 
/// 继承基本模型，添加目标函数支持。
/// Inherits basic model, adding objective function support.
/// 
/// # 设计说明 / Design Notes
/// 
/// `MetaModel` 组合 `BasicModel`，添加目标函数相关功能。
/// 这种设计允许：
/// 1. 从 `MetaModel` 提取 `BasicModel` 用于对偶转换
/// 2. 多个 `MetaModel` 共享相同的 `BasicModel`
/// 3. 统一管理平展、求值、求范围三个缓存上下文
/// 
/// `MetaModel` composes `BasicModel`, adding objective-related functionality.
/// This design allows:
/// 1. Extracting `BasicModel` from `MetaModel` for dual transformation
/// 2. Multiple `MetaModel`s sharing the same `BasicModel`
/// 3. Unified management of flatten/value/range cache contexts
pub struct MetaModel<V> {
    /// 基本模型 / Basic model
    pub basic: BasicModel<V>,
    /// 目标方向 / Objective direction
    pub objective_category: ObjectiveCategory,
    /// 子目标列表 / Sub-objectives
    sub_objectives: Vec<SubObjective<V>>,
}

impl<V: Clone + Debug + Send + Sync + 'static> MetaModel<V> {
    /// 创建新模型 / Create new model
    pub fn new(name: &str) -> Self {
        Self {
            basic: BasicModel::new(name),
            objective_category: ObjectiveCategory::Minimum,
            sub_objectives: Vec::new(),
        }
    }
    
    /// 从基本模型创建 / Create from basic model
    pub fn from_basic(basic: BasicModel<V>) -> Self {
        Self {
            basic,
            objective_category: ObjectiveCategory::Minimum,
            sub_objectives: Vec::new(),
        }
    }
    
    /// 确保平展上下文已初始化 / Ensure flatten context is initialized
    pub fn ensure_flatten_context(&mut self);
    
    /// 确保求值缓存上下文已初始化 / Ensure value cache context is initialized
    pub fn ensure_value_cache_context(&mut self);
    
    /// 确保求范围缓存上下文已初始化 / Ensure range cache context is initialized
    pub fn ensure_range_cache_context(&mut self);
    
    /// 添加目标函数 / Add objective
    pub fn add_objective(&mut self, objective: SubObjective<V>) -> Result<()>;
    
    /// 获取基本模型引用（用于对偶转换）/ Get basic model reference (for dual transformation)
    pub fn as_basic(&self) -> &BasicModel<V> {
        &self.basic
    }
    
    /// 提取基本模型（消耗 self）/ Extract basic model (consumes self)
    pub fn into_basic(self) -> BasicModel<V> {
        self.basic
    }
    
    /// 导出模型 / Export model
    pub async fn export(&self, path: &Path) -> Result<()>;
}

// 委托方法 / Delegate methods
impl<V: Clone + Debug + Send + Sync + 'static> std::ops::Deref for MetaModel<V> {
    type Target = BasicModel<V>;
    fn deref(&self) -> &Self::Target { &self.basic }
}

impl<V: Clone + Debug + Send + Sync + 'static> std::ops::DerefMut for MetaModel<V> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.basic }
}

// ============================================================================
// 基本机理模型层 / Basic Mechanism Model Layer
// ============================================================================

/// 基本机理模型 / Basic Mechanism Model
/// 
/// 只包含展开后的变量和约束，不包含目标函数。
/// Contains only expanded variables and constraints, without objective.
/// 
/// # 用途 / Use Cases
/// 
/// 1. **对偶模型**: 从基本机理模型生成对偶问题
/// 2. **多目标优化**: 组合多个基本机理模型
/// 3. **约束共享**: 不同目标函数共享相同约束集
/// 
/// 1. **Dual Model**: Generate dual problem from basic mechanism model
/// 2. **Multi-objective Optimization**: Compose multiple basic mechanism models
/// 3. **Constraint Sharing**: Different objectives sharing same constraint set
#[derive(Debug, Clone)]
pub struct BasicMechanismModel<V> {
    /// 模型名称 / Model name
    pub name: String,
    /// Token 列表 / Token list
    tokens: Vec<Token<V>>,
    /// 约束列表 / Constraints
    constraints: Vec<Constraint<Linear<V>>>,
    /// Token ID 到索引的映射 / Token ID to index mapping
    token_index: HashMap<VariableId, usize>,
}

impl<V: Clone + Debug + Send + Sync + 'static> BasicMechanismModel<V> {
    /// 创建空模型 / Create empty model
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tokens: Vec::new(),
            constraints: Vec::new(),
            token_index: HashMap::new(),
        }
    }
    
    /// 从基本模型创建 / Create from basic model
    pub async fn from_basic_model(basic: &BasicModel<V>) -> Result<Self>;
    
    /// 添加 Token / Add token
    pub fn add_token(&mut self, token: Token<V>) -> usize {
        let idx = self.tokens.len();
        self.token_index.insert(token.variable.id(), idx);
        self.tokens.push(token);
        idx
    }
    
    /// 添加约束 / Add constraint
    pub fn add_constraint(&mut self, constraint: Constraint<Linear<V>>) {
        self.constraints.push(constraint);
    }
    
    /// 获取所有 Token / Get all tokens
    pub fn tokens(&self) -> &[Token<V>] {
        &self.tokens
    }
    
    /// 获取所有约束 / Get all constraints
    pub fn constraints(&self) -> &[Constraint<Linear<V>>] {
        &self.constraints
    }
    
    /// 通过 ID 查找 Token / Find token by ID
    pub fn find_token(&self, id: VariableId) -> Option<&Token<V>> {
        self.token_index.get(&id).map(|&idx| &self.tokens[idx])
    }
    
    /// 克隆约束结构 / Clone constraint structure
    pub fn clone_constraints(&self) -> Vec<Constraint<Linear<V>>> {
        self.constraints.clone()
    }
    
    /// 合并另一个基本机理模型 / Merge another basic mechanism model
    pub fn merge(&mut self, other: &BasicMechanismModel<V>) -> Result<()> {
        for token in &other.tokens {
            self.add_token(token.clone());
        }
        for constraint in &other.constraints {
            self.add_constraint(constraint.clone());
        }
        Ok(())
    }
    
    /// 转换为标准形式（用于求解器）/ Convert to standard form (for solver)
    pub fn to_standard_form(&self) -> (SparseMatrix<V>, Vec<V>, Vec<V>, Vec<V>);
}

// ============================================================================
// 机理模型（带目标函数）/ Mechanism Model (with Objective)
// ============================================================================

/// 机理模型 / Mechanism Model
/// 
/// 继承基本机理模型，添加目标函数支持。
/// Inherits basic mechanism model, adding objective function support.
#[derive(Debug, Clone)]
pub struct MechanismModel<V> {
    /// 基本机理模型 / Basic mechanism model
    pub basic: BasicMechanismModel<V>,
    /// 目标函数 / Objective function
    objective: Objective<V>,
}

impl<V: Clone + Debug + Send + Sync + 'static> MechanismModel<V> {
    /// 从元模型创建 / Create from meta model
    pub async fn from_meta_model(meta: &MetaModel<V>) -> Result<Self> {
        let basic = BasicMechanismModel::from_basic_model(&meta.basic).await?;
        Ok(Self {
            basic,
            objective: Objective::from_meta_model(meta),
        })
    }
    
    /// 从基本机理模型创建 / Create from basic mechanism model
    pub fn from_basic(basic: BasicMechanismModel<V>) -> Self {
        Self {
            basic,
            objective: Objective::default(),
        }
    }
    
    /// 设置目标函数 / Set objective
    pub fn set_objective(&mut self, objective: Objective<V>) {
        self.objective = objective;
    }
    
    /// 获取目标函数 / Get objective
    pub fn objective(&self) -> &Objective<V> {
        &self.objective
    }
    
    /// 生成最优割 / Generate optimal cut
    pub fn generate_optimal_cut(&self) -> Vec<LinearInequality<V>>;
    
    /// 生成可行割 / Generate feasible cut
    pub fn generate_feasible_cut(&self) -> Vec<LinearInequality<V>>;
    
    /// 获取基本模型引用（用于对偶转换）/ Get basic model reference (for dual transformation)
    pub fn as_basic(&self) -> &BasicMechanismModel<V> {
        &self.basic
    }
    
    /// 提取基本模型（消耗 self）/ Extract basic model (consumes self)
    pub fn into_basic(self) -> BasicMechanismModel<V> {
        self.basic
    }
}

// 委托方法 / Delegate methods
impl<V: Clone + Debug + Send + Sync + 'static> std::ops::Deref for MechanismModel<V> {
    type Target = BasicMechanismModel<V>;
    fn deref(&self) -> &Self::Target { &self.basic }
}

impl<V: Clone + Debug + Send + Sync + 'static> std::ops::DerefMut for MechanismModel<V> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.basic }
}

/// 目标函数 / Objective
#[derive(Debug, Clone)]
pub struct Objective {
    /// 目标方向 / Objective direction
    pub category: ObjectiveCategory,
    /// 子目标列表 / Sub-objectives
    pub sub_objectives: Vec<SubObjective>,
}

/// 子目标 / Sub-objective
#[derive(Debug, Clone)]
pub struct SubObjective {
    /// 目标方向 / Objective direction
    pub category: ObjectiveCategory,
    /// 多项式 / Polynomial
    pub polynomial: Linear,
    /// 名称 / Name
    pub name: String,
}

/// 目标方向 / Objective Category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveCategory {
    /// 最小化 / Minimize
    Minimum,
    /// 最大值 / Maximize
    Maximum,
}
```

---

#### 3.6 中间模型层 (Intermediate Model)

**目录结构**:
```
src/intermediate_model/
├── mod.rs
├── basic_linear_triad_model.rs   # BasicLinearTriadModel (基本线性三角模型)
├── linear_triad_model.rs         # LinearTriadModel (线性三角模型)
├── basic_quadratic_tetrad_model.rs  # BasicQuadraticTetradModel (基本二次四角模型)
└── quadratic_tetrad_model.rs     # QuadraticTetradModel (二次四角模型)
```

**核心类型**:

```rust
// ============================================================================
// 基本线性三角模型 / Basic Linear Triad Model
// ============================================================================

/// 基本线性三角模型 / Basic Linear Triad Model
/// 
/// 只包含变量和约束的标准形式，不包含目标函数。
/// Standard form with only variables and constraints, without objective.
/// 
/// # 用途 / Use Cases
/// 
/// 1. **对偶模型**: 从基本线性三角模型生成对偶问题
/// 2. **多目标优化**: 组合多个目标函数与相同约束集
/// 3. **约束共享**: 不同目标函数共享相同约束集
/// 
/// 1. **Dual Model**: Generate dual problem from basic linear triad model
/// 2. **Multi-objective Optimization**: Compose multiple objectives with same constraint set
/// 3. **Constraint Sharing**: Different objectives sharing same constraint set
/// 
/// 标准形式: Ax ≤ b, x ∈ [lb, ub]
/// Standard form: Ax ≤ b, x ∈ [lb, ub]
#[derive(Debug, Clone)]
pub struct BasicLinearTriadModel<V> {
    /// 变量列表 / Variable list
    pub variables: Vec<Token<V>>,
    /// 约束矩阵 / Constraint matrix
    pub A: SparseMatrix<V>,
    /// 约束右侧 / Right-hand side
    pub b: Vec<V>,
    /// 变量下界 / Lower bounds
    pub lb: Vec<V>,
    /// 变量上界 / Upper bounds
    pub ub: Vec<V>,
    /// 变量类型 / Variable types
    pub var_types: Vec<VariableType>,
    /// Token ID 到索引的映射 / Token ID to index mapping
    token_index: HashMap<VariableId, usize>,
}

impl<V: Clone + Debug + Send + Sync + 'static> BasicLinearTriadModel<V> {
    /// 创建空模型 / Create empty model
    pub fn new() -> Self {
        Self {
            variables: Vec::new(),
            A: SparseMatrix::new(),
            b: Vec::new(),
            lb: Vec::new(),
            ub: Vec::new(),
            var_types: Vec::new(),
            token_index: HashMap::new(),
        }
    }
    
    /// 从基本机理模型创建 / Create from basic mechanism model
    pub fn from_basic_mechanism_model(model: &BasicMechanismModel<V>) -> Result<Self>;
    
    /// 添加变量 / Add variable
    pub fn add_variable(&mut self, token: Token<V>) -> usize {
        let idx = self.variables.len();
        self.token_index.insert(token.variable.id(), idx);
        self.variables.push(token);
        self.lb.push(V::default());
        self.ub.push(V::default());
        self.var_types.push(VariableType::Continuous);
        idx
    }
    
    /// 添加约束 / Add constraint
    pub fn add_constraint(&mut self, row: SparseVector<V>, rhs: V) {
        self.A.add_row(row);
        self.b.push(rhs);
    }
    
    /// 获取变量数量 / Get variable count
    pub fn num_variables(&self) -> usize {
        self.variables.len()
    }
    
    /// 获取约束数量 / Get constraint count
    pub fn num_constraints(&self) -> usize {
        self.b.len()
    }
    
    /// 通过 ID 查找变量索引 / Find variable index by ID
    pub fn find_variable_index(&self, id: VariableId) -> Option<usize> {
        self.token_index.get(&id).copied()
    }
    
    /// 克隆约束结构（用于对偶转换）/ Clone constraint structure (for dual transformation)
    pub fn clone_constraints(&self) -> (SparseMatrix<V>, Vec<V>) {
        (self.A.clone(), self.b.clone())
    }
    
    /// 合并另一个基本线性三角模型 / Merge another basic linear triad model
    pub fn merge(&mut self, other: &BasicLinearTriadModel<V>) -> Result<()>
    where
        V: Add<Output = V>,
    {
        // 合并变量
        for token in &other.variables {
            self.add_variable(token.clone());
        }
        // 合并约束
        for (i, row) in other.A.rows().enumerate() {
            self.add_constraint(row.clone(), other.b[i].clone());
        }
        Ok(())
    }
}

impl<V> Default for BasicLinearTriadModel<V> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 线性三角模型（带目标函数）/ Linear Triad Model (with Objective)
// ============================================================================

/// 线性三角模型 / Linear Triad Model
/// 
/// 继承基本线性三角模型，添加目标函数支持。
/// Inherits basic linear triad model, adding objective function support.
/// 
/// 标准形式: min c^T x s.t. Ax ≤ b, x ∈ [lb, ub]
/// Standard form: min c^T x s.t. Ax ≤ b, x ∈ [lb, ub]
#[derive(Debug, Clone)]
pub struct LinearTriadModel<V> {
    /// 基本线性三角模型 / Basic linear triad model
    pub basic: BasicLinearTriadModel<V>,
    /// 目标函数系数 / Objective coefficients
    pub c: Vec<V>,
    /// 目标方向 / Objective direction
    pub objective_category: ObjectiveCategory,
}

impl<V: Clone + Debug + Send + Sync + 'static> LinearTriadModel<V> {
    /// 创建空模型 / Create empty model
    pub fn new() -> Self {
        Self {
            basic: BasicLinearTriadModel::new(),
            c: Vec::new(),
            objective_category: ObjectiveCategory::Minimum,
        }
    }
    
    /// 从机理模型创建 / Create from mechanism model
    pub fn from_mechanism_model(model: &MechanismModel<V>) -> Result<Self>;
    
    /// 从基本线性三角模型创建 / Create from basic linear triad model
    pub fn from_basic(basic: BasicLinearTriadModel<V>) -> Self {
        let n = basic.num_variables();
        Self {
            basic,
            c: vec![V::default(); n],
            objective_category: ObjectiveCategory::Minimum,
        }
    }
    
    /// 设置目标函数 / Set objective
    pub fn set_objective(&mut self, c: Vec<V>, category: ObjectiveCategory) {
        self.c = c;
        self.objective_category = category;
    }
    
    /// 获取目标函数系数 / Get objective coefficients
    pub fn objective(&self) -> &[V] {
        &self.c
    }
    
    /// 获取基本模型引用（用于对偶转换）/ Get basic model reference (for dual transformation)
    pub fn as_basic(&self) -> &BasicLinearTriadModel<V> {
        &self.basic
    }
    
    /// 提取基本模型（消耗 self）/ Extract basic model (consumes self)
    pub fn into_basic(self) -> BasicLinearTriadModel<V> {
        self.basic
    }
    
    /// 生成对偶模型 / Generate dual model
    /// 
    /// 将原问题转换为对偶问题。
    /// Converts primal problem to dual problem.
    pub fn to_dual(&self) -> Self 
    where
        V: Neg<Output = V> + Copy,
    {
        // 对偶转换逻辑
        // Dual transformation logic
        // 原问题: min c^T x s.t. Ax ≤ b, x ≥ 0
        // 对偶问题: max b^T y s.t. A^T y ≥ c, y ≥ 0
        todo!("Implement dual transformation")
    }
}

// 委托方法 / Delegate methods
impl<V> std::ops::Deref for LinearTriadModel<V> {
    type Target = BasicLinearTriadModel<V>;
    fn deref(&self) -> &Self::Target { &self.basic }
}

impl<V> std::ops::DerefMut for LinearTriadModel<V> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.basic }
}

impl<V> Default for LinearTriadModel<V> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 基本二次四角模型 / Basic Quadratic Tetrad Model
// ============================================================================

/// 基本二次四角模型 / Basic Quadratic Tetrad Model
/// 
/// 只包含变量和约束的标准形式，不包含目标函数。
/// Standard form with only variables and constraints, without objective.
/// 
/// 标准形式: Ax ≤ b, x ∈ [lb, ub]
/// Standard form: Ax ≤ b, x ∈ [lb, ub]
#[derive(Debug, Clone)]
pub struct BasicQuadraticTetradModel<V> {
    /// 基本线性部分 / Basic linear part
    pub linear: BasicLinearTriadModel<V>,
}

impl<V: Clone + Debug + Send + Sync + 'static> BasicQuadraticTetradModel<V> {
    /// 创建空模型 / Create empty model
    pub fn new() -> Self {
        Self {
            linear: BasicLinearTriadModel::new(),
        }
    }
    
    /// 从基本机理模型创建 / Create from basic mechanism model
    pub fn from_basic_mechanism_model(model: &BasicMechanismModel<V>) -> Result<Self>;
    
    /// 获取变量数量 / Get variable count
    pub fn num_variables(&self) -> usize {
        self.linear.num_variables()
    }
    
    /// 获取约束数量 / Get constraint count
    pub fn num_constraints(&self) -> usize {
        self.linear.num_constraints()
    }
    
    /// 克隆约束结构 / Clone constraint structure
    pub fn clone_constraints(&self) -> (SparseMatrix<V>, Vec<V>) {
        self.linear.clone_constraints()
    }
    
    /// 合并另一个基本二次四角模型 / Merge another basic quadratic tetrad model
    pub fn merge(&mut self, other: &BasicQuadraticTetradModel<V>) -> Result<()> {
        self.linear.merge(&other.linear)
    }
}

impl<V> Default for BasicQuadraticTetradModel<V> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 二次四角模型（带目标函数）/ Quadratic Tetrad Model (with Objective)
// ============================================================================

/// 二次四角模型 / Quadratic Tetrad Model
/// 
/// 继承基本二次四角模型，添加目标函数支持。
/// Inherits basic quadratic tetrad model, adding objective function support.
/// 
/// 标准形式: min x^T Q x + c^T x s.t. Ax ≤ b, x ∈ [lb, ub]
/// Standard form: min x^T Q x + c^T x s.t. Ax ≤ b, x ∈ [lb, ub]
#[derive(Debug, Clone)]
pub struct QuadraticTetradModel<V> {
    /// 基本二次四角模型 / Basic quadratic tetrad model
    pub basic: BasicQuadraticTetradModel<V>,
    /// 线性目标系数 / Linear objective coefficients
    pub c: Vec<V>,
    /// 二次目标矩阵 / Quadratic objective matrix
    pub Q: SparseMatrix<V>,
    /// 目标方向 / Objective direction
    pub objective_category: ObjectiveCategory,
}

impl<V: Clone + Debug + Send + Sync + 'static> QuadraticTetradModel<V> {
    /// 创建空模型 / Create empty model
    pub fn new() -> Self {
        Self {
            basic: BasicQuadraticTetradModel::new(),
            c: Vec::new(),
            Q: SparseMatrix::new(),
            objective_category: ObjectiveCategory::Minimum,
        }
    }
    
    /// 从机理模型创建 / Create from mechanism model
    pub fn from_mechanism_model(model: &MechanismModel<V>) -> Result<Self>;
    
    /// 从基本二次四角模型创建 / Create from basic quadratic tetrad model
    pub fn from_basic(basic: BasicQuadraticTetradModel<V>) -> Self {
        let n = basic.num_variables();
        Self {
            basic,
            c: vec![V::default(); n],
            Q: SparseMatrix::new(),
            objective_category: ObjectiveCategory::Minimum,
        }
    }
    
    /// 设置目标函数 / Set objective
    pub fn set_objective(&mut self, c: Vec<V>, Q: SparseMatrix<V>, category: ObjectiveCategory) {
        self.c = c;
        self.Q = Q;
        self.objective_category = category;
    }
    
    /// 获取线性目标系数 / Get linear objective coefficients
    pub fn linear_objective(&self) -> &[V] {
        &self.c
    }
    
    /// 获取二次目标矩阵 / Get quadratic objective matrix
    pub fn quadratic_objective(&self) -> &SparseMatrix<V> {
        &self.Q
    }
    
    /// 获取基本模型引用 / Get basic model reference
    pub fn as_basic(&self) -> &BasicQuadraticTetradModel<V> {
        &self.basic
    }
    
    /// 提取基本模型（消耗 self）/ Extract basic model (consumes self)
    pub fn into_basic(self) -> BasicQuadraticTetradModel<V> {
        self.basic
    }
}

// 委托方法 / Delegate methods
impl<V> std::ops::Deref for QuadraticTetradModel<V> {
    type Target = BasicQuadraticTetradModel<V>;
    fn deref(&self) -> &Self::Target { &self.basic }
}

impl<V> std::ops::DerefMut for QuadraticTetradModel<V> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.basic }
}

impl<V> Default for QuadraticTetradModel<V> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 类型别名 / Type Aliases
// ============================================================================

/// f64 精度的线性三角模型 / Linear triad model with f64 precision
pub type LinearTriadModelF64 = LinearTriadModel<f64>;

/// f64 精度的基本线性三角模型 / Basic linear triad model with f64 precision
pub type BasicLinearTriadModelF64 = BasicLinearTriadModel<f64>;

/// f64 精度的二次四角模型 / Quadratic tetrad model with f64 precision
pub type QuadraticTetradModelF64 = QuadraticTetradModel<f64>;

/// f64 精度的基本二次四角模型 / Basic quadratic tetrad model with f64 precision
pub type BasicQuadraticTetradModelF64 = BasicQuadraticTetradModel<f64>;
```

**模型层级总览 / Model Layer Overview**:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          用户建模层 / User Modeling Layer                 │
│  BasicModel<V> → MetaModel<V>                                            │
│  (变量+约束)       (变量+约束+目标)                                         │
└─────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────┐
│                          机理模型层 / Mechanism Model Layer               │
│  BasicMechanismModel<V> → MechanismModel<V>                              │
│  (展开后变量+约束)          (展开后变量+约束+目标)                           │
└─────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────┐
│                        中间模型层 / Intermediate Model Layer              │
│  BasicLinearTriadModel<V> → LinearTriadModel<V>                          │
│  (标准形式变量+约束)          (标准形式变量+约束+目标)                       │
│                                                                          │
│  BasicQuadraticTetradModel<V> → QuadraticTetradModel<V>                 │
│  (标准形式变量+约束)              (标准形式变量+约束+目标)                   │
└─────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────┐
│                            求解器层 / Solver Layer                        │
│  Gurobi, COPT, SCIP, etc.                                               │
└─────────────────────────────────────────────────────────────────────────┘
```

**对偶模型创建示例 / Dual Model Creation Example**:

```rust
// 从元模型创建机理模型
let mechanism = MechanismModel::from_meta_model(&meta_model).await?;

// 转换为线性三角模型（标准形式）
let triad = LinearTriadModel::from_mechanism_model(&mechanism)?;

// 在中间模型层创建对偶模型
// 对偶转换需要在标准形式下进行
let dual_model = triad.to_dual();

// 或者从基本线性三角模型创建多个对偶模型
let basic_triad = triad.into_basic();

let mut dual1 = LinearTriadModel::from_basic(basic_triad.clone());
dual1.set_objective(c1.clone(), ObjectiveCategory::Maximum);

let mut dual2 = LinearTriadModel::from_basic(basic_triad);
dual2.set_objective(c2.clone(), ObjectiveCategory::Minimum);
```

**对偶转换在设计中的位置 / Position of Dual Transformation in Design**:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          用户建模层 / User Modeling Layer                 │
│  BasicModel<V> → MetaModel<V>                                            │
│  (变量+约束)       (变量+约束+目标)                                         │
│  不支持对偶转换（包含中间符号，未展开）                                       │
└─────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────┐
│                          机理模型层 / Mechanism Model Layer               │
│  BasicMechanismModel<V> → MechanismModel<V>                              │
│  (展开后变量+约束)          (展开后变量+约束+目标)                           │
│  不支持对偶转换（仍包含结构化约束，非标准形式）                                 │
└─────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────┐
│                        中间模型层 / Intermediate Model Layer              │
│  BasicLinearTriadModel<V> → LinearTriadModel<V>                          │
│  (标准形式变量+约束)          (标准形式变量+约束+目标)                       │
│                                                                          │
│  ✅ 支持对偶转换（标准形式，可进行数学变换）                                   │
│  ✅ to_dual() 方法在此层实现                                               │
│                                                                          │
│  BasicQuadraticTetradModel<V> → QuadraticTetradModel<V>                 │
│  (标准形式变量+约束)              (标准形式变量+约束+目标)                   │
└─────────────────────────────────────────────────────────────────────────┘
```

**为什么对偶转换在中间模型层？/ Why Dual Transformation in Intermediate Model Layer?**

1. **标准形式**: 中间模型层已经是标准形式 `min c^T x s.t. Ax ≤ b`，可以直接应用对偶理论
   - **Standard Form**: Intermediate model layer is already in standard form, can directly apply dual theory

2. **完全展开**: 中间符号已全部展开为纯变量表达式
   - **Fully Expanded**: Intermediate symbols are fully expanded to pure variable expressions

3. **矩阵表示**: 约束已转换为矩阵形式 `Ax`，便于对偶变换
   - **Matrix Representation**: Constraints are converted to matrix form, easy for dual transformation

4. **数学正确性**: 对偶理论要求标准形式，前两层不满足条件
   - **Mathematical Correctness**: Dual theory requires standard form, previous layers don't satisfy conditions

---

#### 3.7 求解器接口 (Solver Interface)

**目录结构**:
```
src/solver/
├── mod.rs
├── solver.rs             # Solver trait
├── solver_config.rs      # SolverConfig
├── solver_output.rs      # SolverOutput
└── solvers/
    ├── mod.rs
    ├── gurobi.rs         # Gurobi
    ├── copt.rs           # COPT
    └── scip.rs           # SCIP
```

**核心 Trait**:

```rust
/// 求解器 / Solver
#[async_trait]
pub trait Solver: Send + Sync {
    /// 求解器名称 / Solver name
    fn name(&self) -> &str;
    
    /// 求解线性模型 / Solve linear model
    async fn solve(&self, model: &LinearTriadModel) -> Result<SolverOutput>;
    
    /// 求解二次模型 / Solve quadratic model
    async fn solve_quadratic(&self, model: &QuadraticTetradModel) -> Result<SolverOutput>;
    
    /// 异步求解 / Async solve
    async fn solve_async(&self, model: &LinearTriadModel) -> Receiver<SolverOutput>;
}

/// 求解结果 / Solver Output
#[derive(Debug, Clone)]
pub struct SolverOutput {
    /// 求解状态 / Solver status
    pub status: SolverStatus,
    /// 目标值 / Objective value
    pub objective_value: Option<f64>,
    /// 解向量 / Solution vector
    pub solution: Option<Vec<f64>>,
    /// 对偶解 / Dual solution
    pub dual_solution: Option<Vec<f64>>,
    /// 求解时间 / Solve time
    pub solve_time: Duration,
    /// 迭代次数 / Iteration count
    pub iterations: Option<usize>,
}

/// 求解状态 / Solver Status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverStatus {
    /// 最优 / Optimal
    Optimal,
    /// 不可行 / Infeasible
    Infeasible,
    /// 无界 / Unbounded
    Unbounded,
    /// 达到迭代上限 / Iteration limit
    IterationLimit,
    /// 达到时间上限 / Time limit
    TimeLimit,
    /// 数值错误 / Numeric error
    NumericError,
    /// 未知 / Unknown
    Unknown,
}
```

---

### 四、依赖关系

```toml
[package]
name = "ospf-rust-core"
version = "0.1.0"
edition = "2021"

[dependencies]
ospf-rust-base = { path = "../ospf-rust-base" }
ospf-rust-math = { path = "../ospf-rust-math" }
ospf-rust-multiarray = { path = "../ospf-rust-multiarray", optional = true }
ospf-rust-quantities = { path = "../ospf-rust-quantities", optional = true }

# 异步运行时 / Async runtime
tokio = { version = "1", features = ["rt-multi-thread", "sync"], optional = true }

# 序列化 / Serialization
serde = { version = "1", features = ["derive"], optional = true }

# 错误处理 / Error handling
thiserror = "1"

# 并发 / Concurrency
parking_lot = "0.12"

# 可选求解器绑定 / Optional solver bindings
gurobi = { version = "0.3", optional = true }

[features]
default = []
serde = ["dep:serde"]
async = ["dep:tokio"]
quantities = ["ospf-rust-quantities"]
multiarray = ["ospf-rust-multiarray"]
gurobi = ["dep:gurobi"]
```

---

### 五、实现优先级

| 优先级 | 阶段 | 模块 | 预估工作量 | 状态 |
|--------|------|------|-----------|------|
| **P0** | 1 | 变量系统 | 2-3 天 | ✅ 已完成 |
| **P0** | 2 | Token 系统 | 2-3 天 | ✅ 已完成 |
| **P1** | 3 | 表达式平展系统 | 2-3 天 | ✅ 已完成 |
| **P1** | 4 | 中间符号系统 | 5-7 天 | ✅ 已完成 |
| **P1** | 5 | 约束系统 | 1-2 天 | ✅ 已完成 |
| **P1** | 6 | 模型系统 | 3-4 天 | ✅ 已完成 |
| **P2** | 7 | 中间模型层 | 2-3 天 | ✅ 已完成 |
| **P3** | 8 | 求解器接口 | 3-5 天/求解器 | ✅ 已完成 |
| **P2** | 9 | 回调模型层 | 2-3 天 | ✅ 已完成 |
| **P2** | 10 | IIS 计算模块 | 3-4 天 | ✅ 已完成 |
| **P3** | 11 | 启发式算法模块 | 4-5 天 | ✅ 已完成 |
| **P2** | 12 | 新增函数符号 | 5-7 天 | ✅ 已完成 |

---

### 六、使用示例

```rust
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::variable::{Variable1D, VariableType};
use ospf_rust_core::symbol::functions::MinFunction;

// 创建模型 / Create model
let mut model = MetaModel::new("production_planning");

// 创建变量 / Create variables
let production = Variable1D::new(
    "production",
    10,
    VariableType::Continuous,
    Some(0.0),
    Some(100.0),
);

// 添加变量到模型 / Add variables to model
model.add_variables(production.iter().cloned())?;

// 添加约束 / Add constraints
// 总产量约束 / Total production constraint
model.add_constraint(
    sum(production.iter()) >= 50.0
)?;

// 添加目标函数 / Add objective
model.add_objective(
    sum(production.iter().map(|x| 2.0 * x.clone()))
)?;

// 求解 / Solve
let solver = GurobiSolver::new();
let result = solver.solve(&model).await?;

// 获取解 / Get solution
if let Some(solution) = result.solution {
    for (i, value) in solution.iter().enumerate() {
        println!("production[{}] = {}", i, value);
    }
}
```

---

<a name="chinese-implementation-plan"></a>
### 七、文件结构与详细实现计划

#### 7.1 整体文件结构

```
ospf-rust-core/
├── Cargo.toml
├── README.md
├── README_ch.md
├── design.md                    # 本设计文档
└── src/
    ├── lib.rs                   # 库入口
    │
    ├── variable/                # 阶段 1: 变量系统
    │   ├── mod.rs
    │   ├── variable_type.rs     # VariableType 枚举，VariableTypeTrait
    │   ├── variable_key.rs      # VariableItemKey (新增)
    │   ├── variable_id.rs       # VariableId, VariableIdGenerator
    │   ├── variable_range.rs    # VariableRange<V>
    │   ├── variable_item.rs     # GenericVariableData, GenericVariableItem
    │   ├── variable_combination.rs  # VariableCombination<S>
    │   └── variable_arena.rs    # VariableArena, GenericVariableArena
    │
├── token/                   # 阶段 2: Token 系统
│   ├── mod.rs
│   ├── token.rs             # Token<V>, AnyVariable<V>
│   ├── token_list.rs        # TokenList<V> trait, MutableTokenList
│   ├── token_table.rs       # TokenTable<V> trait, MutableTokenTable
│   ├── auto_token.rs        # AutoTokenList, AutoTokenTable (新增)
│   ├── manual_token.rs      # ManualTokenList, ManualTokenTable (新增)
│   ├── concurrent_token.rs  # ConcurrentTokenTable, ConcurrentAutoTokenTable (新增)
│   └── token_error.rs       # RepeatedSymbolError (新增)
    │
    ├── flatten/                 # 阶段 3: 表达式平展系统
    │   ├── mod.rs
    │   ├── cache_key.rs         # CacheKey, Cacheable trait
    │   ├── inner_box.rs         # LinearMonomialInner, LinearInner 等 Inner Box 设计
    │   ├── flatten_context.rs   # FlattenContextTrait<V>
    │   ├── flattenable.rs       # Flattenable<C, V> trait
    │   └── lazy_context.rs      # LazyLinearFlattenContext, LazyQuadraticFlattenContext
    │
    ├── symbol/                  # 阶段 4: 中间符号系统
    │   ├── mod.rs
    │   ├── intermediate_symbol.rs   # IntermediateSymbol trait
    │   ├── expression_symbol.rs     # ExpressionSymbol, LinearExpressionSymbol
    │   ├── function_symbol.rs       # FunctionSymbol trait
    │   ├── symbol_combination.rs    # SymbolCombination<S> (新增)
    │   ├── monomial_cell.rs         # LinearMonomialCell, QuadraticMonomialCell
    │   └── functions/               # 函数符号实现
    │       ├── mod.rs
    │       ├── min_max.rs           # MinFunction, MaxFunction
    │       ├── max_min.rs           # MaxMinFunction, MinMaxFunction
    │       ├── abs.rs               # AbsFunction
    │       ├── floor_ceiling.rs     # FloorFunction, CeilingFunction
    │       ├── if_then.rs           # IfFunction, IfThenFunction
    │       ├── one_of.rs            # OneOfFunction, IfElseFunction (新增)
    │       ├── binaryzation.rs      # BinaryzationFunction (新增)
    │       ├── semi.rs              # SemiFunction (新增)
    │       ├── balance_ternary.rs   # BalanceTernaryzationFunction (新增)
    │       ├── inequality.rs        # InequalityFunction (新增)
    │       ├── in_step_range.rs     # InStepRangeFunction (新增)
    │       ├── satisfied_amount.rs  # SatisfiedAmountFunction (新增)
    │       ├── same_as.rs           # SameAsFunction (新增)
    │       ├── first.rs             # FirstFunction (新增)
    │       ├── trigonometric.rs     # SinFunction, CosFunction (新增)
    │       ├── mod.rs               # ModFunction (新增)
    │       ├── rounding.rs          # RoundingFunction (新增)
    │       ├── piecewise.rs         # UnivariateLinearPiecewise, BivariateLinearPiecewise
    │       ├── logic.rs             # AndFunction, OrFunction, NotFunction, XorFunction
    │       ├── sigmoid.rs           # SigmoidFunction
    │       ├── slack.rs             # SlackFunction, SlackRangeFunction
    │       └── masking.rs           # MaskingFunction, MaskingRangeFunction
    │
    ├── constraint/              # 阶段 5: 约束系统
    │   ├── mod.rs
    │   ├── constraint.rs        # Constraint<P>
    │   ├── meta_constraint.rs   # MetaConstraint<I>
    │   ├── constraint_group.rs  # MetaConstraintGroup
    │   └── constraint_source.rs # ConstraintSource 枚举 (新增)
    │
├── model/                   # 阶段 6: 模型系统
│   ├── mod.rs
│   ├── basic_model.rs       # BasicModel<V>
│   ├── meta_model.rs        # MetaModel<V> 主实现（用户建模层）
│   ├── mechanism/           # 机理层实现
│   │   ├── mod.rs
│   │   ├── basic_mechanism_model.rs  # BasicMechanismModel<V>
│   │   ├── mechanism_model.rs        # MechanismModel<V>
│   │   ├── constraint.rs
│   │   ├── meta_constraint.rs
│   │   ├── constraint_group.rs
│   │   └── meta_model.rs             # 兼容层：re-export 到 model::meta_model
│   ├── object.rs            # Objective<V>, SubObjective<V>
│   ├── configuration.rs     # MetaModelConfiguration
│   ├── value_cache.rs       # ValueCacheContextTrait, LazyValueCacheContext
│   ├── range_cache.rs       # RangeCacheContextTrait, LazyRangeCacheContext
│   └── multi_object.rs      # MultiObjectLocation, MulObj (新增)
    │
    ├── callback_model/          # 阶段 9: 回调模型层 (新增)
    │   ├── mod.rs
    │   ├── callback_model_trait.rs    # CallBackModelInterface
    │   ├── abstract_callback_model.rs # AbstractCallBackModelInterface
    │   ├── multi_objective_model.rs   # MultiObjectiveModelInterface
    │   └── solution.rs          # Solution 类型定义
    │
    ├── intermediate_model/      # 阶段 7: 中间模型层
    │   ├── mod.rs
    │   ├── basic_linear_triad_model.rs   # BasicLinearTriadModel<V>
    │   ├── linear_triad_model.rs         # LinearTriadModel<V>
    │   ├── linear_triad_model_view.rs    # LinearTriadModelView (新增)
    │   ├── basic_quadratic_tetrad_model.rs  # BasicQuadraticTetradModel<V>
    │   └── quadratic_tetrad_model.rs     # QuadraticTetradModel<V>
    │
    ├── solver/                  # 阶段 8: 求解器接口
    │   ├── mod.rs
    │   ├── solver.rs            # Solver trait
    │   ├── solver_config.rs     # SolverConfig
    │   ├── solver_output.rs     # SolverOutput, SolverStatus
    │   ├── gap.rs               # Gap 计算 (新增)
    │   ├── solvers/
    │   │   ├── mod.rs
    │   │   ├── gurobi.rs        # GurobiSolver
    │   │   ├── copt.rs          # COPTSolver
    │   │   └── scip.rs          # SCIPSolver
    │   ├── iis/                 # IIS 计算 (新增)
    │   │   ├── mod.rs
    │   │   ├── iis_config.rs    # IISConfig
    │   │   ├── iis_model.rs     # LinearIISModel
    │   │   ├── elastic_filtering.rs
    │   │   └── deletion_filtering.rs
    │   └── heuristic/           # 启发式算法 (新增)
    │       ├── mod.rs
    │       ├── individual.rs    # Individual trait
    │       ├── population.rs    # Population
    │       ├── solution_fitness.rs  # SolutionWithFitness
    │       ├── cross.rs         # 交叉算子
    │       ├── mutation.rs      # 变异算子
    │       ├── selection.rs     # 选择算子
    │       ├── migration.rs     # 迁移算子
    │       └── normalization.rs # 归一化
    │
    └── error.rs                 # 错误类型定义
```

#### 7.2 详细实现计划

##### 阶段 1: 变量系统 (Priority P0, 2-3 天)

**任务清单**:

| 任务 | 文件 | 描述 | 预估时间 |
|------|------|------|---------|
| 1.1 | `variable_type.rs` | 定义 `VariableType` 枚举 | 0.5h |
| 1.2 | `variable_type.rs` | 实现 `VariableTypeTrait` 和各类型标记 | 1h |
| 1.3 | `variable_id.rs` | 实现 `VariableId` 结构体 | 0.5h |
| 1.4 | `variable_id.rs` | 实现 `VariableIdGenerator` 和全局生成器 | 0.5h |
| 1.5 | `variable_range.rs` | 实现 `VariableRange<V>` | 0.5h |
| 1.6 | `variable_item.rs` | 实现 `GenericVariableData<VT>` | 1h |
| 1.7 | `variable_item.rs` | 实现 `GenericVariableItem<VT>` 和类型别名 | 1h |
| 1.8 | `variable_combination.rs` | 实现 `VariableCombination<S>` | 2h |
| 1.9 | `variable_arena.rs` | 实现 `VariableArena` (typed_arena) | 1h |
| 1.10 | `variable_arena.rs` | 实现 `GenericVariableArena<VT>` | 0.5h |
| 1.11 | `variable_arena.rs` | 实现 `ConcurrentVariableArena` | 0.5h |
| 1.12 | `mod.rs` | 模块组织和导出 | 0.5h |
| 1.13 | 测试 | 单元测试和文档测试 | 2h |

**关键代码结构**:

```rust
// src/variable/mod.rs
pub mod variable_type;
pub mod variable_id;
pub mod variable_range;
pub mod variable_item;
pub mod variable_combination;
pub mod variable_arena;

pub use variable_type::*;
pub use variable_id::*;
pub use variable_range::*;
pub use variable_item::*;
pub use variable_combination::*;
pub use variable_arena::*;

// 类型别名
pub type BinaryVariableItem = GenericVariableItem<Binary>;
pub type ContinuousVariableItem = GenericVariableItem<Continuous>;
pub type IntegerVariableItem = GenericVariableItem<Integer>;
// ... 更多类型别名

pub type Variable1D = VariableCombination<Shape1>;
pub type Variable2D = VariableCombination<Shape2>;
// ... 更多维度别名
```

##### 阶段 2: Token 系统 (Priority P0, 2-3 天)

**任务清单**:

| 任务 | 文件 | 描述 | 预估时间 |
|------|------|------|---------|
| 2.1 | `token.rs` | 定义 `IntoValue<V>` trait | 0.5h |
| 2.2 | `token.rs` | 定义 `Variable<V>` trait | 0.5h |
| 2.3 | `token.rs` | 实现 `AnyVariable<V>` | 1h |
| 2.4 | `token.rs` | 实现 `GenericVariableWrapper<VT, V>` | 1h |
| 2.5 | `token.rs` | 实现 `Token<V>` | 1h |
| 2.6 | `token_list.rs` | 定义 `TokenList<V>` trait | 0.5h |
| 2.7 | `token_list.rs` | 实现 `VecTokenList<V>` | 1h |
| 2.8 | `token_list.rs` | 实现 `MutableTokenList<V>` | 1h |
| 2.9 | `token_list.rs` | 实现 `AutoTokenList<V>` | 1h |
| 2.10 | `token_list.rs` | 实现 `ManualTokenList<V>` | 1h |
| 2.11 | `token_table.rs` | 定义 `TokenTable<V>` trait | 1h |
| 2.12 | `token_table.rs` | 实现 `TokenTableImpl<V>` | 2h |
| 2.13 | `token_table.rs` | 实现 `MutableTokenTable<V>` | 2h |
| 2.14 | `value_cache.rs` | 定义 `ValueCacheKey` 和 `ValueCacheKind` | 0.5h |
| 2.15 | `value_cache.rs` | 定义 `ValueCacheContextTrait<V>` | 1h |
| 2.16 | `value_cache.rs` | 实现 `LazyValueCacheContext<V, T>` | 1h |
| 2.17 | `range_cache.rs` | 定义 `RangeCacheKey` | 0.5h |
| 2.18 | `range_cache.rs` | 定义 `RangeCacheContextTrait<V>` | 1h |
| 2.19 | `range_cache.rs` | 实现 `LazyRangeCacheContext<V, T>` | 1h |
| 2.20 | `mod.rs` | 模块组织和导出 | 0.5h |
| 2.21 | 测试 | 单元测试 | 2h |

**关键代码结构**:

```rust
// src/token/mod.rs
pub mod token;
pub mod token_list;
pub mod token_table;
pub mod value_cache;
pub mod range_cache;

pub use token::*;
pub use token_list::*;
pub use token_table::*;
pub use value_cache::*;
pub use range_cache::*;

// 类型别名
pub type TokenF64 = Token<f64>;
pub type TokenListF64 = TokenList<f64>;
pub type TokenTableF64 = TokenTable<f64>;
```

##### 阶段 3: 表达式平展系统 (Priority P1, 2-3 天)

**任务清单**:

| 任务 | 文件 | 描述 | 预估时间 |
|------|------|------|---------|
| 3.1 | `cache_key.rs` | 实现 `CacheKey` | 0.5h |
| 3.2 | `cache_key.rs` | 定义 `Cacheable` trait | 0.5h |
| 3.3 | `inner_box.rs` | 实现 `LinearMonomialInner<V>` 和 `LinearMonomial<V>` | 1h |
| 3.4 | `inner_box.rs` | 实现 `LinearInner<V>` 和 `Linear<V>` | 1h |
| 3.5 | `inner_box.rs` | 实现 `QuadraticMonomialInner<V>` 等 | 1h |
| 3.6 | `inner_box.rs` | 实现 `QuadraticInner<V>` 和 `Quadratic<V>` | 1h |
| 3.7 | `inner_box.rs` | 实现 `CanonicalMonomialInner<V>` 等 | 1h |
| 3.8 | `inner_box.rs` | 实现 `CanonicalInner<V>` 和 `Canonical<V>` | 1h |
| 3.9 | `flatten_context.rs` | 定义 `FlattenContextTrait<V>` | 1h |
| 3.10 | `flatten_context.rs` | 定义 `FlattenedMonomial`, `FlattenedPolynomial`, `FlattenedSymbol` | 0.5h |
| 3.11 | `flattenable.rs` | 定义 `Identified` trait | 0.5h |
| 3.12 | `flattenable.rs` | 定义 `Flattenable<C, V>` trait | 1h |
| 3.13 | `lazy_context.rs` | 实现 `LazyLinearFlattenContext<V, T>` | 2h |
| 3.14 | `lazy_context.rs` | 实现 `LazyQuadraticFlattenContext<V, T>` | 1.5h |
| 3.15 | `lazy_context.rs` | 实现 `LazyCanonicalFlattenContext<V, T>` | 1.5h |
| 3.16 | `mod.rs` | 模块组织和导出 | 0.5h |
| 3.17 | 测试 | 单元测试 | 2h |

**关键代码结构**:

```rust
// src/flatten/mod.rs
pub mod cache_key;
pub mod inner_box;
pub mod flatten_context;
pub mod flattenable;
pub mod lazy_context;

pub use cache_key::*;
pub use inner_box::*;
pub use flatten_context::*;
pub use flattenable::*;
pub use lazy_context::*;

// 重新导出 ospf_rust_math::symbol 中的相关类型
pub use ospf_rust_math::symbol::{OwnedSymbol, DynSymbol, SymbolDynId};
```

##### 阶段 4: 中间符号系统 (Priority P1, 5-7 天)

**任务清单**:

| 任务 | 文件 | 描述 | 预估时间 |
|------|------|------|---------|
| 4.1 | `intermediate_symbol.rs` | 定义 `Category` 枚举 | 0.5h |
| 4.2 | `intermediate_symbol.rs` | 定义 `IntermediateSymbol` trait | 1h |
| 4.3 | `intermediate_symbol.rs` | 定义 `LinearIntermediateSymbol` trait | 0.5h |
| 4.4 | `intermediate_symbol.rs` | 定义 `QuadraticIntermediateSymbol` trait | 0.5h |
| 4.5 | `monomial_cell.rs` | 实现 `LinearMonomialCell` | 1h |
| 4.6 | `monomial_cell.rs` | 实现 `QuadraticMonomialCell` | 1h |
| 4.7 | `expression_symbol.rs` | 实现 `ExpressionSymbol` | 2h |
| 4.8 | `expression_symbol.rs` | 实现 `LinearExpressionSymbol` | 2h |
| 4.9 | `expression_symbol.rs` | 实现 `QuadraticExpressionSymbol` | 2h |
| 4.10 | `function_symbol.rs` | 定义 `FunctionSymbol` trait | 1h |
| 4.11 | `function_symbol.rs` | 定义 `LogicFunctionSymbol` trait | 0.5h |
| 4.12 | `function_symbol.rs` | 实现 `LinearFunctionSymbol` | 1.5h |
| 4.13 | `function_symbol.rs` | 实现 `QuadraticFunctionSymbol` | 1.5h |
| 4.14 | `functions/min_max.rs` | 实现 `MinFunction` | 2h |
| 4.15 | `functions/min_max.rs` | 实现 `MaxFunction` | 1h |
| 4.16 | `functions/max_min.rs` | 实现 `MaxMinFunction` | 2h |
| 4.17 | `functions/max_min.rs` | 实现 `MinMaxFunction` | 1h |
| 4.18 | `functions/abs.rs` | 实现 `AbsFunction` | 1.5h |
| 4.19 | `functions/floor_ceiling.rs` | 实现 `FloorFunction` | 1.5h |
| 4.20 | `functions/floor_ceiling.rs` | 实现 `CeilingFunction` | 1h |
| 4.21 | `functions/if_then.rs` | 实现 `IfFunction` | 1.5h |
| 4.22 | `functions/if_then.rs` | 实现 `IfThenFunction` | 2h |
| 4.23 | `functions/piecewise.rs` | 实现 `UnivariateLinearPiecewise` | 3h |
| 4.24 | `functions/piecewise.rs` | 实现 `BivariateLinearPiecewise` | 3h |
| 4.25 | `functions/logic.rs` | 实现 `AndFunction` | 1.5h |
| 4.26 | `functions/logic.rs` | 实现 `OrFunction` | 1h |
| 4.27 | `functions/logic.rs` | 实现 `NotFunction` | 1h |
| 4.28 | `functions/logic.rs` | 实现 `XorFunction` | 1.5h |
| 4.29 | `functions/sigmoid.rs` | 实现 `SigmoidFunction` | 2h |
| 4.30 | `functions/slack.rs` | 实现 `SlackFunction` | 1.5h |
| 4.31 | `functions/slack.rs` | 实现 `SlackRangeFunction` | 1.5h |
| 4.32 | `functions/masking.rs` | 实现 `MaskingFunction` | 1.5h |
| 4.33 | `functions/masking.rs` | 实现 `MaskingRangeFunction` | 1.5h |
| 4.34 | `mod.rs` | 模块组织和导出 | 0.5h |
| 4.35 | 测试 | 单元测试和集成测试 | 4h |

**关键代码结构**:

```rust
// src/symbol/mod.rs
pub mod intermediate_symbol;
pub mod expression_symbol;
pub mod function_symbol;
pub mod monomial_cell;
pub mod functions;

pub use intermediate_symbol::*;
pub use expression_symbol::*;
pub use function_symbol::*;
pub use monomial_cell::*;

// 函数符号重新导出
pub use functions::*;
```

##### 阶段 5: 约束系统 (Priority P1, 1-2 天)

**任务清单**:

| 任务 | 文件 | 描述 | 预估时间 |
|------|------|------|---------|
| 5.1 | `constraint.rs` | 实现 `Constraint<P>` | 1h |
| 5.2 | `meta_constraint.rs` | 定义 `InequalityTrait` | 0.5h |
| 5.3 | `meta_constraint.rs` | 实现 `MetaConstraint<I>` | 1.5h |
| 5.4 | `constraint_group.rs` | 实现 `MetaConstraintGroup` | 1h |
| 5.5 | `mod.rs` | 模块组织和导出 | 0.5h |
| 5.6 | 测试 | 单元测试 | 1h |

##### 阶段 6: 模型系统 (Priority P1, 3-4 天)

**任务清单**:

| 任务 | 文件 | 描述 | 预估时间 |
|------|------|------|---------|
| 6.1 | `object.rs` | 定义 `ObjectiveCategory` 枚举 | 0.5h |
| 6.2 | `object.rs` | 实现 `SubObjective<V>` | 1h |
| 6.3 | `object.rs` | 实现 `Objective<V>` | 1h |
| 6.4 | `configuration.rs` | 实现 `MetaModelConfiguration` | 1h |
| 6.5 | `basic_model.rs` | 实现 `BasicModel<V>` | 3h |
| 6.6 | `basic_model.rs` | 实现变量添加、符号添加方法 | 2h |
| 6.7 | `basic_model.rs` | 实现约束添加方法 | 1.5h |
| 6.8 | `basic_model.rs` | 实现解设置和缓存方法（含三类 context 失效策略） | 1.5h |
| 6.9 | `meta_model.rs` | 实现 `MetaModel<V>` 与 `ensure_*_context()` 初始化流程 | 2h |
| 6.10 | `meta_model.rs` | 实现目标函数添加方法 | 1h |
| 6.11 | `meta_model.rs` | 实现 `Deref`/`DerefMut` 委托 | 0.5h |
| 6.12 | `basic_mechanism_model.rs` | 实现 `BasicMechanismModel<V>` | 3h |
| 6.13 | `basic_mechanism_model.rs` | 实现从 BasicModel 创建 | 2h |
| 6.14 | `mechanism_model.rs` | 实现 `MechanismModel<V>` | 2h |
| 6.15 | `mechanism_model.rs` | 实现从 MetaModel 创建 | 2h |
| 6.16 | `mechanism_model.rs` | 实现割生成方法 | 2h |
| 6.17 | `mod.rs` | 模块组织和导出 | 0.5h |
| 6.18 | 测试 | 单元测试和集成测试 | 3h |

##### 阶段 7: 中间模型层 (Priority P2, 2-3 天)

**任务清单**:

| 任务 | 文件 | 描述 | 预估时间 |
|------|------|------|---------|
| 7.1 | `basic_linear_triad_model.rs` | 实现 `BasicLinearTriadModel<V>` | 3h |
| 7.2 | `basic_linear_triad_model.rs` | 实现变量和约束添加 | 1.5h |
| 7.3 | `basic_linear_triad_model.rs` | 实现从 BasicMechanismModel 创建 | 2h |
| 7.4 | `linear_triad_model.rs` | 实现 `LinearTriadModel<V>` | 2h |
| 7.5 | `linear_triad_model.rs` | 实现目标函数设置 | 1h |
| 7.6 | `linear_triad_model.rs` | 实现对偶转换 `to_dual()` | 3h |
| 7.7 | `basic_quadratic_tetrad_model.rs` | 实现 `BasicQuadraticTetradModel<V>` | 2h |
| 7.8 | `quadratic_tetrad_model.rs` | 实现 `QuadraticTetradModel<V>` | 2h |
| 7.9 | `quadratic_tetrad_model.rs` | 实现目标函数设置 | 1h |
| 7.10 | `mod.rs` | 模块组织和导出 | 0.5h |
| 7.11 | 测试 | 单元测试 | 2h |

##### 阶段 8: 求解器接口 (Priority P3, 3-5 天/求解器)

**任务清单**:

| 任务 | 文件 | 描述 | 预估时间 |
|------|------|------|---------|
| 8.1 | `solver_output.rs` | 定义 `SolverStatus` 枚举 | 0.5h |
| 8.2 | `solver_output.rs` | 实现 `SolverOutput` | 1h |
| 8.3 | `solver_config.rs` | 实现 `SolverConfig` | 1h |
| 8.4 | `solver.rs` | 定义 `Solver` trait | 1h |
| 8.5 | `solvers/gurobi.rs` | 实现 `GurobiSolver` | 4h |
| 8.6 | `solvers/copt.rs` | 实现 `COPTSolver` | 4h |
| 8.7 | `solvers/scip.rs` | 实现 `SCIPSolver` | 4h |
| 8.8 | `mod.rs` | 模块组织和导出 | 0.5h |
| 8.9 | 测试 | 集成测试 | 2h/求解器 |

##### 阶段 9: 回调模型层 (Priority P2, 2-3 天) - 新增

**任务清单**:

| 任务 | 文件 | 描述 | 预估时间 |
|------|------|------|---------|
| 9.1 | `solution.rs` | 定义 `Solution` 类型别名 | 0.5h |
| 9.2 | `callback_model_trait.rs` | 定义 `CallBackModelInterface` trait | 1.5h |
| 9.3 | `callback_model_trait.rs` | 定义 `AbstractCallBackModelInterface` trait | 1h |
| 9.4 | `callback_model_trait.rs` | 实现 `MultiObjectiveModelInterface` | 1h |
| 9.5 | `abstract_callback_model.rs` | 实现 `AbstractCallBackModel` 基类 | 2h |
| 9.6 | `multi_objective_model.rs` | 实现 `MultiObjectLocation` | 0.5h |
| 9.7 | `multi_objective_model.rs` | 实现 `MulObj` 类型和多目标计算逻辑 | 1.5h |
| 9.8 | `mod.rs` | 模块组织和导出 | 0.5h |
| 9.9 | 测试 | 单元测试和集成测试 | 2h |

##### 阶段 10: IIS 计算模块 (Priority P2, 3-4 天) - 新增

**任务清单**:

| 任务 | 文件 | 描述 | 预估时间 |
|------|------|------|---------|
| 10.1 | `iis_config.rs` | 实现 `IISConfig` 配置结构 | 1h |
| 10.2 | `iis_model.rs` | 实现 `LinearIISModel` | 2h |
| 10.3 | `iis_model.rs` | 实现 `BasicLinearTriadModelView` trait | 1h |
| 10.4 | `elastic_filtering.rs` | 实现弹性过滤算法 | 3h |
| 10.5 | `deletion_filtering.rs` | 实现删除过滤算法 | 3h |
| 10.6 | `constraint_source.rs` | 实现 `ConstraintSource` 枚举 | 0.5h |
| 10.7 | `mod.rs` | 模块组织和导出，实现 `computeIIS()` 函数 | 1.5h |
| 10.8 | 测试 | 集成测试 | 2h |

##### 阶段 11: 启发式算法模块 (Priority P3, 4-5 天) - 新增

**任务清单**:

| 任务 | 文件 | 描述 | 预估时间 |
|------|------|------|---------|
| 11.1 | `individual.rs` | 定义 `Individual` trait | 0.5h |
| 11.2 | `solution_fitness.rs` | 实现 `SolutionWithFitness` | 0.5h |
| 11.3 | `population.rs` | 实现 `Population` 结构 | 2h |
| 11.4 | `population.rs` | 实现 `refreshGoodIndividuals()` 函数 | 1h |
| 11.5 | `cross.rs` | 实现交叉算子 | 2h |
| 11.6 | `mutation.rs` | 实现变异算子 | 2h |
| 11.7 | `selection.rs` | 实现选择算子 | 1.5h |
| 11.8 | `migration.rs` | 实现迁移算子 | 1.5h |
| 11.9 | `normalization.rs` | 实现归一化 | 1h |
| 11.10 | `mod.rs` | 模块组织和导出 | 0.5h |
| 11.11 | 测试 | 集成测试 | 2h |

##### 阶段 12: 新增函数符号 (Priority P2, 5-7 天) - 新增

**任务清单**:

| 任务 | 文件 | 描述 | 预估时间 |
|------|------|------|---------|
| 12.1 | `one_of.rs` | 实现 `OneOfFunction`, `IfElseFunction` | 3h |
| 12.2 | `binaryzation.rs` | 实现 `BinaryzationFunction` (4 种实现) | 4h |
| 12.3 | `semi.rs` | 实现 `SemiFunction` | 2h |
| 12.4 | `balance_ternary.rs` | 实现 `BalanceTernaryzationFunction` | 2h |
| 12.5 | `inequality.rs` | 实现 `InequalityFunction` | 1.5h |
| 12.6 | `in_step_range.rs` | 实现 `InStepRangeFunction` | 2h |
| 12.7 | `satisfied_amount.rs` | 实现 `SatisfiedAmountFunction` | 2h |
| 12.8 | `same_as.rs` | 实现 `SameAsFunction` | 1h |
| 12.9 | `first.rs` | 实现 `FirstFunction` | 1.5h |
| 12.10 | `trigonometric.rs` | 实现 `SinFunction`, `CosFunction` | 2h |
| 12.11 | `mod_fn.rs` | 实现 `ModFunction` | 1.5h |
| 12.12 | `rounding.rs` | 实现 `RoundingFunction` | 1.5h |
| 12.13 | 测试 | 单元测试和集成测试 | 3h |

#### 7.3 总体时间估算

| 阶段 | 模块 | 预估时间 | 优先级 |
|------|------|---------|--------|
| 1 | 变量系统 | 2-3 天 | P0 |
| 2 | Token 系统 | 2-3 天 | P0 |
| 3 | 表达式平展系统 | 2-3 天 | P1 |
| 4 | 中间符号系统 (基础) | 3-4 天 | P1 |
| 5 | 约束系统 | 1-2 天 | P1 |
| 6 | 模型系统 | 3-4 天 | P1 |
| 7 | 中间模型层 | 2-3 天 | P2 |
| 8 | 求解器接口 | 3-5 天/求解器 | P3 |
| 9 | 回调模型层 | 2-3 天 | P2 |
| 10 | IIS 计算模块 | 3-4 天 | P2 |
| 11 | 启发式算法模块 | 4-5 天 | P3 |
| 12 | 新增函数符号 | 5-7 天 | P2 |
| **基础功能总计** | | **16-22 天** | |
| **完整功能总计** | | **32-45 天** | |

**优先级说明**:
- **P0 (核心基础)**: 变量系统、Token 系统 - 必须首先完成
- **P1 (核心功能)**: 平展系统、中间符号系统 (基础)、约束系统、模型系统 - 基本建模能力
- **P2 (增强功能)**: 中间模型层、回调模型层、IIS 计算、新增函数符号 - 高级功能
- **P3 (扩展功能)**: 求解器接口、启发式算法 - 可选扩展

#### 7.4 依赖关系图

```
ospf-rust-base (基础类型)
       ↓
ospf-rust-math (symbol 模块)
       ↓
ospf-rust-core
├── variable (阶段 1)
│      ↓
├── token (阶段 2) ──────────────────┐
│      ↓                             │
├── flatten (阶段 3)                 │
│      ↓                             │
├── symbol (阶段 4) ←────────────────┘
│      ↓
├── constraint (阶段 5)
│      ↓
├── model (阶段 6)
│      ↓
├── intermediate_model (阶段 7)
│      ↓
└── solver (阶段 8)
```

### Implementation Status Update (2026-03-23)
- Function symbol implementations under `src/symbol/functions/*` now use explicit type parameters (`IntermediateSymbol<f64>`, `FunctionSymbol<f64>`, `LinearIntermediateSymbol<f64>`).
- Dependency signatures are aligned to `HashSet<Arc<dyn IntermediateSymbol<f64>>>` to avoid implicit default-parameter usage.
- This is a signature-level alignment step; concrete function semantics remain `f64`-centric for now.
- Constraint wrapper now carries value type explicitly: `Constraint<V, P>` with `from: Option<Arc<dyn IntermediateSymbol<V>>>`.
- Mechanism-layer constraint aliases (`LinearConstraint<V>`, `QuadraticConstraint<V>`, symbolic variants) now preserve `V` end-to-end.
- Token layer safety hardening: removed all `unsafe transmute` conversion paths in `AnyVariable`.
- Conversions between model value type and bound metadata now use explicit `IntoValue<f64>` / `from_value` flow.
- Generic registration bounds were relaxed to avoid unnecessary `VT::Value -> V` coupling where only metadata conversion is needed.
- Token metadata is now value-type aware: `VariableData<V = f64>` stores typed bounds (`Option<V>`) instead of hard-coded `f64`.
- `AnyVariable<V>` and `Token<V>` now carry typed variable metadata end-to-end.
- Generic variable registration path is aligned to `VT::Value -> V` conversion, while legacy `VariableItem(f64)` ingress is explicitly converted via `V::from_value`.
- Follow-up correction: after `VariableData<V>` genericization, generic registration now consistently uses `VT::Value -> V`; only legacy `VariableItem` ingress keeps explicit `f64 -> V` conversion.
- Token module export cleanup: `VariableData` is now surfaced as `TokenVariableData` to avoid namespace collision with `variable::VariableData`.
- Objective layer genericization progressed: `SubObjective<V>` now uses typed weight `V` rather than fixed `f64`.
- Default unit-weight construction is trait-driven (`One::one()`), and total-weight aggregation is generic (`Zero + Add`).
- Build-config alignment: added optional `rand` dependency and `rand` feature mapping so heuristic random-path cfg gates match declared feature values.
- Legacy `VariableItem(f64)` ingestion now uses strict `f64 -> V` conversion with explicit error signaling on failure.
- `BasicModel`/`MetaModel` now expose configuration accessors for runtime model-policy management.
- Workspace build hygiene updated: resolver upgraded to `3`, and profile settings centralized at workspace root.
- Cross-crate cleanup completed: removed an unused compiled-term branch in math compile path, yielding warning-free `cargo check -p ospf-rust-core` under current workspace setup.

### Implementation Status Update (2026-03-23, concrete function non-f64 semantics)
- Concrete function symbols in `src/symbol/functions/*` now use generic value type `V` with default `V = f64`.
- Hard-coded constants in function-level linearization paths were replaced by trait-based numeric semantics (`Zero`, `One`, `Neg`).
- Former `f64`-fixed function parameters (`threshold`, `big_m`, bounds, step, divisor, tolerance, right-value, etc.) are now modeled as `V`.
- Function trait implementations were upgraded from explicit `f64` impl blocks to generic `V` impl blocks across all current function symbol files.
- Current bridge constraint: `register_tokens` still relies on `f64: IntoValue<V>` because variable metadata source values are presently `f64`-typed.
- To complete end-to-end non-`f64` runtime semantics, add broader `IntoValue` conversion coverage from variable source value types into model value type `V`.

## Design Delta (2026-03-24): FunctionSymbol Context Integration

- `IntermediateSymbol` now defines two extensible hooks:
  - `register_auxiliary_tokens(...)` for symbol-driven variable/token registration.
  - `evaluate_from_tokens(...)` for token-table based evaluation.
- Default evaluation path is now:
  1. try `evaluate_from_tokens(token_list)`
  2. fallback to legacy `prepare(values)`
  3. write through value cache
- `BasicModel::add_symbol(...)` now invokes `register_auxiliary_tokens(...)` and merges returned tokens into model storage with duplicate-id validation.
- If auxiliary tokens are added, `BasicModel` rebinds flatten/value/range contexts to a refreshed token snapshot.
- This keeps compatibility with existing `prepare(values)` implementations while enabling context-native semantics for function symbols.
