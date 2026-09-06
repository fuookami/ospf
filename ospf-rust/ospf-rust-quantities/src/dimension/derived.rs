//! Derived dimension - 导出量纲
//! Derived dimension - Derived physical dimensions
//!
//! 导出量纲由基础量纲的幂次组成，同时提供运行时和编译时版本。
//! Derived dimensions are composed of powers of base dimensions, providing both runtime and compile-time versions.
//!
//! # 示例 / Examples
//! - Area: L² (面积)
//! - Volume: L³ (体积)
//! - Velocity: L·T⁻¹ (速度)
//! - Acceleration: L·T⁻² (加速度)
//! - Force: L·M·T⁻² (力)
//! - Energy: L²·M·T⁻² (能量)
//!
//! # 分类 / Categories
//! - 基础量纲：长度、质量、时间、电流、温度、物质的量、发光强度、信息量、角度
//! - 几何量纲：面积、体积
//! - 力学量纲：速度、加速度、力、能量、功率等
//! - 电磁学量纲：电荷、电压、电阻、电容等
//! - 光学量纲：光通量、照度、亮度

use super::derived_quantity::{CTDerivedQuantity, QuantityDomain};
use super::fundamental_quantity::{
    I0, I1, Info0, Info1, J0, J1, L0, L1, M0, M1, N0, N1, Omega0, Omega1, Phi0, Phi1, T0, T1,
    Theta0, Theta1,
};
use crate::dimension::{CTDerivedDiv, CTDerivedMul, CTDerivedPow, CTDerivedReciprocal};
use typenum::{P2, P3};

// ============================================================================
// 基础量纲结构体定义 / Base dimension struct definitions
// ============================================================================

/// 无量纲 / Dimensionless
pub struct DimLess;
impl CTDerivedQuantity for DimLess {
    type L = L0;
    type M = M0;
    type T = T0;
    type I = I0;
    type Theta = Theta0;
    type N = N0;
    type J = J0;
    type Info = Info0;
    type Phi = Phi0;
    type Omega = Omega0;

    const NAME: &'static str = "Dimension Less";
}

/// 长度量纲: L / Length dimension: L
pub struct Length;
impl CTDerivedQuantity for Length {
    type L = L1;
    type M = M0;
    type T = T0;
    type I = I0;
    type Theta = Theta0;
    type N = N0;
    type J = J0;
    type Info = Info0;
    type Phi = Phi0;
    type Omega = Omega0;

    const NAME: &'static str = "Length";
}

/// 质量量纲: M / Mass dimension: M
pub struct Mass;
impl CTDerivedQuantity for Mass {
    type L = L0;
    type M = M1;
    type T = T0;
    type I = I0;
    type Theta = Theta0;
    type N = N0;
    type J = J0;
    type Info = Info0;
    type Phi = Phi0;
    type Omega = Omega0;

    const NAME: &'static str = "Mass";
}

/// 时间量纲: T / Time dimension: T
pub struct Time;
impl CTDerivedQuantity for Time {
    type L = L0;
    type M = M0;
    type T = T1;
    type I = I0;
    type Theta = Theta0;
    type N = N0;
    type J = J0;
    type Info = Info0;
    type Phi = Phi0;
    type Omega = Omega0;

    const NAME: &'static str = "Time";
}

/// 电流量纲: I / Electric Current dimension: I
pub struct ElectricCurrent;
impl CTDerivedQuantity for ElectricCurrent {
    type L = L0;
    type M = M0;
    type T = T0;
    type I = I1;
    type Theta = Theta0;
    type N = N0;
    type J = J0;
    type Info = Info0;
    type Phi = Phi0;
    type Omega = Omega0;

    const NAME: &'static str = "Electric Current";
}

