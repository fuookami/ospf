//! Intermediate-model LP export helpers.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::thread;
use crate::model::ConstraintRelation;
use crate::model::ObjectiveCategory;
use crate::variable::VariableType;
use super::{

    BasicLinearTriadModel, BasicQuadraticTetradModel, LinearTriadModel, QuadraticTetradModel,
    SparseMatrix, SparseVector,
};

/// Unified LP export interface for intermediate models.
/// 模型文件格式 / Model file format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelFileFormat {
    /// LP 格式 / LP format
    Lp,
}

impl ModelFileFormat {
    /// 默认扩展名 / Default extension
    pub fn extension(self) -> &'static str {
        match self {
            Self::Lp => "lp",
        }
    }
}

/// 中间模型转储选项 / Intermediate-model dump options
#[derive(Debug, Clone, PartialEq)]
pub struct DumpOptions {
    /// 是否允许并发转储 / Whether concurrent dumping is allowed
    pub concurrent: bool,
    /// 是否输出变量边界 / Whether variable bounds are emitted
    pub include_bounds: bool,
    /// 是否强制输出默认边界 / Whether default bounds are forced into output
    pub force_bounds: bool,
    /// 固定变量映射（solver index -> value）/ Fixed-variable map (solver index -> value)
    pub fixed_variables: BTreeMap<usize, f64>,
}

impl DumpOptions {
    /// 创建默认转储选项 / Create default dump options
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置是否允许并发转储 / Set whether concurrent dumping is allowed
    pub fn with_concurrent(mut self, concurrent: bool) -> Self {
        self.concurrent = concurrent;
        self
    }

    /// 设置是否输出变量边界 / Set whether variable bounds are emitted
    pub fn with_bounds(mut self, include_bounds: bool) -> Self {
        self.include_bounds = include_bounds;
        self
    }

    /// 设置是否强制输出默认边界 / Set whether default bounds are forced into output
    pub fn with_force_bounds(mut self, force_bounds: bool) -> Self {
        self.force_bounds = force_bounds;
        self
    }

    /// 添加一个固定变量 / Add one fixed variable
    pub fn with_fixed_variable(mut self, variable_index: usize, value: f64) -> Self {
        self.fixed_variables.insert(variable_index, value);
        self
    }

    /// 替换固定变量映射 / Replace fixed-variable map
    pub fn with_fixed_variables<I>(mut self, fixed_variables: I) -> Self
    where
        I: IntoIterator<Item = (usize, f64)>,
    {
        self.fixed_variables = fixed_variables.into_iter().collect();
        self
    }

    /// 清空固定变量映射 / Clear fixed-variable map
    pub fn without_fixed_variables(mut self) -> Self {
        self.fixed_variables.clear();
        self
    }
}

impl Default for DumpOptions {
    fn default() -> Self {
        Self {
            concurrent: false,
            include_bounds: true,
            force_bounds: true,
            fixed_variables: BTreeMap::new(),
        }
    }
}

fn path_io_error(path: &Path, err: io::Error) -> io::Error {
    io::Error::new(
        err.kind(),
        format!("failed to dump `{}`: {err}", path.display()),
    )
}

/// 批量导出 LP 文件；当 `options.concurrent = true` 时并发写入。
/// Batch-export LP files; writes concurrently when `options.concurrent = true`.
pub fn dump_lp_batch<'a, M, I, P>(items: I, options: &DumpOptions) -> io::Result<()>
where
    M: LPExportableModel + Sync + 'a,
    I: IntoIterator<Item = (&'a M, P)>,
    P: AsRef<Path>,
{
    let jobs: Vec<(&'a M, PathBuf)> = items
        .into_iter()
        .map(|(model, path)| (model, path.as_ref().to_path_buf()))
        .collect();

    if !options.concurrent || jobs.len() <= 1 {
        for (model, path) in jobs {
            model
                .export_lp_with_options(&path, options)
                .map_err(|err| path_io_error(&path, err))?;
        }
        return Ok(());
    }

    thread::scope(|scope| {
        let handles: Vec<_> = jobs
            .into_iter()
            .map(|(model, path)| {
                let options = options.clone();
                scope.spawn(move || {
                    model
                        .export_lp_with_options(&path, &options)
                        .map_err(|err| path_io_error(&path, err))
                })
            })
            .collect();

        for handle in handles {
            handle
                .join()
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "dump thread panicked"))??;
        }
        Ok(())
    })
}

