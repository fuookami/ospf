//! 线性代数算子 / Linear algebra operators
//!
//! 目前只包含启发式算法需要的对称矩阵特征分解（Jacobi 旋转法），用于 CMA-ES 与快速非支配排序的参考点构造。
//! Currently only the symmetric eigendecomposition (Jacobi rotations) needed by metaheuristics, used by CMA-ES and by
//! reference-point construction in fast non-dominated sorting.

/// 对称矩阵特征分解结果 / Symmetric eigendecomposition result
#[derive(Debug, Clone, PartialEq)]
pub struct SymmetricEigen {
    /// 特征值（降序）/ Eigenvalues in descending order
    pub values: Vec<f64>,
    /// 与特征值对应的特征向量（列向量）/ Eigenvectors corresponding to the eigenvalues (column vectors)
    pub vectors: Vec<Vec<f64>>,
}

/// 用 Jacobi 旋转法分解实对称矩阵 / Decompose a real symmetric matrix with Jacobi rotations
///
/// 返回 `None` 表示输入不是方阵或包含非有限值。
/// Returns `None` when the input is not square or contains non-finite values.
pub fn symmetric_eigen(matrix: &[Vec<f64>]) -> Option<SymmetricEigen> {
    let dimension = matrix.len();
    if dimension == 0 {
        return None;
    }
    if matrix.iter().any(|row| row.len() != dimension) {
        return None;
    }
    if matrix
        .iter()
        .flat_map(|row| row.iter())
        .any(|value| !value.is_finite())
    {
        return None;
    }

    let mut a: Vec<Vec<f64>> = matrix.to_vec();
    let mut v: Vec<Vec<f64>> = (0..dimension)
        .map(|i| {
            (0..dimension)
                .map(|j| if i == j { 1.0 } else { 0.0 })
                .collect()
        })
        .collect();

    let max_sweeps = 100;
    for _ in 0..max_sweeps {
        let mut off_diagonal = 0.0;
        for i in 0..dimension {
            for j in (i + 1)..dimension {
                off_diagonal += a[i][j] * a[i][j];
            }
        }
        if off_diagonal.sqrt() <= 1.0e-12 {
            break;
        }

        for p in 0..dimension {
            for q in (p + 1)..dimension {
                if a[p][q].abs() <= 1.0e-18 {
                    continue;
                }
                let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
                let t = if theta >= 0.0 {
                    1.0 / (theta + (1.0 + theta * theta).sqrt())
                } else {
                    -1.0 / (-theta + (1.0 + theta * theta).sqrt())
                };
                let c = 1.0 / (1.0 + t * t).sqrt();
                let s = t * c;

                for k in 0..dimension {
                    let akp = a[k][p];
                    let akq = a[k][q];
                    a[k][p] = c * akp - s * akq;
                    a[k][q] = s * akp + c * akq;
                }
                for k in 0..dimension {
                    let apk = a[p][k];
                    let aqk = a[q][k];
                    a[p][k] = c * apk - s * aqk;
                    a[q][k] = s * apk + c * aqk;
                }
                for k in 0..dimension {
                    let vkp = v[k][p];
                    let vkq = v[k][q];
                    v[k][p] = c * vkp - s * vkq;
                    v[k][q] = s * vkp + c * vkq;
                }
            }
        }
    }

    let values: Vec<f64> = (0..dimension).map(|i| a[i][i]).collect();
    let mut order: Vec<usize> = (0..dimension).collect();
    order.sort_by(|lhs, rhs| {
        values[*rhs]
            .partial_cmp(&values[*lhs])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let sorted_values = order.iter().map(|index| values[*index]).collect();
    let sorted_vectors = (0..dimension)
        .map(|row| order.iter().map(|index| v[row][*index]).collect())
        .collect();

    Some(SymmetricEigen {
        values: sorted_values,
        vectors: sorted_vectors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagonal_matrix_yields_sorted_eigenvalues() {
        let eigen = symmetric_eigen(&[vec![3.0, 0.0], vec![0.0, 1.0]])
            .expect("diagonal matrix is decomposable");
        assert!((eigen.values[0] - 3.0).abs() < 1e-9);
        assert!((eigen.values[1] - 1.0).abs() < 1e-9);
        assert!((eigen.vectors[0][0] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn rotation_matrix_recovers_known_eigenvalues() {
        let eigen = symmetric_eigen(&[vec![2.0, 1.0], vec![1.0, 2.0]])
            .expect("symmetric matrix is decomposable");
        assert!((eigen.values[0] - 3.0).abs() < 1e-9);
        assert!((eigen.values[1] - 1.0).abs() < 1e-9);
        assert!(symmetric_eigen(&[vec![1.0, 2.0], vec![3.0, 4.0]])
            .is_some());
        assert!(symmetric_eigen(&[vec![f64::NAN]]).is_none());
        assert!(symmetric_eigen(&[vec![1.0, 2.0]]).is_none());
    }
}
