// ColumnGenerationStandardExecutors - 标准执行器 / Standard executors
// ============================================================================

/// 列生成标准执行器 / Column generation standard executors
#[derive(Debug, Clone, Default)]
pub struct ColumnGenerationStandardExecutors;

impl ColumnGenerationStandardExecutors {
    /// 创建默认层生成上下文 / Create default layer generation context
    pub fn default_layer_generation_context<V, U>() -> LayerGenerationContext<V, U>
    where
        V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
        U: CTUnit + Default + Debug + Clone + Send + Sync,
    {
        let mut context = LayerGenerationContext::new();
        context.add_generator(Box::new(BlockLayerGenerator::new()));
        context.add_generator(Box::new(BLLocalLayerGenerator::new()));
        context.add_generator(Box::new(BLGlobalLayerGenerator::new()));
        context.add_generator(Box::new(CirclePackingLayerGenerator::new()));
        context.add_generator(Box::new(PatternLayerGenerator::new()));
        context.add_generator(Box::new(PileLayerGenerator::new()));
        context.add_generator(Box::new(HistoricalLayerGenerator::new()));
        context
    }
}

// ============================================================================
