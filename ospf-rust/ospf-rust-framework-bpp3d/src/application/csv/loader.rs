/// CSV 数据集加载器 / CSV dataset loader
#[derive(Debug, Clone, Default)]
pub struct CsvDatasetLoader;

impl CsvDatasetLoader {
    /// 从多表文本加载 / Load from multi-table text
    pub fn load_str(input: &str) -> Result<CsvDataset, CsvDatasetError> {
        let tables = split_tables(input)?;
        let items = read_table::<CsvItemRecord>(
            &tables,
            ITEMS_TABLE,
            ITEM_REQUIRED_COLUMNS,
            ITEM_OPTIONAL_COLUMNS,
            true,
        )?;
        let bins = read_table::<CsvBinRecord>(
            &tables,
            BINS_TABLE,
            BIN_REQUIRED_COLUMNS,
            BIN_OPTIONAL_COLUMNS,
            true,
        )?;
        let layers = read_table::<CsvLayerRecord>(
            &tables,
            LAYERS_TABLE,
            LAYER_REQUIRED_COLUMNS,
            LAYER_OPTIONAL_COLUMNS,
            false,
        )?;
        let depth_boundary_policy = read_table::<CsvDepthBoundaryPolicyRecord>(
            &tables,
            DEPTH_BOUNDARY_POLICY_TABLE,
            DEPTH_POLICY_REQUIRED_COLUMNS,
            DEPTH_POLICY_OPTIONAL_COLUMNS,
            false,
        )?;
        let radius_weight_functions = read_table::<CsvRadiusWeightFunctionRecord>(
            &tables,
            RADIUS_WEIGHT_FUNCTIONS_TABLE,
            RADIUS_WEIGHT_FUNCTION_REQUIRED_COLUMNS,
            RADIUS_WEIGHT_FUNCTION_OPTIONAL_COLUMNS,
            false,
        )?;
        let dataset = CsvDataset {
            items,
            bins,
            layers,
            depth_boundary_policy,
            radius_weight_functions,
            diagnostics: vec![format!("loaded_tables={}", tables.len())],
        };
        dataset.to_request_draft()?;
        Ok(dataset)
    }

    /// 从 Rust 或 Kotlin Gurobi CSV 文本加载 / Load from Rust or Kotlin Gurobi CSV text
    pub fn load_any_str(input: &str) -> Result<CsvDataset, CsvDatasetError> {
        if looks_like_kotlin_gurobi_csv(input) {
            let adapted = KotlinGurobiCsvAdapter::adapt(input)?;
            return Self::load_str(&adapted);
        }
        match Self::load_str(input) {
            Ok(dataset) => Ok(dataset),
            Err(rust_error) => {
                let adapted = KotlinGurobiCsvAdapter::adapt(input)
                    .map_err(|_| rust_error.clone())?;
                Self::load_str(&adapted).map_err(|_| rust_error)
            }
        }
    }
}

fn looks_like_kotlin_gurobi_csv(input: &str) -> bool {
    let Some(first_line) = input
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
    else {
        return false;
    };
    first_line.contains("group_index,")
        || first_line.contains("layer_index,")
        || first_line.contains("material,width,amount")
}

