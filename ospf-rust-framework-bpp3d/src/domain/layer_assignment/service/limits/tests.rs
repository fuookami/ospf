// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::item::{
        BinLayer, BinType, Bpp3dDemandKey, Bpp3dDemandMode, Bpp3dLayerDemandCoverage,
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
            type_code: "BIN-10".to_string(),
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
            key: crate::domain::item::Bpp3dDemandKey::Item { id: "item1".to_string() },
            demand: 10.0,
        };
        let key = DemandConstraint::<f64, Meter>::shadow_price_key(&entry);
        assert_eq!(key.mode, crate::domain::item::Bpp3dDemandMode::Item);
    }

    #[test]
    fn demand_constraint_registers_linear_cover_rows() {
        let key = Bpp3dDemandKey::Item { id: "item1".to_string() };
        let mut assignment = ImpreciseAssignment {
            layers: vec![
                make_layer(1.0).with_demand_coverage(vec![Bpp3dLayerDemandCoverage::new(
                    Bpp3dDemandMode::Item,
                    key.clone(),
                    1.0,
                )]),
                make_layer(2.0),
            ],
            x: VariableArray1::new("x"),
            upper_bounds: vec![None, None],
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
            x: VariableArray2::new("x"),
            v: VariableArray1::new("v"),
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
            x: VariableArray1::new("x"),
            upper_bounds: vec![None],
        };
        let entries = vec![
            Bpp3dDemandEntry {
                mode: crate::domain::item::Bpp3dDemandMode::Item,
                key: crate::domain::item::Bpp3dDemandKey::Item { id: "item1".to_string() },
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
            key: crate::domain::item::Bpp3dDemandKey::Item { id: "item1".to_string() },
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
            x: VariableArray1::new("x"),
            upper_bounds: vec![None],
        };
        let entries = vec![
            Bpp3dDemandEntry {
                mode: crate::domain::item::Bpp3dDemandMode::Item,
                key: crate::domain::item::Bpp3dDemandKey::Item { id: "item1".to_string() },
                demand: 10.0,
            },
        ];
        let constraint: DemandConstraint<f64, Meter> = DemandConstraint::imprecise(entries, assignment);

        let extractor = constraint.extractor();
        assert!(extractor.is_some());

        let mut map: BasicShadowPriceMap<DemandShadowPriceKey> = BasicShadowPriceMap::new();
        let sp_key = DemandShadowPriceKey {
            mode: crate::domain::item::Bpp3dDemandMode::Item,
            key: crate::domain::item::Bpp3dDemandKey::Item { id: "item1".to_string() },
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

        let constraint = BinCapacityConstraint::from_bins(
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

        let constraint = BinDepthConstraint::from_bins(
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

        let constraint = BinCapacityConstraint::from_bins(
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

        let constraint = BinCapacityConstraint::new(
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

        let constraint = BinDepthConstraint::new(
            vec![10.0],      // depth capacity
            x_indices,
            vec![2.0],       // layer depths
        );

        constraint.register(&mut model);
        constraint.invoke(&model).unwrap();
    }
}
