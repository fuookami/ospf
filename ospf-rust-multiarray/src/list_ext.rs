/// 二维列表类型别名 / 2D list type alias.
pub type List2<T> = std::vec::Vec<std::vec::Vec<T>>;

/// 三维列表类型别名 / 3D list type alias.
pub type List3<T> = std::vec::Vec<List2<T>>;

/// List2 通配访问扩展 / List2 wildcard access extensions.
pub trait List2Ext<T> {
    /// 双索引读取 / Read by two indices.
    fn get2(&self, i: usize, j: usize) -> &T;

    /// 双索引安全读取 / Safely read by two indices.
    fn get2_opt(&self, i: usize, j: usize) -> Option<&T>;

    /// 双索引可变读取 / Mutably read by two indices.
    fn get2_mut(&mut self, i: usize, j: usize) -> Option<&mut T>;

    /// 双索引写入 / Write by two indices.
    fn set2(&mut self, i: usize, j: usize, value: T) -> Option<T>;

    /// 通配匹配值，None 表示该维度全选 / Wildcard match, None means all on that dimension.
    fn values_match(&self, i: Option<usize>, j: Option<usize>) -> Vec<&T>;

    /// 获取指定列上的全部值 / Get all values at a column.
    fn values_at_col(&self, j: usize) -> Vec<&T> {
        self.values_match(None, Some(j))
    }

    /// 获取指定行上的全部值 / Get all values at a row.
    fn values_at_row(&self, i: usize) -> Vec<&T> {
        self.values_match(Some(i), None)
    }

    /// 获取所有值 / Get all values.
    fn values_all(&self) -> Vec<&T> {
        self.values_match(None, None)
    }
}

impl<T> List2Ext<T> for List2<T> {
    fn get2(&self, i: usize, j: usize) -> &T {
        &self[i][j]
    }

    fn get2_opt(&self, i: usize, j: usize) -> Option<&T> {
        self.get(i)?.get(j)
    }

    fn get2_mut(&mut self, i: usize, j: usize) -> Option<&mut T> {
        self.get_mut(i)?.get_mut(j)
    }

    fn set2(&mut self, i: usize, j: usize, value: T) -> Option<T> {
        let slot = self.get2_mut(i, j)?;
        Some(std::mem::replace(slot, value))
    }

    fn values_match(&self, i: Option<usize>, j: Option<usize>) -> Vec<&T> {
        let mut values = Vec::new();

        for (row_index, row) in self.iter().enumerate() {
            if let Some(expected_row) = i {
                if row_index != expected_row {
                    continue;
                }
            }

            for (col_index, value) in row.iter().enumerate() {
                if let Some(expected_col) = j {
                    if col_index != expected_col {
                        continue;
                    }
                }
                values.push(value);
            }
        }

        values
    }
}

/// List3 通配访问扩展 / List3 wildcard access extensions.
pub trait List3Ext<T> {
    /// 三索引读取 / Read by three indices.
    fn get3(&self, i: usize, j: usize, k: usize) -> &T;

    /// 三索引安全读取 / Safely read by three indices.
    fn get3_opt(&self, i: usize, j: usize, k: usize) -> Option<&T>;

    /// 三索引可变读取 / Mutably read by three indices.
    fn get3_mut(&mut self, i: usize, j: usize, k: usize) -> Option<&mut T>;

    /// 三索引写入 / Write by three indices.
    fn set3(&mut self, i: usize, j: usize, k: usize, value: T) -> Option<T>;

    /// 通配匹配值，None 表示该维度全选 / Wildcard match, None means all on that dimension.
    fn values_match(&self, i: Option<usize>, j: Option<usize>, k: Option<usize>) -> Vec<&T>;

    fn values_at_i(&self, i: usize) -> Vec<&T> {
        self.values_match(Some(i), None, None)
    }

    fn values_at_j(&self, j: usize) -> Vec<&T> {
        self.values_match(None, Some(j), None)
    }

