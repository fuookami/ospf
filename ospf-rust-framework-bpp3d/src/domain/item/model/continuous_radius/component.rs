impl ContinuousRadiusModelComponent {
    /// 求解半径 info 键 / Solver radius info key
    pub fn solution_info_key(variable_name: &str) -> String {
        format!("continuous_radius_selected_{}", variable_name)
    }

    /// 求解 PWL 分段 info 键 / Solver PWL segment info key
    pub fn segment_info_key(variable_name: &str) -> String {
        format!("continuous_radius_selected_segment_{}", variable_name)
    }

    /// 求解半径平方 info 键 / Solver radius-squared info key
    pub fn radius_squared_info_key(variable_name: &str) -> String {
        format!(
            "continuous_radius_selected_radius_squared_{}",
            variable_name
        )
    }

    /// 诊断信息 / Diagnostic information
    pub fn info(&self) -> Vec<(&str, String)> {
        let mut info = Vec::new();
        info.push(("prototype_count", self.prototypes.len().to_string()));
        info.push((
            "registered_variables",
            self.registration_plan.registered_variables.join(", "),
        ));
        info.push((
            "blocked_variables",
            self.registration_plan.blocked_variables.join(", "),
        ));
        info.push((
            "weight_function_count",
            self.weight_functions.len().to_string(),
        ));
        let mut weight_function_keys = self.weight_functions.keys().cloned().collect::<Vec<_>>();
        weight_function_keys.sort();
        info.push(("weight_function_keys", weight_function_keys.join(", ")));
        info.push(("objective_policy", self.objective_policy.name().to_string()));
        info
    }

    /// 获取原型对应的权重函数 / Resolve weight function for a prototype
    pub fn weight_function_for_prototype(
        &self,
        prototype: &ContinuousCylinderRadiusSolverPrototype,
    ) -> Option<&ContinuousRadiusWeightFunction> {
        self.weight_functions.get(&prototype.source)
    }

    /// 对求解半径结果求业务权重 / Evaluate business weight for selected solution
    pub fn evaluate_selected_solution_weight(
        &self,
        solution: &ContinuousCylinderRadiusSolution,
    ) -> Option<f64> {
        let function = self.weight_functions.get(&solution.source)?;
        let radius_squared = solution
            .radius_squared
            .unwrap_or(solution.radius * solution.radius);
        if !radius_squared.is_finite() || radius_squared < 0.0 {
            return None;
        }
        let weight = function.evaluate_radius_squared(radius_squared);
        weight.is_finite().then_some(weight)
    }

    /// 注册真实求解器 PWL 模型 / Register real solver PWL model
    pub fn register_solver_model(
        &self,
        model: &mut MetaModel<f64>,
    ) -> Result<ContinuousRadiusModelRegistration, String> {
        model.transaction(|model| self.register_solver_model_inner(model))
    }

    /// 在事务中注册连续半径模型 / Register the continuous-radius model inside a transaction
    fn register_solver_model_inner(
        &self,
        model: &mut MetaModel<f64>,
    ) -> Result<ContinuousRadiusModelRegistration, String> {
        let mut registration = ContinuousRadiusModelRegistration::default();
        for prototype in &self.prototypes {
            let (lower_bound, upper_bound) = prototype.validate_bounds()?;
            let variable_registration = self.register_one_solver_variable(
                model,
                prototype,
                lower_bound,
                upper_bound,
                &mut registration.constraint_count,
            )?;
            let (radius_vars, piecewise_vars, symbols) = variable_registration.to_explicit_fields();
            registration.radius_variables.push(radius_vars);
            registration.piecewise_variables.push(piecewise_vars);
            registration.symbols.push(symbols);
            registration.variables.push(variable_registration);
        }
        self.register_solver_objective(model, &mut registration)?;
        Ok(registration)
    }

    fn register_solver_objective(
        &self,
        model: &mut MetaModel<f64>,
        registration: &mut ContinuousRadiusModelRegistration,
    ) -> Result<(), String> {
        if registration.variables.is_empty() {
            return Ok(());
        }
        let configured_tie_breaker_weight = match &self.objective_policy {
            ContinuousRadiusObjectivePolicy::MinimizeRadiusSquared { weight } => Some(*weight),
            ContinuousRadiusObjectivePolicy::None => None,
        };
        if configured_tie_breaker_weight.is_some_and(|weight| !weight.is_finite()) {
            return Err(format!(
                "continuous radius objective policy has non-finite weight {}",
                configured_tie_breaker_weight.unwrap_or_default(),
            ));
        }
        let mut business_monomials_by_weight = Vec::<(f64, Vec<LinearMonomial<f64>>)>::new();
        let mut unmatched_tie_breaker_monomials = Vec::new();
        for variable in &registration.variables {
            let Some(prototype) = self
                .prototypes
                .iter()
                .find(|prototype| prototype.variable_name == variable.variable_name)
            else {
                registration.diagnostics.push(format!(
                    "continuous radius variable '{}' has no matching prototype for objective",
                    variable.variable_name,
                ));
                unmatched_tie_breaker_monomials
                    .push(LinearMonomial::new(1.0, variable.radius_squared_index));
                continue;
            };
            let Some(function) = self.weight_function_for_prototype(prototype) else {
                unmatched_tie_breaker_monomials
                    .push(LinearMonomial::new(1.0, variable.radius_squared_index));
                registration.diagnostics.push(format!(
                    "continuous radius variable '{}' has no business weight function '{}'; falling back to objective policy {}",
                    prototype.variable_name,
                    prototype.source,
                    self.objective_policy.name(),
                ));
                continue;
            };
            prototype.validate_weight_function(function)?;
            if function.objective_weight == 0.0 || function.radius_squared_coefficient == 0.0 {
                registration.diagnostics.push(format!(
                    "continuous radius weight function '{}' skipped for '{}': zero coefficient/weight",
                    function.key,
                    prototype.variable_name,
                ));
                unmatched_tie_breaker_monomials
                    .push(LinearMonomial::new(1.0, variable.radius_squared_index));
                continue;
            }
            if let Some((_, monomials)) = business_monomials_by_weight
                .iter_mut()
                .find(|(weight, _)| (*weight - function.objective_weight).abs() <= 1e-12)
            {
                monomials.push(LinearMonomial::new(
                    function.radius_squared_coefficient,
                    variable.radius_squared_index,
                ));
            } else {
                business_monomials_by_weight.push((
                    function.objective_weight,
                    vec![LinearMonomial::new(
                        function.radius_squared_coefficient,
                        variable.radius_squared_index,
                    )],
                ));
            }
            registration.business_objective_term_count += 1;
            registration.matched_weight_function_count += 1;
        }
        for (index, (weight, monomials)) in business_monomials_by_weight.into_iter().enumerate() {
            registration.objective_term_count += monomials.len();
            model.add_sub_objective(
                SubObjective::minimize(
                    Linear::new(monomials, 0.0),
                    &format!("continuous_radius_business_weight_{}", index),
                )
                .with_weight(weight),
            );
        }
        let weight = match &self.objective_policy {
            ContinuousRadiusObjectivePolicy::MinimizeRadiusSquared { weight } => *weight,
            ContinuousRadiusObjectivePolicy::None => return Ok(()),
        };
        if unmatched_tie_breaker_monomials.is_empty() || weight == 0.0 {
            return Ok(());
        }
        registration.tie_breaker_objective_term_count = unmatched_tie_breaker_monomials.len();
        registration.objective_term_count += unmatched_tie_breaker_monomials.len();
        model.add_sub_objective(
            SubObjective::minimize(
                Linear::new(unmatched_tie_breaker_monomials, 0.0),
                "continuous_radius_squared_tie_breaker",
            )
            .with_weight(weight),
        );
        Ok(())
    }

    fn register_one_solver_variable(
        &self,
        model: &mut MetaModel<f64>,
        prototype: &ContinuousCylinderRadiusSolverPrototype,
        lower_bound: f64,
        upper_bound: f64,
        constraint_count: &mut usize,
    ) -> Result<ContinuousRadiusVariableRegistration, String> {
        let stem = continuous_radius_model_stem(&prototype.variable_name);
        let radius_index = model
            .register_variable(UContinuousVariableItem::auto_with_range(
                &format!("{}_radius", stem),
                VariableRange::bounded(lower_bound, upper_bound),
            ))
            .map_err(|error| {
                format!(
                    "failed to register continuous radius variable '{}': {:?}",
                    prototype.variable_name, error,
                )
            })?;
        let radius_squared_index = model
            .register_variable(UContinuousVariableItem::auto_with_range(
                &format!("{}_radius_squared", stem),
                VariableRange::bounded(lower_bound * lower_bound, upper_bound * upper_bound),
            ))
            .map_err(|error| {
                format!(
                    "failed to register continuous radius squared variable '{}': {:?}",
                    prototype.variable_name, error,
                )
            })?;

        if (upper_bound - lower_bound).abs() <= 1e-9 {
            return Ok(ContinuousRadiusVariableRegistration {
                variable_name: prototype.variable_name.clone(),
                radius_index,
                radius_squared_index,
                segment_indices: Vec::new(),
                left_lambda_indices: Vec::new(),
                right_lambda_indices: Vec::new(),
                breakpoints: vec![lower_bound],
            });
        }

        let approximation = PwlRadiusSquaredApproximation::try_from_radius_interval(
            lower_bound,
            upper_bound,
            &PwlRadiusApproximationConfig::default(),
        )
        .map_err(|error| {
            format!(
                "failed to build continuous radius approximation '{}': {error}",
                prototype.variable_name,
            )
        })?;
        let mut segment_indices = Vec::with_capacity(approximation.num_segments());
        let mut left_lambda_indices = Vec::with_capacity(approximation.num_segments());
        let mut right_lambda_indices = Vec::with_capacity(approximation.num_segments());
        for segment_index in 0..approximation.num_segments() {
            let segment_model_index = model
                .register_variable(BinaryVariableItem::auto(&format!(
                    "{}_seg_{}",
                    stem, segment_index
                )))
                .map_err(|error| {
                    format!(
                        "failed to register continuous radius segment '{}' #{}: {:?}",
                        prototype.variable_name, segment_index, error,
                    )
                })?;
            let left_lambda_index = model
                .register_variable(UContinuousVariableItem::auto_with_range(
                    &format!("{}_seg_{}_lambda_left", stem, segment_index),
                    VariableRange::bounded(0.0, 1.0),
                ))
                .map_err(|error| {
                    format!(
                        "failed to register continuous radius left lambda '{}' #{}: {:?}",
                        prototype.variable_name, segment_index, error,
                    )
                })?;
            let right_lambda_index = model
                .register_variable(UContinuousVariableItem::auto_with_range(
                    &format!("{}_seg_{}_lambda_right", stem, segment_index),
                    VariableRange::bounded(0.0, 1.0),
                ))
                .map_err(|error| {
                    format!(
                        "failed to register continuous radius right lambda '{}' #{}: {:?}",
                        prototype.variable_name, segment_index, error,
                    )
                })?;
            segment_indices.push(segment_model_index);
            left_lambda_indices.push(left_lambda_index);
            right_lambda_indices.push(right_lambda_index);
        }

        model
            .add_eq_constraint(
                &segment_indices
                    .iter()
                    .map(|&model_index| (model_index, 1.0))
                    .collect::<Vec<_>>(),
                1.0,
                &format!("{}_segment_partition", stem),
            )
            .map_err(|error| {
                format!(
                    "failed to register continuous radius segment partition '{}': {:?}",
                    prototype.variable_name, error,
                )
            })?;
        *constraint_count += 1;

        for segment_index in 0..approximation.num_segments() {
            model
                .add_eq_constraint(
                    &[
                        (left_lambda_indices[segment_index], 1.0),
                        (right_lambda_indices[segment_index], 1.0),
                        (segment_indices[segment_index], -1.0),
                    ],
                    0.0,
                    &format!("{}_segment_{}_lambda_link", stem, segment_index),
                )
                .map_err(|error| {
                    format!(
                        "failed to register continuous radius lambda link '{}' #{}: {:?}",
                        prototype.variable_name, segment_index, error,
                    )
                })?;
            *constraint_count += 1;
        }

        let mut radius_terms = Vec::with_capacity(1 + approximation.num_segments() * 2);
        let mut radius_squared_terms = Vec::with_capacity(1 + approximation.num_segments() * 2);
        radius_terms.push((radius_index, 1.0));
        radius_squared_terms.push((radius_squared_index, 1.0));
        for segment_index in 0..approximation.num_segments() {
            let left = approximation.breakpoints[segment_index];
            let right = approximation.breakpoints[segment_index + 1];
            radius_terms.push((left_lambda_indices[segment_index], -left));
            radius_terms.push((right_lambda_indices[segment_index], -right));
            radius_squared_terms.push((left_lambda_indices[segment_index], -(left * left)));
            radius_squared_terms.push((right_lambda_indices[segment_index], -(right * right)));
        }
        model
            .add_eq_constraint(&radius_terms, 0.0, &format!("{}_radius_pwl_relation", stem))
            .map_err(|error| {
                format!(
                    "failed to register continuous radius PWL relation '{}': {:?}",
                    prototype.variable_name, error,
                )
            })?;
        *constraint_count += 1;
        model
            .add_eq_constraint(
                &radius_squared_terms,
                0.0,
                &format!("{}_radius_squared_pwl_relation", stem),
            )
            .map_err(|error| {
                format!(
                    "failed to register continuous radius squared PWL relation '{}': {:?}",
                    prototype.variable_name, error,
                )
            })?;
        *constraint_count += 1;

        Ok(ContinuousRadiusVariableRegistration {
            variable_name: prototype.variable_name.clone(),
            radius_index,
            radius_squared_index,
            segment_indices,
            left_lambda_indices,
            right_lambda_indices,
            breakpoints: approximation.breakpoints,
        })
    }

    /// 从求解器原始解提取连续半径 / Extract continuous radius from solver primal solution
    pub fn selected_solutions_from_primal(
        &self,
        registration: &ContinuousRadiusModelRegistration,
        solution: &[f64],
    ) -> (Vec<ContinuousCylinderRadiusSolution>, Vec<String>) {
        if registration.is_empty() {
            return (Vec::new(), Vec::new());
        }
        if solution.is_empty() {
            let diagnostics = registration
                .variables
                .iter()
                .map(|variable_registration| {
                    format!(
                        "continuous radius variable '{}' missing solver value: primal solution is empty",
                        variable_registration.variable_name,
                    )
                })
                .collect();
            return (Vec::new(), diagnostics);
        }
        let mut solutions = Vec::new();
        let mut diagnostics = Vec::new();
        for variable_registration in &registration.variables {
            let Some(prototype) = self
                .prototypes
                .iter()
                .find(|prototype| prototype.variable_name == variable_registration.variable_name)
            else {
                diagnostics.push(format!(
                    "continuous radius registration '{}' has no matching prototype",
                    variable_registration.variable_name,
                ));
                continue;
            };
            if let Err(error) = prototype.validate_bounds() {
                diagnostics.push(format!(
                    "continuous radius variable '{}' has invalid prototype bounds: {}",
                    prototype.variable_name, error,
                ));
                continue;
            }
            let Some(radius) = solution.get(variable_registration.radius_index).copied() else {
                diagnostics.push(format!(
                    "continuous radius variable '{}' missing solver value at index {}",
                    prototype.variable_name, variable_registration.radius_index,
                ));
                continue;
            };
            if !radius.is_finite() {
                diagnostics.push(format!(
                    "continuous radius variable '{}' has non-finite selected radius {}",
                    prototype.variable_name, radius,
                ));
                continue;
            }
            if !prototype.accepts_radius(radius) {
                diagnostics.push(format!(
                    "continuous radius variable '{}' selected radius {} outside bounds {:?}..{:?}",
                    prototype.variable_name,
                    radius,
                    prototype.radius_lower_bound,
                    prototype.radius_upper_bound,
                ));
                continue;
            }
            let segment_index = match variable_registration.selected_segment_index_checked(solution)
            {
                Ok(segment_index) => segment_index,
                Err(error) => {
                    diagnostics.push(format!(
                        "continuous radius variable '{}' has invalid PWL segment data: {}",
                        prototype.variable_name, error,
                    ));
                    continue;
                }
            };
            if !variable_registration.segment_indices.is_empty() && segment_index.is_none() {
                diagnostics.push(format!(
                    "continuous radius variable '{}' has no selected PWL segment",
                    prototype.variable_name,
                ));
                continue;
            }
            if let Some(segment_index) = segment_index {
                if let Err(error) = validate_segment_contains_radius(
                    &prototype.variable_name,
                    radius,
                    segment_index,
                    &variable_registration.breakpoints,
                ) {
                    diagnostics.push(error);
                    continue;
                }
            }
            let radius_squared = match solution
                .get(variable_registration.radius_squared_index)
                .copied()
            {
                Some(value) => match prototype.validate_radius_squared(value) {
                    Ok(value) => Some(value),
                    Err(error) => {
                        diagnostics.push(error);
                        continue;
                    }
                },
                None => None,
            };
            if let Some(radius_squared) = radius_squared {
                let expected = segment_index
                    .and_then(|segment_index| {
                        expected_radius_squared_for_segment(
                            &variable_registration.breakpoints,
                            segment_index,
                            radius,
                        )
                    })
                    .or_else(|| variable_registration.evaluate_radius_squared(radius));
                let Some(expected) = expected else {
                    diagnostics.push(format!(
                        "continuous radius variable '{}' cannot evaluate PWL radius squared for radius {}",
                        prototype.variable_name, radius,
                    ));
                    continue;
                };
                if (radius_squared - expected).abs() > CONTINUOUS_RADIUS_RESULT_TOLERANCE {
                    diagnostics.push(format!(
                        "continuous radius variable '{}' radius squared {} does not match PWL value {}",
                        prototype.variable_name,
                        radius_squared,
                        expected,
                    ));
                    continue;
                }
            }
            solutions.push(prototype.selected_solution(radius, radius_squared, segment_index));
        }
        (solutions, diagnostics)
    }

    /// 从 info map 提取求解半径结果 / Extract selected radius solutions from info map
    pub fn selected_solutions_from_info(
        &self,
        info: &HashMap<String, String>,
    ) -> (Vec<ContinuousCylinderRadiusSolution>, Vec<String>) {
        let mut solutions = Vec::new();
        let mut diagnostics = Vec::new();
        'prototype: for prototype in &self.prototypes {
            let key = Self::solution_info_key(&prototype.variable_name);
            let Some(raw_radius) = info.get(&key) else {
                continue;
            };
            let (lower_bound, upper_bound) = match prototype.validate_bounds() {
                Ok(bounds) => bounds,
                Err(error) => {
                    diagnostics.push(format!(
                        "continuous radius variable '{}' has invalid prototype bounds: {}",
                        prototype.variable_name, error,
                    ));
                    continue;
                }
            };
            let radius = match raw_radius.trim().parse::<f64>() {
                Ok(value) if value.is_finite() => value,
                Ok(value) => {
                    diagnostics.push(format!(
                        "continuous radius variable '{}' has non-finite selected radius {}",
                        prototype.variable_name, value,
                    ));
                    continue;
                }
                Err(_) => {
                    diagnostics.push(format!(
                        "continuous radius variable '{}' has invalid selected radius '{}'",
                        prototype.variable_name, raw_radius,
                    ));
                    continue;
                }
            };
            if !prototype.accepts_radius(radius) {
                diagnostics.push(format!(
                    "continuous radius variable '{}' selected radius {} outside bounds {:?}..{:?}",
                    prototype.variable_name,
                    radius,
                    prototype.radius_lower_bound,
                    prototype.radius_upper_bound,
                ));
                continue;
            }
            let segment_key = Self::segment_info_key(&prototype.variable_name);
            let segment_index = match info.get(&segment_key) {
                Some(raw_segment) => match raw_segment.trim().parse::<usize>() {
                    Ok(value) => {
                        let expected_segments = expected_segment_count(lower_bound, upper_bound);
                        if expected_segments.is_some_and(|count| value >= count) {
                            diagnostics.push(format!(
                                "continuous radius variable '{}' has selected segment {} outside PWL range 0..{}",
                                prototype.variable_name,
                                value,
                                expected_segments.unwrap_or(0).saturating_sub(1),
                            ));
                            continue 'prototype;
                        }
                        Some(value)
                    }
                    Err(_) => {
                        diagnostics.push(format!(
                            "continuous radius variable '{}' has invalid selected segment '{}'",
                            prototype.variable_name, raw_segment,
                        ));
                        continue 'prototype;
                    }
                },
                None => None,
            };
            if let Some(segment_index) = segment_index {
                let approximation = match PwlRadiusSquaredApproximation::try_from_radius_interval(
                    lower_bound,
                    upper_bound,
                    &PwlRadiusApproximationConfig::default(),
                ) {
                    Ok(approximation) => approximation,
                    Err(error) => {
                        diagnostics.push(format!(
                            "continuous radius variable '{}' cannot build PWL approximation for segment validation: {}",
                            prototype.variable_name, error,
                        ));
                        continue 'prototype;
                    }
                };
                if let Err(error) = validate_segment_contains_radius(
                    &prototype.variable_name,
                    radius,
                    segment_index,
                    &approximation.breakpoints,
                ) {
                    diagnostics.push(error);
                    continue 'prototype;
                }
            }
            let radius_squared_key = Self::radius_squared_info_key(&prototype.variable_name);
            let radius_squared = match info.get(&radius_squared_key) {
                Some(raw_radius_squared) => match raw_radius_squared.trim().parse::<f64>() {
                    Ok(value) => match prototype.validate_radius_squared(value) {
                        Ok(value) => Some(value),
                        Err(error) => {
                            diagnostics.push(format!(
                                "continuous radius variable '{}' has invalid selected radius squared '{}': {}",
                                prototype.variable_name, raw_radius_squared, error,
                            ));
                            continue 'prototype;
                        }
                    },
                    Err(_) => {
                        diagnostics.push(format!(
                            "continuous radius variable '{}' has invalid selected radius squared '{}'",
                            prototype.variable_name, raw_radius_squared,
                        ));
                        continue 'prototype;
                    }
                },
                None => None,
            };
            if let Some(radius_squared) = radius_squared {
                let expected = segment_index
                    .and_then(|segment_index| {
                        let approximation =
                            PwlRadiusSquaredApproximation::try_from_radius_interval(
                                lower_bound,
                                upper_bound,
                                &PwlRadiusApproximationConfig::default(),
                            )
                            .ok()?;
                        expected_radius_squared_for_segment(
                            &approximation.breakpoints,
                            segment_index,
                            radius,
                        )
                    })
                    .or_else(|| expected_radius_squared_for_prototype(prototype, radius));
                let Some(expected) = expected else {
                    diagnostics.push(format!(
                        "continuous radius variable '{}' cannot evaluate PWL radius squared for radius {}",
                        prototype.variable_name, radius,
                    ));
                    continue 'prototype;
                };
                if (radius_squared - expected).abs() > CONTINUOUS_RADIUS_RESULT_TOLERANCE {
                    diagnostics.push(format!(
                        "continuous radius variable '{}' radius squared {} does not match PWL value {}",
                        prototype.variable_name, radius_squared, expected,
                    ));
                    continue 'prototype;
                }
            }
            solutions.push(prototype.selected_solution(radius, radius_squared, segment_index));
        }
        (solutions, diagnostics)
    }

    /// 应用求解半径到货物 / Apply selected radius solutions to items
    pub fn apply_solutions_to_items<U>(
        &self,
        items: &mut [ActualItem<f64, U>],
        solutions: &[ContinuousCylinderRadiusSolution],
    ) -> Vec<String>
    where
        U: CTUnit + Default + Clone,
    {
        let mut diagnostics = Vec::new();
        for solution in solutions {
            let Some(item) = items.iter_mut().find(|item| item.id == solution.item_id) else {
                diagnostics.push(format!(
                    "continuous radius solution '{}' references missing item '{}'",
                    solution.variable_name, solution.item_id,
                ));
                continue;
            };
            if let Err(error) = self.apply_solution_to_item(item, solution) {
                diagnostics.push(error);
            }
        }
        diagnostics
    }

    /// 应用单个求解半径到货物 / Apply one selected radius solution to an item
    pub fn apply_solution_to_item<U>(
        &self,
        item: &mut ActualItem<f64, U>,
        solution: &ContinuousCylinderRadiusSolution,
    ) -> Result<(), String>
    where
        U: CTUnit + Default + Clone,
    {
        let Some(prototype) = self
            .prototypes
            .iter()
            .find(|prototype| prototype.variable_name == solution.variable_name)
        else {
            return Err(format!(
                "continuous radius solution '{}' has no matching prototype",
                solution.variable_name,
            ));
        };
        if prototype.item_id != item.id {
            return Err(format!(
                "continuous radius solution '{}' targets item '{}' but was applied to '{}'",
                solution.variable_name, prototype.item_id, item.id,
            ));
        }
        if !prototype.accepts_radius(solution.radius) {
            if !solution.radius.is_finite() {
                return Err(format!(
                    "continuous radius solution '{}' has non-finite radius {}",
                    solution.variable_name, solution.radius,
                ));
            }
            return Err(format!(
                "continuous radius solution '{}' radius {} outside bounds {:?}..{:?}",
                solution.variable_name,
                solution.radius,
                prototype.radius_lower_bound,
                prototype.radius_upper_bound,
            ));
        }
        prototype.validate_bounds()?;
        if let Some(radius_squared) = solution.radius_squared {
            prototype.validate_radius_squared(radius_squared)?;
        }
        let Some(PackageShapeSpec::Cylinder {
            axis,
            radius_candidates,
            radius_lower_bound,
            radius_upper_bound,
            ..
        }) = item.shape_spec_override.clone()
        else {
            return Err(format!(
                "continuous radius solution '{}' targets non-cylinder item '{}'",
                solution.variable_name, item.id,
            ));
        };
        if axis != solution.axis {
            return Err(format!(
                "continuous radius solution '{}' axis {:?} does not match item '{}' axis {:?}",
                solution.variable_name, solution.axis, item.id, axis,
            ));
        }
        let radius_in_item_unit = Quantity::<f64, Meter>::new_ct(solution.radius)
            .to_runtime()
            .to_unit(&U::INSTANT)
            .map_err(|error| {
                format!(
                    "continuous radius solution '{}' cannot convert metres to item unit '{}': {:?}",
                    solution.variable_name,
                    U::SYMBOL,
                    error,
                )
            })?;
        if !radius_in_item_unit.value.is_finite() {
            return Err(format!(
                "continuous radius solution '{}' converts to non-finite item radius {} {}",
                solution.variable_name,
                radius_in_item_unit.value,
                U::SYMBOL,
            ));
        }
        let radius = Quantity::<f64, U>::new_ct(radius_in_item_unit.value);
        item.shape_spec_override = Some(PackageShapeSpec::Cylinder {
            axis,
            radius,
            radius_candidates,
            radius_lower_bound,
            radius_upper_bound,
        });
        Ok(())
    }
}

