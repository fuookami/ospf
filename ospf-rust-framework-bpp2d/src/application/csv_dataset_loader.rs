//! CSV 数据集加载器 / CSV dataset loader
//!
//! 支持两种 schema：
//! 1. `grouped-layer`：一行一个物品，附带所属层与板材组。
//! 2. `material-width-amount`：一行一个物品，直接给出板材宽高与件数。
//! 二维专属列为 `height_meter` / `radius_meter` / `shape_type` / `allow_rotate` 及板材尺寸列。
//!
//! 圆形件可由 `radius_meter` 给半径，或由 `width` 作直径推导。
//! schema 门禁（重复列、未知列、缺必填列、非法枚举、非正数、行内列数不匹配、同物品几何不一致）
//! 全部返回 `Result`，不 panic。
//!
//! Two schemas are supported:
//! 1. `grouped-layer`: one item per row carrying its layer and sheet group.
//! 2. `material-width-amount`: one item per row with explicit sheet width, height and amount.
//! 2D-specific columns are `height_meter` / `radius_meter` / `shape_type` / `allow_rotate` plus the sheet size
//! columns. A circle may give `radius_meter`, or let `width` act as its diameter.
//!
//! Schema gates (duplicate columns, unknown columns, missing required columns, illegal enums, non-positive
//! numbers, mismatched field counts, inconsistent item geometry) all return `Result` and never panic.

use std::collections::BTreeMap;

use ospf_rust_base::error::Ret;

use crate::infrastructure::validation_error;

// ============================================================================
// CsvShapeType - CSV 形状类型 / CSV shape type
// ============================================================================

/// CSV 形状类型 / CSV shape type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvShapeType {
    /// 矩形 / Rectangle
    Rectangle,
    /// 圆形 / Circle
    Circle,
}

impl CsvShapeType {
    /// 由文本解析 / Parse from text
    ///
    /// 接受 `rectangle` / `rect` / `矩形` 与 `circle` / `圆形`，大小写不敏感。
    /// Accepts `rectangle` / `rect` / `矩形` and `circle` / `圆形`, case-insensitively.
    pub fn parse(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "rectangle" | "rect" | "矩形" => Some(Self::Rectangle),
            "circle" | "圆形" => Some(Self::Circle),
            _ => None,
        }
    }
}

// ============================================================================
// CsvDatasetSchema - 数据集 schema / Dataset schema
// ============================================================================

/// CSV 数据集 schema / CSV dataset schema
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvDatasetSchema {
    /// 分组层 schema / Grouped layer schema
    GroupedLayer,
    /// 物料宽高件数 schema / Material width and amount schema
    MaterialWidthAmount,
}

impl CsvDatasetSchema {
    /// 必填列 / Required columns
    pub fn required_columns(&self) -> &'static [&'static str] {
        match self {
            Self::GroupedLayer => &["item_id", "width_meter", "amount"],
            Self::MaterialWidthAmount => &[
                "item_id",
                "width_meter",
                "amount",
                "sheet_width_meter",
                "sheet_height_meter",
            ],
        }
    }

    /// 可选的二维专属列 / Optional 2D-specific columns
    pub fn optional_columns(&self) -> &'static [&'static str] {
        &[
            "height_meter",
            "radius_meter",
            "shape_type",
            "allow_rotate",
            "sheet_width_meter",
            "sheet_height_meter",
        ]
    }
}

// ============================================================================
// CsvItemRecord - 物品行 / Item row
// ============================================================================

/// 解析后的物品行 / Parsed item row
#[derive(Debug, Clone, PartialEq)]
pub struct CsvItemRecord {
    /// 物品编号 / Item identifier
    pub item_id: String,
    /// 宽度（矩形宽或圆直径）/ Width (rectangle width or circle diameter)
    pub width: f64,
    /// 高度（矩形高）/ Height (rectangle height)
    pub height: Option<f64>,
    /// 半径（圆形件）/ Radius (circle items)
    pub radius: Option<f64>,
    /// 件数 / Amount
    pub amount: u64,
    /// 形状类型 / Shape type
    pub shape_type: CsvShapeType,
    /// 是否允许旋转 / Whether rotation is allowed
    pub allow_rotate: bool,
    /// 板材宽度（material-width-amount schema）/ Sheet width (material-width-amount schema)
    pub sheet_width: Option<f64>,
    /// 板材高度（material-width-amount schema）/ Sheet height (material-width-amount schema)
    pub sheet_height: Option<f64>,
}

