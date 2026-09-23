//! Login rate limiting, trusted client resolution, and password verification.

use std::{
    collections::{HashMap, VecDeque},
    env::{VarError, var},
    fmt::Display,
    hash::Hash,
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use argon2::{Argon2, PasswordHash, PasswordVerifier, password_hash};
use axum::http::HeaderMap;
use ipnet::IpNet;
use sha2::{Digest, Sha256};
use tokio::{
    sync::{Mutex, OwnedSemaphorePermit, Semaphore},
    task::spawn_blocking,
};
use tracing::{error, info, warn};

use crate::{Error, Result};

const DEFAULT_ACCOUNT_ATTEMPTS: usize = 5;
const DEFAULT_ACCOUNT_WINDOW_SECONDS: u64 = 300;
const DEFAULT_ARGON2_MAX_CONCURRENT: usize = 2;
const DEFAULT_CLIENT_ATTEMPTS: usize = 20;
const DEFAULT_CLIENT_WINDOW_SECONDS: u64 = 300;
const DEFAULT_MAX_IDENTITIES: usize = 10_000;
const MAX_FORWARDED_ADDRESSES: usize = 16;

const DUMMY_PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$OUR3N0tETVhFWk1TQVF2WQ$G9FfWl6jbEKd/ER1N4kHkkI2orZRdyAfzOtrJcUMOHE";

/// The client identity associated with submitted login credentials.
#[derive(Clone, Copy)]
pub enum ClientIdentity {
    /// A client resolved from the transport peer and trusted forwarding chain.
    Address(IpAddr),
    /// A trusted proxy supplied an invalid forwarding chain.
    Invalid,
}

/// Validated authentication security configuration.
pub struct AuthConfig {
    account_attempts: usize,
    account_window: Duration,
    argon2_max_concurrent: usize,
    client_attempts: usize,
    client_window: Duration,
    max_identities: usize,
    trusted_proxies: Vec<IpNet>,
}

impl AuthConfig {
    /// Reads and validates authentication security configuration from the environment.
    ///
    /// # Errors
    /// Returns [`Error::Configuration`] when a configured value is invalid.
    pub fn from_env() -> Result<Self> {
        let account_attempts = positive_usize("AUTH_ACCOUNT_ATTEMPTS", DEFAULT_ACCOUNT_ATTEMPTS)?;
        let account_window = Duration::from_secs(positive_u64(
            "AUTH_ACCOUNT_WINDOW_SECONDS",
            DEFAULT_ACCOUNT_WINDOW_SECONDS,
        )?);
        let argon2_max_concurrent =
            positive_usize("AUTH_ARGON2_MAX_CONCURRENT", DEFAULT_ARGON2_MAX_CONCURRENT)?;
        let client_attempts = positive_usize("AUTH_CLIENT_ATTEMPTS", DEFAULT_CLIENT_ATTEMPTS)?;
        let client_window = Duration::from_secs(positive_u64(
            "AUTH_CLIENT_WINDOW_SECONDS",
            DEFAULT_CLIENT_WINDOW_SECONDS,
        )?);
        let max_identities =
            positive_usize("AUTH_RATE_LIMIT_MAX_IDENTITIES", DEFAULT_MAX_IDENTITIES)?;
        let trusted_proxies = trusted_proxies()?;

        Ok(Self {
            account_attempts,
            account_window,
            argon2_max_concurrent,
            client_attempts,
            client_window,
            max_identities,
            trusted_proxies,
        })
    }
}

/// Shared authentication security state.
pub struct AuthSecurity {
    account_limiter: RateLimiter<AccountIdentity>,
    argon2_slots: Arc<Semaphore>,
    client_limiter: RateLimiter<IpAddr>,
    trusted_proxies: Vec<IpNet>,
}

impl AuthSecurity {
    #[must_use]
    pub fn new(config: AuthConfig) -> Self {
        Self {
            account_limiter: RateLimiter::new(
                config.account_attempts,
                config.account_window,
                config.max_identities,
            ),
            argon2_slots: Arc::new(Semaphore::new(config.argon2_max_concurrent)),
            client_limiter: RateLimiter::new(
                config.client_attempts,
                config.client_window,
                config.max_identities,
            ),
            trusted_proxies: config.trusted_proxies,
        }
    }

    /// Resolves a client address, honoring forwarding data only from trusted proxies.
    #[must_use]
    pub fn resolve_client(&self, peer: IpAddr, headers: &HeaderMap) -> ClientIdentity {
        let peer = normalize_ip(peer);
        if !self.is_trusted_proxy(peer) {
            return ClientIdentity::Address(peer);
        }

        let values = headers.get_all("x-forwarded-for");
        if values.iter().next().is_none() {
            return ClientIdentity::Address(peer);
        }

        let mut addresses = Vec::new();
        for value in values {
            let Ok(value) = value.to_str() else {
                return ClientIdentity::Invalid;
            };

            for address in value.split(',') {
                if addresses.len() == MAX_FORWARDED_ADDRESSES {
                    return ClientIdentity::Invalid;
                }
                let Ok(address) = address.trim().parse().map(normalize_ip) else {
                    return ClientIdentity::Invalid;
                };
                addresses.push(address);
            }
        }

        let Some(first_address) = addresses.first().copied() else {
            return ClientIdentity::Invalid;
        };
        let client = addresses
            .iter()
            .rev()
            .copied()
            .find(|address| !self.is_trusted_proxy(*address))
            .unwrap_or(first_address);

        ClientIdentity::Address(client)
    }

    /// Reserves one attempt for a client identity.
    pub async fn allow_client(&self, client: ClientIdentity) -> bool {
        let ClientIdentity::Address(address) = client else {
            warn!(
                event = "authentication_rejected",
                reason = "invalid_forwarding_chain",
                "authentication rejected"
            );
            return false;
        };

        if self.client_limiter.allow(address).await {
            true
        } else {
            warn!(
                event = "authentication_rejected",
                reason = "rate_limited",
                dimension = "client",
                "authentication rejected"
            );
            false
        }
    }

    /// Reserves one attempt for a known or unknown account identity.
    pub async fn allow_account(&self, user_id: Option<i64>, username: &str) -> bool {
        let identity = user_id.map_or_else(
            || AccountIdentity::Unknown(Sha256::digest(username.as_bytes()).into()),
            AccountIdentity::Known,
        );

        if self.account_limiter.allow(identity).await {
            true
        } else {
            warn!(
                event = "authentication_rejected",
                reason = "rate_limited",
                dimension = "account",
                "authentication rejected"
            );
            false
        }
    }

    /// Reserves capacity for one password verification.
    pub fn verification_permit(&self) -> Option<OwnedSemaphorePermit> {
        let Ok(permit) = Arc::clone(&self.argon2_slots).try_acquire_owned() else {
            warn!(
                event = "authentication_rejected",
                reason = "verification_capacity",
                "authentication rejected"
            );
            return None;
        };

        Some(permit)
    }

    /// Verifies a password using previously reserved Argon2 capacity.
    pub async fn verify_password(
        &self,
        password: String,
        password_hash: Option<String>,
        permit: OwnedSemaphorePermit,
    ) -> bool {
        let account_exists = password_hash.is_some();

        let result = spawn_blocking(move || {
            let _permit = permit;
            check_password(password.as_bytes(), password_hash.as_deref())
        })
        .await;

        match result {
            Ok(PasswordCheck::Match) if account_exists => {
                info!(
                    event = "authentication_succeeded",
                    "authentication succeeded"
                );
                true
            }
            Ok(PasswordCheck::InvalidHash) => {
                error!(
                    event = "authentication_configuration_error",
                    reason = "invalid_password_hash",
                    "stored password hash is unusable"
                );
                warn!(
                    event = "authentication_rejected",
                    reason = "invalid_credentials",
                    "authentication rejected"
                );
                false
            }
            Ok(PasswordCheck::Match | PasswordCheck::Mismatch) => {
                warn!(
                    event = "authentication_rejected",
                    reason = "invalid_credentials",
                    "authentication rejected"
                );
                false
            }
            Err(error) => {
                error!(
                    event = "authentication_verification_error",
                    reason = "blocking_task_failed",
                    %error,
                    "password verification task failed"
                );
                warn!(
                    event = "authentication_rejected",
                    reason = "invalid_credentials",
                    "authentication rejected"
                );
                false
            }
        }
    }

    fn is_trusted_proxy(&self, address: IpAddr) -> bool {
        self.trusted_proxies
            .iter()
            .any(|network| network.contains(&address))
    }
}

#[derive(Eq, Hash, PartialEq)]
enum AccountIdentity {
    Known(i64),
    Unknown([u8; 32]),
}

struct RateLimiter<K> {
    attempts: Mutex<HashMap<K, VecDeque<Instant>>>,
    limit: usize,
    max_identities: usize,
    window: Duration,
}

impl<K> RateLimiter<K>
where
    K: Eq + Hash,
{
    fn new(limit: usize, window: Duration, max_identities: usize) -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
            limit,
            max_identities,
            window,
        }
    }

    async fn allow(&self, identity: K) -> bool {
        self.allow_at(identity, Instant::now()).await
    }

    async fn allow_at(&self, identity: K, now: Instant) -> bool {
        let mut attempts = self.attempts.lock().await;

        if let Some(identity_attempts) = attempts.get_mut(&identity) {
            discard_expired(identity_attempts, now, self.window);
            if identity_attempts.len() >= self.limit {
                return false;
            }
            identity_attempts.push_back(now);
            return true;
        }

        if attempts.len() >= self.max_identities {
            attempts.retain(|_, identity_attempts| {
                discard_expired(identity_attempts, now, self.window);
                !identity_attempts.is_empty()
            });
            if attempts.len() >= self.max_identities {
                return false;
            }
        }

        attempts.insert(identity, VecDeque::from([now]));
        true
    }
}

