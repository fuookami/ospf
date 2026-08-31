    #[test]
    fn continuous_radius_model_component_info() {
        let component = ContinuousRadiusModelComponent {
            prototypes: vec![ContinuousCylinderRadiusSolverPrototype {
                item_id: "c1".to_string(),
                source: "test".to_string(),
                axis: Axis3::Y,
                variable_name: "r_1".to_string(),
                radius_lower_bound: Some(1.0),
                radius_upper_bound: Some(3.0),
            }],
            registration_plan: ContinuousRadiusRegistrationPlan {
                variable_names: vec!["r_1".to_string()],
                registered_variables: vec!["r_1".to_string()],
                blocked_variables: vec![],
            },
            weight_functions: HashMap::new(),
            objective_policy: Default::default(),
        };
        let info = component.info();
        assert_eq!(info[0].1, "1");
        assert!(info.iter().any(|(key, value)| *key == "weight_function_count" && value == "0"));
    }

    #[test]
    fn continuous_radius_solution_updates_item_shape() {
        let component = ContinuousRadiusModelComponent {
            prototypes: vec![ContinuousCylinderRadiusSolverPrototype {
                item_id: "c1".to_string(),
                source: "weight".to_string(),
                axis: Axis3::Y,
                variable_name: "r_c1_weight".to_string(),
                radius_lower_bound: Some(1.0),
                radius_upper_bound: Some(3.0),
            }],
            registration_plan: ContinuousRadiusRegistrationPlan {
                variable_names: vec!["r_c1_weight".to_string()],
                registered_variables: vec!["r_c1_weight".to_string()],
                blocked_variables: vec![],
            },
            weight_functions: HashMap::new(),
            objective_policy: Default::default(),
        };
        let info = HashMap::from([
            (
                ContinuousRadiusModelComponent::solution_info_key("r_c1_weight"),
                "2.25".to_string(),
            ),
            (
                ContinuousRadiusModelComponent::segment_info_key("r_c1_weight"),
                "2".to_string(),
            ),
        ]);
        let (solutions, diagnostics) = component.selected_solutions_from_info(&info);
        assert!(diagnostics.is_empty());
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0].radius, 2.25);
        assert_eq!(solutions[0].radius_squared, None);
        assert_eq!(solutions[0].segment_index, Some(2));

        let mut items = vec![ActualItem {
            id: "c1".to_string(),
            name: "Cylinder".to_string(),
            package_code: None,
            pack: None,
            width: meters(6.0),
            height: meters(5.0),
            depth: meters(6.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: Some(PackageShapeSpec::Cylinder {
                axis: Axis3::Y,
                radius: meters(3.0),
                radius_candidates: None,
                radius_lower_bound: Some(meters(1.0)),
                radius_upper_bound: Some(meters(3.0)),
            }),
        }];

        let diagnostics = component.apply_solutions_to_items(&mut items, &solutions);
        assert!(diagnostics.is_empty());
        let shape = items[0].packing_shape();
        assert_eq!(shape.radius.unwrap().value, 2.25);
        assert_eq!(shape.bounding_width.value, 4.5);
        assert_eq!(shape.bounding_depth.value, 4.5);
    }

    #[test]
    fn continuous_radius_solution_rejects_out_of_bounds_info() {
        let component = ContinuousRadiusModelComponent {
            prototypes: vec![ContinuousCylinderRadiusSolverPrototype {
                item_id: "c1".to_string(),
                source: "weight".to_string(),
                axis: Axis3::Y,
                variable_name: "r_c1_weight".to_string(),
                radius_lower_bound: Some(1.0),
                radius_upper_bound: Some(3.0),
            }],
            registration_plan: ContinuousRadiusRegistrationPlan::default(),
            weight_functions: HashMap::new(),
            objective_policy: Default::default(),
        };
        let info = HashMap::from([(
            ContinuousRadiusModelComponent::solution_info_key("r_c1_weight"),
            "4.0".to_string(),
        )]);

        let (solutions, diagnostics) = component.selected_solutions_from_info(&info);

        assert!(solutions.is_empty());
        assert!(diagnostics[0].contains("outside bounds"));
    }

    #[test]
    fn continuous_radius_registers_pwl_solver_model_and_extracts_primal() {
        let component = ContinuousRadiusModelComponent {
            prototypes: vec![ContinuousCylinderRadiusSolverPrototype {
                item_id: "c1".to_string(),
                source: "weight".to_string(),
                axis: Axis3::Y,
                variable_name: "r_c1_weight".to_string(),
                radius_lower_bound: Some(1.0),
                radius_upper_bound: Some(3.0),
            }],
            registration_plan: ContinuousRadiusRegistrationPlan {
                variable_names: vec!["r_c1_weight".to_string()],
                registered_variables: vec!["r_c1_weight".to_string()],
                blocked_variables: Vec::new(),
            },
            weight_functions: HashMap::new(),
            objective_policy: Default::default(),
        };
        let mut model = MetaModel::<f64>::new("continuous_radius_pwl");

        let registration = component.register_solver_model(&mut model).unwrap();

        assert_eq!(registration.variable_count(), 1);
        assert_eq!(registration.constraint_count, 11);
        assert_eq!(registration.objective_term_count, 1);
        assert_eq!(model.num_constraints(), 11);
        assert_eq!(model.num_tokens(), 26);
        assert_eq!(model.objective().sub_objectives.len(), 1);
        assert_eq!(
            model.objective().sub_objectives[0].name,
            "continuous_radius_squared_tie_breaker",
        );
        let variable = &registration.variables[0];
        assert_eq!(variable.segment_indices.len(), 8);
        assert!(model
            .tokens()
            .iter()
            .any(|token| token.name() == "r_c1_weight_radius"));
        assert!(model
            .tokens()
            .iter()
            .any(|token| token.name() == "r_c1_weight_radius_squared"));

        let mut solution = vec![0.0; model.num_tokens()];
        solution[variable.radius_index] = 2.0;
        solution[variable.radius_squared_index] = variable.evaluate_radius_squared(2.0).unwrap();
        solution[variable.segment_indices[4]] = 1.0;
        solution[variable.left_lambda_indices[4]] = 1.0;

        let (solutions, diagnostics) =
            component.selected_solutions_from_primal(&registration, &solution);

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(solutions.len(), 1);
        assert_eq!(solutions[0].radius, 2.0);
        assert_eq!(
            solutions[0].radius_squared,
            Some(variable.evaluate_radius_squared(2.0).unwrap()),
        );
        assert_eq!(solutions[0].segment_index, Some(4));
    }

    #[test]
    fn continuous_radius_registers_business_weight_objective() {
        let component = ContinuousRadiusModelComponent {
            prototypes: vec![ContinuousCylinderRadiusSolverPrototype {
                item_id: "c1".to_string(),
                source: "cylinder_weight".to_string(),
                axis: Axis3::Y,
                variable_name: "r_c1_weight".to_string(),
                radius_lower_bound: Some(1.0),
                radius_upper_bound: Some(3.0),
            }],
            registration_plan: ContinuousRadiusRegistrationPlan {
                variable_names: vec!["r_c1_weight".to_string()],
                registered_variables: vec!["r_c1_weight".to_string()],
                blocked_variables: Vec::new(),
            },
            weight_functions: HashMap::from([(
                "cylinder_weight".to_string(),
                ContinuousRadiusWeightFunction::new("cylinder_weight", 2.0, 3.5, 7.0),
            )]),
            objective_policy: Default::default(),
        };
        let mut model = MetaModel::<f64>::new("continuous_radius_business_weight");

        let registration = component.register_solver_model(&mut model).unwrap();

        assert_eq!(registration.objective_term_count, 1);
        assert_eq!(registration.business_objective_term_count, 1);
        assert_eq!(registration.tie_breaker_objective_term_count, 0);
        assert_eq!(registration.matched_weight_function_count, 1);
        assert_eq!(model.objective().sub_objectives.len(), 1);
        assert_eq!(
            model.objective().sub_objectives[0].name,
            "continuous_radius_business_weight_0",
        );
        assert_eq!(model.objective().sub_objectives[0].weight, 7.0);
        let monomials = model.objective().sub_objectives[0].polynomial.monomials();
        assert_eq!(monomials.len(), 1);
        assert_eq!(*monomials[0].coefficient(), 3.5);
        assert_eq!(
            monomials[0].var_index(),
            registration.variables[0].radius_squared_index,
        );

        let solution = component.prototypes[0].selected_solution(2.0, Some(4.0), Some(3));
        assert_eq!(
            component.evaluate_selected_solution_weight(&solution),
            Some(16.0),
        );
    }

