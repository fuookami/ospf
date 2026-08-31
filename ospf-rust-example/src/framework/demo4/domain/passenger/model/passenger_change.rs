use std::error::Error;
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::LinearExpressionSymbol;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::IfFunction;
use ospf_rust_core::variable::BinaryVariableItem;
use super::passenger::Passenger;
use crate::framework::demo4::infrastructure::PassengerClass;

/// 旅客变更注册的变量索引 / Passenger change registered variable indices
#[derive(Debug, Clone)]
pub struct PassengerChangeVariables {
    /// class_change_symbol_idx = LinearExpressionSymbol 的 solver index
    pub class_change_symbol_idx: usize,
    /// flight_change_symbol_idx = LinearExpressionSymbol 的 solver index
    pub flight_change_symbol_idx: usize,
    /// class_change_if_idx = IfFunction(class_change_decision, then=1, else=0) 的结果变量 solver index
    /// 当舱位变更生效时值为 1，否则为 0
    pub class_change_if_idx: usize,
    /// flight_change_if_idx = IfFunction(flight_change_decision, then=1, else=0) 的结果变量 solver index
    /// 当航班变更生效时值为 1，否则为 0
    pub flight_change_if_idx: usize,
}

/// 旅客变更 / Passenger change
/// 对齐 Kotlin PassengerChange
#[derive(Debug, Clone)]
pub struct PassengerChange {
    pub passenger: Passenger,
    pub from_flight: String,
    pub to_flight: String,
    pub from_class: PassengerClass,
    pub to_class: PassengerClass,
}

impl PassengerChange {
    /// 注册旅客变更符号到模型
    ///
    /// 对齐 Kotlin PassengerChange.register:
    /// 1. 注册舱位变更 LinearExpressionSymbol
    /// 2. 注册航班变更 LinearExpressionSymbol
    /// 3. 注册舱位变更决策二元变量
    /// 4. 注册航班变更决策二元变量
    /// 5. 创建舱位变更 IfFunction (class_change_if)
    /// 6. 创建航班变更 IfFunction (flight_change_if)
    ///
    /// # Arguments
    /// * `model` - 模型实例
    /// * `next_id` - 下一个可用的符号 ID
    pub fn register(
        &self,
        model: &mut MetaModel<f64>,
        next_id: &mut u64,
    ) -> Result<PassengerChangeVariables, Box<dyn Error>> {
        // 1. 舱位变更符号
        let class_change_symbol = LinearExpressionSymbol::new(
            *next_id,
            &format!("passenger_class_change_{}_{}", self.passenger.id, self.from_flight),
            Vec::new(),
            0.0,
        );
        let class_change_symbol_idx = *next_id as usize;
        model.add_symbol(Arc::new(class_change_symbol))?;
        *next_id += 1;

        // 2. 航班变更符号
        let flight_change_symbol = LinearExpressionSymbol::new(
            *next_id,
            &format!("passenger_flight_change_{}_{}", self.passenger.id, self.from_flight),
            Vec::new(),
            0.0,
        );
        let flight_change_symbol_idx = *next_id as usize;
        model.add_symbol(Arc::new(flight_change_symbol))?;
        *next_id += 1;

        // 3. 舱位变更决策二元变量
        let class_change_decision = BinaryVariableItem::auto(
            &format!("class_change_decision_{}_{}", self.passenger.id, self.from_flight),
        );
        let class_change_decision_idx = model.register_variable(class_change_decision)?;

        // 4. 航班变更决策二元变量
        let flight_change_decision = BinaryVariableItem::auto(
            &format!("flight_change_decision_{}_{}", self.passenger.id, self.from_flight),
        );
        let flight_change_decision_idx = model.register_variable(flight_change_decision)?;

        // 5. 舱位变更 IfFunction
        // 对齐 Kotlin: classChangeIf = IfFunction(condition=classChangeDecision, then=1, else=0)
        let class_condition = Linear::new(
            vec![LinearMonomial::new(1.0, class_change_decision_idx)],
            0.0,
        );
        let class_then = Linear::new(Vec::new(), 1.0);
        let class_else = Linear::new(Vec::new(), 0.0);
        let class_change_if = IfFunction::new(
            *next_id,
            &format!("passenger_class_change_if_{}_{}", self.passenger.id, self.from_flight),
            class_condition,
            class_then,
            class_else,
        );
        let class_change_if_idx = class_change_if.result_variable().index();
        model.add_symbol(Arc::new(class_change_if))?;
        *next_id += 1;

        // 6. 航班变更 IfFunction
        // 对齐 Kotlin: flightChangeIf = IfFunction(condition=flightChangeDecision, then=1, else=0)
        let flight_condition = Linear::new(
            vec![LinearMonomial::new(1.0, flight_change_decision_idx)],
            0.0,
        );
        let flight_then = Linear::new(Vec::new(), 1.0);
        let flight_else = Linear::new(Vec::new(), 0.0);
        let flight_change_if = IfFunction::new(
            *next_id,
            &format!("passenger_flight_change_if_{}_{}", self.passenger.id, self.from_flight),
            flight_condition,
            flight_then,
            flight_else,
        );
        let flight_change_if_idx = flight_change_if.result_variable().index();
        model.add_symbol(Arc::new(flight_change_if))?;
        *next_id += 1;

        Ok(PassengerChangeVariables {
            class_change_symbol_idx,
            flight_change_symbol_idx,
            class_change_if_idx,
            flight_change_if_idx,
        })
    }
}
