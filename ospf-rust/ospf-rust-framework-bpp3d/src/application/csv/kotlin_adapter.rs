/// Kotlin Gurobi CSV 适配器 / Kotlin Gurobi CSV adapter
#[derive(Debug, Clone, Default)]
pub struct KotlinGurobiCsvAdapter;

impl KotlinGurobiCsvAdapter {
    /// 适配 Kotlin Gurobi CSV 为 Rust 多表 CSV / Adapt Kotlin Gurobi CSV into Rust multi-table CSV
    pub fn adapt(input: &str) -> Result<String, CsvDatasetError> {
        let mut reader = ReaderBuilder::new()
            .trim(Trim::All)
            .flexible(true)
            .from_reader(input.as_bytes());
        let headers = reader.headers()
            .map_err(|error| CsvDatasetError::Parse {
                table: "kotlin_gurobi".to_string(),
                message: error.to_string(),
            })?
            .clone();
        if has_columns(&headers, &["group_index", "layer_index", "item_id"]) {
            adapt_grouped_layer(reader, &headers)
        } else if has_columns(&headers, &["material", "width", "amount"]) {
            adapt_material_width_amount(reader, &headers)
        } else {
            Err(CsvDatasetError::MissingRequiredColumn {
                table: "kotlin_gurobi".to_string(),
                column: "group_index/layer_index/item_id or material/width/amount".to_string(),
            })
        }
    }
}

fn split_tables(input: &str) -> Result<HashMap<String, String>, CsvDatasetError> {
    let mut tables = HashMap::new();
    let mut current_table = if input.lines().any(|line| line.trim_start().starts_with("# table:")) {
        None
    } else {
        Some(ITEMS_TABLE.to_string())
    };
    let mut current_lines = Vec::new();

    for raw_line in input.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("# table:") {
            flush_table(&mut tables, current_table.take(), &mut current_lines)?;
            let name = rest.trim().to_string();
            if !known_table(&name) {
                return Err(CsvDatasetError::UnknownTable { table: name });
            }
            current_table = Some(name);
        } else if trimmed.starts_with('#') {
            continue;
        } else {
            current_lines.push(raw_line.to_string());
        }
    }
    flush_table(&mut tables, current_table, &mut current_lines)?;
    Ok(tables)
}

fn flush_table(
    tables: &mut HashMap<String, String>,
    table: Option<String>,
    lines: &mut Vec<String>,
) -> Result<(), CsvDatasetError> {
    if lines.is_empty() {
        return Ok(());
    }
    let table = table.ok_or_else(|| CsvDatasetError::Parse {
        table: "unknown".to_string(),
        message: "rows appear before a # table:<name> marker".to_string(),
    })?;
    if tables.insert(table.clone(), lines.join("\n")).is_some() {
        return Err(CsvDatasetError::Parse {
            table,
            message: "duplicated table section".to_string(),
        });
    }
    lines.clear();
    Ok(())
}

fn read_table<T>(
    tables: &HashMap<String, String>,
    table: &str,
    required: &[&str],
    optional: &[&str],
    required_table: bool,
) -> Result<Vec<T>, CsvDatasetError>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let Some(content) = tables.get(table) else {
        return if required_table {
            Err(CsvDatasetError::MissingTable {
                table: table.to_string(),
            })
        } else {
            Ok(Vec::new())
        };
    };
    let mut reader = ReaderBuilder::new()
        .trim(Trim::All)
        .from_reader(content.as_bytes());
    let headers = reader.headers()
        .map_err(|error| CsvDatasetError::Parse {
            table: table.to_string(),
            message: error.to_string(),
        })?
        .clone();
    CsvSchemaGuard::validate(table, &headers, required, optional)?;
    reader.deserialize()
        .map(|record| {
            record.map_err(|error| CsvDatasetError::Parse {
                table: table.to_string(),
                message: error.to_string(),
            })
        })
        .collect()
}

