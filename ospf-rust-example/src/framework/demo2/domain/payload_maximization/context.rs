use super::aggregation::Aggregation;

/// 业载最大化上下文 / Payload maximization context
/// 对齐 Kotlin PayloadMaximizationContext
#[derive(Debug)]
pub struct PayloadMaximizationContext {
    pub aggregation: Option<Aggregation>,
}

impl PayloadMaximizationContext {
    pub fn new() -> Self {
        Self { aggregation: None }
    }

    /// 从飞机和装载上下文初始化
    /// 对齐 Kotlin PayloadMaximizationContext.init
    pub fn init(
        &mut self,
        aircraft_context: &super::super::aircraft::AircraftContext,
        _stowage_context: &super::super::stowage::context::StowageContext<'_>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let aircraft_agg = aircraft_context
            .aggregation
            .as_ref()
            .ok_or("aircraft context not initialized")?;

        self.aggregation = Some(Aggregation {
            aircraft_model: aircraft_agg.aircraft_model.clone(),
            payload_estimate: 0.0,
        });
        Ok(())
    }

    pub fn register(
        &self,
        model: &mut ospf_rust_core::model::MetaModel<f64>,
        x_idx: &[Vec<usize>],
        cargo_weights: &[f64],
        position_count: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let agg = self.aggregation.as_ref().ok_or("not initialized")?;
        super::service::generate_pipelines(agg, model, x_idx, cargo_weights, position_count)
    }

    pub fn register_for_benders_mp(
        &self,
        model: &mut ospf_rust_core::model::MetaModel<f64>,
        x_idx: &[Vec<usize>],
        cargo_weights: &[f64],
        position_count: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.register(model, x_idx, cargo_weights, position_count)
    }

    pub fn register_for_benders_sp(
        &self,
        _model: &mut ospf_rust_core::model::MetaModel<f64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
