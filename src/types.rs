//! Public crate types.
use std::{
    fmt::{Display, Formatter},
    sync::Arc,
};

use axum_login::{AuthSession, AuthnBackend};
use sqlx::{Error as SqlxError, PgPool};

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
    pub(crate) login_csrf_key: axum_extra::extract::cookie::Key,
    pub(crate) auth_security: Arc<AuthSecurity>,
    pub(crate) db_pool: Arc<PgPool>,
    pub(crate) allowed_origins: Vec<String>,
}

impl axum::extract::FromRef<AppState> for axum_extra::extract::cookie::Key {
    fn from_ref(state: &AppState) -> Self {
        state.login_csrf_key.clone()
    }
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

        if !self
            .auth_security
            .allow_client(&self.db_pool, client)
            .await?
        {
            return Ok(None);
        }
        let Some(verification_permit) = self.auth_security.verification_permit() else {
            return Ok(None);
        };

        let user = get_admin_user_by_username(&(*self.db_pool), &username).await?;
        if !self
            .auth_security
            .allow_account(
                &self.db_pool,
                user.as_ref().map(|user| user.id),
                &username,
                client,
            )
            .await?
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

    async fn get_user(&self, user_id: &i64) -> Result<Option<Self::User>> {
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
    /// Invalid application configuration.
    Configuration(String),
    /// A database query or connection error.
    Database(SqlxError),
    /// An error listening on the specified TCP port.
    Network(std::io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::System(error) => write!(f, "system error: {error}"),
            Self::Configuration(error) => write!(f, "configuration error: {error}"),
            Self::Database(error) => write!(f, "database error: {error}"),
            Self::Network(error) => write!(f, "network error: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::System(_) | Self::Configuration(_) => None,
            Self::Database(error) => Some(error),
            Self::Network(error) => Some(error),
        }
    }
}

impl Error {
    pub(crate) fn unique_constraint(&self) -> Option<&str> {
        match self {
            Self::Database(error) => error.as_database_error().and_then(|error| {
                error
                    .is_unique_violation()
                    .then(|| error.constraint())
                    .flatten()
            }),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        // The server runner reports invalid HOST and PORT as I/O input errors.
        match error.kind() {
            std::io::ErrorKind::InvalidData | std::io::ErrorKind::InvalidInput => {
                Self::Configuration(error.to_string())
            }
            _ => Self::Network(error),
        }
    }
}

impl From<SqlxError> for Error {
    fn from(error: SqlxError) -> Self {
        Self::Database(error)
    }
}

/* ********************************************************************************************** *
 *                                             Result                                             *
 * ********************************************************************************************** */

/// Result type.
pub type Result<T> = core::result::Result<T, Error>;
