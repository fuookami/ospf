#[cfg(test)]
mod tests {
    use super::*;

    fn valid_dataset() -> &'static str {
        r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount,radius,axis,enabled_orientations,pattern_code,allow_mixed_loading,max_stack_layers,package_tags
i1,Item 1,cuboid,2,3,4,1,2,,,Upright|Side,PAT-A,false,2,fragile|top
c1,Cyl 1,cylinder,4,5,4,1,1,2,Y,Upright,,,,
# table:bins
bin_id,type_code,width,height,depth,capacity,is_main
b1,BIN,10,10,10,1000,true
# table:layers
layer_id,bin_id,depth,iteration,from,z
l1,b1,4,0,seed,0
# table:depth_boundary_policy
field,values
first_layer_allowed_cylinder_axes,Y|Z
last_layer_allowed_cylinder_axes,X
first_layer_allowed_cuboid_orientations,Upright|Side
"#
    }

    #[test]
    fn load_valid_multi_table_dataset() {
        let dataset = CsvDatasetLoader::load_str(valid_dataset()).unwrap();
        assert_eq!(dataset.items.len(), 2);
        assert_eq!(dataset.bins.len(), 1);
        assert_eq!(dataset.layers.len(), 1);

        let draft = dataset.to_request_draft().unwrap();
        assert_eq!(draft.item_count, 2);
        assert_eq!(draft.bin_count, 1);
        assert_eq!(draft.layer_count, 1);
        assert!(draft.depth_boundary_policy.is_some());
    }

    #[test]
    fn materialize_valid_multi_table_dataset() {
        let dataset = CsvDatasetLoader::load_str(valid_dataset()).unwrap();
        let request = dataset.materialize().unwrap();

        assert_eq!(request.items.len(), 2);
        assert_eq!(request.item_amounts, vec![("i1".to_string(), 2), ("c1".to_string(), 1)]);
        assert_eq!(request.bins.len(), 1);
        assert_eq!(request.initial_layers.len(), 1);
        assert_eq!(request.initial_layers[0].from, "seed");
        assert!(request.depth_boundary_policy.is_some());
        assert!(matches!(
            request.items[0].shape_spec_override,
            Some(PackageShapeSpec::Cuboid)
        ));
        assert!(matches!(
            &request.items[1].shape_spec_override,
            Some(PackageShapeSpec::Cylinder { axis: Axis3::Y, .. })
        ));
        assert_eq!(request.patterned_items[0].1.pattern_code, "PAT-A");
        assert_eq!(request.package_attributes[0].1.max_stack_layers, Some(2));
        assert!(request
            .diagnostics
            .iter()
            .any(|message| message.contains("patterned item key")));
    }

    #[test]
    fn adapt_kotlin_gurobi_csv_shapes() {
        let grouped = r#"
group_index,layer_index,item_id,material_no,material_name,material_weight_kg,shape_type,radius_min,radius_max,radius_weight_function_key,axis
0,0,item-g0-l0-pwl-cyl,MAT-PWL-H,Material-PWL-H,1.0,vertical_cylinder,0.30,0.50,cylinder_pwl_h,Y
0,0,item-g0-l0-cub1,MAT-PWL-I,Material-PWL-I,1.5,cuboid,,,,,
"#;
        let material_width = r#"
material,width,amount,material_no,material_name,material_weight_kg,shape_type,radius_meter,axis
MAT-A,1200,2,MAT-A,Material-A,1.0,vertical_cylinder,0.4,Y
MAT-B,1000,3,MAT-B,Material-B,1.5,cuboid,,
MAT-C,900,1,MAT-C,Material-C,2.0,vertical_cylinder,0.35,AXIS3.X
"#;

        let grouped_request = CsvDatasetLoader::load_any_str(grouped).unwrap().materialize().unwrap();
        let material_request = CsvDatasetLoader::load_any_str(material_width).unwrap().materialize().unwrap();

        assert_eq!(grouped_request.items.len(), 2);
        assert_eq!(material_request.item_amounts[0].1, 2);
        assert!(matches!(
            grouped_request.items[0].shape_spec_override,
            Some(PackageShapeSpec::Cylinder { .. })
        ));
        assert!(matches!(
            material_request.items[2].shape_spec_override,
            Some(PackageShapeSpec::Cylinder { axis: Axis3::X, .. })
        ));
        assert!(grouped_request
            .validate_business_rules()
            .iter()
            .any(|diagnostic| diagnostic.contains("patterned item")));
    }

    #[test]
    fn materialize_horizontal_cylinder() {
        let input = r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount,radius,axis
c1,Cyl,cylinder,5,4,4,1,1,2,X
# table:bins
bin_id,type_code,width,height,depth,capacity
b1,BIN,10,10,10,1000
"#;
        let request = CsvDatasetLoader::load_str(input).unwrap().materialize().unwrap();
        assert!(matches!(
            &request.items[0].shape_spec_override,
            Some(PackageShapeSpec::Cylinder { axis: Axis3::X, .. })
        ));
    }

    #[test]
    fn materialize_radius_interval_uses_conservative_radius() {
        let input = r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount,radius_min,radius_max,radius_step,axis
c1,Cyl,cylinder,5,4,4,1,1,1,3,1,Y
# table:bins
bin_id,type_code,width,height,depth,capacity
b1,BIN,10,10,10,1000
"#;
        let request = CsvDatasetLoader::load_str(input).unwrap().materialize().unwrap();
        let Some(PackageShapeSpec::Cylinder { radius, radius_candidates, .. }) =
            &request.items[0].shape_spec_override
        else {
            panic!("expected cylinder");
        };
        assert_eq!(radius.value, 3.0);
        assert_eq!(radius_candidates.as_ref().unwrap().len(), 3);
        assert!(request.diagnostics.iter().any(|message| message.contains("radius_max")));
    }

    #[test]
    fn materialize_radius_weight_functions_table() {
        let input = r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount,radius_min,radius_max,radius_step,radius_weight_function_key,axis
c1,Cyl,cylinder,5,4,4,1,1,1,3,1,cylinder_weight,Y
# table:bins
bin_id,type_code,width,height,depth,capacity
b1,BIN,10,10,10,1000
# table:radius_weight_functions
key,radius_squared_coefficient,intercept,objective_weight
cylinder_weight,2.5,1.0,4.0
"#;
        let request = CsvDatasetLoader::load_str(input).unwrap().materialize().unwrap();
        let component = request.continuous_radius_component.as_ref().unwrap();
        let prototype = &component.prototypes[0];
        let function = component.weight_function_for_prototype(prototype).unwrap();

        assert_eq!(component.weight_functions.len(), 1);
        assert_eq!(function.key, "cylinder_weight");
        assert_eq!(function.evaluate_radius_squared(4.0), 11.0);
        assert!(request
            .validate_business_rules()
            .iter()
            .any(|diagnostic| diagnostic.contains("weight_function_count=1")));
    }

    #[test]
    fn materialize_rejects_selected_radius_outside_bounds() {
        let input = r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount,radius,radius_min,radius_max,axis
c1,Cyl,cylinder,5,4,4,1,1,4,1,3,Y
# table:bins
bin_id,type_code,width,height,depth,capacity
b1,BIN,10,10,10,1000
"#;
        let error = CsvDatasetLoader::load_str(input).unwrap().materialize().unwrap_err();
        assert!(matches!(error, CsvDatasetError::InvalidValue { field, .. } if field == "radius"));
    }

    #[test]
    fn reject_duplicated_schema_columns() {
        let input = r#"
# table:items
item_id,item_id,name,shape_type,width,height,depth,weight,amount
i1,i1,Item,cuboid,1,1,1,1,1
# table:bins
bin_id,type_code,width,height,depth,capacity
b1,BIN,1,1,1,1
"#;
        let error = CsvDatasetLoader::load_str(input).unwrap_err();
        assert!(matches!(error, CsvDatasetError::DuplicatedColumn { .. }));
    }

    #[test]
    fn reject_unknown_schema_columns() {
        let input = r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount,extra
i1,Item,cuboid,1,1,1,1,1,x
# table:bins
bin_id,type_code,width,height,depth,capacity
b1,BIN,1,1,1,1
"#;
        let error = CsvDatasetLoader::load_str(input).unwrap_err();
        assert!(matches!(error, CsvDatasetError::UnknownColumn { .. }));
    }

    #[test]
    fn reject_missing_required_columns() {
        let input = r#"
# table:items
item_id,name,shape_type,width,height,depth,weight
i1,Item,cuboid,1,1,1,1
# table:bins
bin_id,type_code,width,height,depth,capacity
b1,BIN,1,1,1,1
"#;
        let error = CsvDatasetLoader::load_str(input).unwrap_err();
        assert!(matches!(error, CsvDatasetError::MissingRequiredColumn { column, .. } if column == "amount"));
    }

    #[test]
    fn reject_invalid_axis() {
        let input = r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount,radius,axis
c1,Cyl,cylinder,1,1,1,1,1,0.5,Q
# table:bins
bin_id,type_code,width,height,depth,capacity
b1,BIN,1,1,1,1
"#;
        let error = CsvDatasetLoader::load_str(input).unwrap_err();
        assert!(matches!(error, CsvDatasetError::InvalidValue { field, .. } if field == "axis"));
    }

    #[test]
    fn reject_invalid_shape_type() {
        let input = r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount
i1,Item,sphere,1,1,1,1,1
# table:bins
bin_id,type_code,width,height,depth,capacity
b1,BIN,1,1,1,1
"#;
        let error = CsvDatasetLoader::load_str(input).unwrap_err();
        assert!(matches!(error, CsvDatasetError::InvalidValue { field, .. } if field == "shape_type"));
    }

    #[test]
    fn reject_invalid_orientation() {
        let input = r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount,enabled_orientations
i1,Item,cuboid,1,1,1,1,1,Upright|Diagonal
# table:bins
bin_id,type_code,width,height,depth,capacity
b1,BIN,1,1,1,1
"#;
        let error = CsvDatasetLoader::load_str(input).unwrap_err();
        assert!(matches!(error, CsvDatasetError::InvalidValue { field, .. } if field == "enabled_orientations"));
    }

    #[test]
    fn reject_empty_depth_boundary_policy_set() {
        let input = r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount
i1,Item,cuboid,1,1,1,1,1
# table:bins
bin_id,type_code,width,height,depth,capacity
b1,BIN,1,1,1,1
# table:depth_boundary_policy
field,values
first_layer_allowed_cylinder_axes,
"#;
        let error = CsvDatasetLoader::load_str(input).unwrap_err();
        assert!(matches!(error, CsvDatasetError::InvalidValue { table, field, .. } if table == DEPTH_BOUNDARY_POLICY_TABLE && field == "values"));
    }

    #[test]
    fn reject_unknown_layer_bin() {
        let input = r#"
# table:items
item_id,name,shape_type,width,height,depth,weight,amount
i1,Item,cuboid,1,1,1,1,1
# table:bins
bin_id,type_code,width,height,depth,capacity
b1,BIN,1,1,1,1
# table:layers
layer_id,bin_id,depth
l1,missing,1
"#;
        let error = CsvDatasetLoader::load_str(input).unwrap_err();
        assert!(matches!(error, CsvDatasetError::InvalidValue { table, field, .. } if table == LAYERS_TABLE && field == "bin_id"));
    }

    #[test]
    fn single_table_input_defaults_to_items_and_requires_bins() {
        let input = r#"
item_id,name,shape_type,width,height,depth,weight,amount
i1,Item,cuboid,1,1,1,1,1
"#;
        let error = CsvDatasetLoader::load_str(input).unwrap_err();
        assert!(matches!(error, CsvDatasetError::MissingTable { table } if table == BINS_TABLE));
    }
}