/// 批量导出模型文件；当 `options.concurrent = true` 时并发写入。
/// Batch-export model files; writes concurrently when `options.concurrent = true`.
pub fn dump_batch<'a, M, I, P>(
    items: I,
    format: ModelFileFormat,
    options: &DumpOptions,
) -> io::Result<()>
where
    M: LPExportableModel + Sync + 'a,
    I: IntoIterator<Item = (&'a M, P)>,
    P: AsRef<Path>,
{
    match format {
        ModelFileFormat::Lp => dump_lp_batch(items, options),
    }
}

/// 统一 LP 导出接口 / Unified LP export interface
pub trait LPExportableModel {
    /// Serialize model into LP-format text with dump options.
    /// 使用转储选项序列化为 LP 格式文本。
    fn to_lp_string_with_options(&self, options: &DumpOptions) -> String;

    /// Serialize model into LP-format text.
    /// 序列化为 LP 格式文本。
    fn to_lp_string(&self) -> String {
        self.to_lp_string_with_options(&DumpOptions::default())
    }

    /// Write LP-format text to writer.
    /// 将 LP 格式文本写入 writer。
    fn write_lp_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(self.to_lp_string().as_bytes())
    }

    /// Write LP-format text to writer with dump options.
    /// 使用转储选项将 LP 格式文本写入 writer。
    fn write_lp_to_with_options<W: Write>(
        &self,
        writer: &mut W,
        options: &DumpOptions,
    ) -> io::Result<()> {
        writer.write_all(self.to_lp_string_with_options(options).as_bytes())
    }

    /// Write LP-format text to file.
    /// 将 LP 格式文本写入文件。
    fn write_lp<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        fs::write(path, self.to_lp_string())
    }

    /// Write LP-format text to file with dump options.
    /// 使用转储选项将 LP 格式文本写入文件。
    fn write_lp_with_options<P: AsRef<Path>>(
        &self,
        path: P,
        options: &DumpOptions,
    ) -> io::Result<()> {
        fs::write(path, self.to_lp_string_with_options(options))
    }

    /// Export LP-format text to file.
    /// 导出 LP 格式文本到文件。
    fn export_lp<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        self.write_lp(path)
    }

    /// Export LP-format text to file with dump options.
    /// 使用转储选项导出 LP 格式文本到文件。
    fn export_lp_with_options<P: AsRef<Path>>(
        &self,
        path: P,
        options: &DumpOptions,
    ) -> io::Result<()> {
        self.write_lp_with_options(path, options)
    }

    /// Export model to file with selected format.
    /// 按指定格式导出模型到文件。
    fn export<P: AsRef<Path>>(&self, path: P, format: ModelFileFormat) -> io::Result<()> {
        match format {
            ModelFileFormat::Lp => self.export_lp(path),
        }
    }

    /// Export model to file with selected format and dump options.
    /// 使用转储选项按指定格式导出模型到文件。
    fn export_with_options<P: AsRef<Path>>(
        &self,
        path: P,
        format: ModelFileFormat,
        options: &DumpOptions,
    ) -> io::Result<()> {
        match format {
            ModelFileFormat::Lp => self.export_lp_with_options(path, options),
        }
    }

    /// Dump LP-format text to file.
    /// 转储 LP 格式文本到文件。
    fn dump_lp<P: AsRef<Path>>(&self, path: P, options: &DumpOptions) -> io::Result<()> {
        self.export_lp_with_options(path, options)
    }

    /// Dump model to file with selected format.
    /// 按指定格式转储模型到文件。
    fn dump<P: AsRef<Path>>(
        &self,
        path: P,
        format: ModelFileFormat,
        options: &DumpOptions,
    ) -> io::Result<()> {
        self.export_with_options(path, format, options)
    }
}

