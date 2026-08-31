use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::flatten::LinearMonomial;
use crate::framework::demo4::infrastructure::PassengerClass;

/// 旅客 / Passenger
/// 对齐 Kotlin Passenger
#[derive(Debug, Clone)]
pub struct Passenger {
    pub id: String,
    pub amount: u64,
    pub flights: Vec<(String, PassengerClass)>,
}

impl Passenger {
    pub fn dep(&self) -> Option<&str> {
        self.flights.first().map(|(id, _)| id.as_str())
    }

    pub fn arr(&self) -> Option<&str> {
        self.flights.last().map(|(id, _)| id.as_str())
    }

    pub fn is_transfer(&self) -> bool {
        self.flights.len() > 1
    }
}

