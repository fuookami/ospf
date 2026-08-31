/// CSV schema guard / CSV schema guard
#[derive(Debug, Clone, Default)]
pub struct CsvSchemaGuard;

impl CsvSchemaGuard {
    /// 校验表头 / Validate header
    pub fn validate(
        table: &str,
        headers: &StringRecord,
        required: &[&str],
        optional: &[&str],
    ) -> Result<(), CsvDatasetError> {
        let mut seen = HashSet::new();
        let allowed: HashSet<&str> = required.iter().chain(optional.iter()).copied().collect();
        for header in headers {
            let column = header.trim();
            if !seen.insert(column.to_string()) {
                return Err(CsvDatasetError::DuplicatedColumn {
                    table: table.to_string(),
                    column: column.to_string(),
                });
            }
            if !allowed.contains(column) {
                return Err(CsvDatasetError::UnknownColumn {
                    table: table.to_string(),
                    column: column.to_string(),
                });
            }
        }
        for column in required {
            if !seen.contains(*column) {
                return Err(CsvDatasetError::MissingRequiredColumn {
                    table: table.to_string(),
                    column: (*column).to_string(),
                });
            }
        }
        Ok(())
    }
}

