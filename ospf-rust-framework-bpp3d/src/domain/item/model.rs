//! 货物领域模型 / Item domain models
//!
//! 映射 Kotlin `bpp3d-domain-item-context` 的核心领域模型。
//! Maps core domain models from Kotlin `bpp3d-domain-item-context`.

use ospf_rust_math::algebra::Field;
use ospf_rust_math::geometry::Axis3;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::infrastructure::orientation::Orientation;
use crate::infrastructure::packing_shape::{PackingShape3, cuboid_packing_shape, cylinder_packing_shape};

// ============================================================================
// PackageShapeSpec - 包装形状规格 / Package shape specification
// ============================================================================

/// 包装形状规格 / Package shape specification
///
/// 区分长方体和轴感知圆柱，不沿用 Kotlin 的 `VerticalCylinder` 命名误导。
/// Distinguishes cuboids and axis-aware cylinders; does not follow Kotlin's
/// misleading `VerticalCylinder` naming.
#[derive(Debug, Clone)]
pub enum PackageShapeSpec<V, U: UnitTrait> {
    /// 长方体 / Cuboid
    Cuboid,
    /// 轴感知圆柱 / Axis-aware cylinder
    Cylinder {
        /// 对齐轴 / Alignment axis
        axis: Axis3,
        /// 半径 / Radius
        radius: Quantity<V, U>,
        /// 半径候选列表（离散半径）/ Radius candidates (discrete radius)
        radius_candidates: Option<Vec<Quantity<V, U>>>,
        /// 半径下界（连续半径）/ Radius lower bound (continuous radius)
        radius_lower_bound: Option<Quantity<V, U>>,
        /// 半径上界（连续半径）/ Radius upper bound (continuous radius)
        radius_upper_bound: Option<Quantity<V, U>>,
    },
}

// ============================================================================
// PackageShape - 包装形状 / Package shape
// ============================================================================

/// 包装形状 / Package shape
///
/// 包装的完整几何描述，包括尺寸、重量和形状规格。
/// Complete geometric description of a package, including dimensions, weight,
/// and shape specification.
#[derive(Debug, Clone)]
pub struct PackageShape<V, U: UnitTrait> {
    /// 宽度 / Width
    pub width: Quantity<V, U>,
    /// 高度 / Height
    pub height: Quantity<V, U>,
    /// 深度 / Depth
    pub depth: Quantity<V, U>,
    /// 重量 / Weight
    pub weight: Quantity<V, U>,
    /// 形状规格 / Shape specification
    pub spec: PackageShapeSpec<V, U>,
}

impl<V: Clone + Field + num_traits::FloatConst, U: CTUnit + Default + Clone> PackageShape<V, U> {
    /// 转换为 PackingShape3 / Convert to PackingShape3
    pub fn to_packing_shape(&self) -> PackingShape3<V, U> {
        match &self.spec {
            PackageShapeSpec::Cuboid => {
                cuboid_packing_shape(
                    self.width.clone(),
                    self.height.clone(),
                    self.depth.clone(),
                    self.weight.clone(),
                )
            }
            PackageShapeSpec::Cylinder { axis, radius, .. } => {
                cylinder_packing_shape(
                    radius.clone(),
                    self.height.clone(),
                    *axis,
                    self.weight.clone(),
                )
            }
        }
    }
}

// ============================================================================
// Material - 物料 / Material
// ============================================================================

/// 物料类型 / Material type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaterialType {
    /// 原材料 / Raw material
    RawMaterial,
    /// 半成品 / Semi-finished product
    SemiFinishedProduct,
    /// 成品 / Finished product
    FinishedProduct,
}

/// 物料标识 / Material key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MaterialKey {
    /// 物料编号 / Material number
    pub no: String,
    /// 物料类型 / Material type
    pub material_type: MaterialType,
}

/// 物料 / Material
#[derive(Debug, Clone)]
pub struct Material<V, U: UnitTrait> {
    /// 编号 / Number
    pub no: String,
    /// 类型 / Type
    pub material_type: MaterialType,
    /// 名称 / Name
    pub name: String,
    /// 仓库 / Warehouse
    pub warehouse: Option<String>,
    /// 重量 / Weight
    pub weight: Quantity<V, U>,
}

