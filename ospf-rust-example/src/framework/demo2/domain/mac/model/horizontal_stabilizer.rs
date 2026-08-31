use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;

/// 水平安定面 / Horizontal stabilizer (对齐 Kotlin HorizontalStabilizer)
#[derive(Debug, Clone)]
pub struct HorizontalStabilizer {
    pub key: String,
    pub points: Vec<(f64, f64)>,
    pub limit: f64,
}

impl HorizontalStabilizer {
    /// 注册水平安定面约束到模型
    /// 对齐 Kotlin HorizontalStabilizer.register
    pub fn register(
        &self,
        stowage_mode: &str,
        model: &mut MetaModel<f64>,
    ) -> Result<(), Box<dyn Error>> {
        if stowage_mode == "WeightRecommendation" {
            return Ok(());
        }

        let next_id = 10200u64;
        let hs_symbol = LinearExpressionSymbol::new(
            next_id,
            &format!("horizontal_stabilizer_{}", self.key),
            Vec::new(),
            self.limit,
        );
        model.add_symbol(Arc::new(hs_symbol))?;

        Ok(())
    }
}
