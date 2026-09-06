#[test]
fn continuous_radius_model_component_info() {
    let component = ContinuousRadiusModelComponent {
        prototypes: vec![ContinuousCylinderRadiusSolverPrototype {
            item_id: "c1".into(),
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
    assert!(
        info.iter()
            .any(|(key, value)| *key == "weight_function_count" && value == "0")
    );
}

#[test]
fn continuous_radius_solution_updates_item_shape() {
    let component = ContinuousRadiusModelComponent {
        prototypes: vec![ContinuousCylinderRadiusSolverPrototype {
            item_id: "c1".into(),
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
            "5".to_string(),
        ),
    ]);
    let (solutions, diagnostics) = component.selected_solutions_from_info(&info);
    assert!(diagnostics.is_empty());
    assert_eq!(solutions.len(), 1);
    assert_eq!(solutions[0].radius, 2.25);
    assert_eq!(solutions[0].radius_squared, None);
    assert_eq!(solutions[0].segment_index, Some(5));

    let mut items = vec![ActualItem {
        id: "c1".into(),
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
            item_id: "c1".into(),
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
            item_id: "c1".into(),
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
    assert!(
        model
            .tokens()
            .iter()
            .any(|token| token.name() == "r_c1_weight_radius")
    );
    assert!(
        model
            .tokens()
            .iter()
            .any(|token| token.name() == "r_c1_weight_radius_squared")
    );

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
            item_id: "c1".into(),
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

#[test]
fn continuous_radius_registration_rolls_back_on_invalid_later_prototype() {
    let component = ContinuousRadiusModelComponent {
        prototypes: vec![
            ContinuousCylinderRadiusSolverPrototype {
                item_id: "valid".into(),
                source: "weight".to_string(),
                axis: Axis3::Y,
                variable_name: "r_valid".to_string(),
                radius_lower_bound: Some(1.0),
                radius_upper_bound: Some(3.0),
            },
            ContinuousCylinderRadiusSolverPrototype {
                item_id: "invalid".into(),
                source: "weight".to_string(),
                axis: Axis3::Y,
                variable_name: "r_invalid".to_string(),
                radius_lower_bound: None,
                radius_upper_bound: Some(3.0),
            },
        ],
        registration_plan: ContinuousRadiusRegistrationPlan::default(),
        weight_functions: HashMap::new(),
        objective_policy: Default::default(),
    };
    let mut model = MetaModel::<f64>::new("continuous_radius_atomic");

    assert!(component.register_solver_model(&mut model).is_err());
    assert_eq!(model.num_tokens(), 0);
    assert_eq!(model.num_constraints(), 0);
    assert!(model.objective().sub_objectives.is_empty());
}

#[test]
fn continuous_radius_quantity_bounds_normalize_compatible_units() {
    use ospf_rust_quantities::unit::derived::{Centimeter, Meter};

    let prototype = ContinuousCylinderRadiusSolverPrototype {
        item_id: "c1".into(),
        source: "weight".to_string(),
        axis: Axis3::Y,
        variable_name: "r_c1_weight".to_string(),
        radius_lower_bound: None,
        radius_upper_bound: None,
    }
    .with_quantity_bounds(
        Quantity::<f64, Centimeter>::new_ct(100.0),
        Quantity::<f64, Centimeter>::new_ct(300.0),
    )
    .unwrap();

    assert_eq!(prototype.validate_bounds().unwrap(), (1.0, 3.0));

    let runtime_prototype = ContinuousCylinderRadiusSolverPrototype {
        item_id: "c2".into(),
        source: "weight".to_string(),
        axis: Axis3::Y,
        variable_name: "r_c2_weight".to_string(),
        radius_lower_bound: None,
        radius_upper_bound: None,
    }
    .with_runtime_quantity_bounds(
        Quantity::new(100.0, Centimeter::INSTANT.clone()),
        Quantity::new(3.0, Meter::INSTANT.clone()),
    )
    .unwrap();

    assert_eq!(runtime_prototype.validate_bounds().unwrap(), (1.0, 3.0));
}

#[test]
fn continuous_radius_quantity_bounds_reject_incompatible_units() {
    use ospf_rust_quantities::unit::derived::{Meter, Second};

    let prototype = ContinuousCylinderRadiusSolverPrototype {
        item_id: "c1".into(),
        source: "weight".to_string(),
        axis: Axis3::Y,
        variable_name: "r_c1_weight".to_string(),
        radius_lower_bound: None,
        radius_upper_bound: None,
    };
    let error = prototype
        .with_runtime_quantity_bounds(
            Quantity::new(1.0, Meter::INSTANT.clone()),
            Quantity::new(3.0, Second::INSTANT.clone()),
        )
        .unwrap_err();

    assert!(error.contains("incompatible with metres"), "{error}");
}

#[test]
fn continuous_radius_solution_converts_solver_metres_to_item_units() {
    use ospf_rust_quantities::unit::derived::Centimeter;

    let component = ContinuousRadiusModelComponent {
        prototypes: vec![ContinuousCylinderRadiusSolverPrototype {
            item_id: "c1".into(),
            source: "weight".to_string(),
            axis: Axis3::Y,
            variable_name: "r_c1_weight".to_string(),
            radius_lower_bound: Some(1.0),
            radius_upper_bound: Some(3.0),
        }],
        registration_plan: ContinuousRadiusRegistrationPlan::default(),
        weight_functions: HashMap::new(),
        objective_policy: ContinuousRadiusObjectivePolicy::None,
    };
    let mut item = ActualItem {
        id: "c1".into(),
        name: "Cylinder".to_string(),
        package_code: None,
        pack: None,
        width: Quantity::<f64, Centimeter>::new_ct(600.0),
        height: Quantity::<f64, Centimeter>::new_ct(500.0),
        depth: Quantity::<f64, Centimeter>::new_ct(600.0),
        weight: Quantity::<f64, Centimeter>::new_ct(1.0),
        enabled_orientations: vec![Orientation::Upright],
        shape_spec_override: Some(PackageShapeSpec::Cylinder {
            axis: Axis3::Y,
            radius: Quantity::<f64, Centimeter>::new_ct(300.0),
            radius_candidates: None,
            radius_lower_bound: Some(Quantity::<f64, Centimeter>::new_ct(100.0)),
            radius_upper_bound: Some(Quantity::<f64, Centimeter>::new_ct(300.0)),
        }),
    };
    let solution = component.prototypes[0].selected_solution(2.0, Some(4.0), None);

    component
        .apply_solution_to_item(&mut item, &solution)
        .unwrap();
    let Some(PackageShapeSpec::Cylinder { radius, .. }) = item.shape_spec_override else {
        panic!("expected cylinder shape");
    };
    assert_eq!(radius.value, 200.0);
}

#[test]
fn continuous_radius_result_filters_non_finite_values_and_invalid_segments() {
    let component = ContinuousRadiusModelComponent {
        prototypes: vec![ContinuousCylinderRadiusSolverPrototype {
            item_id: "c1".into(),
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

    let non_finite_radius = HashMap::from([(
        ContinuousRadiusModelComponent::solution_info_key("r_c1_weight"),
        "NaN".to_string(),
    )]);
    let (solutions, diagnostics) = component.selected_solutions_from_info(&non_finite_radius);
    assert!(solutions.is_empty());
    assert!(
        diagnostics
            .iter()
            .any(|message| message.contains("selected radius"))
    );

    let non_finite_radius_squared = HashMap::from([
        (
            ContinuousRadiusModelComponent::solution_info_key("r_c1_weight"),
            "2.0".to_string(),
        ),
        (
            ContinuousRadiusModelComponent::radius_squared_info_key("r_c1_weight"),
            "Infinity".to_string(),
        ),
    ]);
    let (solutions, diagnostics) =
        component.selected_solutions_from_info(&non_finite_radius_squared);
    assert!(solutions.is_empty());
    assert!(
        diagnostics
            .iter()
            .any(|message| message.contains("non-finite radius squared"))
    );

    let invalid_segment = HashMap::from([
        (
            ContinuousRadiusModelComponent::solution_info_key("r_c1_weight"),
            "2.0".to_string(),
        ),
        (
            ContinuousRadiusModelComponent::segment_info_key("r_c1_weight"),
            "999".to_string(),
        ),
    ]);
    let (solutions, diagnostics) = component.selected_solutions_from_info(&invalid_segment);
    assert!(solutions.is_empty());
    assert!(
        diagnostics
            .iter()
            .any(|message| message.contains("outside PWL range"))
    );
}

#[test]
fn continuous_radius_invalid_weight_coefficients_roll_back_registration() {
    let component = ContinuousRadiusModelComponent {
        prototypes: vec![ContinuousCylinderRadiusSolverPrototype {
            item_id: "c1".into(),
            source: "weight".to_string(),
            axis: Axis3::Y,
            variable_name: "r_c1_weight".to_string(),
            radius_lower_bound: Some(1.0),
            radius_upper_bound: Some(3.0),
        }],
        registration_plan: ContinuousRadiusRegistrationPlan::default(),
        weight_functions: HashMap::from([(
            "weight".to_string(),
            ContinuousRadiusWeightFunction::new("weight", 0.0, f64::NAN, 1.0),
        )]),
        objective_policy: Default::default(),
    };
    let mut model = MetaModel::<f64>::new("continuous_radius_invalid_weight");

    let error = component.register_solver_model(&mut model).unwrap_err();
    assert!(error.contains("non-finite"), "{error}");
    assert_eq!(model.num_tokens(), 0);
    assert_eq!(model.num_constraints(), 0);
    assert!(model.objective().sub_objectives.is_empty());
}

#[test]
fn continuous_radius_registration_rolls_back_to_existing_model_checkpoint() {
    let component = ContinuousRadiusModelComponent {
        prototypes: vec![
            ContinuousCylinderRadiusSolverPrototype {
                item_id: "valid".into(),
                source: "weight".to_string(),
                axis: Axis3::Y,
                variable_name: "r_valid".to_string(),
                radius_lower_bound: Some(1.0),
                radius_upper_bound: Some(3.0),
            },
            ContinuousCylinderRadiusSolverPrototype {
                item_id: "invalid".into(),
                source: "weight".to_string(),
                axis: Axis3::Y,
                variable_name: "r_invalid".to_string(),
                radius_lower_bound: None,
                radius_upper_bound: Some(3.0),
            },
        ],
        registration_plan: ContinuousRadiusRegistrationPlan::default(),
        weight_functions: HashMap::new(),
        objective_policy: Default::default(),
    };
    let mut model = MetaModel::<f64>::new("continuous_radius_existing_checkpoint");
    let existing_index = model
        .register_variable(UContinuousVariableItem::auto_with_range(
            "existing_radius",
            VariableRange::bounded(0.0, 10.0),
        ))
        .unwrap();
    model.tokens()[existing_index].set_result(2.5);
    model
        .add_eq_constraint(&[(existing_index, 1.0)], 2.5, "existing_constraint")
        .unwrap();
    model.add_sub_objective(SubObjective::minimize(
        Linear::new(vec![LinearMonomial::new(3.0, existing_index)], 0.0),
        "existing_objective",
    ));

    let checkpoint_tokens = model
        .tokens()
        .iter()
        .map(|token| {
            (
                token.id(),
                token.name().to_string(),
                token.solver_index,
                token.get_result(),
            )
        })
        .collect::<Vec<_>>();
    let checkpoint_constraint_count = model.num_constraints();
    let checkpoint_objective_category = model.objective().category;
    let checkpoint_objectives = model
        .objective()
        .sub_objectives
        .iter()
        .map(|objective| {
            (
                objective.name.clone(),
                objective.weight,
                objective.polynomial.monomials().len(),
            )
        })
        .collect::<Vec<_>>();

    assert!(component.register_solver_model(&mut model).is_err());

    let current_tokens = model
        .tokens()
        .iter()
        .map(|token| {
            (
                token.id(),
                token.name().to_string(),
                token.solver_index,
                token.get_result(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(current_tokens, checkpoint_tokens);
    assert_eq!(model.num_constraints(), checkpoint_constraint_count);
    assert_eq!(model.objective().category, checkpoint_objective_category);
    let current_objectives = model
        .objective()
        .sub_objectives
        .iter()
        .map(|objective| {
            (
                objective.name.clone(),
                objective.weight,
                objective.polynomial.monomials().len(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(current_objectives, checkpoint_objectives);
}

#[test]
fn continuous_radius_empty_primal_reports_each_registered_variable() {
    let component = ContinuousRadiusModelComponent {
        prototypes: vec![
            ContinuousCylinderRadiusSolverPrototype {
                item_id: "c1".into(),
                source: "weight".to_string(),
                axis: Axis3::Y,
                variable_name: "r_c1_weight".to_string(),
                radius_lower_bound: Some(1.0),
                radius_upper_bound: Some(3.0),
            },
            ContinuousCylinderRadiusSolverPrototype {
                item_id: "c2".into(),
                source: "weight".to_string(),
                axis: Axis3::Y,
                variable_name: "r_c2_weight".to_string(),
                radius_lower_bound: Some(1.0),
                radius_upper_bound: Some(3.0),
            },
        ],
        registration_plan: ContinuousRadiusRegistrationPlan::default(),
        weight_functions: HashMap::new(),
        objective_policy: Default::default(),
    };
    let mut model = MetaModel::<f64>::new("continuous_radius_empty_primal");
    let registration = component.register_solver_model(&mut model).unwrap();

    let (solutions, diagnostics) = component.selected_solutions_from_primal(&registration, &[]);

    assert!(solutions.is_empty());
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|message| {
        message.contains("r_c1_weight") && message.contains("primal solution is empty")
    }));
    assert!(diagnostics.iter().any(|message| {
        message.contains("r_c2_weight") && message.contains("primal solution is empty")
    }));
}

#[test]
fn continuous_radius_primal_rejects_non_finite_radius_and_negative_square() {
    let component = ContinuousRadiusModelComponent {
        prototypes: vec![ContinuousCylinderRadiusSolverPrototype {
            item_id: "c1".into(),
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
    let mut model = MetaModel::<f64>::new("continuous_radius_primal_values");
    let registration = component.register_solver_model(&mut model).unwrap();
    let variable = &registration.variables[0];

    let mut non_finite_solution = vec![0.0; model.num_tokens()];
    non_finite_solution[variable.radius_index] = f64::NAN;
    let (solutions, diagnostics) =
        component.selected_solutions_from_primal(&registration, &non_finite_solution);
    assert!(solutions.is_empty());
    assert!(
        diagnostics
            .iter()
            .any(|message| message.contains("non-finite selected radius"))
    );

    let mut negative_square_solution = vec![0.0; model.num_tokens()];
    negative_square_solution[variable.radius_index] = 2.0;
    negative_square_solution[variable.radius_squared_index] = -1.0;
    negative_square_solution[variable.segment_indices[4]] = 1.0;
    negative_square_solution[variable.left_lambda_indices[4]] = 1.0;
    let (solutions, diagnostics) =
        component.selected_solutions_from_primal(&registration, &negative_square_solution);
    assert!(solutions.is_empty());
    assert!(
        diagnostics
            .iter()
            .any(|message| message.contains("invalid radius squared"))
    );
}

#[test]
fn continuous_radius_primal_rejects_radius_in_mismatched_segment() {
    let component = ContinuousRadiusModelComponent {
        prototypes: vec![ContinuousCylinderRadiusSolverPrototype {
            item_id: "c1".into(),
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
    let mut model = MetaModel::<f64>::new("continuous_radius_primal_segment_mismatch");
    let registration = component.register_solver_model(&mut model).unwrap();
    let variable = &registration.variables[0];
    let mut solution = vec![0.0; model.num_tokens()];
    solution[variable.radius_index] = 2.0;
    solution[variable.radius_squared_index] = 4.0;
    solution[variable.segment_indices[0]] = 1.0;

    let (solutions, diagnostics) =
        component.selected_solutions_from_primal(&registration, &solution);

    assert!(solutions.is_empty());
    assert!(
        diagnostics
            .iter()
            .any(|message| message.contains("does not contain radius"))
    );
}

#[test]
fn continuous_radius_primal_rejects_radius_squared_mismatched_with_segment_interpolation() {
    let component = ContinuousRadiusModelComponent {
        prototypes: vec![ContinuousCylinderRadiusSolverPrototype {
            item_id: "c1".into(),
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
    let mut model = MetaModel::<f64>::new("continuous_radius_primal_pwl_mismatch");
    let registration = component.register_solver_model(&mut model).unwrap();
    let variable = &registration.variables[0];
    let mut solution = vec![0.0; model.num_tokens()];
    solution[variable.radius_index] = 2.0;
    solution[variable.radius_squared_index] = 5.0;
    solution[variable.segment_indices[4]] = 1.0;
    solution[variable.left_lambda_indices[4]] = 1.0;

    let (solutions, diagnostics) =
        component.selected_solutions_from_primal(&registration, &solution);

    assert!(solutions.is_empty());
    assert!(
        diagnostics
            .iter()
            .any(|message| message.contains("does not match PWL value"))
    );
}

#[test]
fn continuous_radius_info_rejects_radius_in_mismatched_segment() {
    let component = ContinuousRadiusModelComponent {
        prototypes: vec![ContinuousCylinderRadiusSolverPrototype {
            item_id: "c1".into(),
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
    let info = HashMap::from([
        (
            ContinuousRadiusModelComponent::solution_info_key("r_c1_weight"),
            "2.0".to_string(),
        ),
        (
            ContinuousRadiusModelComponent::segment_info_key("r_c1_weight"),
            "0".to_string(),
        ),
    ]);

    let (solutions, diagnostics) = component.selected_solutions_from_info(&info);

    assert!(solutions.is_empty());
    assert!(
        diagnostics
            .iter()
            .any(|message| message.contains("does not contain radius"))
    );
}

#[test]
fn continuous_radius_compile_time_quantity_overflow_returns_error() {
    use ospf_rust_quantities::unit::derived::Kilometer;

    let prototype = ContinuousCylinderRadiusSolverPrototype {
        item_id: "c1".into(),
        source: "weight".to_string(),
        axis: Axis3::Y,
        variable_name: "r_c1_weight".to_string(),
        radius_lower_bound: None,
        radius_upper_bound: None,
    };
    let result = prototype.with_quantity_bounds(
        Quantity::<f64, Kilometer>::new_ct(f64::MAX),
        Quantity::<f64, Kilometer>::new_ct(f64::MAX),
    );

    let error = result.unwrap_err();
    assert!(
        error.contains("overflows when converted to metres"),
        "{error}"
    );
}
