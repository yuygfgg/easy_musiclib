use crate::infra::sqlite::db::now_ms;
use anyhow::{Result, anyhow};
use sqlx::SqlitePool;

pub async fn set_liked(pool: &SqlitePool, kind: &str, id: i64, liked: bool) -> Result<()> {
    let table = match kind {
        "tracks" | "albums" | "artists" | "events" => kind,
        _ => return Err(anyhow!("unsupported like kind {kind}")),
    };
    let sql = format!("UPDATE {table} SET liked_at = ? WHERE id = ?");
    let liked_at = liked.then(now_ms);
    sqlx::query(&sql)
        .bind(liked_at)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
