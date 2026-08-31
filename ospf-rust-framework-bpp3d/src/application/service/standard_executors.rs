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

    /// 使用 RMP 模型扩展执行标准 RMP / Execute standard RMP with a model extension
    pub fn execute_rmp_with_extension(
        executor: &MetaModelRmpExecutor,
        state: &ColumnGenerationApplicationState,
        backend: &dyn MetaModelSolverBackend,
        extension: &dyn ColumnGenerationRmpModelExtension,
    ) -> ColumnGenerationRmpExecution {
        executor.execute_with_backend_and_extension(state, backend, Some(extension))
    }

    /// 使用 final 模型扩展执行标准 final / Execute standard final with a model extension
    pub fn execute_final_with_extension(
        executor: &MetaModelFinalExecutor,
        state: &ColumnGenerationApplicationState,
        backend: &dyn MetaModelSolverBackend,
        extension: &dyn ColumnGenerationFinalModelExtension,
    ) -> ColumnGenerationFinalExecution {
        executor.execute_with_backend_and_extension(state, backend, Some(extension))
    }
}

// ============================================================================
