//! 航班链接模型模块 / Flight link model module
use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;

/// 航班链接 / Flight link
/// 对齐 Kotlin FlightLink
#[derive(Debug, Clone)]
pub struct FlightLink {
    /// 前序航班标识 / Preceding flight identifier
    pub from_flight: String,
    /// 后序航班标识 / Succeeding flight identifier
    pub to_flight: String,
    /// 连接时间 / Connection time
    pub connection_time: time::Duration,
}

impl FlightLink {
    /// 注册航班链接符号到模型 / Register flight link symbols to model
    /// 对齐 Kotlin FlightLink.register
    pub fn register(
        &self,
        model: &mut MetaModel<f64>,
        next_id: &mut u64,
    ) -> Result<(), Box<dyn Error>> {
        // 链接符号
        let link_symbol = LinearExpressionSymbol::new(
            *next_id,
            &format!("flight_link_{}_{}", self.from_flight, self.to_flight),
            Vec::new(),
            0.0,
        );
        model.add_symbol(Arc::new(link_symbol))?;
        *next_id += 1;

        // 松弛符号
        let slack_symbol = LinearExpressionSymbol::new(
            *next_id,
            &format!("flight_link_slack_{}_{}", self.from_flight, self.to_flight),
            Vec::new(),
            0.0,
        );
        model.add_symbol(Arc::new(slack_symbol))?;
        *next_id += 1;

        Ok(())
    }

    /// 添加列 / Add columns
    /// 对齐 Kotlin FlightLink.addColumns
    pub fn add_columns(
        &self,
        _model: &mut MetaModel<f64>,
        _new_bunches: &[String],
    ) -> Result<(), Box<dyn Error>> {
        // 完整实现需要: 更新链接约束
        Ok(())
    }
}