    fn values_at_k(&self, k: usize) -> Vec<&T> {
        self.values_match(None, None, Some(k))
    }

    fn values_all(&self) -> Vec<&T> {
        self.values_match(None, None, None)
    }
}

impl<T> List3Ext<T> for List3<T> {
    fn get3(&self, i: usize, j: usize, k: usize) -> &T {
        &self[i][j][k]
    }

    fn get3_opt(&self, i: usize, j: usize, k: usize) -> Option<&T> {
        self.get(i)?.get(j)?.get(k)
    }

    fn get3_mut(&mut self, i: usize, j: usize, k: usize) -> Option<&mut T> {
        self.get_mut(i)?.get_mut(j)?.get_mut(k)
    }

    fn set3(&mut self, i: usize, j: usize, k: usize, value: T) -> Option<T> {
        let slot = self.get3_mut(i, j, k)?;
        Some(std::mem::replace(slot, value))
    }

    fn values_match(&self, i: Option<usize>, j: Option<usize>, k: Option<usize>) -> Vec<&T> {
        let mut values = Vec::new();

        for (i_index, level2) in self.iter().enumerate() {
            if let Some(expected_i) = i {
                if i_index != expected_i {
                    continue;
                }
            }

            for (j_index, row) in level2.iter().enumerate() {
                if let Some(expected_j) = j {
                    if j_index != expected_j {
                        continue;
                    }
                }

                for (k_index, value) in row.iter().enumerate() {
                    if let Some(expected_k) = k {
                        if k_index != expected_k {
                            continue;
                        }
                    }
                    values.push(value);
                }
            }
        }

        values
    }
}

#[cfg(test)]
mod tests {
    use super::{List2Ext, List3Ext};

    #[test]
    fn test_list2_ext() {
        let mut matrix = vec![vec![1, 2, 3], vec![4, 5, 6]];

        let col1: Vec<i32> = matrix.values_at_col(1).into_iter().copied().collect();
        assert_eq!(col1, vec![2, 5]);

        let row0: Vec<i32> = matrix.values_at_row(0).into_iter().copied().collect();
        assert_eq!(row0, vec![1, 2, 3]);

        assert_eq!(*matrix.get2(1, 2), 6);
        assert_eq!(matrix.get2_opt(1, 2), Some(&6));
        assert_eq!(matrix.get2_opt(2, 0), None);
        assert_eq!(matrix.set2(0, 1, 20), Some(2));
        assert_eq!(matrix.get2_opt(0, 1), Some(&20));

        let all: Vec<i32> = matrix.values_all().into_iter().copied().collect();
        assert_eq!(all, vec![1, 20, 3, 4, 5, 6]);
    }

    #[test]
    fn test_list3_ext() {
        let mut cube = vec![vec![vec![1, 2], vec![3, 4]], vec![vec![5, 6], vec![7, 8]]];

        let i0: Vec<i32> = cube.values_at_i(0).into_iter().copied().collect();
        assert_eq!(i0, vec![1, 2, 3, 4]);

        let j1: Vec<i32> = cube.values_at_j(1).into_iter().copied().collect();
        assert_eq!(j1, vec![3, 4, 7, 8]);

        let k0: Vec<i32> = cube.values_at_k(0).into_iter().copied().collect();
        assert_eq!(k0, vec![1, 3, 5, 7]);

        assert_eq!(*cube.get3(1, 1, 1), 8);
        assert_eq!(cube.get3_opt(1, 1, 1), Some(&8));
        assert_eq!(cube.get3_opt(2, 0, 0), None);
        assert_eq!(cube.set3(0, 1, 0, 30), Some(3));
        assert_eq!(cube.get3_opt(0, 1, 0), Some(&30));

        let all: Vec<i32> = List3Ext::values_all(&cube).into_iter().copied().collect();
        assert_eq!(all, vec![1, 2, 30, 4, 5, 6, 7, 8]);
    }
}