/// 热力学温度量纲: Θ / Thermodynamic Temperature dimension: Θ
pub struct ThermodynamicTemperature;
impl CTDerivedQuantity for ThermodynamicTemperature {
    type L = L0;
    type M = M0;
    type T = T0;
    type I = I0;
    type Theta = Theta1;
    type N = N0;
    type J = J0;
    type Info = Info0;
    type Phi = Phi0;
    type Omega = Omega0;

    const NAME: &'static str = "Thermodynamic Temperature";
}

/// 物质的量量纲: N / Amount of Substance dimension: N
pub struct AmountOfSubstance;
impl CTDerivedQuantity for AmountOfSubstance {
    type L = L0;
    type M = M0;
    type T = T0;
    type I = I0;
    type Theta = Theta0;
    type N = N1;
    type J = J0;
    type Info = Info0;
    type Phi = Phi0;
    type Omega = Omega0;

    const NAME: &'static str = "Amount of Substance";
}

/// 发光强度量纲: J / Luminous Intensity dimension: J
pub struct LuminousIntensity;
impl CTDerivedQuantity for LuminousIntensity {
    type L = L0;
    type M = M0;
    type T = T0;
    type I = I0;
    type Theta = Theta0;
    type N = N0;
    type J = J1;
    type Info = Info0;
    type Phi = Phi0;
    type Omega = Omega0;

    const NAME: &'static str = "Luminous Intensity";
}

/// 信息量量纲: ℐ / Information dimension: ℐ
pub struct Information;
impl CTDerivedQuantity for Information {
    type L = L0;
    type M = M0;
    type T = T0;
    type I = I0;
    type Theta = Theta0;
    type N = N0;
    type J = J0;
    type Info = Info1;
    type Phi = Phi0;
    type Omega = Omega0;

    const NAME: &'static str = "Information";
    const DOMAIN: QuantityDomain = QuantityDomain::Discrete;
}

/// 平面角量纲: φ / Plane Angle dimension: φ
pub struct PlaneAngle;
impl CTDerivedQuantity for PlaneAngle {
    type L = L0;
    type M = M0;
    type T = T0;
    type I = I0;
    type Theta = Theta0;
    type N = N0;
    type J = J0;
    type Info = Info0;
    type Phi = Phi1;
    type Omega = Omega0;

    const NAME: &'static str = "Plane Angle";
}

/// 立体角量纲: Ω / Solid Angle dimension: Ω
pub struct SolidAngle;
impl CTDerivedQuantity for SolidAngle {
    type L = L0;
    type M = M0;
    type T = T0;
    type I = I0;
    type Theta = Theta0;
    type N = N0;
    type J = J0;
    type Info = Info0;
    type Phi = Phi0;
    type Omega = Omega1;

    const NAME: &'static str = "Solid Angle";
}

// ============================================================================
// 导出量纲宏定义 / Derived dimension macro definition
// ============================================================================

/// 定义导出量纲的宏 / Macro for defining derived dimensions
///
/// # 参数 / Parameters
/// - `$name`: 量纲结构体名称 / Dimension struct name
/// - `$display_name`: 量纲显示名称 / Dimension display name
/// - `$ct_expr`: 编译时量纲表达式 / Compile-time dimension expression
///
/// # 示例 / Example
/// ```
/// use ospf_rust_quantities::dimension::derived::{Length, Area, Volume};
/// use ospf_rust_quantities::dimension::CTDerivedQuantity;
///
/// // 面积量纲: L²
/// assert_eq!(*Area::SYMBOL, "L^2");
///
/// // 体积量纲: L³
/// assert_eq!(*Volume::SYMBOL, "L^3");
/// ```
macro_rules! define_derived_dimension {
    ($name:ident, $display_name:literal, $ct_expr:ty) => {
        /// 导出量纲结构体 / Derived dimension struct
        pub struct $name;

        impl CTDerivedQuantity for $name {
            type L = <$ct_expr as CTDerivedQuantity>::L;
            type M = <$ct_expr as CTDerivedQuantity>::M;
            type T = <$ct_expr as CTDerivedQuantity>::T;
            type I = <$ct_expr as CTDerivedQuantity>::I;
            type Theta = <$ct_expr as CTDerivedQuantity>::Theta;
            type N = <$ct_expr as CTDerivedQuantity>::N;
            type J = <$ct_expr as CTDerivedQuantity>::J;
            type Info = <$ct_expr as CTDerivedQuantity>::Info;
            type Phi = <$ct_expr as CTDerivedQuantity>::Phi;
            type Omega = <$ct_expr as CTDerivedQuantity>::Omega;

            const NAME: &'static str = $display_name;
            const DOMAIN: QuantityDomain = <$ct_expr as CTDerivedQuantity>::DOMAIN;
        }
    };
}

