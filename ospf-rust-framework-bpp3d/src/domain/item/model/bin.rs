// ============================================================================
// Bin - 箱型 / Bin type
// ============================================================================

/// 箱型 / Bin type
#[derive(Debug, Clone)]
pub struct BinType<V, U: UnitTrait> {
    /// 宽度 / Width
    pub width: Quantity<V, U>,
    /// 高度 / Height
    pub height: Quantity<V, U>,
    /// 深度 / Depth
    pub depth: Quantity<V, U>,
    /// 容量 / Capacity
    pub capacity: Quantity<V, U>,
    /// 类型编码 / Type code
    pub type_code: String,
    /// 是否主箱 / Whether this is the main bin
    pub is_main: bool,
}

/// 箱 / Bin
#[derive(Debug, Clone)]
pub struct Bin<V, U: UnitTrait> {
    /// 箱型 / Bin type
    pub bin_type: BinType<V, U>,
    /// 批号 / Batch number
    pub batch_no: Option<String>,
}

