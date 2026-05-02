use super::albums::album_summary_from_row;
use anyhow::Result;
use easy_musiclib_media::normalize::normalize_name;
use easy_musiclib_shared::*;
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};

pub async fn fetch_event_summary(pool: &SqlitePool, id: i64) -> Result<EventSummary> {
    let row = sqlx::query(
        "SELECT e.id, e.uuid, e.name, e.year, e.date, e.liked_at,
            (SELECT COUNT(*) FROM event_albums ea WHERE ea.event_id = e.id) AS album_count
         FROM events e WHERE e.id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    event_summary_from_row(row)
}

fn event_summary_from_row(row: sqlx::sqlite::SqliteRow) -> Result<EventSummary> {
    Ok(EventSummary {
        id: row.try_get("id")?,
        uuid: row.try_get("uuid")?,
        name: row.try_get("name")?,
        year: row.try_get("year")?,
        date: row.try_get("date")?,
        liked_at: row.try_get("liked_at")?,
        album_count: row.try_get("album_count")?,
    })
}

pub async fn fetch_event_detail(pool: &SqlitePool, id: i64) -> Result<EventDetail> {
    let summary = fetch_event_summary(pool, id).await?;
    let rows = sqlx::query(
        "SELECT al.id, al.uuid, al.title, al.artwork_id, al.year, al.date, al.liked_at,
            ev.id AS event_id, ev.uuid AS event_uuid, ev.name AS event_name,
            (SELECT COUNT(*) FROM tracks t WHERE t.album_id = al.id) AS song_count
         FROM albums al
         JOIN event_albums ea ON ea.album_id = al.id
         JOIN events ev ON ev.id = ea.event_id
         WHERE ea.event_id = ?
         ORDER BY COALESCE(al.year, 0) DESC, al.title",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let mut albums = Vec::new();
    for row in rows {
        albums.push(album_summary_from_row(pool, row).await?);
    }
    Ok(EventDetail { summary, albums })
}

pub async fn list_events(
    pool: &SqlitePool,
    cursor: Option<i64>,
    offset: Option<i64>,
    limit: i64,
    liked: Option<bool>,
    q: Option<String>,
) -> Result<ListResponse<EventSummary>> {
    let limit = limit.clamp(1, 200);
    let offset = offset.map(|v| v.max(0));
    let q_norm = q
        .as_ref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| format!("%{}%", normalize_name(s, true)));
    let total = if offset.is_some() {
        let mut count_qb =
            QueryBuilder::<Sqlite>::new("SELECT COUNT(*) AS total FROM events e WHERE 1=1");
        if let Some(liked) = liked {
            count_qb.push(if liked {
                " AND e.liked_at IS NOT NULL"
            } else {
                " AND e.liked_at IS NULL"
            });
        }
        if let Some(q_norm) = q_norm.as_ref() {
            count_qb.push(" AND e.name_norm LIKE ").push_bind(q_norm);
        }
        Some(
            count_qb
                .build()
                .fetch_one(pool)
                .await?
                .try_get::<i64, _>("total")?,
        )
    } else {
        None
    };
    let mut qb = QueryBuilder::<Sqlite>::new(
        "SELECT e.id, e.uuid, e.name, e.year, e.date, e.liked_at,
            (SELECT COUNT(*) FROM event_albums ea WHERE ea.event_id = e.id) AS album_count
         FROM events e WHERE 1=1",
    );
    if offset.is_none() {
        if let Some(cursor) = cursor {
            qb.push(" AND e.id > ").push_bind(cursor);
        }
    }
    if let Some(liked) = liked {
        qb.push(if liked {
            " AND e.liked_at IS NOT NULL"
        } else {
            " AND e.liked_at IS NULL"
        });
    }
    if let Some(q_norm) = q_norm.as_ref() {
        qb.push(" AND e.name_norm LIKE ").push_bind(q_norm);
    }
    if liked == Some(true) {
        qb.push(" ORDER BY e.liked_at DESC, e.id DESC LIMIT ");
    } else {
        qb.push(" ORDER BY COALESCE(e.year, 0) DESC, e.name, e.id LIMIT ");
    }
    qb.push_bind(limit + 1);
    if let Some(offset) = offset {
        qb.push(" OFFSET ").push_bind(offset);
    }
    let rows = qb.build().fetch_all(pool).await?;
    let mut items = Vec::new();
    let mut next_cursor = None;
    for (idx, row) in rows.into_iter().enumerate() {
        if idx as i64 >= limit {
            next_cursor = Some(row.try_get("id")?);
            break;
        }
        items.push(event_summary_from_row(row)?);
    }
    Ok(ListResponse {
        items,
        next_cursor,
        total,
    })
}
