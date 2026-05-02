use super::albums::list_albums;
use super::tracks::list_tracks;
use anyhow::Result;
use easy_musiclib_media::normalize::normalize_name;
use easy_musiclib_shared::*;
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};

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
        qb.push(" AND (a.name_norm LIKE ")
            .push_bind(q.clone())
            .push(
                " OR EXISTS (SELECT 1 FROM artist_aliases aa WHERE aa.artist_id = a.id AND aa.alias_norm LIKE ",
            )
            .push_bind(q)
            .push("))");
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
