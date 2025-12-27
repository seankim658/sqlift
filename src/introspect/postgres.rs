use postgres::Client;
use tracing::{debug, error, info, trace};

use super::{Introspector, TableFilter};
use crate::prelude::SqliftError;
use crate::schema::{Column, DataType, EnumType, Schema, Table};

/// PostgreSQL introspector
pub struct PostgresIntrospector<'a> {
    client: &'a mut Client,
}

impl<'a> PostgresIntrospector<'a> {
    pub fn new(client: &'a mut Client) -> Self {
        Self { client }
    }
}

impl Introspector for PostgresIntrospector<'_> {
    fn introspect(
        &mut self,
        schema_name: &str,
        filter: &TableFilter,
    ) -> Result<Schema, SqliftError> {
        info!(schema = ?schema_name, "Starting schema introspection");

        let enums = query_enums(self.client, schema_name)?;
        debug!(count = ?enums.len(), "Found enum types");

        let all_table_names = query_tables(self.client, schema_name)?;
        debug!(count = ?all_table_names.len(), "Found all tables");

        let table_names: Vec<String> = all_table_names
            .into_iter()
            .filter(|name| filter.should_include(name))
            .collect();
        debug!(count = ?table_names.len(), "Tables after filtering");

        let mut tables = Vec::with_capacity(table_names.len());
        for table_name in table_names {
            debug!(table = ?table_name, "Introspecting table");

            let columns = query_columns(self.client, schema_name, &table_name)?;
            trace!(table = ?table_name, columns = ?columns.len(), "Found columns");

            let primary_key = query_primary_key(self.client, schema_name, &table_name)?;
            trace!(table = ?table_name, primary_key = ?primary_key, "Found primary key");

            tables.push(Table {
                name: table_name,
                columns,
                primary_key,
            });
        }

        info!(
            schema = ?schema_name,
            tables = ?tables.len(),
            enums = ?enums.len(),
            "Schema introspection complete"
        );

        Ok(Schema {
            name: schema_name.to_string(),
            tables,
            enums,
        })
    }
}

/// Query all table names in a schema
fn query_tables(client: &mut Client, schema_name: &str) -> Result<Vec<String>, SqliftError> {
    trace!(schema = ?schema_name, "Querying tables");

    let sql = r#"
        SELECT c.relname AS table_name
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE c.relkind = 'r'
            AND n.nspname = $1
        ORDER BY c.relname
    "#;

    let rows = client
        .query(sql, &[&schema_name])
        .map_err(|e| SqliftError::Introspection {
            schema: schema_name.to_string(),
            message: format!("Failed to query tables: {}", e),
        })?;

    let tables = rows.iter().map(|row| row.get("table_name")).collect();
    trace!(tables = ?tables, "Tables found");
    Ok(tables)
}

/// Query all columns for a table
fn query_columns(
    client: &mut Client,
    schema_name: &str,
    table_name: &str,
) -> Result<Vec<Column>, SqliftError> {
    trace!(schema = ?schema_name, table = ?table_name, "Querying columns");

    let sql = r#"
        SELECT 
            a.attname AS column_name,
            format_type(a.atttypid, a.atttypmod) AS data_type,
            NOT a.attnotnull AS is_nullable,
            pg_get_expr(d.adbin, d.adrelid) AS default_value,
            a.attnum AS ordinal_position
        FROM pg_attribute a
        JOIN pg_class c ON c.oid = a.attrelid
        JOIN pg_namespace n ON n.oid = c.relnamespace
        LEFT JOIN pg_attrdef d ON d.adrelid = c.oid AND d.adnum = a.attnum
        WHERE c.relname = $1
            AND n.nspname = $2
            AND a.attnum > 0
            AND NOT a.attisdropped
        ORDER BY a.attnum
    "#;

    let rows = client
        .query(sql, &[&table_name, &schema_name])
        .map_err(|e| {
            error!(
                schema = ?schema_name,
                table = ?table_name,
                error = ?e,
                "Failed to query columns"
            );
            SqliftError::Introspection {
                schema: schema_name.to_string(),
                message: format!("Failed to query columns for table '{}': {}", table_name, e),
            }
        })?;

    let mut columns = Vec::with_capacity(rows.len());
    for row in rows {
        let column_name: String = row.get("column_name");
        let data_type_str: String = row.get("data_type");
        let is_nullable: bool = row.get("is_nullable");
        let default_value: Option<String> = row.get("default_value");

        let is_auto_generated = is_auto_generated_column(&default_value);
        let has_default = default_value.is_some();
        let data_type = parse_data_type(&data_type_str);

        trace!(
            column = ?column_name,
            data_type = ?data_type_str,
            parsed_type = ?data_type,
            is_nullable = ?is_nullable,
            has_default = ?has_default,
            is_auto_generated = ?is_auto_generated,
            "Parsed column"
        );

        columns.push(Column {
            name: column_name,
            data_type,
            is_nullable,
            has_default,
            is_auto_generated,
        });
    }

    Ok(columns)
}

