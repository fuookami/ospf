#[cfg(feature = "serde")]
#[test]
fn csv_materialized_continuous_radius_renders_solver_selected_radius() {
    use crate::infrastructure::renderer::{RenderAlgorithmShapeType, RenderAxis3};

    #[derive(Debug, Clone)]
    struct SelectedRadiusFinalExecutor;

    impl ColumnGenerationFinalExecutor for SelectedRadiusFinalExecutor {
        fn execute(
            &self,
            state: &ColumnGenerationApplicationState,
        ) -> ColumnGenerationFinalExecution {
            let mut execution = MetaModelFinalExecutor::new(MetaModelFinalExecutorConfig {
                model_name: "pwl_selected_radius_final".to_string(),
                primal_solution: vec![1.0, 1.0],
                objective: Some(1.0),
            })
            .execute(state);
            execution.info.insert(
                "continuous_radius_selected_r_c1_cylinder_weight".to_string(),
                "2.0".to_string(),
            );
            execution.info.insert(
                "continuous_radius_selected_radius_squared_r_c1_cylinder_weight".to_string(),
                "4.0".to_string(),
            );
            execution.info.insert(
                "continuous_radius_selected_segment_r_c1_cylinder_weight".to_string(),
                "1".to_string(),
            );
            execution
        }
    }

    let input = r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount,radius_min,radius_max,radius_step,radius_weight_function_key,axis
c1,Cylinder,cylinder,6,5,6,1,1,1,3,1,cylinder_weight,Y
# table:bins
bin_id,type_code,width,height,depth,capacity
b1,BIN,10,10,10,1000
# table:layers
layer_id,bin_id,depth
l1,b1,5
"#;
    let request = crate::application::csv::CsvDatasetLoader::load_str(input)
        .unwrap()
        .materialize()
        .unwrap();
    let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
    let rmp = MetaModelRmpExecutor::new(MetaModelRmpExecutorConfig {
        model_name: "pwl_rmp".to_string(),
        shadow_prices: vec![1.0],
        primal_solution: vec![1.0],
        objective: Some(1.0),
    });
    let final_executor = SelectedRadiusFinalExecutor;

    let result = service
        .run_csv_materialized_with_bins(request, &rmp, &final_executor)
        .unwrap();

    let item = &result.result.render_loading_plans[0].items[0];
    assert_eq!(
        item.algorithm_shape_type,
        RenderAlgorithmShapeType::VerticalCylinder
    );
    assert_eq!(item.axis, Some(RenderAxis3::Y));
    assert_eq!(item.radius, Some(2.0));
    assert_eq!(result.selected_radius_solutions.len(), 1);
    assert_eq!(result.selected_radius_solutions[0].segment_index, Some(1));
    assert_eq!(
        result.result.info["continuous_radius_selected_r_c1_cylinder_weight"],
        "2"
    );
    assert_eq!(
        result.result.info["continuous_radius_selected_radius_squared_r_c1_cylinder_weight"],
        "4"
    );
    assert!(
        result
            .result
            .info
            .values()
            .any(|value| value.contains("selected_layers_renderable"))
    );
    assert_eq!(result.result.info["continuous_radius_prototype_count"], "1");
}