impl<V, U: UnitTrait> Material<V, U> {
    /// 获取物料标识 / Get material key
    pub fn key(&self) -> MaterialKey {
        MaterialKey {
            no: self.no.clone(),
            material_type: self.material_type,
        }
    }
}

// ============================================================================
// Package - 包装 / Package
// ============================================================================

/// 包装程序 / Packing program
#[derive(Debug, Clone)]
pub struct PackingProgram<V, U: UnitTrait> {
    /// 形状 / Shape
    pub shape: PackageShape<V, U>,
    /// 材料数量映射 / Material amounts map
    pub materials: Vec<(MaterialKey, u64)>,
}

/// 包装 / Package
#[derive(Debug, Clone)]
pub struct Package<V, U: UnitTrait> {
    /// 编码 / Code
    pub code: Option<String>,
    /// 形状 / Shape
    pub shape: PackageShape<V, U>,
    /// 子包装 / Sub-packages
    pub packages: Option<Vec<Package<V, U>>>,
    /// 材料数量 / Material amounts
    pub materials: Vec<(MaterialKey, u64)>,
    /// 数量 / Amount
    pub amount: u64,
}

// ============================================================================
// Item - 货物 / Item
// ============================================================================

/// 实际货物 / Actual item
///
/// 具有具体尺寸和属性的单个货物。
/// An individual item with concrete dimensions and attributes.
#[derive(Debug, Clone)]
pub struct ActualItem<V, U: UnitTrait> {
    /// 标识 / ID
    pub id: String,
    /// 名称 / Name
    pub name: String,
    /// 包装编码 / Package code
    pub package_code: Option<String>,
    /// 包装 / Package
    pub pack: Option<Package<V, U>>,
    /// 宽度 / Width
    pub width: Quantity<V, U>,
    /// 高度 / Height
    pub height: Quantity<V, U>,
    /// 深度 / Depth
    pub depth: Quantity<V, U>,
    /// 重量 / Weight
    pub weight: Quantity<V, U>,
    /// 允许朝向 / Enabled orientations
    pub enabled_orientations: Vec<Orientation>,
    /// 形状规格覆盖 / Shape specification override
    pub shape_spec_override: Option<PackageShapeSpec<V, U>>,
}

// ============================================================================
// PatternedItem / PackageAttribute - 迁移护栏 / Migration guardrails
// ============================================================================

/// 货物模式键 / Item pattern key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PatternedItemKey {
    /// 模式编码 / Pattern code
    pub pattern_code: String,
}

/// 模式货物 skeleton / Patterned item skeleton
#[derive(Debug, Clone)]
pub struct PatternedItem<V, U: UnitTrait> {
    /// 模式键 / Pattern key
    pub key: PatternedItemKey,
    /// 货物 / Item
    pub item: ActualItem<V, U>,
    /// 诊断信息 / Diagnostics
    pub diagnostics: Vec<String>,
}

impl<V, U> PatternedItem<V, U>
where
    U: UnitTrait,
{
    /// 创建 unsupported skeleton / Create unsupported skeleton
    pub fn unsupported(
        pattern_code: impl Into<String>,
        item: ActualItem<V, U>,
    ) -> Self {
        let pattern_code = pattern_code.into();
        Self {
            key: PatternedItemKey {
                pattern_code: pattern_code.clone(),
            },
            item,
            diagnostics: vec![format!(
                "patterned item '{}' is not implemented",
                pattern_code,
            )],
        }
    }

    /// 创建需求覆盖 / Create demand coverage
    pub fn demand_coverage(&self, coefficient: f64) -> Bpp3dLayerDemandCoverage {
        Bpp3dLayerDemandCoverage::new(
            Bpp3dDemandMode::Item,
            Bpp3dDemandKey::Item {
                id: self.item.id.clone(),
            },
            coefficient,
        )
    }
}

/// 包装属性 / Package attribute
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackageAttribute {
    /// 是否允许混装 / Whether mixed loading is allowed
    pub allow_mixed_loading: Option<bool>,
    /// 最大堆叠层数 / Maximum stacking layers
    pub max_stack_layers: Option<u64>,
    /// 业务标签 / Business tags
    pub tags: Vec<String>,
}

