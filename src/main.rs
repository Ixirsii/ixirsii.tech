use axum::{Router, routing::get, serve};
use dotenvy::{dotenv, var};
use sqlx::{MySqlPool, mysql::MySqlPoolOptions};
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use tracing::{error, info, instrument};

mod database;
mod handler;

/* ********************************************************************************************** *
 *                                           AppState                                            *
 * ********************************************************************************************** */

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub db_pool: MySqlPool,
}

/* ********************************************************************************************** *
 *                                             Error                                              *
 * ********************************************************************************************** */

/// Internal error type.
#[derive(Debug)]
pub enum Error {
    /// A system error (reading file, etc.).
    SystemError,
    /// An error reading environment variables.
    EnvironmentError(String),
    /// An error connecting to the database.
    DatabaseConnectionError,
    /// An error creating a database table.
    DatabaseTableCreationError,
    /// An error inserting a row into the database.
    DatabaseInsertionError,
    /// An error deleting a row from the database.
    DatabaseDeletionError,
    /// An error querying the database.
    DatabaseQueryError,
    /// An error running database migrations.
    DatabaseMigrationError(String),
    /// An error listening on the specified TCP port.
    NetworkError(String),
}

impl From<dotenvy::Error> for Error {
    fn from(error: dotenvy::Error) -> Self {
        Self::EnvironmentError(error.to_string())
    }
}

impl From<sqlx::migrate::MigrateError> for Error {
    fn from(error: sqlx::migrate::MigrateError) -> Self {
        Self::DatabaseMigrationError(error.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::NetworkError(error.to_string())
    }
}

/* ********************************************************************************************** *
 *                                             Result                                             *
 * ********************************************************************************************** */

/// Result type.
pub type Result<T> = core::result::Result<T, Error>;

/* ********************************************************************************************** *
 *                                              Main                                              *
 * ********************************************************************************************** */

#[instrument]
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    dotenv().ok();

    let max_connections: u32 = match var("DATABASE_MAX_CONNECTIONS")?.parse() {
        Ok(value) => value,
        Err(err) => {
            let error = err.to_string();

            error!(error = error, "Error parsing DATABASE_MAX_CONNECTIONS");

            return Err(Error::EnvironmentError(error));
        }
    };
    let database_url: String = var("DATABASE_URL")?;

    let db_pool = match MySqlPoolOptions::new()
        .max_connections(max_connections)
        .connect(&database_url)
        .await
    {
        Ok(value) => value,
        Err(err) => {
            let error = err.to_string();

            error!(error = error, "Error connecting to database");

            return Err(Error::DatabaseConnectionError);
        }
    };

    sqlx::migrate!().run(&db_pool).await?;

    let app_state = AppState { db_pool };

    let static_service = ServeDir::new("./src/static");
    let app = Router::new()
        .route("/", get(handler::home))
        .route("/about", get(handler::about_page))
        .route("/blog", get(handler::blog_index))
        .route("/blog/{slug}", get(handler::blog_post))
        .route("/code", get(handler::code_page))
        .nest_service("/static", static_service)
        .with_state(app_state);
    let listener = TcpListener::bind("127.0.0.1:3000").await?;

    info!("Listening on {}", listener.local_addr()?);

    serve(listener, app).await?;

    Ok(())
}
