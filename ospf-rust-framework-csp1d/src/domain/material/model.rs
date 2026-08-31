//! CSP1D 材料与方案模型 / CSP1D material and plan models

use std::collections::{BTreeMap, btree_map::Entry};
use std::fmt::Debug;

use ospf_rust_core::solver::SolveValue;
use ospf_rust_core::solver::value::SolveValueConversionPolicy;
use ospf_rust_quantities::dimension::derived_quantity::{DerivedQuantity, QuantityDomain};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::scale::Scale;
use ospf_rust_quantities::unit::Unit;
use ospf_rust_quantities::unit::derived::Kilogram;
use ospf_rust_quantities::unit::CTUnit;

use crate::infrastructure::dto::{
    RenderCuttingPlanDTO, RenderCuttingPlanProductionDTO, RenderProductionType, RenderSchemaDTO,
};

include!("id.rs");

/// CSP1D 运行时物理量 / CSP1D runtime quantity
pub type Csp1dQuantity<V> = Quantity<V, Unit>;

/// 物理量算术策略 / Quantity arithmetic policy
pub trait QuantityArithmetic<V: SolveValue>: Debug + Send + Sync {
    /// 加法 / Add
    fn add(&self, lhs: Csp1dQuantity<V>, rhs: Csp1dQuantity<V>) -> Option<Csp1dQuantity<V>>;

    /// 减法 / Subtract
    fn subtract(&self, lhs: Csp1dQuantity<V>, rhs: Csp1dQuantity<V>) -> Option<Csp1dQuantity<V>>;

    /// 零值 / Zero
    fn zero(&self, unit: Unit) -> Option<Csp1dQuantity<V>>;

    /// 是否正数 / Whether positive
    fn is_positive(&self, quantity: &Csp1dQuantity<V>) -> bool {
        to_f64(&quantity.value).is_some_and(|value| value > 0.0)
    }

    /// 是否非负 / Whether non-negative
    fn is_non_negative(&self, quantity: &Csp1dQuantity<V>) -> bool {
        to_f64(&quantity.value).is_some_and(|value| value >= 0.0)
    }

    /// 是否为零 / Whether zero
    fn is_zero(&self, quantity: &Csp1dQuantity<V>) -> bool {
        to_f64(&quantity.value).is_some_and(|value| value == 0.0)
    }
}

/// 默认物理量算术策略 / Default quantity arithmetic policy
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultQuantityArithmetic;

impl DefaultQuantityArithmetic {
    /// 按数值样本解析默认策略 / Resolve default policy by numeric sample
    pub fn resolve_for<V: SolveValue>(_sample: &V) -> Self {
        Self
    }
}

impl<V: SolveValue> QuantityArithmetic<V> for DefaultQuantityArithmetic {
    fn add(&self, lhs: Csp1dQuantity<V>, rhs: Csp1dQuantity<V>) -> Option<Csp1dQuantity<V>> {
        if lhs.unit != rhs.unit {
            return None;
        }
        let lhs_value = to_f64(&lhs.value)?;
        let rhs_value = to_f64(&rhs.value)?;
        Some(Csp1dQuantity {
            value: from_f64(lhs_value + rhs_value)?,
            unit: lhs.unit,
        })
    }

    fn subtract(&self, lhs: Csp1dQuantity<V>, rhs: Csp1dQuantity<V>) -> Option<Csp1dQuantity<V>> {
        if lhs.unit != rhs.unit {
            return None;
        }
        let lhs_value = to_f64(&lhs.value)?;
        let rhs_value = to_f64(&rhs.value)?;
        Some(Csp1dQuantity {
            value: from_f64(lhs_value - rhs_value)?,
            unit: lhs.unit,
        })
    }

    fn zero(&self, unit: Unit) -> Option<Csp1dQuantity<V>> {
        Some(Csp1dQuantity {
            value: from_f64(0.0)?,
            unit,
        })
    }
}

/// 生产对象 / Production target
pub trait Production<V: SolveValue>: Debug + Send + Sync {
    /// 生产对象 ID / Production target id
    fn id(&self) -> &str;

    /// 幅宽列表 / Width list
    fn width(&self) -> &[Csp1dQuantity<V>];

    /// 长度 / Length
    fn length(&self) -> Option<&Csp1dQuantity<V>> {
        None
    }

    /// 单位重量 / Unit weight
    fn unit_weight(&self) -> Option<&Csp1dQuantity<V>> {
        None
    }
}

/// 卷数离散单位标记 / Discrete unit marker for roll count
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RollCountUnit;

/// 张数离散单位标记 / Discrete unit marker for sheet count
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SheetCountUnit;

/// 卷数离散单位 / Discrete roll-count unit
pub fn roll_count_unit() -> Unit {
    Unit::new_with_domain(
        "roll".to_string(),
        "roll".to_string(),
        DerivedQuantity::from_quantities_with_domain(
            "roll count".to_string(),
            Vec::new(),
            QuantityDomain::Discrete,
        ),
        Scale::new(),
        QuantityDomain::Discrete,
    )
}

