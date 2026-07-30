//! Public crate types.
use std::{
    fmt::{Display, Formatter},
    sync::Arc,
};

use axum_login::{AuthSession, AuthnBackend};
use sqlx::{Error as SqlxError, MySqlPool, migrate::MigrateError};

use crate::{
    auth::{AuthSecurity, ClientIdentity},
    database::{User, get_admin_user_by_id, get_admin_user_by_username},
};

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
    pub(crate) auth_security: Arc<AuthSecurity>,
    pub(crate) db_pool: Arc<MySqlPool>,
    pub(crate) allowed_origins: Vec<String>,
}

impl AuthnBackend for AppState {
    type User = User;
    type Credentials = Credentials;
    type Error = Error;

    async fn authenticate(&self, credentials: Self::Credentials) -> Result<Option<Self::User>> {
        let Credentials {
            client,
            password,
            username,
        } = credentials;

        if !self.auth_security.allow_client(client).await {
            return Ok(None);
        }
        let Some(verification_permit) = self.auth_security.verification_permit() else {
            return Ok(None);
        };

        let user = get_admin_user_by_username(&(*self.db_pool), &username).await?;
        if !self
            .auth_security
            .allow_account(user.as_ref().map(|user| user.id), &username)
            .await
        {
            return Ok(None);
        }
        let password_hash = user.as_ref().map(|user| user.password_hash.clone());

        if self
            .auth_security
            .verify_password(password, password_hash, verification_permit)
            .await
        {
            Ok(user)
        } else {
            Ok(None)
        }
    }

    async fn get_user(&self, user_id: &u64) -> Result<Option<Self::User>> {
        get_admin_user_by_id(&(*self.db_pool), *user_id).await
    }
}

/* ********************************************************************************************** *
 *                                          Credentials                                           *
 * ********************************************************************************************** */

pub struct Credentials {
    pub(crate) client: ClientIdentity,
    pub(crate) username: String,
    pub(crate) password: String,
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
    /// Invalid application configuration.
    Configuration(String),
    /// A database query or connection error.
    Database(SqlxError),
    /// An error running database migrations.
    DatabaseMigration(MigrateError),
    /// An error listening on the specified TCP port.
    Network(std::io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::System(error) => write!(f, "system error: {error}"),
            Self::Environment(error) => write!(f, "environment error: {error}"),
            Self::Configuration(error) => write!(f, "configuration error: {error}"),
            Self::Database(error) => write!(f, "database error: {error}"),
            Self::DatabaseMigration(error) => write!(f, "database migration error: {error}"),
            Self::Network(error) => write!(f, "network error: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::System(_) | Self::Configuration(_) => None,
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
