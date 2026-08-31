//! 管道定义
//! Pipeline Definitions
//!
//! 本模块提供三种管道 trait：
//! This module provides three pipeline traits:
//!
//! - [`Pipeline`] - 基础管道，用于模型构建和约束添加 / Basic pipeline for model building and constraint addition
//! - [`CGPipeline`] - 列生成管道，支持 Shadow Price 管理 / Column generation pipeline with Shadow Price management
//! - [`HAPipeline`] - 启发式算法管道，用于解的评估 / Heuristic algorithm pipeline for solution evaluation

use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;
use ospf_rust_core::error::{CoreError, ModelError, Result, SolverError};
use ospf_rust_core::model::mechanism::ConstraintGroup;

/// 基础管道 trait / Basic Pipeline Trait
///
/// 用于模型构建和约束添加。
/// For model building and constraint addition.
///
/// # 类型参数 / Type Parameters
///
/// - `M` - 模型类型 / Model type
pub trait Pipeline<M>: Send + Sync {
    /// 获取管道名称 / Get pipeline name
    fn name(&self) -> &str;

    /// 获取约束组 / Get constraint group
    fn constraint_group(&self) -> Option<&ConstraintGroup>;

    /// 注册模型 / Register model
    ///
    /// 将管道注册到模型中，用于约束组管理。
    /// Registers the pipeline to the model for constraint group management.
    fn register(&self, model: &mut M) {
        // 默认实现：如果模型支持约束组，则注册
        // Default implementation: register if model supports constraint groups
        let _ = model; // 避免未使用警告 / Avoid unused warning
    }

    /// 执行管道 / Execute pipeline
    ///
    /// 执行管道的主要逻辑，如添加约束。
    /// Executes the main logic of the pipeline, such as adding constraints.
    fn invoke(&self, model: &M) -> Result<()>;

    /// 获取线性不可行原因 / Get linear infeasible reasons
    ///
    /// 当模型不可行时，返回可能的原因列表。
    /// When the model is infeasible, returns possible reasons.
    fn linear_infeasible_reasons(&self, model: &M) -> Vec<String> {
        let _ = model;
        Vec::new()
    }

    /// 获取二次不可行原因 / Get quadratic infeasible reasons
    ///
    /// 当模型不可行时，返回可能的原因列表。
    /// When the model is infeasible, returns possible reasons.
    fn quadratic_infeasible_reasons(&self, model: &M) -> Vec<String> {
        let _ = model;
        Vec::new()
    }
}

/// 管道列表执行扩展 / Pipeline List Execution Extension
pub trait PipelineList<M>: Send + Sync {
    /// 执行所有管道 / Execute all pipelines
    fn invoke_all(&self, model: &mut M) -> Result<()>;
}

