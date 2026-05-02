use super::tracks::track_summary_from_row;
use crate::infra::sqlite::db::refs::{album_artists, optional_ref};
use anyhow::Result;
use easy_musiclib_media::normalize::normalize_name;
use easy_musiclib_shared::*;
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};

pub async fn fetch_album_summary(pool: &SqlitePool, id: i64) -> Result<AlbumSummary> {
    let row = sqlx::query(
        "SELECT al.id, al.uuid, al.title, al.artwork_id, al.year, al.date, al.liked_at,
            ev.id AS event_id, ev.uuid AS event_uuid, ev.name AS event_name,
            (SELECT COUNT(*) FROM tracks t WHERE t.album_id = al.id) AS song_count,
            (SELECT COUNT(DISTINCT CASE WHEN t.disc_no > 0 THEN t.disc_no ELSE 1 END) FROM tracks t WHERE t.album_id = al.id) AS disc_count
         FROM albums al
         LEFT JOIN events ev ON ev.id = al.event_id
         WHERE al.id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    album_summary_from_row(pool, row).await
}

pub(super) async fn album_summary_from_row(
    pool: &SqlitePool,
    row: sqlx::sqlite::SqliteRow,
) -> Result<AlbumSummary> {
    let id: i64 = row.try_get("id")?;
    Ok(AlbumSummary {
        id,
        uuid: row.try_get("uuid")?,
        title: row.try_get("title")?,
        album_artists: album_artists(pool, id).await?,
        event: optional_ref(&row, "event_id", "event_uuid", "event_name")?,
        artwork_id: row.try_get("artwork_id")?,
        year: row.try_get("year")?,
        date: row.try_get("date")?,
        liked_at: row.try_get("liked_at")?,
        song_count: row.try_get("song_count")?,
        disc_count: row.try_get("disc_count")?,
    })
}

pub async fn fetch_album_detail(pool: &SqlitePool, id: i64) -> Result<AlbumDetail> {
    let summary = fetch_album_summary(pool, id).await?;
    let rows = sqlx::query(
        "SELECT
            t.id, t.uuid, t.title, t.artwork_id, t.track_no, t.disc_no, t.duration_ms,
            t.year, t.date, t.liked_at, t.cue_track_no,
            al.id AS album_id, al.uuid AS album_uuid, al.title AS album_title,
            ev.id AS event_id, ev.uuid AS event_uuid, ev.name AS event_name,
            tas.renderer AS renderer
         FROM tracks t
         LEFT JOIN albums al ON al.id = t.album_id
         LEFT JOIN events ev ON ev.id = t.event_id
         LEFT JOIN track_audio_sources tas ON tas.track_id = t.id
         WHERE t.album_id = ?
         ORDER BY CASE WHEN t.disc_no > 0 THEN t.disc_no ELSE 1 END, COALESCE(t.track_no, 999999), t.id",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let mut tracks = Vec::new();
    for row in rows {
        tracks.push(track_summary_from_row(pool, row).await?);
    }
    Ok(AlbumDetail { summary, tracks })
}

pub async fn list_albums(
    pool: &SqlitePool,
    cursor: Option<i64>,
    offset: Option<i64>,
    limit: i64,
    artist_id: Option<i64>,
    event_id: Option<i64>,
    liked: Option<bool>,
    q: Option<String>,
) -> Result<ListResponse<AlbumSummary>> {
    let limit = limit.clamp(1, 200);
    let offset = offset.map(|v| v.max(0));
    let q_norm = q
        .as_ref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| format!("%{}%", normalize_name(s, true)));
    let total = if offset.is_some() {
        let mut count_qb = QueryBuilder::<Sqlite>::new(
            "SELECT COUNT(DISTINCT al.id) AS total
             FROM albums al
             LEFT JOIN events ev ON ev.id = al.event_id",
        );
        if artist_id.is_some() {
            count_qb.push(" JOIN album_artists aa_filter ON aa_filter.album_id = al.id");
        }
        count_qb.push(" WHERE 1=1");
        if let Some(event_id) = event_id {
            count_qb.push(" AND al.event_id = ").push_bind(event_id);
        }
        if let Some(artist_id) = artist_id {
            count_qb
                .push(" AND aa_filter.artist_id = ")
                .push_bind(artist_id);
        }
        if let Some(liked) = liked {
            count_qb.push(if liked {
                " AND al.liked_at IS NOT NULL"
            } else {
                " AND al.liked_at IS NULL"
            });
        }
        if let Some(q_norm) = q_norm.as_ref() {
            count_qb.push(" AND al.title_norm LIKE ").push_bind(q_norm);
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
        "SELECT DISTINCT al.id, al.uuid, al.title, al.artwork_id, al.year, al.date, al.liked_at,
            ev.id AS event_id, ev.uuid AS event_uuid, ev.name AS event_name,
            (SELECT COUNT(*) FROM tracks t WHERE t.album_id = al.id) AS song_count,
            (SELECT COUNT(DISTINCT CASE WHEN t.disc_no > 0 THEN t.disc_no ELSE 1 END) FROM tracks t WHERE t.album_id = al.id) AS disc_count
         FROM albums al
         LEFT JOIN events ev ON ev.id = al.event_id",
    );
    if artist_id.is_some() {
        qb.push(" JOIN album_artists aa_filter ON aa_filter.album_id = al.id");
    }
    qb.push(" WHERE 1=1");
    if offset.is_none() {
        if let Some(cursor) = cursor {
            qb.push(" AND al.id > ").push_bind(cursor);
        }
    }
    if let Some(event_id) = event_id {
        qb.push(" AND al.event_id = ").push_bind(event_id);
    }
    if let Some(artist_id) = artist_id {
        qb.push(" AND aa_filter.artist_id = ").push_bind(artist_id);
    }
    if let Some(liked) = liked {
        qb.push(if liked {
            " AND al.liked_at IS NOT NULL"
        } else {
            " AND al.liked_at IS NULL"
        });
    }
    if let Some(q_norm) = q_norm.as_ref() {
        qb.push(" AND al.title_norm LIKE ").push_bind(q_norm);
    }
    if liked == Some(true) {
        qb.push(" ORDER BY al.liked_at DESC, al.id DESC LIMIT ");
    } else {
        qb.push(" ORDER BY COALESCE(al.year, 0) DESC, al.title, al.id LIMIT ");
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
        items.push(album_summary_from_row(pool, row).await?);
    }
    Ok(ListResponse {
        items,
        next_cursor,
        total,
    })
}
