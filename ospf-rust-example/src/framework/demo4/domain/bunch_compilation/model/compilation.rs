//! 编制结果模型模块 / Compilation result model module
use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;

/// 编译结果 / Compilation
/// 对齐 Kotlin Compilation (BunchCompilation)
#[derive(Debug, Clone)]
pub struct Compilation {
    /// 编组标识 / Bunch identifier
    pub bunch_id: String,
    /// 航班标识列表 / Flight identifier list
    pub flights: Vec<String>,
    /// 飞机类型 / Aircraft type
    pub aircraft_type: String,
    /// 成本 / Cost
    pub cost: f64,
}

impl Compilation {
    /// 注册编译符号到模型 / Register compilation symbols to model
    /// 对齐 Kotlin Compilation.register
    pub fn register(
        &self,
        model: &mut MetaModel<f64>,
        next_id: &mut u64,
    ) -> Result<(), Box<dyn Error>> {
        // 编译成本符号
        let cost_symbol = LinearExpressionSymbol::new(
            *next_id,
            &format!("compilation_cost_{}", self.bunch_id),
            Vec::new(),
            self.cost,
        );
        model.add_symbol(Arc::new(cost_symbol))?;
        *next_id += 1;

        Ok(())
    }
}
