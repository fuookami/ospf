/// mock RMP executor / Mock RMP executor
#[derive(Debug, Clone, Default)]
pub struct MockColumnGenerationRmpExecutor {
    /// 目标值 / Objective
    pub objective: Option<f64>,
}

impl ColumnGenerationRmpExecutor for MockColumnGenerationRmpExecutor {
    fn execute(&self, state: &ColumnGenerationApplicationState) -> ColumnGenerationRmpExecution {
        ColumnGenerationRmpExecution {
            objective: self.objective,
            shadow_price_summary: HashMap::new(),
            additional_shadow_prices: HashMap::new(),
            diagnostics: None,
            info: HashMap::from([
                ("executor".to_string(), "mock_rmp".to_string()),
                ("layer_count".to_string(), state.layers.len().to_string()),
            ]),
        }
    }
}

/// mock final executor / Mock final executor
#[derive(Debug, Clone, Default)]
pub struct MockColumnGenerationFinalExecutor;

impl ColumnGenerationFinalExecutor for MockColumnGenerationFinalExecutor {
    fn execute(&self, state: &ColumnGenerationApplicationState) -> ColumnGenerationFinalExecution {
        ColumnGenerationFinalExecution {
            layers: state.layers.clone(),
            packed_bins: Vec::new(),
            objective: None,
            final_solved: true,
            diagnostics: None,
            info: HashMap::from([
                ("executor".to_string(), "mock_final".to_string()),
                ("layer_count".to_string(), state.layers.len().to_string()),
            ]),
        }
    }
}

// ============================================================================