impl PackageAttribute {
    /// 验证业务规则 / Validate business rules
    pub fn validate(&self) -> Vec<String> {
        let mut diagnostics = Vec::new();
        if self.allow_mixed_loading == Some(false) {
            diagnostics.push("package attribute disallows mixed loading".to_string());
        }
        if self.max_stack_layers == Some(0) {
            diagnostics.push("package attribute max_stack_layers must be positive".to_string());
        }
        diagnostics.extend(self.diagnostics());
        diagnostics
    }

    /// 诊断信息 / Diagnostics
    pub fn diagnostics(&self) -> Vec<String> {
        vec![format!(
            "package attribute skeleton: mixed_loading={:?}, max_stack_layers={:?}, tags={}",
            self.allow_mixed_loading,
            self.max_stack_layers,
            self.tags.len(),
        )]
    }

    /// 装箱诊断信息 / Packing diagnostics
    pub fn packing_diagnostics(&self, layer_count: u64) -> Vec<String> {
        let mut diagnostics = self.diagnostics();
        if let Some(max_stack_layers) = self.max_stack_layers {
            if layer_count > max_stack_layers {
                diagnostics.push(format!(
                    "package attribute max_stack_layers exceeded: {} > {}",
                    layer_count,
                    max_stack_layers,
                ));
            }
        }
        if self.allow_mixed_loading == Some(false) {
            diagnostics.push("package attribute disallows mixed loading".to_string());
        }
        diagnostics
    }
}

impl<V: Clone + Field + num_traits::FloatConst, U: CTUnit + Default + Clone> ActualItem<V, U> {
    /// 获取包装形状 / Get package shape
    pub fn package_shape(&self) -> PackageShape<V, U> {
        if let Some(spec) = &self.shape_spec_override {
            PackageShape {
                width: self.width.clone(),
                height: self.height.clone(),
                depth: self.depth.clone(),
                weight: self.weight.clone(),
                spec: spec.clone(),
            }
        } else {
            // 默认为长方体
            PackageShape {
                width: self.width.clone(),
                height: self.height.clone(),
                depth: self.depth.clone(),
                weight: self.weight.clone(),
                spec: PackageShapeSpec::Cuboid,
            }
        }
    }

    /// 获取包装形状3D / Get 3D packing shape
    pub fn packing_shape(&self) -> PackingShape3<V, U> {
        self.package_shape().to_packing_shape()
    }
}

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

// ============================================================================
// BinLayer - 箱层 / Bin layer
// ============================================================================

/// 箱层 / Bin layer
///
/// 列生成算法中一个层候选的描述。
/// Description of a layer candidate in the column generation algorithm.
#[derive(Debug, Clone)]
pub struct BinLayer<V, U: UnitTrait> {
    /// 迭代编号 / Iteration index
    pub iteration: i64,
    /// 来源 / Source
    pub from: String,
    /// 箱型 / Bin type
    pub bin: Option<BinType<V, U>>,
    /// 深度 / Depth
    pub depth: Quantity<V, U>,
    /// 需求覆盖 / Demand coverage
    pub demand_coverage: Vec<Bpp3dLayerDemandCoverage>,
}

impl<V, U: UnitTrait> BinLayer<V, U> {
    /// 带需求覆盖创建副本 / Clone with demand coverage
    pub fn with_demand_coverage(mut self, coverage: Vec<Bpp3dLayerDemandCoverage>) -> Self {
        self.demand_coverage = coverage;
        self
    }

    /// 查询需求覆盖系数 / Query demand coverage coefficient
    pub fn demand_coverage_coefficient(
        &self,
        mode: Bpp3dDemandMode,
        key: &Bpp3dDemandKey,
    ) -> f64 {
        self.demand_coverage
            .iter()
            .find(|coverage| coverage.mode == mode && &coverage.key == key)
            .map(|coverage| coverage.coefficient)
            .unwrap_or(0.0)
    }
}

// ============================================================================
// DemandStatistics - 需求统计 / Demand statistics
// ============================================================================

/// 需求模式 / Demand mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bpp3dDemandMode {
    /// 货物需求 / Item demand
    Item,
    /// 物料需求 / Material demand
    Material,
    /// 货物数量需求 / Item amount demand
    ItemAmount,
    /// 货物重量需求 / Item weight demand
    ItemWeight,
    /// 货物物料数量需求 / Item material amount demand
    ItemMaterialAmount,
    /// 货物物料重量需求 / Item material weight demand
    ItemMaterialWeight,
}

