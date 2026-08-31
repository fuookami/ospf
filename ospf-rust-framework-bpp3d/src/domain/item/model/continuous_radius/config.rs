/// 连续圆柱半径求解结果 / Continuous cylinder radius solution
#[derive(Debug, Clone, PartialEq)]
pub struct ContinuousCylinderRadiusSolution {
    /// 货物标识 / Item id
    pub item_id: String,
    /// 来源 / Source
    pub source: String,
    /// 变量名 / Variable name
    pub variable_name: String,
    /// 对齐轴 / Alignment axis
    pub axis: Axis3,
    /// 求解选中半径 / Solver-selected radius
    pub radius: f64,
    /// PWL 半径平方输出 / PWL radius-squared output
    pub radius_squared: Option<f64>,
    /// 选中 PWL 分段索引 / Selected PWL segment index
    pub segment_index: Option<usize>,
}

/// 连续半径模型组件 / Continuous radius model component
///
/// 管理连续半径圆柱在求解器中的变量注册和结果提取。
/// Manages variable registration and result extraction for continuous-radius
/// cylinders in the solver.
#[derive(Debug, Clone)]
pub struct ContinuousRadiusModelComponent {
    /// 原型列表 / Prototypes
    pub prototypes: Vec<ContinuousCylinderRadiusSolverPrototype>,
    /// 注册计划 / Registration plan
    pub registration_plan: ContinuousRadiusRegistrationPlan,
    /// 半径重量函数 / Radius weight functions
    pub weight_functions: HashMap<String, ContinuousRadiusWeightFunction>,
    /// 目标策略 / Objective policy
    pub objective_policy: ContinuousRadiusObjectivePolicy,
}

/// 连续半径注册计划 / Continuous radius registration plan
#[derive(Debug, Clone, Default)]
pub struct ContinuousRadiusRegistrationPlan {
    /// 变量名 / Variable names
    pub variable_names: Vec<String>,
    /// 注册的变量 / Registered variables
    pub registered_variables: Vec<String>,
    /// 阻塞的变量 / Blocked variables
    pub blocked_variables: Vec<String>,
}

/// 连续半径重量函数 / Continuous radius weight function
///
/// 以 `weight = intercept + radius_squared_coefficient * r²` 表达
/// `radius_weight_function_key` 对应的业务重量/成本函数。
/// Represents the business weight/cost function behind
/// `radius_weight_function_key` as
/// `weight = intercept + radius_squared_coefficient * r²`.
#[derive(Debug, Clone, PartialEq)]
pub struct ContinuousRadiusWeightFunction {
    /// 函数键 / Function key
    pub key: String,
    /// 截距 / Intercept
    pub intercept: f64,
    /// 半径平方系数 / Radius-squared coefficient
    pub radius_squared_coefficient: f64,
    /// 目标权重 / Objective weight
    pub objective_weight: f64,
}

impl ContinuousRadiusWeightFunction {
    /// 创建函数 / Create function
    pub fn new(
        key: impl Into<String>,
        intercept: f64,
        radius_squared_coefficient: f64,
        objective_weight: f64,
    ) -> Self {
        Self {
            key: key.into(),
            intercept,
            radius_squared_coefficient,
            objective_weight,
        }
    }

    /// 对半径平方求值 / Evaluate by radius squared
    pub fn evaluate_radius_squared(&self, radius_squared: f64) -> f64 {
        self.intercept + self.radius_squared_coefficient * radius_squared
    }
}

/// 连续半径目标策略 / Continuous radius objective policy
///
/// 在没有业务 weight 函数时，为连续半径 PWL 变量提供可控的求解 tie-breaker。
/// Provides a controllable solver tie-breaker for continuous-radius PWL
/// variables when no business weight function has been supplied.
#[derive(Debug, Clone, PartialEq)]
pub enum ContinuousRadiusObjectivePolicy {
    /// 不注册目标 / Do not register objective
    None,
    /// 最小化半径平方 / Minimize radius squared
    MinimizeRadiusSquared {
        /// 权重 / Objective weight
        weight: f64,
    },
}

