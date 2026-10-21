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

    /// 列身份键 / Column identity key
    ///
    /// 箱型必须参与列身份：同一排布放在不同箱型上是不同的列，否则多箱型会被错误去重。
    /// 键同时纳入来源、深度与需求覆盖，避免把不同的载荷当成同一列。
    /// The bin type must take part in column identity: the same layout on different bin types is a different
    /// column, otherwise multiple bin types would be deduplicated incorrectly. The key also covers the source,
    /// depth and demand coverage so that different loads are not treated as one column.
    pub fn identity_key(&self) -> String
    where
        V: std::fmt::Debug,
    {
        let bin_code = self
            .bin
            .as_ref()
            .map(|bin| bin.type_code.as_str().to_string())
            .unwrap_or_default();
        let mut coverage: Vec<String> = self
            .demand_coverage
            .iter()
            .map(|entry| {
                format!(
                    "{:?}|{:?}|{:?}",
                    entry.mode, entry.key, entry.coefficient
                )
            })
            .collect();
        coverage.sort();
        format!(
            "{}|{}|{:?}|{}|{}",
            bin_code,
            self.from,
            self.depth.value,
            self.depth.unit.symbol(),
            coverage.join(",")
        )
    }

    /// 深度键（不含箱型）/ Depth key without the bin type
    pub fn depth_key(&self) -> String
    where
        V: std::fmt::Debug,
    {
        format!("{:?}|{}", self.depth.value, self.depth.unit.symbol())
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

