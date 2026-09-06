/// 连续圆柱半径求解结果 / Continuous cylinder radius solution
#[derive(Debug, Clone, PartialEq)]
pub struct ContinuousCylinderRadiusSolution {
    /// 货物标识 / Item id
    pub item_id: ItemId,
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

/// 连续半径变量 / Continuous radius variables
///
/// 持有每个原型的半径和半径平方变量索引，作为显式模型字段。
/// Holds radius and radius-squared variable indices per prototype
/// as explicit model fields.
#[derive(Debug, Clone, PartialEq)]
pub struct ContinuousRadiusVariables {
    /// 变量名 / Variable name
    pub variable_name: String,
    /// 半径变量索引 / Radius variable index
    pub radius_index: usize,
    /// 半径平方变量索引 / Radius-squared variable index
    pub radius_squared_index: usize,
}

/// 连续半径分段变量 / Continuous radius piecewise variables
///
/// 持有每个原型的 PWL 分段变量索引，作为显式模型字段。
/// Holds PWL segment variable indices per prototype as explicit model fields.
#[derive(Debug, Clone, PartialEq)]
pub struct ContinuousRadiusPiecewiseVariables {
    /// 变量名 / Variable name
    pub variable_name: String,
    /// 分段二值变量索引 / Segment binary variable indices
    pub segment_indices: Vec<usize>,
    /// 分段左端点 lambda 变量索引 / Segment left-end lambda variable indices
    pub left_lambda_indices: Vec<usize>,
    /// 分段右端点 lambda 变量索引 / Segment right-end lambda variable indices
    pub right_lambda_indices: Vec<usize>,
    /// 半径断点 / Radius breakpoints
    pub breakpoints: Vec<f64>,
}

/// 连续半径符号 / Continuous radius symbols
///
/// 持有每个原型的派生符号索引，用于约束和目标注册。
/// Holds derived symbol indices per prototype for constraint and objective registration.
#[derive(Debug, Clone, PartialEq)]
pub struct ContinuousRadiusSymbols {
    /// 变量名 / Variable name
    pub variable_name: String,
    /// 半径符号索引（对应 radius 变量） / Radius symbol index (corresponds to radius variable)
    pub radius_symbol_index: usize,
    /// 半径平方符号索引（对应 radius_squared 变量） / Radius-squared symbol index (corresponds to radius_squared variable)
    pub radius_squared_symbol_index: usize,
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
    /// 转换为显式字段结构体 / Convert to explicit field structs
    pub fn to_explicit_fields(
        &self,
    ) -> (
        ContinuousRadiusVariables,
        ContinuousRadiusPiecewiseVariables,
        ContinuousRadiusSymbols,
    ) {
        let variables = ContinuousRadiusVariables {
            variable_name: self.variable_name.clone(),
            radius_index: self.radius_index,
            radius_squared_index: self.radius_squared_index,
        };
        let piecewise = ContinuousRadiusPiecewiseVariables {
            variable_name: self.variable_name.clone(),
            segment_indices: self.segment_indices.clone(),
            left_lambda_indices: self.left_lambda_indices.clone(),
            right_lambda_indices: self.right_lambda_indices.clone(),
            breakpoints: self.breakpoints.clone(),
        };
        let symbols = ContinuousRadiusSymbols {
            variable_name: self.variable_name.clone(),
            radius_symbol_index: self.radius_index,
            radius_squared_symbol_index: self.radius_squared_index,
        };
        (variables, piecewise, symbols)
    }
}

impl ContinuousRadiusVariableRegistration {
    /// 根据求解解向量提取选中分段 / Extract selected segment from solver solution vector
    pub fn selected_segment_index(&self, solution: &[f64]) -> Option<usize> {
        self.selected_segment_index_checked(solution).ok().flatten()
    }

    /// 校验并提取选中分段 / Validate and extract the selected segment
    pub fn selected_segment_index_checked(
        &self,
        solution: &[f64],
    ) -> Result<Option<usize>, String> {
        if self.breakpoints.is_empty() {
            return Err("continuous radius registration has no PWL breakpoints".to_string());
        }
        let expected_segments = self.breakpoints.len().saturating_sub(1);
        if self.segment_indices.len() != expected_segments
            || self.left_lambda_indices.len() != expected_segments
            || self.right_lambda_indices.len() != expected_segments
        {
            return Err(format!(
                "continuous radius registration has inconsistent PWL layout: {} breakpoints, {} segments",
                self.breakpoints.len(),
                self.segment_indices.len(),
            ));
        }
        if self.breakpoints.iter().any(|value| !value.is_finite())
            || self
                .breakpoints
                .windows(2)
                .any(|window| window[0] >= window[1])
        {
            return Err("continuous radius registration has invalid PWL breakpoints".to_string());
        }
        let mut selected = None;
        let mut selected_value = f64::NEG_INFINITY;
        for (segment_index, &model_index) in self.segment_indices.iter().enumerate() {
            let value = solution.get(model_index).copied().ok_or_else(|| {
                format!(
                    "continuous radius segment {} references missing solver value at index {}",
                    segment_index, model_index,
                )
            })?;
            if !value.is_finite() {
                return Err(format!(
                    "continuous radius segment {} has non-finite solver value {}",
                    segment_index, value,
                ));
            }
            if value > 0.5 && value > selected_value {
                selected = Some(segment_index);
                selected_value = value;
            }
        }
        Ok(selected)
    }

    /// 检查分段索引是否属于已注册 PWL / Check whether a segment index belongs to the registered PWL
    pub fn accepts_segment_index(&self, segment_index: usize) -> bool {
        segment_index < self.segment_indices.len()
    }

    /// 评估注册 PWL 的半径平方 / Evaluate registered PWL radius squared
    pub fn evaluate_radius_squared(&self, radius: f64) -> Option<f64> {
        if !radius.is_finite()
            || self.breakpoints.is_empty()
            || self.breakpoints.iter().any(|value| !value.is_finite())
            || self
                .breakpoints
                .windows(2)
                .any(|window| window[0] >= window[1])
        {
            return None;
        }
        if self.breakpoints.len() == 1 {
            return finite_square(self.breakpoints[0]);
        }
        if radius <= self.breakpoints[0] {
            return finite_square(self.breakpoints[0]);
        }
        for window in self.breakpoints.windows(2) {
            let left = window[0];
            let right = window[1];
            if radius <= right {
                let width = right - left;
                if width <= 0.0 {
                    return None;
                }
                let ratio = (radius - left) / width;
                let result = left * left + ratio * (right * right - left * left);
                return result.is_finite().then_some(result);
            }
        }
        self.breakpoints
            .last()
            .and_then(|value| finite_square(*value))
    }
}

fn finite_square(value: f64) -> Option<f64> {
    let squared = value * value;
    squared.is_finite().then_some(squared)
}

/// 连续半径建模注册结果 / Continuous radius model registration result
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ContinuousRadiusModelRegistration {
    /// 变量注册列表 / Variable registrations
    pub variables: Vec<ContinuousRadiusVariableRegistration>,
    /// 显式半径变量字段 / Explicit radius variable fields
    pub radius_variables: Vec<ContinuousRadiusVariables>,
    /// 显式分段变量字段 / Explicit piecewise variable fields
    pub piecewise_variables: Vec<ContinuousRadiusPiecewiseVariables>,
    /// 显式符号字段 / Explicit symbol fields
    pub symbols: Vec<ContinuousRadiusSymbols>,
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