fn relation_to_str(relation: ConstraintRelation) -> &'static str {
    match relation {
        ConstraintRelation::LessEqual => "<=",
        ConstraintRelation::Equal => "=",
        ConstraintRelation::GreaterEqual => ">=",
    }
}

fn variable_name(index: usize) -> String {
    format!("x{index}")
}

fn format_number(value: f64) -> String {
    if value.is_infinite() {
        if value.is_sign_positive() {
            "+inf".to_string()
        } else {
            "-inf".to_string()
        }
    } else if value.abs() <= f64::EPSILON {
        "0".to_string()
    } else {
        let mut text = format!("{value:.12}");
        while text.contains('.') && text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
        if text.is_empty() {
            "0".to_string()
        } else {
            text
        }
    }
}

fn collect_sparse_row_terms(row: &SparseVector<f64>) -> Vec<(usize, f64)> {
    let mut combined: BTreeMap<usize, f64> = BTreeMap::new();
    for (index, coefficient) in &row.entries {
        let entry = combined.entry(*index).or_insert(0.0);
        *entry += *coefficient;
    }
    combined
        .into_iter()
        .filter(|(_, value)| value.abs() > f64::EPSILON)
        .collect()
}

fn combine_linear_terms(terms: impl IntoIterator<Item = (usize, f64)>) -> Vec<(usize, f64)> {
    let mut combined: BTreeMap<usize, f64> = BTreeMap::new();
    for (index, coefficient) in terms {
        let entry = combined.entry(index).or_insert(0.0);
        *entry += coefficient;
    }
    combined
        .into_iter()
        .filter(|(_, value)| value.abs() > f64::EPSILON)
        .collect()
}

fn apply_fixed_to_linear_terms(
    terms: &[(usize, f64)],
    options: &DumpOptions,
) -> (Vec<(usize, f64)>, f64) {
    let mut active_terms = Vec::new();
    let mut constant = 0.0;
    for (index, coefficient) in terms {
        if let Some(value) = options.fixed_variables.get(index) {
            constant += coefficient * value;
        } else {
            active_terms.push((*index, *coefficient));
        }
    }
    (combine_linear_terms(active_terms), constant)
}

fn apply_fixed_to_quadratic_terms(
    terms: &[(usize, Option<usize>, f64)],
    options: &DumpOptions,
) -> (Vec<(usize, Option<usize>, f64)>, Vec<(usize, f64)>, f64) {
    let mut quadratic_terms = Vec::new();
    let mut linear_terms = Vec::new();
    let mut constant = 0.0;

    for (var_index1, var_index2, coefficient) in terms {
        let fixed1 = options.fixed_variables.get(var_index1).copied();
        if let Some(var_index2) = var_index2 {
            let fixed2 = options.fixed_variables.get(var_index2).copied();
            match (fixed1, fixed2) {
                (Some(value1), Some(value2)) => {
                    constant += coefficient * value1 * value2;
                }
                (Some(value1), None) => {
                    linear_terms.push((*var_index2, coefficient * value1));
                }
                (None, Some(value2)) => {
                    linear_terms.push((*var_index1, coefficient * value2));
                }
                (None, None) => {
                    quadratic_terms.push((*var_index1, Some(*var_index2), *coefficient));
                }
            }
        } else if let Some(value) = fixed1 {
            constant += coefficient * value;
        } else {
            linear_terms.push((*var_index1, *coefficient));
        }
    }

    (
        quadratic_terms,
        combine_linear_terms(linear_terms),
        constant,
    )
}