/// 张数离散单位 / Discrete sheet-count unit
pub fn sheet_count_unit() -> Unit {
    Unit::new_with_domain(
        "sheet".to_string(),
        "sheet".to_string(),
        DerivedQuantity::from_quantities_with_domain(
            "sheet count".to_string(),
            Vec::new(),
            QuantityDomain::Discrete,
        ),
        Scale::new(),
        QuantityDomain::Discrete,
    )
}

/// 需求模式 / Demand mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DemandMode {
    /// 卷数 / Roll count
    Roll,
    /// 重量 / Weight
    Weight,
    /// 张数 / Sheet count
    Sheet,
}

/// 物理量区间 / Quantity range
#[derive(Debug, Clone)]
pub struct QuantityRange<V: SolveValue> {
    /// 下界 / Lower bound
    pub lower_bound: Csp1dQuantity<V>,
    /// 上界 / Upper bound
    pub upper_bound: Csp1dQuantity<V>,
    /// 下界是否包含 / Whether lower bound is inclusive
    pub lower_inclusive: bool,
    /// 上界是否包含 / Whether upper bound is inclusive
    pub upper_inclusive: bool,
}

impl<V: SolveValue> QuantityRange<V> {
    /// 创建闭区间 / Create closed range
    pub fn new(lower_bound: Csp1dQuantity<V>, upper_bound: Csp1dQuantity<V>) -> Option<Self> {
        Self::with_bounds(lower_bound, upper_bound, true, true)
    }

    /// 创建指定开闭边界的区间 / Create range with explicit boundary inclusiveness
    pub fn with_bounds(
        lower_bound: Csp1dQuantity<V>,
        upper_bound: Csp1dQuantity<V>,
        lower_inclusive: bool,
        upper_inclusive: bool,
    ) -> Option<Self> {
        if lower_bound.unit != upper_bound.unit {
            return None;
        }
        let lower_value = to_f64(&lower_bound.value)?;
        let upper_value = to_f64(&upper_bound.value)?;
        if lower_value > upper_value {
            return None;
        }
        if lower_value == upper_value && (!lower_inclusive || !upper_inclusive) {
            return None;
        }
        Some(Self {
            lower_bound,
            upper_bound,
            lower_inclusive,
            upper_inclusive,
        })
    }

    /// 判断值是否落在区间内 / Check whether the value is inside the range
    pub fn contains(&self, value: &Csp1dQuantity<V>) -> bool {
        if value.unit != self.lower_bound.unit || value.unit != self.upper_bound.unit {
            return false;
        }
        let Some(value) = to_f64(&value.value) else {
            return false;
        };
        let Some(lower) = to_f64(&self.lower_bound.value) else {
            return false;
        };
        let Some(upper) = to_f64(&self.upper_bound.value) else {
            return false;
        };
        let lower_ok = if self.lower_inclusive {
            value >= lower
        } else {
            value > lower
        };
        let upper_ok = if self.upper_inclusive {
            value <= upper
        } else {
            value < upper
        };
        lower_ok && upper_ok
    }
}

/// 幅宽范围 / Width range
#[derive(Debug, Clone)]
pub struct WidthRange<V: SolveValue> {
    /// 下界 / Lower bound
    pub lower_bound: Csp1dQuantity<V>,
    /// 上界 / Upper bound
    pub upper_bound: Csp1dQuantity<V>,
    /// 步进 / Step
    pub step: Csp1dQuantity<V>,
    /// 宽度值域 / Width value range
    pub width: QuantityRange<V>,
    widths: Vec<Csp1dQuantity<V>>,
}

impl<V: SolveValue> WidthRange<V> {
    /// 创建幅宽范围 / Create width range
    pub fn new(lower_bound: Csp1dQuantity<V>, upper_bound: Csp1dQuantity<V>) -> Self {
        let step = upper_bound.clone();
        let width = QuantityRange::new(lower_bound.clone(), upper_bound.clone())
            .expect("width range bounds must be valid");
        Self::with_step(width, step).expect("width range step unit must match bounds")
    }

    /// 按值域和步进创建幅宽范围 / Create width range from value range and step
    pub fn with_step(width: QuantityRange<V>, step: Csp1dQuantity<V>) -> Option<Self> {
        if width.lower_bound.unit.dimension() != step.unit.dimension()
            || width.upper_bound.unit.dimension() != step.unit.dimension()
        {
            return None;
        }
        Some(Self {
            widths: vec![width.lower_bound.clone(), width.upper_bound.clone()],
            lower_bound: width.lower_bound.clone(),
            upper_bound: width.upper_bound.clone(),
            step,
            width,
        })
    }

    /// 幅宽候选 / Width candidates
    pub fn widths(&self) -> &[Csp1dQuantity<V>] {
        &self.widths
    }

    /// 是否可切 / Whether the width can be cut
    pub fn can_cut(&self, width: &Csp1dQuantity<V>) -> bool {
        if width.unit != self.upper_bound.unit {
            return false;
        }
        width.value <= self.upper_bound.value
    }

    /// 是否包含宽度 / Whether the width is contained
    pub fn contains(&self, width: &Csp1dQuantity<V>) -> bool {
        self.width.contains(width)
    }
}

