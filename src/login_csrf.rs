//! Signed, short-lived login challenges without anonymous database sessions.

use argon2::password_hash::rand_core::{OsRng, RngCore};
use axum_extra::extract::cookie::{Cookie, Key, SameSite, SignedCookieJar};
use sqlx::PgPool;
use subtle::ConstantTimeEq;
use time::{Duration, OffsetDateTime};

const LIFETIME: Duration = Duration::minutes(30);
const COOKIE_NAME: &str = if cfg!(debug_assertions) {
    "login-csrf"
} else {
    "__Host-login-csrf"
};

pub async fn load_key(pool: &PgPool) -> crate::Result<Key> {
    let mut candidate = [0_u8; 64];
    OsRng
        .try_fill_bytes(&mut candidate)
        .map_err(|_| crate::Error::System("Unable to generate the login signing key".to_owned()))?;
    sqlx::query!("INSERT INTO login_csrf_key (singleton, key) VALUES (TRUE, $1) ON CONFLICT (singleton) DO NOTHING", &candidate[..])
        .execute(pool).await?;
    let key = sqlx::query_scalar!("SELECT key FROM login_csrf_key WHERE singleton = TRUE")
        .fetch_one(pool)
        .await?;
    Key::try_from(key.as_slice())
        .map_err(|_| crate::Error::System("Invalid login signing key".to_owned()))
}

pub fn challenge(jar: SignedCookieJar) -> (SignedCookieJar, String) {
    let token = jar
        .get(COOKIE_NAME)
        .filter(|cookie| valid_payload(cookie.value()))
        .map_or_else(
            || {
                let expiry = (OffsetDateTime::now_utc() + LIFETIME).unix_timestamp();
                format!("{expiry}.{}", crate::csrf::generate_token())
            },
            |cookie| cookie.value().to_owned(),
        );
    let cookie = Cookie::build((COOKIE_NAME, token.clone()))
        .path("/")
        .http_only(true)
        .secure(!cfg!(debug_assertions))
        .same_site(SameSite::Strict)
        .max_age(LIFETIME)
        .build();
    (jar.add(cookie), token)
}

pub fn verify(jar: &SignedCookieJar, submitted: &str) -> bool {
    jar.get(COOKIE_NAME).is_some_and(|cookie| {
        valid_payload(cookie.value())
            && bool::from(cookie.value().as_bytes().ct_eq(submitted.as_bytes()))
    })
}

pub fn clear(jar: SignedCookieJar) -> SignedCookieJar {
    jar.remove(
        Cookie::build(COOKIE_NAME)
            .path("/")
            .http_only(true)
            .secure(!cfg!(debug_assertions))
            .same_site(SameSite::Strict)
            .build(),
    )
}

fn valid_payload(value: &str) -> bool {
    let Some((expiry, nonce)) = value.split_once('.') else {
        return false;
    };
    let Ok(expiry) = expiry.parse::<i64>() else {
        return false;
    };
    let now = OffsetDateTime::now_utc().unix_timestamp();
    expiry > now
        && expiry <= now.saturating_add(LIFETIME.whole_seconds())
        && nonce.len() == 64
        && nonce.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        http::{HeaderMap, HeaderValue, header},
        response::IntoResponse,
    };

    #[test]
    fn challenge_requires_matching_signed_cookie_and_unexpired_payload()
    -> Result<(), Box<dyn std::error::Error>> {
        let key = Key::try_from(&[42_u8; 64][..])?;
        let (jar, token) = challenge(SignedCookieJar::new(key.clone()));
        assert!(verify(&jar, &token));
        assert!(!verify(&jar, ""));
        assert!(!verify(&SignedCookieJar::new(key.clone()), &token));
        let response = jar.into_response();
        let cookie = response.headers()[header::SET_COOKIE]
            .to_str()?
            .split(';')
            .next()
            .ok_or("missing cookie")?;
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, HeaderValue::from_str(cookie)?);
        assert!(verify(
            &SignedCookieJar::from_headers(&headers, key.clone()),
            &token
        ));
        headers.insert(
            header::COOKIE,
            HeaderValue::from_str(&format!("{COOKIE_NAME}={token}"))?,
        );
        assert!(!verify(
            &SignedCookieJar::from_headers(&headers, key.clone()),
            &token
        ));
        let expired = format!("0.{}", "a".repeat(64));
        let jar = SignedCookieJar::new(key).add(Cookie::new(COOKIE_NAME, expired.clone()));
        assert!(!verify(&jar, &expired));
        assert!(!valid_payload("not-a-timestamp.nonce"));
        Ok(())
    }

    #[sqlx::test]
    async fn signing_key_is_shared_across_replicas(pool: PgPool) -> crate::Result<()> {
        let first = load_key(&pool).await?;
        let second = load_key(&pool).await?;
        assert_eq!(first, second);
        Ok(())
    }
}
