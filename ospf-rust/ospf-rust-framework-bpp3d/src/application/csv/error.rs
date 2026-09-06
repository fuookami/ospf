/// CSV 数据集错误 / CSV dataset error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsvDatasetError {
    /// 缺少表 / Missing table
    MissingTable {
        /// 表名 / Table name
        table: String,
    },
    /// 未知表 / Unknown table
    UnknownTable {
        /// 表名 / Table name
        table: String,
    },
    /// 缺少必需列 / Missing required column
    MissingRequiredColumn {
        /// 表名 / Table name
        table: String,
        /// 列名 / Column name
        column: String,
    },
    /// 未知列 / Unknown column
    UnknownColumn {
        /// 表名 / Table name
        table: String,
        /// 列名 / Column name
        column: String,
    },
    /// 重复列 / Duplicated column
    DuplicatedColumn {
        /// 表名 / Table name
        table: String,
        /// 列名 / Column name
        column: String,
    },
    /// CSV 解析错误 / CSV parse error
    Parse {
        /// 表名 / Table name
        table: String,
        /// 信息 / Message
        message: String,
    },
    /// 字段值非法 / Invalid field value
    InvalidValue {
        /// 表名 / Table name
        table: String,
        /// 行号 / Row number
        row: usize,
        /// 字段名 / Field name
        field: String,
        /// 字段值 / Field value
        value: String,
        /// 原因 / Reason
        reason: String,
    },
}

impl Display for CsvDatasetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingTable { table } => {
                write!(f, "CSV dataset missing table `{}`. / CSV 数据集缺少表 `{}`。", table, table)
            }
            Self::UnknownTable { table } => {
                write!(f, "CSV dataset has unknown table `{}`. / CSV 数据集包含未知表 `{}`。", table, table)
            }
            Self::MissingRequiredColumn { table, column } => {
                write!(f, "CSV table `{}` missing required column `{}`. / CSV 表 `{}` 缺少必需列 `{}`。", table, column, table, column)
            }
            Self::UnknownColumn { table, column } => {
                write!(f, "CSV table `{}` has unknown column `{}`. / CSV 表 `{}` 包含未知列 `{}`。", table, column, table, column)
            }
            Self::DuplicatedColumn { table, column } => {
                write!(f, "CSV table `{}` has duplicated column `{}`. / CSV 表 `{}` 包含重复列 `{}`。", table, column, table, column)
            }
            Self::Parse { table, message } => {
                write!(f, "CSV table `{}` parse failed: {}. / CSV 表 `{}` 解析失败：{}。", table, message, table, message)
            }
            Self::InvalidValue { table, row, field, value, reason } => {
                write!(
                    f,
                    "CSV table `{}` row {} field `{}` has invalid value `{}`: {}. / CSV 表 `{}` 第 {} 行字段 `{}` 的值 `{}` 非法：{}。",
                    table, row, field, value, reason, table, row, field, value, reason
                )
            }
        }
    }
}

impl std::error::Error for CsvDatasetError {}

