use crate::domain::AccountSummary;
use anyhow::{Context, Result, anyhow};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use futures::future::BoxFuture;
use sha2::{Digest, Sha256};

const ARGON2_MEMORY_KIB: u32 = 64 * 1024;
const ARGON2_ITERATIONS: u32 = 3;
const ARGON2_PARALLELISM: u32 = 1;
const ARGON2_OUTPUT_BYTES: usize = 32;
const SESSION_TOKEN_BYTES: usize = 32;
const SESSION_TTL_MS: i64 = 30 * 24 * 60 * 60 * 1000;
const MIN_PASSWORD_CHARS: usize = 12;
const MAX_PASSWORD_BYTES: usize = 1024;
const MAX_USERNAME_CHARS: usize = 64;

pub trait AuthRepository: Send + Sync {
    fn account_count(&self) -> BoxFuture<'_, Result<i64>>;

    fn list_accounts(&self) -> BoxFuture<'_, Result<Vec<AccountSummary>>>;

    fn find_account(&self, username_norm: String) -> BoxFuture<'_, Result<Option<AccountRecord>>>;

    fn create_account(&self, account: NewAccount) -> BoxFuture<'_, Result<AccountSummary>>;

    fn update_account_password(
        &self,
        username_norm: String,
        password_hash: String,
    ) -> BoxFuture<'_, Result<Option<AccountSummary>>>;

    fn delete_account(&self, username_norm: String) -> BoxFuture<'_, Result<bool>>;

    fn create_session(&self, session: NewSession) -> BoxFuture<'_, Result<()>>;

    fn session_user(
        &self,
        token_hash: String,
        now_ms: i64,
    ) -> BoxFuture<'_, Result<Option<SessionUser>>>;

    fn touch_session(&self, token_hash: String, now_ms: i64) -> BoxFuture<'_, Result<()>>;

    fn delete_session(&self, token_hash: String) -> BoxFuture<'_, Result<()>>;

    fn delete_expired_sessions(&self, now_ms: i64) -> BoxFuture<'_, Result<()>>;
}

#[derive(Debug, Clone)]
pub struct AccountRecord {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
}

#[derive(Debug, Clone)]
pub struct NewAccount {
    pub username: String,
    pub username_norm: String,
    pub password_hash: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct NewSession {
    pub account_id: i64,
    pub token_hash: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub last_seen_at: i64,
}

#[derive(Debug, Clone)]
pub struct SessionUser {
    pub username: String,
}

#[derive(Debug, Clone)]
pub struct LoginSession {
    pub username: String,
    pub token: String,
    pub max_age_seconds: i64,
}

#[derive(Debug)]
pub enum AuthError {
    InvalidInput(String),
    InvalidCredentials,
    NotFound(String),
    Internal(anyhow::Error),
}

impl From<anyhow::Error> for AuthError {
    fn from(value: anyhow::Error) -> Self {
        Self::Internal(value)
    }
}

pub type AuthResult<T> = std::result::Result<T, AuthError>;

pub async fn account_count(repository: &impl AuthRepository) -> Result<i64> {
    repository.account_count().await
}

pub async fn list_accounts(repository: &impl AuthRepository) -> Result<Vec<AccountSummary>> {
    repository.list_accounts().await
}

pub async fn create_account(
    repository: &impl AuthRepository,
    username: String,
    password: String,
) -> AuthResult<AccountSummary> {
    let username = validate_username(username)?;
    let username_norm = normalize_username(&username);
    if repository
        .find_account(username_norm.clone())
        .await
        .map_err(AuthError::from)?
        .is_some()
    {
        return Err(AuthError::InvalidInput(
            "account already exists".to_string(),
        ));
    }
    let password_hash = hash_password(&password)?;
    let now = now_ms();
    repository
        .create_account(NewAccount {
            username,
            username_norm,
            password_hash,
            created_at: now,
            updated_at: now,
        })
        .await
        .map_err(AuthError::from)
}

pub async fn update_account_password(
    repository: &impl AuthRepository,
    username: String,
    password: String,
) -> AuthResult<AccountSummary> {
    let username_norm = normalize_username(&validate_username(username)?);
    let password_hash = hash_password(&password)?;
    repository
        .update_account_password(username_norm, password_hash)
        .await
        .map_err(AuthError::from)?
        .ok_or_else(|| AuthError::NotFound("account not found".to_string()))
}

pub async fn delete_account(
    repository: &impl AuthRepository,
    username: String,
) -> AuthResult<bool> {
    let username_norm = normalize_username(&validate_username(username)?);
    repository
        .delete_account(username_norm)
        .await
        .map_err(AuthError::from)
}

pub async fn login(
    repository: &impl AuthRepository,
    username: String,
    password: String,
) -> AuthResult<LoginSession> {
    let username_norm = normalize_username(&validate_username(username)?);
    let Some(account) = repository
        .find_account(username_norm)
        .await
        .map_err(AuthError::from)?
    else {
        return Err(AuthError::InvalidCredentials);
    };
    if !verify_password(&password, &account.password_hash)? {
        return Err(AuthError::InvalidCredentials);
    }

    let token = new_session_token().map_err(AuthError::from)?;
    let token_hash = session_token_hash(&token);
    let now = now_ms();
    repository
        .delete_expired_sessions(now)
        .await
        .map_err(AuthError::from)?;
    repository
        .create_session(NewSession {
            account_id: account.id,
            token_hash,
            created_at: now,
            expires_at: now + SESSION_TTL_MS,
            last_seen_at: now,
        })
        .await
        .map_err(AuthError::from)?;
    Ok(LoginSession {
        username: account.username,
        token,
        max_age_seconds: SESSION_TTL_MS / 1000,
    })
}

pub async fn authenticated_username(
    repository: &impl AuthRepository,
    token: Option<&str>,
) -> Result<Option<String>> {
    let Some(token) = token else {
        return Ok(None);
    };
    let token_hash = session_token_hash(token);
    let now = now_ms();
    let user = repository.session_user(token_hash.clone(), now).await?;
    if user.is_some() {
        repository.touch_session(token_hash, now).await?;
    }
    Ok(user.map(|user| user.username))
}

pub async fn logout(repository: &impl AuthRepository, token: Option<&str>) -> Result<()> {
    if let Some(token) = token {
        repository.delete_session(session_token_hash(token)).await?;
    }
    Ok(())
}

pub fn session_token_hash(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

fn validate_username(username: String) -> AuthResult<String> {
    let username = username.trim().to_string();
    let char_count = username.chars().count();
    if username.is_empty() {
        return Err(AuthError::InvalidInput("username is required".to_string()));
    }
    if char_count > MAX_USERNAME_CHARS {
        return Err(AuthError::InvalidInput("username is too long".to_string()));
    }
    if username
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, '/' | '\\'))
    {
        return Err(AuthError::InvalidInput(
            "username contains invalid characters".to_string(),
        ));
    }
    Ok(username)
}