#[derive(Debug, Eq, PartialEq)]
enum PasswordCheck {
    Match,
    Mismatch,
    InvalidHash,
}

fn check_password(password: &[u8], password_hash: Option<&str>) -> PasswordCheck {
    let Some(password_hash) = password_hash else {
        run_dummy_verification(password);
        return PasswordCheck::Mismatch;
    };
    let Ok(password_hash) = PasswordHash::new(password_hash) else {
        run_dummy_verification(password);
        return PasswordCheck::InvalidHash;
    };

    match Argon2::default().verify_password(password, &password_hash) {
        Ok(()) => PasswordCheck::Match,
        Err(password_hash::Error::Password) => PasswordCheck::Mismatch,
        Err(_) => {
            run_dummy_verification(password);
            PasswordCheck::InvalidHash
        }
    }
}

fn run_dummy_verification(password: &[u8]) {
    if let Ok(dummy_hash) = PasswordHash::new(DUMMY_PASSWORD_HASH) {
        let _ = Argon2::default().verify_password(password, &dummy_hash);
    }
}

fn discard_expired(attempts: &mut VecDeque<Instant>, now: Instant, window: Duration) {
    while attempts
        .front()
        .is_some_and(|attempt| now.duration_since(*attempt) >= window)
    {
        attempts.pop_front();
    }
}

