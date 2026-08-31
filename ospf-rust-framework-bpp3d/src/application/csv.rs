//! BPP3D CSV 数据集加载 / BPP3D CSV dataset loading
//!
//! 该模块只处理 application 协议边界，不注册 MetaModel。
//! This module only handles the application protocol boundary and does not register MetaModel.

use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};

use csv::{ReaderBuilder, StringRecord, Trim};
use ospf_rust_math::geometry::Axis3;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;

use crate::domain::item::{
    ActualItem, BinLayer, BinType, PackageAttribute, PackageShapeSpec, PatternedItemKey,
};
use crate::infrastructure::orientation::Orientation;

const ITEMS_TABLE: &str = "items";
const BINS_TABLE: &str = "bins";
const LAYERS_TABLE: &str = "layers";
const DEPTH_BOUNDARY_POLICY_TABLE: &str = "depth_boundary_policy";

const ITEM_REQUIRED_COLUMNS: &[&str] = &[
    "item_id",
    "name",
    "shape_type",
    "width",
    "height",
    "depth",
    "weight",
    "amount",
];

const ITEM_OPTIONAL_COLUMNS: &[&str] = &[
    "package_code",
    "material_no",
    "material_name",
    "material_weight",
    "radius",
    "radius_min",
    "radius_max",
    "radius_step",
    "axis",
    "enabled_orientations",
    "pattern_code",
    "allow_mixed_loading",
    "max_stack_layers",
    "package_tags",
];

const BIN_REQUIRED_COLUMNS: &[&str] = &[
    "bin_id",
    "type_code",
    "width",
    "height",
    "depth",
    "capacity",
];

const BIN_OPTIONAL_COLUMNS: &[&str] = &[
    "is_main",
    "batch_no",
];

const LAYER_REQUIRED_COLUMNS: &[&str] = &[
    "layer_id",
    "bin_id",
    "depth",
];

const LAYER_OPTIONAL_COLUMNS: &[&str] = &[
    "iteration",
    "from",
    "z",
];

const DEPTH_POLICY_REQUIRED_COLUMNS: &[&str] = &[
    "field",
    "values",
];

const DEPTH_POLICY_OPTIONAL_COLUMNS: &[&str] = &[];

/// CSV 数据集错误 / CSV dataset error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsvDatasetError {
    /// 缺少表 / Missing table
    MissingTable {
        /// 表名 / Table name
        table: String,
    },
    /// 未知表 / Unknown table
    UnknownTable {
        /// 表名 / Table name
        table: String,
    },
    /// 缺少必需列 / Missing required column
    MissingRequiredColumn {
        /// 表名 / Table name
        table: String,
        /// 列名 / Column name
        column: String,
    },
    /// 未知列 / Unknown column
    UnknownColumn {
        /// 表名 / Table name
        table: String,
        /// 列名 / Column name
        column: String,
    },
    /// 重复列 / Duplicated column
    DuplicatedColumn {
        /// 表名 / Table name
        table: String,
        /// 列名 / Column name
        column: String,
    },
    /// CSV 解析错误 / CSV parse error
    Parse {
        /// 表名 / Table name
        table: String,
        /// 信息 / Message
        message: String,
    },
    /// 字段值非法 / Invalid field value
    InvalidValue {
        /// 表名 / Table name
        table: String,
        /// 行号 / Row number
        row: usize,
        /// 字段名 / Field name
        field: String,
        /// 字段值 / Field value
        value: String,
        /// 原因 / Reason
        reason: String,
    },
}

impl Display for CsvDatasetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingTable { table } => {
                write!(f, "CSV dataset missing table `{}`. / CSV 数据集缺少表 `{}`。", table, table)
            }
            Self::UnknownTable { table } => {
                write!(f, "CSV dataset has unknown table `{}`. / CSV 数据集包含未知表 `{}`。", table, table)
            }
            Self::MissingRequiredColumn { table, column } => {
                write!(f, "CSV table `{}` missing required column `{}`. / CSV 表 `{}` 缺少必需列 `{}`。", table, column, table, column)
            }
            Self::UnknownColumn { table, column } => {
                write!(f, "CSV table `{}` has unknown column `{}`. / CSV 表 `{}` 包含未知列 `{}`。", table, column, table, column)
            }
            Self::DuplicatedColumn { table, column } => {
                write!(f, "CSV table `{}` has duplicated column `{}`. / CSV 表 `{}` 包含重复列 `{}`。", table, column, table, column)
            }
            Self::Parse { table, message } => {
                write!(f, "CSV table `{}` parse failed: {}. / CSV 表 `{}` 解析失败：{}。", table, message, table, message)
            }
            Self::InvalidValue { table, row, field, value, reason } => {
                write!(
                    f,
                    "CSV table `{}` row {} field `{}` has invalid value `{}`: {}. / CSV 表 `{}` 第 {} 行字段 `{}` 的值 `{}` 非法：{}。",
                    table, row, field, value, reason, table, row, field, value, reason
                )
            }
        }
    }
}

