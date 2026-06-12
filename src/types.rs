//! Public crate types.
use std::{
    fmt::{Display, Formatter},
    sync::Arc,
};

use argon2::{Argon2, PasswordHash, PasswordVerifier, password_hash};
use axum_login::{AuthSession, AuthnBackend};
use sqlx::{MySqlPool, migrate::MigrateError};

use crate::database::{User, get_admin_user_by_id, get_admin_user_by_username};

/* ********************************************************************************************** *
 *                                       Admin auth session                                       *
 * ********************************************************************************************** */

pub type AdminAuthSession = AuthSession<AppState>;

/* ********************************************************************************************** *
 *                                           App state                                            *
 * ********************************************************************************************** */

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub db_pool: Arc<MySqlPool>,
    pub allowed_origins: Vec<String>,
}

impl AuthnBackend for AppState {
    type User = User;
    type Credentials = Credentials;
    type Error = Error;

    async fn authenticate(&self, credentials: Self::Credentials) -> Result<Option<Self::User>> {
        let Some(user) =
            get_admin_user_by_username(&(*self.db_pool), &credentials.username).await?
        else {
            return Ok(None);
        };

        let password_hash = PasswordHash::new(&user.password_hash)?;

        match Argon2::default().verify_password(credentials.password.as_bytes(), &password_hash) {
            Ok(()) => Ok(Some(user)),
            Err(password_hash::Error::Password) => Ok(None),
            Err(error) => Err(Self::Error::from(error)),
        }
    }

    async fn get_user(&self, user_id: &u64) -> Result<Option<Self::User>> {
        get_admin_user_by_id(&(*self.db_pool), *user_id).await
    }
}

/* ********************************************************************************************** *
 *                                          Credentials                                           *
 * ********************************************************************************************** */

#[derive(Clone, Debug)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

/* ********************************************************************************************** *
 *                                             Error                                              *
 * ********************************************************************************************** */

/// Internal error type.
#[derive(Debug)]
pub enum Error {
    /// A system error (reading file, etc.).
    SystemError(String),
    /// An error reading environment variables.
    EnvironmentError(String),
    /// An error connecting to the database.
    DatabaseConnectionError(String),
    /// An error creating a database table.
    DatabaseTableCreationError(String),
    /// An error inserting a row into the database.
    DatabaseInsertError(String),
    /// An error deleting a row from the database.
    DatabaseDeleteError(String),
    /// An error selecting from the database.
    DatabaseSelectError(String),
    /// An error updating a row in the database.
    DatabaseUpdateError(String),
    /// An error running database migrations.
    DatabaseMigrationError(String),
    /// An error executing a database transaction.
    DatabaseTransactionError(String),
    /// An error listening on the specified TCP port.
    NetworkError(String),
    /// Password hash error.
    PasswordError(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SystemError(error)
            | Self::EnvironmentError(error)
            | Self::DatabaseConnectionError(error)
            | Self::DatabaseTableCreationError(error)
            | Self::DatabaseInsertError(error)
            | Self::DatabaseDeleteError(error)
            | Self::DatabaseSelectError(error)
            | Self::DatabaseUpdateError(error)
            | Self::DatabaseMigrationError(error)
            | Self::DatabaseTransactionError(error)
            | Self::NetworkError(error)
            | Self::PasswordError(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<dotenvy::Error> for Error {
    fn from(error: dotenvy::Error) -> Self {
        Self::EnvironmentError(error.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::NetworkError(error.to_string())
    }
}

impl From<password_hash::Error> for Error {
    fn from(error: password_hash::Error) -> Self {
        Self::PasswordError(error.to_string())
    }
}

impl From<MigrateError> for Error {
    fn from(error: MigrateError) -> Self {
        Self::DatabaseMigrationError(error.to_string())
    }
}

/* ********************************************************************************************** *
 *                                             Result                                             *
 * ********************************************************************************************** */

/// Result type.
pub type Result<T> = core::result::Result<T, Error>;
