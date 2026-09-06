//! Dataset / Dataset
/// CSV 数据集 / CSV dataset
#[derive(Debug, Clone, Default)]
pub struct CsvDataset {
    /// 货物记录 / Item records
    pub items: Vec<CsvItemRecord>,
    /// 箱记录 / Bin records
    pub bins: Vec<CsvBinRecord>,
    /// 层记录 / Layer records
    pub layers: Vec<CsvLayerRecord>,
    /// 深度边界策略记录 / Depth boundary policy records
    pub depth_boundary_policy: Vec<CsvDepthBoundaryPolicyRecord>,
    /// 半径权重函数记录 / Radius weight function records
    pub radius_weight_functions: Vec<CsvRadiusWeightFunctionRecord>,
    /// 诊断信息 / Diagnostics
    pub diagnostics: Vec<String>,
}

impl CsvDataset {
    /// 构造 application request draft / Build application request draft
    pub fn to_request_draft(&self) -> Result<CsvApplicationRequestDraft, CsvDatasetError> {
        let mut item_ids = HashSet::new();
        for (index, item) in self.items.iter().enumerate() {
            if !item_ids.insert(item.item_id.clone()) {
                return Err(invalid_value(
                    ITEMS_TABLE,
                    index + 2,
                    "item_id",
                    &item.item_id,
                    "duplicated item id",
                ));
            }
            let shape_type = item.parsed_shape_type(index + 2)?;
            if shape_type == CsvShapeType::Cylinder && item.parsed_axis(index + 2)?.is_none() {
                return Err(invalid_value(
                    ITEMS_TABLE,
                    index + 2,
                    "axis",
                    "",
                    "cylinder item requires axis",
                ));
            }
            item.parsed_enabled_orientations(index + 2)?;
        }

        let mut bin_ids = HashSet::new();
        for (index, bin) in self.bins.iter().enumerate() {
            if !bin_ids.insert(bin.bin_id.clone()) {
                return Err(invalid_value(
                    BINS_TABLE,
                    index + 2,
                    "bin_id",
                    &bin.bin_id,
                    "duplicated bin id",
                ));
            }
        }

        for (index, layer) in self.layers.iter().enumerate() {
            if !bin_ids.contains(&layer.bin_id) {
                return Err(invalid_value(
                    LAYERS_TABLE,
                    index + 2,
                    "bin_id",
                    &layer.bin_id,
                    "layer references unknown bin",
                ));
            }
        }

        let depth_boundary_policy = CsvDepthBoundaryPolicy::from_records(&self.depth_boundary_policy)?;
        let mut function_keys = HashSet::new();
        for (index, function) in self.radius_weight_functions.iter().enumerate() {
            let key = function.key.trim();
            if key.is_empty() {
                return Err(invalid_value(
                    RADIUS_WEIGHT_FUNCTIONS_TABLE,
                    index + 2,
                    "key",
                    &function.key,
                    "radius weight function key must be non-empty",
                ));
            }
            if !function_keys.insert(key.to_string()) {
                return Err(invalid_value(
                    RADIUS_WEIGHT_FUNCTIONS_TABLE,
                    index + 2,
                    "key",
                    key,
                    "duplicated radius weight function key",
                ));
            }
            if !function.radius_squared_coefficient.is_finite() {
                return Err(invalid_value(
                    RADIUS_WEIGHT_FUNCTIONS_TABLE,
                    index + 2,
                    "radius_squared_coefficient",
                    &function.radius_squared_coefficient.to_string(),
                    "expected finite number",
                ));
            }
            if let Some(intercept) = function.intercept {
                if !intercept.is_finite() {
                    return Err(invalid_value(
                        RADIUS_WEIGHT_FUNCTIONS_TABLE,
                        index + 2,
                        "intercept",
                        &intercept.to_string(),
                        "expected finite number",
                    ));
                }
            }
            if let Some(objective_weight) = function.objective_weight {
                if !objective_weight.is_finite() {
                    return Err(invalid_value(
                        RADIUS_WEIGHT_FUNCTIONS_TABLE,
                        index + 2,
                        "objective_weight",
                        &objective_weight.to_string(),
                        "expected finite number",
                    ));
                }
            }
        }
        Ok(CsvApplicationRequestDraft {
            item_count: self.items.len(),
            bin_count: self.bins.len(),
            layer_count: self.layers.len(),
            depth_boundary_policy,
            diagnostics: self.diagnostics.clone(),
        })
    }

    /// 物化为 typed application request / Materialize into typed application request
    pub fn materialize(&self) -> Result<CsvMaterializedApplicationRequest, CsvDatasetError> {
        CsvApplicationMaterializer::materialize(self)
    }
}
