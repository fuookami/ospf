// ============================================================================
// Packer - 装箱器 / Packer
// ============================================================================

/// 装箱结果 / Packing result
#[derive(Debug, Clone)]
pub struct PackingResult<V, U: UnitTrait> {
    /// 已装箱列表 / Packed bins
    pub packed_bins: Vec<PackedBin<V, U>>,
    /// 物料汇总 / Material summaries
    pub material_summaries: Vec<MaterialSummary>,
    /// 附加信息 / Additional info
    pub info: HashMap<String, String>,
}

/// 装箱聚合 / Packing aggregation
#[derive(Debug, Clone)]
pub struct PackingAggregation<V, U: UnitTrait> {
    /// 已装箱列表 / Packed bins
    pub packed_bins: Vec<PackedBin<V, U>>,
}

/// 装箱器 / Packer
///
/// 将最终箱子转换为装箱结果，汇总物料使用情况。
/// Converts final bins into packing results, summarizes material usage.
#[derive(Debug, Clone, Default)]
pub struct Packer;

impl Packer {
    /// 创建装箱器 / Create a packer
    pub fn new() -> Self {
        Self
    }

    /// 执行装箱分析 / Execute packing analysis
    ///
    /// 对已分配的箱子执行最终装箱验证和物料汇总。
    /// Performs final packing validation and material summarization
    /// for assigned bins.
    pub fn invoke<V, U>(&self, packed_bins: Vec<PackedBin<V, U>>) -> PackingResult<V, U>
    where
        V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
        U: CTUnit + Default + Clone,
    {
        let material_summaries = Self::summarize_materials(&packed_bins);

        PackingResult {
            packed_bins,
            material_summaries,
            info: HashMap::new(),
        }
    }

    /// 汇总物料使用 / Summarize material usage
    ///
    /// 从每个物品的 Package.materials 中提取物料使用量。
    /// Extracts material usage from each item's Package.materials.
    fn summarize_materials<V, U>(bins: &[PackedBin<V, U>]) -> Vec<MaterialSummary>
    where
        V: Clone + Debug + Send + Sync + Field + num_traits::FloatConst,
        U: CTUnit + Default + Clone,
    {
        let mut summary: HashMap<MaterialKey, u64> = HashMap::new();
        for bin in bins {
            for item in &bin.items {
                // 从 package.materials 提取物料使用量
                if let Some(pack) = &item.item.pack {
                    for (material_key, amount) in &pack.materials {
                        let entry = summary.entry(material_key.clone()).or_insert(0);
                        *entry += amount;
                    }
                }
            }
        }
        summary.into_iter()
            .map(|(material, amount)| MaterialSummary { material, amount })
            .collect()
    }
}