fn format_linear_expression(terms: &[(usize, f64)], var_names: &[String], constant: f64) -> String {
    let mut output = String::new();
    let mut has_any = false;

    for (index, coefficient) in terms {
        if coefficient.abs() <= f64::EPSILON {
            continue;
        }
        let abs = coefficient.abs();
        if has_any {
            output.push_str(if *coefficient >= 0.0 { " + " } else { " - " });
        } else if *coefficient < 0.0 {
            output.push('-');
        }
        output.push_str(&format!("{} {}", format_number(abs), var_names[*index]));
        has_any = true;
    }

    if constant.abs() > f64::EPSILON {
        if has_any {
            output.push_str(if constant >= 0.0 { " + " } else { " - " });
            output.push_str(&format_number(constant.abs()));
        } else {
            output.push_str(&format_number(constant));
            has_any = true;
        }
    }

    if has_any { output } else { "0".to_string() }
}

fn collect_quadratic_objective_terms(
    matrix: &SparseMatrix<f64>,
) -> Vec<(usize, Option<usize>, f64)> {
    let mut terms = Vec::new();
    for (row_index, row) in matrix.rows.iter().enumerate() {
        for (col_index, coefficient) in &row.entries {
            if coefficient.abs() <= f64::EPSILON {
                continue;
            }
            terms.push((row_index, Some(*col_index), *coefficient));
        }
    }
    terms
}

fn format_quadratic_terms(terms: &[(usize, Option<usize>, f64)], var_names: &[String]) -> String {
    let mut output = String::new();
    let mut has_any = false;

    for (var_index1, var_index2, coefficient) in terms {
        if coefficient.abs() <= f64::EPSILON {
            continue;
        }
        let abs = coefficient.abs();
        if has_any {
            output.push_str(if *coefficient >= 0.0 { " + " } else { " - " });
        } else if *coefficient < 0.0 {
            output.push('-');
        }

        if let Some(var_index2) = var_index2 {
            if *var_index2 == *var_index1 {
                output.push_str(&format!(
                    "{} {} ^ 2",
                    format_number(abs),
                    var_names[*var_index1]
                ));
            } else {
                output.push_str(&format!(
                    "{} {} * {}",
                    format_number(abs),
                    var_names[*var_index1],
                    var_names[*var_index2]
                ));
            }
        } else {
            output.push_str(&format!(
                "{} {}",
                format_number(abs),
                var_names[*var_index1]
            ));
        }
        has_any = true;
    }

    if has_any { output } else { "0".to_string() }
}

fn format_mixed_expression(
    linear_terms: &[(usize, f64)],
    quadratic_terms: &[(usize, Option<usize>, f64)],
    var_names: &[String],
    constant: f64,
) -> String {
    let linear = format_linear_expression(linear_terms, var_names, constant);
    let quadratic = format_quadratic_terms(quadratic_terms, var_names);
    if quadratic == "0" {
        return linear;
    }
    if linear == "0" {
        return format!("[ {quadratic} ]");
    }
    format!("{linear} + [ {quadratic} ]")
}

