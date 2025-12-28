//! Python code generator
//!
//! Generates typed Python data access code.

use std::collections::HashSet;
use std::fs;

use minijinja::Environment;
use tracing::{debug, info};

use crate::codegen::{CodeGenConfig, CodeGenerator, FunctionStyle, OutputMode};
use crate::error::SqliftError;
use crate::schema::{to_pascal_case, Column, DataType, EnumType, Schema, Table};

/// Python code generator
pub struct PythonGenerator {
    env: Environment<'static>,
}

impl PythonGenerator {
    pub fn new() -> Self {
        let mut env = Environment::new();

        // Register templates
        env.add_template("record", include_str!("templates/record.py.jinja"))
            .expect("Failed to load python record template");
        env.add_template("standalone", include_str!("templates/standalone.py.jinja"))
            .expect("Failed to load standalone template");
        env.add_template("repository", include_str!("templates/repository.py.jinja"))
            .expect("Failed to load repository template");
        env.add_template("init", include_str!("templates/init.py.jinja"))
            .expect("Failed to load init template");
        env.add_template("flat", include_str!("templates/flat.py.jinja"))
            .expect("Failed to load flat template");
        env.add_template("enum", include_str!("templates/enum.py.jinja"))
            .expect("Failed to load enum template");

        Self { env }
    }
}

impl Default for PythonGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGenerator for PythonGenerator {
    fn generate(&self, schema: &Schema, config: &CodeGenConfig) -> Result<(), SqliftError> {
        info!(
            output = ?config.output_path,
                mode = ?config.output_mode,
                style = ?config.function_style,
                "Generating Python code"
        );

        match config.output_mode {
            OutputMode::Library => self.generate_library(schema, config),
            OutputMode::Flat => self.generate_flat(schema, config),
        }
    }
}

impl PythonGenerator {
    /// Generate library mode output
    fn generate_library(&self, schema: &Schema, config: &CodeGenConfig) -> Result<(), SqliftError> {
        let output_dir = &config.output_path;

        // Create output directory
        // TODO : should we check parent directory exists like in flat
        fs::create_dir_all(output_dir)?;
        debug!(path = ?output_dir, "Created output directory");

        // Generate enum file if there are enums
        if !schema.enums.is_empty() {
            let enum_code = self.render_enums(&schema.enums)?;
            let enum_path = output_dir.join("enums.py");
            fs::write(&enum_path, enum_code)?;
            debug!(path = ?enum_path, "Generated enums file");
        }

        // Generate one file per table
        for table in &schema.tables {
            let code = self.render_table(table, schema, config)?;
            let file_path = output_dir.join(format!("{}.py", table.name));
            fs::write(&file_path, code)?;
            debug!(table = ?table.name, path = ?file_path, "Generated table file")
        }

        let init_code = self.render_init(schema)?;
        let init_path = output_dir.join("__init__.py");
        fs::write(&init_path, init_code)?;
        debug!(path = ?init_path, "Generated __init__.py");

        info!(
            tables = schema.tables.len(),
            enums = schema.enums.len(),
            "Python code generation complete"
        );

        Ok(())
    }

    /// Generate flat mode output (single file)
    fn generate_flat(&self, schema: &Schema, config: &CodeGenConfig) -> Result<(), SqliftError> {
        let output_path = &config.output_path;

        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        let code = self.render_flat(schema, config)?;

        let final_path = if output_path.extension().is_some_and(|ext| ext == "py") {
            output_path.clone()
        } else {
            output_path.with_extension("py")
        };

        fs::write(&final_path, code)?;
        info!(path = ?final_path, "Generated flat Python file");

        Ok(())
    }

    /// Render enums file
    fn render_enums(&self, enums: &[EnumType]) -> Result<String, SqliftError> {
        let template = self
            .env
            .get_template("enum")
            .map_err(|e| SqliftError::CodeGen {
                table: "enums".to_string(),
                message: format!("Template error: {}", e),
            })?;

        let ctx = minijinja::context! {
            enums => enums.iter().map(|e| {
                minijinja::context! {
                    name => to_pascal_case(&e.name),
                    db_name => &e.name,
                    values => &e.values,
                }
            }).collect::<Vec<_>>(),
        };

        template.render(ctx).map_err(|e| SqliftError::CodeGen {
            table: "enums".to_string(),
            message: format!("Render error: {}", e),
        })
    }

