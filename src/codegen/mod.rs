//! Code generation
//!
//! This module provides functionality for generating typed data access code
//! from the introspected database schema.

use std::path::PathBuf;

use crate::prelude::{Schema, SqliftError};

pub mod python;

pub use python::PythonGenerator;

/// Output mode for generated code
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputMode {
    /// One file per table, organized as a library/package
    #[default]
    Library,
    /// Single file with all models and functions
    Flat,
}

/// Function style for generated code
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FunctionStyle {
    /// Functions accept connection as first parameter
    #[default]
    Standalone,
    /// Methods on a repository class
    Class,
}

/// Configuration for code generation
#[derive(Debug, Clone)]
pub struct CodeGenConfig {
    /// Output directory or file path
    pub output_path: PathBuf,
    /// Output mode
    pub output_mode: OutputMode,
    /// Function style
    pub function_style: FunctionStyle,
}

impl CodeGenConfig {
    pub fn new(output_path: PathBuf) -> Self {
        Self {
            output_path,
            output_mode: OutputMode::default(),
            function_style: FunctionStyle::default(),
        }
    }

    pub fn with_output_mode(mut self, mode: OutputMode) -> Self {
        self.output_mode = mode;
        self
    }

    pub fn with_function_style(mut self, style: FunctionStyle) -> Self {
        self.function_style = style;
        self
    }
}

/// Trait for language-specific code generators
pub trait CodeGenerator {
    /// Generate code for the given schema
    fn generate(&self, schema: &Schema, config: &CodeGenConfig) -> Result<(), SqliftError>;
}
