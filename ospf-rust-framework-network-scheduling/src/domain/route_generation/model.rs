//! ESPPRC 定价模型 / ESPPRC pricing models.

use std::sync::Arc;
use std::time::Instant;

use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_core::solver::{CancellationOrigin, CancellationRecord, SolveHandle};
use ospf_rust_quantities::unit::UnitConversionValue;

use crate::domain::vrp::Route;
use crate::domain::vrp::{BranchMask, PricingDuals, VehicleTypeId, VrptwInstance};

/// 定价取消令牌 / Pricing cancellation token.
#[derive(Debug, Clone)]
pub struct CancellationToken {
    handle: Arc<SolveHandle>,
}

impl CancellationToken {
    /// 创建令牌 / Create a token.
    pub fn new() -> Self {
        Self {
            handle: Arc::new(SolveHandle::new()),
        }
    }

    /// 请求用户取消 / Request user cancellation.
    pub fn cancel(&self) -> bool {
        self.cancel_with_origin(CancellationOrigin::User)
    }

    /// 请求带来源的取消 / Request cancellation with a structured origin.
    pub fn cancel_with_origin<O>(&self, origin: O) -> bool
    where
        O: Into<CancellationOrigin>,
    {
        self.handle.cancel(origin)
    }

    /// 检查是否已取消 / Check whether cancellation was requested.
    pub fn is_cancelled(&self) -> bool {
        self.handle.is_cancelled()
    }

    /// 获取底层统一求解句柄 / Get the underlying unified solve handle.
    pub fn handle(&self) -> SolveHandle {
        (*self.handle).clone()
    }

    /// 获取结构化取消事实 / Get the structured cancellation record.
    pub fn cancellation(&self) -> Option<CancellationRecord> {
        self.handle.cancellation()
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// 不可变已访问客户 bitset / Immutable visited-customer bitset.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VisitedCustomers {
    bits: Vec<u64>,
    /// 客户总数 / Total customer count.
    pub size: usize,
}

impl VisitedCustomers {
    /// 创建空 bitset / Create an empty bitset.
    pub fn empty(size: usize) -> Self {
        Self {
            bits: vec![0; size.div_ceil(64)],
            size,
        }
    }

    /// 判断是否包含客户索引 / Check whether an index is contained.
    pub fn contains(&self, index: usize) -> bool {
        let word = index / 64;
        let bit = index % 64;
        self.bits
            .get(word)
            .is_some_and(|value| value & (1u64 << bit) != 0)
    }

    /// 返回新增客户后的副本 / Return a copy with one customer added.
    pub fn add(&self, index: usize) -> Self {
        let mut result = self.clone();
        if let Some(word) = result.bits.get_mut(index / 64) {
            *word |= 1u64 << (index % 64);
        }
        result
    }

    /// 访问客户数量 / Number of visited customers.
    pub fn count(&self) -> usize {
        self.bits
            .iter()
            .map(|value| value.count_ones() as usize)
            .sum()
    }

    /// 是否访问全部客户 / Whether all customers are visited.
    pub fn is_full(&self) -> bool {
        self.count() == self.size
    }

    /// 判断是否为另一个集合的子集 / Check whether this is a subset of another set.
    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.bits
            .iter()
            .enumerate()
            .all(|(index, value)| value & !other.bits.get(index).copied().unwrap_or(0) == 0)
    }
}

/// 不可达客户 bitset / Immutable forbidden-customer bitset.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ForbiddenCustomers {
    bits: Vec<u64>,
    /// 客户总数 / Total customer count.
    pub size: usize,
}

impl ForbiddenCustomers {
    /// 创建空 bitset / Create an empty bitset.
    pub fn empty(size: usize) -> Self {
        Self {
            bits: vec![0; size.div_ceil(64)],
            size,
        }
    }

    /// 从已访问集合创建 / Create from visited customers.
    pub fn from_visited(visited: &VisitedCustomers) -> Self {
        Self {
            bits: visited.bits.clone(),
            size: visited.size,
        }
    }

    /// 判断是否禁止客户 / Check whether a customer is forbidden.
    pub fn contains(&self, index: usize) -> bool {
        let word = index / 64;
        let bit = index % 64;
        self.bits
            .get(word)
            .is_some_and(|value| value & (1u64 << bit) != 0)
    }

    /// 返回新增禁止客户后的副本 / Return a copy with one customer forbidden.
    pub fn add(&self, index: usize) -> Self {
        let mut result = self.clone();
        if let Some(word) = result.bits.get_mut(index / 64) {
            *word |= 1u64 << (index % 64);
        }
        result
    }

    /// 判断是否为另一个集合的子集 / Check subset relation.
    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.bits
            .iter()
            .enumerate()
            .all(|(index, value)| value & !other.bits.get(index).copied().unwrap_or(0) == 0)
    }

    /// 合并两个 bitset / Union two bitsets.
    pub fn union(&self, other: &Self) -> Self {
        let bits = (0..self.bits.len().max(other.bits.len()))
            .map(|index| {
                self.bits.get(index).copied().unwrap_or(0)
                    | other.bits.get(index).copied().unwrap_or(0)
            })
            .collect();
        Self {
            bits,
            size: self.size.max(other.size),
        }
    }
}

