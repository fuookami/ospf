//! 主问题与定价共用的声明行及物理量系数。 / Declared rows and quantity coefficients shared by masters and pricing.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::unit::{Unit, UnitConversionValue};
use ospf_rust_core::solver::value::SolveValue;
use crate::domain::vrp::{Route, VehicleTypeId, VrptwInstance};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{NetworkArcId, NetworkNodeId, validate_linear_units};

/// 逐弧系数的完整业务身份；起终点贡献可按仓库身份判断。 / Complete arc identity; depot identities support initial and terminal contributions.
pub struct RoutePricingArc<'a, V: SolveValue + UnitConversionValue> {
    /// 当前实例 / Current instance.
    pub instance: &'a VrptwInstance<V>,
    /// 车辆类型 / Vehicle type.
    pub vehicle_type: &'a VehicleTypeId,
    /// 稳定弧身份 / Stable arc identity.
    pub arc: &'a NetworkArcId,
    /// 起点 / Origin.
    pub from: &'a NetworkNodeId,
    /// 终点 / Destination.
    pub to: &'a NetworkNodeId,
}

/// 同一提供器生成完整列及增量系数；默认不认证可加分解。 / One provider for whole-column and incremental coefficients; additive certification is opt-in.
pub trait RouteRowCoefficient<V>: Send + Sync
where
    V: SolveValue + UnitConversionValue,
{
    /// 完整路线用量，包含非可加及起终点贡献 / Complete usage, including non-additive and endpoint contributions.
    fn route_coefficient(
        &self,
        route: &Route<V>,
        instance: &VrptwInstance<V>,
    ) -> Result<Quantity<V, Unit>>;

    /// 增量用量；缺省为声明行单位下的零 / Incremental usage; absent means zero in the declared row unit.
    fn arc_coefficient(&self, _arc: &RoutePricingArc<'_, V>) -> Result<Option<Quantity<V, Unit>>> {
        Ok(None)
    }

    /// 业务认证逐弧和等于完整列系数，并保留支配合同 / Business certification of additive whole-column coefficients and dominance preservation.
    fn additive_decomposition_is_exact(&self) -> bool {
        false
    }
}

/// 一条有明确单位及边界的主问题扩展行。 / A master extension row with explicit unit and bounds.
#[derive(Clone)]
pub struct RoutePricingRow<V: SolveValue + UnitConversionValue> {
    /// 非空稳定行键 / Nonempty stable row key.
    pub id: String,
    /// 行用量单位 / Row usage unit.
    pub unit: Unit,
    /// 可选下界 / Optional lower bound.
    pub lower: Option<Quantity<V, Unit>>,
    /// 可选上界 / Optional upper bound.
    pub upper: Option<Quantity<V, Unit>>,
    /// 不持有模型注册态的共享系数提供器 / Shared coefficient provider without model-registration state.
    pub coefficient: Arc<dyn RouteRowCoefficient<V>>,
}

impl<V: SolveValue + UnitConversionValue> Debug for RoutePricingRow<V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoutePricingRow")
            .field("id", &self.id)
            .field("unit", &self.unit)
            .field("lower", &self.lower)
            .field("upper", &self.upper)
            .finish_non_exhaustive()
    }
}

/// 无模型状态的不可变声明目录，可安全供各分支模型共用。 / Immutable declaration catalog without model state, shareable across branch models.
#[derive(Debug, Clone)]
pub struct RoutePricingRows<V: SolveValue + UnitConversionValue> {
    rows: Vec<RoutePricingRow<V>>,
}

impl<V: SolveValue + UnitConversionValue> Default for RoutePricingRows<V> {
    fn default() -> Self {
        Self { rows: Vec::new() }
    }
}