fn adapt_grouped_layer(
    mut reader: csv::Reader<&[u8]>,
    headers: &StringRecord,
) -> Result<String, CsvDatasetError> {
    let mut items = Vec::new();
    let mut layers = HashSet::new();
    let mut depth_boundary_policy = HashMap::<String, String>::new();
    for (index, row) in reader.records().enumerate() {
        let row = row.map_err(|error| CsvDatasetError::Parse {
            table: "kotlin_gurobi".to_string(),
            message: error.to_string(),
        })?;
        let item_id = field(headers, &row, "item_id").unwrap_or_else(|| format!("item-{}", index));
        let layer_index = field(headers, &row, "layer_index").unwrap_or_else(|| "0".to_string());
        let material_name = field(headers, &row, "material_name").unwrap_or_else(|| item_id.clone());
        let material_weight = parse_optional_f64(headers, &row, "material_weight_kg")?.unwrap_or(1.0);
        let shape_type = field(headers, &row, "shape_type").unwrap_or_else(|| "cuboid".to_string());
        let axis = field(headers, &row, "axis").filter(|value| !value.trim().is_empty());
        let radius = parse_optional_f64(headers, &row, "radius_meter")?;
        let (radius_min, radius_max, radius_step) = kotlin_radius_bounds(headers, &row)?;
        let width = parse_optional_f64(headers, &row, "width_meter")?
            .unwrap_or_else(|| radius.or(radius_max).map(|value| value * 2.0).unwrap_or(1.0));
        let height = parse_optional_f64(headers, &row, "height_meter")?
            .unwrap_or_else(|| radius.or(radius_max).map(|value| value * 2.0).unwrap_or(1.0));
        let depth = parse_optional_f64(headers, &row, "depth_meter")?
            .unwrap_or(1.0);
        collect_depth_boundary_policy(headers, &row, &mut depth_boundary_policy)?;
        items.push(CsvItemRecord {
            item_id,
            name: material_name,
            shape_type: normalize_kotlin_shape_type(&shape_type),
            width,
            height,
            depth,
            weight: material_weight,
            amount: 1,
            package_code: None,
            material_no: field(headers, &row, "material_no"),
            material_name: field(headers, &row, "material_name"),
            material_weight: Some(material_weight),
            radius,
            radius_min,
            radius_max,
            radius_step,
            radius_weight_function_key: field(headers, &row, "radius_weight_function_key"),
            axis: axis.or_else(|| if is_kotlin_cylinder(&shape_type) { Some("Y".to_string()) } else { None }),
            enabled_orientations: None,
            pattern_code: Some(format!("G{}", field(headers, &row, "group_index").unwrap_or_else(|| "0".to_string()))),
            allow_mixed_loading: Some(true),
            max_stack_layers: Some(4),
            package_tags: Some("kotlin-grouped".to_string()),
        });
        layers.insert(layer_index);
    }
    build_adapted_dataset(items, layers, depth_boundary_policy)
}

