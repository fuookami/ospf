//! VRPTW 值对象与实例模型 / VRPTW value objects and instance model.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::Hasher;
use time::Duration;

use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use ospf_rust_framework_gantt_scheduling::infrastructure::{TimeRange, TimeWindow};
use ospf_rust_quantities::Quantity;
use ospf_rust_quantities::dimension::DerivedQuantity;
use ospf_rust_quantities::scale::Scale;
use ospf_rust_quantities::unit::{CTUnit, Kilogram, Meter, Unit, UnitConversionValue, UnitTrait};
use time::OffsetDateTime;

use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{NetworkArcId, NetworkNode, NetworkNodeId};

/// VRPTW 网络节点坐标 / VRPTW network-node coordinates.
#[derive(Debug, Clone)]
pub struct Coordinate<V: SolveValue + UnitConversionValue> {
    /// 带单位的坐标轴 / Unit-bearing coordinate axes.
    pub axes: BTreeMap<String, Quantity<V, Unit>>,
}

impl<V> Coordinate<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建坐标 / Create coordinates.
    pub fn new<I, K>(axes: I) -> Result<Self>
    where
        I: IntoIterator<Item = (K, Quantity<V, Unit>)>,
        K: Into<String>,
    {
        let axes = axes
            .into_iter()
            .map(|(name, value)| (name.into(), value))
            .collect::<BTreeMap<_, _>>();
        if axes.is_empty() || axes.keys().any(|axis| axis.is_empty()) {
            return Err(NetworkSchedulingError::validation(
                "坐标轴不能为空 / coordinate axes must be non-empty",
            ));
        }
        Ok(Self { axes })
    }

    /// 创建二维坐标 / Create a two-dimensional coordinate.
    pub fn xy(x: Quantity<V, Unit>, y: Quantity<V, Unit>) -> Result<Self> {
        Self::new([("x", x), ("y", y)])
    }

    /// 返回坐标轴名称 / Return coordinate-axis names.
    pub fn axis_names(&self) -> impl Iterator<Item = &String> {
        self.axes.keys()
    }

    /// 转换到目标距离单位 / Convert all axes to a target distance unit.
    pub fn to_unit(&self, target: &Unit) -> Result<BTreeMap<String, V>> {
        self.axes
            .iter()
            .map(|(axis, value)| {
                let value =
                    value
                        .to_unit(target)
                        .map_err(|error| NetworkSchedulingError::Conversion {
                            message: error.to_string(),
                        })?;
                Ok((axis.clone(), value.value))
            })
            .collect()
    }
}

/// 兼容 Kotlin 网络节点语义的 VRP 节点类型 / VRP node type compatible with Kotlin network-node semantics.
pub type VrpNode<V> = NetworkNode<Coordinate<V>>;

/// 客户稳定 ID / Stable customer ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CustomerId(String);

impl CustomerId {
    /// 创建客户 ID / Create a customer ID.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// 获取字符串 / Get the string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CustomerId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<&str> for CustomerId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for CustomerId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// 车辆类型稳定 ID / Stable vehicle-type ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VehicleTypeId(String);

impl VehicleTypeId {
    /// 创建车辆类型 ID / Create a vehicle-type ID.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// 获取字符串 / Get the string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for VehicleTypeId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl From<&str> for VehicleTypeId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for VehicleTypeId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// 闭区间服务时间窗 / Closed service-time window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ServiceTimeWindow {
    /// 最早服务开始时刻 / Earliest service-start instant.
    pub ready_time: OffsetDateTime,
    /// 最晚服务开始时刻；等号可行 / Latest service-start instant; equality is feasible.
    pub due_time: OffsetDateTime,
}

impl ServiceTimeWindow {
    /// 创建闭区间时间窗 / Create a closed time window.
    pub fn new(ready_time: OffsetDateTime, due_time: OffsetDateTime) -> Result<Self> {
        if due_time < ready_time {
            return Err(NetworkSchedulingError::validation(
                "截止时刻早于起始时刻 / due time is before ready time",
            ));
        }
        Ok(Self {
            ready_time,
            due_time,
        })
    }

