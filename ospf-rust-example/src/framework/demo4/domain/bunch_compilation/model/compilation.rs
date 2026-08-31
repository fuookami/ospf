    }
}

/// 编译结果 / Compilation
/// 对齐 Kotlin Compilation (BunchCompilation)
#[derive(Debug, Clone)]
pub struct Compilation {
    pub bunch_id: String,
    pub flights: Vec<String>,
    pub aircraft_type: String,
    pub cost: f64,
}

impl Compilation {
    /// 注册编译符号到模型
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