// ============================================================================
// 基本导出量纲定义 / Basic derived dimension definitions
// ============================================================================

// 面积量纲: L² / Area dimension: L²
define_derived_dimension!(Area, "Area", CTDerivedMul<Length, Length>);

// 体积量纲: L³ / Volume dimension: L³
define_derived_dimension!(Volume, "Volume", CTDerivedPow<Length, P3>);

// 频率量纲: T⁻¹ / Frequency dimension: T⁻¹
define_derived_dimension!(Frequency, "Frequency", CTDerivedReciprocal<Time>);

// ============================================================================
// 力学量纲定义 / Mechanics dimension definitions
// ============================================================================

// 速度量纲: L·T⁻¹ / Velocity dimension: L·T⁻¹
define_derived_dimension!(Velocity, "Velocity", CTDerivedDiv<Length, Time>);

// 角速度量纲: φ·T⁻¹ / Angular velocity dimension: φ·T⁻¹
define_derived_dimension!(AngularVelocity, "Angular Velocity", CTDerivedDiv<PlaneAngle, Time>);

// 波数量纲: L⁻¹ / Wave number dimension: L⁻¹
define_derived_dimension!(WaveNumber, "Wave Number", CTDerivedReciprocal<Length>);

// 加速度量纲: L·T⁻² / Acceleration dimension: L·T⁻²
define_derived_dimension!(Acceleration, "Acceleration", CTDerivedDiv<Length, CTDerivedPow<Time, P2>>);

// 角加速度量纲: φ·T⁻² / Angular acceleration dimension: φ·T⁻²
define_derived_dimension!(AngularAcceleration, "Angular Acceleration", CTDerivedDiv<PlaneAngle, CTDerivedPow<Time, P2>>);

// 动量量纲: L·M·T⁻¹ / Momentum dimension: L·M·T⁻¹
define_derived_dimension!(Momentum, "Momentum", CTDerivedMul<Mass, Velocity>);

// 角动量量纲: M·φ·T⁻¹ / Angular momentum dimension: M·φ·T⁻¹
define_derived_dimension!(AngularMomentum, "Angular Momentum", CTDerivedMul<Mass, CTDerivedDiv<PlaneAngle, Time>>);

// 转动惯量量纲: M·L² / Moment of inertia dimension: M·L²
define_derived_dimension!(MomentOfInertia, "Moment of Inertia", CTDerivedMul<Mass, CTDerivedPow<Length, P2>>);

// 力量纲: L·M·T⁻² / Force dimension: L·M·T⁻²
define_derived_dimension!(Force, "Force", CTDerivedMul<Mass, Acceleration>);

// 压力量纲: L⁻¹·M·T⁻² / Pressure dimension: L⁻¹·M·T⁻²
define_derived_dimension!(Pressure, "Pressure", CTDerivedDiv<Force, CTDerivedPow<Length, P2>>);

// 冲量量纲: L·M·T⁻¹ (同动量) / Impulse dimension: L·M·T⁻¹ (same as momentum)
pub type Impulse = Momentum;

// 扭矩量纲: L²·M·T⁻² / Torque dimension: L²·M·T⁻²
define_derived_dimension!(Torque, "Torque", CTDerivedMul<Force, Length>);

