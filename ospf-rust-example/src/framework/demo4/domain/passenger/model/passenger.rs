//! 旅客模型模块 / Passenger model module.
use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::flatten::LinearMonomial;
use crate::framework::demo4::infrastructure::PassengerClass;

/// 旅客 / Passenger
/// 对齐 Kotlin Passenger / Aligned with Kotlin Passenger
#[derive(Debug, Clone)]
pub struct Passenger {
    /// 旅客标识 / Passenger identifier
    pub id: String,
    /// 旅客数量 / Passenger amount
    pub amount: u64,
    /// 航班及舱位列表 / Flight and class list
    pub flights: Vec<(String, PassengerClass)>,
}

impl Passenger {
    /// 获取出发航班标识 / Get departure flight identifier
    pub fn dep(&self) -> Option<&str> {
        self.flights.first().map(|(id, _)| id.as_str())
    }

    /// 获取到达航班标识 / Get arrival flight identifier
    pub fn arr(&self) -> Option<&str> {
        self.flights.last().map(|(id, _)| id.as_str())
    }

    /// 是否中转 / Whether this is a transfer passenger
    pub fn is_transfer(&self) -> bool {
        self.flights.len() > 1
    }
}

