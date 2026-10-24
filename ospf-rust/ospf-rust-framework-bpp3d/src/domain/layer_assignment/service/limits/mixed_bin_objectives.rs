// ============================================================================
// MixedBinObjectives - 箱型混合目标 / Mixed bin type objectives
// ============================================================================
//
// 为 BPP3D 补齐「按箱型差异化计价 + 体积消耗为主目标」的能力，与 BPP2D 的
// `SheetAreaMinimization` / `SheetAmountMinimization` / `SheetTypeCostMinimization` 对应。
// Adds per-bin-type pricing and a volume-consumption primary objective to BPP3D, mirroring BPP2D's
// `SheetAreaMinimization` / `SheetAmountMinimization` / `SheetTypeCostMinimization`.
//
// 主目标为什么是「箱型体积消耗最小」而不是「逐箱余料最小」：需求占用的体积由需求本身固定，因此
// `总体积浪费 = Σ 箱型体积 − 需求占用体积 = Σ 箱型体积 − 常数`，最小化箱型体积消耗就是最小化体积浪费。
// 若改为对 `箱型体积 − 已装层体积` 求和，铺满的整箱会记为 0 余料，反而让「一个铺满的大箱」优于
// 「多个小箱」，与真实装载结果相反（BPP2D 实现时由真实求解用例暴露过该错误）。
// Why the primary objective is minimum bin-type volume consumption rather than per-bin waste: the volume
// occupied by demand is fixed, so `total waste = Σ bin volume − demanded volume = Σ bin volume − constant`,
// hence minimising consumed bin volume minimises waste. Summing `bin volume − loaded layer volume` instead
// would report zero waste for a fully packed bin, making "one full large bin" look better than "several small
// bins", the opposite of reality (an error exposed by a real solve case while implementing BPP2D).
//
// 多个 `Minimize` 子目标按权重线性相加：core 会把同类目子目标的系数逐项相加，因此权重必须显式给出。
// Multiple `Minimize` sub-objectives are combined as a weighted sum: the core adds the coefficients of
// sub-objectives with the same category term by term, so weights must be explicit.
//
// 默认值与 BPP2D 对齐：体积消耗为主目标（权重 1），箱数系数降为 1e-6 仅用于在体积消耗相同的解之间打破
// 平局，箱型成本权重保持 0 直到调用方配置 `bin_type_costs`。
// Defaults are aligned with BPP2D: volume consumption is the primary objective (weight 1), the bin amount
// coefficient drops to 1e-6 as a pure tie-breaker between equal-volume solutions, and the per-bin-type cost
// weight stays zero until a caller configures `bin_type_costs`.

/// 计算单箱的余料体积 / Compute the waste volume of one bin
///
/// 余料体积 = 箱型体积 − 已装层体积之和，下限为 0。用于装箱后的统计与 KPI，不作为优化目标
/// （原因见文件头注释）。
/// Waste volume is the bin volume minus the sum of loaded layer volumes, floored at zero. It is used for
/// post-packing statistics and KPI, not as the optimisation objective (see the file header).
///
/// # 参数 / Parameters
/// - `bin_type`: 箱型 / bin type
/// - `loaded_layer_volumes`: 已装层体积 / loaded layer volumes
pub fn bin_waste_volume(
    bin_type: &BinType<f64, Meter>,
    loaded_layer_volumes: &[f64],
) -> f64 {
    let occupied: f64 = loaded_layer_volumes.iter().sum();
    let container = bin_volume(bin_type);
    if container > occupied {
        container - occupied
    } else {
        0.0
    }
}

/// 计算单箱的余料率 / Compute the waste ratio of one bin
///
/// 余料率 = 余料体积 / 箱型体积，落在 `[0, 1]`，便于跨箱型比较。
/// The waste ratio is the waste volume over the bin volume, within `[0, 1]`, comparable across bin types.
pub fn bin_waste_ratio(
    bin_type: &BinType<f64, Meter>,
    loaded_layer_volumes: &[f64],
) -> f64 {
    let container = bin_volume(bin_type);
    if container <= 0.0 {
        return 0.0;
    }
    bin_waste_volume(bin_type, loaded_layer_volumes) / container
}

/// 箱型体积 / Bin type volume
pub fn bin_volume(bin_type: &BinType<f64, Meter>) -> f64 {
    bin_type.width.value * bin_type.height.value * bin_type.depth.value
}

/// 构造按箱型计价的单箱成本函数 / Build a per-bin cost function priced by type
///
/// 未配置成本的箱型按 `default_cost` 处理。
/// Types without a configured cost fall back to `default_cost`.
pub fn per_type_bin_cost(
    type_cost: &HashMap<BinTypeId, f64>,
    default_cost: f64,
) -> impl Fn(&BinType<f64, Meter>) -> f64 + '_ {
    move |bin_type: &BinType<f64, Meter>| -> f64 {
        type_cost
            .get(&bin_type.type_code)
            .copied()
            .unwrap_or(default_cost)
    }
}

