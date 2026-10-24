//! 显式装载合同；重量为 kg，力臂和几何归一到 m。 / Explicit loading contract; weights in kg, arms and geometry normalized to m.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;
use ospf_rust_framework_bpp::{LayoutSelectionGroup, LoadingPartition, PositionConflict};
use ospf_rust_framework_bpp::application::PartitionColumnGenerationConfig;

/// 请求几何长度单位。 / Request geometry length unit.
#[derive(Clone, Copy, Debug, Default)]
pub enum LengthUnitInput {
    /// 米。 / Meters.
    #[default]
    Meter,
    /// 厘米。 / Centimeters.
    Centimeter,
    /// 英寸。 / Inches.
    Inch,
}

impl LengthUnitInput {
    pub(crate) fn meters(self, value: f64) -> f64 {
        value * match self { Self::Meter => 1.0, Self::Centimeter => 0.01, Self::Inch => 0.0254 }
    }
}

/// 货物装载资格；未声明时只有预分配模式必装。 / Cargo eligibility; undeclared cargo is required only in predistribution mode.
#[derive(Clone, Debug, Default)]
pub struct CargoLoadingInput {
    /// 可选严格货物顺序；未声明时使用业务优先级。 / Optional strict cargo order; business priority applies when absent.
    pub order: Option<i32>,
    /// 覆盖模式默认必装状态。 / Override the mode's default required status.
    pub required: Option<bool>,
    /// 明确允许的舱位；None 表示全部，空集合表示无合法位置。 / Explicit permitted positions; None means all, an empty set means none.
    pub allowed_positions: Option<BTreeSet<String>>,
    /// 限定物理空间；None 表示不限甲板。 / Restrict physical space; None means unrestricted decks.
    pub space_id: Option<String>,
}

/// 一个显式舱位；不根据面积、名字或序号推断几何。 / An explicit position; geometry is never inferred from area, name or index.
#[derive(Clone, Debug)]
pub struct PositionLoadingInput {
    /// 稳定业务装载顺序，不是请求列表序号。 / Stable business loading order, not request-list index.
    pub loading_order: i32,
    /// 顺序的业务作用域，同域内比较。 / Business ordering scope, compared only within that scope.
    pub loading_order_scope: String,
    /// 物理空间身份。 / Physical-space identity.
    pub space_id: String,
    /// 前边界。 / Front bound.
    pub front: f64,
    /// 后边界。 / Back bound.
    pub back: f64,
    /// 左边界；仅宽体使用，但仍须有限。 / Left bound; used only for wide bodies, but must be finite.
    pub left: f64,
    /// 右边界。 / Right bound.
    pub right: f64,
    /// 是否允许新货装载，已固定货物不被删除。 / Allow new cargo, without removing fixed loads.
    pub enabled: bool,
    /// 是否允许普通实装决策。 / Allow ordinary actual-loading decisions.
    pub stowage_needed: bool,
    /// 是否允许调整实装决策。 / Allow actual-load adjustment decisions.
    pub adjustment_needed: bool,
    /// 外部固定占用。 / External fixed occupancy.
    pub fixed_occupied: bool,
    /// 是否禁止放空，约束整体占位而非货物件数。 / Forbid empty occupancy, rather than requiring a cargo count.
    pub empty_forbidden: bool,
    /// 可选预测重量上下界（kg）。 / Optional predicate-weight bounds (kg).
    pub predicate_weight: Option<(f64, f64)>,
    /// 可选整数推荐重量上下界（kg），仅重量推荐模式启用。 / Optional integer recommended-weight bounds (kg), enabled only in recommendation mode.
    pub recommended_weight: Option<(f64, f64)>,
}

/// 三条业务路径共享的领域与求解合同。 / Domain and solve contract shared by all three business paths.
#[derive(Clone, Debug)]
pub struct Demo2LoadingInput {
    /// 按稳定舱位身份声明的相邻关系，优先于旧下标字段。 / Adjacency by stable position IDs, taking precedence over legacy index fields.
    pub adjacent_positions: Vec<(String, String)>,
    /// 宽体 CLIM 绝对上界（kg*m）；不从几何推断。 / Absolute wide-body CLIM bound (kg*m), never inferred from geometry.
    pub max_clim: Option<f64>,
    /// 几何单位；其他旧字段仍为 kg、m、kg*m。 / Geometry unit; other existing fields remain kg, m and kg*m.
    pub length_unit: LengthUnitInput,
    /// 按货物身份覆盖资格。 / Eligibility overrides keyed by cargo identity.
    pub cargos: BTreeMap<String, CargoLoadingInput>,
    /// 必须精确覆盖请求舱位。 / Must exactly cover request positions.
    pub positions: BTreeMap<String, PositionLoadingInput>,
    /// 仅追加业务互斥，不能取消几何互斥。 / Additional business conflicts, never removing geometry conflicts.
    pub conflicts: BTreeSet<PositionConflict>,
    /// 显式恰选一布局组。 / Explicit exactly-one layout groups.
    pub layout_groups: Vec<LayoutSelectionGroup>,
    /// 独立分区 CG；不得同时优先 Benders。 / Independent partition CG; mutually exclusive with preferred Benders.
    pub partition_column_generation: bool,
    /// 显式且完整的舱位分区。 / Explicit exhaustive position partitions.
    pub partitions: Vec<LoadingPartition>,
    /// 根节点及补全配置。 / Root and completion configuration.
    pub column_generation: PartitionColumnGenerationConfig,
    /// 包含注册、分解、回退在内的总预算。 / Total budget including registration, decomposition and fallback.
    pub time_limit: Duration,
    /// 启用源收尾时加速阶段占剩余预算的比例，严格在0与1之间；关闭收尾时忽略。 / Accelerator fraction of the remaining budget with source finishing enabled, strictly between zero and one; ignored when finishing is disabled.
    pub acceleration_time_ratio: f64,
}

impl Default for Demo2LoadingInput {
    fn default() -> Self {
        Self { adjacent_positions: vec![], max_clim: None, length_unit: LengthUnitInput::Meter, cargos: BTreeMap::new(), positions: BTreeMap::new(),
            conflicts: BTreeSet::new(), layout_groups: vec![], partition_column_generation: false,
            partitions: vec![], column_generation: PartitionColumnGenerationConfig::default(), time_limit: Duration::from_secs(60), acceleration_time_ratio: 0.5 }
    }
}