// 质量密度量纲: M·L⁻³ / Mass density dimension: M·L⁻³
define_derived_dimension!(MassDensity, "Mass Density", CTDerivedDiv<Mass, Volume>);

// 比体积量纲: L³·M⁻¹ / Specific volume dimension: L³·M⁻¹
define_derived_dimension!(SpecificVolume, "Specific Volume", CTDerivedDiv<Volume, Mass>);

// 能量量纲: L²·M·T⁻² / Energy dimension: L²·M·T⁻²
define_derived_dimension!(Energy, "Energy", CTDerivedMul<Force, Length>);

// 功量纲: 同能量 / Work dimension: same as energy
pub type Work = Energy;

// 热量纲: 同能量 / Heat dimension: same as energy
pub type Heat = Energy;

// 摩尔浓度量纲: N·L⁻³ / Molarity dimension: N·L⁻³
define_derived_dimension!(Molarity, "Molarity", CTDerivedDiv<AmountOfSubstance, Volume>);

// 摩尔体积量纲: L³·N⁻¹ / Molar volume dimension: L³·N⁻¹
define_derived_dimension!(MolarVolume, "Molar Volume", CTDerivedDiv<Volume, AmountOfSubstance>);

// 熵量纲: L²·M·T⁻²·Θ⁻¹ / Entropy dimension: L²·M·T⁻²·Θ⁻¹
define_derived_dimension!(Entropy, "Entropy", CTDerivedDiv<Energy, ThermodynamicTemperature>);

// 摩尔熵量纲: L²·M·T⁻²·Θ⁻¹·N⁻¹ / Molar entropy dimension: L²·M·T⁻²·Θ⁻¹·N⁻¹
define_derived_dimension!(MolarEntropy, "Molar Entropy", CTDerivedDiv<Entropy, AmountOfSubstance>);

// 摩尔热容: 同摩尔熵 / Molar heat capacity: same as molar entropy
pub type MolarHeatCapacity = MolarEntropy;

// 比熵量纲: L²·T⁻²·Θ⁻¹ / Specific entropy dimension: L²·T⁻²·Θ⁻¹
define_derived_dimension!(SpecificEntropy, "Specific Entropy", CTDerivedDiv<Energy, CTDerivedMul<Mass, ThermodynamicTemperature>>);

// 比热容: 同比熵 / Specific heat capacity: same as specific entropy
pub type SpecificHeatCapacity = SpecificEntropy;

// 摩尔能量量纲: L²·M·T⁻²·N⁻¹ / Molar energy dimension: L²·M·T⁻²·N⁻¹
define_derived_dimension!(MolarEnergy, "Molar Energy", CTDerivedDiv<Energy, AmountOfSubstance>);

// 比能量量纲: L²·T⁻² / Specific energy dimension: L²·T⁻²
define_derived_dimension!(SpecificEnergy, "Specific Energy", CTDerivedDiv<Energy, Mass>);

// 能量密度量纲: L⁻¹·M·T⁻² / Energy density dimension: L⁻¹·M·T⁻²
define_derived_dimension!(EnergyDensity, "Energy Density", CTDerivedDiv<Energy, Volume>);

// 热容: 同熵 / Heat capacity: same as entropy
pub type HeatCapacity = Entropy;

// 表面张力量纲: M·T⁻² / Surface tension dimension: M·T⁻²
define_derived_dimension!(SurfaceTension, "Surface Tension", CTDerivedDiv<Force, Length>);

// 功率量纲: L²·M·T⁻³ / Power dimension: L²·M·T⁻³
define_derived_dimension!(Power, "Power", CTDerivedDiv<Energy, Time>);

// 功率密度量纲: M·T⁻³ / Power density dimension: M·T⁻³
define_derived_dimension!(PowerDensity, "Power Density", CTDerivedDiv<Power, Volume>);

// 辐照度: 同功率密度 / Irradiance: same as power density
pub type Irradiance = PowerDensity;

