use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use sqlift::codegen::{CodeGenConfig, FunctionStyle, OutputMode};
use sqlift::config::DbConfig;
use sqlift::introspect::{Introspector, TableFilter};
use sqlift::schema::Schema;

#[derive(Debug, Clone, ValueEnum)]
enum Database {
    Postgres,
}

#[derive(Debug, Clone, ValueEnum)]
enum Language {
    Python,
}

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum CliOutputMode {
    /// One file per table
    #[default]
    Library,
    /// Single file with all models and functions
    Flat,
}

impl From<CliOutputMode> for OutputMode {
    fn from(mode: CliOutputMode) -> Self {
        match mode {
            CliOutputMode::Library => OutputMode::Library,
            CliOutputMode::Flat => OutputMode::Flat,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
enum CliFunctionStyle {
    /// Functions accept connection as first parameter
    #[default]
    Standalone,
    /// Methods on a repository class
    Class,
}

impl From<CliFunctionStyle> for FunctionStyle {
    fn from(style: CliFunctionStyle) -> Self {
        match style {
            CliFunctionStyle::Standalone => FunctionStyle::Standalone,
            CliFunctionStyle::Class => FunctionStyle::Class,
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "sqlift")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Target database type
    database: Database,

    /// Target language for generated code
    language: Language,

    /// Output directory/file path
    #[arg(short, long, default_value = "./database")]
    output: PathBuf,

    /// Output mode
    #[arg(long, value_enum, default_value_t = CliOutputMode::Library)]
    mode: CliOutputMode,

    /// Function style
    #[arg(long, value_enum, default_value_t = CliFunctionStyle::Standalone)]
    style: CliFunctionStyle,

    /// Database schema to introspect
    #[arg(long, default_value = "public")]
    schema: String,

    /// Path to .env file for connection config
    #[arg(long, default_value = "./.env")]
    env_file: PathBuf,

    /// Comma-separated list of tables to include (default: all)
    #[arg(long, value_delimiter = ',')]
    tables: Option<Vec<String>>,

    /// Comma-separated list of tables to exclude
    #[arg(long, value_delimiter = ',')]
    exclude: Option<Vec<String>>,

    /// Verbose output (-v for debug, -vv for trace)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

fn main() {
    if let Err(e) = run() {
        error!(error = ?e, "Fatal error");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    init_tracing(cli.verbose);

    info!("sqlift v{}", env!("CARGO_PKG_VERSION"));
    info!(
        database = ?cli.database,
        language = ?cli.language,
        output = ?cli.output,
        mode = ?cli.mode,
        style = ?cli.style,
        schema = ?cli.schema,
        "Starting code generation"
    );

    // Load configuration
    let config = DbConfig::load(&cli.env_file).context("Failed to load database configuration")?;
    debug!(connection = ?config.redacted_connection_string(), "Loaded configuration");

    // Build table filter
    let filter = TableFilter {
        include: cli.tables,
        exclude: cli.exclude,
    };

    if filter.include.is_some() || filter.exclude.is_some() {
        debug!(filter = ?filter, "Table filter configured");
    }

    // Introspect database
    let schema = introspect_database(&cli.database, &config, &cli.schema, &filter)?;

    if schema.tables.is_empty() {
        warn!("No tables found after filtering");
        return Ok(());
    }

    info!(
        tables = ?schema.tables.len(),
        enums = ?schema.enums.len(),
        "Schema ready for code generation"
    );

    // Log table names at debug level
    for table in &schema.tables {
        debug!(
            table = ?table.name,
            columns = ?table.columns.len(),
            primary_key = ?table.primary_key,
            "Table"
        );
    }

    let codegen_config = CodeGenConfig::new(cli.output)
        .with_output_mode(cli.mode.into())
        .with_function_style(cli.style.into());
    debug!(codegen_config = ?codegen_config, "Code generation config");

    // TODO: Code generation
    info!("Code generation not yet implemented");

    Ok(())
}

fn init_tracing(verbose: u8) {
    let level = match verbose {
        0 => Level::INFO,
        1 => Level::DEBUG,
        _ => Level::TRACE,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");
}

fn introspect_database(
    database: &Database,
    config: &DbConfig,
    schema_name: &str,
    filter: &TableFilter,
) -> Result<Schema> {
    match database {
        Database::Postgres => introspect_postgres(config, schema_name, filter),
    }
}

#[cfg(feature = "postgres")]
fn introspect_postgres(
    config: &DbConfig,
    schema_name: &str,
    filter: &TableFilter,
) -> Result<Schema> {
    use postgres::NoTls;
    use sqlift::PostgresIntrospector;

    info!(connection = ?config.redacted_connection_string(), "Connecting to PostgreSQL");

    let mut client = postgres::Client::connect(&config.postgres_connection_string(), NoTls)
        .with_context(|| {
            format!(
                "Failed to connect to PostgreSQL at {}",
                config.redacted_connection_string()
            )
        })?;

    info!("Connected to database");

    let mut introspector = PostgresIntrospector::new(&mut client);
    let schema = introspector
        .introspect(schema_name, filter)
        .context("Failed to introspect schema")?;

    Ok(schema)
}

#[cfg(not(feature = "postgres"))]
fn introspect_postgres(
    _config: &DbConfig,
    _schema_name: &str,
    _filter: &TableFilter,
) -> Result<Schema> {
    bail!("PostgreSQL support not enabled. Rebuild with --features postgres")
}