    /// Render a single table file
    fn render_table(
        &self,
        table: &Table,
        schema: &Schema,
        config: &CodeGenConfig,
    ) -> Result<String, SqliftError> {
        let template_name = match config.function_style {
            FunctionStyle::Standalone => "standalone",
            FunctionStyle::Class => "repository",
        };

        let template = self
            .env
            .get_template(template_name)
            .map_err(|e| SqliftError::CodeGen {
                table: table.name.clone(),
                message: format!("Template error: {}", e),
            })?;

        let ctx = self.build_table_context(table, schema)?;

        template.render(ctx).map_err(|e| SqliftError::CodeGen {
            table: table.name.clone(),
            message: format!("Render error: {}", e),
        })
    }

    /// Render flat file with all tables
    fn render_flat(&self, schema: &Schema, config: &CodeGenConfig) -> Result<String, SqliftError> {
        let template = self
            .env
            .get_template("flat")
            .map_err(|e| SqliftError::CodeGen {
                table: "flat".to_string(),
                message: format!("Template error: {}", e),
            })?;

        let tables_ctx: Vec<_> = schema
            .tables
            .iter()
            .map(|t| self.build_table_context(t, schema))
            .collect::<Result<_, _>>()?;

        let ctx = minijinja::context! {
            enums => schema.enums.iter().map(|e| {
                minijinja::context! {
                    name => to_pascal_case(&e.name),
                    db_name => &e.name,
                    values => &e.values,
                }
            }).collect::<Vec<_>>(),
            tables => tables_ctx,
            imports => collect_imports(schema),
            function_style => match config.function_style {
                FunctionStyle::Standalone => "standalone",
                FunctionStyle::Class => "class",
            }
        };

        template.render(ctx).map_err(|e| SqliftError::CodeGen {
            table: "flat".to_string(),
            message: format!("Render error: {}", e),
        })
    }

    /// Render __init__.py
    fn render_init(&self, schema: &Schema) -> Result<String, SqliftError> {
        let template = self
            .env
            .get_template("init")
            .map_err(|e| SqliftError::CodeGen {
                table: "__init__".to_string(),
                message: format!("Template error: {}", e),
            })?;

        let ctx = minijinja::context! {
            tables => schema.tables.iter().map(|t| {
                minijinja::context! {
                    module_name => &t.name,
                    record_name => format!("{}Record", t.singular_class_name()),
                }
            }).collect::<Vec<_>>(),
            has_enums => !schema.enums.is_empty(),
            enums => schema.enums.iter().map(|e| to_pascal_case(&e.name)).collect::<Vec<_>>(),
        };

        template.render(ctx).map_err(|e| SqliftError::CodeGen {
            table: "__init__".to_string(),
            message: format!("Render error: {}", e),
        })
    }

    /// Build template context for a table
    fn build_table_context(
        &self,
        table: &Table,
        schema: &Schema,
    ) -> Result<minijinja::Value, SqliftError> {
        let columns_ctx: Vec<_> = table
            .columns
            .iter()
            .map(|col| build_column_context(col, schema))
            .collect();

        let pk_columns_ctx: Vec<_> = table
            .primary_key_columns()
            .iter()
            .map(|col| build_column_context(col, schema))
            .collect();

        let insert_columns_ctx: Vec<_> = table
            .insert_columns()
            .iter()
            .map(|col| build_column_context(col, schema))
            .collect();

        let non_pk_columns_ctx: Vec<_> = table
            .non_pk_columns()
            .iter()
            .map(|col| build_column_context(col, schema))
            .collect();

        Ok(minijinja::context! {
            table_name => &table.name,
            record_name => format!("{}Record", table.singular_class_name()),
            class_name => table.singular_class_name(),
            columns => columns_ctx,
            pk_columns => pk_columns_ctx,
            insert_columns => insert_columns_ctx,
            non_pk_columns => non_pk_columns_ctx,
            has_pk => !table.primary_key.is_empty(),
            has_auto_generated_pk => table.has_auto_generated_pk(),
            imports => collect_table_imports(table, schema),
        })
    }
}

/// Build template context for a column
fn build_column_context(col: &Column, schema: &Schema) -> minijinja::Value {
    minijinja::context! {
        name => &col.name,
        python_type => python_type(&col.data_type, col.is_nullable, schema),
        base_type => python_type(&col.data_type, false, schema),
        is_nullable => col.is_nullable,
        has_default => col.has_default,
        is_auto_generated => col.is_auto_generated,
    }
}

