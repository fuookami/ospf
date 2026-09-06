// ============================================================================
// BinLayer - 箱层 / Bin layer
// ============================================================================

/// 箱层 / Bin layer
///
/// 列生成算法中一个层候选的描述。
/// Description of a layer candidate in the column generation algorithm.
#[derive(Debug, Clone)]
pub struct BinLayer<V, U: UnitTrait> {
    /// 迭代编号 / Iteration index
    pub iteration: i64,
    /// 来源 / Source
    pub from: String,
    /// 箱型 / Bin type
    pub bin: Option<BinType<V, U>>,
    /// 深度 / Depth
    pub depth: Quantity<V, U>,
    /// 需求覆盖 / Demand coverage
    pub demand_coverage: Vec<Bpp3dLayerDemandCoverage>,
}

impl<V, U: UnitTrait> BinLayer<V, U> {
    /// 带需求覆盖创建副本 / Clone with demand coverage
    pub fn with_demand_coverage(mut self, coverage: Vec<Bpp3dLayerDemandCoverage>) -> Self {
        self.demand_coverage = coverage;
        self
    }

    /// 查询需求覆盖系数 / Query demand coverage coefficient
    pub fn demand_coverage_coefficient(
        &self,
        mode: Bpp3dDemandMode,
        key: &Bpp3dDemandKey,
    ) -> f64 {
        self.demand_coverage
            .iter()
            .find(|coverage| coverage.mode == mode && &coverage.key == key)
            .map(|coverage| coverage.coefficient)
            .unwrap_or(0.0)
    }
}