fn append_bounds_and_domains(
    output: &mut String,
    var_names: &[String],
    lb: &[f64],
    ub: &[f64],
    var_types: &[VariableType],
    options: &DumpOptions,
) {
    if options.include_bounds {
        let mut bound_lines = String::new();
        for index in 0..var_names.len() {
            if options.fixed_variables.contains_key(&index) {
                continue;
            }
            let lower = lb[index];
            let upper = ub[index];
            if !options.force_bounds && lower == 0.0 && upper.is_infinite() {
                continue;
            }

            let name = &var_names[index];
            if lower.is_infinite() && upper.is_infinite() {
                bound_lines.push_str(&format!("  {name} free\n"));
            } else if lower.is_infinite() {
                bound_lines.push_str(&format!("  {name} <= {}\n", format_number(upper)));
            } else if upper.is_infinite() {
                bound_lines.push_str(&format!("  {} <= {name}\n", format_number(lower)));
            } else {
                bound_lines.push_str(&format!(
                    "  {} <= {name} <= {}\n",
                    format_number(lower),
                    format_number(upper)
                ));
            }
        }
        if !bound_lines.is_empty() {
            output.push_str("Bounds\n");
            output.push_str(&bound_lines);
        }
    }

    let binaries: Vec<&str> = var_types
        .iter()
        .enumerate()
        .filter_map(|(index, variable_type)| {
            if *variable_type == VariableType::Binary
                && !options.fixed_variables.contains_key(&index)
            {
                Some(var_names[index].as_str())
            } else {
                None
            }
        })
        .collect();
    if !binaries.is_empty() {
        output.push_str("Binaries\n  ");
        output.push_str(&binaries.join(" "));
        output.push('\n');
    }

    let generals: Vec<&str> = var_types
        .iter()
        .enumerate()
        .filter_map(|(index, variable_type)| match variable_type {
            VariableType::Binary => None,
            VariableType::Ternary
            | VariableType::BalancedTernary
            | VariableType::Integer
            | VariableType::UInteger => {
                if options.fixed_variables.contains_key(&index) {
                    None
                } else {
                    Some(var_names[index].as_str())
                }
            }
            _ => None,
        })
        .collect();
    if !generals.is_empty() {
        output.push_str("Generals\n  ");
        output.push_str(&generals.join(" "));
        output.push('\n');
    }
}

impl LPExportableModel for LinearTriadModel {
    fn to_lp_string_with_options(&self, options: &DumpOptions) -> String {
        let var_names: Vec<String> = (0..self.num_variables()).map(variable_name).collect();
        let mut output = String::new();

        output.push_str(match self.objective_category {
            ObjectiveCategory::Minimum => "Minimize\n",
            ObjectiveCategory::Maximum => "Maximize\n",
        });
        let objective_terms: Vec<(usize, f64)> = self
            .c
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, coefficient)| coefficient.abs() > f64::EPSILON)
            .collect();
        let (objective_terms, objective_constant) =
            apply_fixed_to_linear_terms(&objective_terms, options);
        output.push_str(&format!(
            "  obj: {}\n",
            format_linear_expression(&objective_terms, &var_names, objective_constant)
        ));

        output.push_str("Subject To\n");
        for row_index in 0..self.A.rows.len() {
            let row_terms = collect_sparse_row_terms(&self.A.rows[row_index]);
            let (row_terms, row_constant) = apply_fixed_to_linear_terms(&row_terms, options);
            let name = self
                .constraint_names
                .get(row_index)
                .cloned()
                .unwrap_or_else(|| format!("c{row_index}"));
            let rhs = self.b.get(row_index).copied().unwrap_or(0.0);
            output.push_str(&format!(
                "  {name}: {} <= {}\n",
                format_linear_expression(&row_terms, &var_names, row_constant),
                format_number(rhs)
            ));
        }

        append_bounds_and_domains(
            &mut output,
            &var_names,
            &self.lb,
            &self.ub,
            &self.var_types,
            options,
        );
        output.push_str("End\n");
        output
    }
}

impl LPExportableModel for BasicLinearTriadModel {
    fn to_lp_string_with_options(&self, options: &DumpOptions) -> String {
        LinearTriadModel::from_basic(self.clone()).to_lp_string_with_options(options)
    }
}