// 热通量密度: 同功率密度 / Heat flux density: same as power density
pub type HeatFluxDensity = PowerDensity;

// 热导率量纲: L·M·T⁻³·Θ⁻¹ / Thermal conductivity dimension: L·M·T⁻³·Θ⁻¹
define_derived_dimension!(
    ThermalConductivity,
    "Thermal Conductivity",
    CTDerivedDiv<CTDerivedMul<Power, Length>, ThermodynamicTemperature>
);

// 动力粘度量纲: L⁻¹·M·T⁻¹ / Dynamic viscosity dimension: L⁻¹·M·T⁻¹
define_derived_dimension!(
    DynamicViscosity,
    "Dynamic Viscosity",
    CTDerivedDiv<CTDerivedMul<Mass, Length>, Time>
);

// 运动粘度量纲: L²·T⁻¹ / Kinematic viscosity dimension: L²·T⁻¹
define_derived_dimension!(KinematicViscosity, "Kinematic Viscosity", CTDerivedDiv<Area, Time>);

// 摩尔质量量纲: M·N⁻¹ / Molar mass dimension: M·N⁻¹
define_derived_dimension!(MolarMass, "Molar Mass", CTDerivedDiv<Mass, AmountOfSubstance>);

// 线密度量纲: M·L⁻¹ / Linear density dimension: M·L⁻¹
define_derived_dimension!(LinearDensity, "Linear Density", CTDerivedDiv<Mass, Length>);

// 表面密度量纲: M·L⁻² / Surface density dimension: M·L⁻²
define_derived_dimension!(SurfaceDensity, "Surface Density", CTDerivedDiv<Mass, Area>);

// 作用量纲: L²·M·T⁻¹ / Action dimension: L²·M·T⁻¹
define_derived_dimension!(Action, "Action", CTDerivedMul<Energy, Time>);

// 流量量纲: L³·T⁻¹ / Flow rate dimension: L³·T⁻¹
define_derived_dimension!(FlowRate, "Flow Rate", CTDerivedDiv<Volume, Time>);

// ============================================================================
// 电磁学量纲定义 / Electromagnetism dimension definitions
// ============================================================================

// 电荷量纲: I·T / Electric Charge dimension: I·T
define_derived_dimension!(ElectricCharge, "Electric Charge", CTDerivedMul<ElectricCurrent, Time>);

// 电荷密度量纲: I·T·L⁻³ / Electric charge density dimension: I·T·L⁻³
define_derived_dimension!(ElectricChargeDensity, "Electric Charge Density", CTDerivedDiv<ElectricCharge, Volume>);

// 电流密度量纲: I·L⁻² / Electric current density dimension: I·L⁻²
define_derived_dimension!(ElectricCurrentDensity, "Electric Current Density", CTDerivedDiv<ElectricCurrent, Area>);

// 电势量纲: L²·M·T⁻³·I⁻¹ / Electric potential dimension: L²·M·T⁻³·I⁻¹
define_derived_dimension!(ElectricPotential, "Electric Potential", CTDerivedDiv<Power, ElectricCurrent>);

// 电压: 同电势 / Voltage: same as electric potential
pub type Voltage = ElectricPotential;

// 电阻量纲: L²·M·T⁻³·I⁻² / Resistance dimension: L²·M·T⁻³·I⁻²
define_derived_dimension!(Resistance, "Resistance", CTDerivedDiv<ElectricPotential, ElectricCurrent>);

// 阻抗: 同电阻 / Impedance: same as resistance
pub type Impedance = Resistance;

// 电导量纲: L⁻²·M⁻¹·T³·I² / Conductance dimension: L⁻²·M⁻¹·T³·I²
define_derived_dimension!(Conductance, "Conductance", CTDerivedReciprocal<Resistance>);

// 电导率量纲: L⁻³·M⁻¹·T³·I² / Conductivity dimension: L⁻³·M⁻¹·T³·I²
define_derived_dimension!(Conductivity, "Conductivity", CTDerivedDiv<Conductance, Length>);