fn continuous_radius_model_stem(variable_name: &str) -> String {
    let sanitized = variable_name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    if sanitized.is_empty() {
        "continuous_radius".to_string()
    } else {
        sanitized
    }
}

const CONTINUOUS_RADIUS_RESULT_TOLERANCE: f64 = 1e-5;
const CONTINUOUS_RADIUS_SEGMENT_TOLERANCE: f64 = 1e-9;

fn validate_segment_contains_radius(
    variable_name: &str,
    radius: f64,
    segment_index: usize,
    breakpoints: &[f64],
) -> Result<(), String> {
    if !radius.is_finite() {
        return Err(format!(
            "continuous radius variable '{}' has non-finite selected radius {}",
            variable_name, radius,
        ));
    }
    let Some(segment_end) = segment_index.checked_add(1) else {
        return Err(format!(
            "continuous radius variable '{}' has invalid PWL segment {}",
            variable_name, segment_index,
        ));
    };
    let (Some(&left), Some(&right)) =
        (breakpoints.get(segment_index), breakpoints.get(segment_end))
    else {
        return Err(format!(
            "continuous radius variable '{}' selected segment {} is outside PWL range",
            variable_name, segment_index,
        ));
    };
    if !left.is_finite() || !right.is_finite() || left >= right {
        return Err(format!(
            "continuous radius variable '{}' has invalid PWL segment {} bounds {}..{}",
            variable_name, segment_index, left, right,
        ));
    }
    if radius < left - CONTINUOUS_RADIUS_SEGMENT_TOLERANCE
        || radius > right + CONTINUOUS_RADIUS_SEGMENT_TOLERANCE
    {
        return Err(format!(
            "continuous radius variable '{}' selected segment {} does not contain radius {} (expected {}..{})",
            variable_name, segment_index, radius, left, right,
        ));
    }
    Ok(())
}