/// 物料 / Material
#[derive(Debug, Clone)]
pub struct Material<V: SolveValue> {
    /// 物料 ID / Material id
    pub id: MaterialId,
    /// 名称 / Name
    pub name: String,
    /// 幅宽范围 / Width range
    pub width_range: WidthRange<V>,
    /// 卷长 / Coil length
    pub length: Option<Csp1dQuantity<V>>,
    /// 单位重量 / Unit weight
    pub unit_weight: Option<Csp1dQuantity<V>>,
    /// 绑定设备 ID / Bound machine id
    pub machine_id: Option<MachineId>,
    /// 可用批次数 / Available batches
    pub available_batches: u64,
}

impl<V: SolveValue> Production<V> for Material<V> {
    fn id(&self) -> &str {
        &self.id
    }

    fn width(&self) -> &[Csp1dQuantity<V>] {
        self.width_range.widths()
    }

    fn length(&self) -> Option<&Csp1dQuantity<V>> {
        self.length.as_ref()
    }

    fn unit_weight(&self) -> Option<&Csp1dQuantity<V>> {
        self.unit_weight.as_ref()
    }
}

impl<V: SolveValue> Material<V> {
    /// 非幅宽基础可行性 / Basic feasibility without width check
    pub fn enabled_without_width_check(&self, plan: &CuttingPlan<V>) -> bool {
        if plan.material.id != self.id {
            return false;
        }
        if let (Some(material_machine), Some(plan_machine)) = (&self.machine_id, &plan.machine_id) {
            return material_machine == plan_machine;
        }
        true
    }

    /// 非幅宽和设备基础可行性 / Basic feasibility without width check and with machines
    pub fn enabled_without_width_check_with_machines(
        &self,
        plan: &CuttingPlan<V>,
        machines: &[Machine<V>],
    ) -> bool {
        if !self.enabled_without_width_check(plan) {
            return false;
        }
        if let Some(machine_id) = &plan.machine_id {
            if let Some(machine) = machines.iter().find(|machine| machine.id == *machine_id) {
                return machine.enabled(self);
            }
        }
        true
    }

    /// 基础可行性 / Basic feasibility
    pub fn enabled(&self, plan: &CuttingPlan<V>, machines: &[Machine<V>]) -> bool {
        if !self.enabled_without_width_check(plan) {
            return false;
        }
        let Some(used_width) = plan.used_width() else {
            return false;
        };
        if !self.width_range.can_cut(&used_width) {
            return false;
        }
        if let Some(machine_id) = &plan.machine_id {
            if let Some(machine) = machines.iter().find(|machine| machine.id == *machine_id) {
                return machine.enabled(self);
            }
        }
        true
    }
}

/// 产品 legacy 输入 / Product legacy input
#[derive(Debug, Clone)]
pub struct ProductLegacyInput<V: SolveValue> {
    /// 产品 ID / Product id
    pub id: ProductId,
    /// 名称 / Name
    pub name: String,
    /// 无单位幅宽列表 / Unitless width list
    pub width: Vec<V>,
    /// 无单位长度 / Unitless length
    pub length: Option<V>,
    /// 无单位单位重量 / Unitless unit weight
    pub unit_weight: Option<V>,
    /// 无单位重量 / Unitless weight
    pub weight: Option<V>,
    /// 无单位最大超产长度 / Unitless max over-produce length
    pub max_over_produce_length: Option<V>,
    /// 统一单位 / Shared unit
    pub unit: Unit,
}

/// 产品 / Product
#[derive(Debug, Clone)]
pub struct Product<V: SolveValue> {
    /// 产品 ID / Product id
    pub id: ProductId,
    /// 名称 / Name
    pub name: String,
    /// 可选幅宽 / Candidate widths
    pub width: Vec<Csp1dQuantity<V>>,
    /// 长度 / Length
    pub length: Option<Csp1dQuantity<V>>,
    /// 单位重量 / Unit weight
    pub unit_weight: Option<Csp1dQuantity<V>>,
    /// 显式重量 / Explicit weight
    pub weight: Option<Csp1dQuantity<V>>,
    /// 最大超产长度 / Maximum over-produce length
    pub max_over_produce_length: Option<Csp1dQuantity<V>>,
    /// 是否动态长度 / Whether length is dynamic
    pub dynamic_length: bool,
}

impl<V: SolveValue> Product<V> {
    /// legacy 输入转换 / Legacy input adapter
    pub fn legacy(input: ProductLegacyInput<V>) -> Self {
        let unit = input.unit;
        Self {
            id: input.id,
            name: input.name,
            width: input
                .width
                .into_iter()
                .map(|value| Csp1dQuantity {
                    value,
                    unit: unit.clone(),
                })
                .collect(),
            length: input.length.map(|value| Csp1dQuantity {
                value,
                unit: unit.clone(),
            }),
            unit_weight: input.unit_weight.map(|value| Csp1dQuantity {
                value,
                unit: unit.clone(),
            }),
            weight: input.weight.map(|value| Csp1dQuantity {
                value,
                unit: unit.clone(),
            }),
            max_over_produce_length: input.max_over_produce_length.map(|value| Csp1dQuantity {
                value,
                unit,
            }),
            dynamic_length: false,
        }
    }