#[cfg(feature = "serde")]
#[test]
fn csv_materialized_continuous_radius_uses_solver_primal_model_values() {
    use crate::infrastructure::pwl_approximation::{
        PwlRadiusApproximationConfig, PwlRadiusSquaredApproximation,
    };
    use crate::infrastructure::renderer::{RenderAlgorithmShapeType, RenderAxis3};

    #[derive(Debug, Clone)]
    struct PwlPrimalBackend;

    impl MetaModelSolverBackend for PwlPrimalBackend {
        fn name(&self) -> &str {
            "pwl_primal"
        }

        fn solve_rmp(
            &self,
            _model: &MetaModel<f64>,
            diagnostics: &MetaModelExecutionDiagnostics,
        ) -> Result<MetaModelExecutorSolveResult, String> {
            Ok(MetaModelExecutorSolveResult {
                objective: Some(1.0),
                primal_solution: vec![1.0; diagnostics.variable_count],
                dual_solution: vec![1.0; diagnostics.demand_count],
                info: HashMap::from([("backend_phase".to_string(), "rmp".to_string())]),
            })
        }

        fn solve_final(
            &self,
            model: &MetaModel<f64>,
            diagnostics: &MetaModelExecutionDiagnostics,
        ) -> Result<MetaModelExecutorSolveResult, String> {
            let mut primal_solution = vec![0.0; diagnostics.variable_count];
            let approximation = PwlRadiusSquaredApproximation::try_from_radius_interval(
                1.0,
                3.0,
                &PwlRadiusApproximationConfig::default(),
            )
            .map_err(|error| error.to_string())?;
            for token in model.tokens() {
                match token.name() {
                    "x_0_0" | "v_0" => primal_solution[token.solver_index] = 1.0,
                    "r_c1_cylinder_weight_radius" => {
                        primal_solution[token.solver_index] = 2.0;
                    }
                    "r_c1_cylinder_weight_radius_squared" => {
                        primal_solution[token.solver_index] = approximation.evaluate(2.0);
                    }
                    "r_c1_cylinder_weight_seg_4" => {
                        primal_solution[token.solver_index] = 1.0;
                    }
                    "r_c1_cylinder_weight_seg_4_lambda_left" => {
                        primal_solution[token.solver_index] = 1.0;
                    }
                    _ => {}
                }
            }
            Ok(MetaModelExecutorSolveResult {
                objective: Some(1.0),
                primal_solution,
                dual_solution: Vec::new(),
                info: HashMap::from([("backend_phase".to_string(), "final".to_string())]),
            })
        }
    }

    let input = r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount,radius_min,radius_max,radius_step,radius_weight_function_key,axis
c1,Cylinder,cylinder,6,5,6,1,1,1,3,1,cylinder_weight,Y
# table:bins
bin_id,type_code,width,height,depth,capacity
b1,BIN,10,10,10,1000
# table:layers
layer_id,bin_id,depth
l1,b1,5
"#;
    let request = crate::application::csv::CsvDatasetLoader::load_str(input)
        .unwrap()
        .materialize()
        .unwrap();
    let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
    let rmp = SolverBackedMetaModelRmpExecutor::new(
        MetaModelRmpExecutorConfig {
            model_name: "pwl_primal_rmp".to_string(),
            ..Default::default()
        },
        PwlPrimalBackend,
    );
    let final_executor = SolverBackedMetaModelFinalExecutor::new(
        MetaModelFinalExecutorConfig {
            model_name: "pwl_primal_final".to_string(),
            ..Default::default()
        },
        PwlPrimalBackend,
    );

    let result = service
        .run_csv_materialized_with_bins(request, &rmp, &final_executor)
        .unwrap();

    let item = &result.result.render_loading_plans[0].items[0];
    assert_eq!(
        item.algorithm_shape_type,
        RenderAlgorithmShapeType::VerticalCylinder
    );
    assert_eq!(item.axis, Some(RenderAxis3::Y));
    assert_eq!(item.radius, Some(2.0));
    assert_eq!(result.selected_radius_solutions.len(), 1);
    assert_eq!(result.selected_radius_solutions[0].radius, 2.0);
    assert_eq!(
        result.selected_radius_solutions[0].radius_squared,
        Some(4.0),
    );
    assert_eq!(result.selected_radius_solutions[0].segment_index, Some(4));
    assert_eq!(
        result.final_execution.info["continuous_radius_model_variable_count"],
        "1"
    );
    assert_eq!(
        result.final_execution.info["continuous_radius_model_constraint_count"],
        "11"
    );
    assert_eq!(
        result.final_execution.info["continuous_radius_model_objective_term_count"],
        "1"
    );
    assert_eq!(
        result.final_execution.info["continuous_radius_selected_r_c1_cylinder_weight"],
        "2"
    );
    assert_eq!(
        result.final_execution.info["continuous_radius_selected_radius_squared_r_c1_cylinder_weight"],
        "4"
    );
}