impl Default for ContinuousRadiusObjectivePolicy {
    fn default() -> Self {
        Self::MinimizeRadiusSquared { weight: 1e-6 }
    }
}

impl ContinuousRadiusObjectivePolicy {
    /// 策略名称 / Policy name
    pub fn name(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::MinimizeRadiusSquared { .. } => "minimize_radius_squared",
        }
    }
}

/// 连续半径变量建模注册 / Continuous radius variable model registration
#[derive(Debug, Clone, PartialEq)]
pub struct ContinuousRadiusVariableRegistration {
    /// 变量名 / Variable name
    pub variable_name: String,
    /// 半径变量索引 / Radius variable index
    pub radius_index: usize,
    /// 半径平方 PWL 输出索引 / Radius-squared PWL output index
    pub radius_squared_index: usize,
    /// 分段二值变量索引 / Segment binary variable indices
    pub segment_indices: Vec<usize>,
    /// 分段左端点 lambda 变量索引 / Segment left-end lambda variable indices
    pub left_lambda_indices: Vec<usize>,
    /// 分段右端点 lambda 变量索引 / Segment right-end lambda variable indices
    pub right_lambda_indices: Vec<usize>,
    /// 半径断点 / Radius breakpoints
    pub breakpoints: Vec<f64>,
}

impl ContinuousRadiusVariableRegistration {
    /// 根据求解解向量提取选中分段 / Extract selected segment from solver solution vector
    pub fn selected_segment_index(&self, solution: &[f64]) -> Option<usize> {
        self.segment_indices
            .iter()
            .enumerate()
            .filter_map(|(segment_index, &model_index)| {
                solution
                    .get(model_index)
                    .copied()
                    .filter(|value| *value > 0.5)
                    .map(|value| (segment_index, value))
            })
            .max_by(|(_, lhs), (_, rhs)| lhs.partial_cmp(rhs).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(segment_index, _)| segment_index)
    }

    /// 评估注册 PWL 的半径平方 / Evaluate registered PWL radius squared
    pub fn evaluate_radius_squared(&self, radius: f64) -> Option<f64> {
        if self.breakpoints.is_empty() {
            return None;
        }
        if self.breakpoints.len() == 1 {
            return Some(self.breakpoints[0] * self.breakpoints[0]);
        }
        if radius <= self.breakpoints[0] {
            return Some(self.breakpoints[0] * self.breakpoints[0]);
        }
        for window in self.breakpoints.windows(2) {
            let left = window[0];
            let right = window[1];
            if radius <= right {
                let width = right - left;
                if width.abs() <= f64::EPSILON {
                    return Some(right * right);
                }
                let ratio = (radius - left) / width;
                return Some(left * left + ratio * (right * right - left * left));
            }
        }
        self.breakpoints.last().map(|value| value * value)
    }
}

/// 连续半径建模注册结果 / Continuous radius model registration result
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ContinuousRadiusModelRegistration {
    /// 变量注册列表 / Variable registrations
    pub variables: Vec<ContinuousRadiusVariableRegistration>,
    /// 注册约束数量 / Registered constraint count
    pub constraint_count: usize,
    /// 注册目标项数量 / Registered objective term count
    pub objective_term_count: usize,
    /// 注册业务目标项数量 / Registered business objective term count
    pub business_objective_term_count: usize,
    /// 注册 tie-breaker 目标项数量 / Registered tie-breaker objective term count
    pub tie_breaker_objective_term_count: usize,
    /// 已匹配权重函数数量 / Matched weight function count
    pub matched_weight_function_count: usize,
    /// 注册诊断 / Registration diagnostics
    pub diagnostics: Vec<String>,
}

impl ContinuousRadiusModelRegistration {
    /// 是否为空 / Whether no continuous radius variable was registered
    pub fn is_empty(&self) -> bool {
        self.variables.is_empty()
    }

    /// 已注册变量块数量 / Registered variable block count
    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }
}

