//! Configuration loading
//!
//! Loads database connection configuration from environment variables,
//! optionally reading from a .env file first.

use crate::prelude::SqliftError;
use std::{env, path::Path};
use tracing::{debug, error, trace, warn};

/// Database connection configuration
#[derive(Debug, Clone)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
}

impl DbConfig {
    /// Load configuration from environment variables
    ///
    /// Expected variables:
    /// - DB_HOST (default: localhost)
    /// - DB_PORT (default: 5432)
    /// - DB_NAME (required)
    /// - DB_USER (required)
    /// - DB_PASSWORD (required)
    pub fn from_env() -> Result<Self, SqliftError> {
        debug!("Loading database configuration from environment");

        let host = env::var("DB_HOST").unwrap_or_else(|_| {
            trace!("DB_HOST not set, using default");
            "localhost".to_string()
        });

        let port_str = env::var("DB_PORT").unwrap_or_else(|_| {
            trace!("DB_PORT not set, using default");
            "5432".to_string()
        });

        let port = port_str.parse::<u16>().map_err(|e| {
            error!(port = ?port_str, error = ?e, "Invalid DB_PORT value");
            SqliftError::Config("DB_PORT must be a valid port number".to_string())
        })?;

        let database = env::var("DB_NAME").map_err(|_| {
            error!("DB_NAME environment variable is not set");
            SqliftError::Config("DB_NAME environment variable is required".to_string())
        })?;

        let user = env::var("DB_USER").map_err(|_| {
            error!("DB_USER environment variable is not set");
            SqliftError::Config("DB_USER environment variable is required".to_string())
        })?;

        let password = env::var("DB_PASSWORD").map_err(|_| {
            error!("DB_PASSWORD environment variable is not set");
            SqliftError::Config("DB_PASSWORD environment variable is required".to_string())
        })?;

        debug!(host = ?host, port = ?port, database = ?database, user = ?user, "Configuration loaded");

        Ok(Self {
            host,
            port,
            database,
            user,
            password,
        })
    }

    /// Load a .env file and then read configuration from environment
    pub fn load(env_file: &Path) -> Result<Self, SqliftError> {
        if env_file.exists() {
            debug!(path = ?env_file, "Loading environment file");
            dotenvy::from_path(env_file).map_err(|e| {
                error!(path = ?env_file, error = ?e, "Failed to load environment file");
                SqliftError::Config(format!("Failed to load {}: {}", env_file.display(), e))
            })?;
        } else {
            warn!(path = ?env_file, "Environment file not found, using existing environment");
        }

        Self::from_env()
    }

    /// Build a PostgreSQL connection string
    pub fn postgres_connection_string(&self) -> String {
        format!(
            "host={} port={} dbname={} user={} password={}",
            self.host, self.port, self.database, self.user, self.password
        )
    }

    /// Build a connection string with password redacted (for error messages)
    pub fn redacted_connection_string(&self) -> String {
        format!(
            "host={} port={} dbname={} user={} password=***",
            self.host, self.port, self.database, self.user
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn clear_env_vars() {
        env::remove_var("DB_HOST");
        env::remove_var("DB_PORT");
        env::remove_var("DB_NAME");
        env::remove_var("DB_USER");
        env::remove_var("DB_PASSWORD");
    }

    fn set_required_env_vars() {
        env::set_var("DB_NAME", "testdb");
        env::set_var("DB_USER", "testuser");
        env::set_var("DB_PASSWORD", "testpass");
    }

    #[test]
    fn test_from_env_with_defaults() {
        clear_env_vars();
        set_required_env_vars();

        let config = DbConfig::from_env().unwrap();

        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5432);
        assert_eq!(config.database, "testdb");
        assert_eq!(config.user, "testuser");
        assert_eq!(config.password, "testpass");
    }

    #[test]
    fn test_from_env_with_custom_values() {
        clear_env_vars();
        env::set_var("DB_HOST", "db.example.com");
        env::set_var("DB_PORT", "5433");
        set_required_env_vars();

        let config = DbConfig::from_env().unwrap();

        assert_eq!(config.host, "db.example.com");
        assert_eq!(config.port, 5433);
    }

    #[test]
    fn test_from_env_missing_db_name() {
        clear_env_vars();
        env::set_var("DB_USER", "testuser");
        env::set_var("DB_PASSWORD", "testpass");

        let result = DbConfig::from_env();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("DB_NAME"));
    }

    #[test]
    fn test_from_env_invalid_port() {
        clear_env_vars();
        set_required_env_vars();
        env::set_var("DB_PORT", "not_a_number");

        let result = DbConfig::from_env();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("DB_PORT"));
    }

    #[test]
    fn test_postgres_connection_string() {
        let config = DbConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "mydb".to_string(),
            user: "myuser".to_string(),
            password: "secret".to_string(),
        };

        let conn_str = config.postgres_connection_string();

        assert_eq!(
            conn_str,
            "host=localhost port=5432 dbname=mydb user=myuser password=secret"
        );
    }

    #[test]
    fn test_redacted_connection_string() {
        let config = DbConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "mydb".to_string(),
            user: "myuser".to_string(),
            password: "secret".to_string(),
        };

        let conn_str = config.redacted_connection_string();

        assert!(!conn_str.contains("secret"));
        assert!(conn_str.contains("***"));
    }
}
