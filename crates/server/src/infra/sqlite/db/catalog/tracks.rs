use crate::infra::sqlite::db::refs::{optional_ref, track_artists};
use anyhow::Result;
use easy_musiclib_media::normalize::normalize_name;
use easy_musiclib_shared::*;
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};

pub async fn fetch_track_summary(pool: &SqlitePool, id: i64) -> Result<TrackSummary> {
    let row = sqlx::query(
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
         WHERE t.id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    track_summary_from_row(pool, row).await
}

pub(super) async fn track_summary_from_row(
    pool: &SqlitePool,
    row: sqlx::sqlite::SqliteRow,
) -> Result<TrackSummary> {
    let id: i64 = row.try_get("id")?;
    let renderer = row.try_get::<Option<String>, _>("renderer")?;
    Ok(TrackSummary {
        id,
        uuid: row.try_get("uuid")?,
        title: row.try_get("title")?,
        album: optional_ref(&row, "album_id", "album_uuid", "album_title")?,
        artists: track_artists(pool, id).await?,
        event: optional_ref(&row, "event_id", "event_uuid", "event_name")?,
        artwork_id: row.try_get("artwork_id")?,
        track_no: row.try_get("track_no")?,
        disc_no: row.try_get("disc_no")?,
        duration_ms: row.try_get("duration_ms")?,
        year: row.try_get("year")?,
        date: row.try_get("date")?,
        liked_at: row.try_get("liked_at")?,
        is_cue: row.try_get::<Option<i64>, _>("cue_track_no")?.is_some(),
        playable: easy_musiclib_media::cue_render::is_playable_renderer(renderer.as_deref()),
    })
}

pub async fn fetch_track_detail(pool: &SqlitePool, id: i64) -> Result<TrackDetail> {
    let summary = fetch_track_summary(pool, id).await?;
    let row = sqlx::query(
        "SELECT mf.path, tas.renderer, t.cue_track_no
         FROM tracks t
         LEFT JOIN track_audio_sources tas ON tas.track_id = t.id
         LEFT JOIN media_files mf ON mf.id = tas.media_file_id
         WHERE t.id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(TrackDetail {
        summary,
        file_path: row.try_get("path")?,
        renderer: row.try_get("renderer")?,
        cue_track_no: row.try_get("cue_track_no")?,
    })
}

pub async fn list_tracks(
    pool: &SqlitePool,
    cursor: Option<i64>,
    offset: Option<i64>,
    limit: i64,
    artist_id: Option<i64>,
    album_id: Option<i64>,
    event_id: Option<i64>,
    liked: Option<bool>,
    q: Option<String>,
) -> Result<ListResponse<TrackSummary>> {
    let limit = limit.clamp(1, 200);
    let offset = offset.map(|v| v.max(0));
    let q_norm = q
        .as_ref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| format!("%{}%", normalize_name(s, true)));
    let total = if offset.is_some() {
        let mut count_qb = QueryBuilder::<Sqlite>::new(
            "SELECT COUNT(DISTINCT t.id) AS total
             FROM tracks t
             LEFT JOIN albums al ON al.id = t.album_id
             LEFT JOIN events ev ON ev.id = t.event_id
             LEFT JOIN track_audio_sources tas ON tas.track_id = t.id",
        );
        if artist_id.is_some() {
            count_qb.push(" JOIN track_artists ta_filter ON ta_filter.track_id = t.id");
        }
        count_qb.push(" WHERE 1=1");
        if let Some(album_id) = album_id {
            count_qb.push(" AND t.album_id = ").push_bind(album_id);
        }
        if let Some(event_id) = event_id {
            count_qb.push(" AND t.event_id = ").push_bind(event_id);
        }
        if let Some(artist_id) = artist_id {
            count_qb
                .push(" AND ta_filter.artist_id = ")
                .push_bind(artist_id);
        }
        if let Some(liked) = liked {
            if liked {
                count_qb.push(" AND t.liked_at IS NOT NULL");
            } else {
                count_qb.push(" AND t.liked_at IS NULL");
            }
        }
        if let Some(q_norm) = q_norm.as_ref() {
            count_qb
                .push(" AND (t.title_norm LIKE ")
                .push_bind(q_norm.clone())
                .push(" OR al.title_norm LIKE ")
                .push_bind(q_norm.clone())
                .push(" OR ev.name_norm LIKE ")
                .push_bind(q_norm.clone())
                .push(
                    " OR EXISTS (SELECT 1 FROM track_artists ta JOIN artists a ON a.id = ta.artist_id WHERE ta.track_id = t.id AND a.name_norm LIKE ",
                )
                .push_bind(q_norm.clone())
                .push("))");
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
        "SELECT DISTINCT
            t.id, t.uuid, t.title, t.artwork_id, t.track_no, t.disc_no, t.duration_ms,
            t.year, t.date, t.liked_at, t.cue_track_no,
            al.id AS album_id, al.uuid AS album_uuid, al.title AS album_title,
            ev.id AS event_id, ev.uuid AS event_uuid, ev.name AS event_name,
            tas.renderer AS renderer
         FROM tracks t
         LEFT JOIN albums al ON al.id = t.album_id
         LEFT JOIN events ev ON ev.id = t.event_id
         LEFT JOIN track_audio_sources tas ON tas.track_id = t.id",
    );
    if artist_id.is_some() {
        qb.push(" JOIN track_artists ta_filter ON ta_filter.track_id = t.id");
    }
    qb.push(" WHERE 1=1");
    if offset.is_none() {
        if let Some(cursor) = cursor {
            qb.push(" AND t.id > ").push_bind(cursor);
        }
    }
    if let Some(album_id) = album_id {
        qb.push(" AND t.album_id = ").push_bind(album_id);
    }
    if let Some(event_id) = event_id {
        qb.push(" AND t.event_id = ").push_bind(event_id);
    }
    if let Some(artist_id) = artist_id {
        qb.push(" AND ta_filter.artist_id = ").push_bind(artist_id);
    }
    if let Some(liked) = liked {
        if liked {
            qb.push(" AND t.liked_at IS NOT NULL");
        } else {
            qb.push(" AND t.liked_at IS NULL");
        }
    }
    if let Some(q_norm) = q_norm.as_ref() {
        qb.push(" AND (t.title_norm LIKE ")
            .push_bind(q_norm.clone())
            .push(" OR al.title_norm LIKE ")
            .push_bind(q_norm.clone())
            .push(" OR ev.name_norm LIKE ")
            .push_bind(q_norm.clone())
            .push(
                " OR EXISTS (SELECT 1 FROM track_artists ta JOIN artists a ON a.id = ta.artist_id WHERE ta.track_id = t.id AND a.name_norm LIKE ",
            )
            .push_bind(q_norm.clone())
            .push("))");
    }
    if liked == Some(true) {
        qb.push(" ORDER BY t.liked_at DESC, t.id DESC LIMIT ");
    } else {
        qb.push(" ORDER BY t.id LIMIT ");
    }
    qb.push_bind(limit + 1);
    if let Some(offset) = offset {
        qb.push(" OFFSET ").push_bind(offset);
    }
    let rows = qb.build().fetch_all(pool).await?;
    rows_to_track_list(pool, rows, limit, total).await
}

async fn rows_to_track_list(
    pool: &SqlitePool,
    rows: Vec<sqlx::sqlite::SqliteRow>,
    limit: i64,
    total: Option<i64>,
) -> Result<ListResponse<TrackSummary>> {
    let mut items = Vec::new();
    let mut next_cursor = None;
    for (idx, row) in rows.into_iter().enumerate() {
        if idx as i64 >= limit {
            next_cursor = Some(row.try_get("id")?);
            break;
        }
        items.push(track_summary_from_row(pool, row).await?);
    }
    Ok(ListResponse {
        items,
        next_cursor,
        total,
    })
}