    /// `new` 的显式 try 别名 / Explicit try alias for `new`.
    pub fn try_new(ready_time: OffsetDateTime, due_time: OffsetDateTime) -> Result<Self> {
        Self::new(ready_time, due_time)
    }

    /// 判断服务开始是否落在闭区间 / Check whether a service start is in the closed interval.
    pub fn contains(&self, instant: OffsetDateTime) -> bool {
        self.ready_time <= instant && instant <= self.due_time
    }
}

/// 非负行驶时间 / Non-negative travel time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TravelTime {
    /// 行驶持续时间 / Travel duration.
    pub duration: Duration,
}

impl TravelTime {
    /// 创建行驶时间 / Create a travel time.
    pub fn new(duration: Duration) -> Result<Self> {
        if duration < Duration::ZERO {
            return Err(NetworkSchedulingError::validation(
                "行驶时间不能为负 / travel time cannot be negative",
            ));
        }
        Ok(Self { duration })
    }
}

/// VRPTW 基础网络弧 / Base-network arc for VRPTW.
#[derive(Debug, Clone)]
pub struct VrptwArc<V: SolveValue + UnitConversionValue> {
    /// 稳定弧 ID / Stable arc ID.
    pub id: NetworkArcId,
    /// 起点 / Origin.
    pub from: NetworkNodeId,
    /// 终点 / Destination.
    pub to: NetworkNodeId,
    /// 弧距离 / Arc distance.
    pub distance: Quantity<V, Unit>,
    /// 弧行驶时间 / Arc travel time.
    pub travel_time: TravelTime,
    /// 弧成本 / Arc cost.
    pub cost: Quantity<V, Unit>,
    /// 是否允许该弧 / Whether this arc is feasible.
    pub feasible: bool,
}

impl<V> VrptwArc<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建基础网络弧 / Create a base-network arc.
    pub fn new(
        id: impl Into<NetworkArcId>,
        from: impl Into<NetworkNodeId>,
        to: impl Into<NetworkNodeId>,
        distance: Quantity<V, Unit>,
        travel_time: TravelTime,
        cost: Quantity<V, Unit>,
    ) -> Result<Self> {
        let from = from.into();
        let to = to.into();
        let id = id.into();
        if from == to {
            return Err(NetworkSchedulingError::validation(
                "基础网络弧不允许自环 / base-network arcs cannot be self-loops",
            ));
        }
        if id.as_str().is_empty() {
            return Err(NetworkSchedulingError::validation(
                "基础网络弧 ID 不能为空 / base-network arc ID cannot be empty",
            ));
        }
        let zero = V::from_f64_with_policy(0.0, SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        if distance.value < zero {
            return Err(NetworkSchedulingError::validation(
                "基础网络弧距离不能为负 / base-network arc distance cannot be negative",
            ));
        }
        Ok(Self {
            id,
            from,
            to,
            distance,
            travel_time,
            cost,
            feasible: true,
        })
    }

    /// 设置弧可行性 / Set arc feasibility.
    pub fn with_feasibility(mut self, feasible: bool) -> Self {
        self.feasible = feasible;
        self
    }
}

/// VRPTW 客户 / VRPTW customer.
#[derive(Debug, Clone)]
pub struct Customer<V: SolveValue + UnitConversionValue> {
    /// 客户 ID / Customer ID.
    pub id: CustomerId,
    /// 网络节点 / Network node.
    pub node: VrpNode<V>,
    /// 非负需求 / Non-negative demand.
    pub demand: Quantity<V, Unit>,
    /// 服务时间窗 / Service-time window.
    pub time_window: ServiceTimeWindow,
    /// 服务持续时间 / Service duration.
    pub service_time: Duration,
}