/// 不可变 ESPPRC 标签 / Immutable ESPPRC label.
#[derive(Debug, Clone)]
pub struct EspprcLabel {
    /// 累计 reduced cost / Accumulated reduced cost.
    pub reduced_cost: f64,
    /// 当前时间轴数值 / Current timeline value.
    pub time: f64,
    /// 累计负载 / Accumulated load.
    pub load: f64,
    /// 当前节点在定价图中的索引 / Current pricing-graph node index.
    pub current_node: usize,
    /// 已访问客户 / Visited customers.
    pub visited: VisitedCustomers,
    /// 已禁止客户 / Forbidden customers.
    pub forbidden: ForbiddenCustomers,
    /// 前驱标签索引 / Predecessor label index.
    pub predecessor: Option<usize>,
    /// 进入当前节点的弧索引 / Incoming arc index.
    pub predecessor_arc: Option<usize>,
}

/// 定价统计 / Pricing statistics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LabelStatistics {
    /// 创建标签数 / Created labels.
    pub labels_created: usize,
    /// 扩展次数 / Extension count.
    pub extensions: usize,
    /// 支配比较次数 / Dominance comparisons.
    pub dominance_checks: usize,
    /// 被支配标签数 / Dominated labels.
    pub dominated_labels: usize,
    /// 已知不可达客户标记数 / Feillet unreachable-customer markings.
    pub unreachable_markings: usize,
}

/// 定价截断原因 / Pricing truncation reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TruncationReason {
    /// 返回列数达到上限 / Returned-column limit reached.
    MaxColumns,
    /// 标签数达到上限 / Label limit reached.
    MaxLabels,
    /// 搜索深度达到上限 / Search-depth limit reached.
    MaxDepth,
}

/// 单车辆类型定价诊断 / Per-vehicle-type pricing diagnostic.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VehiclePricingDiagnostic {
    /// 车辆类型 / Vehicle type.
    pub vehicle_type_id: VehicleTypeId,
    /// 最小 reduced cost / Minimum reduced cost.
    pub min_reduced_cost: f64,
    /// 生成的负列数 / Number of negative columns.
    pub negative_columns: usize,
    /// 是否完成精确定价 / Whether exact pricing completed.
    pub exact_complete: bool,
    /// 标签统计 / Label statistics.
    pub statistics: LabelStatistics,
}

/// 定价诊断 / Pricing diagnostic.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PricingDiagnostic {
    /// 车辆类型诊断 / Vehicle-type diagnostics.
    pub vehicle_types: Vec<VehiclePricingDiagnostic>,
}

/// 定价请求 / Pricing request.
#[derive(Debug, Clone)]
pub struct PricingRequest<V: SolveValue + UnitConversionValue> {
    /// VRPTW 实例 / VRPTW instance.
    pub instance: Arc<VrptwInstance<V>>,
    /// 当前阶段对偶 / Current-phase duals.
    pub duals: PricingDuals,
    /// 当前分支遮罩 / Current branch mask.
    pub branch_mask: Option<BranchMask<VehicleTypeId>>,
    /// 定价容差 / Pricing tolerance.
    pub pricing_tolerance: f64,
    /// 每次最多返回列数；不影响完成证明 / Maximum returned columns; does not affect completion proof.
    pub max_columns_per_pricing: usize,
    /// 当前车辆类型 / Vehicle type being priced.
    pub vehicle_type_id: VehicleTypeId,
    /// 可选取消令牌 / Optional cancellation token.
    pub cancellation: Option<CancellationToken>,
    /// 可选截止时间 / Optional deadline.
    pub deadline: Option<Instant>,
    /// 标签上限 / Label limit.
    pub max_labels: Option<usize>,
    /// 路径客户数上限；默认由客户数决定 / Customer-depth limit; defaults to customer count.
    pub max_depth: Option<usize>,
}

impl<V> PricingRequest<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建定价请求 / Create a pricing request.
    pub fn new(
        instance: Arc<VrptwInstance<V>>,
        duals: PricingDuals,
        vehicle_type_id: VehicleTypeId,
    ) -> Self {
        Self {
            pricing_tolerance: instance.tolerances.pricing,
            instance,
            duals,
            branch_mask: None,
            max_columns_per_pricing: usize::MAX,
            vehicle_type_id,
            cancellation: None,
            deadline: None,
            max_labels: None,
            max_depth: None,
        }
    }

    /// 检查统一中断条件 / Check unified interruption conditions.
    pub fn interrupted(&self) -> bool {
        self.cancellation
            .as_ref()
            .is_some_and(CancellationToken::is_cancelled)
            || self
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
    }
}

/// 定价结果 / Pricing result.
#[derive(Debug, Clone)]
pub struct PricingResult<V: SolveValue + UnitConversionValue> {
    /// 发现的负 reduced-cost 路线 / Found negative reduced-cost routes.
    pub routes: Vec<Route<V>>,
    /// 最小 reduced cost / Minimum reduced cost.
    pub min_reduced_cost: f64,
    /// 所有车辆类型是否精确定价完成 / Whether all priced vehicle types are exact-complete.
    pub exact_pricing_complete: bool,
    /// 是否中断 / Whether interrupted.
    pub interrupted: bool,
    /// 是否因返回/搜索上限截断 / Whether truncated by output/search limit.
    pub truncated: bool,
    /// 截断原因 / Truncation reason.
    pub truncation_reason: Option<TruncationReason>,
    /// 标签和支配统计 / Label and dominance statistics.
    pub statistics: LabelStatistics,
    /// 按车辆类型诊断 / Per-vehicle-type diagnostics.
    pub diagnostic: PricingDiagnostic,
}

impl<V> PricingResult<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建空的完成结果 / Create an empty complete result.
    pub fn empty() -> Self {
        Self {
            routes: Vec::new(),
            min_reduced_cost: 0.0,
            exact_pricing_complete: true,
            interrupted: false,
            truncated: false,
            truncation_reason: None,
            statistics: LabelStatistics::default(),
            diagnostic: PricingDiagnostic::default(),
        }
    }
}