/// Query primary key columns for a table
fn query_primary_key(
    client: &mut Client,
    schema_name: &str,
    table_name: &str,
) -> Result<Vec<String>, SqliftError> {
    trace!(schema = ?schema_name, table = ?table_name, "Querying primary key");

    let sql = r#"
        SELECT a.attname AS column_name
        FROM pg_constraint con
        JOIN pg_class c ON c.oid = con.conrelid
        JOIN pg_namespace n ON n.oid = c.relnamespace
        JOIN pg_attribute a ON a.attrelid = c.oid AND a.attnum = ANY(con.conkey)
        WHERE con.contype = 'p'
            AND c.relname = $1
            AND n.nspname = $2
        ORDER BY array_position(con.conkey, a.attnum)
    "#;

    let rows = client
        .query(sql, &[&table_name, &schema_name])
        .map_err(|e| {
            error!(
                schema = ?schema_name,
                table = ?table_name,
                error = ?e,
                "Failed to query primary key"
            );
            SqliftError::Introspection {
                schema: schema_name.to_string(),
                message: format!(
                    "Failed to query primary key for table '{}': {}",
                    table_name, e
                ),
            }
        })?;

    let pk_columns = rows.iter().map(|row| row.get("column_name")).collect();
    trace!(table = ?table_name, primary_key = ?pk_columns, "Primary key found");
    Ok(pk_columns)
}

/// Query all enum types in a schema
fn query_enums(client: &mut Client, schema_name: &str) -> Result<Vec<EnumType>, SqliftError> {
    trace!(schema = ?schema_name, "Querying enum types");

    let sql = r#"
        SELECT 
            t.typname AS enum_name,
            e.enumlabel AS enum_value
        FROM pg_type t
        JOIN pg_enum e ON e.enumtypid = t.oid
        JOIN pg_namespace n ON n.oid = t.typnamespace
        WHERE n.nspname = $1
        ORDER BY t.typname, e.enumsortorder
    "#;

    let rows = client.query(sql, &[&schema_name]).map_err(|e| {
        error!(schema = ?schema_name, error = ?e, "Failed to query enum types");
        SqliftError::Introspection {
            schema: schema_name.to_string(),
            message: format!("Failed to query enums: {}", e),
        }
    })?;

    // Group enum values by enum name
    let mut enums: Vec<EnumType> = Vec::new();
    for row in rows {
        let enum_name: String = row.get("enum_name");
        let enum_value: String = row.get("enum_value");

        // Find existing enum or create new one
        if let Some(existing) = enums.iter_mut().find(|e| e.name == enum_name) {
            existing.values.push(enum_value);
        } else {
            trace!(enum_name = ?enum_name, "Found new enum type");
            enums.push(EnumType {
                name: enum_name,
                values: vec![enum_value],
            });
        }
    }

    for e in &enums {
        trace!(name = ?e.name, values = ?e.values, "Enum type");
    }

    Ok(enums)
}

/// Check if a column is auto-generated (SERIAL, BIGSERIAL, IDENTITY)
fn is_auto_generated_column(default_value: &Option<String>) -> bool {
    match default_value {
        Some(default) => {
            let lower = default.to_lowercase();
            // SERIAL/BIGSERIAL columns have nextval('sequence_name') as default
            lower.contains("nextval(")
                // IDENTITY columns
                || lower.contains("generated")
        }
        None => false,
    }
}