impl<V> Customer<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建客户并执行基础校验 / Create a customer with basic validation.
    pub fn new(
        id: impl Into<CustomerId>,
        node: VrpNode<V>,
        demand: Quantity<V, Unit>,
        time_window: ServiceTimeWindow,
        service_time: Duration,
    ) -> Result<Self> {
        let id = id.into();
        if id.as_str().is_empty() {
            return Err(NetworkSchedulingError::validation(
                "客户 ID 不能为空 / customer ID cannot be empty",
            ));
        }
        if demand.value
            < V::from_f64_with_policy(0.0, SolveValueConversionPolicy::AllowRounding).map_err(
                |error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                },
            )?
        {
            return Err(NetworkSchedulingError::validation(
                "客户需求不能为负 / customer demand cannot be negative",
            ));
        }
        Ok(Self {
            id,
            node,
            demand,
            time_window,
            service_time,
        })
    }
}

/// 起止仓库 / Start or end depot.
#[derive(Debug, Clone)]
pub struct Depot<V: SolveValue + UnitConversionValue> {
    /// 网络节点 / Network node.
    pub node: VrpNode<V>,
    /// 仓库开放时间窗 / Depot opening time window.
    pub time_window: ServiceTimeWindow,
}

/// 有限车队车辆类型 / Vehicle type in a finite fleet.
#[derive(Debug, Clone)]
pub struct VehicleType<V: SolveValue + UnitConversionValue> {
    /// 车辆类型 ID / Vehicle-type ID.
    pub id: VehicleTypeId,
    /// 单车容量 / Per-vehicle capacity.
    pub capacity: Quantity<V, Unit>,
    /// 固定使用成本 / Fixed usage cost.
    pub fixed_cost: Quantity<V, Unit>,
    /// 可用车辆数 / Available vehicle count.
    pub amount: usize,
}

impl<V> VehicleType<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建车辆类型并执行基础校验 / Create a vehicle type with basic validation.
    pub fn new(
        id: impl Into<VehicleTypeId>,
        capacity: Quantity<V, Unit>,
        fixed_cost: Quantity<V, Unit>,
        amount: usize,
    ) -> Result<Self> {
        let id = id.into();
        let zero = V::from_f64_with_policy(0.0, SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        if id.as_str().is_empty()
            || capacity.value <= zero
            || fixed_cost.value < zero
            || amount == 0
        {
            return Err(NetworkSchedulingError::validation(
                "车辆类型参数无效 / vehicle-type parameters are invalid",
            ));
        }
        Ok(Self {
            id,
            capacity,
            fixed_cost,
            amount,
        })
    }
}

/// VRPTW 单位口径 / VRPTW unit contract.
#[derive(Debug, Clone)]
pub struct VrptwUnits {
    /// 距离单位 / Distance unit.
    pub distance_unit: Unit,
    /// 负载单位 / Load unit.
    pub load_unit: Unit,
    /// 成本单位 / Cost unit.
    pub cost_unit: Unit,
}

impl VrptwUnits {
    /// 创建单位口径并校验线性距离单位 / Create and validate a unit contract with a linear distance unit.
    pub fn new(distance_unit: Unit, load_unit: Unit, cost_unit: Unit) -> Result<Self> {
        if !distance_unit.is_linear() || distance_unit.dimension_symbol() != "L" {
            return Err(NetworkSchedulingError::validation(
                "距离必须是线性长度单位 / distance must be a linear length unit",
            ));
        }
        if !load_unit.is_linear() || !cost_unit.is_linear() {
            return Err(NetworkSchedulingError::validation(
                "负载和成本单位必须是线性单位 / load and cost units must be linear",
            ));
        }
        Ok(Self {
            distance_unit,
            load_unit,
            cost_unit,
        })
    }

    /// 默认米、千克和无量纲成本单位 / Default meter, kilogram, and dimensionless cost units.
    #[allow(clippy::borrow_interior_mutable_const)]
    pub fn default_units() -> Self {
        let cost_unit = Unit::new(
            "dimensionless".to_owned(),
            "1".to_owned(),
            DerivedQuantity::none("dimensionless".to_owned()),
            Scale::new(),
        );
        Self {
            distance_unit: Meter::INSTANT.clone(),
            load_unit: Kilogram::INSTANT.clone(),
            cost_unit,
        }
    }
}

