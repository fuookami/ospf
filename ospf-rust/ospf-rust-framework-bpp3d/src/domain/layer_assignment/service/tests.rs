// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_core::variable::VariableRange;
    use ospf_rust_framework::model::ColumnRange;
    use ospf_rust_quantities::unit::derived::Meter;
    use ospf_rust_quantities::quantity::Quantity;

    fn meters(v: f64) -> Quantity<f64, Meter> {
        Quantity::new_ct(v)
    }

    #[test]
    fn default_solver_value_adapter() {
        let adapter = DefaultBpp3dSolverValueAdapter;
        assert_eq!(adapter.amount_to_solver(10), 10.0);
        assert_eq!(adapter.length_to_solver(1.5), 1.5);
    }

    #[test]
    fn scaled_solver_value_adapter() {
        let adapter = ScaledBpp3dSolverValueAdapter::ten_times();
        assert_eq!(adapter.amount_to_solver(5), 50.0);
        assert_eq!(adapter.length_to_solver(1.5), 15.0);
        assert_eq!(adapter.weight_to_solver(2.0), 20.0);
        assert_eq!(adapter.volume_to_solver(3.0), 30.0);
        assert_eq!(adapter.depth_to_solver(1.0), 10.0);
    }

    #[test]
    fn scaled_solver_value_adapter_default() {
        let adapter = ScaledBpp3dSolverValueAdapter::default();
        assert_eq!(adapter.amount_to_solver(5), 5.0);
        assert_eq!(adapter.length_to_solver(1.5), 1.5);
    }

    #[test]
    fn layer_aggregation_add_columns() {
        let mut agg: LayerAggregation<f64, Meter> = LayerAggregation::new();
        assert!(agg.layers.is_empty());

        let layer1 = BinLayer {
            iteration: 0,
            from: "test".to_string(),
            bin: None,
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };
        let layer2 = BinLayer {
            iteration: 0,
            from: "test".to_string(),
            bin: None,
            depth: meters(2.0),
            demand_coverage: Vec::new(),
        };

        let added = agg.add_columns(vec![layer1.clone(), layer2.clone()]);
        assert_eq!(added.len(), 2);
        assert_eq!(agg.layers.len(), 2);
        assert_eq!(agg.last_iteration_layers().len(), 2);
    }

    #[test]
    fn layer_aggregation_deduplication() {
        let mut agg: LayerAggregation<f64, Meter> = LayerAggregation::new();

        let layer = BinLayer {
            iteration: 0,
            from: "test".to_string(),
            bin: None,
            depth: meters(1.0),
            demand_coverage: Vec::new(),
        };

        agg.add_columns(vec![layer.clone()]);
        assert_eq!(agg.layers.len(), 1);

        // 重复添加应被去重
        agg.add_columns(vec![layer.clone()]);
        assert_eq!(agg.layers.len(), 1);
    }

    #[test]
    fn load_model_construction() {
        let entries = vec![
            Bpp3dDemandEntry {
                mode: Bpp3dDemandMode::Item,
                key: Bpp3dDemandKey::Item { id: "item1".into() },
                demand: 10.0,
            },
        ];
        let load = Load::new(entries);
        assert_eq!(load.demand_entries.len(), 1);
    }

    #[test]
    fn capacity_model_construction() {
        let capacity = Capacity::new();
        assert_eq!(capacity.load_weight.len(), 0);
        assert_eq!(capacity.load_volume.len(), 0);
    }

    #[test]
    fn precise_load_capacity() {
        let plc = PreciseLoadCapacity {
            weight_capacity: 100.0,
            volume_capacity: 50.0,
            depth_capacity: 10.0,
        };
        assert_eq!(plc.weight_capacity, 100.0);
        assert_eq!(plc.volume_capacity, 50.0);
        assert_eq!(plc.depth_capacity, 10.0);
    }

    #[test]
    fn demand_shadow_price_key() {
        let key = DemandShadowPriceKey {
            mode: Bpp3dDemandMode::Item,
            key: Bpp3dDemandKey::Item { id: "item1".into() },
        };
        assert_eq!(key.mode, Bpp3dDemandMode::Item);
    }

    #[test]
    fn layer_assignment_aggregation_rmp() {
        let assignment = ImpreciseAssignment {
            layers: vec![BinLayer {
                iteration: 0,
                from: "test".to_string(),
                bin: None,
                depth: meters(1.0),
                demand_coverage: Vec::new(),
            }],
            x: None,
            upper_bounds: vec![None],
            load_weight_symbols: Vec::new(),
            load_volume_symbols: Vec::new(),
            load_depth_symbols: Vec::new(),
            load_symbols: Vec::new(),
        };
        let load = Load::new(vec![]);
        let capacity = Capacity::new();
        let agg = LayerAssignmentAggregation::rmp(assignment, load, capacity);
        assert!(agg.imprecise_assignment.is_some());
        assert!(agg.precise_assignment.is_none());
    }

    #[test]
    fn layer_assignment_context_register() {
        let mut model = MetaModel::<f64>::new("test_bpp3d_context");

        let assignment = ImpreciseAssignment {
            layers: vec![BinLayer {
                iteration: 0,
                from: "test".to_string(),
                bin: None,
                depth: meters(1.0),
                demand_coverage: Vec::new(),
            }],
            x: None,
            upper_bounds: vec![None],
            load_weight_symbols: Vec::new(),
            load_volume_symbols: Vec::new(),
            load_depth_symbols: Vec::new(),
            load_symbols: Vec::new(),
        };
        let load = Load::new(vec![]);
        let capacity = Capacity::new();
        let agg = LayerAssignmentAggregation::rmp(assignment, load, capacity);
        let mut ctx = LayerAssignmentContext::new(agg);

        ctx.register(&mut model).unwrap();
        ctx.invoke(&model).unwrap();
    }

    #[test]
    fn iterative_layer_assignment_context_uses_framework_lifecycle() {
        let layers = vec![
            BinLayer {
                iteration: 0,
                from: "test-a".to_string(),
                bin: None,
                depth: meters(1.0),
                demand_coverage: Vec::new(),
            },
            BinLayer {
                iteration: 0,
                from: "test-b".to_string(),
                bin: None,
                depth: meters(2.0),
                demand_coverage: Vec::new(),
            },
        ];
        let mut iterative = IterativeLayerAssignmentContext::new();
        let added = iterative.add_columns(0, layers.clone(), vec![Some(2.0), Some(3.0)]);
        assert_eq!(added.len(), 2);

        let mut assignment = ImpreciseAssignment {
            layers,
            x: None,
            upper_bounds: vec![Some(2.0), Some(3.0)],
            load_weight_symbols: Vec::new(),
            load_volume_symbols: Vec::new(),
            load_depth_symbols: Vec::new(),
            load_symbols: Vec::new(),
        };
        let mut model = MetaModel::<f64>::new("test_bpp3d_iterative_lifecycle");
        assignment.register(&mut model).unwrap();
        iterative.bind_assignment(&assignment);

        let first_model_index = assignment.x.as_ref().unwrap().model_index(&0).unwrap();
        let second_model_index = assignment.x.as_ref().unwrap().model_index(&1).unwrap();
        let mut lifecycle = DynamicModelLifecycle::new();

        iterative
            .hide_columns_in_model(&mut lifecycle, &mut model, [0])
            .unwrap();
        assert_eq!(
            model.variable_range_by_index(first_model_index),
            Some(VariableRange::bounded(0.0, 0.0))
        );

        iterative
            .fix_columns_in_model(&mut lifecycle, &mut model, [1])
            .unwrap();
        assert_eq!(
            model.variable_range_by_index(second_model_index),
            Some(VariableRange::bounded(1.0, 1.0))
        );

        lifecycle.set_solution_to_model(&mut model, vec![0.0, 1.0]);
        let values = iterative.extract_selectable_values(&[0.0, 1.0], &lifecycle);
        assert_eq!(values.get(&1), Some(&1.0));
        assert!(!values.contains_key(&0));

        iterative
            .flush_model(&mut lifecycle, &mut model, false)
            .unwrap();
        assert_eq!(lifecycle.column_range(first_model_index), Some(ColumnRange::binary()));
        assert_eq!(
            model.variable_range_by_index(first_model_index),
            Some(VariableRange::new(Some(0.0), Some(2.0)))
        );
        assert_eq!(
            model.variable_range_by_index(second_model_index),
            Some(VariableRange::new(Some(0.0), Some(3.0)))
        );

        iterative
            .remove_columns_in_model(&mut lifecycle, &mut model, [0])
            .unwrap();
        assert_eq!(
            model.variable_range_by_index(first_model_index),
            Some(VariableRange::bounded(0.0, 0.0))
        );
        assert_eq!(iterative.active_column_count(), 1);
    }

    #[test]
    fn iterative_layer_assignment_context_add_columns_to_existing_model() {
        let mut iterative = IterativeLayerAssignmentContext::new();
        let mut assignment = ImpreciseAssignment {
            layers: Vec::new(),
            x: None,
            upper_bounds: Vec::new(),
            load_weight_symbols: Vec::new(),
            load_volume_symbols: Vec::new(),
            load_depth_symbols: Vec::new(),
            load_symbols: Vec::new(),
        };
        let mut model = MetaModel::<f64>::new("test_bpp3d_incremental_columns");

        let added = iterative
            .add_columns_to_model(
                0,
                vec![
                    BinLayer {
                        iteration: 0,
                        from: "test-a".to_string(),
                        bin: None,
                        depth: meters(1.0),
                        demand_coverage: Vec::new(),
                    },
                    BinLayer {
                        iteration: 0,
                        from: "test-b".to_string(),
                        bin: None,
                        depth: meters(2.0),
                        demand_coverage: Vec::new(),
                    },
                ],
                vec![Some(2.0), Some(3.0)],
                &mut assignment,
                &mut model,
            )
            .unwrap();

        assert_eq!(added.len(), 2);
        assert_eq!(assignment.layers.len(), 2);
        assert_eq!(assignment.upper_bounds, vec![Some(2.0), Some(3.0)]);

        assignment.register(&mut model).unwrap();
        iterative.bind_assignment(&assignment);
        assert_eq!(assignment.x.as_ref().unwrap().model_index(&0), Some(0));
        assert_eq!(assignment.x.as_ref().unwrap().model_index(&1), Some(1));
        assert_eq!(
            model.variable_range_by_index(0),
            Some(VariableRange::new(Some(0.0), Some(2.0)))
        );
        assert_eq!(
            model.variable_range_by_index(1),
            Some(VariableRange::new(Some(0.0), Some(3.0)))
        );
        assert_eq!(iterative.column_to_model_index.get(&0), Some(&0));
        assert_eq!(iterative.column_to_model_index.get(&1), Some(&1));

        let mut lifecycle = DynamicModelLifecycle::new();
        iterative
            .hide_columns_in_model(&mut lifecycle, &mut model, [0])
            .unwrap();
        assert_eq!(
            model.variable_range_by_index(0),
            Some(VariableRange::bounded(0.0, 0.0))
        );

        iterative
            .fix_columns_in_model(&mut lifecycle, &mut model, [1])
            .unwrap();
        assert_eq!(
            model.variable_range_by_index(1),
            Some(VariableRange::bounded(1.0, 1.0))
        );

        lifecycle.set_solution_to_model(&mut model, vec![0.0, 1.0]);
        let values = iterative.extract_selectable_values(&[0.0, 1.0], &lifecycle);
        assert_eq!(values.get(&1), Some(&1.0));
        assert!(!values.contains_key(&0));

        iterative
            .flush_model(&mut lifecycle, &mut model, false)
            .unwrap();
        assert_eq!(
            model.variable_range_by_index(0),
            Some(VariableRange::new(Some(0.0), Some(2.0)))
        );
        assert_eq!(
            model.variable_range_by_index(1),
            Some(VariableRange::new(Some(0.0), Some(3.0)))
        );
    }
}