/// 需求键 / Demand key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Bpp3dDemandKey {
    /// 货物键 / Item key
    Item { id: String },
    /// 物料键 / Material key
    Material { no: String },
}

/// 层需求覆盖 / Layer demand coverage
///
/// 描述一个层候选对某个需求条目的覆盖系数。
/// Describes the coverage coefficient of a layer candidate for a demand entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Bpp3dLayerDemandCoverage {
    /// 需求模式 / Demand mode
    pub mode: Bpp3dDemandMode,
    /// 需求键 / Demand key
    pub key: Bpp3dDemandKey,
    /// 覆盖系数 / Coverage coefficient
    pub coefficient: f64,
}

impl Bpp3dLayerDemandCoverage {
    /// 创建覆盖条目 / Create coverage entry
    pub fn new(mode: Bpp3dDemandMode, key: Bpp3dDemandKey, coefficient: f64) -> Self {
        Self {
            mode,
            key,
            coefficient,
        }
    }
}

/// 需求值 / Demand value
#[derive(Debug, Clone)]
pub enum Bpp3dDemandValue<V, U: UnitTrait> {
    /// 数量 / Amount
    Amount(u64),
    /// 重量 / Weight
    Weight(Quantity<V, U>),
}

/// 需求统计 / Demand statistics
#[derive(Debug, Clone)]
pub struct DemandStatistics<V, U: UnitTrait> {
    /// 统计条目 / Statistics entries
    pub entries: Vec<(Bpp3dDemandKey, Bpp3dDemandValue<V, U>)>,
}

// ============================================================================
// CylinderShapeContract - 圆柱形状契约 / Cylinder shape contract
// ============================================================================

/// 圆柱能力状态 / Cylinder capability status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CylinderCapabilityStatus {
    /// 仅长方体 / Cuboid only
    CuboidOnly,
    /// 仅竖直候选 / Vertical candidate only
    VerticalCandidateOnly,
    /// 轴感知候选 / Axis-aware candidate
    AxisAwareCandidate,
    /// 验证过的生成放置 / Verified generated placement
    VerifiedGeneratedPlacement,
    /// 竖直竖向支撑 / Upright vertical support only
    UprightVerticalSupportOnly,
    /// 已知坐标最终验证 / Known-coordinate final validation
    KnownCoordinateFinalValidation,
}

/// 圆柱形状契约 / Cylinder shape contract
///
/// 定义了 BPP3D 中圆柱形状在不同算法路径下的能力验证规则。
/// Defines capability verification rules for cylinder shapes across
/// different algorithm paths in BPP3D.
pub struct CylinderShapeContract;

impl CylinderShapeContract {
    /// 检查是否有圆柱形状 / Check if there are any cylinder shapes
    pub fn has_cylinder<V, U: UnitTrait>(items: &[ActualItem<V, U>]) -> bool {
        items.iter().any(|item| {
            matches!(&item.shape_spec_override, Some(PackageShapeSpec::Cylinder { .. }))
        })
    }

    /// 要求竖直圆柱轴 / Require vertical cylinder axis
    pub fn require_vertical_axis(axis: Axis3) -> Result<(), String> {
        if axis != Axis3::Y {
            Err(format!(
                "Vertical cylinder axis required, got {:?}. / 要求竖直圆柱轴，得到 {:?}。",
                axis, axis
            ))
        } else {
            Ok(())
        }
    }

    /// 要求轴感知圆柱候选 / Require axis-aware cylinder candidate
    pub fn require_axis_aware_candidate(axis: Axis3) -> Result<(), String> {
        match axis {
            Axis3::X | Axis3::Y | Axis3::Z => Ok(()),
        }
    }
}

// ============================================================================
// ContinuousRadiusModelComponent - 连续半径模型组件 / Continuous radius model component
// ============================================================================

