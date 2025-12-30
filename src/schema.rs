//! Schema data structures
//!
//! These types represent database schema information and form the contract
//! between introspection (produces) and code generation (consumes).

/// A complete database schema
#[derive(Debug, Clone)]
pub struct Schema {
    pub name: String,
    pub tables: Vec<Table>,
    pub enums: Vec<EnumType>,
}

/// Database table
#[derive(Debug, Clone)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
    /// Column names that form the primary key (in order)
    pub primary_key: Vec<String>,
}

impl Table {
    /// Returns PascalCase class name from snake_case table name
    pub fn class_name(&self) -> String {
        self.name
            .split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        let first_upper = first.to_uppercase().to_string();
                        first_upper + chars.as_str()
                    }
                }
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Returns singular form of the class name (basic heuristic)
    pub fn singular_class_name(&self) -> String {
        let class_name = self.class_name();

        // Basic singularization rules
        if class_name.ends_with("ies") {
            format!("{}y", &class_name[..class_name.len() - 3])
        } else if class_name.ends_with('s') && !class_name.ends_with("ss") {
            class_name[..class_name.len() - 1].to_string()
        } else {
            class_name
        }
    }

    /// Returns singular form of the table name in snake_case (basic heuristic)
    pub fn singular_name(&self) -> String {
        let name = &self.name;

        // Basic singularization rules for snake_case names
        if name.ends_with("ies") {
            format!("{}y", &name[..name.len() - 3])
        } else if name.ends_with('s') && !name.ends_with("ss") {
            name[..name.len() - 1].to_string()
        } else {
            name.clone()
        }
    }

    /// Check if the primary key is auto-generated (SERIAL, BIGSERIAL, identity)
    ///
    /// Used to determine if upsert should be generated - tables with
    /// auto-generated PKs don't get upsert since you typically don't
    /// know the PK value before insertion.
    pub fn has_auto_generated_pk(&self) -> bool {
        // Find primary key columns and check if any are auto-generated
        self.primary_key.iter().any(|pk_name| {
            self.columns
                .iter()
                .find(|col| &col.name == pk_name)
                .map(|col| col.is_auto_generated)
                .unwrap_or(false)
        })
    }

    /// Get primary key columns in order
    pub fn primary_key_columns(&self) -> Vec<&Column> {
        self.primary_key
            .iter()
            .filter_map(|pk_name| self.columns.iter().find(|col| &col.name == pk_name))
            .collect()
    }

    /// Get columns that should be parameters for insert
    /// (excludes auto-generated columns and columns with defaults)
    pub fn insert_columns(&self) -> Vec<&Column> {
        let mut cols: Vec<&Column> = self
            .columns
            .iter()
            .filter(|col| !col.is_auto_generated && !col.has_default)
            .collect();
        cols.sort_by_key(|col| col.is_nullable);
        cols
    }

    /// Get non-primary-key columns (for update SET clause)
    pub fn non_pk_columns(&self) -> Vec<&Column> {
        self.columns
            .iter()
            .filter(|col| !self.primary_key.contains(&col.name))
            .collect()
    }
}

/// A table column
#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub data_type: DataType,
    pub is_nullable: bool,
    /// Column has a server-side default (DEFAULT value, NOW(), etc.)
    pub has_default: bool,
    /// Column is auto-generated (SERIAL, BIGSERIAL, IDENTITY)
    pub is_auto_generated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    SmallInt,
    Integer,
    BigInt,
    Boolean,
    Text,
    Varchar(Option<u32>),
    Char(Option<u32>),
    Real,
    DoublePrecision,
    Numeric,
    Timestamp,
    TimestampTz,
    Date,
    Time,
    TimeTz,
    Uuid,
    Json,
    JsonBinary,
    Binary,
    Array(Box<DataType>),
    /// Custom enum type, stores the enum name
    Enum(String),
}

/// A custom enum type defined in the database
#[derive(Debug, Clone)]
pub struct EnumType {
    pub name: String,
    pub values: Vec<String>,
}

/// Convert snake_case to PascalCase
///
/// This is a shared utility used by code generators for all target languages.
pub fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let first_upper = first.to_uppercase().to_string();
                    first_upper + chars.as_str()
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_name_simple() {
        let table = Table {
            name: "users".to_string(),
            columns: vec![],
            primary_key: vec![],
        };
        assert_eq!(table.class_name(), "Users");
    }

    #[test]
    fn test_class_name_snake_case() {
        let table = Table {
            name: "user_accounts".to_string(),
            columns: vec![],
            primary_key: vec![],
        };
        assert_eq!(table.class_name(), "UserAccounts");
    }

    #[test]
    fn test_class_name_multiple_underscores() {
        let table = Table {
            name: "order_line_items".to_string(),
            columns: vec![],
            primary_key: vec![],
        };
        assert_eq!(table.class_name(), "OrderLineItems");
    }

    #[test]
    fn test_singular_class_name_regular_plural() {
        let table = Table {
            name: "users".to_string(),
            columns: vec![],
            primary_key: vec![],
        };
        assert_eq!(table.singular_class_name(), "User");
    }

    #[test]
    fn test_singular_class_name_ies_plural() {
        let table = Table {
            name: "categories".to_string(),
            columns: vec![],
            primary_key: vec![],
        };
        assert_eq!(table.singular_class_name(), "Category");
    }

    #[test]
    fn test_singular_class_name_no_change() {
        let table = Table {
            name: "staff".to_string(),
            columns: vec![],
            primary_key: vec![],
        };
        assert_eq!(table.singular_class_name(), "Staff");
    }

    #[test]
    fn test_has_auto_generated_pk_true() {
        let table = Table {
            name: "users".to_string(),
            columns: vec![Column {
                name: "id".to_string(),
                data_type: DataType::Integer,
                is_nullable: false,
                has_default: true,
                is_auto_generated: true,
            }],
            primary_key: vec!["id".to_string()],
        };
        assert!(table.has_auto_generated_pk());
    }

    #[test]
    fn test_has_auto_generated_pk_false() {
        let table = Table {
            name: "users".to_string(),
            columns: vec![Column {
                name: "id".to_string(),
                data_type: DataType::Uuid,
                is_nullable: false,
                has_default: false,
                is_auto_generated: false,
            }],
            primary_key: vec!["id".to_string()],
        };
        assert!(!table.has_auto_generated_pk());
    }
}