fn adapt_material_width_amount(
    mut reader: csv::Reader<&[u8]>,
    headers: &StringRecord,
) -> Result<String, CsvDatasetError> {
    let mut items = Vec::new();
    let mut layers = HashSet::new();
    let depth_boundary_policy = HashMap::new();
    for (index, row) in reader.records().enumerate() {
        let row = row.map_err(|error| CsvDatasetError::Parse {
            table: "kotlin_gurobi".to_string(),
            message: error.to_string(),
        })?;
        let material = field(headers, &row, "material").unwrap_or_else(|| format!("material-{}", index));
        let width = parse_required_f64(headers, &row, "width")? / 1000.0;
        let amount = parse_optional_u64(headers, &row, "amount")?.unwrap_or(1);
        let shape_type = field(headers, &row, "shape_type").unwrap_or_else(|| "cuboid".to_string());
        let radius = parse_optional_f64(headers, &row, "radius_meter")?;
        let (radius_min, radius_max, radius_step) = kotlin_radius_bounds(headers, &row)?;
        items.push(CsvItemRecord {
            item_id: material.clone(),
            name: field(headers, &row, "material_name").unwrap_or_else(|| material.clone()),
            shape_type: normalize_kotlin_shape_type(&shape_type),
            width,
            height: radius.map(|value| value * 2.0).unwrap_or(1.0),
            depth: radius.map(|value| value * 2.0).unwrap_or(width),
            weight: parse_optional_f64(headers, &row, "material_weight_kg")?.unwrap_or(1.0),
            amount,
            package_code: None,
            material_no: field(headers, &row, "material_no"),
            material_name: field(headers, &row, "material_name"),
            material_weight: parse_optional_f64(headers, &row, "material_weight_kg")?,
            radius,
            radius_min,
            radius_max,
            radius_step,
            radius_weight_function_key: field(headers, &row, "radius_weight_function_key"),
            axis: field(headers, &row, "axis").filter(|value| !value.trim().is_empty())
                .or_else(|| if is_kotlin_cylinder(&shape_type) { Some("Y".to_string()) } else { None }),
            enabled_orientations: None,
            pattern_code: Some("material-width-amount".to_string()),
            allow_mixed_loading: Some(true),
            max_stack_layers: Some(4),
            package_tags: Some("kotlin-material-width-amount".to_string()),
        });
        layers.insert("0".to_string());
    }
    build_adapted_dataset(items, layers, depth_boundary_policy)
}

fn build_adapted_dataset(
    items: Vec<CsvItemRecord>,
    layers: HashSet<String>,
    depth_boundary_policy: HashMap<String, String>,
) -> Result<String, CsvDatasetError> {
    let mut output = String::new();
    output.push_str("# table:items\n");
    output.push_str("item_id,name,shape_type,width,height,depth,weight,amount,radius,radius_min,radius_max,radius_step,radius_weight_function_key,axis,pattern_code,allow_mixed_loading,max_stack_layers,package_tags\n");
    for item in items {
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            item.item_id,
            item.name,
            item.shape_type,
            item.width,
            item.height,
            item.depth,
            item.weight,
            item.amount,
            optional_f64(item.radius),
            optional_f64(item.radius_min),
            optional_f64(item.radius_max),
            optional_f64(item.radius_step),
            item.radius_weight_function_key.unwrap_or_default(),
            item.axis.unwrap_or_default(),
            item.pattern_code.unwrap_or_default(),
            item.allow_mixed_loading.unwrap_or(true),
            item.max_stack_layers.unwrap_or(4),
            item.package_tags.unwrap_or_default(),
        ));
    }
    output.push_str("# table:bins\n");
    output.push_str("bin_id,type_code,width,height,depth,capacity\n");
    output.push_str("b0,BIN,20,20,20,100000\n");
    output.push_str("# table:layers\n");
    output.push_str("layer_id,bin_id,depth\n");
    let mut layers = layers.into_iter().collect::<Vec<_>>();
    layers.sort();
    if layers.is_empty() {
        layers.push("0".to_string());
    }
    for layer in layers {
        output.push_str(&format!("l{},b0,1\n", layer));
    }
    if !depth_boundary_policy.is_empty() {
        output.push_str("# table:depth_boundary_policy\n");
        output.push_str("field,values\n");
        let mut fields = depth_boundary_policy.into_iter().collect::<Vec<_>>();
        fields.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
        for (field, values) in fields {
            output.push_str(&format!("{},{}\n", field, values));
        }
    }
    Ok(output)
}

fn has_columns(headers: &StringRecord, columns: &[&str]) -> bool {
    columns.iter().all(|column| {
        headers
            .iter()
            .any(|header| header.eq_ignore_ascii_case(column))
    })
}