impl<V: SolveValue + UnitConversionValue> RoutePricingRows<V> {
    /// 在模型修改前校验并归一化边界 / Validate and normalize bounds before modifying any model.
    pub fn new(mut rows: Vec<RoutePricingRow<V>>) -> Result<Self> {
        let mut keys = BTreeSet::new();
        for row in &mut rows {
            if row.id.trim().is_empty() || !keys.insert(row.id.clone())
                || row.lower.is_none() && row.upper.is_none()
            {
                return Err(NetworkSchedulingError::validation("扩展行键重复、为空或缺少边界 / duplicate, empty or unbounded extension row"));
            }
            validate_linear_units([&row.unit])?;
            for bound in [&mut row.lower, &mut row.upper].into_iter().flatten() {
                *bound = Quantity::new(normalize(bound, &row.unit)?, row.unit.clone());
            }
            if row.lower.as_ref().zip(row.upper.as_ref())
                .is_some_and(|(lower, upper)| lower.value > upper.value)
            {
                return Err(NetworkSchedulingError::validation("扩展行边界反向 / reversed extension-row bounds"));
            }
        }
        Ok(Self { rows })
    }

    /// 全部声明行；包含零系数行 / All declared rows, including zero-coefficient rows.
    pub fn rows(&self) -> &[RoutePricingRow<V>] {
        &self.rows
    }

    /// 仅全部提供器明确认证时允许增量系数支配 / Permit incremental dominance only when every provider explicitly certifies it.
    pub fn additive_decomposition_is_exact(&self) -> bool {
        self.rows.iter().all(|row| row.coefficient.additive_decomposition_is_exact())
    }

    /// 校验价格键完整覆盖声明域；零价格也必须显式存在 / Require exact declared-key coverage, including explicit zero prices.
    pub fn validate_dual_keys<T>(&self, prices: &BTreeMap<String, T>) -> Result<()> {
        if prices.len() != self.rows.len() || self.rows.iter().any(|row| !prices.contains_key(&row.id)) {
            return Err(NetworkSchedulingError::pricing("扩展对偶缺失或含未知行 / missing extension dual or unknown row"));
        }
        Ok(())
    }

    /// 使用声明单位提取逐弧系数，不经过 solver 数值转换 / Obtain arc coefficients in row units without solver-value conversion.
    pub fn arc_coefficients(&self, arc: &RoutePricingArc<'_, V>) -> Result<BTreeMap<String, V>> {
        self.rows.iter().map(|row| {
            let value = match row.coefficient.arc_coefficient(arc)? {
                Some(value) => normalize(&value, &row.unit)?,
                None => zero::<V>()?,
            };
            Ok((row.id.clone(), value))
        }).collect()
    }

    /// 完整列系数；认证分解另行逐弧复核 / Whole-column coefficients, independently replaying certified decompositions.
    pub fn route_coefficients(
        &self,
        route: &Route<V>,
        instance: &VrptwInstance<V>,
    ) -> Result<BTreeMap<String, V>> {
        let mut result = BTreeMap::new();
        for row in &self.rows {
            let value = normalize(&row.coefficient.route_coefficient(route, instance)?, &row.unit)?;
            if row.coefficient.additive_decomposition_is_exact() {
                let mut sum = zero::<V>()?;
                for (id, pair) in route.effective_arc_ids().iter().zip(route.stops.windows(2)) {
                    let arc = RoutePricingArc {
                        instance,
                        vehicle_type: &route.vehicle_type_id,
                        arc: id,
                        from: &pair[0].node_id,
                        to: &pair[1].node_id,
                    };
                    if let Some(term) = row.coefficient.arc_coefficient(&arc)? {
                        sum = sum + normalize(&term, &row.unit)?;
                    }
                }
                if sum != value {
                    return Err(NetworkSchedulingError::contract("认证扩展行分解与完整系数不一致 / certified extension-row decomposition disagrees with whole coefficient"));
                }
            }
            result.insert(row.id.clone(), value);
        }
        Ok(result)
    }
}

pub(crate) fn zero<V: UnitConversionValue>() -> Result<V> {
    V::from_decimal(&0.into()).ok_or_else(|| NetworkSchedulingError::validation("无法构造领域零值 / cannot construct domain zero"))
}

fn normalize<V: SolveValue + UnitConversionValue>(quantity: &Quantity<V, Unit>, unit: &Unit) -> Result<V> {
    validate_linear_units([&quantity.unit, unit])?;
    let value = quantity.to_unit(unit).map_err(|error| NetworkSchedulingError::Conversion {
        message: error.to_string(),
    })?.value;
    if value.clone() - value.clone() != zero::<V>()? {
        return Err(NetworkSchedulingError::validation("扩展行系数及边界必须有限 / extension-row coefficients and bounds must be finite"));
    }
    Ok(value)
}
