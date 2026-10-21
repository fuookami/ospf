// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::item::{
        BinLayer, BinType, Bpp3dDemandKey, Bpp3dDemandMode, Bpp3dLayerDemandCoverage, bin_type_id,
    };
    use crate::domain::layer_assignment::model::{
        Bpp3dModelComponent, VariableArray1, VariableArray2,
    };
    use crate::domain::layer_assignment::service::ScaledBpp3dSolverValueAdapter;
    use ospf_rust_quantities::unit::derived::Meter;
    use ospf_rust_quantities::quantity::Quantity;

    fn meters(v: f64) -> Quantity<f64, Meter> {
        Quantity::new_ct(v)
    }

    fn make_bin_type() -> BinType<f64, Meter> {
        BinType {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
            capacity: meters(1000.0),
            type_code: "BIN-10".into(),
            is_main: true,
        }
    }

    fn make_layer(depth: f64) -> BinLayer<f64, Meter> {
        BinLayer {
            iteration: 0,
            from: "test".to_string(),
            bin: None,
            depth: meters(depth),
            demand_coverage: Vec::new(),
        }
    }

    #[test]
    fn demand_constraint_shadow_price_key() {
        let entry = Bpp3dDemandEntry {
            mode: crate::domain::item::Bpp3dDemandMode::Item,
            key: crate::domain::item::Bpp3dDemandKey::Item { id: "item1".into() },
            demand: 10.0,
        };
        let key = DemandConstraint::<f64, Meter>::shadow_price_key(&entry);
        assert_eq!(key.mode, crate::domain::item::Bpp3dDemandMode::Item);
    }

    #[test]
    fn demand_constraint_registers_linear_cover_rows() {
        let key = Bpp3dDemandKey::Item { id: "item1".into() };
        let mut assignment = ImpreciseAssignment {
            layers: vec![
                make_layer(1.0).with_demand_coverage(vec![Bpp3dLayerDemandCoverage::new(
                    Bpp3dDemandMode::Item,
                    key.clone(),
                    1.0,
                )]),
                make_layer(2.0),
            ],
            x: None,
            upper_bounds: vec![None, None],
            load_weight_symbols: Vec::new(),
            load_volume_symbols: Vec::new(),
            load_depth_symbols: Vec::new(),
            load_symbols: Vec::new(),
        };
        let mut model = MetaModel::<f64>::new("demand_cover");
        assignment.register(&mut model).unwrap();
        let constraint: DemandConstraint<f64, Meter> = DemandConstraint::imprecise(
            vec![Bpp3dDemandEntry {
                mode: Bpp3dDemandMode::Item,
                key,
                demand: 1.0,
            }],
            assignment,
        );

        constraint.register(&mut model);

        assert_eq!(model.num_constraints(), 1);
    }

    #[test]
    fn precise_activation_constraint_registers_x_to_v_rows() {
        let bin = make_bin_type();
        let mut assignment = PreciseAssignment {
            bins: vec![bin],
            layers: vec![make_layer(1.0), make_layer(2.0)],
            x: None,
            v: None,
            load_weight_symbols: Vec::new(),
            load_volume_symbols: Vec::new(),
            load_depth_symbols: Vec::new(),
        };
        let mut model = MetaModel::<f64>::new("activation");
        assignment.register(&mut model).unwrap();
        let constraint = PreciseAssignmentActivationConstraint::new(assignment);

        constraint.register(&mut model);

        assert_eq!(model.num_constraints(), 2);
    }

    #[test]
    fn demand_constraint_cgpipeline_refresh() {
        let assignment = ImpreciseAssignment {
            layers: vec![make_layer(1.0)],
            x: None,
            upper_bounds: vec![None],
            load_weight_symbols: Vec::new(),
            load_volume_symbols: Vec::new(),
            load_depth_symbols: Vec::new(),
            load_symbols: Vec::new(),
        };
        let entries = vec![
            Bpp3dDemandEntry {
                mode: crate::domain::item::Bpp3dDemandMode::Item,
                key: crate::domain::item::Bpp3dDemandKey::Item { id: "item1".into() },
                demand: 10.0,
            },
        ];
        let constraint: DemandConstraint<f64, Meter> = DemandConstraint::imprecise(entries, assignment);

        let mut map: BasicShadowPriceMap<DemandShadowPriceKey> = BasicShadowPriceMap::new();
        let model = MetaModel::<f64>::new("test");
        let shadow_prices = vec![2.5];

        constraint.refresh(&mut map, &model, &shadow_prices).unwrap();

        // 验证 shadow price 已写入 map
        let sp_key = DemandShadowPriceKey {
            mode: crate::domain::item::Bpp3dDemandMode::Item,
            key: crate::domain::item::Bpp3dDemandKey::Item { id: "item1".into() },
        };
        let lookup_key = ShadowPriceKey::named::<DemandShadowPriceKey>(format!("{:?}", sp_key));
        let sp = map.get(&lookup_key);
        assert!(sp.is_some());
        assert_eq!(sp.unwrap().price, 2.5);
    }

    #[test]
    fn demand_constraint_cgpipeline_extractor() {
        let assignment = ImpreciseAssignment {
            layers: vec![make_layer(1.0)],
            x: None,
            upper_bounds: vec![None],
            load_weight_symbols: Vec::new(),
            load_volume_symbols: Vec::new(),
            load_depth_symbols: Vec::new(),
            load_symbols: Vec::new(),
        };
        let entries = vec![
            Bpp3dDemandEntry {
                mode: crate::domain::item::Bpp3dDemandMode::Item,
                key: crate::domain::item::Bpp3dDemandKey::Item { id: "item1".into() },
                demand: 10.0,
            },
        ];
        let constraint: DemandConstraint<f64, Meter> = DemandConstraint::imprecise(entries, assignment);

        let extractor = constraint.extractor();
        assert!(extractor.is_some());

        let mut map: BasicShadowPriceMap<DemandShadowPriceKey> = BasicShadowPriceMap::new();
        let sp_key = DemandShadowPriceKey {
            mode: crate::domain::item::Bpp3dDemandMode::Item,
            key: crate::domain::item::Bpp3dDemandKey::Item { id: "item1".into() },
        };
        let lookup_key = ShadowPriceKey::named::<DemandShadowPriceKey>(format!("{:?}", sp_key));
        map.put(ShadowPrice::new(lookup_key, 3.0));

        let extract_fn = extractor.unwrap();
        let price = extract_fn(&map, &sp_key);
        assert_eq!(price, 3.0);
    }

    #[test]
    fn bin_capacity_constraint_register() {
        let mut model = MetaModel::<f64>::new("test_bin_cap");

        // 注册赋值变量
        let bin_keys = vec![0usize];
        let layer_keys = vec![0usize, 1usize];
        let mut x: VariableArray2<usize, usize, ospf_rust_core::variable::variable_item::BinaryVariableItem> = VariableArray2::new("x");
        x.register_binary(&bin_keys, &layer_keys, &mut model).unwrap();

        let x_indices: Vec<Vec<(usize, usize)>> = vec![
            vec![
                (0, x.index(&0, &0).unwrap()),
                (1, x.index(&0, &1).unwrap()),
            ],
        ];

        let constraint = BinCapacityConstraint::<f64, Meter>::from_bins(
            &[make_bin_type()],
            x_indices,
            vec![5.0, 3.0],    // layer weights
            vec![20.0, 15.0],  // layer volumes
            &Bpp3dSolverValueAdapterKind::Default,
        );

        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }

    #[test]
    fn bin_depth_constraint_register() {
        let mut model = MetaModel::<f64>::new("test_bin_depth");

        let bin_keys = vec![0usize];
        let layer_keys = vec![0usize, 1usize];
        let mut x: VariableArray2<usize, usize, ospf_rust_core::variable::variable_item::BinaryVariableItem> = VariableArray2::new("x");
        x.register_binary(&bin_keys, &layer_keys, &mut model).unwrap();

        let x_indices: Vec<Vec<(usize, usize)>> = vec![
            vec![
                (0, x.index(&0, &0).unwrap()),
                (1, x.index(&0, &1).unwrap()),
            ],
        ];

        let constraint = BinDepthConstraint::<f64, Meter>::from_bins(
            &[make_bin_type()],
            x_indices,
            vec![2.0, 3.0],    // layer depths
            &Bpp3dSolverValueAdapterKind::Default,
        );

        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }

    #[test]
    fn bin_amount_minimization_register() {
        let mut model = MetaModel::<f64>::new("test_bin_amount_obj");

        // 注册 v 变量
        let mut v: VariableArray1<usize, ospf_rust_core::variable::variable_item::BinaryVariableItem> = VariableArray1::new("v");
        v.register_binary(&[0usize, 1usize], &mut model).unwrap();

        let obj = BinAmountMinimization::new(
            vec![v.index(&0).unwrap(), v.index(&1).unwrap()],
            1.0,
        );

        obj.register(&mut model);
        obj.invoke(&model).unwrap();
    }

    #[test]
    fn volume_minimization_register() {
        let mut model = MetaModel::<f64>::new("test_vol_min");

        let mut x: VariableArray1<usize, ospf_rust_core::variable::variable_item::ContinuousVariableItem> = VariableArray1::new("x");
        x.register_continuous(&[0usize, 1usize], &mut model).unwrap();

        let obj = VolumeMinimization::new(
            vec![
                (x.index(&0).unwrap(), 10.0),
                (x.index(&1).unwrap(), 20.0),
            ],
            1.0,
        );

        obj.register(&mut model);
        obj.invoke(&model).unwrap();
    }

    #[test]
    fn better_layer_maximization_register() {
        let mut model = MetaModel::<f64>::new("test_better_layer");

        let mut x: VariableArray1<usize, ospf_rust_core::variable::variable_item::ContinuousVariableItem> = VariableArray1::new("x");
        x.register_continuous(&[0usize, 1usize], &mut model).unwrap();

        let obj = BetterLayerMaximization::new(
            vec![
                (x.index(&0).unwrap(), 5.0),
                (x.index(&1).unwrap(), 3.0),
            ],
            1.0,
        );

        obj.register(&mut model);
        obj.invoke(&model).unwrap();
    }

    #[test]
    fn tail_bin_assignment_constraint_register() {
        let mut model = MetaModel::<f64>::new("test_tail_bin");

        let mut v: VariableArray1<usize, ospf_rust_core::variable::variable_item::BinaryVariableItem> = VariableArray1::new("v");
        v.register_binary(&[0usize, 1usize, 2usize], &mut model).unwrap();

        let constraint = TailBinAssignmentConstraint::new(
            vec![v.index(&0).unwrap(), v.index(&1).unwrap(), v.index(&2).unwrap()],
        );

        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();

        // 对于 3 个箱，应有 C(3,2)=3 个约束
        // v[1] <= v[0], v[2] <= v[0], v[2] <= v[1]
    }

    #[test]
    fn deferred_limits_report_diagnostics() {
        assert!(RestAmountMinimization::new()
            .diagnostics()[0]
            .contains("conservative unmet-demand objective"));
        assert!(TailBinLoadingRateMinimization::new()
            .diagnostics()[0]
            .contains("conservative tail-load objective"));
        assert!(BinLoadingOrderConstraint::new()
            .diagnostics()[0]
            .contains("adjacent activation ordering rows"));
        let objective_plan = RestAmountMinimization::new().registration_plan(vec![1, 2]);
        let constraint_plan = BinLoadingOrderConstraint::new()
            .constraint_registration_plan(vec![3, 4]);

        assert_eq!(objective_plan.objective_family.as_deref(), Some("rest_amount_minimization"));
        assert_eq!(objective_plan.variable_indices, vec![1, 2]);
        assert_eq!(constraint_plan.constraint_family.as_deref(), Some("bin_loading_order_constraint"));
        assert_eq!(constraint_plan.variable_indices, vec![3, 4]);

        let mut model = MetaModel::<f64>::new("deferred_register");
        let mut v: VariableArray1<usize, ospf_rust_core::variable::variable_item::ContinuousVariableItem> =
            VariableArray1::new("v");
        v.register_continuous(&[0usize, 1usize], &mut model).unwrap();
        let indices = vec![v.index(&0).unwrap(), v.index(&1).unwrap()];
        let _ = RestAmountMinimization::new()
            .register_objective(&mut model, indices.clone(), 1.0);
        let _ = TailBinLoadingRateMinimization::new()
            .register_objective(&mut model, indices.clone(), 0.5);
        assert_eq!(model.objective().sub_objectives.len(), 2);
        assert_eq!(model.objective().sub_objectives[0].name, "rest_amount_minimization");
        assert_eq!(model.objective().sub_objectives[1].name, "tail_bin_loading_rate_minimization");
        let before_constraints = model.num_constraints();
        let _ = BinLoadingOrderConstraint::new()
            .register_constraint(&mut model, indices);

        assert!(model.num_constraints() > before_constraints);
    }

    #[test]
    fn deferred_limits_register_production_semantic_rows() {
        let mut model = MetaModel::<f64>::new("production_semantic_limits");
        let mut v: VariableArray1<usize, ospf_rust_core::variable::variable_item::ContinuousVariableItem> =
            VariableArray1::new("v");
        v.register_continuous(&[0usize, 1usize, 2usize], &mut model).unwrap();
        let indices = vec![
            v.index(&0).unwrap(),
            v.index(&1).unwrap(),
            v.index(&2).unwrap(),
        ];

        let rest_plan = RestAmountMinimization::new()
            .register_objective(&mut model, indices.clone(), 2.0);
        let tail_plan = TailBinLoadingRateMinimization::new()
            .register_objective(&mut model, indices.clone(), 0.25);
        let before_constraints = model.num_constraints();
        let order_plan = BinLoadingOrderConstraint::new()
            .register_constraint(&mut model, indices.clone());

        assert_eq!(rest_plan.objective_family.as_deref(), Some("rest_amount_minimization"));
        assert_eq!(tail_plan.objective_family.as_deref(), Some("tail_bin_loading_rate_minimization"));
        assert_eq!(order_plan.constraint_family.as_deref(), Some("bin_loading_order_constraint"));
        assert_eq!(rest_plan.variable_indices, indices);
        assert_eq!(model.objective().sub_objectives.len(), 2);
        assert_eq!(model.num_constraints(), before_constraints + 2);
    }

    #[test]
    fn scaled_adapter_with_bin_capacity() {
        let mut model = MetaModel::<f64>::new("test_scaled_cap");

        let bin_keys = vec![0usize];
        let layer_keys = vec![0usize];
        let mut x: VariableArray2<usize, usize, ospf_rust_core::variable::variable_item::BinaryVariableItem> = VariableArray2::new("x");
        x.register_binary(&bin_keys, &layer_keys, &mut model).unwrap();

        let x_indices: Vec<Vec<(usize, usize)>> = vec![
            vec![(0, x.index(&0, &0).unwrap())],
        ];

        let constraint = BinCapacityConstraint::<f64, Meter>::from_bins(
            &[make_bin_type()],
            x_indices,
            vec![5.0],
            vec![20.0],
            &Bpp3dSolverValueAdapterKind::Scaled(ScaledBpp3dSolverValueAdapter::ten_times()),
        );

        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }

    #[test]
    fn bin_capacity_constraint_direct() {
        let mut model = MetaModel::<f64>::new("test_bin_cap_direct");

        let bin_keys = vec![0usize];
        let layer_keys = vec![0usize];
        let mut x: VariableArray2<usize, usize, ospf_rust_core::variable::variable_item::BinaryVariableItem> = VariableArray2::new("x");
        x.register_binary(&bin_keys, &layer_keys, &mut model).unwrap();

        let x_indices: Vec<Vec<(usize, usize)>> = vec![
            vec![(0, x.index(&0, &0).unwrap())],
        ];

        let constraint = BinCapacityConstraint::<f64, Meter>::new(
            vec![100.0],    // weight capacity
            vec![500.0],    // volume capacity
            x_indices,
            vec![5.0],      // layer weights
            vec![20.0],     // layer volumes
        );

        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }

    #[test]
    fn bin_depth_constraint_direct() {
        let mut model = MetaModel::<f64>::new("test_bin_depth_direct");

        let bin_keys = vec![0usize];
        let layer_keys = vec![0usize];
        let mut x: VariableArray2<usize, usize, ospf_rust_core::variable::variable_item::BinaryVariableItem> = VariableArray2::new("x");
        x.register_binary(&bin_keys, &layer_keys, &mut model).unwrap();

        let x_indices: Vec<Vec<(usize, usize)>> = vec![
            vec![(0, x.index(&0, &0).unwrap())],
        ];

        let constraint = BinDepthConstraint::<f64, Meter>::new(
            vec![10.0],      // depth capacity
            x_indices,
            vec![2.0],       // layer depths
        );

        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }

    // ============================================================================
    // 混合箱型目标 / Mixed bin type objectives
    // ============================================================================

    /// 构造指定体积与编码的箱型 / Build a bin type with the given volume and code
    fn sized_bin_type(code: &str, side: f64) -> BinType<f64, Meter> {
        BinType {
            width: meters(side),
            height: meters(side),
            depth: meters(side),
            capacity: meters(1000.0),
            type_code: code.into(),
            is_main: false,
        }
    }

    /// 读取目标中指定名称的系数表 / Read the coefficient map of a named sub-objective
    fn objective_coefficients(model: &MetaModel<f64>, name: &str) -> Vec<(usize, f64)> {
        model
            .objective()
            .sub_objectives
            .iter()
            .find(|sub| sub.name == name)
            .map(|sub| {
                sub.polynomial
                    .monomials()
                    .iter()
                    .map(|monomial| (monomial.var_index(), *monomial.coefficient()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 体积消耗目标按箱型体积作系数 / Volume minimisation weighs each marker by its bin volume
    ///
    /// 断言系数落在体积量级（8 m³）而非箱数量级（1），这是本目标具备真实区分度的前提。
    /// Asserts the coefficient is on the volume scale (8 m³) rather than the bin-count scale (1), which is the
    /// precondition for this objective to actually discriminate.
    #[test]
    fn bin_volume_minimization_uses_bin_volumes() {
        let mut model = MetaModel::<f64>::new("test_bin_volume_obj");
        let mut v: VariableArray1<usize, ospf_rust_core::variable::variable_item::BinaryVariableItem> =
            VariableArray1::new("v");
        v.register_binary(&[0usize, 1usize], &mut model).unwrap();
        let v_indices = vec![v.index(&0).unwrap(), v.index(&1).unwrap()];

        let bins = vec![
            sized_bin_type("20GP", 2.0),
            sized_bin_type("40HQ", 4.0),
        ];
        let objective = BinVolumeMinimization::from_bins(&bins, v_indices.clone(), 1.0);
        objective.register(&mut model);
        objective.invoke(&model).unwrap();

        let coefficients = objective_coefficients(&model, "bin_volume_minimization");
        assert_eq!(coefficients.len(), 2);
        assert!((coefficients[0].1 - 8.0).abs() < 1e-9, "{:?}", coefficients);
        assert!((coefficients[1].1 - 64.0).abs() < 1e-9, "{:?}", coefficients);
        assert!(
            coefficients[1].1 > coefficients[0].1,
            "the larger bin type must cost more volume"
        );
    }

    /// 权重为零时目标整体跳过 / Zero weight skips the whole objective
    #[test]
    fn bin_volume_minimization_skips_when_weight_is_zero() {
        let mut model = MetaModel::<f64>::new("test_bin_volume_zero");
        let mut v: VariableArray1<usize, ospf_rust_core::variable::variable_item::BinaryVariableItem> =
            VariableArray1::new("v");
        v.register_binary(&[0usize], &mut model).unwrap();

        let objective = BinVolumeMinimization::from_bins(
            &[sized_bin_type("20GP", 2.0)],
            vec![v.index(&0).unwrap()],
            0.0,
        );
        objective.register(&mut model);
        objective.invoke(&model).unwrap();

        assert!(
            objective_coefficients(&model, "bin_volume_minimization").is_empty(),
            "zero weight must not register a sub-objective"
        );
    }

    /// 箱型成本可以覆盖体积偏好 / Bin type cost can override the volume preference
    ///
    /// 给体积更大的箱型配置负成本（即偏好），使其加权系数低于体积更小的箱型。
    /// Give the larger bin type a negative cost (a preference) so that its weighted coefficient drops below the
    /// smaller bin type's.
    #[test]
    fn bin_type_cost_can_override_volume_preference() {
        let mut model = MetaModel::<f64>::new("test_bin_type_cost");
        let mut v: VariableArray1<usize, ospf_rust_core::variable::variable_item::BinaryVariableItem> =
            VariableArray1::new("v");
        v.register_binary(&[0usize, 1usize], &mut model).unwrap();
        let v_indices = vec![v.index(&0).unwrap(), v.index(&1).unwrap()];

        let bins = vec![
            sized_bin_type("20GP", 2.0),
            sized_bin_type("40HQ", 4.0),
        ];
        let mut type_cost = HashMap::new();
        type_cost.insert(bin_type_id("20GP"), 1.0);
        type_cost.insert(bin_type_id("40HQ"), -100.0);

        let objective =
            BinTypeCostMinimization::from_bins(&bins, v_indices, &type_cost, 1.0);
        objective.register(&mut model);
        objective.invoke(&model).unwrap();

        let coefficients = objective_coefficients(&model, "bin_type_cost_minimization");
        assert_eq!(coefficients.len(), 2);
        assert!((coefficients[0].1 - 1.0).abs() < 1e-9, "{:?}", coefficients);
        assert!((coefficients[1].1 + 100.0).abs() < 1e-9, "{:?}", coefficients);
    }

    /// 未配置成本的箱型按零处理 / Types without a configured cost count as zero
    #[test]
    fn bin_type_cost_missing_types_count_as_zero() {
        let mut model = MetaModel::<f64>::new("test_bin_type_cost_missing");
        let mut v: VariableArray1<usize, ospf_rust_core::variable::variable_item::BinaryVariableItem> =
            VariableArray1::new("v");
        v.register_binary(&[0usize, 1usize], &mut model).unwrap();
        let v_indices = vec![v.index(&0).unwrap(), v.index(&1).unwrap()];

        let bins = vec![
            sized_bin_type("20GP", 2.0),
            sized_bin_type("40HQ", 4.0),
        ];
        let mut type_cost = HashMap::new();
        type_cost.insert(bin_type_id("20GP"), 5.0);

        let objective =
            BinTypeCostMinimization::from_bins(&bins, v_indices, &type_cost, 1.0);
        objective.register(&mut model);
        objective.invoke(&model).unwrap();

        // 未配置成本且系数为零的项必须被跳过，不得注册成零系数单项式。
        // A missing cost produces a zero coefficient and must be skipped rather than registered as a zero
        // monomial.
        let coefficients = objective_coefficients(&model, "bin_type_cost_minimization");
        assert_eq!(coefficients.len(), 1);
        assert_eq!(coefficients[0].0, 0);
        assert!((coefficients[0].1 - 5.0).abs() < 1e-9, "{:?}", coefficients);
    }

    /// 余料体积与余料率按箱型体积归一 / Waste volume and ratio are normalised by the bin volume
    #[test]
    fn bin_waste_statistics_are_normalised() {
        let bin = sized_bin_type("20GP", 2.0);
        assert!((bin_volume(&bin) - 8.0).abs() < 1e-9);
        assert!((bin_waste_volume(&bin, &[3.0, 2.0]) - 3.0).abs() < 1e-9);
        assert!((bin_waste_ratio(&bin, &[3.0, 2.0]) - 3.0 / 8.0).abs() < 1e-9);
        // 装载超出箱型体积时余料不得为负。
        // Waste must never go negative when the load exceeds the bin volume.
        assert_eq!(bin_waste_volume(&bin, &[10.0]), 0.0);
        assert_eq!(bin_waste_ratio(&bin, &[10.0]), 0.0);
    }

    /// 按箱型成本函数对未配置箱型回退 / Per-type cost function falls back for unconfigured types
    #[test]
    fn per_type_bin_cost_falls_back() {
        let mut type_cost = HashMap::new();
        type_cost.insert(bin_type_id("20GP"), 7.0);
        let cost = per_type_bin_cost(&type_cost, 0.5);

        assert!((cost(&sized_bin_type("20GP", 2.0)) - 7.0).abs() < 1e-9);
        assert!((cost(&sized_bin_type("40HQ", 4.0)) - 0.5).abs() < 1e-9);
    }

    /// final 执行器默认配置与 BPP2D 对齐 / Final executor defaults align with BPP2D
    #[test]
    fn final_executor_objective_defaults_match_bpp2d() {
        let config = crate::application::service::MetaModelFinalExecutorConfig::default();
        assert_eq!(config.final_bin_volume_weight, 1.0);
        assert_eq!(config.final_bin_amount_coefficient, 1e-6);
        assert_eq!(config.final_bin_type_cost_weight, 0.0);
        assert!(config.final_bin_type_costs.is_empty());
    }
}