// 电容量纲: T⁴·I²·L⁻²·M⁻¹ / Capacitance dimension: T⁴·I²·L⁻²·M⁻¹
define_derived_dimension!(Capacitance, "Capacitance", CTDerivedDiv<ElectricCharge, ElectricPotential>);

// 介电常数量纲: T⁴·I²·L⁻³·M⁻¹ / Permittivity dimension: T⁴·I²·L⁻³·M⁻¹
define_derived_dimension!(Permittivity, "Permittivity", CTDerivedDiv<Capacitance, Length>);

// 电场强度量纲: L·M·T⁻³·I⁻¹ / Electric field strength dimension: L·M·T⁻³·I⁻¹
define_derived_dimension!(ElectricFieldStrength, "Electric Field Strength", CTDerivedDiv<ElectricPotential, Length>);

// 电感量纲: L²·M·T⁻²·I⁻² / Inductance dimension: L²·M·T⁻²·I⁻²
define_derived_dimension!(
    Inductance,
    "Inductance",
    CTDerivedDiv<CTDerivedMul<ElectricPotential, Time>, ElectricCurrent>
);

// 磁感应强度量纲: M·T⁻²·I⁻¹ / Magnetic field density dimension: M·T⁻²·I⁻¹
define_derived_dimension!(MagneticFieldDensity, "Magnetic Field Density", CTDerivedDiv<Force, CTDerivedMul<ElectricCurrent, Length>>);

// 磁场强度量纲: I·L⁻¹ / Magnetic field intensity dimension: I·L⁻¹
define_derived_dimension!(MagneticFieldIntensity, "Magnetic Field Intensity", CTDerivedDiv<ElectricCurrent, Length>);

// 磁通量量纲: L²·M·T⁻²·I⁻¹ / Magnetic flux dimension: L²·M·T⁻²·I⁻¹
define_derived_dimension!(MagneticFlux, "Magnetic Flux", CTDerivedMul<MagneticFieldDensity, Area>);

// 磁导率量纲: L·M·T⁻²·I⁻² / Magnetic permeability dimension: L·M·T⁻²·I⁻²
define_derived_dimension!(MagneticPermeability, "Magnetic Permeability", CTDerivedDiv<Inductance, Length>);

// 磁阻量纲: L⁻¹·M⁻¹·T²·I² / Magnetic reluctance dimension: L⁻¹·M⁻¹·T²·I²
define_derived_dimension!(
    MagneticReluctance,
    "Magnetic Reluctance",
    CTDerivedReciprocal<Inductance>
);

// 电荷线密度量纲: I·T·L⁻¹ / Electric charge linear density dimension: I·T·L⁻¹
define_derived_dimension!(ElectricChargeLinearDensity, "Electric Charge Linear Density", CTDerivedDiv<ElectricCharge, Length>);

// 电荷面密度量纲: I·T·L⁻² / Electric charge surface density dimension: I·T·L⁻²
define_derived_dimension!(ElectricChargeSurfaceDensity, "Electric Charge Surface Density", CTDerivedDiv<ElectricCharge, Area>);

// ============================================================================
// 光学量纲定义 / Optics dimension definitions
// ============================================================================

// 光通量量纲: J·Ω / Luminous flux dimension: J·Ω
define_derived_dimension!(LuminousFlux, "Luminous Flux", CTDerivedMul<LuminousIntensity, SolidAngle>);

// 照度量纲: J·Ω·L⁻² / Illuminance dimension: J·Ω·L⁻²
define_derived_dimension!(Illuminance, "Illuminance", CTDerivedDiv<LuminousFlux, Area>);

// 亮度量纲: J·L⁻² / Luminance dimension: J·L⁻²
define_derived_dimension!(Luminance, "Luminance", CTDerivedDiv<LuminousIntensity, Area>);

