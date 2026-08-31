//! 统一求解参数对象
//! Unified solve options object

use super::column_generation::{RegistrationStatusCallback, SolvingStatusCallback};
use ospf_rust_core::model::ModelBuildingStatusCallback;
use ospf_rust_core::solver::SolveValueConversionPolicy;

/// 统一求解参数 / Unified solve options
#[derive(Clone)]
pub struct SolveOptions {
    /// 自定义求解名称 / Custom solve name
    pub name: Option<String>,
    /// 是否记录模型 / Whether to log model
    pub to_log_model: bool,
    /// 期望的解数量（多解接口）/ Expected solution amount (for multi-solution APIs)
    pub solution_amount: usize,
    /// 建模状态回调 / Model-building status callback
    pub model_building_status_callback: Option<ModelBuildingStatusCallback>,
    /// 注册状态回调 / Registration status callback
    pub registration_status_callback: Option<RegistrationStatusCallback>,
    /// 求解状态回调 / Solving status callback
    pub solving_status_callback: Option<SolvingStatusCallback>,
    /// Benders 最大迭代次数 / Benders max iterations
    pub max_iterations: usize,
    /// Benders 收敛容忍度 / Benders convergence tolerance
    pub tolerance: f64,
    /// Benders 停滞窗口（连续无新 cut 的最大轮数）/
    /// Benders stall window (max consecutive iterations without new cuts)
    pub max_stall_iterations: Option<usize>,
    /// Benders 目标改进停滞窗口（连续改进低于 tolerance 的最大轮数）/
    /// Benders objective stall window (max consecutive iterations with improvement below tolerance)
    pub objective_stall_iterations: Option<usize>,
    /// 数值转换策略 / Numeric conversion policy
    pub value_conversion_policy: SolveValueConversionPolicy,
}

impl Default for SolveOptions {
    fn default() -> Self {
        Self {
            name: None,
            to_log_model: false,
            solution_amount: 1,
            model_building_status_callback: None,
            registration_status_callback: None,
            solving_status_callback: None,
            max_iterations: 100,
            tolerance: 1e-9,
            max_stall_iterations: None,
            objective_stall_iterations: Some(1),
            value_conversion_policy: SolveValueConversionPolicy::Strict,
        }
    }
}

impl SolveOptions {
    /// 创建默认参数 / Create default options
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置求解名称 / Set solve name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// 设置模型日志开关 / Set model logging flag
    pub fn with_log_model(mut self, to_log_model: bool) -> Self {
        self.to_log_model = to_log_model;
        self
    }

    /// 设置解数量 / Set solution amount
    pub fn with_solution_amount(mut self, solution_amount: usize) -> Self {
        self.solution_amount = solution_amount.max(1);
        self
    }

    /// 设置建模回调 / Set model-building callback
    pub fn with_building_callback(mut self, callback: Option<ModelBuildingStatusCallback>) -> Self {
        self.model_building_status_callback = callback;
        self
    }

    /// 设置注册回调 / Set registration callback
    pub fn with_registration_callback(
        mut self,
        callback: Option<RegistrationStatusCallback>,
    ) -> Self {
        self.registration_status_callback = callback;
        self
    }

    /// 设置求解回调 / Set solving callback
    pub fn with_solving_callback(mut self, callback: Option<SolvingStatusCallback>) -> Self {
        self.solving_status_callback = callback;
        self
    }

    /// 设置 Benders 迭代参数 / Set Benders iteration options
    pub fn with_iterations(mut self, max_iterations: usize, tolerance: f64) -> Self {
        self.max_iterations = max_iterations;
        self.tolerance = tolerance;
        self
    }

    /// 设置 Benders 停滞窗口 / Set Benders stall window
    pub fn with_stall_iterations(mut self, max_stall_iterations: usize) -> Self {
        self.max_stall_iterations = Some(max_stall_iterations.max(1));
        self
    }

    /// 设置 Benders 目标改进停滞窗口 / Set Benders objective stall window
    pub fn with_objective_stall_iterations(mut self, objective_stall_iterations: usize) -> Self {
        self.objective_stall_iterations = Some(objective_stall_iterations.max(1));
        self
    }

    /// 设置数值转换策略 / Set numeric conversion policy
    pub fn with_value_conversion_policy(
        mut self,
        value_conversion_policy: SolveValueConversionPolicy,
    ) -> Self {
        self.value_conversion_policy = value_conversion_policy;
        self
    }
}