/// Convert DataType to Python type string
fn python_type(data_type: &DataType, is_nullable: bool, schema: &Schema) -> String {
    let base_type = match data_type {
        DataType::SmallInt | DataType::Integer | DataType::BigInt => "int".to_string(),
        DataType::Boolean => "bool".to_string(),
        DataType::Text | DataType::Varchar(_) | DataType::Char(_) => "str".to_string(),
        DataType::Real | DataType::DoublePrecision => "float".to_string(),
        DataType::Numeric => "Decimal".to_string(),
        DataType::Timestamp | DataType::TimestampTz => "datetime".to_string(),
        DataType::Date => "date".to_string(),
        DataType::Time | DataType::TimeTz => "time".to_string(),
        DataType::Uuid => "UUID".to_string(),
        DataType::Json | DataType::JsonBinary => "dict[str, Any]".to_string(),
        DataType::Binary => "bytes".to_string(),
        DataType::Array(inner) => {
            let inner_type = python_type(inner, false, schema);
            format!("list[{}]", inner_type)
        }
        DataType::Enum(name) => {
            // Check if this enum exists in the schema
            if schema.enums.iter().any(|e| &e.name == name) {
                to_pascal_case(name)
            } else {
                // Unknown enum, fall back to str
                "str".to_string()
            }
        }
    };

    if is_nullable {
        format!("{} | None", base_type)
    } else {
        base_type
    }
}

/// Collect required imports for a table
fn collect_table_imports(table: &Table, schema: &Schema) -> Vec<String> {
    let mut imports = HashSet::new();

    for col in &table.columns {
        collect_type_imports(&col.data_type, schema, &mut imports);
    }

    let mut sorted: Vec<_> = imports.into_iter().collect();
    sorted.sort();
    sorted
}

/// Collect required imports for the entire schema
fn collect_imports(schema: &Schema) -> Vec<String> {
    let mut imports = HashSet::new();

    for table in &schema.tables {
        for col in &table.columns {
            collect_type_imports(&col.data_type, schema, &mut imports);
        }
    }

    let mut sorted: Vec<_> = imports.into_iter().collect();
    sorted.sort();
    sorted
}

/// Collect imports needed for a specific data type
fn collect_type_imports(data_type: &DataType, schema: &Schema, imports: &mut HashSet<String>) {
    match data_type {
        DataType::Numeric => {
            imports.insert("from decimal import Decimal".to_string());
        }
        DataType::Timestamp | DataType::TimestampTz => {
            imports.insert("from datetime import datetime".to_string());
        }
        DataType::Date => {
            imports.insert("from datetime import date".to_string());
        }
        DataType::Time | DataType::TimeTz => {
            imports.insert("from datetime import time".to_string());
        }
        DataType::Uuid => {
            imports.insert("from uuid import UUID".to_string());
        }
        DataType::Json | DataType::JsonBinary => {
            imports.insert("from typing import Any".to_string());
        }
        DataType::Array(inner) => {
            collect_type_imports(inner, schema, imports);
        }
        DataType::Enum(name) => {
            // Only import if it's a known enum
            if schema.enums.iter().any(|e| &e.name == name) {
                imports.insert(format!("from .enums import {}", to_pascal_case(name)));
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_schema() -> Schema {
        Schema {
            name: "public".to_string(),
            tables: vec![],
            enums: vec![],
        }
    }

    #[test]
    fn test_python_type_simple() {
        let schema = empty_schema();
        assert_eq!(python_type(&DataType::Integer, false, &schema), "int");
        assert_eq!(python_type(&DataType::Text, false, &schema), "str");
        assert_eq!(python_type(&DataType::Boolean, false, &schema), "bool");
    }

    #[test]
    fn test_python_type_nullable() {
        let schema = empty_schema();
        assert_eq!(python_type(&DataType::Integer, true, &schema), "int | None");
        assert_eq!(python_type(&DataType::Text, true, &schema), "str | None");
    }

    #[test]
    fn test_python_type_complex() {
        let schema = empty_schema();
        assert_eq!(python_type(&DataType::Uuid, false, &schema), "UUID");
        assert_eq!(
            python_type(&DataType::JsonBinary, false, &schema),
            "dict[str, Any]"
        );
        assert_eq!(python_type(&DataType::Numeric, false, &schema), "Decimal");
    }

    #[test]
    fn test_python_type_array() {
        let schema = empty_schema();
        let array_type = DataType::Array(Box::new(DataType::Integer));
        assert_eq!(python_type(&array_type, false, &schema), "list[int]");
    }

    #[test]
    fn test_python_type_enum() {
        let schema = Schema {
            name: "public".to_string(),
            tables: vec![],
            enums: vec![EnumType {
                name: "order_status".to_string(),
                values: vec!["pending".to_string(), "completed".to_string()],
            }],
        };
        assert_eq!(
            python_type(&DataType::Enum("order_status".to_string()), false, &schema),
            "OrderStatus"
        );
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("user"), "User");
        assert_eq!(to_pascal_case("order_status"), "OrderStatus");
        assert_eq!(to_pascal_case("order_line_items"), "OrderLineItems");
    }
}