/// 连续半径求解器原型 / Continuous radius solver prototype
#[derive(Debug, Clone)]
pub struct ContinuousCylinderRadiusSolverPrototype {
    /// 来源 / Source
    pub source: String,
    /// 对齐轴 / Alignment axis
    pub axis: Axis3,
    /// 变量名 / Variable name
    pub variable_name: String,
    /// 半径下界 / Radius lower bound
    pub radius_lower_bound: Option<f64>,
    /// 半径上界 / Radius upper bound
    pub radius_upper_bound: Option<f64>,
}

/// 连续半径模型组件 / Continuous radius model component
///
/// 管理连续半径圆柱在求解器中的变量注册和结果提取。
/// Manages variable registration and result extraction for continuous-radius
/// cylinders in the solver.
#[derive(Debug, Clone)]
pub struct ContinuousRadiusModelComponent {
    /// 原型列表 / Prototypes
    pub prototypes: Vec<ContinuousCylinderRadiusSolverPrototype>,
    /// 注册计划 / Registration plan
    pub registration_plan: ContinuousRadiusRegistrationPlan,
}

/// 连续半径注册计划 / Continuous radius registration plan
#[derive(Debug, Clone, Default)]
pub struct ContinuousRadiusRegistrationPlan {
    /// 变量名 / Variable names
    pub variable_names: Vec<String>,
    /// 注册的变量 / Registered variables
    pub registered_variables: Vec<String>,
    /// 阻塞的变量 / Blocked variables
    pub blocked_variables: Vec<String>,
}

