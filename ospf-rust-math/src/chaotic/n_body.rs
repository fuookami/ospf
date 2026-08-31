//! N 体系统。
//! N-body system.

use num_traits::Float;
use crate::algebra::Field;
use crate::geometry::Point3;
/// N 体系统。
/// N-body system.
///
/// 公式: dv_i/dt = sum_{j!=i} G * m_j * (x_j - x_i) / |x_j - x_i|^3
#[derive(Clone, Debug, PartialEq)]
pub struct NBodySystem<S: Field + Float = f64> {
    m: Vec<S>,
    g: S,
    h: S,
}

impl<S: Field + Float> NBodySystem<S> {
    pub fn new(m: Vec<S>, g: S, h: S) -> Self {
        Self { m, g, h }
    }

    pub fn m(&self) -> &[S] { &self.m }
    pub fn g(&self) -> S { self.g }
    pub fn h(&self) -> S { self.h }

    /// 执行一次 N 体步进。
    /// Execute one N-body step.
    ///
    /// 状态为 (位置, 速度) 对的列表。
    /// State is a list of (position, velocity) pairs.
    pub fn step(&self, state: &[(Point3<S>, Point3<S>)]) -> Vec<(Point3<S>, Point3<S>)> {
        let n = state.len();
        let zero = S::zero();
        (0..n)
            .map(|i| {
                let (ref pos_i, ref vel_i) = state[i];
                let (mut ax, mut ay, mut az) = (zero, zero, zero);
                for j in 0..n {
                    if i != j {
                        let (ref pos_j, _) = state[j];
                        let dx = pos_j.x() - pos_i.x();
                        let dy = pos_j.y() - pos_i.y();
                        let dz = pos_j.z() - pos_i.z();
                        let dist_sq = dx * dx + dy * dy + dz * dz;
                        let dist = dist_sq.sqrt();
                        let dist_cubed = dist_sq * dist;
                        if dist_cubed > zero {
                            let force = self.g * self.m[j] / dist_cubed;
                            ax = ax + force * dx;
                            ay = ay + force * dy;
                            az = az + force * dz;
                        }
                    }
                }
                let new_pos = Point3::new(
                    pos_i.x() + self.h * vel_i.x(),
                    pos_i.y() + self.h * vel_i.y(),
                    pos_i.z() + self.h * vel_i.z(),
                );
                let new_vel = Point3::new(
                    vel_i.x() + self.h * ax,
                    vel_i.y() + self.h * ay,
                    vel_i.z() + self.h * az,
                );
                (new_pos, new_vel)
            })
            .collect()
    }

    pub fn generator(self, initial: Vec<(Point3<S>, Point3<S>)>) -> NBodySystemGenerator<S> {
        NBodySystemGenerator::new(self, initial)
    }
}

/// N 体系统序列生成器。
/// N-body system sequence generator.
#[derive(Clone, Debug, PartialEq)]
pub struct NBodySystemGenerator<S: Field + Float = f64> {
    system: NBodySystem<S>,
    state: Vec<(Point3<S>, Point3<S>)>,
}

impl<S: Field + Float> NBodySystemGenerator<S> {
    pub fn new(system: NBodySystem<S>, state: Vec<(Point3<S>, Point3<S>)>) -> Self {
        Self { system, state }
    }

    pub fn system(&self) -> &NBodySystem<S> { &self.system }
    pub fn state(&self) -> &[(Point3<S>, Point3<S>)] { &self.state }

    pub fn next_state(&mut self) -> Vec<(Point3<S>, Point3<S>)> {
        let current = self.state.clone();
        self.state = self.system.step(&current);
        current
    }
}

impl<S: Field + Float> Iterator for NBodySystemGenerator<S> {
    type Item = Vec<(Point3<S>, Point3<S>)>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_state())
    }
}

/// 创建 N 体系统。
/// Create an N-body system.
pub fn n_body_system<S: Field + Float>(m: Vec<S>, g: S, h: S) -> NBodySystem<S> {
    NBodySystem::new(m, g, h)
}

/// 创建 N 体系统生成器。
/// Create an N-body system generator.
pub fn n_body_system_generator<S: Field + Float>(
    m: Vec<S>, g: S, h: S, state: Vec<(Point3<S>, Point3<S>)>,
) -> NBodySystemGenerator<S> {
    NBodySystemGenerator::new(NBodySystem::new(m, g, h), state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn n_body_two_bodies() {
        let system = NBodySystem::new(vec![1.0_f64, 1.0], 1.0, 0.001);
        let state = vec![
            (Point3::new(-1.0, 0.0, 0.0), Point3::new(0.0, 0.0, 0.0)),
            (Point3::new(1.0, 0.0, 0.0), Point3::new(0.0, 0.0, 0.0)),
        ];
        let next = system.step(&state);
        assert_eq!(next.len(), 2);
        // Bodies should accelerate toward each other (positive x accel for body 0)
        assert!(next[0].1.x() > 0.0);
        assert!(next[1].1.x() < 0.0);
    }
}