fn maybe_register_constraint_group<M>(model: &mut M, group: &ConstraintGroup) -> Result<()>
where
    M: 'static,
{
    let any = model as &mut dyn Any;
    if let Some(meta_model) = any.downcast_mut::<ospf_rust_core::model::MetaModel<f64>>() {
        match meta_model.create_constraint_group(group.id, &group.name) {
            Ok(_) => {}
            Err(CoreError::Model(ModelError::ConstraintConflict(_))) => {}
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

impl<M> PipelineList<M> for Vec<Arc<dyn Pipeline<M>>>
where
    M: 'static,
{
    fn invoke_all(&self, model: &mut M) -> Result<()> {
        for pipeline in self {
            if let Some(group) = pipeline.constraint_group() {
                maybe_register_constraint_group(model, group)?;
            }
            pipeline.register(model);
            pipeline.invoke(model)?;
        }
        Ok(())
    }
}

/// 约束组注册能力 / Constraint-group registration capability
pub trait ConstraintGroupRegistrar {
    /// 确保约束组已注册 / Ensure constraint group is registered
    fn ensure_constraint_group(&mut self, group: &ConstraintGroup) -> Result<()>;
}

impl<V> ConstraintGroupRegistrar for ospf_rust_core::model::MetaModel<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    fn ensure_constraint_group(&mut self, group: &ConstraintGroup) -> Result<()> {
        match self.create_constraint_group(group.id, &group.name) {
            Ok(_) => Ok(()),
            Err(CoreError::Model(ModelError::ConstraintConflict(_))) => Ok(()),
            Err(err) => Err(err),
        }
    }
}

/// 具备约束组注册的管道执行扩展 / Pipeline execution extension with group registration
pub trait PipelineListWithGroupRegistration<M>: Send + Sync {
    /// 先自动注册约束组，再执行所有管道
    /// Automatically register constraint groups before invoking all pipelines
    fn invoke_all_with_group_registration(&self, model: &mut M) -> Result<()>
    where
        M: ConstraintGroupRegistrar;
}

impl<M> PipelineListWithGroupRegistration<M> for Vec<Arc<dyn Pipeline<M>>> {
    fn invoke_all_with_group_registration(&self, model: &mut M) -> Result<()>
    where
        M: ConstraintGroupRegistrar,
    {
        for pipeline in self {
            if let Some(group) = pipeline.constraint_group() {
                model.ensure_constraint_group(group)?;
            }
            pipeline.register(model);
            pipeline.invoke(model)?;
        }
        Ok(())
    }
}

/// 列生成管道 trait / Column Generation Pipeline Trait
///
/// 扩展基础管道，支持 Shadow Price 管理。
/// Extends basic pipeline with shadow price management support.
///
/// # 类型参数 / Type Parameters
///
/// - `Args` - 参数类型 / Argument type
/// - `M` - 模型类型 / Model type
/// - `Map` - Shadow Price 映射表类型 / Shadow price map type
pub trait CGPipeline<Args, M, Map>: Pipeline<M>
where
    Args: Send + Sync + 'static,
    Map: Send + Sync,
{
    /// Shadow Price 提取器类型 / Shadow price extractor type
    type Extractor: Fn(&Map, &Args) -> f64 + Send + Sync + 'static;

    /// 获取 Shadow Price 提取器 / Get shadow price extractor
    ///
    /// 返回一个函数，用于从映射表中提取 Shadow Price。
    /// Returns a function to extract shadow price from the map.
    fn extractor(&self) -> Option<Self::Extractor> {
        None
    }

    /// 刷新 Shadow Price / Refresh shadow price
    ///
    /// 根据对偶解更新 Shadow Price 映射表。
    /// Updates shadow price map based on dual solution.
    fn refresh(&self, shadow_price_map: &mut Map, model: &M, shadow_prices: &[f64]) -> Result<()>;
}

/// 启发式算法管道目标值 / Heuristic Algorithm Pipeline Objective Value
#[derive(Debug, Clone)]
pub struct HAPipelineObj {
    /// 标签 / Tag
    pub tag: String,
    /// 目标值 / Objective value
    pub value: f64,
}

impl HAPipelineObj {
    /// 创建新的目标值 / Create new objective value
    pub fn new(tag: impl Into<String>, value: f64) -> Self {
        Self {
            tag: tag.into(),
            value,
        }
    }
}

/// 启发式算法管道 trait / Heuristic Algorithm Pipeline Trait
///
/// 用于启发式算法中解的评估。
/// For solution evaluation in heuristic algorithms.
///
/// # 类型参数 / Type Parameters
///
/// - `M` - 模型类型 / Model type
pub trait HAPipeline<M>: Pipeline<M> {
    /// 计算目标值 / Calculate objective value
    ///
    /// 根据给定解计算目标值。
    /// Calculates objective value based on given solution.
    fn calculate(&self, model: &M, solution: &[f64]) -> Result<Option<f64>>;

    /// 检查解 / Check solution
    ///
    /// 检查给定解是否有效。
    /// Checks if the given solution is valid.
    fn check(&self, model: &M, solution: &[f64]) -> Result<()> {
        self.calculate(model, solution)?
            .map(|_| ())
            .ok_or_else(|| CoreError::Solver(SolverError::NoSolution))
    }

    /// 执行管道并返回目标值 / Execute pipeline and return objective value
    fn invoke_with_solution(&self, model: &M, solution: &[f64]) -> Result<HAPipelineObj> {
        let value = self
            .calculate(model, solution)?
            .ok_or_else(|| CoreError::Solver(SolverError::NoSolution))?;
        Ok(HAPipelineObj::new(self.name(), value))
    }
}

/// 启发式算法管道列表 / Heuristic Algorithm Pipeline List
pub trait HAPipelineList<M>: Send + Sync {
    /// 执行所有管道并返回目标值列表 / Execute all pipelines and return objective values
    fn invoke_all_with_solution(&self, model: &M, solution: &[f64]) -> Result<Vec<HAPipelineObj>>;
}

impl<M> HAPipelineList<M> for Vec<Arc<dyn HAPipeline<M>>> {
    fn invoke_all_with_solution(&self, model: &M, solution: &[f64]) -> Result<Vec<HAPipelineObj>> {
        self.iter()
            .map(|pipeline| pipeline.invoke_with_solution(model, solution))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_core::model::MetaModel;
    use std::sync::{Arc, Mutex};

    struct TestModel;

    impl ConstraintGroupRegistrar for TestModel {
        fn ensure_constraint_group(&mut self, _group: &ConstraintGroup) -> Result<()> {
            Ok(())
        }
    }

    struct TestPipeline {
        name: String,
        group: Option<ConstraintGroup>,
    }

    impl Pipeline<TestModel> for TestPipeline {
        fn name(&self) -> &str {
            &self.name
        }

        fn constraint_group(&self) -> Option<&ConstraintGroup> {
            self.group.as_ref()
        }

        fn invoke(&self, _model: &TestModel) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_pipeline_list() {
        let pipelines: Vec<Arc<dyn Pipeline<TestModel>>> = vec![
            Arc::new(TestPipeline {
                name: "pipeline1".to_string(),
                group: None,
            }),
            Arc::new(TestPipeline {
                name: "pipeline2".to_string(),
                group: None,
            }),
        ];

        let mut model = TestModel;
        let result = pipelines.invoke_all(&mut model);
        assert!(result.is_ok());
    }

    struct TrackingModel {
        registered_group_ids: Arc<Mutex<Vec<u64>>>,
    }

    impl ConstraintGroupRegistrar for TrackingModel {
        fn ensure_constraint_group(&mut self, group: &ConstraintGroup) -> Result<()> {
            self.registered_group_ids.lock().unwrap().push(group.id);
            Ok(())
        }
    }

    impl Pipeline<TrackingModel> for TestPipeline {
        fn name(&self) -> &str {
            &self.name
        }

        fn constraint_group(&self) -> Option<&ConstraintGroup> {
            self.group.as_ref()
        }

        fn invoke(&self, _model: &TrackingModel) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_pipeline_list_with_group_registration() {
        let pipelines: Vec<Arc<dyn Pipeline<TrackingModel>>> = vec![Arc::new(TestPipeline {
            name: "pipeline_with_group".to_string(),
            group: Some(ConstraintGroup::new(88, "cg_group")),
        })];

        let registered = Arc::new(Mutex::new(Vec::new()));
        let mut model = TrackingModel {
            registered_group_ids: registered.clone(),
        };
        let result = pipelines.invoke_all_with_group_registration(&mut model);
        assert!(result.is_ok());
        assert_eq!(registered.lock().unwrap().as_slice(), &[88]);
    }

    struct MetaModelPipeline {
        name: String,
        group: Option<ConstraintGroup>,
    }

    impl Pipeline<MetaModel<f64>> for MetaModelPipeline {
        fn name(&self) -> &str {
            &self.name
        }

        fn constraint_group(&self) -> Option<&ConstraintGroup> {
            self.group.as_ref()
        }

        fn invoke(&self, _model: &MetaModel<f64>) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_pipeline_list_auto_registers_constraint_group_for_meta_model() {
        let group = ConstraintGroup::new(1001, "auto_registered_group");
        let pipelines: Vec<Arc<dyn Pipeline<MetaModel<f64>>>> = vec![Arc::new(MetaModelPipeline {
            name: "meta_pipeline".to_string(),
            group: Some(group.clone()),
        })];
        let mut model = MetaModel::<f64>::new("meta_pipeline_model");

        let result = pipelines.invoke_all(&mut model);
        assert!(result.is_ok());
        // 若默认 invoke_all 已自动注册约束组，此处再次创建同 ID 应报冲突。
        // If invoke_all auto-registers the group, creating the same id again must conflict.
        assert!(matches!(
            model.create_constraint_group(group.id, &group.name),
            Err(CoreError::Model(ModelError::ConstraintConflict(_)))
        ));
    }
}