fn field(headers: &StringRecord, row: &StringRecord, name: &str) -> Option<String> {
    headers
        .iter()
        .position(|header| header.eq_ignore_ascii_case(name))
        .and_then(|index| row.get(index))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn kotlin_radius_bounds(
    headers: &StringRecord,
    row: &StringRecord,
) -> Result<(Option<f64>, Option<f64>, Option<f64>), CsvDatasetError> {
    let radius_min = parse_optional_f64(headers, row, "radius_min")?;
    let radius_max = parse_optional_f64(headers, row, "radius_max")?;
    let radius_step = parse_optional_f64(headers, row, "radius_step")?;
    let diameter_min = parse_optional_f64(headers, row, "diameter_min")?;
    let diameter_max = parse_optional_f64(headers, row, "diameter_max")?;
    let diameter_step = parse_optional_f64(headers, row, "diameter_step")?;
    Ok((
        radius_min.or_else(|| diameter_min.map(|value| value / 2.0)),
        radius_max.or_else(|| diameter_max.map(|value| value / 2.0)),
        radius_step
            .or_else(|| diameter_step.map(|value| value / 2.0))
            .or_else(|| {
                radius_min
                    .zip(radius_max)
                    .map(|(min, max)| (max - min).max(0.0))
            })
            .or_else(|| {
                diameter_min
                    .zip(diameter_max)
                    .map(|(min, max)| ((max - min) / 2.0).max(0.0))
            }),
    ))
}

fn collect_depth_boundary_policy(
    headers: &StringRecord,
    row: &StringRecord,
    policies: &mut HashMap<String, String>,
) -> Result<(), CsvDatasetError> {
    for field_name in [
        "first_layer_allowed_cylinder_axes",
        "last_layer_allowed_cylinder_axes",
        "first_layer_allowed_cuboid_orientations",
        "last_layer_allowed_cuboid_orientations",
    ] {
        let Some(value) = field(headers, row, field_name) else {
            continue;
        };
        match policies.get(field_name) {
            Some(previous) if previous != &value => {
                return Err(invalid_value(
                    "kotlin_gurobi",
                    0,
                    field_name,
                    &value,
                    "depth boundary policy must be consistent within one scenario",
                ));
            }
            Some(_) => {}
            None => {
                policies.insert(field_name.to_string(), value);
            }
        }
    }
    Ok(())
}

fn parse_required_f64(
    headers: &StringRecord,
    row: &StringRecord,
    name: &str,
) -> Result<f64, CsvDatasetError> {
    let value = field(headers, row, name).ok_or_else(|| CsvDatasetError::MissingRequiredColumn {
        table: "kotlin_gurobi".to_string(),
        column: name.to_string(),
    })?;
    value.parse::<f64>().map_err(|_| invalid_value(
        "kotlin_gurobi",
        0,
        name,
        &value,
        "expected number",
    ))
}

fn parse_optional_f64(
    headers: &StringRecord,
    row: &StringRecord,
    name: &str,
) -> Result<Option<f64>, CsvDatasetError> {
    field(headers, row, name)
        .map(|value| {
            value.parse::<f64>().map_err(|_| invalid_value(
                "kotlin_gurobi",
                0,
                name,
                &value,
                "expected number",
            ))
        })
        .transpose()
}

fn parse_optional_u64(
    headers: &StringRecord,
    row: &StringRecord,
    name: &str,
) -> Result<Option<u64>, CsvDatasetError> {
    field(headers, row, name)
        .map(|value| {
            value.parse::<u64>().map_err(|_| invalid_value(
                "kotlin_gurobi",
                0,
                name,
                &value,
                "expected unsigned integer",
            ))
        })
        .transpose()
}

fn normalize_kotlin_shape_type(shape_type: &str) -> String {
    if is_kotlin_cylinder(shape_type) {
        "cylinder".to_string()
    } else {
        "cuboid".to_string()
    }
}

fn is_kotlin_cylinder(shape_type: &str) -> bool {
    normalize_token(shape_type).contains("cylinder")
}