fn normalize_username(username: &str) -> String {
    username.trim().to_lowercase()
}

fn hash_password(password: &str) -> AuthResult<String> {
    validate_password(password)?;
    let mut salt_bytes = [0_u8; 32];
    fill_random(&mut salt_bytes).map_err(AuthError::from)?;
    let salt = SaltString::encode_b64(&salt_bytes)
        .context("failed to encode password salt")
        .map_err(AuthError::from)?;
    let params = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        Some(ARGON2_OUTPUT_BYTES),
    )
    .context("failed to configure Argon2id")
    .map_err(AuthError::from)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .context("failed to hash password")
        .map_err(AuthError::from)
}

fn verify_password(password: &str, password_hash: &str) -> AuthResult<bool> {
    let parsed_hash = PasswordHash::new(password_hash)
        .context("stored password hash is invalid")
        .map_err(AuthError::from)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

fn validate_password(password: &str) -> AuthResult<()> {
    if password.chars().count() < MIN_PASSWORD_CHARS {
        return Err(AuthError::InvalidInput(
            "password must be at least 12 characters".to_string(),
        ));
    }
    if password.len() > MAX_PASSWORD_BYTES {
        return Err(AuthError::InvalidInput("password is too long".to_string()));
    }
    Ok(())
}

fn new_session_token() -> Result<String> {
    let mut token = [0_u8; SESSION_TOKEN_BYTES];
    fill_random(&mut token)?;
    Ok(URL_SAFE_NO_PAD.encode(token))
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn fill_random(dest: &mut [u8]) -> Result<()> {
    getrandom::fill(dest).map_err(|err| anyhow!("failed to read secure random bytes: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_username_shape() {
        assert_eq!(validate_username("  Alice  ".to_string()).unwrap(), "Alice");
        assert!(matches!(
            validate_username("".to_string()),
            Err(AuthError::InvalidInput(_))
        ));
        assert!(matches!(
            validate_username("a/b".to_string()),
            Err(AuthError::InvalidInput(_))
        ));
        assert!(matches!(
            validate_username("a\nb".to_string()),
            Err(AuthError::InvalidInput(_))
        ));
    }

    #[test]
    fn enforces_minimum_password_length() {
        assert!(validate_password("twelve chars").is_ok());
        assert!(matches!(
            validate_password("short"),
            Err(AuthError::InvalidInput(_))
        ));
    }

    #[test]
    fn password_hash_uses_argon2id_and_verifies() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password("correct horse battery staple", &hash).unwrap());
        assert!(!verify_password("wrong horse battery staple", &hash).unwrap());
    }

    #[test]
    fn session_token_hash_is_stored_as_digest() {
        let digest = session_token_hash("session-token");
        assert_ne!(digest, "session-token");
        assert_eq!(digest.len(), 64);
        assert!(digest.chars().all(|ch| ch.is_ascii_hexdigit()));
    }
}
