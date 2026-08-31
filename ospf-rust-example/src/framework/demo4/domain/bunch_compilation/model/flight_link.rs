    pub from_flight: String,
    pub to_flight: String,
    pub connection_time: time::Duration,
}

impl FlightLink {
    /// 注册航班链接符号到模型
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