impl LPExportableModel for QuadraticTetradModel {
    fn to_lp_string_with_options(&self, options: &DumpOptions) -> String {
        let var_names: Vec<String> = (0..self.num_variables()).map(variable_name).collect();
        let mut output = String::new();

        output.push_str(match self.objective_category {
            ObjectiveCategory::Minimum => "Minimize\n",
            ObjectiveCategory::Maximum => "Maximize\n",
        });
        let objective_linear_terms: Vec<(usize, f64)> = self
            .c
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, coefficient)| coefficient.abs() > f64::EPSILON)
            .collect();
        let (objective_linear_terms, objective_linear_constant) =
            apply_fixed_to_linear_terms(&objective_linear_terms, options);
        let objective_quadratic_terms = collect_quadratic_objective_terms(&self.Q);
        let (objective_quadratic_terms, objective_fixed_linear_terms, objective_quadratic_constant) =
            apply_fixed_to_quadratic_terms(&objective_quadratic_terms, options);
        let objective_linear_terms = combine_linear_terms(
            objective_linear_terms
                .into_iter()
                .chain(objective_fixed_linear_terms),
        );
        output.push_str(&format!(
            "  obj: {}\n",
            format_mixed_expression(
                &objective_linear_terms,
                &objective_quadratic_terms,
                &var_names,
                objective_linear_constant + objective_quadratic_constant
            )
        ));

        output.push_str("Subject To\n");
        for row_index in 0..self.basic.linear.A.rows.len() {
            let row_terms = collect_sparse_row_terms(&self.basic.linear.A.rows[row_index]);
            let (row_terms, row_constant) = apply_fixed_to_linear_terms(&row_terms, options);
            let name = self
                .basic
                .linear
                .constraint_names
                .get(row_index)
                .cloned()
                .unwrap_or_else(|| format!("c{row_index}"));
            let rhs = self.basic.linear.b.get(row_index).copied().unwrap_or(0.0);
            output.push_str(&format!(
                "  {name}: {} <= {}\n",
                format_linear_expression(&row_terms, &var_names, row_constant),
                format_number(rhs)
            ));
        }

        for (quadratic_index, inequality) in self.quadratic_constraints.iter().enumerate() {
            let mut linear_terms = Vec::new();
            let mut quadratic_terms = Vec::new();
            for monomial in inequality.polynomial.monomials() {
                if let Some(var_index2) = monomial.var_index2() {
                    quadratic_terms.push((
                        monomial.var_index1(),
                        Some(var_index2),
                        *monomial.coefficient(),
                    ));
                } else {
                    linear_terms.push((monomial.var_index1(), *monomial.coefficient()));
                }
            }
            let (linear_terms, linear_constant) =
                apply_fixed_to_linear_terms(&linear_terms, options);
            let (quadratic_terms, fixed_linear_terms, quadratic_constant) =
                apply_fixed_to_quadratic_terms(&quadratic_terms, options);
            let linear_terms =
                combine_linear_terms(linear_terms.into_iter().chain(fixed_linear_terms));
            let lhs = format_mixed_expression(
                &linear_terms,
                &quadratic_terms,
                &var_names,
                *inequality.polynomial.constant() + linear_constant + quadratic_constant,
            );
            let relation = relation_to_str(inequality.relation);
            let rhs = format_number(inequality.rhs);
            let name = self
                .quadratic_constraint_names
                .get(quadratic_index)
                .cloned()
                .unwrap_or_else(|| format!("qc{quadratic_index}"));
            output.push_str(&format!("  {name}: {lhs} {relation} {rhs}\n"));
        }

        append_bounds_and_domains(
            &mut output,
            &var_names,
            &self.basic.linear.lb,
            &self.basic.linear.ub,
            &self.basic.linear.var_types,
            options,
        );
        output.push_str("End\n");
        output
    }
}

impl LPExportableModel for BasicQuadraticTetradModel {
    fn to_lp_string_with_options(&self, options: &DumpOptions) -> String {
        QuadraticTetradModel::from_basic(self.clone()).to_lp_string_with_options(options)
    }
}

#[cfg(test)]
mod tests {
    use super::LPExportableModel;
    use super::*;
    use crate::model::QuadraticInequality;
    use crate::model::{ConstraintRelation, ObjectiveCategory};
    use crate::token::Token;
    use crate::variable::{BinaryVariableItem, UContinuousVariableItem, VariableType};