/// 箱型体积消耗最小化目标 / Bin volume consumption minimisation
///
/// `min weight · Σ_i volume_i · v_i`，其中 `v_i` 是箱子 i 是否被使用。因为需求体积固定，该目标等价于
/// 体积浪费最少。
/// `min weight · Σ_i volume_i · v_i` where `v_i` marks whether bin i is used. Since the demanded volume is
/// fixed, this is equivalent to minimum volume waste.
#[derive(Debug)]
pub struct BinVolumeMinimization {
    name: String,
    /// 箱使用标记变量索引 / Bin usage marker variable indices
    pub v_indices: Vec<usize>,
    /// 每个箱型的体积 / Volume of each bin type
    pub volumes: Vec<f64>,
    /// 目标权重 / Objective weight
    pub weight: f64,
}

impl BinVolumeMinimization {
    /// 创建体积消耗最小化目标 / Create bin volume consumption minimisation
    pub fn new(v_indices: Vec<usize>, volumes: Vec<f64>, weight: f64) -> Self {
        Self {
            name: "bin_volume_minimization".to_string(),
            v_indices,
            volumes,
            weight,
        }
    }

    /// 从箱型与箱使用标记变量索引创建 / Create from bin types and bin usage marker indices
    ///
    /// `v_indices[i]` 必须对应 `bins[i]`。
    /// `v_indices[i]` must correspond to `bins[i]`.
    pub fn from_bins(bins: &[BinType<f64, Meter>], v_indices: Vec<usize>, weight: f64) -> Self {
        let volumes = bins.iter().map(bin_volume).collect();
        Self::new(v_indices, volumes, weight)
    }
}

impl Pipeline<MetaModel<f64>> for BinVolumeMinimization {
    fn name(&self) -> &str {
        &self.name
    }

    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        None
    }

    fn register(&self, model: &mut MetaModel<f64>) {
        if self.weight == 0.0 {
            return;
        }
        let monomials: Vec<LinearMonomial<f64>> = self
            .v_indices
            .iter()
            .zip(self.volumes.iter())
            .filter_map(|(&index, &volume)| {
                let coefficient = volume * self.weight;
                if coefficient == 0.0 {
                    None
                } else {
                    Some(LinearMonomial::new(coefficient, index))
                }
            })
            .collect();
        if monomials.is_empty() {
            return;
        }
        let polynomial = Linear::new(monomials, 0.0);
        model.add_sub_objective(SubObjective::minimize(polynomial, &self.name));
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}

/// 箱型成本最小化目标 / Per-bin-type cost minimisation
///
/// `min weight · Σ_i type_cost(type_i) · v_i`，用于表达「同体积消耗下优先 40GP 而非 40HQ」这类偏好；
/// 未配置成本的箱型按 0 处理。与原有单一 `final_bin_amount_coefficient` 的区别是这里按箱型计价。
/// `min weight · Σ_i type_cost(type_i) · v_i`, expressing preferences such as "prefer 40GP over 40HQ at equal
/// volume consumption". Types without a configured cost count as zero. Unlike the single
/// `final_bin_amount_coefficient`, the pricing here is per bin type.
#[derive(Debug)]
pub struct BinTypeCostMinimization {
    name: String,
    /// 箱使用标记变量索引 / Bin usage marker variable indices
    pub v_indices: Vec<usize>,
    /// 每个箱型的成本 / Cost of each bin type
    pub type_costs: Vec<f64>,
    /// 目标权重 / Objective weight
    pub weight: f64,
}

impl BinTypeCostMinimization {
    /// 创建箱型成本最小化目标 / Create per-bin-type cost minimisation
    pub fn new(v_indices: Vec<usize>, type_costs: Vec<f64>, weight: f64) -> Self {
        Self {
            name: "bin_type_cost_minimization".to_string(),
            v_indices,
            type_costs,
            weight,
        }
    }

    /// 从箱型与成本映射创建 / Create from bin types and a cost map
    ///
    /// 未配置成本的箱型按 0 处理。
    /// Types without a configured cost count as zero.
    pub fn from_bins(
        bins: &[BinType<f64, Meter>],
        v_indices: Vec<usize>,
        type_cost: &HashMap<BinTypeId, f64>,
        weight: f64,
    ) -> Self {
        let type_costs = bins
            .iter()
            .map(|bin| type_cost.get(&bin.type_code).copied().unwrap_or(0.0))
            .collect();
        Self::new(v_indices, type_costs, weight)
    }
}

impl Pipeline<MetaModel<f64>> for BinTypeCostMinimization {
    fn name(&self) -> &str {
        &self.name
    }

    fn constraint_group(&self) -> Option<&ConstraintGroup> {
        None
    }

    fn register(&self, model: &mut MetaModel<f64>) {
        if self.weight == 0.0 {
            return;
        }
        let monomials: Vec<LinearMonomial<f64>> = self
            .v_indices
            .iter()
            .zip(self.type_costs.iter())
            .filter_map(|(&index, &cost)| {
                let coefficient = cost * self.weight;
                if coefficient == 0.0 {
                    None
                } else {
                    Some(LinearMonomial::new(coefficient, index))
                }
            })
            .collect();
        if monomials.is_empty() {
            return;
        }
        let polynomial = Linear::new(monomials, 0.0);
        model.add_sub_objective(SubObjective::minimize(polynomial, &self.name));
    }

    fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
        Ok(())
    }
}