impl std::error::Error for CsvDatasetError {}

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

/// CSV application request 草稿 / CSV application request draft
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvApplicationRequestDraft {
    /// 货物数量 / Item count
    pub item_count: usize,
    /// 箱数量 / Bin count
    pub bin_count: usize,
    /// 层数量 / Layer count
    pub layer_count: usize,
    /// 深度边界策略 / Depth boundary policy
    pub depth_boundary_policy: Option<CsvDepthBoundaryPolicy>,
    /// 诊断信息 / Diagnostics
    pub diagnostics: Vec<String>,
}

/// CSV 物化 application request / CSV materialized application request
#[derive(Debug, Clone)]
pub struct CsvMaterializedApplicationRequest {
    /// 货物列表 / Items
    pub items: Vec<ActualItem<f64, Meter>>,
    /// 货物数量 / Item amounts
    pub item_amounts: Vec<(String, u64)>,
    /// 箱型列表 / Bin types
    pub bins: Vec<BinType<f64, Meter>>,
    /// 初始层 / Initial layers
    pub initial_layers: Vec<BinLayer<f64, Meter>>,
    /// 深度边界策略 / Depth boundary policy
    pub depth_boundary_policy: Option<CsvDepthBoundaryPolicy>,
    /// 模式货物键 / Patterned item keys
    pub patterned_items: Vec<(String, PatternedItemKey)>,
    /// 包装属性 / Package attributes
    pub package_attributes: Vec<(String, PackageAttribute)>,
    /// 诊断信息 / Diagnostics
    pub diagnostics: Vec<String>,
}

impl CsvMaterializedApplicationRequest {
    /// 校验物化业务规则 / Validate materialized business rules
    pub fn validate_business_rules(&self) -> Vec<String> {
        let mut diagnostics = Vec::new();
        let item_ids = self
            .items
            .iter()
            .map(|item| item.id.clone())
            .collect::<std::collections::HashSet<_>>();
        let mut pattern_counts = std::collections::HashMap::<String, usize>::new();
        for (item_id, pattern) in &self.patterned_items {
            if item_ids.contains(item_id) {
                diagnostics.push(format!(
                    "patterned item '{}' mapped to pattern '{}'",
                    item_id,
                    pattern.pattern_code,
                ));
                *pattern_counts.entry(pattern.pattern_code.clone()).or_default() += 1;
            } else {
                diagnostics.push(format!(
                    "patterned item '{}' references unknown item",
                    item_id,
                ));
            }
        }
        let mut mixed_loading_disabled = 0usize;
        let mut max_stack_layers = Vec::new();
        let mut tags = std::collections::BTreeSet::new();
        for (item_id, attribute) in &self.package_attributes {
            if !item_ids.contains(item_id) {
                diagnostics.push(format!(
                    "package attribute '{}' references unknown item",
                    item_id,
                ));
            }
            if attribute.allow_mixed_loading == Some(false) {
                mixed_loading_disabled += 1;
            }
            if let Some(layer_count) = attribute.max_stack_layers {
                max_stack_layers.push(layer_count);
            }
            tags.extend(attribute.tags.iter().cloned());
            diagnostics.extend(
                attribute
                    .validate()
                    .into_iter()
                    .map(|diagnostic| format!("package attribute '{}': {}", item_id, diagnostic)),
            );
        }
        if !pattern_counts.is_empty() {
            let mut summary = pattern_counts
                .into_iter()
                .collect::<Vec<_>>();
            summary.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
            diagnostics.push(format!(
                "patterned item groups: {}",
                summary
                    .iter()
                    .map(|(pattern, count)| format!("{}={}", pattern, count))
                    .collect::<Vec<_>>()
                    .join(","),
            ));
        }
        if !self.package_attributes.is_empty() {
            diagnostics.push(format!(
                "package attribute summary: total={}, mixed_loading_disabled={}, min_stack_layers={}, tags={}",
                self.package_attributes.len(),
                mixed_loading_disabled,
                max_stack_layers.into_iter().min().unwrap_or(0),
                tags.into_iter().collect::<Vec<_>>().join("|"),
            ));
        }
        if self
            .initial_layers
            .iter()
            .all(|layer| layer.demand_coverage.is_empty())
            && !self.items.is_empty()
        {
            diagnostics.push(
                "initial layers do not carry explicit demand coverage; application defaults will be applied"
                    .to_string(),
            );
        }
        diagnostics
    }
}