    /// 创建动态长度产品 / Create dynamic-length product
    pub fn dynamic_length_of(
        id: impl Into<ProductId>,
        name: impl Into<String>,
        width: Vec<Csp1dQuantity<V>>,
    ) -> Self {
        Self::dynamic_length_of_with_unit_weight(id, name, width, None)
    }

    /// 创建动态长度产品（带单位重量） / Create dynamic-length product with unit weight
    pub fn dynamic_length_of_with_unit_weight(
        id: impl Into<ProductId>,
        name: impl Into<String>,
        width: Vec<Csp1dQuantity<V>>,
        unit_weight: Option<Csp1dQuantity<V>>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            width,
            length: None,
            unit_weight,
            weight: None,
            max_over_produce_length: None,
            dynamic_length: true,
        }
    }

    /// 最大幅宽 / Maximum width
    pub fn max_width(&self) -> Option<Csp1dQuantity<V>> {
        self.width
            .iter()
            .cloned()
            .max_by(|lhs, rhs| {
                let lhs_value = to_f64(&lhs.value).unwrap_or(f64::NEG_INFINITY);
                let rhs_value = to_f64(&rhs.value).unwrap_or(f64::NEG_INFINITY);
                lhs_value
                    .partial_cmp(&rhs_value)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// 指定宽度和长度下的重量 / Weight for the given width and length
    pub fn weight_for(
        &self,
        width: &Csp1dQuantity<V>,
        length: Option<&Csp1dQuantity<V>>,
    ) -> Option<Csp1dQuantity<V>> {
        let current_length = length.or(self.length.as_ref())?;
        let unit_weight = self.unit_weight.as_ref()?;
        if width.unit != current_length.unit {
            return None;
        }
        let width_value = to_f64(&width.value)?;
        let length_value = to_f64(&current_length.value)?;
        let unit_weight_value = to_f64(&unit_weight.value)?;
        let unit = product_weight_unit(&current_length.unit, &unit_weight.unit, &width.unit);
        Some(Csp1dQuantity {
            value: from_f64(width_value * length_value * unit_weight_value)?,
            unit,
        })
    }

    /// 产品重量 / Product weight
    pub fn weight(&self) -> Option<Csp1dQuantity<V>> {
        if let Some(weight) = &self.weight {
            return Some(weight.clone());
        }
        let width = self.max_width()?;
        self.weight_for(&width, self.length.as_ref())
    }
}

impl<V: SolveValue> Production<V> for Product<V> {
    fn id(&self) -> &str {
        &self.id
    }

    fn width(&self) -> &[Csp1dQuantity<V>] {
        &self.width
    }

    fn length(&self) -> Option<&Csp1dQuantity<V>> {
        self.length.as_ref()
    }

    fn unit_weight(&self) -> Option<&Csp1dQuantity<V>> {
        self.unit_weight.as_ref()
    }
}

/// 产品需求 / Product demand
#[derive(Debug, Clone)]
pub struct ProductDemand<V: SolveValue> {
    /// 产品 / Product
    pub product: Product<V>,
    /// 需求量 / Demand quantity
    pub quantity: Csp1dQuantity<V>,
    /// 需求模式 / Demand mode
    pub mode: Option<DemandMode>,
}

impl<V: SolveValue> ProductDemand<V> {
    /// 卷数需求 / Roll demand
    pub fn roll(product: Product<V>, quantity: Csp1dQuantity<V>) -> Self {
        Self {
            product,
            quantity,
            mode: Some(DemandMode::Roll),
        }
    }

    /// 重量需求 / Weight demand
    pub fn weight(product: Product<V>, quantity: Csp1dQuantity<V>) -> Self {
        Self {
            product,
            quantity,
            mode: Some(DemandMode::Weight),
        }
    }

    /// 张数需求 / Sheet demand
    pub fn sheet(product: Product<V>, quantity: Csp1dQuantity<V>) -> Self {
        Self {
            product,
            quantity,
            mode: Some(DemandMode::Sheet),
        }
    }

    /// legacy 卷数输入转换 / Legacy roll-amount input adapter
    pub fn legacy_roll(product: Product<V>, roll_amount: V) -> Self {
        Self::legacy_roll_with_unit(product, roll_amount, roll_count_unit())
    }

    /// legacy 卷数输入转换（指定单位） / Legacy roll-amount input adapter with unit
    pub fn legacy_roll_with_unit(
        product: Product<V>,
        roll_amount: V,
        unit: Unit,
    ) -> Self {
        Self::roll(product, Csp1dQuantity {
            value: roll_amount,
            unit,
        })
    }

    /// legacy 重量输入转换 / Legacy weight-amount input adapter
    pub fn legacy_weight(product: Product<V>, weight_amount: V) -> Self {
        Self::legacy_weight_with_unit(product, weight_amount, Kilogram::INSTANT.clone())
    }

    /// legacy 重量输入转换（指定单位） / Legacy weight-amount input adapter with unit
    pub fn legacy_weight_with_unit(
        product: Product<V>,
        weight_amount: V,
        unit: Unit,
    ) -> Self {
        Self::weight(product, Csp1dQuantity {
            value: weight_amount,
            unit,
        })
    }

    /// legacy 张数输入转换 / Legacy sheet-amount input adapter
    pub fn legacy_sheet(product: Product<V>, sheet_amount: V) -> Self {
        Self::legacy_sheet_with_unit(product, sheet_amount, sheet_count_unit())
    }

    /// legacy 张数输入转换（指定单位） / Legacy sheet-amount input adapter with unit
    pub fn legacy_sheet_with_unit(
        product: Product<V>,
        sheet_amount: V,
        unit: Unit,
    ) -> Self {
        Self::sheet(product, Csp1dQuantity {
            value: sheet_amount,
            unit,
        })
    }

    /// 是否离散需求 / Whether demand is discrete
    pub fn is_discrete(&self) -> bool {
        self.quantity.unit.domain() == QuantityDomain::Discrete
    }

    /// 是否连续需求 / Whether demand is continuous
    pub fn is_continuous(&self) -> bool {
        self.quantity.unit.domain() == QuantityDomain::Continuous
    }
}

/// 配规 / Costar
#[derive(Debug, Clone)]
pub struct Costar<V: SolveValue> {
    /// 配规 ID / Costar id
    pub id: CostarId,
    /// 名称 / Name
    pub name: String,
    /// 幅宽 / Width
    pub width: Vec<Csp1dQuantity<V>>,
    /// 长度 / Length
    pub length: Option<Csp1dQuantity<V>>,
    /// 单位重量 / Unit weight
    pub unit_weight: Option<Csp1dQuantity<V>>,
}

impl<V: SolveValue> Production<V> for Costar<V> {
    fn id(&self) -> &str {
        &self.id
    }

    fn width(&self) -> &[Csp1dQuantity<V>] {
        &self.width
    }

    fn length(&self) -> Option<&Csp1dQuantity<V>> {
        self.length.as_ref()
    }

    fn unit_weight(&self) -> Option<&Csp1dQuantity<V>> {
        self.unit_weight.as_ref()
    }
}

/// 设备 / Machine
#[derive(Debug, Clone)]
pub struct Machine<V: SolveValue> {
    /// 设备 ID / Machine id
    pub id: MachineId,
    /// 名称 / Name
    pub name: String,
    /// 最大批次数 / Maximum batch count
    pub max_batch_count: Option<u64>,
    /// 最大换料次数 / Maximum switch count
    pub max_switch_count: Option<u64>,
    /// 可用幅宽范围 / Available width range
    pub width_range: Option<WidthRange<V>>,
    /// 产能 / Capacity
    pub capacity: Option<Csp1dQuantity<V>>,
}

impl<V: SolveValue> Machine<V> {
    /// 是否可加工物料 / Whether the machine can process the material
    pub fn enabled(&self, material: &Material<V>) -> bool {
        let Some(width_range) = &self.width_range else {
            return true;
        };
        width_range.contains(&material.width_range.lower_bound)
            && width_range.contains(&material.width_range.upper_bound)
    }
}

/// 切割方案生产对象 / Cutting-plan production target
#[derive(Debug, Clone)]
pub enum CuttingPlanProduction<V: SolveValue> {
    /// 产品 / Product
    Product(Product<V>),
    /// 配规 / Costar
    Costar(Costar<V>),
}

impl<V: SolveValue> CuttingPlanProduction<V> {
    /// ID / Id
    pub fn id(&self) -> &str {
        match self {
            Self::Product(product) => &product.id,
            Self::Costar(costar) => &costar.id,
        }
    }

    /// 名称 / Name
    pub fn name(&self) -> &str {
        match self {
            Self::Product(product) => &product.name,
            Self::Costar(costar) => &costar.name,
        }
    }

    /// 长度 / Length
    pub fn length(&self) -> Option<&Csp1dQuantity<V>> {
        match self {
            Self::Product(product) => product.length.as_ref(),
            Self::Costar(costar) => costar.length.as_ref(),
        }
    }

    /// 渲染类型 / Render type
    pub fn render_type(&self) -> RenderProductionType {
        match self {
            Self::Product(_) => RenderProductionType::Product,
            Self::Costar(_) => RenderProductionType::Costar,
        }
    }
}

/// 切割方案切片 / Cutting plan slice
#[derive(Debug, Clone)]
pub struct CuttingPlanSlice<V: SolveValue> {
    /// 生产对象 / Production target
    pub production: CuttingPlanProduction<V>,
    /// 幅宽 / Width
    pub width: Csp1dQuantity<V>,
    /// 份数 / Amount
    pub amount: u64,
}

/// 需求贡献 / Demand contribution
#[derive(Debug, Clone)]
pub struct CuttingPlanDemandContribution<V: SolveValue> {
    /// 产品 / Product
    pub product: Product<V>,
    /// 贡献量 / Contribution quantity
    pub quantity: Csp1dQuantity<V>,
}

impl<V: SolveValue> CuttingPlanDemandContribution<V> {
    /// 按需求口径创建贡献 / Build contribution by demand unit
    pub fn from_demand(
        demand: &ProductDemand<V>,
        width: &Csp1dQuantity<V>,
        amount: u64,
        length: Option<&Csp1dQuantity<V>>,
    ) -> Self {
        Self::of(
            &demand.product,
            width,
            amount,
            &demand.quantity.unit,
            length,
        )
    }

    /// 按指定需求单位创建贡献 / Build contribution by a demand unit
    pub fn of(
        product: &Product<V>,
        width: &Csp1dQuantity<V>,
        amount: u64,
        demand_unit: &Unit,
        length: Option<&Csp1dQuantity<V>>,
    ) -> Self {
        Self {
            product: product.clone(),
            quantity: Self::quantity_of(product, width, amount, demand_unit, length),
        }
    }

    /// 计算切片贡献量 / Calculate slice contribution quantity
    pub fn quantity_of(
        product: &Product<V>,
        width: &Csp1dQuantity<V>,
        amount: u64,
        demand_unit: &Unit,
        length: Option<&Csp1dQuantity<V>>,
    ) -> Csp1dQuantity<V> {
        let contribution_length = length.or(product.length.as_ref());
        if let (Some(unit_weight), Some(current_length)) =
            (product.unit_weight.as_ref(), contribution_length)
        {
            if unit_weight.unit == *demand_unit {
                let contribution = to_f64(&width.value)
                    .zip(to_f64(&current_length.value))
                    .zip(to_f64(&unit_weight.value))
                    .and_then(|((width, length), unit_weight)| {
                        from_f64(width * length * unit_weight * amount as f64)
                    });
                if let Some(value) = contribution {
                    return Csp1dQuantity {
                        value,
                        unit: demand_unit.clone(),
                    };
                }
            }
        }
        Csp1dQuantity {
            value: from_f64(amount as f64).unwrap_or_else(|| width.value.clone()),
            unit: demand_unit.clone(),
        }
    }
}

/// 切割方案 / Cutting plan
#[derive(Debug, Clone)]
pub struct CuttingPlan<V: SolveValue> {
    /// 方案 ID / Plan id
    pub id: CuttingPlanId,
    /// 物料 / Material
    pub material: Material<V>,
    /// 设备 ID / Machine id
    pub machine_id: Option<MachineId>,
    /// 切片 / Slices
    pub slices: Vec<CuttingPlanSlice<V>>,
    /// 需求贡献 / Demand contributions
    pub demand_contributions: Vec<CuttingPlanDemandContribution<V>>,
    /// 产能消耗 / Capacity consumption
    pub capacity_consumption: Option<Csp1dQuantity<V>>,
}

impl<V: SolveValue> CuttingPlan<V> {
    /// 规范化 key / Canonical key
    pub fn canonical_key(&self) -> String {
        let slices = canonical_slice_keys(&self.slices).join(",");
        let demand_contributions = canonical_demand_contribution_keys(&self.demand_contributions)
            .join(",");
        let capacity_consumption = self
            .capacity_consumption
            .as_ref()
            .map(canonical_quantity_key)
            .unwrap_or_default();
        format!(
            "{}|{}|{}|{}|{}",
            self.material.id,
            self.machine_id
                .as_ref()
                .map(|id| id.to_string())
                .unwrap_or_default(),
            capacity_consumption,
            slices,
            demand_contributions,
        )
    }

    /// 已用幅宽 / Used width
    pub fn used_width(&self) -> Option<Csp1dQuantity<V>> {
        let first = self.slices.first()?;
        let unit = first.width.unit.clone();
        let mut total = 0.0;
        for slice in &self.slices {
            if slice.width.unit != unit {
                return None;
            }
            total += to_f64(&slice.width.value)? * slice.amount as f64;
        }
        Some(Csp1dQuantity {
            value: from_f64(total)?,
            unit,
        })
    }

    /// 余宽 / Rest width
    pub fn rest_width(&self) -> Option<Csp1dQuantity<V>> {
        let used = self.used_width()?;
        if self.material.width_range.upper_bound.unit != used.unit {
            return None;
        }
        let rest = to_f64(&self.material.width_range.upper_bound.value)? - to_f64(&used.value)?;
        Some(Csp1dQuantity {
            value: from_f64(rest)?,
            unit: used.unit,
        })
    }
}

fn canonical_slice_keys<V: SolveValue>(slices: &[CuttingPlanSlice<V>]) -> Vec<String> {
    let mut grouped: BTreeMap<(String, String, String), u64> = BTreeMap::new();
    for slice in slices {
        let key = (
            canonical_production_type(&slice.production).to_string(),
            slice.production.id().to_string(),
            canonical_quantity_key(&slice.width),
        );
        *grouped.entry(key).or_insert(0) += slice.amount;
    }
    grouped
        .into_iter()
        .map(|((production_type, production_id, width), amount)| {
            format!("{production_type}:{production_id}:{width}:{amount}")
        })
        .collect()
}

fn canonical_demand_contribution_keys<V: SolveValue>(
    contributions: &[CuttingPlanDemandContribution<V>],
) -> Vec<String> {
    let mut grouped: BTreeMap<(ProductId, String), V> = BTreeMap::new();
    for contribution in contributions {
        let key = (
            contribution.product.id.clone(),
            canonical_unit_key(&contribution.quantity.unit),
        );
        match grouped.entry(key) {
            Entry::Occupied(mut entry) => {
                if let (Some(lhs), Some(rhs)) =
                    (to_f64(entry.get()), to_f64(&contribution.quantity.value))
                {
                    if let Some(value) = from_f64(lhs + rhs) {
                        *entry.get_mut() = value;
                    }
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(contribution.quantity.value.clone());
            }
        }
    }
    grouped
        .into_iter()
        .map(|((product_id, unit), quantity)| {
            format!("{product_id}:{unit}:{quantity:?}")
        })
        .collect()
}

fn canonical_quantity_key<V: SolveValue>(quantity: &Csp1dQuantity<V>) -> String {
    format!("{:?}:{}", quantity.value, canonical_unit_key(&quantity.unit))
}

fn canonical_unit_key(unit: &Unit) -> String {
    unit.symbol().to_string()
}

fn canonical_production_type<V: SolveValue>(production: &CuttingPlanProduction<V>) -> &'static str {
    match production {
        CuttingPlanProduction::Product(_) => "product",
        CuttingPlanProduction::Costar(_) => "costar",
    }
}

fn product_weight_unit(length_unit: &Unit, unit_weight_unit: &Unit, width_unit: &Unit) -> Unit {
    let unit = ((length_unit * unit_weight_unit) * width_unit).build();
    if unit.dimension() == Kilogram::INSTANT.dimension() {
        Kilogram::INSTANT.clone()
    } else {
        unit
    }
}

/// shadow price key 族 / Shadow price key family
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Csp1dShadowPriceKey {
    /// 需求约束 / Demand constraint
    ProductDemand(ProductDemandShadowPriceKey),
    /// 物料使用约束 / Material usage constraint
    MaterialUsage(MaterialUsageShadowPriceKey),
    /// 设备批次约束 / Machine batch constraint
    MachineBatch(MachineBatchShadowPriceKey),
    /// 设备产能约束 / Machine capacity constraint
    MachineCapacity(MachineCapacityShadowPriceKey),
    /// Yield 超产上界 / Yield over-production bound
    YieldOverProductionBound(YieldOverProductionBoundShadowPriceKey),
}

/// 产品需求影子价格键 / Product demand shadow price key
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProductDemandShadowPriceKey {
    /// 产品 ID / Product id
    pub product_id: ProductId,
    /// 单位符号 / Unit symbol
    pub unit_symbol: String,
}

/// 物料使用影子价格键 / Material usage shadow price key
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MaterialUsageShadowPriceKey {
    /// 物料 ID / Material id
    pub material_id: MaterialId,
}

/// 设备批次影子价格键 / Machine batch shadow price key
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MachineBatchShadowPriceKey {
    /// 设备 ID / Machine id
    pub machine_id: MachineId,
}

/// 设备产能影子价格键 / Machine capacity shadow price key
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MachineCapacityShadowPriceKey {
    /// 设备 ID / Machine id
    pub machine_id: MachineId,
}

/// 产出超产上界影子价格键 / Yield over-production bound shadow price key
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YieldOverProductionBoundShadowPriceKey {
    /// 产品 ID / Product id
    pub product_id: ProductId,
    /// 单位符号 / Unit symbol
    pub unit_symbol: String,
}

/// shadow price map / Shadow price map
pub type ShadowPriceMap<V> = BTreeMap<Csp1dShadowPriceKey, V>;

/// shadow price 单位符号 / Shadow price unit symbol
pub fn shadow_price_unit_symbol(unit: &Unit) -> String {
    unit.symbol().to_string()
}

/// 影子价格 key 序列化 / Shadow price key serialization
pub fn shadow_price_key_to_string(key: &Csp1dShadowPriceKey) -> String {
    match key {
        Csp1dShadowPriceKey::ProductDemand(key) => {
            format!("product-demand:{}:{}", key.product_id, key.unit_symbol)
        }
        Csp1dShadowPriceKey::MaterialUsage(key) => {
            format!("material-usage:{}", key.material_id)
        }
        Csp1dShadowPriceKey::MachineBatch(key) => {
            format!("machine-batch:{}", key.machine_id)
        }
        Csp1dShadowPriceKey::MachineCapacity(key) => {
            format!("machine-capacity:{}", key.machine_id)
        }
        Csp1dShadowPriceKey::YieldOverProductionBound(key) => {
            format!("yield-over-production-bound:{}:{}", key.product_id, key.unit_symbol)
        }
    }
}

/// 影子价格 key 反序列化 / Shadow price key deserialization
pub fn shadow_price_key_from_string(value: &str) -> Option<Csp1dShadowPriceKey> {
    if value.contains('|') {
        return shadow_price_key_from_legacy_string(value);
    }
    let mut colon_parts = value.split(':');
    let family = colon_parts.next()?;
    match family {
        "product-demand" => Some(Csp1dShadowPriceKey::ProductDemand(
            ProductDemandShadowPriceKey {
                product_id: colon_parts.next()?.into(),
                unit_symbol: colon_parts.next()?.to_string(),
            },
        )),
        "material-usage" => Some(Csp1dShadowPriceKey::MaterialUsage(
            MaterialUsageShadowPriceKey {
                material_id: colon_parts.next()?.into(),
            },
        )),
        "machine-batch" => Some(Csp1dShadowPriceKey::MachineBatch(
            MachineBatchShadowPriceKey {
                machine_id: colon_parts.next()?.into(),
            },
        )),
        "machine-capacity" => Some(Csp1dShadowPriceKey::MachineCapacity(
            MachineCapacityShadowPriceKey {
                machine_id: colon_parts.next()?.into(),
            },
        )),
        "yield-over-production-bound" => Some(Csp1dShadowPriceKey::YieldOverProductionBound(
            YieldOverProductionBoundShadowPriceKey {
                product_id: colon_parts.next()?.into(),
                unit_symbol: colon_parts.next()?.to_string(),
            },
        )),
        _ => None,
    }
}

fn shadow_price_key_from_legacy_string(value: &str) -> Option<Csp1dShadowPriceKey> {
    let mut parts = value.split('|');
    let family = parts.next()?;
    match family {
        "productDemand" => Some(Csp1dShadowPriceKey::ProductDemand(
            ProductDemandShadowPriceKey {
                product_id: parts.next()?.into(),
                unit_symbol: parts.next()?.to_string(),
            },
        )),
        "materialUsage" => Some(Csp1dShadowPriceKey::MaterialUsage(
            MaterialUsageShadowPriceKey {
                material_id: parts.next()?.into(),
            },
        )),
        "machineBatch" => Some(Csp1dShadowPriceKey::MachineBatch(
            MachineBatchShadowPriceKey {
                machine_id: parts.next()?.into(),
            },
        )),
        "machineCapacity" => Some(Csp1dShadowPriceKey::MachineCapacity(
            MachineCapacityShadowPriceKey {
                machine_id: parts.next()?.into(),
            },
        )),
        "yieldOverProductionBound" => Some(Csp1dShadowPriceKey::YieldOverProductionBound(
            YieldOverProductionBoundShadowPriceKey {
                product_id: parts.next()?.into(),
                unit_symbol: parts.next()?.to_string(),
            },
        )),
        _ => None,
    }
}

/// 数值转 f64 / Convert value to f64
pub fn to_f64<V: SolveValue>(value: &V) -> Option<f64> {
    value
        .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
        .ok()
}

/// f64 转数值 / Convert f64 to value
pub fn from_f64<V: SolveValue>(value: f64) -> Option<V> {
    V::from_f64_with_policy(value, SolveValueConversionPolicy::AllowRounding).ok()
}

/// solver 值转领域数值 / Convert solver value to domain value
pub fn convert_solver_value<V: SolveValue>(value: f64) -> Option<V> {
    from_f64(value)
}

/// 宽度渲染行 / Render production row
pub fn to_render_production_dto<V: SolveValue>(
    production: &CuttingPlanProduction<V>,
    x: f64,
    width: Csp1dQuantity<V>,
    amount: u64,
) -> RenderCuttingPlanProductionDTO {
    let mut info = BTreeMap::new();
    info.insert("amount".to_string(), amount.to_string());
    RenderCuttingPlanProductionDTO {
        name: production.name().to_string(),
        x: x.to_string(),
        id: production.id().to_string(),
        width: format!("{:?}", width.value),
        unit_length: production.length().map(|length| format!("{:?}", length.value)),
        production_type: production.render_type(),
        amount,
        info,
    }
}

/// 渲染方案 / Render plan
pub fn render_cutting_plan<V: SolveValue>(
    plan: &CuttingPlan<V>,
    amount: u64,
) -> RenderCuttingPlanDTO {
    let mut cursor = 0.0;
    let productions = plan
        .slices
        .iter()
        .map(|slice| {
            let x = cursor;
            if let Some(width) = to_f64(&slice.width.value) {
                cursor += width;
            }
            to_render_production_dto(
                &slice.production,
                x,
                slice.width.clone(),
                slice.amount,
            )
        })
        .collect();
    let rest_width = plan.rest_width();
    let mut info = BTreeMap::new();
    info.insert("planId".to_string(), plan.id.to_string());
    if let Some(rest_width) = &rest_width {
        info.insert("restWidth".to_string(), format!("{:?}", rest_width.value));
    }
    RenderCuttingPlanDTO {
        group: vec![
            plan.material.name.clone(),
            plan.machine_id
                .as_ref()
                .map(|id| id.to_string())
                .unwrap_or_else(|| "unassigned-machine".to_string()),
        ],
        id: plan.id.to_string(),
        material_id: plan.material.id.to_string(),
        amount,
        productions,
        width: plan
            .used_width()
            .map(|width| format!("{:?}", width.value))
            .unwrap_or_else(|| "0".to_string()),
        standard_width: format!("{:?}", plan.material.width_range.upper_bound.value),
        rest_width: rest_width.map(|width| format!("{:?}", width.value)),
        info,
    }
}

/// 渲染根对象 / Render schema
pub fn render_schema<V: SolveValue>(plans: &[CuttingPlan<V>]) -> RenderSchemaDTO {
    RenderSchemaDTO {
        kpi: BTreeMap::new(),
        cutting_plans: plans.iter().map(|plan| render_cutting_plan(plan, 1)).collect(),
    }
}
