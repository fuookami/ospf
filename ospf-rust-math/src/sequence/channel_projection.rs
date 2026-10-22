//! 通道投影 / Channel projection
//!
//! 混沌吸引子与分形迭代器会产出多分量序列（标量、二维点、三维点、四维点等）。本模块把这些异构输出
//! 统一投影为单个标量通道，使上层参数演化算子可以用同一套接口消费混沌与分形序列。
//! Chaotic attractors and fractal iterators produce multi-component sequences (scalar, 2D, 3D, 4D points, …). This module
//! projects those heterogeneous outputs into a single scalar channel so parameter-evolution operators can consume
//! chaotic and fractal sequences through one interface.

use std::fmt::{Display, Formatter};

use crate::geometry::{Point2, Point3, Point4};

/// 序列投影错误 / Sequence projection error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceError {
    message: String,
}

impl SequenceError {
    /// 创建投影错误 / Create a projection error
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// 错误信息 / Error message
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for SequenceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SequenceError {}

/// 序列投影结果 / Sequence projection result
pub type SequenceResult<T> = std::result::Result<T, SequenceError>;

/// 可投影的多分量值 / Projectable multi-component value
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectedValue {
    /// 标量 / Scalar
    Scalar(f64),
    /// 二维点 / 2D point
    Point2(Point2<f64>),
    /// 三维点 / 3D point
    Point3(Point3<f64>),
    /// 四维点 / 4D point
    Point4(Point4<f64>),
}

impl ProjectedValue {
    /// 分量列表 / Component list
    pub fn components(&self) -> Vec<f64> {
        match self {
            ProjectedValue::Scalar(value) => vec![*value],
            ProjectedValue::Point2(point) => vec![point.x(), point.y()],
            ProjectedValue::Point3(point) => vec![point.x(), point.y(), point.z()],
            ProjectedValue::Point4(point) => {
                vec![point.x(), point.y(), point.z(), point.w()]
            }
        }
    }
}

/// 通道投影策略 / Channel projection strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelProjection {
    /// 标量通道，直接取值 / Scalar channel, taking the value as is
    Scalar,
    /// 分量通道，取指定分量下标 / Component channel, taking the component at the given index
    Component(usize),
    /// 模长通道，取 L2 范数 / Magnitude channel, taking the L2 norm
    Magnitude,
    /// 相位通道，取复平面极角（需要至少两个分量）/ Phase channel, taking the polar angle in the complex plane (needs at least two components)
    Phase,
}

impl ChannelProjection {
    /// 将多分量值投影为标量通道 / Project a multi-component value into a scalar channel
    pub fn project(&self, value: &ProjectedValue) -> SequenceResult<f64> {
        match value {
            ProjectedValue::Scalar(scalar) => self.project_scalar(*scalar),
            _ => self.project_components(&value.components()),
        }
    }

    /// 将标量投影为标量通道 / Project a scalar into a scalar channel
    pub fn project_scalar(&self, value: f64) -> SequenceResult<f64> {
        match self {
            ChannelProjection::Scalar => Ok(value),
            ChannelProjection::Magnitude => Ok(value.abs()),
            _ => Err(SequenceError::new(
                "projection requires a multi-component value / 该投影需要多分量输入",
            )),
        }
    }

    /// 将分量列表投影为标量通道 / Project a component list into a scalar channel
    pub fn project_components(&self, components: &[f64]) -> SequenceResult<f64> {
        if components.is_empty() {
            return Err(SequenceError::new(
                "components must be non-empty / 分量列表不得为空",
            ));
        }

        match self {
            ChannelProjection::Scalar => {
                if components.len() == 1 {
                    Ok(components[0])
                } else {
                    Err(SequenceError::new(
                        "scalar projection requires exactly one component / 标量投影要求分量个数恰为 1",
                    ))
                }
            }
            ChannelProjection::Component(index) => components.get(*index).copied().ok_or_else(|| {
                SequenceError::new(format!(
                    "component index {index} is out of range / 分量下标 {index} 越界"
                ))
            }),
            ChannelProjection::Magnitude => {
                let squared: f64 = components.iter().map(|value| value * value).sum();
                Ok(squared.sqrt())
            }
            ChannelProjection::Phase => {
                if components.len() >= 2 {
                    Ok(components[1].atan2(components[0]))
                } else {
                    Err(SequenceError::new(
                        "phase projection requires at least two components / 相位投影至少需要两个分量",
                    ))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_channel_accepts_scalars_and_magnitude_takes_abs() {
        assert_eq!(
            ChannelProjection::Scalar.project_scalar(1.5).unwrap(),
            1.5
        );
        assert_eq!(
            ChannelProjection::Magnitude.project_scalar(-2.5).unwrap(),
            2.5
        );
        assert!(ChannelProjection::Phase.project_scalar(1.0).is_err());
    }

    #[test]
    fn point_channels_project_components() {
        let point = ProjectedValue::Point2(Point2::new(3.0, 4.0));
        assert_eq!(ChannelProjection::Magnitude.project(&point).unwrap(), 5.0);
        assert_eq!(
            ChannelProjection::Component(1).project(&point).unwrap(),
            4.0
        );
        assert!((ChannelProjection::Phase.project(&point).unwrap()
            - 4.0_f64.atan2(3.0))
        .abs()
            < 1.0e-12);
        assert!(ChannelProjection::Component(2).project(&point).is_err());
        assert!(ChannelProjection::Scalar.project(&point).is_err());
    }
}
