//! Run SQL against a Parquet file using polars-sql.

use polars::prelude::*;
use polars_sql::SQLContext;

use crate::file::sample_data::ParquetSampleData;

/// Result of a SQL query: either success data (same shape as ParquetSampleData) or an error message.
#[derive(Debug, Clone)]
pub enum SqlResult {
    Ok(ParquetSampleData),
    Err(String),
}

/// Execute a SQL query against the Parquet file at `path`.
/// The table is registered as "parquet". Returns result data or an error string.
pub fn run_sql(path: &str, query: &str) -> SqlResult {
    if query.trim().is_empty() {
        return SqlResult::Err("Empty query".to_string());
    }
    let lf = match LazyFrame::scan_parquet(PlPath::new(path), Default::default()) {
        Ok(lf) => lf,
        Err(e) => return SqlResult::Err(e.to_string()),
    };
    let mut ctx = SQLContext::new();
    ctx.register("parquet", lf);
    let result_lf = match ctx.execute(query) {
        Ok(lf) => lf,
        Err(e) => return SqlResult::Err(e.to_string()),
    };
    let df = match result_lf.collect() {
        Ok(df) => df,
        Err(e) => return SqlResult::Err(e.to_string()),
    };
    dataframe_to_sample_data(df)
        .map(SqlResult::Ok)
        .unwrap_or_else(SqlResult::Err)
}

fn dataframe_to_sample_data(df: DataFrame) -> Result<ParquetSampleData, String> {
    let flattened_columns: Vec<String> = df
        .get_column_names()
        .iter()
        .map(|s| s.to_string())
        .collect();
    let total_columns = flattened_columns.len();
    let mut rows = Vec::new();
    for row_idx in 0..df.height() {
        let mut row = Vec::new();
        for col in df.get_columns() {
            let series = col.as_materialized_series();
            row.push(get_value_as_string(series, row_idx));
        }
        rows.push(row);
    }
    Ok(ParquetSampleData {
        flattened_columns,
        total_columns,
        total_rows: df.height(),
        rows,
    })
}

fn get_value_as_string(col: &Series, row_idx: usize) -> String {
    match col.get(row_idx) {
        Ok(any_value) => {
            if any_value.is_null() {
                "NULL".to_string()
            } else {
                format!("{any_value}")
            }
        }
        Err(_) => "NULL".to_string(),
    }
}
