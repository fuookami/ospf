use std::collections::HashMap;

/// 影子价格参数 / Shadow price arguments
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskShadowPriceArguments {
    pub task_id: String,
    pub aircraft_id: Option<String>,
}

/// 影子价格映射 / Shadow price map (对齐 Kotlin ShadowPriceMap)
#[derive(Debug, Clone)]
pub struct ShadowPriceMap {
    pub prices: HashMap<TaskShadowPriceArguments, f64>,
}

impl ShadowPriceMap {
    pub fn new() -> Self {
        Self { prices: HashMap::new() }
    }

    pub fn get(&self, args: &TaskShadowPriceArguments) -> Option<f64> {
        self.prices.get(args).copied()
    }

    pub fn set(&mut self, args: TaskShadowPriceArguments, price: f64) {
        self.prices.insert(args, price);
    }

    /// 计算缩减成本 / Compute reduced cost
    pub fn reduced_cost(&self, task_id: &str, cost: f64) -> f64 {
        let args = TaskShadowPriceArguments {
            task_id: task_id.to_string(),
            aircraft_id: None,
        };
        let shadow_price = self.prices.get(&args).copied().unwrap_or(0.0);
        cost - shadow_price
    }
}