    #[test]
    fn linear_model_lp_export_contains_expected_sections() {
        let mut basic = BasicLinearTriadModel::new("lp_linear");
        basic.add_variable_with_bounds(
            Token::from_generic(BinaryVariableItem::auto("x"), 0),
            0.0,
            1.0,
            VariableType::Binary,
        );
        basic.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("y"), 1),
            0.0,
            f64::INFINITY,
            VariableType::UContinuous,
        );
        let mut row = SparseVector::new();
        row.add(0, 1.0);
        row.add(1, 2.0);
        basic.add_constraint_with_metadata(row, 3.0, "cap".to_string(), None, false, 0, None, None);

        let mut model = LinearTriadModel::from_basic(basic);
        model.set_objective(vec![1.0, -2.0], ObjectiveCategory::Maximum);

        let lp = model.to_lp_string();
        assert!(lp.contains("Maximize"));
        assert!(lp.contains("Subject To"));
        assert!(lp.contains("Bounds"));
        assert!(lp.contains("Binaries"));
        assert!(lp.contains("cap:"));
        assert!(lp.contains("x0"));
    }

    #[test]
    fn linear_model_export_helpers_write_lp_text() {
        let mut basic = BasicLinearTriadModel::new("lp_export_helpers");
        basic.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("x"), 0),
            0.0,
            f64::INFINITY,
            VariableType::UContinuous,
        );
        let model = LinearTriadModel::from_basic(basic);

        let mut bytes = Vec::new();
        model
            .write_lp_to(&mut bytes)
            .expect("write LP text to memory");
        let text = String::from_utf8(bytes).expect("LP text should be UTF-8");
        assert!(text.contains("Minimize"));
        assert_eq!(ModelFileFormat::Lp.extension(), "lp");

        let path = std::env::temp_dir().join(format!(
            "ospf_rust_core_lp_export_helpers_{}.lp",
            std::process::id()
        ));
        model
            .export(&path, ModelFileFormat::Lp)
            .expect("export LP file");
        let exported = std::fs::read_to_string(&path).expect("read exported LP");
        let _ = std::fs::remove_file(&path);
        assert!(exported.contains("Bounds"));
    }

    #[test]
    fn dump_lp_batch_uses_concurrent_option_for_multiple_files() {
        let mut basic_a = BasicLinearTriadModel::new("lp_batch_a");
        basic_a.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("x_a"), 0),
            0.0,
            f64::INFINITY,
            VariableType::UContinuous,
        );
        let model_a = LinearTriadModel::from_basic(basic_a);

        let mut basic_b = BasicLinearTriadModel::new("lp_batch_b");
        basic_b.add_variable_with_bounds(
            Token::from_generic(BinaryVariableItem::auto("x_b"), 0),
            0.0,
            1.0,
            VariableType::Binary,
        );
        let model_b = LinearTriadModel::from_basic(basic_b);

        let dir = std::env::temp_dir();
        let path_a = dir.join(format!(
            "ospf_rust_core_lp_batch_a_{}.lp",
            std::process::id()
        ));
        let path_b = dir.join(format!(
            "ospf_rust_core_lp_batch_b_{}.lp",
            std::process::id()
        ));
        let options = DumpOptions::new().with_concurrent(true);

        dump_lp_batch([(&model_a, &path_a), (&model_b, &path_b)], &options)
            .expect("batch LP dump should succeed");

        let exported_a = std::fs::read_to_string(&path_a).expect("read first batch LP");
        let exported_b = std::fs::read_to_string(&path_b).expect("read second batch LP");
        let _ = std::fs::remove_file(&path_a);
        let _ = std::fs::remove_file(&path_b);

        assert!(exported_a.contains("Minimize"));
        assert!(exported_b.contains("Binaries"));
    }

    #[test]
    fn dump_options_can_compact_or_suppress_bounds() {
        let mut basic = BasicLinearTriadModel::new("lp_dump_options_bounds");
        basic.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("x"), 0),
            0.0,
            f64::INFINITY,
            VariableType::UContinuous,
        );
        basic.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("y"), 1),
            -1.0,
            3.0,
            VariableType::Continuous,
        );
        let model = LinearTriadModel::from_basic(basic);

        let compact = model.to_lp_string_with_options(&DumpOptions::new().with_force_bounds(false));
        assert!(!compact.contains("0 <= x0"));
        assert!(compact.contains("-1 <= x1 <= 3"));

        let no_bounds = model.to_lp_string_with_options(&DumpOptions::new().with_bounds(false));
        assert!(!no_bounds.contains("Bounds"));
    }

    #[test]
    fn dump_options_substitute_fixed_variables_in_linear_lp() {
        let mut basic = BasicLinearTriadModel::new("lp_fixed_linear");
        basic.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("x"), 0),
            0.0,
            f64::INFINITY,
            VariableType::UContinuous,
        );
        basic.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("y"), 1),
            0.0,
            f64::INFINITY,
            VariableType::UContinuous,
        );
        let mut row = SparseVector::new();
        row.add(0, 1.0);
        row.add(1, 1.0);
        basic.add_constraint(row, 5.0);
        let mut model = LinearTriadModel::from_basic(basic);
        model.set_objective(vec![1.0, 2.0], ObjectiveCategory::Minimum);

        let lp = model.to_lp_string_with_options(&DumpOptions::new().with_fixed_variable(0, 2.0));
        assert!(lp.contains("obj: 2 x1 + 2"));
        assert!(lp.contains("c0: 1 x1 + 2 <= 5"));
        assert!(!lp.contains("x0"));
    }

    #[test]
    fn quadratic_model_lp_export_contains_quadratic_sections() {
        let mut basic = BasicQuadraticTetradModel::new("lp_quadratic");
        basic.linear.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("x"), 0),
            0.0,
            f64::INFINITY,
            VariableType::UContinuous,
        );
        basic.linear.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("y"), 1),
            0.0,
            f64::INFINITY,
            VariableType::UContinuous,
        );

        let mut model = QuadraticTetradModel::from_basic(basic);
        let mut q = SparseMatrix::new();
        q.add_row(SparseVector::new());
        q.add_row(SparseVector::new());
        q.rows[0].add(0, 2.0);
        q.rows[0].add(1, 1.5);
        model.set_objective(vec![1.0, 0.0], q, ObjectiveCategory::Minimum);

        model.add_quadratic_constraint_with_metadata(
            QuadraticInequality::new(
                crate::symbol::flatten::Quadratic::new(
                    vec![
                        crate::symbol::flatten::QuadraticMonomial::new_quadratic(1.0, 0, 0),
                        crate::symbol::flatten::QuadraticMonomial::new_linear(-1.0, 1),
                    ],
                    0.5,
                ),
                ConstraintRelation::LessEqual,
                2.0,
            ),
            "qc_cap".to_string(),
            None,
            false,
            0,
            None,
            None,
        );

        let lp = model.to_lp_string();
        assert!(lp.contains("Minimize"));
        assert!(lp.contains("obj:"));
        assert!(lp.contains("["));
        assert!(lp.contains("qc_cap:"));
        assert!(lp.contains("^ 2"));
    }

    #[test]
    fn dump_options_substitute_fixed_variables_in_quadratic_lp() {
        let mut basic = BasicQuadraticTetradModel::new("lp_fixed_quadratic");
        basic.linear.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("x"), 0),
            0.0,
            f64::INFINITY,
            VariableType::UContinuous,
        );
        basic.linear.add_variable_with_bounds(
            Token::from_generic(UContinuousVariableItem::auto("y"), 1),
            0.0,
            f64::INFINITY,
            VariableType::UContinuous,
        );

        let mut model = QuadraticTetradModel::from_basic(basic);
        let mut q = SparseMatrix::new();
        q.add_row(SparseVector::new());
        q.add_row(SparseVector::new());
        q.rows[0].add(1, 3.0);
        q.rows[1].add(1, 2.0);
        model.set_objective(vec![1.0, 0.0], q, ObjectiveCategory::Minimum);

        let lp = model.to_lp_string_with_options(&DumpOptions::new().with_fixed_variable(0, 2.0));
        assert!(lp.contains("obj: 6 x1 + 2 + [ 2 x1 ^ 2 ]"));
        assert!(!lp.contains("x0"));
    }
}