// ============================================================================
// CsvDatasetLoader - CSV 数据集加载器 / CSV dataset loader
// ============================================================================

/// CSV 数据集加载器 / CSV dataset loader
#[derive(Debug, Clone, Default)]
pub struct CsvDatasetLoader;

impl CsvDatasetLoader {
    /// 创建加载器 / Create the loader
    pub fn new() -> Self {
        Self
    }

    /// 从文本加载数据集 / Load a dataset from text
    ///
    /// 逐行解析，任何 schema 违规都返回 `Err` 并带上行号，便于定位。
    /// Rows are parsed one by one; any schema violation returns `Err` with the row number for localisation.
    pub fn load_str(&self, text: &str, schema: CsvDatasetSchema) -> Ret<Vec<CsvItemRecord>> {
        let mut lines = text.lines().filter(|line| !line.trim().is_empty());
        let Some(header_line) = lines.next() else {
            return Err(validation_error(
                "CSV 为空，缺少表头 / The CSV is empty: the header row is missing",
            ));
        };
        let header = Self::parse_header(header_line, schema)?;
        let mut records = Vec::new();
        let mut seen: BTreeMap<String, CsvItemRecord> = BTreeMap::new();

        for (offset, line) in lines.enumerate() {
            let row_number = offset + 2;
            let fields: Vec<&str> = line.split(',').map(|field| field.trim()).collect();
            if fields.len() != header.len() {
                return Err(validation_error(format!(
                    "第 {row_number} 行有 {} 列，表头为 {} 列 / row {row_number} has {} fields but the header has {}",
                    fields.len(),
                    header.len(),
                    fields.len(),
                    header.len()
                )));
            }
            let record = Self::parse_row(&header, &fields, schema, row_number)?;
            // 同一物品的几何必须一致，否则数据集自相矛盾。
            // The geometry of one item must be consistent, otherwise the dataset contradicts itself.
            if let Some(previous) = seen.get(&record.item_id) {
                if previous.shape_type != record.shape_type
                    || (previous.width - record.width).abs() > 1e-9
                    || previous.height.map(|value| (value - record.height.unwrap_or(value)).abs() > 1e-9)
                        == Some(true)
                {
                    return Err(validation_error(format!(
                        "第 {row_number} 行的物品 {} 与之前的几何不一致 / item {} on row {row_number} has inconsistent geometry",
                        record.item_id, record.item_id
                    )));
                }
            } else {
                seen.insert(record.item_id.clone(), record.clone());
            }
            records.push(record);
        }
        if records.is_empty() {
            return Err(validation_error(
                "CSV 没有数据行 / The CSV has no data row",
            ));
        }
        Ok(records)
    }

    /// 解析表头：校验必填列、重复列与未知列 / Parse the header: validate required, duplicate and unknown columns
    fn parse_header(line: &str, schema: CsvDatasetSchema) -> Ret<Vec<String>> {
        let names: Vec<String> = line
            .split(',')
            .map(|name| name.trim().to_ascii_lowercase())
            .collect();
        if names.is_empty() {
            return Err(validation_error("表头为空 / The header is empty"));
        }
        // 重复列：出现两次即视为 schema 违规。
        // Duplicate columns: appearing twice is a schema violation.
        let mut seen = std::collections::HashSet::new();
        for name in &names {
            if !seen.insert(name.clone()) {
                return Err(validation_error(format!(
                    "表头存在重复列 {name} / the header repeats column {name}"
                )));
            }
        }
        // 未知列：既不是必填也不是可选列。
        // Unknown columns: neither required nor optional.
        let required = schema.required_columns();
        let optional = schema.optional_columns();
        for name in &names {
            if !required.contains(&name.as_str()) && !optional.contains(&name.as_str()) {
                return Err(validation_error(format!(
                    "表头存在未知列 {name} / the header has unknown column {name}"
                )));
            }
        }
        // 缺必填列。
        // Missing required columns.
        for name in required {
            if !names.iter().any(|existing| existing == name) {
                return Err(validation_error(format!(
                    "表头缺少必填列 {name} / the header is missing required column {name}"
                )));
            }
        }
        Ok(names)
    }