impl Default for VrptwUnits {
    fn default() -> Self {
        Self::default_units()
    }
}

/// 五类统一 VRPTW 容差 / Five unified VRPTW tolerances.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VrptwTolerances {
    /// 可行性容差 / Feasibility tolerance.
    pub feasibility: f64,
    /// 定价容差 / Pricing tolerance.
    pub pricing: f64,
    /// 整数性容差 / Integrality tolerance.
    pub integrality: f64,
    /// 成本复核容差 / Cost-validation tolerance.
    pub cost_validation: f64,
    /// 相对 gap 容差 / Relative-gap tolerance.
    pub relative_gap: f64,
}

impl Default for VrptwTolerances {
    fn default() -> Self {
        Self {
            feasibility: 1e-7,
            pricing: 1e-8,
            integrality: 1e-7,
            cost_validation: 1e-7,
            relative_gap: 1e-4,
        }
    }
}

impl VrptwTolerances {
    /// 创建自定义容差 / Create custom tolerances.
    pub fn new(
        feasibility: f64,
        pricing: f64,
        integrality: f64,
        cost_validation: f64,
        relative_gap: f64,
    ) -> Result<Self> {
        let values = [
            feasibility,
            pricing,
            integrality,
            cost_validation,
            relative_gap,
        ];
        if values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(NetworkSchedulingError::validation(
                "容差必须是有限非负数 / tolerances must be finite and non-negative",
            ));
        }
        Ok(Self {
            feasibility,
            pricing,
            integrality,
            cost_validation,
            relative_gap,
        })
    }
}

/// 路线停靠点的完整资源状态 / Complete resource state at a route stop.
#[derive(Debug, Clone)]
pub struct RouteStop<V: SolveValue + UnitConversionValue> {
    /// 节点 ID / Node ID.
    pub node_id: NetworkNodeId,
    /// 客户 ID；仓库处为空 / Customer ID; absent at depots.
    pub customer_id: Option<CustomerId>,
    /// 到达时刻 / Arrival instant.
    pub arrival: OffsetDateTime,
    /// 服务开始时刻 / Service-start instant.
    pub service_start: OffsetDateTime,
    /// 离开时刻 / Departure instant.
    pub departure: OffsetDateTime,
    /// 累计负载 / Accumulated load.
    pub accumulated_load: Quantity<V, Unit>,
}

/// 单车 elementary 路线 / Elementary single-vehicle route.
#[derive(Debug, Clone)]
pub struct Route<V: SolveValue + UnitConversionValue> {
    /// 车辆类型 / Vehicle type.
    pub vehicle_type_id: VehicleTypeId,
    /// 有序停靠点 / Ordered stops.
    pub stops: Vec<RouteStop<V>>,
    /// 可选的有序稳定弧 ID；为空时由节点对推导 / Optional ordered arc IDs; node pairs are used when empty.
    pub arc_ids: Vec<NetworkArcId>,
    /// 总距离 / Total distance.
    pub distance: Quantity<V, Unit>,
    /// 总成本 / Total cost.
    pub cost: Quantity<V, Unit>,
}