/// CSV application 物化器 / CSV application materializer
#[derive(Debug, Clone, Default)]
pub struct CsvApplicationMaterializer;

impl CsvApplicationMaterializer {
    /// 物化 CSV 数据集 / Materialize CSV dataset
    pub fn materialize(dataset: &CsvDataset) -> Result<CsvMaterializedApplicationRequest, CsvDatasetError> {
        let draft = dataset.to_request_draft()?;
        let mut diagnostics = draft.diagnostics.clone();
        let items = dataset.items
            .iter()
            .enumerate()
            .map(|(index, record)| materialize_item(record, index + 2, &mut diagnostics))
            .collect::<Result<Vec<_>, _>>()?;
        let item_amounts = dataset.items
            .iter()
            .map(|record| (record.item_id.clone(), record.amount))
            .collect();
        let bins = dataset.bins
            .iter()
            .map(materialize_bin)
            .collect::<Vec<_>>();
        let bin_by_id = dataset.bins
            .iter()
            .cloned()
            .zip(bins.iter().cloned())
            .map(|(record, bin)| (record.bin_id, bin))
            .collect::<HashMap<_, _>>();
        let initial_layers = dataset.layers
            .iter()
            .enumerate()
            .map(|(index, record)| materialize_layer(record, index + 2, &bin_by_id))
            .collect::<Result<Vec<_>, _>>()?;
        let patterned_items = dataset.items
            .iter()
            .filter_map(|record| {
                record.pattern_code.as_ref().and_then(|pattern_code| {
                    let pattern_code = pattern_code.trim();
                    (!pattern_code.is_empty()).then(|| {
                        (
                            record.item_id.clone(),
                            PatternedItemKey {
                                pattern_code: pattern_code.to_string(),
                            },
                        )
                    })
                })
            })
            .collect::<Vec<_>>();
        let package_attributes = dataset.items
            .iter()
            .filter_map(|record| materialize_package_attribute(record))
            .collect::<Vec<_>>();

        Ok(CsvMaterializedApplicationRequest {
            items,
            item_amounts,
            bins,
            initial_layers,
            depth_boundary_policy: draft.depth_boundary_policy,
            patterned_items,
            package_attributes,
            diagnostics,
        })
    }
}

/// CSV depth boundary policy / CSV depth boundary policy
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvDepthBoundaryPolicy {
    /// 首层允许圆柱轴 / First layer allowed cylinder axes
    pub first_layer_allowed_cylinder_axes: Option<HashSet<Axis3>>,
    /// 末层允许圆柱轴 / Last layer allowed cylinder axes
    pub last_layer_allowed_cylinder_axes: Option<HashSet<Axis3>>,
    /// 首层允许长方体朝向 / First layer allowed cuboid orientations
    pub first_layer_allowed_cuboid_orientations: Option<HashSet<Orientation>>,
    /// 末层允许长方体朝向 / Last layer allowed cuboid orientations
    pub last_layer_allowed_cuboid_orientations: Option<HashSet<Orientation>>,
}

impl CsvDepthBoundaryPolicy {
    fn from_records(records: &[CsvDepthBoundaryPolicyRecord]) -> Result<Option<Self>, CsvDatasetError> {
        if records.is_empty() {
            return Ok(None);
        }
        let mut policy = Self {
            first_layer_allowed_cylinder_axes: None,
            last_layer_allowed_cylinder_axes: None,
            first_layer_allowed_cuboid_orientations: None,
            last_layer_allowed_cuboid_orientations: None,
        };
        for (index, record) in records.iter().enumerate() {
            let row = index + 2;
            match normalize_token(&record.field).as_str() {
                "firstlayerallowedcylinderaxes" => {
                    policy.first_layer_allowed_cylinder_axes = Some(parse_axis_set(
                        DEPTH_BOUNDARY_POLICY_TABLE,
                        row,
                        "values",
                        &record.values,
                    )?);
                }
                "lastlayerallowedcylinderaxes" => {
                    policy.last_layer_allowed_cylinder_axes = Some(parse_axis_set(
                        DEPTH_BOUNDARY_POLICY_TABLE,
                        row,
                        "values",
                        &record.values,
                    )?);
                }
                "firstlayerallowedcuboidorientations" => {
                    policy.first_layer_allowed_cuboid_orientations = Some(parse_orientation_set(
                        DEPTH_BOUNDARY_POLICY_TABLE,
                        row,
                        "values",
                        Some(&record.values),
                        true,
                    )?.into_iter().collect());
                }
                "lastlayerallowedcuboidorientations" => {
                    policy.last_layer_allowed_cuboid_orientations = Some(parse_orientation_set(
                        DEPTH_BOUNDARY_POLICY_TABLE,
                        row,
                        "values",
                        Some(&record.values),
                        true,
                    )?.into_iter().collect());
                }
                _ => {
                    return Err(invalid_value(
                        DEPTH_BOUNDARY_POLICY_TABLE,
                        row,
                        "field",
                        &record.field,
                        "unknown depth boundary policy field",
                    ));
                }
            }
        }
        Ok(Some(policy))
    }
}

