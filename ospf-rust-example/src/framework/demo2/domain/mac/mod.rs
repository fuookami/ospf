pub mod model;
pub mod service;

use model::*;

/// MAC 领域聚合 / MAC domain aggregation (对齐 Kotlin mac Aggregation)
#[derive(Debug)]
pub struct Aggregation {
    pub torque: Torque,
    pub mac: Mac,
    pub horizontal_stabilizers: Vec<HorizontalStabilizer>,
}

impl Aggregation {
    pub fn register(
        &self,
        _stowage_mode: &str,
        model: &mut ospf_rust_core::model::MetaModel<f64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.torque.register(model, &[], &[], &[], &[])?;
        self.mac.register(model, 0.0, 0.0, 0.0, 1.0)?;
        let mut next_id = 60000u64;
        for hs in &self.horizontal_stabilizers {
            hs.register(_stowage_mode, model, next_id, 0, 0, 0)?;
            next_id += 10;
        }
        Ok(())
    }

    pub fn register_for_benders_mp(
        &self,
        model: &mut ospf_rust_core::model::MetaModel<f64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.register("FullLoad", model)
    }

    pub fn register_for_benders_sp(
        &self,
        _model: &mut ospf_rust_core::model::MetaModel<f64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

/// MAC 上下文 / MAC context (对齐 Kotlin MacContext)
#[derive(Debug)]
pub struct MacContext {
    pub aggregation: Option<Aggregation>,
}

impl MacContext {
    pub fn new() -> Self {
        Self { aggregation: None }
    }

    /// 从飞机和装载上下文初始化 MAC 聚合
    /// 对齐 Kotlin MacContext.init
    pub fn init(
        &mut self,
        aircraft_context: &super::aircraft::AircraftContext,
        _stowage_context: &super::stowage::context::StowageContext<'_>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let _aircraft_agg = aircraft_context
            .aggregation
            .as_ref()
            .ok_or("aircraft context not initialized")?;

        // 构建 MAC 聚合
        let torque = Torque {
            estimate_longitudinal: 0.0,
            actual_longitudinal: 0.0,
            lateral: 0.0,
        };
        let mac = Mac {
            value: 0.0,
            percentage: 0.0,
        };

        self.aggregation = Some(Aggregation {
            torque,
            mac,
            horizontal_stabilizers: Vec::new(),
        });

        Ok(())
    }

    pub fn register(
        &self,
        stowage_mode: &str,
        model: &mut ospf_rust_core::model::MetaModel<f64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(agg) = &self.aggregation {
            agg.register(stowage_mode, model)?;
        }
        Ok(())
    }
}
