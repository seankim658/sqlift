//! # sqlift
//!
//! Generate typed data access code from database schemas
//!
//! This crate provides a CLI tool and library for introspecting databse schemas
//! and generating type safe data access code.

pub mod codegen;
pub mod config;
pub mod error;
pub mod introspect;
pub mod schema;

pub mod prelude {
    pub use crate::codegen::{CodeGenConfig, CodeGenerator, FunctionStyle, OutputMode};
    pub use crate::config::DbConfig;
    pub use crate::error::SqliftError;
    pub use crate::introspect::{Introspector, TableFilter};
    pub use crate::schema::{Column, DataType, EnumType, Schema, Table};
}

#[cfg(feature = "postgres")]
pub use introspect::PostgresIntrospector;