/// CSV schema guard / CSV schema guard
#[derive(Debug, Clone, Default)]
pub struct CsvSchemaGuard;

impl CsvSchemaGuard {
    /// 校验表头 / Validate header
    pub fn validate(
        table: &str,
        headers: &StringRecord,
        required: &[&str],
        optional: &[&str],
    ) -> Result<(), CsvDatasetError> {
        let mut seen = HashSet::new();
        let allowed: HashSet<&str> = required.iter().chain(optional.iter()).copied().collect();
        for header in headers {
            let column = header.trim();
            if !seen.insert(column.to_string()) {
                return Err(CsvDatasetError::DuplicatedColumn {
                    table: table.to_string(),
                    column: column.to_string(),
                });
            }
            if !allowed.contains(column) {
                return Err(CsvDatasetError::UnknownColumn {
                    table: table.to_string(),
                    column: column.to_string(),
                });
            }
        }
        for column in required {
            if !seen.contains(*column) {
                return Err(CsvDatasetError::MissingRequiredColumn {
                    table: table.to_string(),
                    column: (*column).to_string(),
                });
            }
        }
        Ok(())
    }
}

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
        let dataset = CsvDataset {
            items,
            bins,
            layers,
            depth_boundary_policy,
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
        let radius_min = parse_optional_f64(headers, &row, "radius_min")?;
        let radius_max = parse_optional_f64(headers, &row, "radius_max")?;
        let width = parse_optional_f64(headers, &row, "width_meter")?
            .unwrap_or_else(|| radius.or(radius_max).map(|value| value * 2.0).unwrap_or(1.0));
        let height = parse_optional_f64(headers, &row, "height_meter")?
            .unwrap_or_else(|| radius.or(radius_max).map(|value| value * 2.0).unwrap_or(1.0));
        let depth = parse_optional_f64(headers, &row, "depth_meter")?
            .unwrap_or(1.0);
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
            radius_step: parse_optional_f64(headers, &row, "radius_step")?
                .or_else(|| radius_min.zip(radius_max).map(|(min, max)| (max - min).max(0.0))),
            axis: axis.or_else(|| if is_kotlin_cylinder(&shape_type) { Some("Y".to_string()) } else { None }),
            enabled_orientations: None,
            pattern_code: Some(format!("G{}", field(headers, &row, "group_index").unwrap_or_else(|| "0".to_string()))),
            allow_mixed_loading: Some(true),
            max_stack_layers: Some(4),
            package_tags: Some("kotlin-grouped".to_string()),
        });
        layers.insert(layer_index);
    }
    build_adapted_dataset(items, layers)
}

fn adapt_material_width_amount(
    mut reader: csv::Reader<&[u8]>,
    headers: &StringRecord,
) -> Result<String, CsvDatasetError> {
    let mut items = Vec::new();
    let mut layers = HashSet::new();
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
            radius_min: parse_optional_f64(headers, &row, "radius_min")?,
            radius_max: parse_optional_f64(headers, &row, "radius_max")?,
            radius_step: parse_optional_f64(headers, &row, "radius_step")?,
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
    build_adapted_dataset(items, layers)
}

fn build_adapted_dataset(
    items: Vec<CsvItemRecord>,
    layers: HashSet<String>,
) -> Result<String, CsvDatasetError> {
    let mut output = String::new();
    output.push_str("# table:items\n");
    output.push_str("item_id,name,shape_type,width,height,depth,weight,amount,radius,radius_min,radius_max,radius_step,axis,pattern_code,allow_mixed_loading,max_stack_layers,package_tags\n");
    for item in items {
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
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
        id: record.item_id.clone(),
        name: record.name.clone(),
        package_code: record.package_code.clone(),
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
        },
    ))
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
        type_code: record.type_code.clone(),
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
        ITEMS_TABLE | BINS_TABLE | LAYERS_TABLE | DEPTH_BOUNDARY_POLICY_TABLE
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