impl<V> Route<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建路线值对象 / Create a route value object.
    pub fn new(
        vehicle_type_id: impl Into<VehicleTypeId>,
        stops: Vec<RouteStop<V>>,
        distance: Quantity<V, Unit>,
        cost: Quantity<V, Unit>,
    ) -> Result<Self> {
        Self::with_arc_ids(vehicle_type_id, stops, Vec::new(), distance, cost)
    }

    /// 创建带稳定弧序列的路线 / Create a route with a stable arc sequence.
    pub fn with_arc_ids(
        vehicle_type_id: impl Into<VehicleTypeId>,
        stops: Vec<RouteStop<V>>,
        arc_ids: Vec<NetworkArcId>,
        distance: Quantity<V, Unit>,
        cost: Quantity<V, Unit>,
    ) -> Result<Self> {
        let zero = V::from_f64_with_policy(0.0, SolveValueConversionPolicy::AllowRounding)
            .map_err(|error| NetworkSchedulingError::Conversion {
                message: error.to_string(),
            })?;
        if stops.len() < 2 || distance.value < zero || cost.value < zero {
            return Err(NetworkSchedulingError::validation(
                "路线、距离或成本无效 / route, distance, or cost is invalid",
            ));
        }
        if !arc_ids.is_empty() && arc_ids.len() != stops.len() - 1 {
            return Err(NetworkSchedulingError::validation(
                "弧序列长度必须等于停靠点数减一 / arc sequence length must equal stop count minus one",
            ));
        }
        Ok(Self {
            vehicle_type_id: vehicle_type_id.into(),
            stops,
            arc_ids,
            distance,
            cost,
        })
    }

    /// 获取路线签名；有弧 ID 时保留平行弧区别 / Get route signature, preserving parallel-arc identity when arc IDs exist.
    pub fn signature(&self) -> String {
        let mut signature = String::from("route-v1;");
        append_signature_field(&mut signature, self.vehicle_type_id.as_str());
        for arc_id in self.effective_arc_ids() {
            append_signature_field(&mut signature, arc_id.as_str());
        }
        signature
    }

    /// 返回路线访问的客户集合 / Return visited customers.
    pub fn customer_ids(&self) -> HashSet<CustomerId> {
        self.stops
            .iter()
            .filter_map(|stop| stop.customer_id.clone())
            .collect()
    }

    /// 判断路线是否使用指定弧 / Check whether the route uses an arc.
    pub fn uses_arc(&self, arc_id: &NetworkArcId) -> bool {
        self.effective_arc_ids()
            .iter()
            .any(|candidate| candidate == arc_id)
    }

    /// 返回节点对推导的弧签名 / Return node-pair-derived arc IDs.
    pub fn effective_arc_ids(&self) -> Vec<NetworkArcId> {
        if !self.arc_ids.is_empty() {
            return self.arc_ids.clone();
        }
        self.stops
            .windows(2)
            .map(|pair| default_arc_id(&pair[0].node_id, &pair[1].node_id))
            .collect()
    }
}

fn append_signature_field(signature: &mut String, value: &str) {
    signature.push_str(&value.len().to_string());
    signature.push(':');
    signature.push_str(value);
    signature.push(';');
}

/// 由端点生成默认弧 ID / Build a default arc ID from endpoints.
pub fn default_arc_id(from: &NetworkNodeId, to: &NetworkNodeId) -> NetworkArcId {
    let mut value = String::from("default-arc-v2;");
    append_signature_field(&mut value, from.as_str());
    append_signature_field(&mut value, to.as_str());
    NetworkArcId::new(value)
}

/// VRPTW 不可变实例 / Immutable VRPTW instance.
#[derive(Debug, Clone)]
pub struct VrptwInstance<V: SolveValue + UnitConversionValue> {
    /// 实例名称 / Instance name.
    pub name: String,
    /// 起始仓库 / Start depot.
    pub start_depot: Depot<V>,
    /// 结束仓库 / End depot.
    pub end_depot: Depot<V>,
    /// 客户快照 / Customer snapshot.
    pub customers: Vec<Customer<V>>,
    /// 车辆类型快照 / Vehicle-type snapshot.
    pub vehicle_types: Vec<VehicleType<V>>,
    /// 可选基础网络弧目录；为空时使用策略生成隐式完全图 / Optional base-arc catalog; an empty catalog uses the policy-generated complete graph.
    pub arcs: Vec<VrptwArc<V>>,
    /// 业务绝对时间轴 / Business absolute time axis.
    pub scheduling_window: TimeWindow<V>,
    /// 单位口径 / Unit contract.
    pub units: VrptwUnits,
    /// 算法容差 / Algorithm tolerances.
    pub tolerances: VrptwTolerances,
    /// 客户 ID 索引 / Customer ID index.
    pub customer_by_id: HashMap<CustomerId, usize>,
    /// 车辆类型 ID 索引 / Vehicle-type ID index.
    pub vehicle_type_by_id: HashMap<VehicleTypeId, usize>,
    /// 稳定实例身份；克隆保留身份，重新创建实例获得新身份 / Stable instance identity; clones preserve it and new instances get a new identity.
    pub(crate) instance_identity: u64,
}

