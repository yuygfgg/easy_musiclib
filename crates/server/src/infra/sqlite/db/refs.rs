use anyhow::{Result, anyhow};
use easy_musiclib_media::normalize::normalize_name;
use easy_musiclib_shared::EntityRef;
use sqlx::{Row, SqlitePool};

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn is_unknown_event_name(name: &str) -> bool {
    let compact: String = normalize_name(name, false)
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '_' && *ch != '-')
        .collect();
    compact == "unknownevent"
}

pub async fn resolve_id(pool: &SqlitePool, kind: &str, ident: &str) -> Result<i64> {
    let table = match kind {
        "tracks" | "albums" | "artists" | "events" => kind,
        _ => return Err(anyhow!("unsupported entity kind {kind}")),
    };
    if let Ok(id) = ident.parse::<i64>() {
        let sql = format!("SELECT id FROM {table} WHERE id = ?");
        if let Some(row) = sqlx::query(&sql).bind(id).fetch_optional(pool).await? {
            return Ok(row.try_get("id")?);
        }
    }
    let sql = format!("SELECT id FROM {table} WHERE uuid = ?");
    if let Some(row) = sqlx::query(&sql).bind(ident).fetch_optional(pool).await? {
        return Ok(row.try_get("id")?);
    }
    if kind == "artists" {
        if let Some(row) = sqlx::query(
            "SELECT target_artist_id AS id FROM artist_merge_audit WHERE source_artist_uuid = ? ORDER BY id DESC LIMIT 1",
        )
        .bind(ident)
        .fetch_optional(pool)
        .await?
        {
            return Ok(row.try_get("id")?);
        }
        let norm = normalize_name(ident, false);
        if let Some(row) =
            sqlx::query("SELECT artist_id AS id FROM artist_aliases WHERE alias_norm = ?")
                .bind(norm)
                .fetch_optional(pool)
                .await?
        {
            return Ok(row.try_get("id")?);
        }
    }
    Err(anyhow!("{kind} not found: {ident}"))
}

pub async fn entity_ref(pool: &SqlitePool, table: &str, id: i64) -> Result<EntityRef> {
    let (name_col, table) = match table {
        "tracks" => ("title", "tracks"),
        "albums" => ("title", "albums"),
        "artists" => ("name", "artists"),
        "events" => ("name", "events"),
        _ => return Err(anyhow!("unsupported ref table {table}")),
    };
    let row = sqlx::query(&format!(
        "SELECT id, uuid, {name_col} AS name FROM {table} WHERE id = ?"
    ))
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(EntityRef {
        id: row.try_get("id")?,
        uuid: row.try_get("uuid")?,
        name: row.try_get("name")?,
    })
}

pub async fn track_artists(pool: &SqlitePool, track_id: i64) -> Result<Vec<EntityRef>> {
    let rows = sqlx::query(
        "SELECT a.id, a.uuid, a.name
         FROM artists a
         JOIN track_artists ta ON ta.artist_id = a.id
         WHERE ta.track_id = ?
         ORDER BY ta.position, a.name",
    )
    .bind(track_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(row_to_entity_ref).collect()
}

pub async fn album_artists(pool: &SqlitePool, album_id: i64) -> Result<Vec<EntityRef>> {
    let rows = sqlx::query(
        "SELECT a.id, a.uuid, a.name
         FROM artists a
         JOIN album_artists aa ON aa.artist_id = a.id
         WHERE aa.album_id = ?
         ORDER BY aa.position, a.name",
    )
    .bind(album_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(row_to_entity_ref).collect()
}

pub(super) fn optional_ref(
    row: &sqlx::sqlite::SqliteRow,
    id_col: &str,
    uuid_col: &str,
    name_col: &str,
) -> Result<Option<EntityRef>> {
    let id: Option<i64> = row.try_get(id_col)?;
    Ok(match id {
        Some(id) => Some(EntityRef {
            id,
            uuid: row.try_get(uuid_col)?,
            name: row.try_get(name_col)?,
        }),
        None => None,
    })
}

fn row_to_entity_ref(row: sqlx::sqlite::SqliteRow) -> Result<EntityRef> {
    Ok(EntityRef {
        id: row.try_get("id")?,
        uuid: row.try_get("uuid")?,
        name: row.try_get("name")?,
    })
}
