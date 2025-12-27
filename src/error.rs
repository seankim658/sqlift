use thiserror::Error;

/// sqlift errors
#[derive(Error, Debug)]
pub enum SqliftError {
    #[error("Failed to connect to database: {0}")]
    Connection(String),

    #[error("Failed to introspect schema '{schema}': {message}")]
    Introspection { schema: String, message: String },

    #[error("Code generation failed for table '{table}': {message}")]
    CodeGen { table: String, message: String },

    #[error("Failed to write output: {0}")]
    Output(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),
}