    /// 解析一行 / Parse one row
    fn parse_row(
        header: &[String],
        fields: &[&str],
        schema: CsvDatasetSchema,
        row_number: usize,
    ) -> Ret<CsvItemRecord> {
        let value_of = |name: &str| -> Option<&str> {
            header
                .iter()
                .position(|column| column == name)
                .and_then(|index| fields.get(index).copied())
        };
        let parse_number = |name: &str, required: bool| -> Ret<Option<f64>> {
            match value_of(name) {
                Some(raw) if !raw.is_empty() => raw.parse::<f64>().map(Some).map_err(|_| {
                    validation_error(format!(
                        "第 {row_number} 行的 {name} 不是合法数字 / {name} on row {row_number} is not a number"
                    ))
                }),
                _ if required => Err(validation_error(format!(
                    "第 {row_number} 行缺少 {name} / row {row_number} is missing {name}"
                ))),
                _ => Ok(None),
            }
        };

        let Some(item_id) = value_of("item_id") else {
            return Err(validation_error(format!(
                "第 {row_number} 行缺少 item_id / row {row_number} is missing item_id"
            )));
        };
        if item_id.is_empty() {
            return Err(validation_error(format!(
                "第 {row_number} 行的 item_id 为空 / item_id on row {row_number} is empty"
            )));
        }

        let width = parse_number("width_meter", true)?.unwrap_or(0.0);
        let height = parse_number("height_meter", false)?;
        let radius = parse_number("radius_meter", false)?;
        let amount_raw = parse_number("amount", true)?.unwrap_or(0.0);
        if amount_raw <= 0.0 || amount_raw.fract() != 0.0 {
            return Err(validation_error(format!(
                "第 {row_number} 行的 amount 必须为正整数 / amount on row {row_number} must be a positive integer"
            )));
        }
        let amount = amount_raw as u64;

        let shape_type = match value_of("shape_type") {
            Some(raw) if !raw.is_empty() => CsvShapeType::parse(raw).ok_or_else(|| {
                validation_error(format!(
                    "第 {row_number} 行的 shape_type 非法：{raw} / illegal shape_type {raw} on row {row_number}"
                ))
            })?,
            _ => {
                if radius.is_some() {
                    CsvShapeType::Circle
                } else {
                    CsvShapeType::Rectangle
                }
            }
        };

        let allow_rotate = match value_of("allow_rotate") {
            Some(raw) if !raw.is_empty() => match raw.to_ascii_lowercase().as_str() {
                "true" | "1" | "yes" | "y" => true,
                "false" | "0" | "no" | "n" => false,
                _ => {
                    return Err(validation_error(format!(
                        "第 {row_number} 行的 allow_rotate 非法：{raw} / illegal allow_rotate {raw} on row {row_number}"
                    )))
                }
            },
            _ => true,
        };

        // 非正数门禁：尺寸必须为正。
        // Non-positive gate: sizes must be positive.
        match shape_type {
            CsvShapeType::Rectangle => {
                if width <= 0.0 {
                    return Err(validation_error(format!(
                        "第 {row_number} 行的 width_meter 必须为正 / width_meter on row {row_number} must be positive"
                    )));
                }
                if let Some(height) = height
                    && height <= 0.0
                {
                    return Err(validation_error(format!(
                        "第 {row_number} 行的 height_meter 必须为正 / height_meter on row {row_number} must be positive"
                    )));
                }
            }
            CsvShapeType::Circle => {
                let diameter = radius.map(|radius| radius * 2.0).unwrap_or(width);
                if diameter <= 0.0 {
                    return Err(validation_error(format!(
                        "第 {row_number} 行的圆形尺寸必须为正 / the circle size on row {row_number} must be positive"
                    )));
                }
            }
        }

        let sheet_width = parse_number("sheet_width_meter", false)?;
        let sheet_height = parse_number("sheet_height_meter", false)?;
        if schema == CsvDatasetSchema::MaterialWidthAmount {
            if sheet_width.unwrap_or(0.0) <= 0.0 {
                return Err(validation_error(format!(
                    "第 {row_number} 行的 sheet_width_meter 必须为正 / sheet_width_meter on row {row_number} must be positive"
                )));
            }
            if sheet_height.unwrap_or(0.0) <= 0.0 {
                return Err(validation_error(format!(
                    "第 {row_number} 行的 sheet_height_meter 必须为正 / sheet_height_meter on row {row_number} must be positive"
                )));
            }
        }

        Ok(CsvItemRecord {
            item_id: item_id.to_string(),
            width,
            height,
            radius,
            amount,
            shape_type,
            allow_rotate,
            sheet_width,
            sheet_height,
        })
    }
}
