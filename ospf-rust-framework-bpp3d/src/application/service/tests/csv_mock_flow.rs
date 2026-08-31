    #[cfg(feature = "serde")]
    #[test]
    fn application_service_runs_csv_materialized_mock_flow() {
        let input = r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount
i1,Item,cuboid,2,3,4,1,1
# table:bins
bin_id,type_code,width,height,depth,capacity
b1,BIN,10,10,10,1000
# table:layers
layer_id,bin_id,depth
l1,b1,4
"#;
        let request = crate::application::csv::CsvDatasetLoader::load_str(input)
            .unwrap()
            .materialize()
            .unwrap();
        let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
        let rmp = MockColumnGenerationRmpExecutor {
            objective: Some(3.0),
        };
        let final_executor = MockColumnGenerationFinalExecutor;

        let result = service
            .run_csv_materialized(request, &rmp, &final_executor)
            .unwrap();

        assert_eq!(result.result.layers.len(), 1);
        assert_eq!(result.rmp.objective, Some(3.0));
    }
