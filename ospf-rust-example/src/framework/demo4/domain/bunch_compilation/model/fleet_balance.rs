    pub max_balance: i64,
}

/// 机队平衡 / Fleet balance
/// 对齐 Kotlin FleetBalance
#[derive(Debug, Clone)]
pub struct FleetBalance {
    pub aircraft_type: String,
    pub balance: i64,
    pub checkpoints: Vec<FleetBalanceCheckpoint>,
    pub limits: Vec<FleetBalanceLimit>,
}

impl FleetBalance {
    /// 注册机队平衡符号到模型
    /// 对齐 Kotlin FleetBalance.register
    pub fn register(
        &self,
        model: &mut MetaModel<f64>,
        next_id: &mut u64,
    ) -> Result<(), Box<dyn Error>> {
        // 机队平衡符号
        let balance_symbol = LinearExpressionSymbol::new(
            *next_id,
            &format!("fleet_balance_{}", self.aircraft_type),
            Vec::new(),
            self.balance as f64,
        );
        model.add_symbol(Arc::new(balance_symbol))?;
        *next_id += 1;

        // 松弛变量
        let slack_symbol = LinearExpressionSymbol::new(
            *next_id,
            &format!("fleet_balance_slack_{}", self.aircraft_type),
            Vec::new(),
            0.0,
        );
        model.add_symbol(Arc::new(slack_symbol))?;
        *next_id += 1;

        Ok(())
    }

    /// 添加列 / Add columns
    /// 对齐 Kotlin FleetBalance.addColumns
    pub fn add_columns(
        &self,
        _model: &mut MetaModel<f64>,
        _new_bunches: &[String],
    ) -> Result<(), Box<dyn Error>> {
        // 完整实现需要: 更新机队平衡约束
        Ok(())
    }
}

/// 航班链接 / Flight link
/// 对齐 Kotlin FlightLink
#[derive(Debug, Clone)]
pub struct FlightLink {