impl<V> VrptwInstance<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建并完整校验 VRPTW 实例 / Create and fully validate a VRPTW instance.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: impl Into<String>,
        start_depot: Depot<V>,
        end_depot: Depot<V>,
        customers: Vec<Customer<V>>,
        vehicle_types: Vec<VehicleType<V>>,
        scheduling_window: TimeWindow<V>,
        units: VrptwUnits,
        tolerances: VrptwTolerances,
    ) -> Result<Self> {
        crate::domain::vrp::VrptwValidator::create(
            name.into(),
            start_depot,
            end_depot,
            customers,
            vehicle_types,
            scheduling_window,
            units,
            tolerances,
        )
    }

    /// 使用显式基础网络弧创建 VRPTW 实例 / Create a VRPTW instance with explicit base-network arcs.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_arcs(
        name: impl Into<String>,
        start_depot: Depot<V>,
        end_depot: Depot<V>,
        customers: Vec<Customer<V>>,
        vehicle_types: Vec<VehicleType<V>>,
        scheduling_window: TimeWindow<V>,
        units: VrptwUnits,
        tolerances: VrptwTolerances,
        arcs: Vec<VrptwArc<V>>,
    ) -> Result<Self> {
        crate::domain::vrp::VrptwValidator::create_with_arcs(
            name.into(),
            start_depot,
            end_depot,
            customers,
            vehicle_types,
            scheduling_window,
            units,
            tolerances,
            arcs,
        )
    }

    /// 通过客户 ID 获取客户 / Get a customer by ID.
    pub fn customer(&self, id: &CustomerId) -> Option<&Customer<V>> {
        self.customer_by_id
            .get(id)
            .and_then(|index| self.customers.get(*index))
    }

    /// 通过车辆类型 ID 获取车辆类型 / Get a vehicle type by ID.
    pub fn vehicle_type(&self, id: &VehicleTypeId) -> Option<&VehicleType<V>> {
        self.vehicle_type_by_id
            .get(id)
            .and_then(|index| self.vehicle_types.get(*index))
    }

    /// 获取稳定实例身份 / Get the stable instance identity.
    pub fn instance_identity(&self) -> u64 {
        self.instance_identity
    }

    /// 获取用于绑定预构建图的内容快照指纹 / Get a content fingerprint for binding prebuilt graphs.
    pub(crate) fn pricing_fingerprint(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        hasher.write(format!("{self:?}").as_bytes());
        hasher.finish()
    }

    /// 获取任意节点的时间窗 / Get the time window of any instance node.
    pub fn time_window_of(&self, node: &NetworkNodeId) -> Option<ServiceTimeWindow> {
        if node == &self.start_depot.node.id {
            return Some(self.start_depot.time_window);
        }
        if node == &self.end_depot.node.id {
            return Some(self.end_depot.time_window);
        }
        self.customers
            .iter()
            .find(|customer| customer.node.id == *node)
            .map(|customer| customer.time_window)
    }

    /// 获取任意节点服务时长 / Get service duration of an instance node.
    pub fn service_time_of(&self, node: &NetworkNodeId) -> Duration {
        self.customers
            .iter()
            .find(|customer| customer.node.id == *node)
            .map(|customer| customer.service_time)
            .unwrap_or(Duration::ZERO)
    }

    /// 获取任意节点需求 / Get demand of an instance node.
    pub fn demand_of(&self, node: &NetworkNodeId) -> Option<&Quantity<V, Unit>> {
        self.customers
            .iter()
            .find(|customer| customer.node.id == *node)
            .map(|customer| &customer.demand)
    }

    /// 按弧 ID 获取基础网络弧 / Get a base-network arc by stable ID.
    pub fn arc(&self, id: &NetworkArcId) -> Option<&VrptwArc<V>> {
        self.arcs.iter().find(|arc| &arc.id == id)
    }

    /// 按弧 ID 或端点获取基础网络弧 / Get a base-network arc by ID or endpoint pair.
    pub fn arc_for_route(
        &self,
        id: &NetworkArcId,
        from: &NetworkNodeId,
        to: &NetworkNodeId,
    ) -> Option<&VrptwArc<V>> {
        if let Some(arc) = self.arc(id) {
            return (&arc.from == from && &arc.to == to).then_some(arc);
        }
        if id != &default_arc_id(from, to) {
            return None;
        }
        let matches = self
            .arcs
            .iter()
            .filter(|arc| &arc.from == from && &arc.to == to)
            .collect::<Vec<_>>();
        if matches.len() == 1 {
            matches.into_iter().next()
        } else {
            None
        }
    }

    /// 校验路线弧身份与端点；隐式完全图只接受默认弧 ID。
    /// Validate route-arc identity and endpoints; an implicit complete graph accepts only default arc IDs.
    pub fn validate_route_arc_ids(&self, route: &Route<V>) -> Result<()> {
        let arc_ids = route.effective_arc_ids();
        if arc_ids.len() != route.stops.len().saturating_sub(1) {
            return Err(NetworkSchedulingError::contract(
                "路线弧索引长度不一致 / route arc-index length is inconsistent",
            ));
        }
        for (index, pair) in route.stops.windows(2).enumerate() {
            let arc_id = &arc_ids[index];
            if self.arcs.is_empty() {
                if arc_id != &default_arc_id(&pair[0].node_id, &pair[1].node_id) {
                    return Err(NetworkSchedulingError::validation(
                        "隐式完全图路线只能使用端点派生弧 ID / implicit complete-graph routes may only use endpoint-derived arc IDs",
                    ));
                }
            } else if self
                .arc_for_route(arc_id, &pair[0].node_id, &pair[1].node_id)
                .is_none()
            {
                return Err(NetworkSchedulingError::validation(
                    "路线引用不存在或有歧义的基础网络弧 / route references a missing or ambiguous base-network arc",
                ));
            }
        }
        Ok(())
    }
}

