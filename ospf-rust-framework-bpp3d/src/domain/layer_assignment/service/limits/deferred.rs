// ============================================================================
// Deferred objectives and limits - 延后目标和约束 / Deferred objectives and limits
// ============================================================================

/// 延后注册计划 / Deferred registration plan
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DeferredRegistrationPlan {
    /// 名称 / Name
    pub name: String,
    /// 目标族 / Objective family
    pub objective_family: Option<String>,
    /// 约束族 / Constraint family
    pub constraint_family: Option<String>,
    /// 变量索引 / Variable indices
    pub variable_indices: Vec<usize>,
    /// 诊断信息 / Diagnostics
    pub diagnostics: Vec<String>,
}

macro_rules! deferred_limit {
    ($type_name:ident, $name:literal, $message:literal) => {
        /// 延后约束或目标入口 / Deferred constraint or objective entry
        #[derive(Debug, Clone, Default)]
        pub struct $type_name {
            /// 名称 / Name
            pub name: String,
        }

        impl $type_name {
            /// 创建入口 / Create entry
            pub fn new() -> Self {
                Self {
                    name: $name.to_string(),
                }
            }

            /// 诊断信息 / Diagnostics
            pub fn diagnostics(&self) -> Vec<String> {
                vec![$message.to_string()]
            }

            /// 创建结构化注册计划 / Create structured registration plan
            pub fn registration_plan(&self, variable_indices: Vec<usize>) -> DeferredRegistrationPlan {
                DeferredRegistrationPlan {
                    name: self.name.clone(),
                    objective_family: Some(self.name.clone()),
                    constraint_family: None,
                    variable_indices,
                    diagnostics: self.diagnostics(),
                }
            }

            /// 注册最小目标 / Register minimal objective
            pub fn register_minimal_objective(
                &self,
                model: &mut MetaModel<f64>,
                variable_indices: Vec<usize>,
                coefficient: f64,
            ) -> DeferredRegistrationPlan {
                let plan = self.registration_plan(variable_indices.clone());
                if !variable_indices.is_empty() {
                    let polynomial = Linear::new(
                        variable_indices
                            .iter()
                            .map(|index| LinearMonomial::new(coefficient, *index))
                            .collect(),
                        0.0,
                    );
                    model.add_sub_objective(SubObjective::minimize(polynomial, &self.name));
                }
                plan
            }
        }
    };
}

deferred_limit!(
    RestAmountMinimization,
    "rest_amount_minimization",
    "rest amount minimization registers conservative unmet-demand objective"
);
deferred_limit!(
    TailBinLoadingRateMinimization,
    "tail_bin_loading_rate_minimization",
    "tail bin loading rate minimization registers conservative tail-load objective"
);
deferred_limit!(
    BinLoadingOrderConstraint,
    "bin_loading_order_constraint",
    "bin loading order constraint registers adjacent activation ordering rows"
);

impl BinLoadingOrderConstraint {
    /// 创建结构化约束注册计划 / Create structured constraint registration plan
    pub fn constraint_registration_plan(&self, variable_indices: Vec<usize>) -> DeferredRegistrationPlan {
        DeferredRegistrationPlan {
            name: self.name.clone(),
            objective_family: None,
            constraint_family: Some(self.name.clone()),
            variable_indices,
            diagnostics: self.diagnostics(),
        }
    }

    /// 注册最小顺序约束 / Register minimal ordering constraint
    pub fn register_minimal_constraint(
        &self,
        model: &mut MetaModel<f64>,
        variable_indices: Vec<usize>,
    ) -> DeferredRegistrationPlan {
        let plan = self.constraint_registration_plan(variable_indices.clone());
        for pair in variable_indices.windows(2) {
            let _ = model.add_le_constraint(
                &[(pair[1], 1.0), (pair[0], -1.0)],
                0.0,
                &format!("{}_{}_{}", self.name, pair[0], pair[1]),
            );
        }
        plan
    }
}

impl RestAmountMinimization {
    /// 注册余量最小化目标 / Register rest-amount minimization objective
    pub fn register_objective(
        &self,
        model: &mut MetaModel<f64>,
        rest_variable_indices: Vec<usize>,
        coefficient: f64,
    ) -> DeferredRegistrationPlan {
        self.register_minimal_objective(model, rest_variable_indices, coefficient)
    }
}

impl TailBinLoadingRateMinimization {
    /// 注册尾箱装载率最小化目标 / Register tail-bin loading-rate minimization objective
    pub fn register_objective(
        &self,
        model: &mut MetaModel<f64>,
        tail_bin_variable_indices: Vec<usize>,
        coefficient: f64,
    ) -> DeferredRegistrationPlan {
        self.register_minimal_objective(model, tail_bin_variable_indices, coefficient)
    }
}

impl BinLoadingOrderConstraint {
    /// 注册装箱顺序约束 / Register bin loading order constraint
    pub fn register_constraint(
        &self,
        model: &mut MetaModel<f64>,
        ordered_variable_indices: Vec<usize>,
    ) -> DeferredRegistrationPlan {
        self.register_minimal_constraint(model, ordered_variable_indices)
    }
}

