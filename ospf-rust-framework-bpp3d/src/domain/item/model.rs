//! 货物领域模型 / Item domain models

/// 货物占位 / Item placeholder
#[derive(Debug, Clone, Default)]
pub struct Item;

/// 实际货物占位 / Actual item placeholder
#[derive(Debug, Clone, Default)]
pub struct ActualItem;

/// 包装占位 / Package placeholder
#[derive(Debug, Clone, Default)]
pub struct Package;

/// 包装形状占位 / Package shape placeholder
#[derive(Debug, Clone, Default)]
pub struct PackageShape;

/// 物料占位 / Material placeholder
#[derive(Debug, Clone, Default)]
pub struct Material;

/// 箱型占位 / Bin placeholder
#[derive(Debug, Clone, Default)]
pub struct Bin;

/// 箱层占位 / Bin layer placeholder
#[derive(Debug, Clone, Default)]
pub struct BinLayer;

/// 连续半径模型组件占位 / Continuous radius model component placeholder
#[derive(Debug, Clone, Default)]
pub struct ContinuousRadiusModelComponent;
