/// CSV 形状类型 / CSV shape type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CsvShapeType {
    /// 长方体 / Cuboid
    Cuboid,
    /// 圆柱 / Cylinder
    Cylinder,
}

impl CsvShapeType {
    fn parse(table: &str, row: usize, field: &str, value: &str) -> Result<Self, CsvDatasetError> {
        match normalize_token(value).as_str() {
            "cuboid" | "box" | "rectangle" => Ok(Self::Cuboid),
            "cylinder" | "verticalcylinder" | "horizontalcylinder" => Ok(Self::Cylinder),
            _ => Err(invalid_value(
                table,
                row,
                field,
                value,
                "expected cuboid or cylinder",
            )),
        }
    }
}

/// CSV item 记录 / CSV item record
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CsvItemRecord {
    /// 货物 ID / Item id
    pub item_id: String,
    /// 名称 / Name
    pub name: String,
    /// 形状类型 / Shape type
    pub shape_type: String,
    /// 宽度 / Width
    pub width: f64,
    /// 高度 / Height
    pub height: f64,
    /// 深度 / Depth
    pub depth: f64,
    /// 重量 / Weight
    pub weight: f64,
    /// 数量 / Amount
    pub amount: u64,
    /// 包装编码 / Package code
    #[serde(default)]
    pub package_code: Option<String>,
    /// 物料编号 / Material number
    #[serde(default)]
    pub material_no: Option<String>,
    /// 物料名称 / Material name
    #[serde(default)]
    pub material_name: Option<String>,
    /// 物料重量 / Material weight
    #[serde(default)]
    pub material_weight: Option<f64>,
    /// 半径 / Radius
    #[serde(default)]
    pub radius: Option<f64>,
    /// 半径下界 / Radius lower bound
    #[serde(default)]
    pub radius_min: Option<f64>,
    /// 半径上界 / Radius upper bound
    #[serde(default)]
    pub radius_max: Option<f64>,
    /// 半径步长 / Radius step
    #[serde(default)]
    pub radius_step: Option<f64>,
    /// 半径权重函数键 / Radius weight function key
    #[serde(default)]
    pub radius_weight_function_key: Option<String>,
    /// 圆柱轴 / Cylinder axis
    #[serde(default)]
    pub axis: Option<String>,
    /// 允许朝向 / Enabled orientations
    #[serde(default)]
    pub enabled_orientations: Option<String>,
    /// 模式编码 / Pattern code
    #[serde(default)]
    pub pattern_code: Option<String>,
    /// 是否允许混装 / Whether mixed loading is allowed
    #[serde(default)]
    pub allow_mixed_loading: Option<bool>,
    /// 最大堆叠层数 / Maximum stacking layers
    #[serde(default)]
    pub max_stack_layers: Option<u64>,
    /// 包装标签 / Package tags
    #[serde(default)]
    pub package_tags: Option<String>,
}

impl CsvItemRecord {
    /// 解析形状类型 / Parse shape type
    pub fn parsed_shape_type(&self, row: usize) -> Result<CsvShapeType, CsvDatasetError> {
        CsvShapeType::parse(ITEMS_TABLE, row, "shape_type", &self.shape_type)
    }

    /// 解析圆柱轴 / Parse cylinder axis
    pub fn parsed_axis(&self, row: usize) -> Result<Option<Axis3>, CsvDatasetError> {
        self.axis
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|value| parse_axis(ITEMS_TABLE, row, "axis", value))
            .transpose()
    }

    /// 解析允许朝向 / Parse enabled orientations
    pub fn parsed_enabled_orientations(&self, row: usize) -> Result<Vec<Orientation>, CsvDatasetError> {
        parse_orientation_set(
            ITEMS_TABLE,
            row,
            "enabled_orientations",
            self.enabled_orientations.as_deref(),
            false,
        )
    }
}

/// CSV bin 记录 / CSV bin record
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CsvBinRecord {
    /// 箱 ID / Bin id
    pub bin_id: String,
    /// 类型编码 / Type code
    pub type_code: String,
    /// 宽度 / Width
    pub width: f64,
    /// 高度 / Height
    pub height: f64,
    /// 深度 / Depth
    pub depth: f64,
    /// 容量 / Capacity
    pub capacity: f64,
    /// 是否主箱 / Whether main bin
    #[serde(default)]
    pub is_main: Option<bool>,
    /// 批号 / Batch number
    #[serde(default)]
    pub batch_no: Option<String>,
}

/// CSV layer 记录 / CSV layer record
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CsvLayerRecord {
    /// 层 ID / Layer id
    pub layer_id: String,
    /// 箱 ID / Bin id
    pub bin_id: String,
    /// 深度 / Depth
    pub depth: f64,
    /// 迭代编号 / Iteration index
    #[serde(default)]
    pub iteration: Option<i64>,
    /// 来源 / Source
    #[serde(default)]
    pub from: Option<String>,
    /// 深度坐标 / Depth position
    #[serde(default)]
    pub z: Option<f64>,
}

/// CSV depth boundary policy 记录 / CSV depth boundary policy record
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CsvDepthBoundaryPolicyRecord {
    /// 策略字段 / Policy field
    pub field: String,
    /// 策略值列表 / Policy value list
    pub values: String,
}

/// CSV 半径权重函数记录 / CSV radius weight function record
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CsvRadiusWeightFunctionRecord {
    /// 函数键 / Function key
    pub key: String,
    /// 半径平方系数 / Radius-squared coefficient
    pub radius_squared_coefficient: f64,
    /// 截距 / Intercept
    #[serde(default)]
    pub intercept: Option<f64>,
    /// 目标权重 / Objective weight
    #[serde(default)]
    pub objective_weight: Option<f64>,
}

