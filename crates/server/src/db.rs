use anyhow::{Result, anyhow};
use easy_musiclib_media::normalize::{fuzzy_score, normalize_name};
use easy_musiclib_shared::*;
use sqlx::sqlite::SqliteQueryResult;
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};
use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use uuid::Uuid;

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
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

fn row_to_entity_ref(row: sqlx::sqlite::SqliteRow) -> Result<EntityRef> {
    Ok(EntityRef {
        id: row.try_get("id")?,
        uuid: row.try_get("uuid")?,
        name: row.try_get("name")?,
    })
}

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

async fn track_summary_from_row(
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
        playable: matches!(
            renderer.as_deref(),
            Some("passthrough" | "flac_tracksplit" | "wav_slice")
        ),
    })
}

fn optional_ref(
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

pub async fn fetch_album_summary(pool: &SqlitePool, id: i64) -> Result<AlbumSummary> {
    let row = sqlx::query(
        "SELECT al.id, al.uuid, al.title, al.artwork_id, al.year, al.date, al.liked_at,
            ev.id AS event_id, ev.uuid AS event_uuid, ev.name AS event_name,
            (SELECT COUNT(*) FROM tracks t WHERE t.album_id = al.id) AS song_count
         FROM albums al
         LEFT JOIN events ev ON ev.id = al.event_id
         WHERE al.id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    album_summary_from_row(pool, row).await
}

async fn album_summary_from_row(
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
         ORDER BY COALESCE(t.disc_no, 1), COALESCE(t.track_no, 999999), t.id",
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
            (SELECT COUNT(*) FROM tracks t WHERE t.album_id = al.id) AS song_count
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

pub async fn fetch_artist_summary(pool: &SqlitePool, id: i64) -> Result<ArtistSummary> {
    let row = sqlx::query(
        "SELECT a.id, a.uuid, a.name, a.artwork_id, a.liked_at,
            (SELECT COUNT(DISTINCT album_id) FROM album_artists WHERE artist_id = a.id) AS album_count,
            (SELECT COUNT(DISTINCT track_id) FROM track_artists WHERE artist_id = a.id) AS track_count
         FROM artists a WHERE a.id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    artist_summary_from_row(row)
}

fn artist_summary_from_row(row: sqlx::sqlite::SqliteRow) -> Result<ArtistSummary> {
    Ok(ArtistSummary {
        id: row.try_get("id")?,
        uuid: row.try_get("uuid")?,
        name: row.try_get("name")?,
        artwork_id: row.try_get("artwork_id")?,
        liked_at: row.try_get("liked_at")?,
        album_count: row.try_get("album_count")?,
        track_count: row.try_get("track_count")?,
    })
}

pub async fn fetch_artist_detail(pool: &SqlitePool, id: i64) -> Result<ArtistDetail> {
    let summary = fetch_artist_summary(pool, id).await?;
    let albums = list_albums(pool, None, None, 500, Some(id), None, None, None)
        .await?
        .items;
    let tracks = list_tracks(pool, None, None, 1000, Some(id), None, None, None, None)
        .await?
        .items;
    let aliases =
        sqlx::query("SELECT alias FROM artist_aliases WHERE artist_id = ? ORDER BY alias")
            .bind(id)
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|row| row.try_get("alias"))
            .collect::<Result<Vec<String>, sqlx::Error>>()?;
    Ok(ArtistDetail {
        summary,
        albums,
        tracks,
        aliases,
    })
}

pub async fn list_artists(
    pool: &SqlitePool,
    cursor: Option<i64>,
    offset: Option<i64>,
    limit: i64,
    liked: Option<bool>,
    q: Option<String>,
) -> Result<ListResponse<ArtistSummary>> {
    let limit = limit.clamp(1, 200);
    let offset = offset.map(|v| v.max(0));
    let q_norm = q
        .as_ref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| format!("%{}%", normalize_name(s, true)));
    let total = if offset.is_some() {
        let mut count_qb =
            QueryBuilder::<Sqlite>::new("SELECT COUNT(*) AS total FROM artists a WHERE 1=1");
        if let Some(liked) = liked {
            count_qb.push(if liked {
                " AND a.liked_at IS NOT NULL"
            } else {
                " AND a.liked_at IS NULL"
            });
        }
        if let Some(q_norm) = q_norm.as_ref() {
            count_qb
                .push(" AND (a.name_norm LIKE ")
                .push_bind(q_norm.clone())
                .push(
                    " OR EXISTS (SELECT 1 FROM artist_aliases aa WHERE aa.artist_id = a.id AND aa.alias_norm LIKE ",
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
        "SELECT a.id, a.uuid, a.name, a.artwork_id, a.liked_at,
            (SELECT COUNT(DISTINCT album_id) FROM album_artists WHERE artist_id = a.id) AS album_count,
            (SELECT COUNT(DISTINCT track_id) FROM track_artists WHERE artist_id = a.id) AS track_count
         FROM artists a WHERE 1=1",
    );
    if offset.is_none() {
        if let Some(cursor) = cursor {
            qb.push(" AND a.id > ").push_bind(cursor);
        }
    }
    if let Some(liked) = liked {
        qb.push(if liked {
            " AND a.liked_at IS NOT NULL"
        } else {
            " AND a.liked_at IS NULL"
        });
    }
    if let Some(q) = q_norm.as_ref() {
        qb.push(
            " AND (a.name_norm LIKE ",
        ).push_bind(q.clone()).push(
            " OR EXISTS (SELECT 1 FROM artist_aliases aa WHERE aa.artist_id = a.id AND aa.alias_norm LIKE ",
        ).push_bind(q).push("))");
    }
    if liked == Some(true) {
        qb.push(" ORDER BY a.liked_at DESC, a.id DESC LIMIT ");
    } else {
        qb.push(" ORDER BY a.name, a.id LIMIT ");
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
        items.push(artist_summary_from_row(row)?);
    }
    Ok(ListResponse {
        items,
        next_cursor,
        total,
    })
}

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

pub async fn search(pool: &SqlitePool, q: &str, limit: i64) -> Result<SearchResponse> {
    let limit = limit.clamp(1, 100);
    let norm = normalize_name(q, true);
    let like = format!("%{norm}%");

    let mut track_rows = sqlx::query(
        "SELECT DISTINCT
            t.id, t.uuid, t.title, t.artwork_id, t.track_no, t.disc_no, t.duration_ms,
            t.year, t.date, t.liked_at, t.cue_track_no,
            al.id AS album_id, al.uuid AS album_uuid, al.title AS album_title,
            ev.id AS event_id, ev.uuid AS event_uuid, ev.name AS event_name,
            tas.renderer AS renderer
         FROM tracks t
         LEFT JOIN albums al ON al.id = t.album_id
         LEFT JOIN events ev ON ev.id = t.event_id
         LEFT JOIN track_audio_sources tas ON tas.track_id = t.id
         WHERE t.title_norm LIKE ?
            OR al.title_norm LIKE ?
            OR ev.name_norm LIKE ?
            OR EXISTS (
              SELECT 1 FROM track_artists ta
              JOIN artists a ON a.id = ta.artist_id
              WHERE ta.track_id = t.id AND a.name_norm LIKE ?
            )
         LIMIT ?",
    )
    .bind(&like)
    .bind(&like)
    .bind(&like)
    .bind(&like)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    track_rows.sort_by(|a, b| {
        let at: String = a.try_get("title").unwrap_or_default();
        let bt: String = b.try_get("title").unwrap_or_default();
        fuzzy_score(&bt, q)
            .partial_cmp(&fuzzy_score(&at, q))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut tracks = Vec::new();
    for row in track_rows {
        tracks.push(track_summary_from_row(pool, row).await?);
    }

    let albums = list_albums(
        pool,
        None,
        None,
        limit,
        None,
        None,
        None,
        Some(q.to_string()),
    )
    .await?
    .items;
    let artists = list_artists(pool, None, None, limit, None, Some(q.to_string()))
        .await?
        .items;
    let events = list_events(pool, None, None, limit, None, Some(q.to_string()))
        .await?
        .items;
    Ok(SearchResponse {
        tracks,
        albums,
        artists,
        events,
    })
}

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

pub async fn create_artist(pool: &SqlitePool, name: &str) -> Result<ArtistSummary> {
    let id = ensure_artist(pool, name, None).await?;
    refresh_artist_search(pool, id).await?;
    fetch_artist_summary(pool, id).await
}

pub async fn ensure_artist(pool: &SqlitePool, name: &str, artwork_id: Option<i64>) -> Result<i64> {
    let name = name.trim();
    if name.is_empty() {
        return Err(anyhow!("artist name is empty"));
    }
    let norm = normalize_name(name, false);
    if let Some(row) = sqlx::query("SELECT id FROM artists WHERE name_norm = ?")
        .bind(&norm)
        .fetch_optional(pool)
        .await?
    {
        let id = row.try_get("id")?;
        if artwork_id.is_some() {
            sqlx::query("UPDATE artists SET artwork_id = COALESCE(artwork_id, ?) WHERE id = ?")
                .bind(artwork_id)
                .bind(id)
                .execute(pool)
                .await?;
        }
        return Ok(id);
    }
    let res =
        sqlx::query("INSERT INTO artists (uuid, name, name_norm, artwork_id) VALUES (?, ?, ?, ?)")
            .bind(Uuid::new_v4().to_string())
            .bind(name)
            .bind(norm)
            .bind(artwork_id)
            .execute(pool)
            .await?;
    Ok(res.last_insert_rowid())
}

pub async fn ensure_event(
    pool: &SqlitePool,
    name: Option<&str>,
    date: Option<&str>,
    year: Option<i64>,
) -> Result<Option<i64>> {
    let Some(name) = name.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let norm = normalize_name(name, false);
    if let Some(row) = sqlx::query("SELECT id, date, year FROM events WHERE name_norm = ?")
        .bind(&norm)
        .fetch_optional(pool)
        .await?
    {
        let id: i64 = row.try_get("id")?;
        update_event_date_year(pool, id, date, year).await?;
        return Ok(Some(id));
    }
    let res = sqlx::query(
        "INSERT INTO events (uuid, name, name_norm, date, year) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(name)
    .bind(norm)
    .bind(date)
    .bind(year)
    .execute(pool)
    .await?;
    Ok(Some(res.last_insert_rowid()))
}

pub async fn update_event_date_year(
    pool: &SqlitePool,
    event_id: i64,
    date: Option<&str>,
    year: Option<i64>,
) -> Result<()> {
    if date.is_some() || year.is_some() {
        sqlx::query(
            "UPDATE events
             SET date = CASE
                    WHEN ? IS NULL THEN date
                    WHEN date IS NULL THEN ?
                    WHEN length(date) <= 4 AND length(?) > length(date) THEN ?
                    ELSE date
                 END,
                 year = COALESCE(year, ?)
             WHERE id = ?",
        )
        .bind(date)
        .bind(date)
        .bind(date)
        .bind(date)
        .bind(year)
        .bind(event_id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn find_or_create_album(
    pool: &SqlitePool,
    title: &str,
    album_artist_ids: &[i64],
    year: Option<i64>,
    date: Option<&str>,
    event_id: Option<i64>,
    artwork_id: Option<i64>,
) -> Result<i64> {
    let title = title.trim();
    let title = if title.is_empty() {
        "Unknown Album"
    } else {
        title
    };
    let title_norm = normalize_name(title, false);
    let candidates = sqlx::query(
        "SELECT id FROM albums
         WHERE title_norm = ?
           AND (year = ? OR year IS NULL OR ? IS NULL)
         ORDER BY id",
    )
    .bind(&title_norm)
    .bind(year)
    .bind(year)
    .fetch_all(pool)
    .await?;
    let wanted: BTreeSet<i64> = album_artist_ids.iter().copied().collect();
    for row in candidates {
        let id: i64 = row.try_get("id")?;
        let existing: BTreeSet<i64> =
            sqlx::query("SELECT artist_id FROM album_artists WHERE album_id = ?")
                .bind(id)
                .fetch_all(pool)
                .await?
                .into_iter()
                .map(|r| r.try_get("artist_id"))
                .collect::<Result<BTreeSet<i64>, sqlx::Error>>()?;
        if existing.is_empty() || wanted.is_empty() || existing == wanted {
            sqlx::query(
                "UPDATE albums SET
                    date = COALESCE(date, ?),
                    year = COALESCE(year, ?),
                    event_id = COALESCE(event_id, ?),
                    artwork_id = COALESCE(artwork_id, ?)
                 WHERE id = ?",
            )
            .bind(date)
            .bind(year)
            .bind(event_id)
            .bind(artwork_id)
            .bind(id)
            .execute(pool)
            .await?;
            return Ok(id);
        }
    }

    let res = sqlx::query(
        "INSERT INTO albums (uuid, title, title_norm, date, year, event_id, artwork_id)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(title)
    .bind(title_norm)
    .bind(date)
    .bind(year)
    .bind(event_id)
    .bind(artwork_id)
    .execute(pool)
    .await?;
    let id = res.last_insert_rowid();
    for (pos, artist_id) in album_artist_ids.iter().enumerate() {
        sqlx::query(
            "INSERT OR IGNORE INTO album_artists (album_id, artist_id, position) VALUES (?, ?, ?)",
        )
        .bind(id)
        .bind(artist_id)
        .bind(pos as i64)
        .execute(pool)
        .await?;
    }
    if let Some(event_id) = event_id {
        sqlx::query("INSERT OR IGNORE INTO event_albums (event_id, album_id) VALUES (?, ?)")
            .bind(event_id)
            .bind(id)
            .execute(pool)
            .await?;
    }
    Ok(id)
}

pub async fn insert_track(
    pool: &SqlitePool,
    new_track: NewTrack<'_>,
    artist_ids: &[i64],
) -> Result<i64> {
    let res = sqlx::query(
        "INSERT INTO tracks
         (uuid, title, title_norm, album_id, event_id, cue_track_no, disc_no, track_no,
          duration_ms, date, year, artwork_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(new_track.title)
    .bind(normalize_name(new_track.title, false))
    .bind(new_track.album_id)
    .bind(new_track.event_id)
    .bind(new_track.cue_track_no)
    .bind(new_track.disc_no)
    .bind(new_track.track_no)
    .bind(new_track.duration_ms)
    .bind(new_track.date)
    .bind(new_track.year)
    .bind(new_track.artwork_id)
    .execute(pool)
    .await?;
    let track_id = res.last_insert_rowid();
    for (pos, artist_id) in artist_ids.iter().enumerate() {
        sqlx::query(
            "INSERT OR IGNORE INTO track_artists (track_id, artist_id, position) VALUES (?, ?, ?)",
        )
        .bind(track_id)
        .bind(artist_id)
        .bind(pos as i64)
        .execute(pool)
        .await?;
    }
    Ok(track_id)
}

pub struct NewTrack<'a> {
    pub title: &'a str,
    pub album_id: Option<i64>,
    pub event_id: Option<i64>,
    pub cue_track_no: Option<i64>,
    pub disc_no: Option<i64>,
    pub track_no: Option<i64>,
    pub duration_ms: Option<i64>,
    pub date: Option<&'a str>,
    pub year: Option<i64>,
    pub artwork_id: Option<i64>,
}

pub async fn insert_track_audio_source(
    pool: &SqlitePool,
    track_id: i64,
    src: NewTrackAudioSource<'_>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO track_audio_sources
         (track_id, kind, media_file_id, cue_sheet_id, codec, sample_rate,
          start_sample, end_sample, start_ms, end_ms, renderer)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(track_id)
    .bind(src.kind)
    .bind(src.media_file_id)
    .bind(src.cue_sheet_id)
    .bind(src.codec)
    .bind(src.sample_rate)
    .bind(src.start_sample)
    .bind(src.end_sample)
    .bind(src.start_ms)
    .bind(src.end_ms)
    .bind(src.renderer)
    .execute(pool)
    .await?;
    Ok(())
}

pub struct NewTrackAudioSource<'a> {
    pub kind: &'a str,
    pub media_file_id: i64,
    pub cue_sheet_id: Option<i64>,
    pub codec: &'a str,
    pub sample_rate: Option<i64>,
    pub start_sample: Option<i64>,
    pub end_sample: Option<i64>,
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
    pub renderer: &'a str,
}

pub async fn upsert_media_file(
    pool: &SqlitePool,
    path: &str,
    path_hash: &str,
    size: i64,
    mtime_ns: i64,
    format: &str,
) -> Result<(i64, bool)> {
    if let Some(row) = sqlx::query("SELECT id, size, mtime_ns FROM media_files WHERE path_hash = ?")
        .bind(path_hash)
        .fetch_optional(pool)
        .await?
    {
        let id: i64 = row.try_get("id")?;
        let old_size: i64 = row.try_get("size")?;
        let old_mtime: i64 = row.try_get("mtime_ns")?;
        let changed = old_size != size || old_mtime != mtime_ns;
        sqlx::query(
            "UPDATE media_files
             SET path = ?, size = ?, mtime_ns = ?, format = ?, last_scanned_at = ?, missing = 0,
                 scan_error = CASE WHEN ? THEN NULL ELSE scan_error END
             WHERE id = ?",
        )
        .bind(path)
        .bind(size)
        .bind(mtime_ns)
        .bind(format)
        .bind(now_ms())
        .bind(changed)
        .bind(id)
        .execute(pool)
        .await?;
        Ok((id, changed))
    } else {
        let res = sqlx::query(
            "INSERT INTO media_files
             (path, path_hash, size, mtime_ns, format, last_scanned_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(path)
        .bind(path_hash)
        .bind(size)
        .bind(mtime_ns)
        .bind(format)
        .bind(now_ms())
        .execute(pool)
        .await?;
        Ok((res.last_insert_rowid(), true))
    }
}

pub async fn delete_tracks_for_media_file(pool: &SqlitePool, media_file_id: i64) -> Result<()> {
    let ids = sqlx::query("SELECT track_id FROM track_audio_sources WHERE media_file_id = ?")
        .bind(media_file_id)
        .fetch_all(pool)
        .await?;
    for row in ids {
        let id: i64 = row.try_get("track_id")?;
        sqlx::query("DELETE FROM search_index WHERE kind = 'track' AND entity_id = ?")
            .bind(id)
            .execute(pool)
            .await
            .ok();
        sqlx::query("DELETE FROM tracks WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn delete_cue_sheet_for_file(pool: &SqlitePool, cue_file_id: i64) -> Result<()> {
    let sheets = sqlx::query("SELECT id FROM cue_sheets WHERE cue_file_id = ?")
        .bind(cue_file_id)
        .fetch_all(pool)
        .await?;
    for row in sheets {
        let sheet_id: i64 = row.try_get("id")?;
        let track_ids =
            sqlx::query("SELECT track_id FROM track_audio_sources WHERE cue_sheet_id = ?")
                .bind(sheet_id)
                .fetch_all(pool)
                .await?;
        for tr in track_ids {
            let track_id: i64 = tr.try_get("track_id")?;
            sqlx::query("DELETE FROM search_index WHERE kind = 'track' AND entity_id = ?")
                .bind(track_id)
                .execute(pool)
                .await
                .ok();
            sqlx::query("DELETE FROM tracks WHERE id = ?")
                .bind(track_id)
                .execute(pool)
                .await?;
        }
        sqlx::query("DELETE FROM cue_sheets WHERE id = ?")
            .bind(sheet_id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn insert_cue_sheet(
    pool: &SqlitePool,
    cue_file_id: i64,
    audio_file_id: i64,
    album_title: Option<&str>,
    performer: Option<&str>,
    date: Option<&str>,
) -> Result<i64> {
    let res = sqlx::query(
        "INSERT INTO cue_sheets
         (cue_file_id, audio_file_id, album_title, performer, date)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(cue_file_id)
    .bind(audio_file_id)
    .bind(album_title)
    .bind(performer)
    .bind(date)
    .execute(pool)
    .await?;
    Ok(res.last_insert_rowid())
}

pub async fn ensure_artwork_source(
    pool: &SqlitePool,
    kind: &str,
    media_file_id: Option<i64>,
    sidecar_path: Option<&str>,
    embedded_picture_index: Option<i64>,
    mime: Option<&str>,
) -> Result<Option<i64>> {
    let row = match kind {
        "sidecar" => {
            if sidecar_path.is_none() {
                return Ok(None);
            }
            sqlx::query(
                "SELECT id FROM artwork_sources WHERE kind = 'sidecar' AND sidecar_path = ?",
            )
            .bind(sidecar_path)
            .fetch_optional(pool)
            .await?
        }
        "embedded" => {
            if media_file_id.is_none() {
                return Ok(None);
            }
            sqlx::query(
                "SELECT id FROM artwork_sources
                 WHERE kind = 'embedded' AND media_file_id = ? AND embedded_picture_index = ?",
            )
            .bind(media_file_id)
            .bind(embedded_picture_index.unwrap_or(0))
            .fetch_optional(pool)
            .await?
        }
        _ => None,
    };
    if let Some(row) = row {
        return Ok(Some(row.try_get("id")?));
    }
    let res = sqlx::query(
        "INSERT INTO artwork_sources
         (kind, media_file_id, sidecar_path, embedded_picture_index, mime, last_checked_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(kind)
    .bind(media_file_id)
    .bind(sidecar_path)
    .bind(embedded_picture_index)
    .bind(mime)
    .bind(now_ms())
    .execute(pool)
    .await?;
    Ok(Some(res.last_insert_rowid()))
}

pub async fn refresh_track_search(pool: &SqlitePool, track_id: i64) -> Result<()> {
    let detail = fetch_track_detail(pool, track_id).await?;
    sqlx::query("DELETE FROM search_index WHERE kind = 'track' AND entity_id = ?")
        .bind(track_id)
        .execute(pool)
        .await?;
    let artists = detail
        .summary
        .artists
        .iter()
        .map(|a| a.name.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    sqlx::query(
        "INSERT INTO search_index (kind, entity_id, title, artists, album, event, aliases)
         VALUES ('track', ?, ?, ?, ?, ?, '')",
    )
    .bind(track_id)
    .bind(&detail.summary.title)
    .bind(artists)
    .bind(detail.summary.album.as_ref().map(|a| a.name.as_str()))
    .bind(detail.summary.event.as_ref().map(|e| e.name.as_str()))
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn refresh_album_search(pool: &SqlitePool, album_id: i64) -> Result<()> {
    let summary = fetch_album_summary(pool, album_id).await?;
    sqlx::query("DELETE FROM search_index WHERE kind = 'album' AND entity_id = ?")
        .bind(album_id)
        .execute(pool)
        .await?;
    let artists = summary
        .album_artists
        .iter()
        .map(|a| a.name.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    sqlx::query(
        "INSERT INTO search_index (kind, entity_id, title, artists, album, event, aliases)
         VALUES ('album', ?, ?, ?, '', ?, '')",
    )
    .bind(album_id)
    .bind(summary.title)
    .bind(artists)
    .bind(summary.event.as_ref().map(|e| e.name.as_str()))
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn refresh_artist_search(pool: &SqlitePool, artist_id: i64) -> Result<()> {
    let summary = fetch_artist_summary(pool, artist_id).await?;
    let aliases = sqlx::query("SELECT alias FROM artist_aliases WHERE artist_id = ?")
        .bind(artist_id)
        .fetch_all(pool)
        .await?
        .into_iter()
        .filter_map(|r| r.try_get::<String, _>("alias").ok())
        .collect::<Vec<_>>()
        .join(" ");
    sqlx::query("DELETE FROM search_index WHERE kind = 'artist' AND entity_id = ?")
        .bind(artist_id)
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT INTO search_index (kind, entity_id, title, artists, album, event, aliases)
         VALUES ('artist', ?, ?, '', '', '', ?)",
    )
    .bind(artist_id)
    .bind(summary.name)
    .bind(aliases)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn refresh_event_search(pool: &SqlitePool, event_id: i64) -> Result<()> {
    let summary = fetch_event_summary(pool, event_id).await?;
    sqlx::query("DELETE FROM search_index WHERE kind = 'event' AND entity_id = ?")
        .bind(event_id)
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT INTO search_index (kind, entity_id, title, artists, album, event, aliases)
         VALUES ('event', ?, ?, '', '', ?, '')",
    )
    .bind(event_id)
    .bind(&summary.name)
    .bind(&summary.name)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn add_artist_alias(pool: &SqlitePool, artist_id: i64, alias: &str) -> Result<()> {
    let alias = alias.trim();
    if alias.is_empty() {
        return Ok(());
    }
    let norm = normalize_name(alias, false);
    sqlx::query(
        "INSERT OR IGNORE INTO artist_aliases (artist_id, alias, alias_norm) VALUES (?, ?, ?)",
    )
    .bind(artist_id)
    .bind(alias)
    .bind(norm)
    .execute(pool)
    .await?;
    refresh_artist_search(pool, artist_id).await?;
    Ok(())
}

pub async fn merge_artists(
    pool: &SqlitePool,
    target_id: i64,
    source_id: i64,
    reason: &str,
) -> Result<()> {
    if target_id == source_id {
        return Ok(());
    }
    let source = entity_ref(pool, "artists", source_id).await?;
    sqlx::query(
        "UPDATE artists
         SET artwork_id = COALESCE(artwork_id, (SELECT artwork_id FROM artists WHERE id = ?))
         WHERE id = ?",
    )
    .bind(source_id)
    .bind(target_id)
    .execute(pool)
    .await?;
    add_artist_alias(pool, target_id, &source.name).await?;

    sqlx::query(
        "INSERT OR IGNORE INTO track_artists (track_id, artist_id, role, position)
         SELECT track_id, ?, role, position FROM track_artists WHERE artist_id = ?",
    )
    .bind(target_id)
    .bind(source_id)
    .execute(pool)
    .await?;
    sqlx::query("DELETE FROM track_artists WHERE artist_id = ?")
        .bind(source_id)
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT OR IGNORE INTO album_artists (album_id, artist_id, position)
         SELECT album_id, ?, position FROM album_artists WHERE artist_id = ?",
    )
    .bind(target_id)
    .bind(source_id)
    .execute(pool)
    .await?;
    sqlx::query("DELETE FROM album_artists WHERE artist_id = ?")
        .bind(source_id)
        .execute(pool)
        .await?;
    sqlx::query("UPDATE artist_aliases SET artist_id = ? WHERE artist_id = ?")
        .bind(target_id)
        .bind(source_id)
        .execute(pool)
        .await?;
    sqlx::query(
        "INSERT INTO artist_merge_audit
         (target_artist_id, source_artist_uuid, source_artist_name, reason, merged_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(target_id)
    .bind(source.uuid)
    .bind(source.name)
    .bind(reason)
    .bind(now_ms())
    .execute(pool)
    .await?;
    sqlx::query("DELETE FROM search_index WHERE kind = 'artist' AND entity_id = ?")
        .bind(source_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM artists WHERE id = ?")
        .bind(source_id)
        .execute(pool)
        .await?;

    refresh_artist_search(pool, target_id).await?;
    rebuild_relations(pool).await?;
    Ok(())
}

pub async fn rebuild_relations(pool: &SqlitePool) -> Result<()> {
    sqlx::query("DELETE FROM artist_relation_edges")
        .execute(pool)
        .await?;
    let mut edges: BTreeMap<(i64, i64), (i64, BTreeSet<String>)> = BTreeMap::new();

    let rows = sqlx::query(
        "SELECT t.id AS track_id, t.uuid AS track_uuid, t.title, a.id AS artist_id, a.name
         FROM tracks t
         JOIN track_artists ta ON ta.track_id = t.id
         JOIN artists a ON a.id = ta.artist_id
         WHERE lower(a.name) <> 'various artists'
         ORDER BY t.id, ta.position",
    )
    .fetch_all(pool)
    .await?;
    let mut by_track: BTreeMap<i64, (String, String, Vec<i64>)> = BTreeMap::new();
    for row in rows {
        let track_id: i64 = row.try_get("track_id")?;
        by_track
            .entry(track_id)
            .or_insert_with(|| {
                (
                    row.try_get::<String, _>("title").unwrap_or_default(),
                    row.try_get::<String, _>("track_uuid").unwrap_or_default(),
                    Vec::new(),
                )
            })
            .2
            .push(row.try_get("artist_id")?);
    }
    for (_, (title, uuid, artists)) in by_track {
        add_pairs(&mut edges, &artists, format!("same song: {title} ({uuid})"));
    }

    let rows = sqlx::query(
        "SELECT al.id AS album_id, al.uuid AS album_uuid, al.title, a.id AS artist_id
         FROM albums al
         JOIN album_artists aa ON aa.album_id = al.id
         JOIN artists a ON a.id = aa.artist_id
         WHERE lower(a.name) <> 'various artists'
         ORDER BY al.id, aa.position",
    )
    .fetch_all(pool)
    .await?;
    let mut by_album: BTreeMap<i64, (String, String, Vec<i64>)> = BTreeMap::new();
    for row in rows {
        let album_id: i64 = row.try_get("album_id")?;
        by_album
            .entry(album_id)
            .or_insert_with(|| {
                (
                    row.try_get::<String, _>("title").unwrap_or_default(),
                    row.try_get::<String, _>("album_uuid").unwrap_or_default(),
                    Vec::new(),
                )
            })
            .2
            .push(row.try_get("artist_id")?);
    }
    for (_, (title, uuid, artists)) in &by_album {
        add_pairs(&mut edges, artists, format!("same album: {title} ({uuid})"));
    }

    let rows = sqlx::query(
        "SELECT DISTINCT al.id AS album_id, al.uuid AS album_uuid, al.title,
                aa.artist_id AS album_artist_id, ta.artist_id AS song_artist_id
         FROM albums al
         JOIN album_artists aa ON aa.album_id = al.id
         JOIN tracks t ON t.album_id = al.id
         JOIN track_artists ta ON ta.track_id = t.id
         WHERE aa.artist_id <> ta.artist_id",
    )
    .fetch_all(pool)
    .await?;
    for row in rows {
        let a: i64 = row.try_get("album_artist_id")?;
        let b: i64 = row.try_get("song_artist_id")?;
        let title: String = row.try_get("title")?;
        let uuid: String = row.try_get("album_uuid")?;
        add_edge(
            &mut edges,
            a,
            b,
            format!("album artist with song artist: {title} ({uuid})"),
        );
    }

    for ((a, b), (strength, details)) in edges {
        sqlx::query(
            "INSERT INTO artist_relation_edges
             (artist_a_id, artist_b_id, strength, details_json, updated_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(a)
        .bind(b)
        .bind(strength)
        .bind(serde_json::to_string(
            &details.into_iter().collect::<Vec<_>>(),
        )?)
        .bind(now_ms())
        .execute(pool)
        .await?;
    }
    Ok(())
}

fn add_pairs(
    edges: &mut BTreeMap<(i64, i64), (i64, BTreeSet<String>)>,
    artists: &[i64],
    detail: String,
) {
    for i in 0..artists.len() {
        for j in i + 1..artists.len() {
            add_edge(edges, artists[i], artists[j], detail.clone());
        }
    }
}

fn add_edge(
    edges: &mut BTreeMap<(i64, i64), (i64, BTreeSet<String>)>,
    a: i64,
    b: i64,
    detail: String,
) {
    if a == b {
        return;
    }
    let key = if a < b { (a, b) } else { (b, a) };
    let entry = edges.entry(key).or_insert_with(|| (0, BTreeSet::new()));
    entry.0 += 1;
    entry.1.insert(detail);
}

pub async fn relation_graph(
    pool: &SqlitePool,
    artist_id: Option<i64>,
    depth: i64,
    limit_nodes: i64,
) -> Result<RelationGraph> {
    let limit_nodes = limit_nodes.clamp(1, 2000) as usize;
    let mut node_ids = BTreeSet::new();
    let mut edge_keys = BTreeSet::new();

    if let Some(start) = artist_id {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::from([(start, 0i64)]);
        visited.insert(start);
        while let Some((id, d)) = queue.pop_front() {
            if node_ids.len() >= limit_nodes {
                break;
            }
            node_ids.insert(id);
            if d >= depth {
                continue;
            }
            let rows = sqlx::query(
                "SELECT artist_a_id, artist_b_id
                 FROM artist_relation_edges
                 WHERE artist_a_id = ? OR artist_b_id = ?",
            )
            .bind(id)
            .bind(id)
            .fetch_all(pool)
            .await?;
            for row in rows {
                let a: i64 = row.try_get("artist_a_id")?;
                let b: i64 = row.try_get("artist_b_id")?;
                edge_keys.insert((a.min(b), a.max(b)));
                let next = if a == id { b } else { a };
                if visited.insert(next) {
                    queue.push_back((next, d + 1));
                }
            }
        }
    } else {
        let rows = sqlx::query(
            "SELECT artist_a_id, artist_b_id
             FROM artist_relation_edges
             ORDER BY strength DESC
             LIMIT ?",
        )
        .bind(limit_nodes as i64)
        .fetch_all(pool)
        .await?;
        for row in rows {
            let a: i64 = row.try_get("artist_a_id")?;
            let b: i64 = row.try_get("artist_b_id")?;
            node_ids.insert(a);
            node_ids.insert(b);
            edge_keys.insert((a.min(b), a.max(b)));
            if node_ids.len() >= limit_nodes {
                break;
            }
        }
    }

    let mut nodes = Vec::new();
    for id in &node_ids {
        if let Ok(r) = entity_ref(pool, "artists", *id).await {
            nodes.push(RelationNode {
                id: r.id,
                uuid: r.uuid,
                name: r.name,
            });
        }
    }

    let mut edges = Vec::new();
    for (a, b) in edge_keys {
        if !node_ids.contains(&a) || !node_ids.contains(&b) {
            continue;
        }
        if let Some(row) = sqlx::query(
            "SELECT artist_a_id, artist_b_id, strength, details_json
             FROM artist_relation_edges WHERE artist_a_id = ? AND artist_b_id = ?",
        )
        .bind(a)
        .bind(b)
        .fetch_optional(pool)
        .await?
        {
            let details_json: String = row.try_get("details_json")?;
            edges.push(RelationEdge {
                source: row.try_get("artist_a_id")?,
                target: row.try_get("artist_b_id")?,
                strength: row.try_get("strength")?,
                details: serde_json::from_str(&details_json).unwrap_or_default(),
            });
        }
    }
    Ok(RelationGraph { nodes, edges })
}

pub async fn import_alias_csv(pool: &SqlitePool, csv: &str) -> Result<usize> {
    let mut count = 0;
    for line in csv.lines() {
        let cols = parse_csv_line(line);
        if cols.len() < 2 {
            continue;
        }
        let primary = cols[0].trim();
        if primary.is_empty() {
            continue;
        }
        let target_id = ensure_artist(pool, primary, None).await?;
        for alias in cols
            .iter()
            .skip(1)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            let alias_id = ensure_artist(pool, alias, None).await?;
            if alias_id != target_id {
                merge_artists(pool, target_id, alias_id, "alias_csv").await?;
                count += 1;
            } else {
                add_artist_alias(pool, target_id, alias).await?;
            }
        }
    }
    rebuild_relations(pool).await?;
    Ok(count)
}

pub async fn auto_merge(pool: &SqlitePool) -> Result<usize> {
    let rows = sqlx::query(
        "SELECT aa.artist_id AS target_id, a2.id AS source_id
         FROM artist_aliases aa
         JOIN artists a2 ON a2.name_norm = aa.alias_norm
         WHERE a2.id <> aa.artist_id",
    )
    .fetch_all(pool)
    .await?;
    let mut count = 0;
    for row in rows {
        merge_artists(
            pool,
            row.try_get("target_id")?,
            row.try_get("source_id")?,
            "auto_merge_alias",
        )
        .await?;
        count += 1;
    }
    repair_event_dates_and_artwork(pool).await?;
    rebuild_relations(pool).await?;
    Ok(count)
}

pub async fn repair_event_dates_and_artwork(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "UPDATE albums
         SET year = COALESCE(year, (SELECT year FROM events WHERE events.id = albums.event_id)),
             date = CASE
               WHEN date IS NULL OR length(date) <= 4
               THEN COALESCE((SELECT date FROM events WHERE events.id = albums.event_id), date)
               ELSE date
             END
         WHERE event_id IS NOT NULL",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE tracks
         SET year = COALESCE(year, (SELECT year FROM events WHERE events.id = tracks.event_id)),
             date = CASE
               WHEN date IS NULL OR length(date) <= 4
               THEN COALESCE((SELECT date FROM events WHERE events.id = tracks.event_id), date)
               ELSE date
             END,
             artwork_id = COALESCE(artwork_id, (SELECT artwork_id FROM albums WHERE albums.id = tracks.album_id))
         WHERE album_id IS NOT NULL",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE artists
         SET artwork_id = COALESCE(
           artwork_id,
           (SELECT al.artwork_id
            FROM album_artists aa
            JOIN albums al ON al.id = aa.album_id
            WHERE aa.artist_id = artists.id AND al.artwork_id IS NOT NULL
            ORDER BY COALESCE(al.year, 0) DESC
            LIMIT 1)
         )",
    )
    .execute(pool)
    .await?;
    Ok(())
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    out.push(cur.trim().to_string());
    out
}

pub async fn insert_or_update_scan_job(
    pool: &SqlitePool,
    roots: &[String],
) -> Result<ScanJobStatus> {
    let root_paths = serde_json::to_string(roots)?;
    let res = sqlx::query(
        "INSERT INTO scan_jobs (status, root_paths, started_at) VALUES ('queued', ?, ?)",
    )
    .bind(root_paths)
    .bind(now_ms())
    .execute(pool)
    .await?;
    scan_job(pool, res.last_insert_rowid()).await
}

pub async fn scan_job(pool: &SqlitePool, id: i64) -> Result<ScanJobStatus> {
    let row = sqlx::query("SELECT * FROM scan_jobs WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    let root_paths_json: String = row.try_get("root_paths")?;
    Ok(ScanJobStatus {
        id: row.try_get("id")?,
        status: row.try_get("status")?,
        root_paths: serde_json::from_str(&root_paths_json).unwrap_or_default(),
        total_files: row.try_get("total_files")?,
        scanned_files: row.try_get("scanned_files")?,
        started_at: row.try_get("started_at")?,
        finished_at: row.try_get("finished_at")?,
        error: row.try_get("error")?,
    })
}

pub async fn update_scan_job_counts(
    pool: &SqlitePool,
    id: i64,
    status: &str,
    total: Option<i64>,
    scanned: Option<i64>,
    error: Option<&str>,
    finished: bool,
) -> Result<SqliteQueryResult> {
    sqlx::query(
        "UPDATE scan_jobs
         SET status = ?,
             total_files = COALESCE(?, total_files),
             scanned_files = COALESCE(?, scanned_files),
             error = COALESCE(?, error),
             finished_at = CASE WHEN ? THEN ? ELSE finished_at END
         WHERE id = ?",
    )
    .bind(status)
    .bind(total)
    .bind(scanned)
    .bind(error)
    .bind(finished)
    .bind(now_ms())
    .bind(id)
    .execute(pool)
    .await
    .map_err(Into::into)
}

pub async fn track_render_source(pool: &SqlitePool, track_id: i64) -> Result<RenderSourceRow> {
    let row = sqlx::query(
        "SELECT t.id, t.title, t.track_no, t.date, al.title AS album_title,
                mf.path, tas.renderer, tas.codec, tas.start_sample, tas.end_sample,
                tas.start_ms, tas.end_ms
         FROM tracks t
         JOIN track_audio_sources tas ON tas.track_id = t.id
         JOIN media_files mf ON mf.id = tas.media_file_id
         LEFT JOIN albums al ON al.id = t.album_id
         WHERE t.id = ?",
    )
    .bind(track_id)
    .fetch_one(pool)
    .await?;
    let artist = track_artists(pool, track_id)
        .await?
        .into_iter()
        .map(|a| a.name)
        .collect::<Vec<_>>()
        .join(", ");
    Ok(RenderSourceRow {
        title: row.try_get("title")?,
        artist,
        album: row.try_get("album_title")?,
        track_no: row.try_get("track_no")?,
        date: row.try_get("date")?,
        path: row.try_get("path")?,
        renderer: row.try_get("renderer")?,
        codec: row.try_get("codec")?,
        start_sample: row.try_get("start_sample")?,
        end_sample: row.try_get("end_sample")?,
        start_ms: row.try_get("start_ms")?,
        end_ms: row.try_get("end_ms")?,
    })
}

#[derive(Debug, Clone)]
pub struct RenderSourceRow {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub track_no: Option<i64>,
    pub date: Option<String>,
    pub path: String,
    pub renderer: String,
    pub codec: String,
    pub start_sample: Option<i64>,
    pub end_sample: Option<i64>,
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
}

pub async fn source_for_artwork(pool: &SqlitePool, artwork_id: i64) -> Result<ArtworkSourceRow> {
    let row = sqlx::query(
        "SELECT ars.id, ars.kind, ars.media_file_id, ars.sidecar_path,
                ars.embedded_picture_index, ars.mime, mf.path AS media_path
         FROM artwork_sources ars
         LEFT JOIN media_files mf ON mf.id = ars.media_file_id
         WHERE ars.id = ?",
    )
    .bind(artwork_id)
    .fetch_one(pool)
    .await?;
    Ok(ArtworkSourceRow {
        id: row.try_get("id")?,
        kind: row.try_get("kind")?,
        media_file_id: row.try_get("media_file_id")?,
        sidecar_path: row.try_get("sidecar_path")?,
        embedded_picture_index: row.try_get("embedded_picture_index")?,
        mime: row.try_get("mime")?,
        media_path: row.try_get("media_path")?,
    })
}

#[derive(Debug, Clone)]
pub struct ArtworkSourceRow {
    pub id: i64,
    pub kind: String,
    pub media_file_id: Option<i64>,
    pub sidecar_path: Option<String>,
    pub embedded_picture_index: Option<i64>,
    pub mime: Option<String>,
    pub media_path: Option<String>,
}

pub async fn get_artwork_blob(
    pool: &SqlitePool,
    source_id: i64,
    variant: &str,
) -> Result<Option<(Vec<u8>, String)>> {
    if let Some(row) =
        sqlx::query("SELECT bytes, mime FROM artwork_blobs WHERE source_id = ? AND variant = ?")
            .bind(source_id)
            .bind(variant)
            .fetch_optional(pool)
            .await?
    {
        Ok(Some((row.try_get("bytes")?, row.try_get("mime")?)))
    } else {
        Ok(None)
    }
}

pub async fn put_artwork_blob(
    pool: &SqlitePool,
    source_id: i64,
    variant: &str,
    mime: &str,
    width: Option<i64>,
    height: Option<i64>,
    bytes: Vec<u8>,
) -> Result<()> {
    sqlx::query(
        "INSERT OR REPLACE INTO artwork_blobs
         (source_id, variant, mime, width, height, bytes, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(source_id)
    .bind(variant)
    .bind(mime)
    .bind(width)
    .bind(height)
    .bind(bytes)
    .bind(now_ms())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn cache_lyrics(
    pool: &SqlitePool,
    track_id: Option<i64>,
    candidate: &LyricsCandidate,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO lyric_cache
         (track_id, title, artist, album, duration_ms, provider, lyrics, score, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(track_id)
    .bind(&candidate.title)
    .bind(&candidate.artist)
    .bind(&candidate.album)
    .bind(candidate.duration_ms)
    .bind(&candidate.provider)
    .bind(&candidate.lyrics)
    .bind(candidate.score)
    .bind(now_ms())
    .execute(pool)
    .await?;
    Ok(())
}
