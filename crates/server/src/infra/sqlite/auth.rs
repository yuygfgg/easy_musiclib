use crate::application::auth::{
    AccountRecord, AuthRepository, NewAccount, NewSession, SessionUser,
};
use crate::domain::AccountSummary;
use anyhow::Result;
use futures::FutureExt;
use futures::future::BoxFuture;
use sqlx::{Row, SqlitePool};

#[derive(Clone)]
pub struct SqliteAuthRepository {
    pool: SqlitePool,
}

impl SqliteAuthRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl AuthRepository for SqliteAuthRepository {
    fn account_count(&self) -> BoxFuture<'_, Result<i64>> {
        async move { account_count(&self.pool).await }.boxed()
    }

    fn list_accounts(&self) -> BoxFuture<'_, Result<Vec<AccountSummary>>> {
        async move { list_accounts(&self.pool).await }.boxed()
    }

    fn find_account(&self, username_norm: String) -> BoxFuture<'_, Result<Option<AccountRecord>>> {
        async move { find_account(&self.pool, &username_norm).await }.boxed()
    }

    fn create_account(&self, account: NewAccount) -> BoxFuture<'_, Result<AccountSummary>> {
        async move { create_account(&self.pool, account).await }.boxed()
    }

    fn update_account_password(
        &self,
        username_norm: String,
        password_hash: String,
    ) -> BoxFuture<'_, Result<Option<AccountSummary>>> {
        async move { update_account_password(&self.pool, &username_norm, &password_hash).await }
            .boxed()
    }

    fn delete_account(&self, username_norm: String) -> BoxFuture<'_, Result<bool>> {
        async move { delete_account(&self.pool, &username_norm).await }.boxed()
    }

    fn create_session(&self, session: NewSession) -> BoxFuture<'_, Result<()>> {
        async move { create_session(&self.pool, session).await }.boxed()
    }

    fn session_user(
        &self,
        token_hash: String,
        now_ms: i64,
    ) -> BoxFuture<'_, Result<Option<SessionUser>>> {
        async move { session_user(&self.pool, &token_hash, now_ms).await }.boxed()
    }

    fn touch_session(&self, token_hash: String, now_ms: i64) -> BoxFuture<'_, Result<()>> {
        async move { touch_session(&self.pool, &token_hash, now_ms).await }.boxed()
    }

    fn delete_session(&self, token_hash: String) -> BoxFuture<'_, Result<()>> {
        async move { delete_session(&self.pool, &token_hash).await }.boxed()
    }

    fn delete_expired_sessions(&self, now_ms: i64) -> BoxFuture<'_, Result<()>> {
        async move { delete_expired_sessions(&self.pool, now_ms).await }.boxed()
    }
}

async fn account_count(pool: &SqlitePool) -> Result<i64> {
    let row = sqlx::query("SELECT COUNT(*) AS count FROM accounts")
        .fetch_one(pool)
        .await?;
    Ok(row.try_get("count")?)
}

async fn list_accounts(pool: &SqlitePool) -> Result<Vec<AccountSummary>> {
    let rows = sqlx::query(
        "SELECT username, created_at, updated_at
         FROM accounts
         ORDER BY username_norm",
    )
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok(AccountSummary {
                username: row.try_get("username")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })
        })
        .collect()
}

async fn find_account(pool: &SqlitePool, username_norm: &str) -> Result<Option<AccountRecord>> {
    let Some(row) = sqlx::query(
        "SELECT id, username, password_hash
         FROM accounts
         WHERE username_norm = ?",
    )
    .bind(username_norm)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    Ok(Some(AccountRecord {
        id: row.try_get("id")?,
        username: row.try_get("username")?,
        password_hash: row.try_get("password_hash")?,
    }))
}

async fn create_account(pool: &SqlitePool, account: NewAccount) -> Result<AccountSummary> {
    sqlx::query(
        "INSERT INTO accounts (username, username_norm, password_hash, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&account.username)
    .bind(&account.username_norm)
    .bind(&account.password_hash)
    .bind(account.created_at)
    .bind(account.updated_at)
    .execute(pool)
    .await?;
    Ok(AccountSummary {
        username: account.username,
        created_at: account.created_at,
        updated_at: account.updated_at,
    })
}

async fn update_account_password(
    pool: &SqlitePool,
    username_norm: &str,
    password_hash: &str,
) -> Result<Option<AccountSummary>> {
    let now = chrono::Utc::now().timestamp_millis();
    let mut tx = pool.begin().await?;
    let result = sqlx::query(
        "UPDATE accounts
         SET password_hash = ?, updated_at = ?
         WHERE username_norm = ?",
    )
    .bind(password_hash)
    .bind(now)
    .bind(username_norm)
    .execute(&mut *tx)
    .await?;
    if result.rows_affected() == 0 {
        tx.commit().await?;
        return Ok(None);
    }
    sqlx::query(
        "DELETE FROM auth_sessions
         WHERE account_id = (SELECT id FROM accounts WHERE username_norm = ?)",
    )
    .bind(username_norm)
    .execute(&mut *tx)
    .await?;

    let summary = if let Some(row) = sqlx::query(
        "SELECT username, created_at, updated_at
         FROM accounts
         WHERE username_norm = ?",
    )
    .bind(username_norm)
    .fetch_optional(&mut *tx)
    .await?
    {
        Some(AccountSummary {
            username: row.try_get("username")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    } else {
        None
    };
    tx.commit().await?;
    Ok(summary)
}

async fn delete_account(pool: &SqlitePool, username_norm: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM accounts WHERE username_norm = ?")
        .bind(username_norm)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

async fn create_session(pool: &SqlitePool, session: NewSession) -> Result<()> {
    sqlx::query(
        "INSERT INTO auth_sessions (token_hash, account_id, created_at, expires_at, last_seen_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&session.token_hash)
    .bind(session.account_id)
    .bind(session.created_at)
    .bind(session.expires_at)
    .bind(session.last_seen_at)
    .execute(pool)
    .await?;
    Ok(())
}

async fn session_user(
    pool: &SqlitePool,
    token_hash: &str,
    now_ms: i64,
) -> Result<Option<SessionUser>> {
    let Some(row) = sqlx::query(
        "SELECT accounts.username AS username
         FROM auth_sessions
         JOIN accounts ON accounts.id = auth_sessions.account_id
         WHERE auth_sessions.token_hash = ? AND auth_sessions.expires_at > ?",
    )
    .bind(token_hash)
    .bind(now_ms)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    Ok(Some(SessionUser {
        username: row.try_get("username")?,
    }))
}

async fn touch_session(pool: &SqlitePool, token_hash: &str, now_ms: i64) -> Result<()> {
    sqlx::query("UPDATE auth_sessions SET last_seen_at = ? WHERE token_hash = ?")
        .bind(now_ms)
        .bind(token_hash)
        .execute(pool)
        .await?;
    Ok(())
}

async fn delete_session(pool: &SqlitePool, token_hash: &str) -> Result<()> {
    sqlx::query("DELETE FROM auth_sessions WHERE token_hash = ?")
        .bind(token_hash)
        .execute(pool)
        .await?;
    Ok(())
}

async fn delete_expired_sessions(pool: &SqlitePool, now_ms: i64) -> Result<()> {
    sqlx::query("DELETE FROM auth_sessions WHERE expires_at <= ?")
        .bind(now_ms)
        .execute(pool)
        .await?;
    Ok(())
}