/// 已校验路线集合 / Validated route set.
#[derive(Debug, Clone)]
pub struct VrptwSolution<V: SolveValue + UnitConversionValue> {
    /// 路线快照 / Route snapshot.
    pub routes: Vec<Route<V>>,
    /// 总距离 / Total distance.
    pub total_distance: Quantity<V, Unit>,
    /// 总成本 / Total cost.
    pub total_cost: Quantity<V, Unit>,
}

/// 创建业务时间轴的辅助函数 / Helper for creating the business time axis.
pub fn scheduling_window<V>(window: TimeRange, interval: Duration) -> Result<TimeWindow<V>>
where
    V: SolveValue,
{
    TimeWindow::try_new(
        window,
        true,
        ospf_rust_framework_gantt_scheduling::infrastructure::DurationUnit::Seconds,
        Duration::ZERO,
        interval,
    )
    .map_err(NetworkSchedulingError::dependency)
}

/// 为测试和示例提供二维节点构造 / Construct a two-dimensional node for tests and examples.
pub fn coordinate_node<V>(
    id: impl Into<NetworkNodeId>,
    x: V,
    y: V,
    unit: Unit,
) -> Result<VrpNode<V>>
where
    V: SolveValue + UnitConversionValue,
{
    Ok(NetworkNode::new(
        id,
        Coordinate::xy(Quantity::new(x, unit.clone()), Quantity::new(y, unit))?,
    ))
}
