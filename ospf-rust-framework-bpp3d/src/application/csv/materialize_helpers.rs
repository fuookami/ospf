fn optional_f64(value: Option<f64>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn materialize_item(
    record: &CsvItemRecord,
    row: usize,
    diagnostics: &mut Vec<String>,
) -> Result<ActualItem<f64, Meter>, CsvDatasetError> {
    let shape_type = record.parsed_shape_type(row)?;
    let enabled_orientations = {
        let parsed = record.parsed_enabled_orientations(row)?;
        if parsed.is_empty() {
            vec![Orientation::Upright]
        } else {
            parsed
        }
    };
    let shape_spec_override = match shape_type {
        CsvShapeType::Cuboid => Some(PackageShapeSpec::Cuboid),
        CsvShapeType::Cylinder => {
            let axis = record.parsed_axis(row)?.ok_or_else(|| {
                invalid_value(ITEMS_TABLE, row, "axis", "", "cylinder item requires axis")
            })?;
            let radius = materialize_radius(record, row, diagnostics)?;
            Some(PackageShapeSpec::Cylinder {
                axis,
                radius: meters(radius),
                radius_candidates: materialize_radius_candidates(record, row)?,
                radius_lower_bound: record.radius_min.map(meters),
                radius_upper_bound: record.radius_max.map(meters),
            })
        }
    };
    if let Some(pattern_code) = record.pattern_code.as_ref().filter(|value| !value.trim().is_empty()) {
        diagnostics.push(format!(
            "item {} uses patterned item key {}",
            record.item_id,
            pattern_code.trim(),
        ));
    }
    if let Some(attribute) = materialize_package_attribute(record) {
        diagnostics.extend(attribute.1.packing_diagnostics(1));
    }

    Ok(ActualItem {
        id: record.item_id.clone().into(),
        name: record.name.clone(),
        package_code: record.package_code.clone().or_else(|| {
            record
                .pattern_code
                .as_ref()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        }),
        pack: None,
        width: meters(record.width),
        height: meters(record.height),
        depth: meters(record.depth),
        weight: meters(record.weight),
        enabled_orientations,
        shape_spec_override,
    })
}

fn materialize_package_attribute(record: &CsvItemRecord) -> Option<(String, PackageAttribute)> {
    let tags = record
        .package_tags
        .as_deref()
        .map(parse_tags)
        .unwrap_or_default();
    if record.allow_mixed_loading.is_none()
        && record.max_stack_layers.is_none()
        && tags.is_empty()
    {
        return None;
    }
    Some((
        record.item_id.clone(),
        PackageAttribute {
            allow_mixed_loading: record.allow_mixed_loading,
            max_stack_layers: record.max_stack_layers,
            tags,
            ..Default::default()
        },
    ))
}

fn materialize_continuous_radius_component(
    records: &[CsvItemRecord],
) -> Result<Option<ContinuousRadiusModelComponent>, CsvDatasetError> {
    let mut prototypes = Vec::new();
    let mut variable_names = Vec::new();
    let mut registered_variables = Vec::new();
    let mut blocked_variables = Vec::new();
    for (index, record) in records.iter().enumerate() {
        let Some(function_key) = record
            .radius_weight_function_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        if record.parsed_shape_type(index + 2)? != CsvShapeType::Cylinder {
            return Err(invalid_value(
                ITEMS_TABLE,
                index + 2,
                "radius_weight_function_key",
                function_key,
                "radius weight function key requires cylinder shape",
            ));
        }
        let axis = record.parsed_axis(index + 2)?.ok_or_else(|| {
            invalid_value(
                ITEMS_TABLE,
                index + 2,
                "axis",
                "",
                "continuous radius cylinder requires axis",
            )
        })?;
        let variable_name = format!("r_{}_{}", sanitize_identifier(&record.item_id), sanitize_identifier(function_key));
        variable_names.push(variable_name.clone());
        if record.radius_min.is_some() && record.radius_max.is_some() {
            registered_variables.push(variable_name.clone());
        } else {
            blocked_variables.push(variable_name.clone());
        }
        prototypes.push(ContinuousCylinderRadiusSolverPrototype {
            item_id: record.item_id.clone().into(),
            source: function_key.to_string(),
            axis,
            variable_name,
            radius_lower_bound: record.radius_min,
            radius_upper_bound: record.radius_max,
        });
    }
    if prototypes.is_empty() {
        return Ok(None);
    }
    Ok(Some(ContinuousRadiusModelComponent {
        prototypes,
        registration_plan: ContinuousRadiusRegistrationPlan {
            variable_names,
            registered_variables,
            blocked_variables,
        },
        weight_functions: HashMap::new(),
        objective_policy: Default::default(),
    }))
}

fn materialize_radius_weight_functions(
    records: &[CsvRadiusWeightFunctionRecord],
) -> Result<HashMap<String, ContinuousRadiusWeightFunction>, CsvDatasetError> {
    let mut functions = HashMap::new();
    for (index, record) in records.iter().enumerate() {
        let key = record.key.trim();
        if key.is_empty() {
            return Err(invalid_value(
                RADIUS_WEIGHT_FUNCTIONS_TABLE,
                index + 2,
                "key",
                &record.key,
                "radius weight function key must be non-empty",
            ));
        }
        if functions
            .insert(
                key.to_string(),
                ContinuousRadiusWeightFunction::new(
                    key,
                    record.intercept.unwrap_or(0.0),
                    record.radius_squared_coefficient,
                    record.objective_weight.unwrap_or(1.0),
                ),
            )
            .is_some()
        {
            return Err(invalid_value(
                RADIUS_WEIGHT_FUNCTIONS_TABLE,
                index + 2,
                "key",
                key,
                "duplicated radius weight function key",
            ));
        }
    }
    Ok(functions)
}

fn sanitize_identifier(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    if sanitized.is_empty() {
        "unnamed".to_string()
    } else {
        sanitized
    }
}

fn parse_tags(value: &str) -> Vec<String> {
    value
        .split(['|', ';', ','])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn materialize_radius(
    record: &CsvItemRecord,
    row: usize,
    diagnostics: &mut Vec<String>,
) -> Result<f64, CsvDatasetError> {
    if let Some(radius) = record.radius {
        if let (Some(min), Some(max)) = (record.radius_min, record.radius_max) {
            if radius < min || radius > max {
                return Err(invalid_value(
                    ITEMS_TABLE,
                    row,
                    "radius",
                    &radius.to_string(),
                    "selected radius must be within radius_min and radius_max",
                ));
            }
        }
        return Ok(radius);
    }
    match (record.radius_min, record.radius_max) {
        (Some(min), Some(max)) if min <= max => {
            diagnostics.push(format!(
                "item {} uses radius_max as conservative materialized radius",
                record.item_id
            ));
            Ok(max)
        }
        (Some(_), Some(_)) => Err(invalid_value(
            ITEMS_TABLE,
            row,
            "radius_min",
            &record.radius_min.unwrap_or_default().to_string(),
            "radius_min must be <= radius_max",
        )),
        _ => {
            diagnostics.push(format!(
                "item {} uses width/2 as inferred cylinder radius",
                record.item_id
            ));
            Ok(record.width / 2.0)
        }
    }
}

fn materialize_radius_candidates(
    record: &CsvItemRecord,
    row: usize,
) -> Result<Option<Vec<Quantity<f64, Meter>>>, CsvDatasetError> {
    let (Some(min), Some(max), Some(step)) = (record.radius_min, record.radius_max, record.radius_step) else {
        return Ok(None);
    };
    if min > max {
        return Err(invalid_value(
            ITEMS_TABLE,
            row,
            "radius_min",
            &min.to_string(),
            "radius_min must be <= radius_max",
        ));
    }
    if step <= 0.0 {
        return Err(invalid_value(
            ITEMS_TABLE,
            row,
            "radius_step",
            &step.to_string(),
            "radius_step must be positive",
        ));
    }
    let mut values = Vec::new();
    let mut value = min;
    while value <= max + 1e-9 {
        values.push(meters(value));
        value += step;
    }
    Ok(Some(values))
}

fn materialize_bin(record: &CsvBinRecord) -> BinType<f64, Meter> {
    BinType {
        width: meters(record.width),
        height: meters(record.height),
        depth: meters(record.depth),
        capacity: meters(record.capacity),
        type_code: record.type_code.clone().into(),
        is_main: record.is_main.unwrap_or(false),
    }
}

fn materialize_layer(
    record: &CsvLayerRecord,
    row: usize,
    bin_by_id: &HashMap<String, BinType<f64, Meter>>,
) -> Result<BinLayer<f64, Meter>, CsvDatasetError> {
    let bin = bin_by_id.get(&record.bin_id).cloned().ok_or_else(|| {
        invalid_value(LAYERS_TABLE, row, "bin_id", &record.bin_id, "layer references unknown bin")
    })?;
    Ok(BinLayer {
        iteration: record.iteration.unwrap_or(0),
        from: record.from.clone().unwrap_or_else(|| "csv".to_string()),
        bin: Some(bin),
        depth: meters(record.depth),
        demand_coverage: Vec::new(),
    })
}

fn known_table(table: &str) -> bool {
    matches!(
        table,
        ITEMS_TABLE
            | BINS_TABLE
            | LAYERS_TABLE
            | DEPTH_BOUNDARY_POLICY_TABLE
            | RADIUS_WEIGHT_FUNCTIONS_TABLE
    )
}

fn parse_axis(table: &str, row: usize, field: &str, value: &str) -> Result<Axis3, CsvDatasetError> {
    match normalize_token(value).as_str() {
        "x" | "axis3x" => Ok(Axis3::X),
        "y" | "axis3y" => Ok(Axis3::Y),
        "z" | "axis3z" => Ok(Axis3::Z),
        _ => Err(invalid_value(table, row, field, value, "expected X, Y, or Z")),
    }
}

fn parse_axis_set(
    table: &str,
    row: usize,
    field: &str,
    value: &str,
) -> Result<HashSet<Axis3>, CsvDatasetError> {
    let values = split_list(value);
    if values.is_empty() {
        return Err(invalid_value(table, row, field, value, "axis set must be non-empty"));
    }
    values
        .iter()
        .map(|axis| parse_axis(table, row, field, axis))
        .collect()
}

fn parse_orientation(
    table: &str,
    row: usize,
    field: &str,
    value: &str,
) -> Result<Orientation, CsvDatasetError> {
    match normalize_token(value).as_str() {
        "upright" => Ok(Orientation::Upright),
        "uprightrotated" => Ok(Orientation::UprightRotated),
        "side" => Ok(Orientation::Side),
        "siderotated" => Ok(Orientation::SideRotated),
        "lie" => Ok(Orientation::Lie),
        "lierotated" => Ok(Orientation::LieRotated),
        _ => Err(invalid_value(table, row, field, value, "unknown orientation")),
    }
}

fn parse_orientation_set(
    table: &str,
    row: usize,
    field: &str,
    value: Option<&str>,
    require_non_empty: bool,
) -> Result<Vec<Orientation>, CsvDatasetError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let values = split_list(value);
    if require_non_empty && values.is_empty() {
        return Err(invalid_value(table, row, field, value, "orientation set must be non-empty"));
    }
    values
        .iter()
        .map(|orientation| parse_orientation(table, row, field, orientation))
        .collect()
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(['|', ';', ','])
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn normalize_token(value: &str) -> String {
    value
        .trim()
        .replace(['_', '-', ' ', '.'], "")
        .to_ascii_lowercase()
}

fn invalid_value(
    table: &str,
    row: usize,
    field: &str,
    value: &str,
    reason: &str,
) -> CsvDatasetError {
    CsvDatasetError::InvalidValue {
        table: table.to_string(),
        row,
        field: field.to_string(),
        value: value.to_string(),
        reason: reason.to_string(),
    }
}

fn meters(value: f64) -> Quantity<f64, Meter> {
    Quantity::new_ct(value)
}