fn expected_radius_squared_for_segment(
    breakpoints: &[f64],
    segment_index: usize,
    radius: f64,
) -> Option<f64> {
    let right_index = segment_index.checked_add(1)?;
    let left = *breakpoints.get(segment_index)?;
    let right = *breakpoints.get(right_index)?;
    if !radius.is_finite()
        || !left.is_finite()
        || !right.is_finite()
        || left >= right
        || radius < left - CONTINUOUS_RADIUS_SEGMENT_TOLERANCE
        || radius > right + CONTINUOUS_RADIUS_SEGMENT_TOLERANCE
    {
        return None;
    }
    let ratio = ((radius - left) / (right - left)).clamp(0.0, 1.0);
    let expected = left * left + ratio * (right * right - left * left);
    expected.is_finite().then_some(expected)
}

fn expected_segment_count(lower_bound: f64, upper_bound: f64) -> Option<usize> {
    if (upper_bound - lower_bound).abs() <= 1e-9 {
        return Some(0);
    }
    PwlRadiusSquaredApproximation::try_from_radius_interval(
        lower_bound,
        upper_bound,
        &PwlRadiusApproximationConfig::default(),
    )
    .ok()
    .map(|approximation| approximation.num_segments())
}

/// 计算原型半径对应的 PWL 平方值 / Evaluate the PWL squared-radius value for a prototype
fn expected_radius_squared_for_prototype(
    prototype: &ContinuousCylinderRadiusSolverPrototype,
    radius: f64,
) -> Option<f64> {
    let (lower_bound, upper_bound) = prototype.validate_bounds().ok()?;
    if radius < lower_bound || radius > upper_bound || !radius.is_finite() {
        return None;
    }
    if (upper_bound - lower_bound).abs() <= 1e-9 {
        return Some(radius * radius);
    }
    PwlRadiusSquaredApproximation::try_from_radius_interval(
        lower_bound,
        upper_bound,
        &PwlRadiusApproximationConfig::default(),
    )
    .ok()
    .map(|approximation| approximation.evaluate(radius))
}