// ============================================================================
// 放射学量纲定义 / Radiology dimension definitions
// ============================================================================

// 活度量纲: T⁻¹ / Activity dimension: T⁻¹
pub type Activity = Frequency;

// 吸收剂量量纲: L²·T⁻² / Absorbed dose dimension: L²·T⁻²
pub type AbsorbedDose = SpecificEnergy;

// 剂量当量: 同吸收剂量 / Dose equivalent: same as absorbed dose
pub type DoseEquivalent = SpecificEnergy;

// 剂量率量纲: L²·T⁻³ / Dosing rate dimension: L²·T⁻³
define_derived_dimension!(DosingRate, "Dosing Rate", CTDerivedDiv<SpecificEnergy, Time>);

// ============================================================================
// 其他量纲定义 / Other dimension definitions
// ============================================================================

// 催化活度量纲: N·T⁻¹ / Catalytic activity dimension: N·T⁻¹
define_derived_dimension!(CatalyticActivity, "Catalytic Activity", CTDerivedDiv<AmountOfSubstance, Time>);

// 带宽量纲: ℐ·T⁻¹ / Bandwidth dimension: ℐ·T⁻¹
define_derived_dimension!(Bandwidth, "Bandwidth", CTDerivedDiv<Information, Time>);

// ============================================================================
// 单元测试 / Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dimension::derived_quantity::CTDerivedQuantity;

    #[test]
    fn test_area_dimension() {
        // 验证 Area 的符号 / Verify Area symbol
        assert_eq!(*Area::SYMBOL, "L^2");
        assert_eq!(Area::NAME, "Area");
    }

    #[test]
    fn test_volume_dimension() {
        // 验证 Volume 的符号 / Verify Volume symbol
        assert_eq!(*Volume::SYMBOL, "L^3");
        assert_eq!(Volume::NAME, "Volume");
    }

    #[test]
    fn test_velocity_dimension() {
        // 验证 Velocity 的符号 / Verify Velocity symbol
        assert_eq!(*Velocity::SYMBOL, "L·T^-1");
        assert_eq!(Velocity::NAME, "Velocity");
    }

    #[test]
    fn test_acceleration_dimension() {
        // 验证 Acceleration 的符号 / Verify Acceleration symbol
        assert_eq!(*Acceleration::SYMBOL, "L·T^-2");
        assert_eq!(Acceleration::NAME, "Acceleration");
    }

    #[test]
    fn test_force_dimension() {
        // 验证 Force 的符号 / Verify Force symbol
        // Force = M * L * T^-2
        assert_eq!(*Force::SYMBOL, "L·M·T^-2");
        assert_eq!(Force::NAME, "Force");
    }

    #[test]
    fn test_energy_dimension() {
        // 验证 Energy 的符号 / Verify Energy symbol
        // Energy = L^2 * M * T^-2
        assert_eq!(*Energy::SYMBOL, "L^2·M·T^-2");
        assert_eq!(Energy::NAME, "Energy");
    }

    #[test]
    fn test_power_dimension() {
        // 验证 Power 的符号 / Verify Power symbol
        // Power = L^2 * M * T^-3
        assert_eq!(*Power::SYMBOL, "L^2·M·T^-3");
        assert_eq!(Power::NAME, "Power");
    }

    #[test]
    fn test_pressure_dimension() {
        // 验证 Pressure 的符号 / Verify Pressure symbol
        // Pressure = L^-1 * M * T^-2
        assert_eq!(*Pressure::SYMBOL, "L^-1·M·T^-2");
        assert_eq!(Pressure::NAME, "Pressure");
    }

    #[test]
    fn test_frequency_dimension() {
        // 验证 Frequency 的符号 / Verify Frequency symbol
        assert_eq!(*Frequency::SYMBOL, "T^-1");
        assert_eq!(Frequency::NAME, "Frequency");
    }

    #[test]
    fn test_electric_charge_dimension() {
        // 验证 ElectricCharge 的符号 / Verify ElectricCharge symbol
        // ElectricCharge = I·T, 符号顺序取决于实现 / symbol order depends on implementation
        let symbol = &*ElectricCharge::SYMBOL;
        assert!(
            symbol == "I·T" || symbol == "T·I",
            "Expected I·T or T·I, got {}",
            symbol
        );
        assert_eq!(ElectricCharge::NAME, "Electric Charge");
    }

    #[test]
    fn test_electric_potential_dimension() {
        // 验证 ElectricPotential 的符号 / Verify ElectricPotential symbol
        // ElectricPotential = L^2 * M * T^-3 * I^-1
        assert_eq!(*ElectricPotential::SYMBOL, "L^2·M·T^-3·I^-1");
        assert_eq!(ElectricPotential::NAME, "Electric Potential");
    }

    #[test]
    fn test_resistance_dimension() {
        // 验证 Resistance 的符号 / Verify Resistance symbol
        // Resistance = L^2 * M * T^-3 * I^-2
        assert_eq!(*Resistance::SYMBOL, "L^2·M·T^-3·I^-2");
        assert_eq!(Resistance::NAME, "Resistance");
    }

    #[test]
    fn test_dimensionless() {
        // 验证无量纲 / Verify dimensionless
        assert_eq!(*DimLess::SYMBOL, "1");
        assert_eq!(DimLess::NAME, "Dimension Less");
    }

    #[test]
    fn test_base_dimensions() {
        // 验证基本量纲 / Verify base dimensions
        assert_eq!(*Length::SYMBOL, "L");
        assert_eq!(*Mass::SYMBOL, "M");
        assert_eq!(*Time::SYMBOL, "T");
        assert_eq!(*ElectricCurrent::SYMBOL, "I");
        assert_eq!(*ThermodynamicTemperature::SYMBOL, "Θ");
        assert_eq!(*AmountOfSubstance::SYMBOL, "N");
        assert_eq!(*LuminousIntensity::SYMBOL, "J");
        assert_eq!(*Information::SYMBOL, "ℐ");
        assert_eq!(*PlaneAngle::SYMBOL, "φ");
        assert_eq!(*SolidAngle::SYMBOL, "Ω");
    }

    #[test]
    fn test_type_aliases() {
        // 验证类型别名 / Verify type aliases
        // 这些类型别名应该与原始类型有相同的符号
        fn assert_same_symbol<D1: CTDerivedQuantity, D2: CTDerivedQuantity>() {
            assert_eq!(*D1::SYMBOL, *D2::SYMBOL);
        }

        assert_same_symbol::<Work, Energy>();
        assert_same_symbol::<Heat, Energy>();
        assert_same_symbol::<Impulse, Momentum>();
        assert_same_symbol::<Voltage, ElectricPotential>();
        assert_same_symbol::<Impedance, Resistance>();
        assert_same_symbol::<Activity, Frequency>();
        assert_same_symbol::<AbsorbedDose, SpecificEnergy>();
    }

    #[test]
    fn test_derived_dimension_composition() {
        // 验证导出量纲的组成 / Verify composition of derived dimensions
        // Force = Mass * Acceleration
        // Acceleration = Length / Time^2
        // 所以 Force = Mass * Length / Time^2

        // 验证编译时类型 / Verify compile-time types
        type ExpectedForce = CTDerivedMul<Mass, CTDerivedDiv<Length, CTDerivedPow<Time, P2>>>;
        assert_eq!(*Force::SYMBOL, *ExpectedForce::SYMBOL);

        // Energy = Force * Length
        type ExpectedEnergy = CTDerivedMul<Force, Length>;
        assert_eq!(*Energy::SYMBOL, *ExpectedEnergy::SYMBOL);

        // Power = Energy / Time
        type ExpectedPower = CTDerivedDiv<Energy, Time>;
        assert_eq!(*Power::SYMBOL, *ExpectedPower::SYMBOL);
    }
}