/// Parse PostgreSQL type string into DataType enum
fn parse_data_type(type_str: &str) -> DataType {
    let lower = type_str.to_lowercase();
    let trimmed = lower.trim();

    // Handle arrays first (e.g., "integer[]", "text[]", "character varying(255)[]")
    if trimmed.ends_with("[]") {
        let inner_type = &trimmed[..trimmed.len() - 2];
        let inner = parse_data_type(inner_type);
        return DataType::Array(Box::new(inner));
    }

    // Handle types with parameters
    if trimmed.starts_with("character varying") || trimmed.starts_with("varchar") {
        let len = extract_length(trimmed);
        return DataType::Varchar(len);
    }
    if trimmed.starts_with("character(") || trimmed.starts_with("char(") {
        let len = extract_length(trimmed);
        return DataType::Char(len);
    }
    if trimmed.starts_with("numeric") || trimmed.starts_with("decimal") {
        return DataType::Numeric;
    }

    // Handle timestamp variations
    if trimmed.starts_with("timestamp") {
        if trimmed.contains("with time zone") || trimmed.contains("timestamptz") {
            return DataType::TimestampTz;
        }
        return DataType::Timestamp;
    }

    // Handle time variations
    if trimmed.starts_with("time ") || trimmed == "time" {
        if trimmed.contains("with time zone") {
            return DataType::TimeTz;
        }
        return DataType::Time;
    }

    // Simple type matching
    match trimmed {
        "smallint" | "int2" => DataType::SmallInt,
        "integer" | "int" | "int4" => DataType::Integer,
        "bigint" | "int8" => DataType::BigInt,
        "boolean" | "bool" => DataType::Boolean,
        "text" => DataType::Text,
        "real" | "float4" => DataType::Real,
        "double precision" | "float8" => DataType::DoublePrecision,
        "date" => DataType::Date,
        "uuid" => DataType::Uuid,
        "json" => DataType::Json,
        "jsonb" => DataType::JsonBinary,
        "bytea" => DataType::Binary,
        "timetz" => DataType::TimeTz,
        "timestamptz" => DataType::TimestampTz,
        _ => {
            // Assume it's a custom enum type
            DataType::Enum(type_str.to_string())
        }
    }
}

/// Extract length parameter from type like "varchar(255)" or "character varying(100)"
fn extract_length(type_str: &str) -> Option<u32> {
    if let Some(start) = type_str.find('(') {
        if let Some(end) = type_str.find(')') {
            let len_str = &type_str[start + 1..end];
            // Handle numeric(10,2) - just take first number
            let first_num = len_str.split(',').next().unwrap_or(len_str);
            return first_num.trim().parse().ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_types() {
        assert_eq!(parse_data_type("integer"), DataType::Integer);
        assert_eq!(parse_data_type("int"), DataType::Integer);
        assert_eq!(parse_data_type("bigint"), DataType::BigInt);
        assert_eq!(parse_data_type("boolean"), DataType::Boolean);
        assert_eq!(parse_data_type("text"), DataType::Text);
        assert_eq!(parse_data_type("uuid"), DataType::Uuid);
        assert_eq!(parse_data_type("jsonb"), DataType::JsonBinary);
    }

    #[test]
    fn test_parse_varchar() {
        assert_eq!(
            parse_data_type("varchar(255)"),
            DataType::Varchar(Some(255))
        );
        assert_eq!(
            parse_data_type("character varying(100)"),
            DataType::Varchar(Some(100))
        );
        assert_eq!(
            parse_data_type("character varying"),
            DataType::Varchar(None)
        );
    }

    #[test]
    fn test_parse_timestamp() {
        assert_eq!(parse_data_type("timestamp"), DataType::Timestamp);
        assert_eq!(
            parse_data_type("timestamp without time zone"),
            DataType::Timestamp
        );
        assert_eq!(
            parse_data_type("timestamp with time zone"),
            DataType::TimestampTz
        );
        assert_eq!(parse_data_type("timestamptz"), DataType::TimestampTz);
    }

    #[test]
    fn test_parse_array() {
        assert_eq!(
            parse_data_type("integer[]"),
            DataType::Array(Box::new(DataType::Integer))
        );
        assert_eq!(
            parse_data_type("text[]"),
            DataType::Array(Box::new(DataType::Text))
        );
        assert_eq!(
            parse_data_type("character varying(255)[]"),
            DataType::Array(Box::new(DataType::Varchar(Some(255))))
        );
    }

    #[test]
    fn test_parse_custom_enum() {
        assert_eq!(
            parse_data_type("order_status"),
            DataType::Enum("order_status".to_string())
        );
    }

    #[test]
    fn test_is_auto_generated() {
        assert!(is_auto_generated_column(&Some(
            "nextval('users_id_seq'::regclass)".to_string()
        )));
        assert!(is_auto_generated_column(&Some(
            "GENERATED ALWAYS AS IDENTITY".to_string()
        )));
        assert!(!is_auto_generated_column(&Some(
            "'default_value'".to_string()
        )));
        assert!(!is_auto_generated_column(&None));
    }

    #[test]
    fn test_extract_length() {
        assert_eq!(extract_length("varchar(255)"), Some(255));
        assert_eq!(extract_length("character varying(100)"), Some(100));
        assert_eq!(extract_length("numeric(10,2)"), Some(10));
        assert_eq!(extract_length("text"), None);
    }
}
