// ColumnGenerationPackingAnalyzer - 装箱分析器 / Packing analyzer
// ============================================================================

/// 列生成装箱分析 / Column generation packing analysis
#[derive(Debug, Clone)]
pub struct ColumnGenerationPackingAnalysis<V, U: UnitTrait> {
    /// 装箱结果 / Packing result
    pub packing_result: PackingResult<V, U>,
    /// 渲染装载计划 / Render loading plans
    pub render_loading_plans: Vec<RenderLoadingPlanDto>,
}

/// 列生成装箱分析器 / Column generation packing analyzer
#[derive(Debug, Clone, Default)]
pub struct ColumnGenerationPackingAnalyzer {
    /// 装箱器 / Packer
    pub packer: Packer,
    /// 渲染适配器 / Renderer adapter
    pub renderer: PackingRendererAdapter,
}

impl ColumnGenerationPackingAnalyzer {
    /// 创建分析器 / Create analyzer
    pub fn new() -> Self {
        Self {
            packer: Packer::new(),
            renderer: PackingRendererAdapter::new(),
        }
    }

    /// 分析已装箱结果 / Analyze packed bins
    pub fn analyze<V, U>(
        &self,
        packed_bins: Vec<PackedBin<V, U>>,
    ) -> Result<ColumnGenerationPackingAnalysis<V, U>, Vec<String>>
    where
        V: num_traits::Float + Field + Clone + Debug + Send + Sync + PartialOrd + num_traits::FloatConst + Into<f64>,
        U: CTUnit + Default + Clone,
    {
        for packed_bin in &packed_bins {
            PackingGeometryGuard::validate(packed_bin)?;
        }

        let packing_result = self.packer.invoke(packed_bins);
        let render_loading_plans = self.renderer.to_render_dto(&packing_result);
        Ok(ColumnGenerationPackingAnalysis {
            packing_result,
            render_loading_plans,
        })
    }
}

// ============================================================================