impl ContinuousRadiusModelComponent {
    /// 诊断信息 / Diagnostic information
    pub fn info(&self) -> Vec<(&str, String)> {
        let mut info = Vec::new();
        info.push(("prototype_count", self.prototypes.len().to_string()));
        info.push(("registered_variables", self.registration_plan.registered_variables.join(", ")));
        info.push(("blocked_variables", self.registration_plan.blocked_variables.join(", ")));
        info
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::packing_shape::PackingShapeType;
    use ospf_rust_quantities::unit::derived::Meter;

    fn meters(v: f64) -> Quantity<f64, Meter> {
        Quantity::new_ct(v)
    }

    #[test]
    fn package_shape_spec_cuboid() {
        let spec = PackageShapeSpec::<f64, Meter>::Cuboid;
        assert!(matches!(spec, PackageShapeSpec::Cuboid));
    }

    #[test]
    fn package_shape_spec_cylinder_axis_aware() {
        let spec = PackageShapeSpec::Cylinder {
            axis: Axis3::Y,
            radius: meters(2.0),
            radius_candidates: None,
            radius_lower_bound: None,
            radius_upper_bound: None,
        };
        if let PackageShapeSpec::Cylinder { axis, radius, .. } = spec {
            assert_eq!(axis, Axis3::Y);
            assert_eq!(radius.value, 2.0);
        } else {
            panic!("Expected Cylinder spec");
        }
    }

    #[test]
    fn actual_item_cuboid_packing_shape() {
        let item = ActualItem {
            id: "item1".to_string(),
            name: "Test Item".to_string(),
            package_code: None,
            pack: None,
            width: meters(2.0),
            height: meters(3.0),
            depth: meters(4.0),
            weight: meters(1.0), // 重量单位不同，此处简化
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: None,
        };

        let shape = item.packing_shape();
        assert_eq!(shape.shape_type, PackingShapeType::Cuboid);
    }

    #[test]
    fn actual_item_cylinder_packing_shape() {
        let item = ActualItem {
            id: "cyl1".to_string(),
            name: "Cylinder Item".to_string(),
            package_code: None,
            pack: None,
            width: meters(4.0), // diameter
            height: meters(5.0),
            depth: meters(4.0), // diameter
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: Some(PackageShapeSpec::Cylinder {
                axis: Axis3::Y,
                radius: meters(2.0),
                radius_candidates: None,
                radius_lower_bound: None,
                radius_upper_bound: None,
            }),
        };

        let shape = item.packing_shape();
        assert_eq!(shape.shape_type, PackingShapeType::Cylinder);
        assert_eq!(shape.axis, Some(Axis3::Y));
    }

    #[test]
    fn patterned_item_and_package_attribute_report_diagnostics() {
        let item = ActualItem {
            id: "i0".to_string(),
            name: "Item".to_string(),
            package_code: None,
            pack: None,
            width: meters(1.0),
            height: meters(1.0),
            depth: meters(1.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: None,
        };
        let patterned = PatternedItem::unsupported("p0", item);
        let attribute = PackageAttribute {
            allow_mixed_loading: Some(false),
            max_stack_layers: Some(2),
            tags: vec!["fragile".to_string()],
        };

        assert_eq!(patterned.key.pattern_code, "p0");
        assert!(patterned.diagnostics[0].contains("not implemented"));
        assert_eq!(
            patterned.demand_coverage(2.0).coefficient,
            2.0
        );
        assert!(attribute.diagnostics()[0].contains("max_stack_layers"));
        assert!(attribute
            .validate()
            .iter()
            .any(|diagnostic| diagnostic.contains("disallows mixed loading")));
        assert!(attribute
            .packing_diagnostics(3)
            .iter()
            .any(|diagnostic| diagnostic.contains("exceeded")));
    }

    #[test]
    fn material_key_extraction() {
        let material = Material {
            no: "MAT001".to_string(),
            material_type: MaterialType::RawMaterial,
            name: "Steel".to_string(),
            warehouse: Some("WH-A".to_string()),
            weight: meters(1.0),
        };
        let key = material.key();
        assert_eq!(key.no, "MAT001");
        assert_eq!(key.material_type, MaterialType::RawMaterial);
    }

    #[test]
    fn bin_type_construction() {
        let bin_type = BinType {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
            capacity: meters(1000.0),
            type_code: "BIN-10".to_string(),
            is_main: true,
        };
        assert!(bin_type.is_main);
        assert_eq!(bin_type.type_code, "BIN-10");
    }

    #[test]
    fn bin_layer_demand_coverage_lookup() {
        let key = Bpp3dDemandKey::Item { id: "i1".to_string() };
        let layer: BinLayer<f64, Meter> = BinLayer {
            iteration: 0,
            from: "test".to_string(),
            bin: None,
            depth: meters(1.0),
            demand_coverage: vec![Bpp3dLayerDemandCoverage::new(
                Bpp3dDemandMode::Item,
                key.clone(),
                2.0,
            )],
        };

        assert_eq!(
            layer.demand_coverage_coefficient(Bpp3dDemandMode::Item, &key),
            2.0
        );
    }

    #[test]
    fn cylinder_shape_contract_require_vertical_axis() {
        assert!(CylinderShapeContract::require_vertical_axis(Axis3::Y).is_ok());
        assert!(CylinderShapeContract::require_vertical_axis(Axis3::X).is_err());
    }

    #[test]
    fn cylinder_shape_contract_has_cylinder() {
        let items = vec![
            ActualItem {
                id: "cuboid".to_string(),
                name: "Cuboid".to_string(),
                package_code: None,
                pack: None,
                width: meters(2.0),
                height: meters(3.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![],
                shape_spec_override: None,
            },
        ];
        assert!(!CylinderShapeContract::has_cylinder(&items));

        let items_with_cyl = vec![
            ActualItem {
                id: "cyl".to_string(),
                name: "Cylinder".to_string(),
                package_code: None,
                pack: None,
                width: meters(4.0),
                height: meters(5.0),
                depth: meters(4.0),
                weight: meters(1.0),
                enabled_orientations: vec![],
                shape_spec_override: Some(PackageShapeSpec::Cylinder {
                    axis: Axis3::Y,
                    radius: meters(2.0),
                    radius_candidates: None,
                    radius_lower_bound: None,
                    radius_upper_bound: None,
                }),
            },
        ];
        assert!(CylinderShapeContract::has_cylinder(&items_with_cyl));
    }

    #[test]
    fn continuous_radius_model_component_info() {
        let component = ContinuousRadiusModelComponent {
            prototypes: vec![ContinuousCylinderRadiusSolverPrototype {
                source: "test".to_string(),
                axis: Axis3::Y,
                variable_name: "r_1".to_string(),
                radius_lower_bound: Some(1.0),
                radius_upper_bound: Some(3.0),
            }],
            registration_plan: ContinuousRadiusRegistrationPlan {
                variable_names: vec!["r_1".to_string()],
                registered_variables: vec!["r_1".to_string()],
                blocked_variables: vec![],
            },
        };
        let info = component.info();
        assert_eq!(info[0].1, "1");
    }
}
