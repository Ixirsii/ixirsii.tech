//! Interactive provisioning of administrator accounts.

use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use sqlx::{Connection, PgConnection};

use crate::{Error, Result, configured_database_url, database};

pub async fn create(username: &str) -> Result<()> {
    if username.trim() != username
        || username.chars().any(char::is_control)
        || !(1..=127).contains(&username.chars().count())
    {
        return Err(Error::Configuration(
            "username must be 1–127 characters with no control characters or surrounding whitespace"
                .to_owned(),
        ));
    }

    let mut connection = PgConnection::connect(&configured_database_url()?).await?;
    let hash = prompt_password_hash()?;
    if !database::insert_admin_user(&mut connection, username, &hash).await? {
        return Err(Error::System("admin username already exists".to_owned()));
    }
    println!("Created admin account '{username}'.");
    Ok(())
}

fn prompt_password_hash() -> Result<String> {
    let password = inquire::Password::new("New password:")
        .with_custom_confirmation_message("Confirm password:")
        .prompt()
        .map_err(|_| {
            Error::System("could not read a confirmed password from terminal".to_owned())
        })?;
    if password.trim().is_empty() {
        return Err(Error::Configuration(
            "password must not be blank".to_owned(),
        ));
    }
    hash_password(&password)
}

pub fn hash_password(password: &str) -> Result<String> {
    Argon2::default()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|hash| hash.to_string())
        .map_err(|_| Error::System("could not hash password".to_owned()))
}
