//! Public crate types.
use std::{
    fmt::{Display, Formatter},
    sync::Arc,
};

use argon2::{Argon2, PasswordHash, PasswordVerifier, password_hash};
use axum_login::{AuthSession, AuthnBackend};
use sqlx::{Error as SqlxError, MySqlPool, migrate::MigrateError};

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
    System(String),
    /// An error reading environment variables.
    Environment(dotenvy::Error),
    /// A database query or connection error.
    Database(SqlxError),
    /// An error running database migrations.
    DatabaseMigration(MigrateError),
    /// An error listening on the specified TCP port.
    Network(std::io::Error),
    /// Password hash error.
    Password(password_hash::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::System(error) => write!(f, "system error: {error}"),
            Self::Environment(error) => write!(f, "environment error: {error}"),
            Self::Database(error) => write!(f, "database error: {error}"),
            Self::DatabaseMigration(error) => write!(f, "database migration error: {error}"),
            Self::Network(error) => write!(f, "network error: {error}"),
            Self::Password(error) => write!(f, "password error: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::System(_) | Self::Password(_) => None,
            Self::Environment(error) => Some(error),
            Self::Database(error) => Some(error),
            Self::DatabaseMigration(error) => Some(error),
            Self::Network(error) => Some(error),
        }
    }
}

impl From<dotenvy::Error> for Error {
    fn from(error: dotenvy::Error) -> Self {
        Self::Environment(error)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Network(error)
    }
}

impl From<SqlxError> for Error {
    fn from(error: SqlxError) -> Self {
        Self::Database(error)
    }
}

impl From<password_hash::Error> for Error {
    fn from(error: password_hash::Error) -> Self {
        Self::Password(error)
    }
}

impl From<MigrateError> for Error {
    fn from(error: MigrateError) -> Self {
        Self::DatabaseMigration(error)
    }
}

/* ********************************************************************************************** *
 *                                             Result                                             *
 * ********************************************************************************************** */

/// Result type.
pub type Result<T> = core::result::Result<T, Error>;
