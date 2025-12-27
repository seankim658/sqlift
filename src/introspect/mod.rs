//! Database introspection
//!
//! This module provides functionality for extracting schema information
//! from databases. Each supported database has its own feature-gated submodule.

use crate::prelude::{Schema, SqliftError};

/// Filters to apply during introspection
#[derive(Debug, Default, Clone)]
pub struct TableFilter {
    /// Only include these tables (if Some)
    pub include: Option<Vec<String>>,
    /// Exclude these tables
    pub exclude: Option<Vec<String>>,
}

impl TableFilter {
    /// Check if a table should be included
    pub fn should_include(&self, table_name: &str) -> bool {
        // Check include list
        if let Some(include) = &self.include {
            if !include.iter().any(|t| t == table_name) {
                return false;
            }
        }

        // Check exclude list
        if let Some(exclude) = &self.exclude {
            if exclude.iter().any(|t| t == table_name) {
                return false;
            }
        }

        true
    }
}

/// Trait for database introspection implementations
pub trait Introspector {
    /// Introspect a database schema and return structured schema information
    fn introspect(&mut self, schema_name: &str, filter: &TableFilter) -> Result<Schema, SqliftError>;
}

// Feature-gated database implementations
#[cfg(feature = "postgres")]
mod postgres;

#[cfg(feature = "postgres")]
pub use postgres::PostgresIntrospector;