fn normalize_ip(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V6(address) => address
            .to_ipv4_mapped()
            .map_or(IpAddr::V6(address), IpAddr::V4),
        IpAddr::V4(_) => address,
    }
}

fn positive_usize(name: &str, default: usize) -> Result<usize> {
    positive_number(name, default, str::parse)
}

fn positive_u64(name: &str, default: u64) -> Result<u64> {
    positive_number(name, default, str::parse)
}

fn positive_number<T, E>(
    name: &str,
    default: T,
    parse: impl FnOnce(&str) -> core::result::Result<T, E>,
) -> Result<T>
where
    E: Display,
    T: Copy + PartialEq + From<u8>,
{
    match var(name) {
        Ok(value) => {
            let parsed = parse(&value).map_err(|error| {
                Error::Configuration(format!("{name} must be a positive integer: {error}"))
            })?;
            if parsed == T::from(0) {
                return Err(Error::Configuration(format!(
                    "{name} must be greater than zero"
                )));
            }
            Ok(parsed)
        }
        Err(VarError::NotPresent) => Ok(default),
        Err(VarError::NotUnicode(_)) => Err(Error::Configuration(format!(
            "{name} must contain valid Unicode"
        ))),
    }
}

fn trusted_proxies() -> Result<Vec<IpNet>> {
    let value = match var("AUTH_TRUSTED_PROXY_CIDRS") {
        Ok(value) => value,
        Err(VarError::NotPresent) => return Ok(Vec::new()),
        Err(VarError::NotUnicode(_)) => {
            return Err(Error::Configuration(
                "AUTH_TRUSTED_PROXY_CIDRS must contain valid Unicode".to_string(),
            ));
        }
    };

    value
        .split(',')
        .map(str::trim)
        .filter(|network| !network.is_empty())
        .map(|network| {
            network.parse().map_err(|error| {
                Error::Configuration(format!(
                    "invalid AUTH_TRUSTED_PROXY_CIDRS entry {network:?}: {error}"
                ))
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{
        net::{IpAddr, Ipv4Addr},
        time::Duration,
    };

    use axum::http::{HeaderMap, HeaderValue};

    use super::{
        AuthConfig, AuthSecurity, ClientIdentity, DUMMY_PASSWORD_HASH, PasswordCheck, RateLimiter,
        check_password,
    };

    #[test]
    fn untrusted_peer_ignores_forwarded_header() {
        let security = security(&["10.0.0.0/8"], 2);
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.10"));
        let peer = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));

        assert!(matches!(
            security.resolve_client(peer, &headers),
            ClientIdentity::Address(address) if address == peer
        ));
    }

    #[test]
    fn trusted_proxy_uses_nearest_untrusted_forwarded_address() {
        let security = security(&["10.0.0.0/8"], 2);
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("198.51.100.8, 203.0.113.10, 10.0.0.2"),
        );

        assert!(matches!(
            security.resolve_client(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 3)), &headers),
            ClientIdentity::Address(IpAddr::V4(address))
                if address == Ipv4Addr::new(203, 0, 113, 10)
        ));
    }

    #[test]
    fn trusted_proxy_rejects_malformed_or_excessive_forwarding_chains() {
        let security = security(&["10.0.0.0/8"], 2);
        let peer = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 3));
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("not-an-address"),
        );
        assert!(matches!(
            security.resolve_client(peer, &headers),
            ClientIdentity::Invalid
        ));

        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static(
                "1.1.1.1,1.1.1.2,1.1.1.3,1.1.1.4,1.1.1.5,1.1.1.6,1.1.1.7,1.1.1.8,1.1.1.9,1.1.1.10,1.1.1.11,1.1.1.12,1.1.1.13,1.1.1.14,1.1.1.15,1.1.1.16,1.1.1.17",
            ),
        );
        assert!(matches!(
            security.resolve_client(peer, &headers),
            ClientIdentity::Invalid
        ));
    }

    #[test]
    fn ipv4_mapped_peer_is_normalized_before_proxy_matching() {
        let security = security(&["192.0.2.0/24"], 2);
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.10"));
        let peer = IpAddr::V6(Ipv4Addr::new(192, 0, 2, 1).to_ipv6_mapped());

        assert!(matches!(
            security.resolve_client(peer, &headers),
            ClientIdentity::Address(IpAddr::V4(address))
                if address == Ipv4Addr::new(203, 0, 113, 10)
        ));
    }

    #[tokio::test]
    async fn limiter_enforces_independent_sliding_windows() {
        let limiter = RateLimiter::new(2, Duration::from_mins(1), 10);
        let start = std::time::Instant::now();

        assert!(limiter.allow_at("first", start).await);
        assert!(limiter.allow_at("first", start).await);
        assert!(!limiter.allow_at("first", start).await);
        assert!(limiter.allow_at("second", start).await);
        assert!(
            limiter
                .allow_at("first", start + Duration::from_secs(61))
                .await
        );
    }

    #[tokio::test]
    async fn limiter_fails_closed_at_capacity_and_reclaims_expired_keys() {
        let limiter = RateLimiter::new(2, Duration::from_mins(1), 1);
        let start = std::time::Instant::now();

        assert!(limiter.allow_at("first", start).await);
        assert!(!limiter.allow_at("second", start).await);
        assert!(
            limiter
                .allow_at("second", start + Duration::from_secs(61))
                .await
        );
    }

    #[tokio::test]
    async fn limiter_retains_attempts_when_window_exceeds_process_uptime() {
        let limiter = RateLimiter::new(1, Duration::MAX, 10);
        let start = std::time::Instant::now();

        assert!(limiter.allow_at("identity", start).await);
        assert!(!limiter.allow_at("identity", start).await);
    }

    #[test]
    fn password_checks_cover_known_unknown_and_invalid_hashes() {
        assert_eq!(
            check_password(b"9DEV-45 fixed dummy password", Some(DUMMY_PASSWORD_HASH)),
            PasswordCheck::Match
        );
        assert_eq!(
            check_password(b"wrong password", Some(DUMMY_PASSWORD_HASH)),
            PasswordCheck::Mismatch
        );
        assert_eq!(
            check_password(b"9DEV-45 fixed dummy password", None),
            PasswordCheck::Mismatch
        );
        assert_eq!(
            check_password(b"password", Some("invalid hash")),
            PasswordCheck::InvalidHash
        );
    }

    #[tokio::test]
    async fn password_verification_rejects_when_concurrency_is_saturated() {
        let security = security(&[], 1);
        let permit = security.verification_permit();
        assert!(permit.is_some());

        let saturated = security.verification_permit().is_none();
        drop(permit);

        assert!(saturated);
    }

    fn security(trusted_proxies: &[&str], argon2_max_concurrent: usize) -> AuthSecurity {
        AuthSecurity::new(AuthConfig {
            account_attempts: 5,
            account_window: Duration::from_mins(5),
            argon2_max_concurrent,
            client_attempts: 20,
            client_window: Duration::from_mins(5),
            max_identities: 100,
            trusted_proxies: trusted_proxies
                .iter()
                .filter_map(|network| network.parse().ok())
                .collect(),
        })
    }
}
